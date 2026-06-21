//! `lint_brain` — a reading 1-bit model: it learns by *reading*, and only speaks when
//! it actually KNOWS. A faithful Rust port of the OneBit-PC "grownet" idea, built as a
//! self-contained Helpers subsystem (no external model, no toolchain, integer/bit only).
//!
//! How it reads (cloze self-supervision): slide over text; for each content word, form a
//! context vector from the words around it and train a binary associative block to
//! predict that word's code from that context. Reading the dictionary teaches it what
//! words mean by the company they keep; reading the docs teaches it the rules the same
//! way. Blocks fill, then freeze and a fresh block grows — so new reading never
//! overwrites old knowledge (continual learning).
//!
//! How it answers without lying (the KNOWING gate): recall the word the net most
//! associates with a query, but only report it when the answer is *caused by the
//! query's distinctive subject*, not by the generic frame. Drop the highest-IDF
//! observed query word; if the prediction changes, the subject drove it ⇒ it KNOWS.
//! If the answer is unchanged, it was a generic guess ⇒ abstain. This is what keeps it
//! from confidently saying the wrong thing.

use std::collections::HashMap;

use rayon::prelude::*;

/// Code/prediction width in bits. 8192 keeps random codes well separated (lower widths
/// lost enough capacity to confuse the KNOWING gate); the O(BD²) predict cost is held in
/// check by parallelism and a per-session reading budget rather than by shrinking it.
const BD: usize = 8192;
/// Words of `u64` per code.
const BL: usize = BD / 64;
/// Context half-window: a word sees `WIN` neighbours on each side.
const WIN: usize = 4;
/// Cloze items buffered before a block trains-to-convergence and freezes.
const CAP_PER_BLOCK: usize = 2000;
/// Training passes over a block's buffer before it freezes.
const EPOCHS: usize = 20;
/// An association is only "real" when its bit-distance is under this fraction of `BD`.
const CONF: f64 = 0.42;
/// Minimum word length to count as content (no hardcoded stoplist; short words are
/// boundary markers). Digit runs are always content.
const MIN_CONTENT: usize = 4;

/// Splitmix64 PRNG step — deterministic fuel for codes and the learning jitter.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// FNV-1a hash of a token — the codebook seed.
fn fnv(s: &str) -> u64 {
    let mut h = 0xCBF29CE484222325u64;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001B3);
    }
    h
}

/// Bit `i` of a packed vector.
fn gb(v: &[u64], i: usize) -> bool {
    (v[i >> 6] >> (i & 63)) & 1 == 1
}
/// Set bit `i` of a packed vector.
fn sbit(v: &mut [u64], i: usize) {
    v[i >> 6] |= 1u64 << (i & 63);
}
/// Hamming distance between two `BD`-bit vectors.
fn ham(a: &[u64], b: &[u64]) -> u32 {
    (0..BL).map(|j| (a[j] ^ b[j]).count_ones()).sum()
}
/// Bipolar dot `BD − 2·Hamming` — positive when the vectors agree on most bits.
fn dot(a: &[u64], b: &[u64]) -> i32 {
    BD as i32 - 2 * ham(a, b) as i32
}

/// The token's code in a given block: a deterministic random vector salted by the block,
/// so the same token reads differently into each frozen block (decorrelates them).
fn code_of(token: &str, blk: usize, out: &mut [u64]) {
    let mut s = fnv(token) ^ 0x6604EC0DE ^ (blk as u64).wrapping_mul(0x9E3779B1);
    for w in out.iter_mut() {
        *w = splitmix(&mut s);
    }
}

/// The reading 1-bit model: a growing stack of frozen associative blocks plus the vocab
/// and word-frequency table that drive IDF weighting and subject detection.
pub struct Brain {
    blocks: Vec<Vec<u64>>, // each: BD*BL u64 — row i is the weight vector for output bit i
    tok: Vec<String>,
    index: HashMap<String, usize>,
    freq: Vec<u64>,
    total: u64,
    buf: Vec<(Vec<usize>, usize)>, // pending cloze items for the active (last) block
}

impl Default for Brain {
    fn default() -> Self {
        Brain::new()
    }
}

impl Brain {
    /// A fresh brain with one empty block ready to learn.
    pub fn new() -> Brain {
        let mut b = Brain {
            blocks: Vec::new(),
            tok: Vec::new(),
            index: HashMap::new(),
            freq: Vec::new(),
            total: 0,
            buf: Vec::new(),
        };
        b.add_block();
        b
    }

    /// Vocabulary id for a token, interning it when `add` is set.
    fn vid(&mut self, t: &str, add: bool) -> Option<usize> {
        if let Some(&id) = self.index.get(t) {
            return Some(id);
        }
        if !add {
            return None;
        }
        let id = self.tok.len();
        self.tok.push(t.to_string());
        self.freq.push(0);
        self.index.insert(t.to_string(), id);
        Some(id)
    }

    /// Integer-free inverse document frequency: rarer observed words weigh more, so a
    /// distinctive token dominates a context bag and identifies the subject.
    fn idf(&self, id: usize) -> f64 {
        let f = self.freq.get(id).copied().unwrap_or(0);
        ((2.0 + self.total as f64) / (1.0 + f as f64)).ln()
    }

    /// Grow a fresh, randomly-initialized block and make it active.
    fn add_block(&mut self) {
        let mut s = 0xA11CEu64 ^ ((self.blocks.len() as u64) << 17);
        let mut w = vec![0u64; BD * BL];
        for x in w.iter_mut() {
            *x = splitmix(&mut s);
        }
        self.blocks.push(w);
        self.buf.clear();
    }

    /// IDF-weighted majority bundle of token codes → one context vector for block `blk`.
    fn bag(&self, ids: &[usize], blk: usize, out: &mut [u64]) {
        out.iter_mut().for_each(|w| *w = 0);
        if ids.is_empty() {
            return;
        }
        let mut votes = vec![0.0f64; BD];
        let mut tot = 0.0;
        let mut c = [0u64; BL];
        for &id in ids {
            let w = self.idf(id);
            tot += w;
            code_of(&self.tok[id], blk, &mut c);
            for (b, vote) in votes.iter_mut().enumerate() {
                if gb(&c, b) {
                    *vote += w;
                }
            }
        }
        for (b, vote) in votes.iter().enumerate() {
            if vote * 2.0 > tot {
                sbit(out, b);
            }
        }
    }

    /// Predict an output code from a context, in block `b`: output bit i is set when the
    /// context agrees (positive dot) with that bit's learned weight vector. Each output
    /// word's 64 rows are independent, so the rows are evaluated in parallel — this is
    /// the O(BD²) hot path that gates training speed.
    fn predict_blk(&self, b: usize, ctx: &[u64], out: &mut [u64]) {
        let w = &self.blocks[b];
        out.par_iter_mut().enumerate().for_each(|(word, slot)| {
            let mut bits = 0u64;
            for bit in 0..64 {
                let i = word * 64 + bit;
                if dot(ctx, &w[i * BL..i * BL + BL]) > 0 {
                    bits |= 1u64 << bit;
                }
            }
            *slot = bits;
        });
    }

    /// Predictive-coding update: where the prediction disagrees with the observed word's
    /// code, nudge a couple of that output bit's weights toward (or away from) the
    /// context, so next time the prediction moves the right way. Pure bit-flips.
    fn learn_blk(&mut self, b: usize, ctx: &[u64], obs: &[u64]) {
        let mut p = [0u64; BL];
        self.predict_blk(b, ctx, &mut p);
        let mut seed = 0x5EEDu64 ^ ctx[0] ^ (obs[0] << 1);
        let w = &mut self.blocks[b];
        for i in 0..BD {
            if gb(&p, i) == gb(obs, i) {
                continue;
            }
            let want = gb(obs, i);
            let wi = &mut w[i * BL..i * BL + BL];
            let (mut k, mut tr) = (0, 0);
            while k < 2 && tr < 24 {
                tr += 1;
                let pos = (splitmix(&mut seed) % BD as u64) as usize;
                let agree = gb(wi, pos) == gb(ctx, pos);
                if (want && !agree) || (!want && agree) {
                    wi[pos >> 6] ^= 1u64 << (pos & 63);
                    k += 1;
                }
            }
        }
    }

    /// Train the active block to convergence on its buffered cloze items, then freeze it
    /// and grow a new one.
    fn train_and_freeze(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let active = self.blocks.len() - 1;
        let items = std::mem::take(&mut self.buf);
        let mut ctx = [0u64; BL];
        let mut obs = [0u64; BL];
        for _ in 0..EPOCHS {
            for (ctxids, obsid) in &items {
                self.bag(ctxids, active, &mut ctx);
                code_of(&self.tok[*obsid], active, &mut obs);
                self.learn_blk(active, &ctx, &obs);
            }
        }
        self.add_block();
    }

    /// Tokenize text into content-word ids (len ≥ `MIN_CONTENT`, or any digit run);
    /// shorter words become boundary markers (`None`) that break context windows.
    fn tokenize(&mut self, text: &str, add: bool) -> Vec<Option<usize>> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut digit = false;
        let flush = |cur: &mut String, digit: bool, me: &mut Brain, out: &mut Vec<Option<usize>>| {
            if cur.is_empty() {
                return;
            }
            if digit || cur.len() >= MIN_CONTENT {
                out.push(me.vid(cur, add));
            } else {
                out.push(None);
            }
            cur.clear();
        };
        for ch in text.chars() {
            if ch.is_ascii_alphanumeric() || ch == '\'' {
                if cur.is_empty() {
                    digit = ch.is_ascii_digit();
                }
                cur.push(ch.to_ascii_lowercase());
            } else {
                flush(&mut cur, digit, self, &mut out);
            }
        }
        flush(&mut cur, digit, self, &mut out);
        out
    }

    /// Read a block of text: count word frequencies, emit cloze items (each content word
    /// from its neighbours), and train+freeze whenever a block fills.
    pub fn observe(&mut self, text: &str) {
        let toks = self.tokenize(text, true);
        for t in toks.iter().flatten() {
            self.freq[*t] += 1;
            self.total += 1;
        }
        for i in 0..toks.len() {
            let Some(obs) = toks[i] else { continue };
            let lo = i.saturating_sub(WIN);
            let mut ctx = Vec::new();
            for (j, item) in toks.iter().enumerate().take((i + WIN + 1).min(toks.len())).skip(lo) {
                if j != i {
                    if let Some(id) = item {
                        ctx.push(*id);
                    }
                }
            }
            if ctx.len() < 2 {
                continue;
            }
            self.buf.push((ctx, obs));
            if self.buf.len() >= CAP_PER_BLOCK {
                self.train_and_freeze();
            }
        }
    }

    /// Deliberately rehearse a short text `repeats` times so it weighs heavily once the
    /// buffer is trained. Does NOT freeze — call [`Brain::flush`] (or keep reading until a
    /// block fills) to actually learn it, so many `study` calls share one trained block
    /// instead of fragmenting into many thin ones.
    pub fn study(&mut self, text: &str, repeats: usize) {
        for _ in 0..repeats.clamp(1, 64) {
            self.observe(text);
        }
    }

    /// Train the active block on everything buffered so far and freeze it. Call this at
    /// the end of a reading/study session so a small corpus is actually learned (reading
    /// large volume freezes blocks automatically as they fill).
    pub fn flush(&mut self) {
        self.train_and_freeze();
    }

    /// Parse a query into content-word ids it already knows (no interning).
    fn query_ids(&mut self, query: &str) -> Vec<usize> {
        self.tokenize(query, false).into_iter().flatten().collect()
    }

    /// The vocab token the net most associates with `qids` across all blocks, excluding
    /// `drop` (and the query's own words) from both context and candidates. Returns the
    /// winner and its bit-distance.
    fn recall_ids(&self, qids: &[usize], drop: Option<usize>) -> (Option<usize>, u32) {
        let use_ids: Vec<usize> = qids.iter().copied().filter(|&i| Some(i) != drop).collect();
        if use_ids.is_empty() {
            return (None, BD as u32 + 1);
        }
        let mut best: Option<usize> = None;
        let mut bd = BD as u32 + 1;
        let mut ctx = [0u64; BL];
        let mut pr = [0u64; BL];
        for b in 0..self.blocks.len() {
            self.bag(&use_ids, b, &mut ctx);
            self.predict_blk(b, &ctx, &mut pr);
            // nearest vocab code to the prediction, scanned in parallel
            let (d, t) = (0..self.tok.len())
                .into_par_iter()
                .filter(|&t| Some(t) != drop && !use_ids.contains(&t))
                .map(|t| {
                    let mut c = [0u64; BL];
                    code_of(&self.tok[t], b, &mut c);
                    (ham(&pr, &c), t)
                })
                .min()
                .unwrap_or((BD as u32 + 1, usize::MAX));
            if d < bd {
                bd = d;
                best = Some(t);
            }
        }
        (best, bd)
    }

    /// Recall the associated word for `query` when the association is real (close enough),
    /// else `None`. This is the lenient path — see [`Brain::known`] for the strict gate.
    pub fn recall(&mut self, query: &str) -> Option<String> {
        let qids = self.query_ids(query);
        let (best, bd) = self.recall_ids(&qids, None);
        match best {
            Some(t) if bd <= (BD as f64 * CONF) as u32 => Some(self.tok[t].clone()),
            _ => None,
        }
    }

    /// The KNOWING gate: answer only when the query's distinctive *subject* causes the
    /// answer. The subject is the highest-IDF query word the net has actually observed;
    /// drop it and re-recall — if the answer changes, the subject drove it (the net
    /// knows the specific fact); if not, it was a generic guess, so abstain. This is the
    /// safeguard against confidently saying the wrong thing.
    pub fn known(&mut self, query: &str) -> Option<String> {
        let qids = self.query_ids(query);
        if qids.len() < 2 {
            return None;
        }
        let (best, bd) = self.recall_ids(&qids, None);
        let best = best?;
        if bd > (BD as f64 * CONF) as u32 {
            return None; // association isn't real
        }
        // subject = most distinctive query word the net has genuinely observed
        let mut subj = None;
        let mut best_idf = -1.0;
        for &id in &qids {
            if self.freq.get(id).copied().unwrap_or(0) < 2 {
                continue;
            }
            let w = self.idf(id);
            if w > best_idf {
                best_idf = w;
                subj = Some(id);
            }
        }
        let subj = subj?;
        let (generic, _) = self.recall_ids(&qids, Some(subj));
        if generic == Some(best) {
            return None; // answer was generic-frame, not subject-specific
        }
        Some(self.tok[best].clone())
    }

    /// Vocabulary size — how many distinct words the net has read.
    pub fn vocab(&self) -> usize {
        self.tok.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trained_brain() -> Brain {
        let mut b = Brain::new();
        // Read a few facts with the same frame but different subject→answer, then flush
        // once so they share a single well-trained block.
        for _ in 0..10 {
            b.observe("the capital city of france is paris");
            b.observe("the capital city of spain is madrid");
            b.observe("the capital city of italy is rome");
        }
        b.flush();
        b
    }

    #[test]
    fn reads_and_recalls_a_learned_association() {
        let mut b = trained_brain();
        assert!(b.recall("capital city france").is_some(), "expected a confident recall");
    }

    #[test]
    fn the_knowing_gate_abstains_on_a_generic_frame() {
        let mut b = trained_brain();
        // A query with only the generic frame and no learned distinctive subject must
        // not be reported as KNOWN (it can only be a generic guess).
        assert_eq!(b.known("capital city"), None);
    }
}
