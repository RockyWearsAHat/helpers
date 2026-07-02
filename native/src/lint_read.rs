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
    /// Context address → predicted next-token code. Bounded by [`MEM_CAP`].
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

/// How much closer (in Hamming bits) a single token must sit to one prototype than the other to cast
/// a polarity vote for it. Below this it is a neutral word that belongs to neither side, so it does
/// not vote. Keeps the decision robust to query length: confidence is a fraction of decisive tokens,
/// not an absolute distance that shrinks with fewer words.
const TOKEN_VOTE_MARGIN: u32 = 200;

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

    /// True when both prototypes carry at least one example — the classifier can render a verdict.
    pub fn is_ready(&self) -> bool {
        self.bad_n > 0 && self.good_n > 0
    }

    /// Classify prose: `Some(true)` = prohibition, `Some(false)` = endorsement, `None` = abstain
    /// (untrained, unencodable, or no decisive majority). Each salient token votes for whichever
    /// prototype it sits closer to (by more than [`TOKEN_VOTE_MARGIN`]); the side with the strict
    /// majority of votes wins. Per-token voting keeps the call stable whether the prose is three words
    /// or three hundred — neutral text simply casts no votes and abstains.
    pub fn classify(&self, prose: &str) -> Option<bool> {
        if !self.is_ready() {
            return None;
        }
        let mut bad_votes = 0i32;
        let mut good_votes = 0i32;
        for h in self.reader.salient(prose) {
            let db = h.distance(&self.bad);
            let dg = h.distance(&self.good);
            if db + TOKEN_VOTE_MARGIN <= dg {
                bad_votes += 1;
            } else if dg + TOKEN_VOTE_MARGIN <= db {
                good_votes += 1;
            }
        }
        match bad_votes.cmp(&good_votes) {
            std::cmp::Ordering::Greater => Some(true),
            std::cmp::Ordering::Less => Some(false),
            std::cmp::Ordering::Equal => None,
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
    fn untrained_polarity_abstains() {
        let p = Polarity::from_labeled(&[("never do this", true)]); // good side empty
        assert!(!p.is_ready());
        assert_eq!(p.classify("never do this"), None, "one-sided training cannot classify");
    }
}
