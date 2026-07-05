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
    freq: HashMap<u64, u32>,
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
        Reader { mem: HashMap::new(), freq: HashMap::new(), total: 0, head: std::sync::OnceLock::new() }
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
        self.freq.retain(|_, c| *c >= n);
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
            let mut counts: Vec<u32> = self.freq.values().copied().collect();
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

    /// How many times this reader has read `token` — 0 for a word it has never seen. The raw
    /// evidence rarity rankings are built from (a rarer word carries more of a sentence's
    /// identity).
    pub fn read_count(&self, token: &str) -> u32 {
        let seed = crate::lint_ai::token_seed(&token.to_lowercase());
        self.freq.get(&seed).copied().unwrap_or(0)
    }

    /// Read a span sequentially and LEARN from it: at each token, when the memory's prediction for
    /// the current context is wrong (or absent), update only that one addressed slot to expect this
    /// token. Correct predictions touch nothing. Token frequencies are tallied alongside. This is the
    /// predictive-coding write path.
    pub fn learn_span(&mut self, text: &str) {
        let mut ctx = Hv::zero();
        for tok in tokens(text) {
            let seed = crate::lint_ai::token_seed(&tok);
            *self.freq.entry(seed).or_default() += 1;
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
}

impl PolarityBuilder {
    /// Start from a reader that has already read the corpus (so its learned stop-list applies to the
    /// prose it encodes).
    pub fn new(reader: Reader) -> PolarityBuilder {
        PolarityBuilder { reader, bad: Bundler::new(), good: Bundler::new() }
    }

    /// Fold one grounded example into the prototypes: every salient token of `prose` votes into the
    /// prohibition prototype (the toolchain flagged its code) or the endorsement one (it accepted it).
    /// Voting per token — not per sentence — keeps a distinctive word's signal from being washed out
    /// by two layers of majority, and each vote weighs the token's information content
    /// ([`Reader::info_bits`]) so the prototypes are centroids of MEANING, not of register filler.
    pub fn accumulate(&mut self, prose: &str, is_bad: bool) {
        let toks = self.reader.salient_weighted(prose);
        let target = if is_bad { &mut self.bad } else { &mut self.good };
        for (h, w) in &toks {
            target.add_weighted(h, *w);
        }
    }

    /// Freeze the prototypes and the reader into a [`Polarity`] classifier.
    pub fn build(self) -> Polarity {
        Polarity {
            bad: self.bad.finalize(),
            good: self.good.finalize(),
            bad_n: self.bad.len(),
            good_n: self.good.len(),
            margin: std::sync::OnceLock::new(),
            reader: self.reader,
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

    /// True when both prototypes carry at least one example — the classifier can render a verdict.
    pub fn is_ready(&self) -> bool {
        self.bad_n > 0 && self.good_n > 0
    }

    /// Classify prose: `Some(true)` = prohibition, `Some(false)` = endorsement, `None` = abstain
    /// (untrained, unencodable, or no decisive majority). Each salient token votes for whichever
    /// prototype it sits closer to by more than the CALIBRATED margin ([`calibrated_margin`] — the
    /// trained prototypes' own measured noise floor, never a hand constant), and its vote weighs
    /// its information content ([`Reader::info_bits`]): common register words barely count, rare
    /// words decide — so neutral manual prose cannot become law off filler vocabulary. The side
    /// with the strictly heavier vote wins. Per-token voting keeps the call stable whether the
    /// prose is three words or three hundred — neutral text simply casts no votes and abstains.
    pub fn classify(&self, prose: &str) -> Option<bool> {
        if !self.is_ready() {
            return None;
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
        // ("(JS)", never "(around four years)") and an abbreviation ABBREVIATES — it is
        // strictly shorter than the name (which also makes single-letter names like "c"
        // alias-free: they resolve as themselves).
        if bytes.get(abbr_end) == Some(&b')')
            && abbr.len() >= 2
            && abbr.len() < name.len()
            && abbr.bytes().any(|b| b.is_ascii_alphabetic())
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn untrained_polarity_abstains() {
        let p = Polarity::from_labeled(&[("never do this", true)]); // good side empty
        assert!(!p.is_ready());
        assert_eq!(p.classify("never do this"), None, "one-sided training cannot classify");
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
        let memory = Memory { bindings: vec![b], reference: vec!["ok = (1, 2)".into()], polarity: Some(polarity), pages_read: 1, extensions: Default::default() };
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
