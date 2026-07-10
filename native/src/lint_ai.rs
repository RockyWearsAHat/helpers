//! `lint_ai` — two systems sharing one hypervector (Hv) substrate.
//!
//! ## Memory subsystem
//! 8192-bit hypervectors (`Hv`) with XOR binding and Hamming-distance retrieval.
//! Used by `memory/embed`, `memory/retriever`, `memory/store`, and the crawler.
//!
//! ## Concept confirmation gate (`ConceptModel`)
//!
//! The linter's *firing* engine is [`crate::lint_match::RuleSet`]: each documented rule
//! compiles to a lossless AST sub-tree pattern (or, for grammarless languages, a
//! discriminating token regex). That engine decides *whether* a rule fires and on which
//! line — precisely, with no statistics.
//!
//! `ConceptModel` no longer fires anything. It is a **confirmation gate** the live lint path
//! applies to the imprecise findings — the token-regex fallbacks used for grammarless languages
//! and description-derived rules (precise AST matches are exact and report directly). A rule's
//! `rule_hv` is the bundled hypervector of every token in its English description (dictionary
//! words weighted 2×) and its documented example. When a text-fallback rule fires on a construct,
//! [`ConceptModel::confirms`] bundles that whole construct's tokens (node level — never a single
//! leaf) and keeps the finding only when the fired rule's fingerprint is the concept the
//! construct is closest to. A regex that hit a token belonging more to some *other* rule is
//! incidental and dropped.
//!
//! This is why there is no per-token / all-rules free-firing here anymore, and why the
//! former enumerated inference blocklists (a DF stop set, a dictionary-word filter, the
//! keyword whitelist) are gone from the inference path: the node-level bundle plus the
//! nearest-concept comparison is a structural signal, not a hand-maintained word list.
//!
//! ### Getting started
//! No arguments required. On first `lint` the rule set trains automatically from the
//! committed `lint-index/` catalogs and project `.helpers/lint-rules/`, then runs. To add
//! a language: `lint_add_source` then `lint_learn`; to add project rules: drop a `*.md`
//! in `.helpers/lint-rules/`.

use std::collections::HashSet;
use std::sync::OnceLock;

// ── Hypervector substrate ─────────────────────────────────────────────────────

/// Hypervector width in bits. 8192 bits — near-orthogonal random codes.
pub const DIM: usize = 8192;
const WORDS: usize = DIM / 64;

/// A `DIM`-bit binary hypervector. The one and only representation the memory engine uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Hv(#[serde(with = "hv_serde")] [u64; WORDS]);

impl Hv {
    pub fn zero() -> Hv { Hv([0; WORDS]) }
    pub fn as_words(&self) -> &[u64] { &self.0 }

    pub fn from_words(words: &[u64]) -> Hv {
        let mut w = [0u64; WORDS];
        for (slot, v) in w.iter_mut().zip(words.iter()) { *slot = *v; }
        Hv(w)
    }

    /// Deterministic pseudo-random vector for `seed` — the codebook entry for any token.
    pub fn random(seed: u64) -> Hv {
        let mut s = seed ^ 0xA0761D6478BD642F;
        let mut w = [0u64; WORDS];
        for word in w.iter_mut() { *word = splitmix64(&mut s); }
        Hv(w)
    }

    pub fn xor(&self, other: &Hv) -> Hv {
        let mut w = [0u64; WORDS];
        for (out, (a, b)) in w.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            *out = a ^ b;
        }
        Hv(w)
    }

    /// Hamming distance — 0 = identical, ~DIM/2 = unrelated.
    pub fn distance(&self, other: &Hv) -> u32 {
        self.0.iter().zip(other.0.iter()).map(|(a, b)| (a ^ b).count_ones()).sum()
    }

    pub fn rotl1_pub(&self) -> Hv { self.rotl1() }

    fn rotl1(&self) -> Hv {
        let mut w = [0u64; WORDS];
        let top = self.0[WORDS - 1] >> 63;
        for (i, out) in w.iter_mut().enumerate() {
            let carry_in = if i == 0 { top } else { self.0[i - 1] >> 63 };
            *out = (self.0[i] << 1) | carry_in;
        }
        Hv(w)
    }

    fn rotate(&self, k: usize) -> Hv {
        let mut v = *self;
        for _ in 0..(k % DIM) { v = v.rotl1(); }
        v
    }
}

mod hv_serde {
    //! Hypervector wire format: one fixed-width hex string (`WORDS × 16` chars). A learned
    //! model is mostly hypervectors, so this decides cache size and parse speed — the JSON
    //! number-array of 128 u64s cost ~30% more bytes and far more parse work (measured on the
    //! multi-MB concept models every warm run loads). Reading still accepts the legacy array
    //! form, so existing caches and published registry catalogs stay valid.
    use super::WORDS;
    use serde::{Deserializer, Serializer, de::SeqAccess, de::Visitor};

    pub fn serialize<S: Serializer>(arr: &[u64; WORDS], s: S) -> Result<S::Ok, S::Error> {
        use std::fmt::Write as _;
        let mut out = String::with_capacity(WORDS * 16);
        for v in arr.iter() {
            let _ = write!(out, "{v:016x}");
        }
        s.serialize_str(&out)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u64; WORDS], D::Error> {
        struct Vis;
        impl<'de> Visitor<'de> for Vis {
            type Value = [u64; WORDS];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a {}-char hex string or an array of {WORDS} u64 values", WORDS * 16)
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
                let mut arr = [0u64; WORDS];
                for (slot, chunk) in arr.iter_mut().zip(s.as_bytes().chunks(16)) {
                    let hex = std::str::from_utf8(chunk).map_err(E::custom)?;
                    *slot = u64::from_str_radix(hex, 16).map_err(E::custom)?;
                }
                Ok(arr)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut arr = [0u64; WORDS];
                for slot in arr.iter_mut() { *slot = seq.next_element()?.unwrap_or(0); }
                Ok(arr)
            }
        }
        d.deserialize_any(Vis)
    }
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// FNV-1a hash of a token string — the seed that maps any token to its code.
pub fn token_seed(token: &str) -> u64 {
    let mut h = 0xCBF29CE484222325u64;
    for b in token.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001B3);
    }
    h
}

/// The code for a single token: universal, language-agnostic.
pub fn token_hv(token: &str) -> Hv { Hv::random(token_seed(token)) }

/// Encode a token window into one vector via position rotation + majority bundling.
pub fn bind(tokens: &[&str]) -> Hv {
    let mut b = Bundler::new();
    for (i, t) in tokens.iter().enumerate() {
        b.add(&token_hv(t).rotate(i));
    }
    b.finalize()
}

/// Per-bit majority vote accumulator — bundles vectors into one prototype.
pub struct Bundler {
    counts: Vec<i32>,
    n: usize,
}

impl Bundler {
    pub fn new() -> Bundler { Bundler { counts: vec![0; DIM], n: 0 } }

    pub fn add(&mut self, hv: &Hv) {
        self.add_weighted(hv, 1);
    }

    /// Bundle `hv` with `weight` votes — the weighted form majority bundling reduces to when
    /// every vector matters equally. Callers weight by information content (a rare word's code
    /// counts for more of the prototype than a near-stop-word's).
    pub fn add_weighted(&mut self, hv: &Hv, weight: u32) {
        let w = weight as i32;
        for bit in 0..DIM {
            let set = (hv.0[bit / 64] >> (bit % 64)) & 1 == 1;
            self.counts[bit] += if set { w } else { -w };
        }
        self.n += weight as usize;
    }

    pub fn len(&self) -> usize { self.n }
    pub fn is_empty(&self) -> bool { self.n == 0 }

    pub fn finalize(&self) -> Hv {
        let mut w = [0u64; WORDS];
        for bit in 0..DIM {
            if self.counts[bit] > 0 { w[bit / 64] |= 1 << (bit % 64); }
        }
        Hv(w)
    }
}

impl Default for Bundler { fn default() -> Self { Bundler::new() } }

/// UNWEIGHTED majority bundle over word-wise bit-sliced counters — bit-identical to
/// `Bundler::add`×k + `finalize` (ties at even k fall to 0, same as `count > 0` there),
/// with no per-bit loop: adds ripple through ⌈log₂(k+1)⌉ carry planes and the threshold
/// compare is a word-wise subtractor. Measured: the per-bit `Bundler` charged ~8µs per
/// add; this is ~0.2µs — the gate's query building was the batch's real cost, not the
/// popcounts. `None` when `vecs` is empty (mirrors the empty-bundle abstain).
pub fn majority_bundle(vecs: &[&Hv]) -> Option<Hv> {
    if vecs.is_empty() {
        return None;
    }
    let k = vecs.len();
    let planes_n = usize::BITS as usize - k.leading_zeros() as usize; // ⌈log₂(k+1)⌉
    let mut planes = vec![[0u64; WORDS]; planes_n];
    for v in vecs {
        for w in 0..WORDS {
            // Bit-sliced increment: add 1 (the vector's bit) with ripple carry.
            let mut carry = v.0[w];
            for plane in planes.iter_mut() {
                let t = plane[w] & carry;
                plane[w] ^= carry;
                carry = t;
                if carry == 0 {
                    break;
                }
            }
        }
    }
    // Majority: bit set iff count ≥ T with T = k/2 + 1 (strictly more than half — the
    // exact `counts > 0` rule of the ±1 Bundler). Word-wise subtract: borrow of (s − T);
    // s ≥ T ⇔ no final borrow.
    let t_const = k / 2 + 1;
    let mut out = [0u64; WORDS];
    for w in 0..WORDS {
        let mut borrow = 0u64;
        for (i, plane) in planes.iter().enumerate() {
            let tb = if (t_const >> i) & 1 == 1 { u64::MAX } else { 0 };
            let s = plane[w];
            // full subtractor: borrow_out = (!s & (tb | borrow)) | (tb & borrow)
            let new_borrow = (!s & (tb | borrow)) | (tb & borrow);
            borrow = new_borrow;
        }
        out[w] = !borrow; // no borrow ⇒ s ≥ T ⇒ majority bit set
    }
    Some(Hv(out))
}

// ── Hv-based concept confirmation gate (ConceptModel) ────────────────────────

// ── LangBrain: dictionary-grounded English understanding ─────────────────────

/// English word set from `/usr/share/dict/words`. Loaded once at first use.
/// Used only when *building* fingerprints, to give real English words extra weight so a
/// rule's concept leans on the words that carry its meaning; never used to filter tokens
/// at inference time.
pub(crate) fn dict_words() -> &'static HashSet<String> {
    static DICT: OnceLock<HashSet<String>> = OnceLock::new();
    DICT.get_or_init(|| {
        std::fs::read_to_string("/usr/share/dict/words")
            .map(|s| s.lines()
                .map(|l| l.trim().to_lowercase())
                .filter(|w| w.len() >= 3)
                .collect())
            .unwrap_or_default()
    })
}

// ── ConceptModel ─────────────────────────────────────────────────────────────

/// One compiled rule's concept fingerprint for the confirmation gate.
///
/// `rule_hv` is the bundle of every token in the rule's English description (dictionary
/// words weighted 2×) and its documented example — "what this rule is about", as one Hv.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CompiledRule {
    /// FNV hash of the rule id (the key the gate is queried by).
    pub id_hash: u64,
    /// Concept fingerprint: bundle of description dictionary tokens (semantic layer,
    /// weighted 2×) + all alphanumeric tokens from the documented example.
    pub rule_hv: Hv,
}

/// The Hv concept gate. Compiled from the same documented rules the
/// [`crate::lint_match::RuleSet`] compiles and cached beside the pattern model (same stamp);
/// it fires nothing on its own — it only confirms or rejects the firing engine's imprecise
/// (text-fallback) findings.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConceptModel {
    /// One fingerprint per rule that carried learnable tokens.
    pub rules: Vec<CompiledRule>,
}

impl ConceptModel {
    /// Compile rules into concept fingerprints.
    ///
    /// Each rule `(id, description, example)` produces one `rule_hv` via majority bundling of
    /// its tokens; dictionary words are added twice so the English meaning of the rule
    /// dominates over incidental identifiers. Rules with no usable tokens are skipped.
    pub fn compile(rules: &[(String, String, String)], _lang: &str) -> ConceptModel {
        let dict = dict_words();
        let mut compiled = Vec::new();
        let mut seen = HashSet::new();
        for (id, description, example) in rules {
            let mut b = Bundler::new();
            for tok in format!("{} {}", description, example)
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            {
                let t = tok.to_lowercase();
                if t.len() < 2 || t.len() > 64 { continue; }
                b.add(&token_hv(&t));
                // 2× weight for dictionary words: English semantics reinforce code signal.
                if t.len() >= 3 && dict.contains(&t) { b.add(&token_hv(&t)); }
            }
            if b.is_empty() { continue; }
            let id_hash = token_seed(id);
            if !seen.insert(id_hash) { continue; }
            compiled.push(CompiledRule { id_hash, rule_hv: b.finalize() });
        }
        ConceptModel { rules: compiled }
    }

    /// Confirm a single text-fallback finding — the 1-query case of [`ConceptModel::confirms_batch`].
    pub fn confirms(&self, rule_id: &str, tokens: &[&str]) -> bool {
        self.confirms_batch(&[(rule_id, tokens.to_vec())])[0]
    }

    /// Confirm a batch of text-fallback findings in ONE gate query ([`crate::hv_batch::gate`]).
    ///
    /// Each item is `(fired rule id, tokens of the whole matched construct)` — node level, the
    /// line/statement the regex fired on, never one leaf. Per item, the bundle of the construct's
    /// tokens is compared to every rule's fingerprint and the finding is kept only when the fired
    /// rule's fingerprint is the concept the construct is closest to (ties keep it): a construct
    /// whose tokens belong more to some *other* rule matched this rule's regex only incidentally,
    /// so it is rejected. When the model has no fingerprint for the fired rule, or the construct
    /// has no usable tokens, the gate abstains and keeps the finding — it never manufactures a
    /// rejection it cannot justify. Batching the whole run's findings into one call lets the
    /// popcount grid run as a single dispatch (GPU when the workload warrants it), bit-identical
    /// to the scalar decision.
    pub fn confirms_batch(&self, items: &[(&str, Vec<&str>)]) -> Vec<bool> {
        let keys: Vec<Hv> = self.rules.iter().map(|r| r.rule_hv).collect();
        let mut verdicts = vec![true; items.len()]; // abstain = keep
        let mut queries: Vec<Hv> = Vec::new();
        let mut fired: Vec<usize> = Vec::new();
        let mut batch_slots: Vec<usize> = Vec::new(); // items[i] behind each queries row
        for (i, (rule_id, tokens)) in items.iter().enumerate() {
            let target = token_seed(rule_id);
            let Some(fired_idx) = self.rules.iter().position(|r| r.id_hash == target) else {
                continue; // unknown rule → abstain
            };
            let hvs: Vec<Hv> = tokens
                .iter()
                .filter_map(|t| {
                    let t = t.to_lowercase();
                    (t.len() >= 2 && t.len() <= 64).then(|| token_hv(&t))
                })
                .collect();
            let refs: Vec<&Hv> = hvs.iter().collect();
            let Some(q) = majority_bundle(&refs) else {
                continue; // no usable tokens → abstain
            };
            queries.push(q);
            fired.push(fired_idx);
            batch_slots.push(i);
        }
        // A tie is STATISTICAL (LINTER.md, "Hv concept gate"): one line can violate the
        // fired rule while legitimately containing another rule's territory (`var doubled =
        // eval(…)` put the eval concepts 85 bits nearer and winner-take-all killed a true
        // finding), so the fired rule loses only when the nearest concept beats it by more
        // than 3σ of the code geometry's distance noise — within that band the distances
        // are indistinguishable and the finding is kept.
        let tie_band = (3.0 * (DIM as f64).sqrt() / 2.0) as u32;
        for (row, slot) in crate::hv_batch::gate(&queries, &keys, &fired).into_iter().zip(batch_slots) {
            verdicts[slot] = row.fired_dist <= row.nearest_dist.saturating_add(tie_band);
        }
        verdicts
    }

    /// Number of compiled concept fingerprints.
    pub fn rule_count(&self) -> usize { self.rules.len() }

    /// Merge two gates, `first` outranking `second` (same trust-order first-wins as
    /// [`crate::lint_match::RuleSet::merged`]): one fingerprint per rule id.
    pub fn merged(first: ConceptModel, second: ConceptModel) -> ConceptModel {
        let mut seen = std::collections::HashSet::new();
        let mut rules = Vec::new();
        for r in first.rules.into_iter().chain(second.rules) {
            if seen.insert(r.id_hash) {
                rules.push(r);
            }
        }
        ConceptModel { rules }
    }
}

// ── HLM1 binary codecs (LINTER.md, "Save") ────────────────────────────────────

impl crate::lint_codec::Bin for Hv {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.hv(self);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Hv> {
        d.hv()
    }
}

impl crate::lint_codec::Bin for CompiledRule {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.fixed_u64(self.id_hash);
        e.hv(&self.rule_hv);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<CompiledRule> {
        Some(CompiledRule { id_hash: d.fixed_u64()?, rule_hv: d.hv()? })
    }
}

impl crate::lint_codec::Bin for ConceptModel {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        self.rules.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<ConceptModel> {
        Some(ConceptModel { rules: Vec::dec(d)? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_is_self_inverse() {
        let a = token_hv("foo");
        let b = token_hv("bar");
        assert_eq!(a.xor(&b).xor(&b), a);
    }

    #[test]
    fn distinct_tokens_near_orthogonal() {
        let a = token_hv("unwrap");
        let b = token_hv("expect");
        assert_eq!(token_hv("unwrap").distance(&a), 0);
        let d = a.distance(&b);
        assert!((3500..4700).contains(&d), "distance {d} not near DIM/2");
    }

    #[test]
    fn bind_is_order_sensitive() {
        assert_ne!(bind(&["a", "==", "true"]), bind(&["true", "==", "a"]));
    }

    #[test]
    fn concept_model_compiles_from_example_text() {
        // 3-tuple: (id, description, example). Signal = description + example tokens.
        let rules = vec![(
            "no-var".to_string(),
            "avoid var; prefer let or const for block scoping".to_string(),
            "var x = 1; var count = 42; var y = true;".to_string(),
        )];
        let model = ConceptModel::compile(&rules, "javascript");
        assert_eq!(model.rule_count(), 1);
    }

    #[test]
    fn confirms_keeps_the_nearest_concept_and_abstains_when_unknown() {
        let rules = vec![
            (
                "no-eval".to_string(),
                "avoid eval; it executes arbitrary code".to_string(),
                "eval(userInput)".to_string(),
            ),
            (
                "no-with".to_string(),
                "avoid the with statement; it confuses scope".to_string(),
                "with (obj) { x = 1 }".to_string(),
            ),
        ];
        let model = ConceptModel::compile(&rules, "javascript");
        // A construct that is clearly about `eval` confirms the eval rule …
        assert!(model.confirms("no-eval", &["eval", "userInput"]));
        // … and the same construct does NOT confirm the unrelated `with` rule.
        assert!(!model.confirms("no-with", &["eval", "userInput"]));
        // An unknown rule id is abstained on (kept), never a manufactured rejection.
        assert!(model.confirms("not-a-rule", &["eval"]));
    }

    #[test]
    fn majority_bundle_is_bit_identical_to_the_bundler() {
        // The fast path must be a pure representation change: for every k (odd, even, 1,
        // large) the word-wise bit-sliced result equals Bundler::add×k + finalize exactly,
        // ties included.
        for k in [1usize, 2, 3, 4, 5, 8, 13, 40] {
            let vecs: Vec<Hv> = (0..k).map(|i| token_hv(&format!("tok{i}"))).collect();
            let mut b = Bundler::new();
            for v in &vecs {
                b.add(v);
            }
            let slow = b.finalize();
            let refs: Vec<&Hv> = vecs.iter().collect();
            let fast = majority_bundle(&refs).expect("non-empty");
            assert_eq!(slow.distance(&fast), 0, "k={k}: bit-sliced majority diverged");
        }
        assert!(majority_bundle(&[]).is_none(), "empty bundle abstains");
    }

    #[test]
    fn an_unseen_identifier_cannot_reject_a_true_finding() {
        // `var leaky = 2;` in a tiny project: "leaky" appears in no rule's reading, so its
        // random code drags the two-token bundle far from everything — but among concepts
        // of rules that can actually fire (LINTER.md, "Hv concept gate": detector-less
        // rules compile no fingerprint), the fired rule's own `var` mass must keep it the
        // nearest concept, whatever identifier sits beside it.
        let rules = vec![
            (
                "no_var_declaration".to_string(),
                "Never declare variables with var. Declare with const, or let when reassignment is needed.".to_string(),
                "var count = 1;".to_string(),
            ),
            (
                "no-eval".to_string(),
                "avoid eval; it executes arbitrary code".to_string(),
                "eval(userInput)".to_string(),
            ),
            (
                "no-with".to_string(),
                "avoid the with statement; it confuses scope".to_string(),
                "with (obj) { x = 1 }".to_string(),
            ),
        ];
        let model = ConceptModel::compile(&rules, "javascript");
        for ident in ["leaky", "zzqxv", "frobnicator", "blorp"] {
            assert!(
                model.confirms("no_var_declaration", &["var", ident]),
                "unseen identifier {ident:?} must not reject a construct the detector matched"
            );
        }
        // Evidence-based rejection still stands: a construct whose tokens a DIFFERENT
        // concept fully accounts for is rejected, noise floor or not.
        assert!(!model.confirms("no-with", &["eval", "userInput"]));
    }

    #[test]
    fn a_line_violating_one_rule_while_mentioning_anothers_territory_still_flags() {
        // `var doubled = eval("total * 2")`: the construct contains the eval concept's
        // territory, which can sit a few dozen bits nearer than the fired var rule — a
        // statistical tie, not evidence of an incidental hit. The fired rule must lose
        // only past the 3σ tie band (LINTER.md, "Hv concept gate").
        let rules = vec![
            (
                "no_var_declaration".to_string(),
                "Never declare variables with var. Declare with const, or let when reassignment is needed.".to_string(),
                "var count = 1;".to_string(),
            ),
            (
                "no_eval".to_string(),
                "Never call eval. It executes arbitrary strings as code. Parse the input explicitly instead.".to_string(),
                "const result = eval(expression);".to_string(),
            ),
        ];
        let model = ConceptModel::compile(&rules, "javascript");
        let line = ["var", "doubled", "eval", "total"];
        assert!(
            model.confirms("no_var_declaration", &line),
            "the var half of a double violation must survive the gate"
        );
        assert!(
            model.confirms("no_eval", &line),
            "the eval half of a double violation must survive the gate"
        );
    }

    #[test]
    fn batch_gate_matches_scalar_decisions() {
        let rules = vec![
            ("no-eval".to_string(), "avoid eval; it executes arbitrary code".to_string(), "eval(userInput)".to_string()),
            ("no-with".to_string(), "avoid the with statement; it confuses scope".to_string(), "with (obj) { x = 1 }".to_string()),
        ];
        let model = ConceptModel::compile(&rules, "javascript");
        let items: Vec<(&str, Vec<&str>)> = vec![
            ("no-eval", vec!["eval", "userInput"]),      // right concept → keep
            ("no-with", vec!["eval", "userInput"]),      // wrong concept → reject
            ("not-a-rule", vec!["eval"]),                 // unknown rule → abstain (keep)
            ("no-eval", vec![]),                          // no tokens → abstain (keep)
        ];
        let batch = model.confirms_batch(&items);
        let scalar: Vec<bool> = items.iter().map(|(id, t)| model.confirms(id, t)).collect();
        assert_eq!(batch, scalar, "one batched dispatch is bit-identical to the scalar loop");
        assert_eq!(batch, vec![true, false, true, true]);
    }
}
