//! `lint_ai` — two systems sharing one 1-bit binary vector substrate.
//!
//! ## Memory subsystem (unchanged)
//! 8192-bit hypervectors (`Hv`) with XOR binding and Hamming-distance retrieval.
//! Used by `memory/embed`, `memory/retriever`, `memory/store`, and the crawler.
//!
//! ## Concept-based rule validator (`ConceptModel`)
//! Rules are compiled from documentation: tree-sitter parses each rule's bad and good
//! examples, concept bits are the AST node types and keyword tokens present in the bad
//! AST but absent from the good AST. At inference the full file AST is walked; a node
//! whose concept bits intersect a rule's concept bits is a violation.
//!
//! Zero hand-crafting: the doc crawler fetches the examples; tree-sitter gives the node
//! types; the diff gives the rule. The model is a flat binary blob — magic header, then
//! per rule: id-hash (u64) + concept bits (8 × u64 = 512 bits). ~72 bytes per rule,
//! ~120 KB for 1642 rules. No strings, no JSON, no human-readable structure.

use std::collections::HashMap;
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

// ── Concept-based rule validator ──────────────────────────────────────────────

/// Concept vector width in bits. 512 bits keeps the file compact (~64 bytes/rule)
/// while staying sparse enough that 2–5 bits set per rule rarely collide in practice.
const CONCEPT_DIM: usize = 512;
const CONCEPT_WORDS: usize = CONCEPT_DIM / 64; // 8

/// A packed 512-bit concept set. Each set bit marks one tree-sitter construct.
pub type ConceptVec = [u64; CONCEPT_WORDS];

/// Map any construct name (node type, keyword text) to its bit position.
/// Uses the same FNV-1a seed as the HV substrate so the hash is uniform.
pub fn concept_bit(name: &str) -> usize {
    token_seed(name) as usize % CONCEPT_DIM
}

/// Set one concept bit in `vec`.
pub fn add_concept(vec: &mut ConceptVec, name: &str) {
    let bit = concept_bit(name);
    vec[bit / 64] |= 1u64 << (bit % 64);
}

/// True if the two concept sets share at least one bit (rule matches this node).
pub fn concepts_intersect(a: &ConceptVec, b: &ConceptVec) -> bool {
    a.iter().zip(b.iter()).any(|(x, y)| x & y != 0)
}

/// Collect concept bits from one AST node shallowly: the node's kind plus the text
/// of its immediate leaf children that look like keywords or identifiers (≤32 ASCII chars).
fn node_concepts(node: Node, src: &[u8]) -> ConceptVec {
    let mut vec = [0u64; CONCEPT_WORDS];
    add_concept(&mut vec, node.kind());
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.child_count() == 0 {
            if let Ok(text) = std::str::from_utf8(&src[child.byte_range()]) {
                let t = text.trim();
                if !t.is_empty() && t.len() <= 32
                    && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    add_concept(&mut vec, t);
                }
            }
        }
    }
    vec
}

/// Collect all concept bits from an entire parsed AST (used when compiling rules).
fn collect_all_concepts(node: Node, src: &[u8], out: &mut ConceptVec) {
    let c = node_concepts(node, src);
    for (a, b) in out.iter_mut().zip(c.iter()) { *a |= b; }
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        collect_all_concepts(child, src, out);
    }
}

/// Parse `src` as `lang` and return the full concept set, or `None` if unparsable.
fn source_concepts(src: &str, lang: &str) -> Option<ConceptVec> {
    let language = crate::lint_match::language(lang)?;
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    let tree = parser.parse(src, None)?;
    let mut vec = [0u64; CONCEPT_WORDS];
    collect_all_concepts(tree.root_node(), src.as_bytes(), &mut vec);
    Some(vec)
}

/// One compiled rule: its id hash and the concept bits that distinguish its bad
/// example from its good example. These bits are what fires a violation.
#[derive(Clone)]
pub struct CompiledRule {
    /// FNV-1a of the rule id — the only identifier in the binary model.
    pub id_hash: u64,
    /// Concept bits present in bad but absent from good: the violation signature.
    pub concepts: ConceptVec,
}

/// One violation found by the concept model.
pub struct Flag {
    /// 1-based source line of the violating AST node.
    pub line: usize,
    /// The rule's string id (resolved from `ConceptModel::id_map`).
    pub rule_id: String,
}

/// The compiled 1-bit concept linter.
///
/// On disk: `LNCT` magic (4 bytes) + n_rules (u32) + entries (id_hash u64 + concepts
/// 8×u64 each). No strings, no JSON. ~72 bytes/rule, ~120 KB for 1642 rules.
/// In memory: the same data plus an `id_map` (hash → rule id string) for reporting.
pub struct ConceptModel {
    /// Compiled rules: id hash + concept bits.
    pub rules: Vec<CompiledRule>,
    /// Hash → rule id string. Built from `rule_advice` at load/compile time; not persisted.
    pub id_map: HashMap<u64, String>,
}

impl ConceptModel {
    /// Compile from `(id, bad_example, good_example)` triples for language `lang`.
    ///
    /// For each rule: tree-sitter parses bad and good; concept bits = constructs in bad
    /// that are absent from good. Rules where bad and good are structurally identical
    /// (no distinguishing constructs) are silently dropped.
    pub fn compile(rules: &[(String, String, String)], lang: &str) -> ConceptModel {
        let mut compiled = Vec::new();
        let mut id_map = HashMap::new();
        for (id, bad, good) in rules {
            let Some(bad_c) = source_concepts(bad, lang) else { continue };
            let Some(good_c) = source_concepts(good, lang) else { continue };
            // Keep only concepts in bad that are absent from good — the violation signature.
            let mut concepts = [0u64; CONCEPT_WORDS];
            for i in 0..CONCEPT_WORDS {
                concepts[i] = bad_c[i] & !good_c[i];
            }
            if concepts.iter().all(|&w| w == 0) { continue; }
            let id_hash = token_seed(id);
            compiled.push(CompiledRule { id_hash, concepts });
            id_map.insert(id_hash, id.clone());
        }
        ConceptModel { rules: compiled, id_map }
    }

    /// Merge `id_lookup` (hash → rule id) into this model's map without replacing entries
    /// already present. Call after loading from binary to restore string ids.
    pub fn merge_ids(&mut self, id_lookup: &HashMap<u64, String>) {
        for (k, v) in id_lookup {
            self.id_map.entry(*k).or_insert_with(|| v.clone());
        }
    }

    /// Walk `src`'s full AST and return one `Flag` per node that intersects any rule.
    pub fn validate(&self, src: &str, lang: &str) -> Vec<Flag> {
        if self.rules.is_empty() { return Vec::new(); }
        let Some(language) = crate::lint_match::language(lang) else { return Vec::new() };
        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() { return Vec::new(); }
        let Some(tree) = parser.parse(src, None) else { return Vec::new() };
        let mut flags = Vec::new();
        self.check_node(tree.root_node(), src.as_bytes(), &mut flags);
        flags
    }

    fn check_node(&self, node: Node, src: &[u8], flags: &mut Vec<Flag>) {
        let concepts = node_concepts(node, src);
        for rule in &self.rules {
            if concepts_intersect(&concepts, &rule.concepts) {
                let rule_id = self.id_map.get(&rule.id_hash)
                    .cloned()
                    .unwrap_or_else(|| format!("{:016x}", rule.id_hash));
                flags.push(Flag { line: node.start_position().row + 1, rule_id });
            }
        }
        let mut cur = node.walk();
        for child in node.children(&mut cur) {
            self.check_node(child, src, flags);
        }
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }

    /// Write the compact binary model to `path`.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let entry_size = 8 + CONCEPT_WORDS * 8;
        let mut buf = Vec::with_capacity(8 + self.rules.len() * entry_size);
        buf.extend_from_slice(b"LNCT");
        buf.extend_from_slice(&(self.rules.len() as u32).to_le_bytes());
        for r in &self.rules {
            buf.extend_from_slice(&r.id_hash.to_le_bytes());
            for w in &r.concepts { buf.extend_from_slice(&w.to_le_bytes()); }
        }
        std::fs::write(path, buf)
    }

    /// Load from the compact binary model. Returns `None` on any format error.
    /// Call `merge_ids` afterward to attach rule id strings from the current rule set.
    pub fn load(path: &Path) -> Option<ConceptModel> {
        let data = std::fs::read(path).ok()?;
        if data.len() < 8 || &data[..4] != b"LNCT" { return None; }
        let n = u32::from_le_bytes(data[4..8].try_into().ok()?) as usize;
        let entry = 8 + CONCEPT_WORDS * 8;
        if data.len() < 8 + n * entry { return None; }
        let mut rules = Vec::with_capacity(n);
        let mut pos = 8usize;
        for _ in 0..n {
            let id_hash = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
            pos += 8;
            let mut concepts = [0u64; CONCEPT_WORDS];
            for w in concepts.iter_mut() {
                *w = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
                pos += 8;
            }
            rules.push(CompiledRule { id_hash, concepts });
        }
        Some(ConceptModel { rules, id_map: HashMap::new() })
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
    fn concept_model_catches_var_not_let() {
        // Requires a JS tree-sitter grammar; skip if absent.
        let rules = vec![
            ("no-var".to_string(), "var x = 1;".to_string(), "let x = 1;".to_string()),
        ];
        let model = ConceptModel::compile(&rules, "javascript");
        if model.rule_count() == 0 { return; } // no grammar available in this env

        let hits = model.validate("var count = 42;", "javascript");
        assert!(!hits.is_empty(), "var declaration must fire");
        assert!(hits.iter().all(|f| f.rule_id == "no-var"));

        let clean = model.validate("let count = 42;", "javascript");
        assert!(clean.is_empty(), "let declaration must not fire");
    }
}
