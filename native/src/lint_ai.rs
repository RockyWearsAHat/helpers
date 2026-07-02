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
    use super::WORDS;
    use serde::{Deserializer, Serializer, de::SeqAccess, de::Visitor, ser::SerializeSeq};

    pub fn serialize<S: Serializer>(arr: &[u64; WORDS], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(WORDS))?;
        for v in arr.iter() { seq.serialize_element(v)?; }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u64; WORDS], D::Error> {
        struct Vis;
        impl<'de> Visitor<'de> for Vis {
            type Value = [u64; WORDS];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "an array of {WORDS} u64 values")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut arr = [0u64; WORDS];
                for slot in arr.iter_mut() { *slot = seq.next_element()?.unwrap_or(0); }
                Ok(arr)
            }
        }
        d.deserialize_seq(Vis)
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
        for bit in 0..DIM {
            let set = (hv.0[bit / 64] >> (bit % 64)) & 1 == 1;
            self.counts[bit] += if set { 1 } else { -1 };
        }
        self.n += 1;
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

// ── Hv-based concept confirmation gate (ConceptModel) ────────────────────────

// ── LangBrain: dictionary-grounded English understanding ─────────────────────

/// English word set from `/usr/share/dict/words`. Loaded once at first use.
/// Used only when *building* fingerprints, to give real English words extra weight so a
/// rule's concept leans on the words that carry its meaning; never used to filter tokens
/// at inference time.
fn dict_words() -> &'static HashSet<String> {
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
#[derive(Clone)]
pub struct CompiledRule {
    /// FNV hash of the rule id (the key the gate is queried by).
    pub id_hash: u64,
    /// Concept fingerprint: bundle of description dictionary tokens (semantic layer,
    /// weighted 2×) + all alphanumeric tokens from the documented example.
    pub rule_hv: Hv,
}

/// The Hv concept gate. Built in memory each lint run from the same documented rules the
/// [`crate::lint_match::RuleSet`] compiles; it fires nothing on its own — it only confirms
/// or rejects the firing engine's imprecise (text-fallback) findings.
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

    /// Confirm a text-fallback finding for `rule_id` against the `tokens` of the whole matched
    /// construct (node level — the line/statement the regex fired on, never one leaf).
    ///
    /// The bundle of the construct's tokens is compared to every rule's fingerprint. The finding
    /// is kept only when the fired rule's fingerprint is the concept the construct is closest to
    /// (ties keep it): a construct whose tokens belong more to some *other* rule matched this
    /// rule's regex only incidentally, so it is rejected. When the model has no fingerprint for
    /// `rule_id`, or the construct has no usable tokens, the gate abstains and keeps the finding —
    /// it never manufactures a rejection it cannot justify.
    pub fn confirms(&self, rule_id: &str, tokens: &[&str]) -> bool {
        let target = token_seed(rule_id);
        let Some(fired) = self.rules.iter().find(|r| r.id_hash == target) else { return true };
        let mut b = Bundler::new();
        for t in tokens {
            let t = t.to_lowercase();
            if t.len() < 2 || t.len() > 64 { continue; }
            b.add(&token_hv(&t));
        }
        if b.is_empty() { return true; }
        let node_hv = b.finalize();
        let fired_d = node_hv.distance(&fired.rule_hv);
        let nearest = self.rules.iter().map(|r| node_hv.distance(&r.rule_hv)).min().unwrap_or(fired_d);
        fired_d <= nearest
    }

    /// Number of compiled concept fingerprints.
    pub fn rule_count(&self) -> usize { self.rules.len() }
}

// ── Language keyword set (still used by memory/embed for token normalization) ──

/// Keywords and well-known built-ins that the memory subsystem's token normalizer preserves.
pub fn keywords() -> &'static std::collections::HashSet<&'static str> {
    static SET: OnceLock<std::collections::HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| [
        "if", "else", "elif", "for", "while", "do", "switch", "case", "default",
        "break", "continue", "return", "yield", "loop", "match", "defer", "goto",
        "select", "range", "then",
        "var", "let", "mut", "const", "static", "final",
        "fn", "func", "fun", "def", "function",
        "class", "struct", "enum", "interface", "trait", "type",
        "impl", "extends", "implements", "mod", "module", "namespace",
        "pub", "public", "private", "protected", "abstract", "native",
        "synchronized", "transient", "volatile", "override", "virtual",
        "readonly", "declare", "sealed",
        "try", "catch", "except", "finally", "throw", "raise", "throws",
        "void", "int", "long", "short", "byte", "float", "double", "char",
        "bool", "boolean", "str", "string", "uint", "usize", "isize",
        "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128",
        "f32", "f64",
        "null", "undefined", "nil", "None", "Some", "Ok", "Err",
        "true", "false", "True", "False", "NaN", "Infinity",
        "async", "await", "sync", "unsafe", "move", "ref", "box", "dyn", "where",
        "import", "export", "from", "use", "require", "include",
        "package", "crate", "extern", "super", "self", "Self",
        "new", "delete", "typeof", "instanceof", "in", "of", "as",
        "is", "not", "and", "or", "with", "pass", "assert", "del",
        "global", "nonlocal", "lambda",
        "this", "super",
        "go", "chan", "make", "cap", "close", "recover", "panic",
        "console", "Math", "Object", "Array", "String", "Number", "Boolean",
        "Promise", "Error", "JSON", "Symbol", "Map", "Set", "WeakMap", "WeakSet",
        "Date", "RegExp", "Buffer", "process", "global", "window", "document",
        "eval", "arguments", "prototype", "constructor",
        "print", "len", "range", "list", "dict", "tuple", "type", "set",
        "isinstance", "hasattr", "getattr", "setattr", "open", "input", "iter",
        "next", "enumerate", "zip", "map", "filter", "sorted", "reversed",
        "staticmethod", "classmethod", "property", "super",
        "Vec", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
        "Option", "Result", "Box", "Rc", "Arc", "Cell", "RefCell",
        "println", "eprintln", "format", "todo", "unimplemented", "unreachable",
        "assert", "assert_eq", "assert_ne", "debug_assert",
        "unwrap", "expect", "clone", "collect", "iter", "into_iter",
        "push", "pop", "len", "is_empty", "contains", "insert", "remove",
        "unwrap_or", "unwrap_or_else",
        "log", "warn", "error", "info", "debug",
        "get", "set", "has", "add",
        "map", "filter", "reduce", "find", "some", "every", "includes",
        "join", "split", "slice", "splice", "concat", "flat", "flatMap",
        "toString", "valueOf", "toFixed", "toInt", "toFloat",
        "apply", "call", "bind",
        "then", "catch", "finally",
        "keys", "values", "entries", "assign", "create", "freeze",
        "parseInt", "parseFloat", "isNaN", "isFinite",
        "throws", "abstract", "native", "strictfp",
    ].iter().copied().collect())
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
}
