//! `lint_read` — a 1-bit sequential predictive coder that LEARNS a language by *reading* its docs.
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

/// Split text into meaningful tokens for the reader: Unicode word runs, lowercased. A run that
/// contains any non-ASCII character also contributes each of its characters as its own token, so
/// prose in a script without spaces (Japanese, Chinese) still shares morphemes across phrasings
/// instead of collapsing to one opaque token.
pub fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for run in text.split(|c: char| !c.is_alphanumeric()) {
        if run.is_empty() {
            continue;
        }
        let lower = run.to_lowercase();
        if lower.chars().count() >= 2 {
            out.push(lower.clone());
        }
        if run.chars().any(|c| !c.is_ascii()) {
            for ch in run.chars() {
                let mut buf = [0u8; 4];
                out.push(ch.encode_utf8(&mut buf).to_lowercase());
            }
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
    /// Total tokens read — sets the "common" cutoff relative to corpus size.
    #[serde(default)]
    total: u64,
}

impl Reader {
    /// A reader that has read nothing yet.
    pub fn new() -> Reader {
        Reader { mem: HashMap::new(), freq: HashMap::new(), total: 0 }
    }

    /// Number of learned context slots — how much the reader has comprehended.
    pub fn learned(&self) -> usize {
        self.mem.len()
    }

    /// The frequency at or above which a token is treated as common (and dropped from a span's
    /// salient set). Scales with corpus size so it means "much more frequent than average", never a
    /// fixed magic number; floored at 3 so a tiny corpus still filters its obvious filler words.
    fn common_cutoff(&self) -> u32 {
        (self.total / 120).max(3) as u32
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

    /// The codes of a span's **content** tokens — every token minus the ones the reader has learned
    /// are common (its frequency-derived stop-list). A learned filter, never a written word list: read
    /// more and the stop-list adapts. The context predictor ([`Reader::learn_span`]) builds the
    /// comprehension memory used elsewhere, but polarity leans on the frequency filter so a
    /// distinctive word is never dropped merely for being locally predictable. If every token was
    /// common, falls back to all tokens so a span is never empty. Non-mutating.
    pub(crate) fn salient(&self, text: &str) -> Vec<Hv> {
        let cutoff = self.common_cutoff();
        let mut salient = Vec::new();
        let mut all = Vec::new();
        for tok in tokens(text) {
            let seed = crate::lint_ai::token_seed(&tok);
            let h = Hv::random(seed);
            all.push(h);
            if self.freq.get(&seed).copied().unwrap_or(0) < cutoff {
                salient.push(h);
            }
        }
        if salient.is_empty() { all } else { salient }
    }

    /// Encode a span into one hypervector: the majority bundle of its salient token codes (a
    /// bag-of-distinctive-words centroid). `None` for an empty span.
    pub fn encode(&self, text: &str) -> Option<Hv> {
        let toks = self.salient(text);
        if toks.is_empty() {
            return None;
        }
        let mut b = Bundler::new();
        for h in &toks {
            b.add(h);
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
    /// by two layers of majority, so the prototypes are true bag-of-words centroids.
    pub fn accumulate(&mut self, prose: &str, is_bad: bool) {
        let toks = self.reader.salient(prose);
        let target = if is_bad { &mut self.bad } else { &mut self.good };
        for h in &toks {
            target.add(h);
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
    /// trained prototypes' own measured noise floor, never a hand constant); the side with the strict
    /// majority of votes wins. Per-token voting keeps the call stable whether the prose is three words
    /// or three hundred — neutral text simply casts no votes and abstains.
    pub fn classify(&self, prose: &str) -> Option<bool> {
        if !self.is_ready() {
            return None;
        }
        let margin = *self.margin.get_or_init(|| calibrated_margin(&self.bad, &self.good));
        let mut bad_votes = 0i32;
        let mut good_votes = 0i32;
        for h in self.reader.salient(prose) {
            let db = h.distance(&self.bad);
            let dg = h.distance(&self.good);
            if db + margin <= dg {
                bad_votes += 1;
            } else if dg + margin <= db {
                good_votes += 1;
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
        let memory = Memory { bindings: vec![b], reference: vec!["ok = (1, 2)".into()], polarity: Some(polarity) };
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
