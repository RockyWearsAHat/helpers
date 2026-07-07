//! `lint_read` — a 1-bit sequential predictive coder that LEARNS a language by *reading* its docs.
//!
//! Cross-module theory, evidence hierarchy, and the failure ledger live in `LINTER.md` at the
//! repo root — the single authoritative doc; update it BEFORE changing semantics here.
//!
//! The linter's brief is that expanding what it understands must never require a code change — only
//! more reading. So there is no authored keyword list and no hand-labeled example corpus here. As the
//! crawler streams a document, [`Reader`] walks its tokens in order and does predictive coding with
//! pure 1-bit hypervector ops (permute / XOR / majority / Hamming, no floats, no backprop):
//!
//!   * it keeps a rolling **context** hypervector of the recent tokens (`ctx' = ρ(ctx) ⊕ hv(tok)`);
//!   * it **predicts** the next token from an associative memory addressed by that context;
//!   * when the prediction is right it touches nothing (the token was already comprehended — it
//!     carries no new information); when it errs it updates ONLY the addressed memory slot (local,
//!     least-touched error correction, everything else intact).
//!
//! That learned memory is what "having read the docs" means: tokens the reader can already predict in
//! context (the common connective vocabulary) stop being informative, so [`Reader::encode`] bundles
//! only the *surprising* tokens of a span into its hypervector — a learned stop-list, never a written
//! one. [`Polarity`] then classifies documentation prose as prohibition vs endorsement by nearest
//! prototype, where the prototypes are accumulated from reading + toolchain grounding
//! ([`crate::lint_toolchain`]), not from any authored labels.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::lint_ai::{token_hv, Bundler, Hv, DIM};

/// A token is judged "predicted" (already comprehended, not bundled) only when the memory's
/// expectation for its context lands within this Hamming radius of it. Distinct 8192-bit token codes
/// sit ~`DIM/2` apart, so this narrow radius means "essentially the token this context predicted".
const SURPRISE_RADIUS: u32 = (DIM / 4) as u32;

/// Cap on distinct context slots the reader remembers, so reading a whole doc site stays bounded in
/// memory and serialization size. Once full, existing slots still update (adopt the latest surprise)
/// but no new context is added — the memory saturates rather than growing without limit.
const MEM_CAP: usize = 4096;

/// Split text into meaningful tokens for the reader: Unicode word runs, lowercased. The
/// vocabulary is SELF-DEFINING — any string becomes a code, nothing is enumerated — and a run
/// additionally contributes its sub-parts so an unseen compound is COMPREHENDED through parts
/// the reader has read:
///
///   * a run with any non-ASCII character contributes each character (Japanese/Chinese prose
///     shares morphemes across phrasings instead of collapsing to one opaque token);
///   * a run with internal case or letter/digit boundaries (camelCase, PascalCase, `env2json`)
///     contributes each part — `getUserName` is read as itself PLUS get/user/name, so a word the
///     reader never saw still lands near the vocabulary it is built from. Case boundaries are
///     script typography, exactly like whitespace — no vocabulary is consulted.
pub fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (full, parts) in read_units(text) {
        if !full.is_empty() {
            out.push(full);
        }
        out.extend(parts);
    }
    out
}

/// Split prose into SENTENCES: a boundary is sentence punctuation (`.;!?` or a newline) followed
/// by whitespace or the end of the text. A `.` between letters (`string.format`, `pickle.loads`)
/// is part of a word, not a boundary — naive splitting mutilates exactly the constructs prose
/// laws name. Pure typography, no vocabulary.
pub fn sentences(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        let boundary = matches!(b, b'.' | b';' | b'!' | b'?')
            && bytes.get(i + 1).is_none_or(|n| n.is_ascii_whitespace());
        if boundary || *b == b'\n' {
            let s = text[start..i].trim();
            if !s.is_empty() {
                out.push(s);
            }
            start = i + 1;
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// One reading unit per word run: the run itself (lowercased; empty when under 2 chars) plus its
/// typographic sub-parts. Parts come from script typography only — CJK characters, camelCase /
/// acronym case boundaries, letter↔digit edges — never a vocabulary, so the decomposition works
/// for words that do not exist yet. [`Reader`] encodes an uncommon unit as its word AND its
/// parts, which is how an unseen compound is comprehended through vocabulary already read.
pub fn read_units(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for run in text.split(|c: char| !c.is_alphanumeric()) {
        if run.is_empty() {
            continue;
        }
        let lower = run.to_lowercase();
        let full = if lower.chars().count() >= 2 { lower } else { String::new() };
        let mut parts: Vec<String> = Vec::new();
        if run.chars().any(|c| !c.is_ascii()) {
            for ch in run.chars() {
                let mut buf = [0u8; 4];
                parts.push(ch.encode_utf8(&mut buf).to_lowercase());
            }
        } else {
            // Case/digit boundaries: before an uppercase following a lowercase (camel), before
            // the last uppercase of an acronym head (HTMLParser → html, parser), at digit edges.
            let chars: Vec<char> = run.chars().collect();
            let mut segs: Vec<String> = Vec::new();
            let mut start = 0usize;
            for i in 1..chars.len() {
                let (prev, cur) = (chars[i - 1], chars[i]);
                let camel = cur.is_ascii_uppercase() && prev.is_ascii_lowercase();
                let acronym_end = cur.is_ascii_lowercase()
                    && prev.is_ascii_uppercase()
                    && i >= 2
                    && chars[i - 2].is_ascii_uppercase();
                let digit_edge = cur.is_ascii_digit() != prev.is_ascii_digit();
                if camel || digit_edge {
                    segs.push(chars[start..i].iter().collect());
                    start = i;
                } else if acronym_end {
                    segs.push(chars[start..i - 1].iter().collect());
                    start = i - 1;
                }
            }
            if start > 0 {
                segs.push(chars[start..].iter().collect());
                parts.extend(
                    segs.into_iter().map(|p| p.to_lowercase()).filter(|p| p.chars().count() >= 2),
                );
            }
        }
        if !full.is_empty() || !parts.is_empty() {
            out.push((full, parts));
        }
    }
    out
}

/// Serialize a `HashSet<u64>` as a SORTED array — deterministic JSON for committed
/// bootstraps (clean diffs) and for value-level artifact comparison in tests. Deserialization
/// is untouched (an array reads into a set whatever its order).
pub(crate) fn sorted_u64_set<S: serde::Serializer>(
    set: &std::collections::HashSet<u64>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut sorted: Vec<u64> = set.iter().copied().collect();
    sorted.sort_unstable();
    serde::Serialize::serialize(&sorted, s)
}

/// A stable u32 address for a context hypervector — the slot the predictor reads and writes. Similar
/// contexts need not collide (this is a hash, not an LSH), which keeps the predictor conservative:
/// it only ever claims to "know" a token in a context it has literally seen before.
fn ctx_key(ctx: &Hv) -> u32 {
    let mut h = 0xCBF29CE484222325u64;
    for w in ctx.as_words() {
        h = h.rotate_left(5) ^ *w;
        h = h.wrapping_mul(0x100000001B3);
    }
    (h ^ (h >> 32)) as u32
}

/// Token-frequency storage with exactly ONE live representation: a HOT map while a reader is
/// learning, or a FROZEN sorted-array pair when loaded from a binary artifact — decoding is a
/// bulk copy off the `HLM1` RAW stream and lookups binary-search (LINTER.md, "Save"). The
/// first write thaws the arrays into the map; lint runs never write, so a loaded substrate
/// stays frozen for its whole life and the 400k-entry English brain loads in microseconds.
#[derive(Clone, Default)]
struct FreqTable {
    /// Sorted token seeds (frozen representation; empty while `hot` is live).
    keys: Vec<u64>,
    /// Read counts parallel to `keys`.
    counts: Vec<u32>,
    /// The learning representation (empty while the frozen arrays are live).
    hot: HashMap<u64, u32>,
}

impl FreqTable {
    /// Adopt decoded arrays as the frozen representation; sorts them if the artifact was not
    /// already sorted (it always is — [`FreqTable::sorted_pairs`] writes it that way).
    fn frozen(keys: Vec<u64>, counts: Vec<u32>) -> Option<FreqTable> {
        if keys.len() != counts.len() {
            return None;
        }
        if keys.windows(2).all(|w| w[0] < w[1]) {
            return Some(FreqTable { keys, counts, hot: HashMap::new() });
        }
        let mut pairs: Vec<(u64, u32)> = keys.into_iter().zip(counts).collect();
        pairs.sort_unstable_by_key(|(k, _)| *k);
        pairs.dedup_by_key(|(k, _)| *k);
        let (keys, counts) = pairs.into_iter().unzip();
        Some(FreqTable { keys, counts, hot: HashMap::new() })
    }

    /// The table as sorted parallel arrays — the encode-side view, whatever the representation.
    fn sorted_pairs(&self) -> (Vec<u64>, Vec<u32>) {
        if self.hot.is_empty() {
            return (self.keys.clone(), self.counts.clone());
        }
        let mut pairs: Vec<(u64, u32)> = self.hot.iter().map(|(k, v)| (*k, *v)).collect();
        pairs.sort_unstable_by_key(|(k, _)| *k);
        pairs.into_iter().unzip()
    }

    fn get(&self, seed: u64) -> u32 {
        if !self.hot.is_empty() {
            return self.hot.get(&seed).copied().unwrap_or(0);
        }
        self.keys.binary_search(&seed).map(|i| self.counts[i]).unwrap_or(0)
    }

    /// Add one read to `seed`'s count — thaws a frozen table first (learning path only).
    fn bump(&mut self, seed: u64) {
        if !self.keys.is_empty() {
            self.hot = self.keys.drain(..).zip(self.counts.drain(..)).collect();
        }
        *self.hot.entry(seed).or_default() += 1;
    }

    /// Drop entries read fewer than `n` times, in whichever representation is live.
    fn retain_read_at_least(&mut self, n: u32) {
        if self.hot.is_empty() {
            let kept: Vec<(u64, u32)> = self
                .keys
                .iter()
                .zip(&self.counts)
                .filter(|(_, c)| **c >= n)
                .map(|(k, c)| (*k, *c))
                .collect();
            (self.keys, self.counts) = kept.into_iter().unzip();
            return;
        }
        self.hot.retain(|_, c| *c >= n);
    }

    /// Every read count, unordered — the head-cutoff mass calculation's input.
    fn count_values(&self) -> Vec<u32> {
        if self.hot.is_empty() {
            return self.counts.clone();
        }
        self.hot.values().copied().collect()
    }
}

/// JSON view (committed bootstraps, legacy caches): the table is a plain `{seed: count}` map
/// in both directions, exactly what the former `HashMap<u64, u32>` field serialized as.
impl Serialize for FreqTable {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.hot.is_empty() {
            return s.collect_map(self.keys.iter().zip(self.counts.iter()));
        }
        s.collect_map(self.hot.iter())
    }
}

impl<'de> Deserialize<'de> for FreqTable {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<FreqTable, D::Error> {
        Ok(FreqTable { hot: HashMap::deserialize(d)?, ..FreqTable::default() })
    }
}

/// The reader's learned comprehension of a corpus: an associative memory from local context to the
/// token that context predicts. It grows as the reader reads and is the serializable artifact that
/// makes a warm run skip re-reading. All operations are 1-bit (XOR / permute / Hamming).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Reader {
    /// Context address → predicted next-token code. Bounded by [`MEM_CAP`]. Not serialized: it
    /// steers only the LEARNING pass (which tokens surprised the reader while reading); the frozen
    /// classifier consults `freq` alone, and skipping ~4k × 8192-bit slots keeps a cached memory
    /// small enough to load instantly on warm runs.
    #[serde(skip)]
    mem: HashMap<u32, Hv>,
    /// Learned token frequencies (token seed → times read). The reader's own record of which words
    /// are common — a corpus-derived stop-list, never a written one.
    #[serde(default)]
    freq: FreqTable,
    /// Total tokens read — the mass the corpus head is measured against.
    #[serde(default)]
    total: u64,
    /// Cached head cutoff ([`Reader::head_cutoff`]); recomputed lazily per loaded reader.
    #[serde(skip)]
    head: std::sync::OnceLock<u32>,
}

impl Reader {
    /// A reader that has read nothing yet.
    pub fn new() -> Reader {
        Reader { mem: HashMap::new(), freq: FreqTable::default(), total: 0, head: std::sync::OnceLock::new() }
    }

    /// Number of learned context slots — how much the reader has comprehended.
    pub fn learned(&self) -> usize {
        self.mem.len()
    }

    /// Total tokens this reader has read — the corpus mass its frequency judgments stand on.
    pub fn total_read(&self) -> u64 {
        self.total
    }

    /// Drop the frequency tail below `n` reads — data compaction for a COMMITTED substrate
    /// (single reads are scan noise and hapax typos, not knowledge). `total` stays untouched:
    /// it is the mass that was READ, and the head/information judgments must keep standing on
    /// the real corpus size.
    pub fn retain_read_at_least(&mut self, n: u32) {
        self.freq.retain_read_at_least(n);
    }

    /// The frequency at or above which a token is dropped from a span's salient VOTING set.
    /// Scales with corpus size ("much more frequent than average"), floored so a tiny corpus
    /// still filters its obvious filler. Voting and connectiveness are different questions —
    /// see [`Reader::is_head_word`] for the latter.
    fn common_cutoff(&self) -> u32 {
        (self.total / 120).max(3) as u32
    }

    /// True when this reader has LEARNED that `token` is common connective prose (its read
    /// frequency reached the corpus-scaled cutoff). A token the reader never read is not common —
    /// unknown words stay salient. This is the word-level view of the learned stop-list.
    pub fn is_common_word(&self, token: &str) -> bool {
        self.read_count(token) >= self.common_cutoff()
    }

    /// The read count at which a token belongs to the corpus HEAD: the most-read words that
    /// together account for half of everything the reader has ever read (Zipf's head — "the",
    /// "to", "do", "use"…). Mass-based, so it stays meaningful at every corpus size. This is the
    /// CONNECTIVE-word judgment construct SELECTION ranks with; it is deliberately stricter than
    /// the voting filter — silencing a polar word like "deprecated" costs verdicts, but ranking
    /// it behind a rarer construct costs nothing. Cached per loaded reader.
    fn head_cutoff(&self) -> u32 {
        *self.head.get_or_init(|| {
            let mut counts: Vec<u32> = self.freq.count_values();
            counts.sort_unstable_by(|a, b| b.cmp(a));
            let half = self.total / 2;
            let mut seen = 0u64;
            for c in counts {
                seen += c as u64;
                if seen >= half {
                    return c.max(2);
                }
            }
            u32::MAX // nothing read yet — no word is common
        })
    }

    /// Whether `token` sits in the corpus head ([`Reader::head_cutoff`]) — connective filler for
    /// RANKING purposes.
    pub fn is_head_word(&self, token: &str) -> bool {
        self.read_count(token) >= self.head_cutoff()
    }

    /// [`is_head_word`] by token seed — for callers that only carry seeds (the dictionary
    /// meaning network stores definitions as seed lists).
    pub fn is_head_seed(&self, seed: u64) -> bool {
        self.freq.get(seed) >= self.head_cutoff()
    }

    /// How many times this reader has read `token` — 0 for a word it has never seen. The raw
    /// evidence rarity rankings are built from (a rarer word carries more of a sentence's
    /// identity).
    pub fn read_count(&self, token: &str) -> u32 {
        let seed = crate::lint_ai::token_seed(&token.to_lowercase());
        self.freq.get(seed)
    }

    /// Read a span sequentially and LEARN from it: at each token, when the memory's prediction for
    /// the current context is wrong (or absent), update only that one addressed slot to expect this
    /// token. Correct predictions touch nothing. Token frequencies are tallied alongside. This is the
    /// predictive-coding write path.
    pub fn learn_span(&mut self, text: &str) {
        let mut ctx = Hv::zero();
        for tok in tokens(text) {
            let seed = crate::lint_ai::token_seed(&tok);
            self.freq.bump(seed);
            self.total += 1;
            let h = token_hv(&tok);
            let key = ctx_key(&ctx);
            let surprising = self.mem.get(&key).map_or(true, |p| p.distance(&h) > SURPRISE_RADIUS);
            if surprising && (self.mem.len() < MEM_CAP || self.mem.contains_key(&key)) {
                self.mem.insert(key, h); // local, least-touched: only the addressed slot changes
            }
            ctx = ctx.rotl1_pub().xor(&h);
        }
    }

    /// A word's meaning weight in BITS — Shannon self-information from the learned frequencies
    /// (`−log₂` of the word's read probability, integer). The reverse-logarithmic curve: the most
    /// common words weigh almost nothing, the band below them is valued, and rare words carry the
    /// actual meaning. An unread word carries maximum surprise. Never zero — a token that earns a
    /// vote at all keeps at least one bit.
    pub(crate) fn info_bits(&self, word: &str) -> u32 {
        if self.total == 0 {
            return 1;
        }
        let count = u64::from(self.read_count(word).max(1));
        (self.total / count).max(2).ilog2()
    }

    /// [`Reader::salient`] with each code's information weight — the salient set for weighted
    /// voting and weighted bundling. The stop-list still drops the ultra-common outright; the
    /// weights grade everything that survives.
    pub(crate) fn salient_weighted(&self, text: &str) -> Vec<(Hv, u32)> {
        let cutoff = self.common_cutoff();
        let mut salient = Vec::new();
        let mut all = Vec::new();
        for (full, parts) in read_units(text) {
            let word = if full.is_empty() { parts.first().cloned().unwrap_or_default() } else { full };
            if word.is_empty() {
                continue;
            }
            let h = Hv::random(crate::lint_ai::token_seed(&word));
            all.push((h, self.info_bits(&word)));
            if self.read_count(&word) < cutoff {
                salient.push((h, self.info_bits(&word)));
                // COMPREHENSION: a word the reader never read still contributes its
                // typographic parts' codes, so `getUserName` shares structure with everything
                // else built from get/user/name.
                if self.read_count(&word) == 0 {
                    for p in &parts {
                        salient.push((Hv::random(crate::lint_ai::token_seed(p)), self.info_bits(p)));
                    }
                }
            }
        }
        if salient.is_empty() { all } else { salient }
    }

    /// Encode a span into one hypervector: the information-weighted majority bundle of its salient
    /// token codes (a bag-of-distinctive-words centroid where rarer words pull harder). `None` for
    /// an empty span.
    pub fn encode(&self, text: &str) -> Option<Hv> {
        let toks = self.salient_weighted(text);
        if toks.is_empty() {
            return None;
        }
        let mut b = Bundler::new();
        for (h, w) in &toks {
            b.add_weighted(h, *w);
        }
        Some(b.finalize())
    }
}

/// How many deterministic probe codes measure a prototype pair's noise floor. Probe codes are
/// random token hypervectors — vocabulary that belongs to neither side by construction — so the
/// widest |d(bad) − d(good)| they show is what "no signal" looks like for THESE prototypes.
const CALIBRATION_PROBES: u64 = 512;

/// The vote margin CALIBRATED from the trained prototypes: 1.5× the widest lean shown by
/// [`CALIBRATION_PROBES`] neutral probe codes. The probes' maximum estimates the noise floor; the
/// 50% headroom covers the distribution tail past a finite sample (verified by the pseudo-word
/// sweep in the tests). A tiny or imbalanced training has a wide floor and gets a wide margin; a
/// large one tightens automatically — no hand-tuned constant, the trained model measures itself.
fn calibrated_margin(bad: &Hv, good: &Hv) -> u32 {
    let mut floor = 0u32;
    for i in 0..CALIBRATION_PROBES {
        let probe = Hv::random(0x9E3779B97F4A7C15 ^ i.wrapping_mul(0xA24BAED4963EE407));
        floor = floor.max(probe.distance(bad).abs_diff(probe.distance(good)));
    }
    floor + floor / 2
}

/// Accumulator that builds a [`Polarity`] from read + toolchain-grounded examples. Prohibition prose
/// (the context around code the toolchain flags) bundles into the bad prototype; endorsement prose
/// (the context around code the toolchain accepts) into the good one. No authored labels involved.
pub struct PolarityBuilder {
    reader: Reader,
    bad: Bundler,
    good: Bundler,
    /// Per-token side tallies (token seed → summed info-bit weight under a BAD label,
    /// under clean EXPOSURE — good labels and plain clean-grounded reality together) — the
    /// evidence layer of the side-count design (LINTER.md, "The side-count evidence
    /// layer"). Honest grounding labels are the only labels; exposure is the denominator.
    tallies: std::collections::HashMap<u64, (u32, u32)>,
}

impl PolarityBuilder {
    /// Start from a reader that has already read the corpus (so its learned stop-list applies to the
    /// prose it encodes).
    pub fn new(reader: Reader) -> PolarityBuilder {
        PolarityBuilder {
            reader,
            bad: Bundler::new(),
            good: Bundler::new(),
            tallies: std::collections::HashMap::new(),
        }
    }

    /// Fold one grounded example into the prototypes: every salient token of `prose` votes into the
    /// prohibition prototype (the toolchain flagged its code) or the endorsement one (it accepted it).
    /// Voting per token — not per sentence — keeps a distinctive word's signal from being washed out
    /// by two layers of majority, and each vote weighs the token's information content
    /// ([`Reader::info_bits`]) so the prototypes are centroids of MEANING, not of register filler.
    pub fn accumulate(&mut self, prose: &str, is_bad: bool) {
        let target = if is_bad { &mut self.bad } else { &mut self.good };
        for (h, w) in self.reader.salient_weighted(prose) {
            target.add_weighted(&h, w);
        }
        self.tally(prose, is_bad);
    }

    /// Tally one grounded prose span WITHOUT training the prototypes — the EXPOSURE side of
    /// the side-count design (LINTER.md): a clean verdict is not an endorsement label, but
    /// it IS reality the word stood next to, and without it every ubiquitous word would
    /// lean bad by default ("not" appears in 6% flagged prose → neutral; "deprecated" in
    /// 75% → prohibition vocabulary). Callers feed every clean-grounded unit through here.
    pub fn tally_exposure(&mut self, prose: &str) {
        self.tally(prose, false);
    }

    /// Tallies cover EVERY read token, common words included (LINTER.md, "The side-count
    /// evidence layer"): English carries prohibition in its most ubiquitous words — the
    /// negation primitives — and reality must be allowed to polarize them. Commonness
    /// discounts the recorded WEIGHT (info bits), never the existence of the evidence.
    fn tally(&mut self, prose: &str, is_bad: bool) {
        for (full, parts) in read_units(prose) {
            let word = if full.is_empty() { parts.first().cloned().unwrap_or_default() } else { full };
            if word.is_empty() {
                continue;
            }
            let w = self.reader.info_bits(&word).max(1);
            let t = self.tallies.entry(crate::lint_ai::token_seed(&word)).or_insert((0, 0));
            if is_bad {
                t.0 = t.0.saturating_add(w);
            } else {
                t.1 = t.1.saturating_add(w);
            }
        }
    }

    /// Freeze the prototypes and the reader into a [`Polarity`] classifier.
    pub fn build(self) -> Polarity {
        let mut tallies: Vec<(u64, u32, u32)> =
            self.tallies.into_iter().map(|(k, (f, c))| (k, f, c)).collect();
        tallies.sort_unstable_by_key(|(k, _, _)| *k);
        Polarity {
            bad: self.bad.finalize(),
            good: self.good.finalize(),
            bad_n: self.bad.len(),
            good_n: self.good.len(),
            margin: std::sync::OnceLock::new(),
            reader: self.reader,
            tallies,
        }
    }
}

/// A learned good/bad polarity classifier over documentation prose. Two prototype hypervectors —
/// prohibition and endorsement — let the model DECIDE a snippet's polarity by nearest prototype
/// instead of matching a hand-kept keyword list. The prototypes are grown from reading + toolchain
/// grounding, so expanding coverage is more reading, never a code edit.
#[derive(Clone, Serialize, Deserialize)]
pub struct Polarity {
    /// The reader whose learned stop-list encodes prose for classification.
    reader: Reader,
    /// Prohibition prototype (context around toolchain-flagged code).
    bad: Hv,
    /// Endorsement prototype (context around toolchain-accepted code).
    good: Hv,
    /// Token votes bundled into `bad` — zero means the bad side is untrained (classifier abstains).
    bad_n: usize,
    /// Token votes bundled into `good` — zero means the good side is untrained.
    good_n: usize,
    /// Lazily computed calibrated vote margin (deterministic; excluded from serialization so a
    /// loaded memory re-measures its own prototypes).
    #[serde(skip)]
    margin: std::sync::OnceLock<u32>,
    /// Frozen per-token side tallies `(token seed, bad-label weight, clean-exposure
    /// weight)`, sorted by seed — the side-count evidence layer. Empty on legacy JSON
    /// artifacts (they abstain to the prototypes).
    #[serde(default)]
    tallies: Vec<(u64, u32, u32)>,
}

impl Reader {
    /// The shared EMPTY reader — a machine that has read nothing yet. Every word reads as
    /// unread (count 0, no corpus head), so callers that rank by reading knowledge degrade
    /// to English knowledge + existence + document order: the self-bootstrap floor a cold
    /// machine's project law compiles through (LINTER.md, "Honest grounding labels").
    pub fn empty() -> &'static Reader {
        static EMPTY: std::sync::OnceLock<Reader> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Reader::new)
    }
}

impl Polarity {
    /// Build directly from labeled prose — the offline/test constructor. The reader first READS all
    /// the prose (learning its common-word stop-list), then `(prose, is_bad)` pairs accumulate exactly
    /// as the grounded path does.
    pub fn from_labeled(examples: &[(&str, bool)]) -> Polarity {
        let mut reader = Reader::new();
        for (prose, _) in examples {
            reader.learn_span(prose);
        }
        let mut b = PolarityBuilder::new(reader);
        for (prose, is_bad) in examples {
            b.accumulate(prose, *is_bad);
        }
        b.build()
    }

    /// Total grounded votes behind this classifier — how much reality-tested reading trained it.
    /// The transfer store keeps whichever classifier carries the most.
    pub fn votes(&self) -> usize {
        self.bad_n + self.good_n
    }

    /// The side-count evidence for one token: summed info-bit weight under bad labels vs
    /// under good labels. `(0, 0)` for a token the grounding never met.
    pub fn tally_of(&self, word: &str) -> (u32, u32) {
        self.tally_of_seed(crate::lint_ai::token_seed(&word.to_lowercase()))
    }

    /// [`tally_of`] by token seed — the form the dictionary hop reads with.
    fn tally_of_seed(&self, seed: u64) -> (u32, u32) {
        match self.tallies.binary_search_by_key(&seed, |(k, _, _)| *k) {
            Ok(i) => (self.tallies[i].1, self.tallies[i].2),
            Err(_) => (0, 0),
        }
    }

    /// The token's side-count LEAN, mildly asymmetric by design (LINTER.md, "The side-count
    /// evidence layer"): a bad label is reality's own verdict (the toolchain flagged the
    /// code), so a bad lean needs a 2:1 majority; a good label is structural inference (the
    /// fix-sibling convention, which some pages break), so a good lean needs 4:1. With under
    /// 4 bits of direct evidence the word is read through its own DICTIONARY DEFINITION —
    /// one hop through the LangBrain's meaning network ("forbid" = "order NOT to do"
    /// inherits the grounded lean of "not"), so reality's verdicts on a few thousand pages
    /// polarize the whole English vocabulary. No decisive majority anywhere → abstain.
    pub fn tally_lean(&self, word: &str) -> Option<bool> {
        let seed = crate::lint_ai::token_seed(&word.to_lowercase());
        // Dictionary negation is KNOWLEDGE, not statistics: exposure beside parsing code
        // cannot un-negate a word — a parse verdict cannot see style wrongness, so
        // "incorrect" beside a thousand clean-parsing `var x = 1` examples still states
        // wrongness (in-correct = not correct, the brain's own discovery). Negation
        // OPERATORS lean prohibition warm or cold; their info-bit weight still decides
        // how much any one of them moves a span.
        if crate::lint_english::brain().is_some_and(|e| e.is_negation(seed)) {
            return Some(true);
        }
        let (f, c) = self.tally_of_seed(seed);
        if f + c >= 4 {
            return lean_of(f, c);
        }
        let english = crate::lint_english::brain()?;
        let def = english.definition_of(seed)?;
        // The hop inherits PROHIBITION only, and only from decisive CONTENT words: words
        // English itself reads as common (pronouns, articles — "it", "of", "a") pick up
        // accidental one-sided tallies from every labeled span and must never donate a
        // lean ("fold" = "bend ... over on ITself" says nothing about prohibition).
        // Negation inheritance is not this hop's job — the discovered negation cluster
        // ([`crate::lint_english::English::is_negation`], the cold floor) carries it.
        // Endorsement never crosses the hop — only reality can endorse.
        let mut df = 0u32;
        for s in def {
            if english.reader.is_head_seed(*s) {
                continue;
            }
            let (a, b) = self.tally_of_seed(*s);
            if a + b >= 4 && lean_of(a, b) == Some(true) {
                df = df.saturating_add(a);
            }
        }
        // The same ≥8-bit bar the span layer uses: one genuinely informative donor plus
        // change — a single tiny-corpus co-sighting (~7 bits) is not inherited meaning.
        (df >= 8).then_some(true)
    }

    /// Classify a span by SIDE-COUNT evidence alone: information-weighted votes from
    /// tokens with a decisive lean; prohibition needs the bad side to carry twice the
    /// good side's bits and at least 8 bits outright (one genuinely informative leaning
    /// word plus change). `None` when the tallies cannot decide — the caller falls back
    /// to the span prototypes.
    pub fn classify_tallied(&self, prose: &str) -> Option<bool> {
        if self.tallies.is_empty() {
            return None;
        }
        let mut bad_bits = 0u64;
        let mut good_bits = 0u64;
        for (full, parts) in read_units(prose) {
            let word = if full.is_empty() { parts.first().cloned().unwrap_or_default() } else { full };
            if word.is_empty() {
                continue;
            }
            match self.tally_lean(&word) {
                Some(true) => bad_bits += u64::from(self.info_bits(&word)),
                Some(false) => good_bits += u64::from(self.info_bits(&word)),
                None => {}
            }
        }
        if bad_bits >= 8 && bad_bits >= good_bits * 2 {
            Some(true)
        } else if good_bits >= 8 && good_bits >= bad_bits * 2 {
            Some(false)
        } else {
            None
        }
    }

    /// The reader's info-bits for a word — exposed for the side-count layer.
    fn info_bits(&self, word: &str) -> u32 {
        self.reader.info_bits(word)
    }

    /// The COLD FLOOR: classify overt prohibition through the dictionary's discovered
    /// negation cluster alone (LINTER.md, "The cold floor") — the reading an UNREADY
    /// classifier still renders. A span is a prohibition when its negation-clustered words
    /// carry ≥8 info bits; everything else abstains, and endorsement is never rendered
    /// here — only reality can endorse.
    fn classify_negation(&self, prose: &str) -> Option<bool> {
        // One REAL negation word is the floor: prohibition sentences typically carry
        // exactly one ("Never use X"), and a genuine reading of it is worth ~4+ bits.
        (self.negation_bits(prose) >= 4).then_some(true)
    }

    /// The information weight of `prose`'s negation-clustered words — the cold floor's raw
    /// reading, exposed so callers can compare SPANS relatively (a fix block must read
    /// decisively less negated than its violation; absolute cluster membership alone cannot
    /// tell "Never use X" from "use the plain form" — "plain" is dictionary-defined via
    /// litotes, "not decorated").
    pub fn negation_bits(&self, prose: &str) -> u64 {
        let Some(english) = crate::lint_english::brain() else { return 0 };
        let mut bits = 0u64;
        for (full, parts) in read_units(prose) {
            let word = if full.is_empty() { parts.first().cloned().unwrap_or_default() } else { full };
            if word.is_empty() {
                continue;
            }
            if english.is_negation(crate::lint_ai::token_seed(&word)) {
                bits += u64::from(self.info_bits(&word).max(1));
            }
        }
        bits
    }
}

/// The side-count ratio bars (LINTER.md, "The side-count evidence layer"): bad needs 2:1
/// (reality's own verdict), good needs 4:1 (structural inference), anything else abstains.
fn lean_of(f: u32, c: u32) -> Option<bool> {
    if f >= c.saturating_mul(2) {
        Some(true)
    } else if c >= f.saturating_mul(4) {
        Some(false)
    } else {
        None
    }
}

impl Polarity {

    /// True when both prototypes carry at least one example — the classifier can render a verdict.
    pub fn is_ready(&self) -> bool {
        self.bad_n > 0 && self.good_n > 0
    }

    /// Whether this classifier carries ANY of its own grounded evidence — prototype votes or
    /// side-count tallies. A language whose own reading produced evidence never reads through
    /// the transfer store: reality about THIS language outranks reality about others.
    pub fn has_evidence(&self) -> bool {
        self.votes() > 0 || !self.tallies.is_empty()
    }

    /// Classify prose: `Some(true)` = prohibition, `Some(false)` = endorsement, `None` = abstain
    /// (untrained, unencodable, or no decisive majority). The reading is **words → sentences**
    /// (LINTER.md, "The side-count evidence layer"): the per-token side-count tallies vote first
    /// — each word's own grounded history — and only when they cannot decide do the span
    /// prototypes vote. Prototype votes go to whichever prototype the token sits closer to by
    /// more than the CALIBRATED margin ([`calibrated_margin`] — the trained prototypes' own
    /// measured noise floor, never a hand constant), and every vote in both layers weighs the
    /// token's information content ([`Reader::info_bits`]): common register words barely count,
    /// rare words decide — so neutral manual prose cannot become law off filler vocabulary. The
    /// side with the strictly heavier vote wins; callers keep document order as their own final
    /// fallback. Per-token voting keeps the call stable whether the prose is three words or
    /// three hundred — neutral text simply casts no votes and abstains.
    pub fn classify(&self, prose: &str) -> Option<bool> {
        if !self.is_ready() {
            // Unready ≠ unread: a language whose docs never pair violations with fixes
            // (reference manuals) trains no good PROTOTYPE, but its grounded tallies are
            // rich — and they, not the crude negation floor, are the evidence. The floor
            // (LINTER.md, "The side-count evidence layer") decides only when the tallies
            // cannot: truly cold reading. Prohibition only — reality alone can endorse.
            if let Some(v) = self.classify_tallied(prose) {
                return Some(v);
            }
            return self.classify_negation(prose);
        }
        if let Some(v) = self.classify_tallied(prose) {
            return Some(v);
        }
        let margin = *self.margin.get_or_init(|| calibrated_margin(&self.bad, &self.good));
        let mut bad_votes = 0u64;
        let mut good_votes = 0u64;
        for (h, w) in self.reader.salient_weighted(prose) {
            let db = h.distance(&self.bad);
            let dg = h.distance(&self.good);
            if db + margin <= dg {
                bad_votes += u64::from(w);
            } else if dg + margin <= db {
                good_votes += u64::from(w);
            }
        }
        match bad_votes.cmp(&good_votes) {
            std::cmp::Ordering::Greater => Some(true),
            std::cmp::Ordering::Less => Some(false),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// The prose hypervector this classifier assigns to `prose` — its reader's content-token bundle.
    /// The prose side of an association [`Binding`]. `None` when the prose carries no content token.
    pub fn prose_hv(&self, prose: &str) -> Option<Hv> {
        self.reader.encode(prose)
    }

    /// The decisive lean of ONE token: `Some(true)` when it sits inside the prohibition
    /// prototype's calibrated margin, `Some(false)` for the endorsement side, `None` when it
    /// belongs to neither. This is the classifier's per-word view — the primitive that lets a
    /// caller read polarity ALONG a text (which words forbid, which endorse) instead of chopping
    /// the text into pieces and classifying the pieces.
    pub fn token_lean(&self, token: &str) -> Option<bool> {
        if !self.is_ready() {
            return None;
        }
        // Words before geometry: the token's own grounded side-count history is direct
        // evidence; prototype distance is the fallback when the tallies never met it.
        if let Some(v) = self.tally_lean(token) {
            return Some(v);
        }
        let margin = *self.margin.get_or_init(|| calibrated_margin(&self.bad, &self.good));
        let h = crate::lint_ai::token_hv(&token.to_lowercase());
        let (db, dg) = (h.distance(&self.bad), h.distance(&self.good));
        if db + margin <= dg {
            Some(true)
        } else if dg + margin <= db {
            Some(false)
        } else {
            None
        }
    }

    /// The reader this classifier carries — the learned word-frequency knowledge other layers
    /// (e.g. grounded construct selection in [`crate::lint_match`]) consult.
    pub fn reader(&self) -> &Reader {
        &self.reader
    }
}

// ── Association memory (reading IS the knowledge) ─────────────────────────────

/// One read unit: a documentation prose snippet bound to the code example it governs, plus the
/// provenance to reconstruct a rule from it. `bind` = prose_hv ⊗ code_hv is the associative key — the
/// hypervector that says "this explanation goes with this code shape".
#[derive(Clone, Serialize, Deserialize)]
pub struct Binding {
    /// The page the pair was read from (rule id source and citation).
    pub url: String,
    /// The slug (rule id) for the page/section.
    pub slug: String,
    /// The governing prose — the docs' own words about the code.
    pub prose: String,
    /// The code example the prose governs.
    pub code: String,
    /// prose_hv ⊗ code_hv — the bound association.
    pub bind: Hv,
}

impl Binding {
    /// Bind `prose` to `code` under `lang`, using `polarity`'s reader for the prose side and the
    /// code's structural n-grams for the code side. `None` when the prose has no content token.
    pub fn form(lang: &str, url: &str, slug: &str, prose: &str, code: &str, polarity: &Polarity) -> Option<Binding> {
        let prose_hv = polarity.prose_hv(prose)?;
        let code_hv = code_hv(lang, code);
        Some(Binding {
            url: url.to_string(),
            slug: slug.to_string(),
            prose: prose.to_string(),
            code: code.to_string(),
            bind: prose_hv.xor(&code_hv),
        })
    }
}

/// The structural hypervector of a code example: the majority bundle of its node-kind path trigrams
/// (AST when a grammar exists) or token trigrams — a shape fingerprint, not its text. `Hv::zero()`
/// for code with no extractable n-gram (so a binding is still well-defined).
pub fn code_hv(lang: &str, code: &str) -> Hv {
    let mut b = Bundler::new();
    for ng in crate::lint_match::code_ngrams(lang, code) {
        b.add(&token_hv(&ng));
    }
    if b.is_empty() { Hv::zero() } else { b.finalize() }
}

/// Everything the reader took away from reading a language's docs: the bound (prose, code) units, a
/// reference corpus of real code, and the trained polarity classifier (which carries the reader).
/// This is the serialized artifact that means "the model read the docs" — a warm run loads it and
/// never re-crawls; expanding what is understood is more bindings (more reading), never code.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Memory {
    /// Every prose⊗code association read from the docs.
    pub bindings: Vec<Binding>,
    /// Real idiomatic code the docs served — the "what's normal" corpus that calibrates the engine.
    #[serde(default)]
    pub reference: Vec<String>,
    /// The classifier read + toolchain-grounded during the crawl (carries the reader). `None` when no
    /// toolchain grounded the language, in which case no rule can be queried out.
    #[serde(default)]
    pub polarity: Option<Polarity>,
    /// How many documentation pages' prose the reader actually read — the witness that a read
    /// HAPPENED. A prose-only spec site (no code blocks anywhere) yields zero bindings and zero
    /// reference, yet the language was read and is set up (LINTER.md, "reading IS the module").
    #[serde(default)]
    pub pages_read: usize,
    /// Token seeds of the example codes the TOOLCHAIN actually flagged during grounding — the
    /// reality-tested labels. A compiled detector may keep literal example tokens only when its
    /// example is in here or the law's own words name them (LINTER.md, ledger #19): a
    /// Clean-parsing example's identifiers are just code the docs showed, never evidence.
    #[serde(default, serialize_with = "sorted_u64_set")]
    pub flagged: std::collections::HashSet<u64>,
    /// The language's file-extension claims, learned from its own documentation (LINTER.md,
    /// "File types are learned by reading"): dot-led tokens tallied while reading, corpus-head
    /// words dropped. `{extension → mention count}`; empty when the docs never name a file.
    #[serde(default)]
    pub extensions: std::collections::BTreeMap<String, u32>,
}

/// Read a language's self-defined abbreviations out of its own prose: the parenthetical
/// definition "JavaScript (JS)" / "TypeScript (TS)" is how documentation introduces its own
/// short name, and that short name is how the world names the language's files. A found
/// abbreviation claims maximal strength (`u32::MAX`) in `tally` — the language's own docs
/// defining their own name outrank any mention count (LINTER.md, "File types are learned by
/// reading"). Pure typography over the registered name: no abbreviation list anywhere.
pub fn tally_name_aliases(lang: &str, text: &str, tally: &mut std::collections::BTreeMap<String, u32>) {
    let lower = text.to_lowercase();
    let name = lang.to_lowercase();
    let bytes = lower.as_bytes();
    let mut from = 0usize;
    while let Some(pos) = lower[from..].find(&name) {
        let start = from + pos;
        let end = start + name.len();
        from = end;
        let word_start = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let word_end = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if !word_start || !word_end {
            continue;
        }
        // Optional whitespace, then a parenthesized short run: `(js)`, `(YAML™)`.
        let mut i = end;
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'(' {
            continue;
        }
        let abbr_start = i + 1;
        let mut abbr_end = abbr_start;
        while abbr_end < bytes.len() && bytes[abbr_end].is_ascii_alphanumeric() {
            abbr_end += 1;
        }
        let abbr = &lower[abbr_start..abbr_end];
        // A definition, not a parenthetical remark: the run closes the parens itself
        // ("(JS)", never "(around four years)"), it is strictly shorter than the name, and —
        // ledger #20 — an abbreviation ABBREVIATES: its letters come FROM the name, first
        // letter anchored, in order ("js" ⊆ javascript, "ts" ⊆ typescript). Real docs write
        // "Python (REPL)", "Kotlin (J2K)", "Erlang (BEAM)" — true parentheticals that are
        // NOT the language's short name, and at maximal strength one such line dethroned
        // `.py` from python machine-wide.
        let abbreviates = abbr.len() >= 2
            && abbr.starts_with(&name[..1])
            && {
                let mut rest = name.as_bytes();
                abbr.bytes().all(|b| {
                    if let Some(i) = rest.iter().position(|&n| n == b) {
                        rest = &rest[i + 1..];
                        true
                    } else {
                        false
                    }
                })
            };
        if bytes.get(abbr_end) == Some(&b')')
            && abbr.len() < name.len()
            && abbr.bytes().any(|b| b.is_ascii_alphabetic())
            && abbreviates
        {
            tally.insert(abbr.to_string(), u32::MAX);
        }
    }
}

/// Tally every dot-led token of `text` into `tally` — the typographic half of learning a
/// language's file extensions from its docs (LINTER.md, "File types are learned by reading").
/// A claim candidate is `.` + a 1..=8-char alphanumeric run containing at least one letter,
/// closing its word (`main.rs`, ".py", `hello.kt -d hello.jar`). A run a call opener follows
/// is an invocation, never a filename (`console.log(x)`, `.println(...)`); version numbers
/// (".0") and longer runs never qualify. Pure typography: no vocabulary, no extension list.
pub fn tally_dotted_tokens(text: &str, tally: &mut std::collections::BTreeMap<String, u32>) {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'.' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
                end += 1;
            }
            let run = &text[start..end];
            let closes_word = end >= bytes.len() || !matches!(bytes[end], b'.' | b'_');
            let called = bytes.get(end).is_some_and(|b| matches!(b, b'(' | b'<' | b'{' | b'['));
            if (1..=8).contains(&run.len())
                && run.bytes().any(|b| b.is_ascii_alphabetic())
                && closes_word
                && !called
            {
                let n = tally.entry(run.to_ascii_lowercase()).or_default();
                *n = n.saturating_add(1);
            }
            i = end;
        } else {
            i += 1;
        }
    }
}

// ── HLM1 binary codecs (LINTER.md, "Save") ────────────────────────────────────
//
// Field order IS the wire order; hypervectors and the frequency arrays ride the RAW stream
// (bulk-copy decode), text rides the deflated DATA stream. A decoded reader's frequency table
// stays FROZEN — sorted arrays consulted by binary search — until something learns through it.

impl crate::lint_codec::Bin for Reader {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        let (keys, counts) = self.freq.sorted_pairs();
        e.raw_u64s(&keys);
        e.raw_u32s(&counts);
        e.u(self.total);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Reader> {
        let freq = FreqTable::frozen(d.raw_u64s()?, d.raw_u32s()?)?;
        let total = d.u()?;
        Some(Reader { mem: HashMap::new(), freq, total, head: std::sync::OnceLock::new() })
    }
}

impl crate::lint_codec::Bin for Polarity {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        self.reader.enc(e);
        e.hv(&self.bad);
        e.hv(&self.good);
        e.u(self.bad_n as u64);
        e.u(self.good_n as u64);
        // Side-count tallies ride the RAW stream as three parallel arrays (seed-sorted).
        e.raw_u64s(&self.tallies.iter().map(|(k, _, _)| *k).collect::<Vec<_>>());
        e.raw_u32s(&self.tallies.iter().map(|(_, f, _)| *f).collect::<Vec<_>>());
        e.raw_u32s(&self.tallies.iter().map(|(_, _, c)| *c).collect::<Vec<_>>());
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Polarity> {
        let reader = Reader::dec(d)?;
        let (bad, good) = (d.hv()?, d.hv()?);
        let (bad_n, good_n) = (d.u()? as usize, d.u()? as usize);
        // Format 2 always writes the three parallel arrays; format-1 artifacts never reach
        // this decoder (the container FORMAT byte rejects them at `open`).
        let (keys, f, c) = (d.raw_u64s()?, d.raw_u32s()?, d.raw_u32s()?);
        if keys.len() != f.len() || f.len() != c.len() {
            return None;
        }
        let tallies = keys.into_iter().zip(f).zip(c).map(|((k, f), c)| (k, f, c)).collect();
        Some(Polarity {
            reader,
            bad,
            good,
            bad_n,
            good_n,
            margin: std::sync::OnceLock::new(),
            tallies,
        })
    }
}

impl crate::lint_codec::Bin for Binding {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.url);
        e.str(&self.slug);
        e.str(&self.prose);
        e.str(&self.code);
        e.hv(&self.bind);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Binding> {
        Some(Binding { url: d.str()?, slug: d.str()?, prose: d.str()?, code: d.str()?, bind: d.hv()? })
    }
}

impl crate::lint_codec::Bin for Memory {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        self.bindings.enc(e);
        self.reference.enc(e);
        self.polarity.enc(e);
        e.u(self.pages_read as u64);
        self.flagged.enc(e);
        self.extensions.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Memory> {
        Some(Memory {
            bindings: Vec::dec(d)?,
            reference: Vec::dec(d)?,
            polarity: Option::dec(d)?,
            pages_read: d.u()? as usize,
            flagged: crate::lint_codec::Bin::dec(d)?,
            extensions: crate::lint_codec::Bin::dec(d)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A decoded reader is FROZEN — sorted arrays, binary-search lookups — and must answer
    /// exactly like the map it was encoded from: same counts, same head judgment, same
    /// info bits. The first write thaws it and learning continues without losing a count.
    #[test]
    fn frozen_reader_answers_like_the_map_and_thaws_on_write() {
        use crate::lint_codec::{kind, Bin as _, Dec, Enc};
        let mut original = Reader::new();
        original.learn_span("the the the quick quick fox reads documentation carefully");
        let mut e = Enc::new();
        original.enc(&mut e);
        let bytes = e.finish(kind::ENGLISH, "reader");
        let (_, mut d) = Dec::open(&bytes, kind::ENGLISH).expect("opens");
        let mut frozen = Reader::dec(&mut d).expect("decodes");

        assert_eq!(frozen.total_read(), original.total_read());
        for word in ["the", "quick", "fox", "documentation", "unseen"] {
            assert_eq!(frozen.read_count(word), original.read_count(word), "count of {word:?}");
            assert_eq!(frozen.info_bits(word), original.info_bits(word), "info bits of {word:?}");
            assert_eq!(frozen.is_head_word(word), original.is_head_word(word), "head judgment of {word:?}");
        }

        // Writing thaws: counts merge, nothing is lost.
        frozen.learn_span("fox fox");
        assert_eq!(frozen.read_count("fox"), original.read_count("fox") + 2);
        assert_eq!(frozen.read_count("the"), original.read_count("the"));
    }

    #[test]
    fn tokenizes_words_and_cjk_characters() {
        let t = tokens("Avoid this: 避けて");
        assert!(t.contains(&"avoid".to_string()));
        assert!(t.contains(&"this".to_string()));
        // The CJK run contributes its individual characters so phrasings share morphemes.
        assert!(t.iter().any(|c| c == "避"), "CJK chars are tokenized: {t:?}");
    }

    #[test]
    fn an_unseen_compound_is_comprehended_through_its_parts() {
        // Self-expanding vocabulary: "getUserName" was never given to anyone — the reader
        // deciphers it because case boundaries are typography, like whitespace.
        let t = tokens("call getUserName here");
        assert!(t.contains(&"getusername".to_string()), "the word itself is a token: {t:?}");
        for part in ["get", "user", "name"] {
            assert!(t.contains(&part.to_string()), "part {part} is read too: {t:?}");
        }
        // Acronym heads split off cleanly; digits are their own morpheme.
        let t = tokens("HTMLParser env2json");
        assert!(t.contains(&"html".to_string()) && t.contains(&"parser".to_string()), "{t:?}");
        assert!(t.contains(&"env".to_string()) && t.contains(&"json".to_string()), "{t:?}");
        // A plain word stays exactly one token — no decomposition invented.
        assert_eq!(tokens("simple"), vec!["simple".to_string()]);
        // And comprehension is measurable: a reader that has read plain prose about users and
        // names places the unseen compound's span near a sibling compound, not near noise.
        let mut r = Reader::new();
        for _ in 0..10 {
            r.learn_span("get the user name and set the user name for the account");
        }
        let a = r.encode("getUserName").expect("encodes");
        let b = r.encode("setUserName").expect("encodes");
        let noise = r.encode("zqxfluor").expect("encodes");
        assert!(a.distance(&b) < a.distance(&noise), "shared parts pull compounds together");
    }

    #[test]
    fn reader_learns_and_filters_predicted_tokens() {
        let mut r = Reader::new();
        // Read a repeated common phrase so its transitions become predictable.
        for _ in 0..3 {
            r.learn_span("the function returns the value of the input");
        }
        assert!(r.learned() > 0, "reader learned some context slots");
        // Encoding is deterministic and non-mutating.
        let a = r.encode("the function returns the value").unwrap();
        let b = r.encode("the function returns the value").unwrap();
        assert_eq!(a, b, "encode is deterministic");
        assert_eq!(r.learned(), r.clone().learned(), "encode did not mutate the reader");
    }

    #[test]
    fn classifies_prohibition_endorsement_and_abstains_on_neutral() {
        // Prototypes accumulated from reading, not a keyword list.
        let examples: &[(&str, bool)] = &[
            ("never do this; it breaks under concurrent load", true),
            ("this pattern is deprecated and unsafe", true),
            ("avoid mutating shared state directly here", true),
            ("this approach is fragile and error prone", true),
            ("passing a raw handle here leaks the resource", true),
            ("do not swallow the exception silently", true),
            ("this call is discouraged and will be removed", true),
            ("using a global here is dangerous and brittle", true),
            ("this blocks the thread and causes a deadlock", true),
            ("hard coding the path is unsafe and unportable", true),
            ("this obsolete method leaks memory badly", true),
            ("reusing the buffer here causes a race condition", true),
            ("prefer immutable data; this is the recommended way", false),
            ("this is the idiomatic and safe form to use", false),
            ("always validate input at the boundary, it is clean", false),
            ("this scales well and is the supported pattern", false),
            ("keep it small and explicit, which is correct", false),
            ("prefer composition and return errors explicitly", false),
            ("this canonical form is clear and well tested", false),
            ("document every public function for clarity", false),
            ("return early to keep the logic flat and readable", false),
            ("favor descriptive names over short abbreviations", false),
            ("this efficient approach is correct and maintainable", false),
            ("handle the result and close the resource cleanly", false),
        ];
        let p = Polarity::from_labeled(examples);
        assert_eq!(p.classify("never do this; it breaks"), Some(true), "prohibition → bad");
        assert_eq!(p.classify("prefer the recommended safe form"), Some(false), "endorsement → good");
        // A novel SENTENCE never seen verbatim still classifies bad, because its distinctive words
        // (deprecated / unsafe / leaks / fragile) were each read in DIFFERENT bad sentences — genuine
        // bag-of-words generalization, not memorization of a phrasing.
        assert_eq!(
            p.classify("a deprecated, unsafe helper that leaks and is fragile"),
            Some(true),
            "an unseen recombination of learned prohibition vocabulary classifies bad"
        );
        // Neutral, contentless prose abstains.
        assert_eq!(p.classify("the module has three sections and a table"), None, "neutral abstains");
    }

    #[test]
    fn random_vocabulary_abstains_for_any_training_size() {
        // The abstain margin must come from the TRAINED prototypes (their measured noise floor),
        // not a hand-tuned constant: whatever the training size or balance, prose made of tokens
        // that belong to neither side must cast no verdict. Tiny and imbalanced trainings have a
        // much wider noise floor than large ones — a fixed margin fails one or the other.
        let trainings: Vec<Vec<(&str, bool)>> = vec![
            // Tiny: one example per side.
            vec![("never do this dangerous thing", true), ("this is the recommended form", false)],
            // Imbalanced: many bad, two good.
            vec![
                ("never do this; it breaks under load", true),
                ("this pattern is deprecated and unsafe", true),
                ("avoid mutating shared state here", true),
                ("this approach is fragile and error prone", true),
                ("passing a raw handle leaks the resource", true),
                ("this obsolete call corrupts memory", true),
                ("this is the recommended supported approach", false),
                ("prefer immutable data for clean clarity", false),
            ],
        ];
        // Neutral sentences whose vocabulary is fully disjoint from every training example — a
        // token the model has actually read (even once) legitimately carries lean; abstention is
        // only owed to vocabulary that belongs to neither side.
        let neutral = [
            "quarterly ledger totals seventeen columns",
            "violin quartets rehearse beside harbors",
            "granite outcrops flank meadow trailheads",
            "recipes fold saffron into warm butter",
        ];
        for (i, training) in trainings.iter().enumerate() {
            let p = Polarity::from_labeled(training);
            for q in &neutral {
                assert_eq!(p.classify(q), None, "training set {i}: neutral {q:?} must abstain");
            }
            // Sweep a large deterministic sample of never-seen pseudo-words: a hand-picked constant
            // margin sits somewhere in this distribution's tail and eventually miscalls one; the
            // calibrated margin must clear the whole sweep.
            for w in 0..400 {
                let q = format!("zk{w}a zk{w}b zk{w}c zk{w}d zk{w}e");
                assert_eq!(p.classify(&q), None, "training set {i}: pseudo-words {q:?} must abstain");
            }
        }
    }


    #[test]
    fn untrained_polarity_reads_negation_and_nothing_else() {
        // One-sided training leaves the PROTOTYPES unusable — but the dictionary's negation
        // cluster still reads overt prohibition (LINTER.md, "The cold floor"): that is what
        // lets a fresh machine mint "Never use X" laws from an ungroundable language's docs.
        // Good side empty; enough one-sided READING for information weights to mean
        // anything (a three-token universe cannot weigh any word).
        let p = Polarity::from_labeled(&[
            ("colorful widgets everywhere on the mantle beside the brass clock", true),
            ("porcelain vases line the hallway shelves under warm afternoon light", true),
            ("the garden path curves gently past rosemary beds toward the gate", true),
        ]);
        assert!(!p.is_ready());
        assert_eq!(
            p.classify("never do this"),
            Some(true),
            "overt negation reads as prohibition through the dictionary alone"
        );
        assert_eq!(
            p.classify("the module has three sections"),
            None,
            "without negation an unready classifier abstains"
        );
    }

    #[test]
    fn code_hv_fingerprints_shape_not_text() {
        // Same construct family with different identifiers/values → near fingerprints (they share
        // the assign-a-list trigrams, differing only in leaf literal kinds); a structurally
        // different construct → much farther. Text never enters the fingerprint.
        let a = code_hv("python", "scores = [90, 85, 77]");
        let b = code_hv("python", "labels = [\"math\", \"sci\"]");
        let c = code_hv("python", "def f(x):\n    return x + 1");
        assert!(a.distance(&b) < a.distance(&c),
                "list-assign is nearer to list-assign than to a function def ({} vs {})",
                a.distance(&b), a.distance(&c));
    }

    #[test]
    fn binding_forms_and_memory_round_trips_through_json() {
        let polarity = Polarity::from_labeled(&[
            ("never do this dangerous thing", true),
            ("this is the recommended clean form", false),
        ]);
        let b = Binding::form("python", "https://d/r/no-x", "no-x", "never do this dangerous thing", "x = [1]", &polarity)
            .expect("content prose binds");
        assert_eq!(b.bind, polarity.prose_hv("never do this dangerous thing").unwrap().xor(&code_hv("python", "x = [1]")),
                   "bind is prose ⊗ code");
        let memory = Memory { bindings: vec![b], reference: vec!["ok = (1, 2)".into()], polarity: Some(polarity), pages_read: 1, extensions: Default::default(), flagged: Default::default() };
        let json = serde_json::to_string(&memory).expect("serializes");
        let back: Memory = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back.bindings.len(), 1);
        assert_eq!(back.bindings[0].slug, "no-x");
        assert_eq!(back.bindings[0].bind, memory.bindings[0].bind, "the association survives the round trip");
        assert_eq!(back.reference, memory.reference);
        // The classifier still works after the round trip (freq + prototypes serialized).
        assert_eq!(back.polarity.as_ref().unwrap().classify("never do this dangerous thing"), Some(true));
    }
}
