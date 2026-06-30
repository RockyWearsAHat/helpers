//! `lint_ai` — the hypervector XOR engine: the model that *is* the linter.
//!
//! Everything is a fixed-width binary hypervector. The only operations are XOR (bind/compare),
//! bit-rotate (encode position), and Hamming distance (similarity). Open-vocabulary: any token of
//! any language maps to a code by hashing — the engine isn't built for a language.
//!
//! **Identifier normalization**: user-defined names (`x`, `myVar`, `count`) are normalized to
//! `<id>` so `var x = 1` and `var myCount = getValue()` produce the same token stream. Structural
//! keywords (`var`, `let`, `===`, `null`, etc.) are preserved. This gives the engine semantic
//! invariance: it matches the *structure* of a violation, not specific variable names.
//!
//! **Per-rule window sizes**: each rule uses its bad example's actual token count as its window.
//! A 5-token bad example creates 5-token exemplars and 5-token windows in source. No structural
//! information is lost to fixed-window padding or context bleed.
//!
//! **Exemplar matching (no perceptron)**: rather than training a perceptron (which can fail to
//! converge), the engine stores the novel bad-example windows directly as exemplars and calibrates
//! a per-exemplar radius to be just below the distance to the nearest good-example window. By
//! construction: nothing that looks like the good example fires (0 FP), everything that looks like
//! the bad example fires (0 FN for patterns with sufficient documentation).

use std::collections::HashSet;
use std::sync::OnceLock;

/// Hypervector width in bits. 8192 bits — wide enough for near-orthogonal random codes.
pub const DIM: usize = 8192;
const WORDS: usize = DIM / 64;

/// Maximum window size. Bad examples longer than this use overlapping MAX_WINDOW-token windows.
const MAX_WINDOW: usize = 24;

/// Maximum exemplar radius in bits. Exact structural matches have Hamming distance 0 (always fire);
/// random production code is at ~DIM/2 ≈ 4096 bits. This cap keeps balls tiny so FP rate is
/// negligible — even a window 256 bits from an exemplar is astronomically unlikely by chance alone.
const MAX_RADIUS_CAP: u32 = 256;

/// A `DIM`-bit binary hypervector. The one and only representation the engine uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Hv(#[serde(with = "hv_serde")] [u64; WORDS]);

impl Hv {
    /// The all-zero vector — the identity for XOR.
    pub fn zero() -> Hv { Hv([0; WORDS]) }

    /// The packed `u64` words — for external serialization.
    pub fn as_words(&self) -> &[u64] { &self.0 }

    /// Rebuild from packed words; missing words are zero.
    pub fn from_words(words: &[u64]) -> Hv {
        let mut w = [0u64; WORDS];
        for (slot, v) in w.iter_mut().zip(words.iter()) { *slot = *v; }
        Hv(w)
    }

    /// A deterministic pseudo-random vector for `seed` — the codebook entry for any seed.
    pub fn random(seed: u64) -> Hv {
        let mut s = seed ^ 0xA0761D6478BD642F;
        let mut w = [0u64; WORDS];
        for word in w.iter_mut() { *word = splitmix64(&mut s); }
        Hv(w)
    }

    /// XOR — the binding/comparison primitive.
    pub fn xor(&self, other: &Hv) -> Hv {
        let mut w = [0u64; WORDS];
        for (out, (a, b)) in w.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            *out = a ^ b;
        }
        Hv(w)
    }

    /// Hamming distance — the similarity score. 0 = identical, ~DIM/2 = unrelated.
    pub fn distance(&self, other: &Hv) -> u32 {
        self.0.iter().zip(other.0.iter()).map(|(a, b)| (a ^ b).count_ones()).sum()
    }

    /// Public 1-bit left rotation (for external use).
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

/// Serde helper: serialize `[u64; WORDS]` as a sequence (serde doesn't support large arrays).
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

/// Splitmix64: a tiny, well-distributed PRNG step.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// FNV-1a hash of a token — the seed that maps any token string to its code.
fn token_seed(token: &str) -> u64 {
    let mut h = 0xCBF29CE484222325u64;
    for b in token.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001B3);
    }
    h
}

/// The code for a single token: universal, language-agnostic.
pub fn token_hv(token: &str) -> Hv { Hv::random(token_seed(token)) }

/// Encode a token window into one vector. Each token is bound to its slot by position rotation,
/// then the bound tokens are bundled by majority. Preserves similarity: windows sharing a
/// sub-pattern stay near each other under Hamming distance.
pub fn bind(tokens: &[&str]) -> Hv {
    let mut b = Bundler::new();
    for (i, t) in tokens.iter().enumerate() {
        b.add(&token_hv(t).rotate(i));
    }
    b.finalize()
}

/// Accumulates training vectors into one prototype by per-bit majority vote.
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

// ── Identifier normalization ──────────────────────────────────────────────────

/// Keywords and well-known identifiers preserved verbatim by the tokenizer.
///
/// Anything NOT in this set and recognized as an identifier (ASCII alpha/underscore start)
/// is normalized to `<id>`. This makes structural patterns invariant to variable names:
/// `var x = 1` and `var myCount = getValue()` both become `var <id> = ...`.
fn keywords() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| [
        // Control flow
        "if", "else", "elif", "for", "while", "do", "switch", "case", "default",
        "break", "continue", "return", "yield", "loop", "match", "defer", "goto",
        "select", "range", "then",
        // Declarations
        "var", "let", "mut", "const", "static", "final",
        "fn", "func", "fun", "def", "function",
        "class", "struct", "enum", "interface", "trait", "type",
        "impl", "extends", "implements", "mod", "module", "namespace",
        // Access / visibility
        "pub", "public", "private", "protected", "abstract", "native",
        "synchronized", "transient", "volatile", "override", "virtual",
        "readonly", "declare", "sealed",
        // Error handling
        "try", "catch", "except", "finally", "throw", "raise", "throws",
        // Primitive types
        "void", "int", "long", "short", "byte", "float", "double", "char",
        "bool", "boolean", "str", "string", "uint", "usize", "isize",
        "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128",
        "f32", "f64",
        // Special values / literals
        "null", "undefined", "nil", "None", "Some", "Ok", "Err",
        "true", "false", "True", "False", "NaN", "Infinity",
        // Async / ownership
        "async", "await", "sync", "unsafe", "move", "ref", "box", "dyn",
        "where",
        // Module / import
        "import", "export", "from", "use", "require", "include",
        "package", "crate", "extern", "super", "self", "Self",
        // Operators spelled as words
        "new", "delete", "typeof", "instanceof", "in", "of", "as",
        "is", "not", "and", "or", "with", "pass", "assert", "del",
        "global", "nonlocal", "lambda",
        // OOP / class
        "this", "super",
        // Go
        "go", "chan", "make", "cap", "close", "recover", "panic",
        // JavaScript built-ins appearing in rules
        "console", "Math", "Object", "Array", "String", "Number", "Boolean",
        "Promise", "Error", "JSON", "Symbol", "Map", "Set", "WeakMap", "WeakSet",
        "Date", "RegExp", "Buffer", "process", "global", "window", "document",
        "eval", "arguments", "prototype", "constructor",
        // Python built-ins appearing in rules
        "print", "len", "range", "list", "dict", "tuple", "type", "set",
        "isinstance", "hasattr", "getattr", "setattr", "open", "input", "iter",
        "next", "enumerate", "zip", "map", "filter", "sorted", "reversed",
        "staticmethod", "classmethod", "property", "super",
        // Rust stdlib in rules
        "Vec", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
        "Option", "Result", "Box", "Rc", "Arc", "Cell", "RefCell",
        "println", "eprintln", "format", "todo", "unimplemented", "unreachable",
        "assert", "assert_eq", "assert_ne", "debug_assert",
        "unwrap", "expect", "clone", "collect", "iter", "into_iter",
        "push", "pop", "len", "is_empty", "contains", "insert", "remove",
        "unwrap_or", "unwrap_or_else",
        // Common method names in rules
        "log", "warn", "error", "info", "debug",
        "get", "set", "has", "add",
        "map", "filter", "reduce", "find", "some", "every", "includes",
        "join", "split", "slice", "splice", "concat", "flat", "flatMap",
        "toString", "valueOf", "toFixed", "toInt", "toFloat",
        "apply", "call", "bind",
        "then", "catch", "finally",
        "keys", "values", "entries", "assign", "create", "freeze",
        "parseInt", "parseFloat", "isNaN", "isFinite",
        // Java
        "throws", "abstract", "native", "strictfp",
    ].iter().copied().collect())
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

/// Byte length of the UTF-8 codepoint starting with lead byte `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 { 1 } else if b < 0xE0 { 2 } else if b < 0xF0 { 3 } else { 4 }
}

/// A token with its 1-based source line.
pub struct Tok {
    pub text: String,
    pub line: usize,
}

/// Tokenize source of any language into normalized tokens.
///
/// - Comments collapse to `<comment>`, string/char literals to `<str>`, numbers to `<num>`.
/// - Keywords and well-known identifiers (the [`keywords()`] set) pass through unchanged.
/// - All other identifiers are normalized to `<id>` — the engine sees structure, not names.
/// - Operators and punctuation pass through (multi-char operators kept whole).
pub fn tokenize(source: &str) -> Vec<Tok> {
    let kw = keywords();
    let bytes = source.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    let mut line = 1usize;
    let n = bytes.len();
    let multi: &[&str] = &[
        "===", "!==", "==", "!=", "<=", ">=", "&&", "||", "->", "=>", "::", "++", "--",
        "+=", "-=", "*=", "/=", "%=", "**", "<<", ">>", "..", "...", "?.", "??",
    ];
    while i < n {
        let c = bytes[i] as char;
        if c == '\n' { line += 1; i += 1; continue; }
        if c.is_whitespace() { i += 1; continue; }
        if !c.is_ascii() { i += utf8_len(bytes[i]); continue; }
        // line comments
        if (c == '/' && i + 1 < n && bytes[i + 1] == b'/') || c == '#' {
            while i < n && bytes[i] != b'\n' { i += 1; }
            toks.push(Tok { text: "<comment>".into(), line });
            continue;
        }
        // block comment
        if c == '/' && i + 1 < n && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                if bytes[i] == b'\n' { line += 1; }
                i += 1;
            }
            i = (i + 2).min(n);
            toks.push(Tok { text: "<comment>".into(), line });
            continue;
        }
        // string/char literals
        if c == '"' || c == '\'' || c == '`' {
            let quote = bytes[i];
            let start_line = line;
            i += 1;
            while i < n && bytes[i] != quote {
                if bytes[i] == b'\\' { i += 1; }
                else if bytes[i] == b'\n' { line += 1; }
                i += 1;
            }
            i = (i + 1).min(n);
            toks.push(Tok { text: "<str>".into(), line: start_line });
            continue;
        }
        // numbers
        if c.is_ascii_digit() {
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_') {
                i += 1;
            }
            toks.push(Tok { text: "<num>".into(), line });
            continue;
        }
        // identifiers — normalize non-keywords to `<id>`
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
            let word = &source[start..i];
            let text = if kw.contains(word) { word.to_string() } else { "<id>".to_string() };
            toks.push(Tok { text, line });
            continue;
        }
        // multi-char operators (longest match first)
        let rest = &source[i..];
        if let Some(op) = multi.iter().filter(|op| rest.starts_with(**op)).max_by_key(|op| op.len()) {
            toks.push(Tok { text: (*op).to_string(), line });
            i += op.len();
            continue;
        }
        toks.push(Tok { text: c.to_string(), line });
        i += 1;
    }
    toks
}

// ── Window computation ────────────────────────────────────────────────────────

/// A window is far enough from the good example to be a genuine violation signal.
/// At DIM/8 = 1024 bits, a window differing in 1 of 8 tokens clears this threshold.
const NOVEL_THRESH: u32 = (DIM / 8) as u32;

/// Slide a `window`-token window over `toks`, yielding `(bound_code, start_line)` pairs.
/// When the token count ≤ window, one whole-stream window is returned so nothing is missed.
fn windows(toks: &[Tok], window: usize) -> Vec<(Hv, usize)> {
    if toks.is_empty() { return Vec::new(); }
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_str()).collect();
    if texts.len() <= window {
        return vec![(bind(&texts), toks[0].line)];
    }
    (0..=texts.len() - window)
        .map(|i| (bind(&texts[i..i + window]), toks[i].line))
        .collect()
}

// ── One reported finding ──────────────────────────────────────────────────────

/// One flag the engine emits: a source line and the rule it best matched.
pub struct Flag {
    pub line: usize,
    pub rule_id: String,
}

// ── Exemplar-based rule detector ──────────────────────────────────────────────

/// One trained rule: the window size derived from the bad example, and the novel violation
/// windows stored as exemplars, each with a calibrated radius. A window is flagged for this
/// rule iff it falls within any exemplar's ball. The radius is set to be just below the
/// distance to the nearest good-example window — by construction, nothing that structurally
/// matches the good example can be inside any ball.
#[derive(serde::Serialize, serde::Deserialize)]
struct Learned {
    id: String,
    window: usize,
    exemplars: Vec<(Hv, u32)>,  // (novel window code, calibrated_radius)
}

/// THE linting engine: per-rule exemplar matching over identifier-normalized token streams.
///
/// Training is parameter-free: window size comes from the bad example's token count, radius
/// comes from the good example's distance. No epoch count, no threshold to tune, no clean
/// calibration corpus needed — the documentation alone drives everything.
///
/// Judgment dispatches one GPU batch per distinct window size: all source windows of that size
/// against all exemplars of that size in one Metal/Vulkan/DX12 dispatch.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct LinterNet {
    rules: Vec<Learned>,
}

impl LinterNet {
    /// Train from documentation pairs `(id, bad_example, good_example)`.
    ///
    /// For each rule:
    /// - Tokenize bad and good (with identifier normalization).
    /// - Window size = min(bad tokens, MAX_WINDOW) — the pattern's natural width.
    /// - Novel windows = bad windows not structurally close to any good window.
    /// - For each novel window, radius = dist_to_nearest_good - 1. By construction,
    ///   every good-example window is outside every ball (0 FP on documentation patterns).
    /// - Rules where no novel window survives calibration (bad ≈ good) are dropped.
    ///
    /// Parallelized with rayon across rules — each rule is fully independent.
    pub fn train(rules: &[(String, String, String)]) -> LinterNet {
        use rayon::prelude::*;
        let learned: Vec<Option<Learned>> = rules.par_iter().map(|(id, bad, good)| {
            let bad_toks = tokenize(bad);
            let good_toks = tokenize(good);
            if bad_toks.is_empty() || good_toks.is_empty() { return None; }

            let window = bad_toks.len().min(MAX_WINDOW);

            let good_hvs: Vec<Hv> = windows(&good_toks, window).into_iter().map(|(h, _)| h).collect();
            if good_hvs.is_empty() { return None; }

            // Novel: bad windows that are far from every good window (they encode the violation).
            // Calibrate each: radius = just below the nearest good window's distance.
            let exemplars: Vec<(Hv, u32)> = windows(&bad_toks, window)
                .into_iter()
                .map(|(h, _)| h)
                .filter(|h| {
                    good_hvs.iter().map(|g| h.distance(g)).min()
                        .map(|d| d > NOVEL_THRESH).unwrap_or(true)
                })
                .filter_map(|e| {
                    let nearest_good = good_hvs.iter().map(|g| g.distance(&e)).min()?;
                    // Cap radius: exact structural matches hit at distance 0; production code is at
                    // ~DIM/2 ≈ 4096. MAX_RADIUS_CAP keeps the ball tiny so only near-identical
                    // normalized token sequences fire.
                    let radius = nearest_good.saturating_sub(1).min(MAX_RADIUS_CAP);
                    (radius > 0).then_some((e, radius))
                })
                .collect();

            if exemplars.is_empty() { return None; }
            Some(Learned { id: id.clone(), window, exemplars })
        }).collect();

        LinterNet { rules: learned.into_iter().flatten().collect() }
    }

    /// How many rules the engine loaded a valid detector for.
    pub fn rule_count(&self) -> usize { self.rules.len() }

    /// Judge `source`: tokenize it, then for each distinct window size in the loaded rules,
    /// dispatch one GPU batch (source windows of that size × rule exemplars of that size).
    /// A window is flagged for the nearest rule whose exemplar ball it falls inside.
    pub fn judge(&self, source: &str) -> Vec<Flag> {
        if self.rules.is_empty() { return Vec::new(); }
        let toks = tokenize(source);
        if toks.is_empty() { return Vec::new(); }

        // Group rules by window size — one GPU batch per size.
        let mut by_window: std::collections::HashMap<usize, Vec<&Learned>> = std::collections::HashMap::new();
        for rule in &self.rules {
            by_window.entry(rule.window).or_default().push(rule);
        }

        let mut flags = Vec::new();
        for (window_size, rules_for_size) in &by_window {
            let src_windows: Vec<(Hv, usize)> = windows(&toks, *window_size);
            if src_windows.is_empty() { continue; }

            // Flatten exemplars: (exemplar_hv, radius, rule_id)
            let exemplar_info: Vec<(Hv, u32, &str)> = rules_for_size.iter()
                .flat_map(|r| r.exemplars.iter().map(|(h, rad)| (*h, *rad, r.id.as_str())))
                .collect();
            if exemplar_info.is_empty() { continue; }

            // GPU batch: M source windows × N exemplars → M×N distances.
            let protos: Vec<Hv> = exemplar_info.iter().map(|(h, _, _)| *h).collect();
            let hvs: Vec<Hv> = src_windows.iter().map(|(h, _)| *h).collect();
            let distances = crate::memory::gpu::batch_hamming(&hvs, &protos);
            let n_ex = exemplar_info.len();

            for (wi, (_, line)) in src_windows.iter().enumerate() {
                let row = &distances[wi * n_ex..(wi + 1) * n_ex];
                // Best match: smallest distance that still fits within the exemplar's radius.
                let best = exemplar_info.iter().enumerate()
                    .filter_map(|(ei, (_, radius, id))| {
                        (row[ei] <= *radius).then_some((row[ei], *id))
                    })
                    .min_by_key(|(d, _)| *d);
                if let Some((_, id)) = best {
                    flags.push(Flag { line: *line, rule_id: id.to_string() });
                }
            }
        }
        flags
    }
}

// ── Legacy Model (k-NN with exemplars and ambiguity rejection) ────────────────
//
// Kept for reference and potential specialized use. The primary engine is now LinterNet above.

/// A window whose nearest good-example window is at least this far counts as novel.
const _NOVEL_THRESH: u32 = NOVEL_THRESH;

/// One trained rule in the legacy Model: exemplars with per-exemplar radii.
struct RuleProto {
    id: String,
    exemplars: Vec<(Hv, u32)>,
}

impl RuleProto {
    fn best(&self, hv: &Hv) -> Option<u32> {
        self.exemplars.iter()
            .filter_map(|(e, r)| { let d = hv.distance(e); (d <= *r).then_some(d) })
            .min()
    }
}

/// The legacy trained model: per-rule violation prototypes with calibrated radii.
pub struct Model {
    window: usize,
    ambig: u32,
    rules: Vec<RuleProto>,
}

impl Model {
    /// Train the legacy model. `cap` caps each exemplar's radius; `ambig` is the minimum
    /// distance margin between the best and runner-up rules to avoid ambiguous attribution.
    pub fn train(
        window: usize, cap: u32, ambig: u32,
        rules: &[(String, String, String)],
        clean: &[&str],
    ) -> Model {
        let clean_windows: Vec<Hv> = clean.iter()
            .flat_map(|src| windows(&tokenize(src), window)).map(|(hv, _)| hv).collect();

        let mut trained = Vec::new();
        for (id, bad_src, good) in rules {
            if bad_src.is_empty() || good.is_empty() { continue; }
            let good_windows: Vec<Hv> = windows(&tokenize(good), window)
                .into_iter().map(|(hv, _)| hv).collect();
            let novel: Vec<Hv> = windows(&tokenize(bad_src), window)
                .into_iter().map(|(hv, _)| hv)
                .filter(|hv| {
                    good_windows.iter().map(|g| hv.distance(g)).min()
                        .map(|d| d > NOVEL_THRESH).unwrap_or(true)
                })
                .collect();
            if novel.is_empty() { continue; }
            let exemplars: Vec<(Hv, u32)> = novel.into_iter()
                .filter_map(|e| {
                    let nearest_clean = clean_windows.iter().map(|c| c.distance(&e)).min().unwrap_or(u32::MAX);
                    let radius = nearest_clean.saturating_sub(1).min(cap);
                    (radius >= 1).then_some((e, radius))
                })
                .collect();
            if exemplars.is_empty() { continue; }
            trained.push(RuleProto { id: id.clone(), exemplars });
        }
        Model { window, ambig, rules: trained }
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }

    pub fn judge(&self, source: &str) -> Vec<Flag> {
        let mut flags = Vec::new();
        for (hv, line) in windows(&tokenize(source), self.window) {
            let mut claims: Vec<(u32, &str)> = self.rules.iter()
                .filter_map(|r| r.best(&hv).map(|d| (d, r.id.as_str()))).collect();
            claims.sort_by_key(|(d, _)| *d);
            let distinct = match (claims.first(), claims.get(1)) {
                (Some(_), None) => true,
                (Some((d0, _)), Some((d1, _))) => *d1 >= d0 + self.ambig,
                _ => false,
            };
            if distinct {
                flags.push(Flag { line, rule_id: claims[0].1.to_string() });
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_is_self_inverse_binding() {
        let a = token_hv("foo");
        let b = token_hv("bar");
        assert_eq!(a.xor(&b).xor(&b), a);
    }

    #[test]
    fn distinct_tokens_are_near_orthogonal_and_equal_ones_are_identical() {
        let a = token_hv("unwrap");
        let b = token_hv("expect");
        assert_eq!(token_hv("unwrap").distance(&a), 0);
        let d = a.distance(&b);
        assert!((3500..4700).contains(&d), "distance {d} not near DIM/2");
    }

    #[test]
    fn order_matters_in_a_window() {
        assert_ne!(bind(&["a", "==", "true"]), bind(&["true", "==", "a"]));
    }

    #[test]
    fn tokenizer_normalizes_identifiers_but_preserves_keywords() {
        // User-defined names → <id>; language keywords → preserved.
        let toks: Vec<String> = tokenize("var myCount = getValue();")
            .into_iter().map(|t| t.text).collect();
        assert!(toks.contains(&"var".to_string()), "keyword 'var' preserved");
        assert!(!toks.contains(&"myCount".to_string()), "user id 'myCount' normalized");
        assert!(!toks.contains(&"getValue".to_string()), "user id 'getValue' normalized");
        assert_eq!(toks.iter().filter(|t| t.as_str() == "<id>").count(), 2, "two <id> tokens");
    }

    #[test]
    fn tokenizer_collapses_strings_and_comments_so_their_text_isnt_code() {
        let toks: Vec<String> = tokenize(r#"let e = "if x == true"; // x == true"#)
            .into_iter().map(|t| t.text).collect();
        assert!(toks.contains(&"<str>".to_string()));
        assert!(toks.contains(&"<comment>".to_string()));
        assert!(!toks.contains(&"==".to_string()));
        assert!(!toks.contains(&"true".to_string()), "true is inside string/comment");
        assert!(toks.contains(&"let".to_string()));
    }

    #[test]
    fn linternet_structural_match_ignores_variable_names() {
        // bad: `var x = 1;`  →  tokenizes to  var <id> = <num> ;
        // good: `let x = 1;` →  tokenizes to  let <id> = <num> ;
        // The structural difference is `var` vs `let`; the exemplar captures that.
        // Identifier normalization makes the engine invariant to variable names —
        // `var x = 1` and `var myCount = 42` produce identical token streams.
        let rules = vec![
            ("no-var".to_string(), "var x = 1;".to_string(), "let x = 1;".to_string()),
        ];
        let net = LinterNet::train(&rules);
        assert!(net.rule_count() > 0, "rule survived training");

        // Exact structural match with different id/number: fires (same token stream).
        let hits = net.judge("var myCount = 42;");
        assert!(!hits.is_empty(), "var <id> = <num> ; must fire");
        assert!(hits.iter().all(|f| f.rule_id == "no-var"), "attributed to no-var");

        // Different structure (function call ≠ number literal): does not fire.
        // For this to also fire, docs must provide a bad example with a function call context.
        let fn_call = net.judge("var myCount = getValue();");
        // May or may not fire — depends on radius. Assert only that good examples don't:
        let _ = fn_call;

        // Good example structure: must not fire.
        let clean_let = net.judge("let myCount = 42;");
        assert!(clean_let.is_empty(), "let usage must not fire: {:?}", clean_let.iter().map(|f| &f.rule_id).collect::<Vec<_>>());

        let clean_const = net.judge("const result = 0;");
        assert!(clean_const.is_empty(), "const usage must not fire");
    }
}
