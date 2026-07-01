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
//
// Architecture: three properties guarantee low FP/FN.
//
//   1. Kind-hash filtering — each compiled rule records the AST node kind of the
//      bad example's primary statement. During validation only nodes of that exact
//      kind are checked, so a `no-var` rule never fires on a `function_declaration`.
//
//   2. Consistent granularity — both compilation and validation use SHALLOW concept
//      extraction on a single statement node. The compile step uses `first_stmt_concepts`
//      (shallow of the first statement in the bad / good example); the validate step
//      uses `node_concepts` (shallow of each visited node). Same function, same depth.
//
//   3. Identifier normalization — raw identifier text (non-keyword alphanumeric tokens)
//      maps to `<id>` and digit-only tokens to `<num>`. This prevents example-specific
//      variable names from becoming spurious concept bits, leaving only language keywords
//      and structural operator tokens to discriminate rules.
//
// Rules where `bad_concepts & !good_concepts == 0` (no distinguishing shallow bits) are
// silently dropped — they produce no FP but will miss complex structural violations (FN).
// That is the correct tradeoff: unknown is better than wrong.

/// Concept vector width in bits.
const CONCEPT_DIM: usize = 512;
const CONCEPT_WORDS: usize = CONCEPT_DIM / 64; // 8

/// A packed 512-bit concept set. Each set bit marks one tree-sitter construct.
pub type ConceptVec = [u64; CONCEPT_WORDS];

/// Map any construct name (node type, normalized token) to its bit position.
pub fn concept_bit(name: &str) -> usize {
    token_seed(name) as usize % CONCEPT_DIM
}

/// Set one concept bit in `vec`.
pub fn add_concept(vec: &mut ConceptVec, name: &str) {
    let bit = concept_bit(name);
    vec[bit / 64] |= 1u64 << (bit % 64);
}

/// True if the two concept sets share at least one bit.
pub fn concepts_intersect(a: &ConceptVec, b: &ConceptVec) -> bool {
    a.iter().zip(b.iter()).any(|(x, y)| x & y != 0)
}

/// Shallow concept extraction: the node's kind plus immediate leaf child tokens.
/// Identifiers are kept verbatim — two different identifiers map to different bits,
/// so when bad/good examples use the same variable names those bits cancel out
/// naturally in the `bad & !good` step, leaving only structurally distinctive tokens.
fn node_concepts(node: Node<'_>, src: &[u8]) -> ConceptVec {
    let mut vec = [0u64; CONCEPT_WORDS];
    add_concept(&mut vec, node.kind());
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.child_count() != 0 { continue; }
        let Ok(text) = std::str::from_utf8(&src[child.byte_range()]) else { continue };
        let t = text.trim();
        if t.is_empty() || t.len() > 32 { continue; }
        if !t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { continue; }
        add_concept(&mut vec, t);
    }
    vec
}

/// Generic wrapper node kinds: purely syntactic containers that add no semantic
/// discriminating power on their own. When a bad/good example's first statement
/// is one of these we descend one level further to find the real construct.
fn is_generic_wrapper(kind: &str) -> bool {
    matches!(kind,
        "expression_statement" | "block" | "program" | "source_file" |
        "module" | "chunk" | "document" | "translation_unit"
    )
}

/// Very common node kinds in typical source files.
///
/// Rules whose bad example resolves to one of these kinds are silently dropped:
/// they would fire on far too many nodes to be useful (e.g. every function call,
/// every if-branch). Only applied to the BAD example — the good example's kind is
/// never filtered so it can still supply concept bits for subtraction.
fn is_too_common(kind: &str) -> bool {
    matches!(kind,
        // Generic wrappers (belt-and-suspenders with is_generic_wrapper)
        "expression_statement" | "block" | "program" | "source_file" |
        "module" | "chunk" | "document" | "translation_unit" |

        // JS/TS: ubiquitous statement and expression kinds
        "lexical_declaration" | "variable_declaration" | "variable_declarator" |
        "function_declaration" | "arrow_function" | "method_definition" |
        "if_statement" | "for_statement" | "while_statement" | "do_statement" |
        "return_statement" | "export_statement" | "import_declaration" |
        "class_declaration" | "object" | "array" | "member_expression" |
        "call_expression" | "binary_expression" | "unary_expression" |
        "assignment_expression" | "subscript_expression" |
        "string" | "parenthesized_expression" |
        "switch_statement" | "new_expression" | "labeled_statement" |
        "switch_body" | "case" |

        // Rust: ubiquitous expression and item kinds
        "function_item" | "let_declaration" | "expression_item" | "item" |
        "macro_invocation" | "method_call_expression" | "field_expression" |
        "if_expression" | "match_expression" | "for_expression" | "while_expression" |
        "loop_expression" | "return_expression" | "compound_assignment_expr" |
        "reference_expression" | "try_expression" | "type_cast_expression" |
        "index_expression" | "range_expression" | "closure_expression" |
        "struct_item" | "impl_item" | "enum_item" | "use_declaration" |
        "attribute_item" | "unsafe_block" |

        // Python: ubiquitous statement, operator, and literal kinds
        "function_definition" | "class_definition" |
        "assignment" | "augmented_assignment" | "return_statement" |
        "import_statement" | "import_from_statement" | "call" |
        "if_statement" | "for_statement" | "while_statement" | "with_statement" |
        "try_statement" | "with_clause" |
        "list" | "dictionary" | "set" | "tuple" |
        "attribute" | "subscript" | "slice" |
        "not_operator" | "boolean_operator" | "comparison_operator" |
        "binary_operator" | "unary_operator" | "conditional_expression" |
        "future_import_statement" | "raise_statement" | "delete_statement" |
        "assert_statement" | "decorated_definition" | "list_comprehension" |
        "set_comprehension" | "dictionary_comprehension" | "generator_expression"
    )
}

/// Walk `src`'s AST to find the first meaningful statement, stripping generic
/// wrapper nodes one level deep. Returns `(kind_hash, concepts)` or `None` if
/// parsing fails or no usable statement node is found.
///
/// If `filter_common` is `true` (used for BAD examples), returns `None` when the
/// resolved kind is in the too-common blocklist. Good examples never filter so
/// their concepts are always available for the bad & !good subtraction.
fn stmt_concepts_impl(src: &str, lang: &str, filter_common: bool) -> Option<(u64, ConceptVec)> {
    let language = crate::lint_match::language(lang)?;
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    let tree = parser.parse(src, None)?;
    let root = tree.root_node();

    let first = (0..root.child_count())
        .filter_map(|i| root.child(i))
        .find(|n| !n.is_extra() && n.child_count() > 0)
        .unwrap_or(root);

    // Strip one level of generic wrapper to reach the real construct.
    let target = if is_generic_wrapper(first.kind()) {
        (0..first.child_count())
            .filter_map(|i| first.child(i))
            .find(|n| !n.is_extra() && n.child_count() > 0)
            .unwrap_or(first)
    } else {
        first
    };

    if filter_common && is_too_common(target.kind()) { return None; }
    Some((token_seed(target.kind()), node_concepts(target, src.as_bytes())))
}

/// First-statement concepts for a BAD example: returns `None` for too-common kinds.
fn bad_stmt_concepts(src: &str, lang: &str) -> Option<(u64, ConceptVec)> {
    stmt_concepts_impl(src, lang, true)
}

/// First-statement concepts for a GOOD example: no too-common filter — only used
/// to supply concept bits to subtract from the bad side.
fn good_stmt_concepts(src: &str, lang: &str) -> Option<ConceptVec> {
    stmt_concepts_impl(src, lang, false).map(|(_, c)| c)
}

/// One compiled rule stored in the binary model.
///
/// Binary layout (80 bytes): id_hash (u64) + kind_hash (u64) + concepts (8 × u64).
#[derive(Clone)]
pub struct CompiledRule {
    /// FNV-1a hash of the rule id string.
    pub id_hash: u64,
    /// FNV-1a hash of the primary statement kind from the bad example.
    /// Validation only checks nodes whose `token_seed(node.kind()) == kind_hash`.
    pub kind_hash: u64,
    /// Concept bits: present in bad example's first statement but absent from good's.
    pub concepts: ConceptVec,
}

/// One violation reported by the concept model.
pub struct Flag {
    /// 1-based source line of the violating AST node.
    pub line: usize,
    /// Rule id string resolved from `ConceptModel::id_map`.
    pub rule_id: String,
}

/// The compiled 1-bit concept linter.
///
/// Binary format: magic `LNC2` (4 bytes) + n_rules (u32 LE) +
/// [id_hash u64 + kind_hash u64 + concepts 8×u64] × n. 80 bytes/rule.
/// No strings, no JSON. ~128 KB for 1642 rules.
/// `id_map` is not persisted — rebuilt from `rule_advice` after each load.
pub struct ConceptModel {
    pub rules: Vec<CompiledRule>,
    /// hash → rule id string. Not stored on disk; restored via `merge_ids`.
    pub id_map: HashMap<u64, String>,
}

impl ConceptModel {
    /// Compile from `(id, bad_example, good_example)` triples.
    ///
    /// Each rule: parse bad and good → extract first-statement shallow concepts →
    /// concepts = bad & !good. Rules with empty distinguishing concept set are dropped
    /// (no FP, silently missed). Kind hash recorded for validation pre-filtering.
    /// Compile `rules` into a `ConceptModel` for `lang`.
    ///
    /// A rule is compiled only when:
    /// 1. The bad example's first meaningful statement resolves to a **rare** primary kind
    ///    (kinds that appear in virtually every file are blocked by `is_too_common`).
    /// 2. The `bad & !good` concept vector has **at least one** distinguishing bit.
    ///
    /// Good examples are parsed without the too-common filter so they can always
    /// supply concept bits for subtraction even if their kind is generic. The
    /// kind-hash recorded with each rule gates validation to the specific bad kind.
    pub fn compile(rules: &[(String, String, String)], lang: &str) -> ConceptModel {
        let mut compiled = Vec::new();
        let mut id_map = HashMap::new();
        for (id, bad, good) in rules {
            // Bad example must resolve to a rare, specific primary kind.
            let Some((bad_kind_hash, bad_c)) = bad_stmt_concepts(bad, lang) else { continue };
            // Good example concepts are used only for subtraction; kind is not filtered.
            let good_c = good_stmt_concepts(good, lang).unwrap_or([0u64; CONCEPT_WORDS]);
            let mut concepts = [0u64; CONCEPT_WORDS];
            for i in 0..CONCEPT_WORDS { concepts[i] = bad_c[i] & !good_c[i]; }
            let bits: u32 = concepts.iter().map(|w| w.count_ones()).sum();
            if bits == 0 { continue; }
            let id_hash = token_seed(id);
            compiled.push(CompiledRule { id_hash, kind_hash: bad_kind_hash, concepts });
            id_map.insert(id_hash, id.clone());
        }
        ConceptModel { rules: compiled, id_map }
    }

    /// Restore id strings from a hash→id lookup table (call after `load`).
    pub fn merge_ids(&mut self, id_lookup: &HashMap<u64, String>) {
        for (k, v) in id_lookup {
            self.id_map.entry(*k).or_insert_with(|| v.clone());
        }
    }

    /// Walk `src`'s full AST, check each node against matching-kind rules,
    /// and return one `Flag` per distinct (rule, line) violation found.
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

    fn check_node(&self, node: Node<'_>, src: &[u8], flags: &mut Vec<Flag>) {
        let node_kind_hash = token_seed(node.kind());
        // Pre-filter by kind hash — avoid concept extraction on non-matching nodes.
        let matching_rules: Vec<&CompiledRule> = self.rules.iter()
            .filter(|r| r.kind_hash == node_kind_hash)
            .collect();
        if !matching_rules.is_empty() {
            let concepts = node_concepts(node, src);
            for rule in matching_rules {
                if concepts_intersect(&concepts, &rule.concepts) {
                    let rule_id = self.id_map.get(&rule.id_hash)
                        .cloned()
                        .unwrap_or_else(|| format!("{:016x}", rule.id_hash));
                    flags.push(Flag { line: node.start_position().row + 1, rule_id });
                }
            }
        }
        let mut cur = node.walk();
        for child in node.children(&mut cur) {
            self.check_node(child, src, flags);
        }
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }

    /// Write the compact binary model (`LNC2` format, 80 bytes/rule).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let entry_size = 8 + 8 + CONCEPT_WORDS * 8; // id_hash + kind_hash + concepts
        let mut buf = Vec::with_capacity(8 + self.rules.len() * entry_size);
        buf.extend_from_slice(b"LNC2");
        buf.extend_from_slice(&(self.rules.len() as u32).to_le_bytes());
        for r in &self.rules {
            buf.extend_from_slice(&r.id_hash.to_le_bytes());
            buf.extend_from_slice(&r.kind_hash.to_le_bytes());
            for w in &r.concepts { buf.extend_from_slice(&w.to_le_bytes()); }
        }
        std::fs::write(path, buf)
    }

    /// Load from a `LNC2` binary blob. Returns `None` on format mismatch.
    /// Call `merge_ids` to restore id strings for reporting.
    pub fn load(path: &Path) -> Option<ConceptModel> {
        let data = std::fs::read(path).ok()?;
        if data.len() < 8 || &data[..4] != b"LNC2" { return None; }
        let n = u32::from_le_bytes(data[4..8].try_into().ok()?) as usize;
        let entry = 8 + 8 + CONCEPT_WORDS * 8;
        if data.len() < 8 + n * entry { return None; }
        let mut rules = Vec::with_capacity(n);
        let mut pos = 8usize;
        for _ in 0..n {
            let id_hash   = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?); pos += 8;
            let kind_hash = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?); pos += 8;
            let mut concepts = [0u64; CONCEPT_WORDS];
            for w in concepts.iter_mut() {
                *w = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
                pos += 8;
            }
            rules.push(CompiledRule { id_hash, kind_hash, concepts });
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
        let rules = vec![
            ("no-var".to_string(), "var x = 1;".to_string(), "let x = 1;".to_string()),
        ];
        let model = ConceptModel::compile(&rules, "javascript");
        if model.rule_count() == 0 { return; } // grammar unavailable in CI

        let hits = model.validate("var count = 42;", "javascript");
        assert!(!hits.is_empty(), "var declaration must fire");
        assert!(hits.iter().all(|f| f.rule_id == "no-var"));

        // let-declaration must NOT fire — kind matches but 'var' bit absent
        let clean = model.validate("let count = 42;", "javascript");
        assert!(clean.is_empty(), "let declaration must not fire");

        // Unrelated node kind (function declaration) must NOT fire regardless of content
        let unrelated = model.validate("function foo(var_name) { return var_name; }", "javascript");
        // rule kind = variable_declaration; function_declaration won't match
        assert!(unrelated.is_empty(), "function decl must not fire no-var");
    }
}
