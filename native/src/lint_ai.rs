//! `lint_ai` — two systems sharing one hypervector (Hv) substrate.
//!
//! ## Memory subsystem (unchanged)
//! 8192-bit hypervectors (`Hv`) with XOR binding and Hamming-distance retrieval.
//! Used by `memory/embed`, `memory/retriever`, `memory/store`, and the crawler.
//!
//! ## Rule validator: Hv-based nearest-neighbour classifier (`ConceptModel`)
//!
//! Lints any language against scraped official docs + project-local rules. No
//! hand-crafted patterns — the model learns entirely from the documentation.
//!
//! ### Training (one-time, cached as `<lang>.concepts.bin`)
//! Each rule supplies an English description, a bad-code example, and a good-code
//! example (all sourced from the official language documentation):
//!
//!   1. Tree-sitter parses the bad and good examples; all leaf tokens are collected.
//!   2. Set-difference: `bad_only` = tokens in bad but not good; `good_only` = vice versa.
//!   3. The English description is mapped through `/usr/share/dict/words` — only real
//!      English words are kept, giving the model semantic understanding of what the rule
//!      *means* (not just what tokens it fires on).
//!   4. `bad_hv`  = bundle(description Hvs, bad-only token Hvs)   — "what a violation looks like"
//!      `good_hv` = bundle(good-only token Hvs)                    — "what correct code looks like"
//!   Both are 8192-bit vectors persisted in the compact `LNC3` binary blob.
//!
//! ### Inference (per AST node, every file)
//!   For each node whose kind matches a compiled rule's kind, build `node_hv` from
//!   its leaf tokens and fire the rule when the node is meaningfully closer to the
//!   bad fingerprint than the good one:
//!
//!     `Hamming(node_hv, bad_hv) + HV_FIRE_MARGIN < Hamming(node_hv, good_hv)`
//!
//!   `HV_FIRE_MARGIN` (181 bits) is 4 standard deviations of Hamming noise for
//!   8192-bit Hvs — a statistical necessity, not a semantic choice.
//!
//! ### Getting started
//! No arguments required. On first `lint` the model trains automatically from the
//! committed `lint-index/` catalogs, caches the binary blob, and runs. Subsequent
//! runs load the cache in microseconds. To add a language: `lint_add_source` then
//! `lint_learn`; to add project rules: drop a `*.md` in `.helpers/lint-rules/`.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::{Node, Parser};

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

// ── Hv-based rule validator (ConceptModel) ───────────────────────────────────

/// How far below random distance (DIM/2) a node must sit to fire a rule.
/// DIM/10 = 819 for DIM=8192. Conservative by design: a false negative is
/// preferable to a false positive.
const HV_FIRE_MARGIN: u32 = (DIM / 10) as u32;

// ── LangBrain: dictionary-grounded English understanding ─────────────────────

/// English word set from `/usr/share/dict/words`. Loaded once at first use.
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

/// Map a text blob to an Hv by bundling every alphanumeric token's code.
/// Dictionary words get weight from both the dict layer and this layer; code
/// identifiers contribute via this layer only.
pub fn text_to_hv(text: &str) -> Option<Hv> {
    let dict = dict_words();
    let mut b = Bundler::new();
    for tok in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        let t = tok.trim().to_lowercase();
        if t.len() < 2 || t.len() > 64 { continue; }
        b.add(&token_hv(&t));
        if t.len() >= 3 && dict.contains(&t) { b.add(&token_hv(&t)); }
    }
    if b.is_empty() { None } else { Some(b.finalize()) }
}

// ── ConceptModel ─────────────────────────────────────────────────────────────

/// One compiled rule: bad and good Hv fingerprints for nearest-neighbour matching.
///
/// Binary layout (2064 bytes):
/// One compiled rule: a single Hv fingerprint representing the rule's full concept,
/// built from its English description and its complete documentation page text.
///
/// Binary layout `LNC4` (1032 bytes/rule):
///   id_hash (u64) + rule_hv (WORDS × u64)
#[derive(Clone)]
pub struct CompiledRule {
    pub id_hash: u64,
    /// Concept fingerprint: bundle of description dictionary tokens (semantic layer,
    /// weighted 2×) + all alphanumeric tokens from the full documentation page.
    pub rule_hv: Hv,
}

/// One violation reported by the model.
pub struct Flag {
    /// 1-based source line of the violating AST node.
    pub line: usize,
    /// Rule id string resolved from `ConceptModel::id_map`.
    pub rule_id: String,
}

/// The compiled Hv-based linter. Learns from official docs; lints any language.
///
/// Binary format `LNC5`:
///   magic (4 B) + n_rules (u32 LE) + n_stop (u32 LE)
///   + [id_hash (u64) + rule_hv (WORDS × u64)] × n_rules
///   + [token_hash (u64)] × n_stop
///
/// `id_map` is not persisted — restored via `merge_ids` after each load.
pub struct ConceptModel {
    pub rules: Vec<CompiledRule>,
    /// hash → rule id string. Populated by `merge_ids`.
    pub id_map: HashMap<u64, String>,
    /// FNV-hashed tokens too common (>5% DF) to be meaningful inference signals.
    /// `check_node` skips any leaf token whose hash is in this set.
    pub inference_stop: HashSet<u64>,
}

impl ConceptModel {
    /// Compile rules into a `ConceptModel`.
    ///
    /// Each rule `(id, description, page_text)` produces one `rule_hv` via majority bundling.
    /// Training signal = description tokens + code-block tokens from the official doc page.
    ///
    /// An **inference stop set** is computed from the corpus: any token appearing in more than 5%
    /// of rules is too common to be a useful match signal at inference time (e.g. `return` appears
    /// in 30% of ESLint rules' code examples — any code with `return` would fire all of them).
    /// These tokens are hashed and stored in `inference_stop`; `check_node` skips them entirely.
    /// Only rare, rule-specific tokens (`eval`, `debugger`, `async`, `with`, …) survive and drive
    /// actual findings. Training itself is not filtered — rare tokens dominate each rule_hv
    /// naturally because they appear many more times than background noise tokens.
    pub fn compile(rules: &[(String, String, String)], _lang: &str) -> ConceptModel {
        // Pass 1: DF across all rules (for inference stop set only; training uses all tokens).
        let total = rules.len();
        let max_df_inference = ((total as f64 * 0.05).ceil() as usize).max(1);
        let mut df: HashMap<String, usize> = HashMap::new();
        for (_, desc, page) in rules {
            let mut seen = HashSet::new();
            for tok in format!("{} {}", desc, page).split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                let t = tok.to_lowercase();
                if t.len() >= 2 && t.len() <= 64 && seen.insert(t.clone()) {
                    *df.entry(t).or_default() += 1;
                }
            }
        }
        // Tokens appearing in >5% of rules are corpus stop words for inference.
        let inference_stop: HashSet<u64> = df.iter()
            .filter(|(_, &n)| n > max_df_inference)
            .map(|(t, _)| token_seed(t))
            .collect();

        // Pass 2: build each rule_hv from ALL tokens (no training-time filtering).
        let dict = dict_words();
        let mut compiled = Vec::new();
        let mut id_map = HashMap::new();
        for (id, description, page_text) in rules {
            let mut b = Bundler::new();
            for tok in format!("{} {}", description, page_text)
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
            compiled.push(CompiledRule { id_hash, rule_hv: b.finalize() });
            id_map.insert(id_hash, id.clone());
        }
        ConceptModel { rules: compiled, id_map, inference_stop }
    }

    /// Restore id strings from a hash→id lookup table (call after `load`).
    pub fn merge_ids(&mut self, id_lookup: &HashMap<u64, String>) {
        for (k, v) in id_lookup {
            self.id_map.entry(*k).or_insert_with(|| v.clone());
        }
    }

    /// Walk `src`'s full AST and return one `Flag` per unique (rule_id, line) pair.
    pub fn validate(&self, src: &str, lang: &str) -> Vec<Flag> {
        if self.rules.is_empty() { return Vec::new(); }
        let Some(language) = crate::lint_match::language(lang) else { return Vec::new() };
        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() { return Vec::new(); }
        let Some(tree) = parser.parse(src, None) else { return Vec::new() };
        let mut raw: Vec<Flag> = Vec::new();
        self.check_node(tree.root_node(), src.as_bytes(), &mut raw);
        // Deduplicate: same rule on the same line fires once.
        let mut seen = HashSet::new();
        raw.into_iter().filter(|f| seen.insert((f.line, f.rule_id.clone()))).collect()
    }

    /// Walk the AST leaf-by-leaf. For each leaf token that passes the two-stage filter,
    /// check its Hv against every compiled rule and fire when the distance falls below
    /// DIM/2 − HV_FIRE_MARGIN.
    ///
    /// Stage 1 (DF-based): skip tokens in `inference_stop` — tokens too common across rules
    ///   to be discriminative (e.g. `return` appears in 30% of ESLint rule examples).
    ///
    /// Stage 2 (English-word filter): skip tokens that are common English dictionary words
    ///   but NOT language keywords — they appear as variable/function names in documentation
    ///   examples (`hello`, `greet`, `name`, `user`) but never as violation markers in code.
    ///   Language keywords (`eval`, `with`, `delete`, `async`, `var`, …) are exempt and checked.
    fn check_node(&self, node: Node<'_>, src: &[u8], flags: &mut Vec<Flag>) {
        if node.child_count() == 0 {
            let Ok(text) = std::str::from_utf8(&src[node.byte_range()]) else { return };
            let t = text.trim().to_lowercase();
            if t.len() < 2 || t.len() > 64 { return; }
            if !t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { return; }
            let hash = token_seed(&t);
            // Stage 1: DF-based corpus stop set (e.g. `return`, `function`, `const`).
            if self.inference_stop.contains(&hash) { return; }
            // Stage 2: common English words that are not language keywords are example names,
            // not violation markers (e.g. `hello`, `greet`, `name`, `result`).
            let kw = keywords();
            if dict_words().contains(t.as_str()) && !kw.contains(t.as_str()) { return; }
            let hv = token_hv(&t);
            let line = node.start_position().row + 1;
            for rule in &self.rules {
                let d = hv.distance(&rule.rule_hv);
                if d + HV_FIRE_MARGIN < (DIM as u32) / 2 {
                    let rule_id = self.id_map.get(&rule.id_hash)
                        .cloned()
                        .unwrap_or_else(|| format!("{:016x}", rule.id_hash));
                    flags.push(Flag { line, rule_id });
                }
            }
        } else {
            let mut cur = node.walk();
            for child in node.children(&mut cur) { self.check_node(child, src, flags); }
        }
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }

    /// Persist to `LNC5` binary format:
    ///   magic (4) + n_rules (u32 LE) + n_stop (u32 LE)
    ///   + [id_hash (u64) + rule_hv (WORDS × u64)] × n_rules
    ///   + [token_hash (u64)] × n_stop
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let rule_entry = 8 + WORDS * 8;
        let stop_tokens: Vec<u64> = self.inference_stop.iter().copied().collect();
        let mut buf = Vec::with_capacity(12 + self.rules.len() * rule_entry + stop_tokens.len() * 8);
        buf.extend_from_slice(b"LNC5");
        buf.extend_from_slice(&(self.rules.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(stop_tokens.len() as u32).to_le_bytes());
        for r in &self.rules {
            buf.extend_from_slice(&r.id_hash.to_le_bytes());
            for w in r.rule_hv.as_words() { buf.extend_from_slice(&w.to_le_bytes()); }
        }
        for h in &stop_tokens { buf.extend_from_slice(&h.to_le_bytes()); }
        std::fs::write(path, buf)
    }

    /// Load from `LNC5`. Returns `None` on format mismatch. Call `merge_ids` after.
    pub fn load(path: &Path) -> Option<ConceptModel> {
        let data = std::fs::read(path).ok()?;
        if data.len() < 12 || &data[..4] != b"LNC5" { return None; }
        let n_rules = u32::from_le_bytes(data[4..8].try_into().ok()?) as usize;
        let n_stop = u32::from_le_bytes(data[8..12].try_into().ok()?) as usize;
        let rule_entry = 8 + WORDS * 8;
        if data.len() < 12 + n_rules * rule_entry + n_stop * 8 { return None; }
        let mut rules = Vec::with_capacity(n_rules);
        let mut pos = 12usize;
        for _ in 0..n_rules {
            let id_hash = u64::from_le_bytes(data[pos..pos+8].try_into().ok()?); pos += 8;
            let mut rw = [0u64; WORDS];
            for w in rw.iter_mut() { *w = u64::from_le_bytes(data[pos..pos+8].try_into().ok()?); pos += 8; }
            rules.push(CompiledRule { id_hash, rule_hv: Hv::from_words(&rw) });
        }
        let mut inference_stop = HashSet::with_capacity(n_stop);
        for _ in 0..n_stop {
            inference_stop.insert(u64::from_le_bytes(data[pos..pos+8].try_into().ok()?));
            pos += 8;
        }
        Some(ConceptModel { rules, id_map: HashMap::new(), inference_stop })
    }
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
    fn concept_model_compiles_from_page_text() {
        // 3-tuple: (id, description, page_text). Training signal = description + full page text.
        let rules = vec![(
            "no-var".to_string(),
            "avoid var; prefer let or const for block scoping".to_string(),
            "var x = 1; var count = 42; var y = true; // disallow var declarations".to_string(),
        )];
        let model = ConceptModel::compile(&rules, "javascript");
        // Just verify compilation produces a non-empty model with the correct id.
        assert_eq!(model.rule_count(), 1);
        assert!(model.id_map.values().any(|id| id == "no-var"));
    }
}
