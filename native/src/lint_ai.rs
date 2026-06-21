//! `lint_ai` — the 1-bit XOR associative engine: the model that *is* the linter.
//!
//! No tree-sitter, no grammar, no per-rule code. Everything is a fixed-width binary
//! hypervector and the only operations are XOR (bind/compare), bit-rotate (order), and
//! majority (bundle). That makes it 1-bit, float-free, and **open-vocabulary**: any
//! token of any language maps to a code by hashing, so the model isn't built for a
//! language — it ingests whatever it's shown. Knowledge of "all the languages" is just
//! the codebook being universal.
//!
//! How it represents code:
//!   * `Hv` — a `DIM`-bit vector packed into `u64`s.
//!   * `token_hv(t)` — a token's code: a deterministic random `Hv` seeded by the token
//!     hash. Same token ⇒ same code, everywhere, in every language.
//!   * a window of tokens ⇒ one `Hv` by XOR-binding each token under a position
//!     rotation (`bind`), so order matters and the whole window is one code.
//!   * a *class* (e.g. a rule's bad pattern, or "clean") ⇒ a prototype `Hv`: the
//!     majority-bundle of all its training windows, thresholded back to 1 bit.
//!
//! How it judges: encode a window, XOR it against each prototype, count set bits
//! (`distance`). Nearest prototype wins. A window is flagged for a rule only when it is
//! closer to that rule's bad prototype than to the clean prototype by a margin — the
//! associative form of "matches the documented bad example, not the good one".

/// Hypervector width in bits. 8192 bits = 1 KiB/vector — wide enough that random codes
/// are near-orthogonal (binding/bundling stay separable), small enough to be cheap.
pub const DIM: usize = 8192;
const WORDS: usize = DIM / 64;

/// A `DIM`-bit binary hypervector. The one and only representation the engine uses;
/// all knowledge lives as these. `Copy` so windows compose without allocation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hv([u64; WORDS]);

impl Hv {
    /// The all-zero vector — the identity for XOR and the empty bundle.
    pub fn zero() -> Hv {
        Hv([0; WORDS])
    }

    /// A deterministic pseudo-random vector for `seed`. Filling every bit from a
    /// splitmix64 stream makes independent seeds near-orthogonal, which is what lets
    /// XOR-binding and majority-bundling stay separable.
    pub fn random(seed: u64) -> Hv {
        let mut s = seed ^ 0xA0761D6478BD642F;
        let mut w = [0u64; WORDS];
        for word in w.iter_mut() {
            *word = splitmix64(&mut s);
        }
        Hv(w)
    }

    /// XOR — the binding/comparison primitive. Self-inverse: `a.xor(b).xor(b) == a`.
    pub fn xor(&self, other: &Hv) -> Hv {
        let mut w = [0u64; WORDS];
        for (out, (a, b)) in w.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            *out = a ^ b;
        }
        Hv(w)
    }

    /// Number of differing bits (`popcount` of the XOR). 0 ⇒ identical; ~`DIM/2` ⇒
    /// unrelated. This is the only "score" in the engine — smaller is more similar.
    pub fn distance(&self, other: &Hv) -> u32 {
        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum()
    }

    /// Rotate all `DIM` bits left by one. Composed `k` times this encodes position `k`,
    /// so a token bound at slot `k` is distinguishable from the same token at slot `j`.
    fn rotl1(&self) -> Hv {
        let mut w = [0u64; WORDS];
        let top = self.0[WORDS - 1] >> 63; // wraps around into bit 0
        for (i, out) in w.iter_mut().enumerate() {
            let carry_in = if i == 0 { top } else { self.0[i - 1] >> 63 };
            *out = (self.0[i] << 1) | carry_in;
        }
        Hv(w)
    }

    /// This vector rotated left by `k` bits — the position-`k` role transform.
    fn rotate(&self, k: usize) -> Hv {
        let mut v = *self;
        for _ in 0..(k % DIM) {
            v = v.rotl1();
        }
        v
    }
}

/// Splitmix64: a tiny, well-distributed PRNG step. Deterministic codebook fuel.
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

/// The code for a single token: a universal, language-agnostic codebook entry.
pub fn token_hv(token: &str) -> Hv {
    Hv::random(token_seed(token))
}

/// Encode a token window into one vector. Each token is *bound* to its slot by a
/// position rotation (so order matters), then the bound tokens are *bundled* by
/// majority into one code. Bundling — not XOR-folding — is what preserves similarity:
/// two windows that share a sub-pattern (say `== true`) keep that pattern's bits in
/// common, so they stay near each other under the XOR distance. (XOR-folding would
/// cancel the shared part and erase exactly the signal we match on.) Fixed-width, so
/// any window of any length in any language becomes one comparable code.
pub fn bind(tokens: &[&str]) -> Hv {
    let mut b = Bundler::new();
    for (i, t) in tokens.iter().enumerate() {
        b.add(&token_hv(t).rotate(i));
    }
    b.finalize()
}

/// Accumulates training vectors into one prototype by per-bit majority vote. Storage
/// of the result is 1-bit; only the running tally is integer, thresholded on finalize.
pub struct Bundler {
    counts: Vec<i32>,
    n: usize,
}

impl Bundler {
    /// An empty bundle.
    pub fn new() -> Bundler {
        Bundler {
            counts: vec![0; DIM],
            n: 0,
        }
    }

    /// Fold one example vector into the running majority tally.
    pub fn add(&mut self, hv: &Hv) {
        for bit in 0..DIM {
            let set = (hv.0[bit / 64] >> (bit % 64)) & 1 == 1;
            self.counts[bit] += if set { 1 } else { -1 };
        }
        self.n += 1;
    }

    /// How many examples have been bundled.
    pub fn len(&self) -> usize {
        self.n
    }

    /// True when nothing has been bundled yet.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Threshold the tally back to a 1-bit prototype: a bit is set iff it was set in the
    /// majority of examples. Ties (count 0) resolve to 0.
    pub fn finalize(&self) -> Hv {
        let mut w = [0u64; WORDS];
        for bit in 0..DIM {
            if self.counts[bit] > 0 {
                w[bit / 64] |= 1 << (bit % 64);
            }
        }
        Hv(w)
    }
}

impl Default for Bundler {
    fn default() -> Self {
        Bundler::new()
    }
}

/// Byte length of the UTF-8 codepoint that starts with lead byte `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

/// A token with its 1-based source line. Tokens are the model's only view of code.
pub struct Tok {
    /// The normalized token text (a codebook key).
    pub text: String,
    /// 1-based source line the token starts on.
    pub line: usize,
}

/// Tokenize source of *any* language into normalized tokens. This is a universal
/// lexer, not a grammar: identifiers/keywords pass through (so language keywords and
/// API names become codebook entries), numbers collapse to `<num>`, string and comment
/// bodies collapse to `<str>`/`<comment>` so text *inside* them is never seen as code,
/// and operators/punctuation pass through (multi-char operators kept whole). Anything
/// it doesn't recognize still becomes a single-char token — nothing is dropped, so the
/// model can learn structure from whatever it's shown.
pub fn tokenize(source: &str) -> Vec<Tok> {
    let bytes = source.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    let mut line = 1usize;
    let n = bytes.len();
    let multi: &[&str] = &[
        "==", "!=", "<=", ">=", "&&", "||", "->", "=>", "::", "++", "--", "+=", "-=", "*=", "/=",
        "%=", "**", "<<", ">>", "..", "...", "?.", "??",
    ];
    while i < n {
        let c = bytes[i] as char;
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // non-ASCII byte outside a string/comment: skip the whole UTF-8 codepoint so we
        // never slice mid-character (code tokens are ASCII; this is stray text/symbols).
        if !c.is_ascii() {
            i += utf8_len(bytes[i]);
            continue;
        }
        // line comments: // ... and # ...
        if (c == '/' && i + 1 < n && bytes[i + 1] == b'/') || c == '#' {
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            toks.push(Tok { text: "<comment>".into(), line });
            continue;
        }
        // block comment: /* ... */
        if c == '/' && i + 1 < n && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                if bytes[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            i = (i + 2).min(n);
            toks.push(Tok { text: "<comment>".into(), line });
            continue;
        }
        // string/char literals: collapse the whole body so its contents aren't code
        if c == '"' || c == '\'' || c == '`' {
            let quote = bytes[i];
            let start_line = line;
            i += 1;
            while i < n && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1; // skip escaped char
                } else if bytes[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            i = (i + 1).min(n);
            toks.push(Tok { text: "<str>".into(), line: start_line });
            continue;
        }
        // numbers collapse to a single class token
        if c.is_ascii_digit() {
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_') {
                i += 1;
            }
            toks.push(Tok { text: "<num>".into(), line });
            continue;
        }
        // identifiers / keywords
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            toks.push(Tok { text: source[start..i].to_string(), line });
            continue;
        }
        // multi-char operators (longest match), else a single punctuation char
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

/// One flag the model emits: a source line and the rule it best matched.
pub struct Flag {
    /// 1-based source line of the offending window.
    pub line: usize,
    /// The id of the rule whose bad prototype the window matched.
    pub rule_id: String,
}

/// A window whose nearest good-example window is at least this far away counts as
/// *novel* — it represents the violation, not the shared scaffolding. Token-identical
/// windows are distance 0; changing even one token moves a window far past this, so the
/// threshold cleanly separates "present in the good example too" from "new in the bad".
const NOVEL_THRESH: u32 = (DIM / 8) as u32;

/// One trained rule: its id and its violation windows kept as sharp exemplars, each
/// paired with its OWN decision radius. A window flags the rule iff it lies within some
/// exemplar's radius — k-nearest-neighbour association with a per-exemplar, repo-
/// calibrated boundary. Per-exemplar radii (not one rule-wide value) let a distinctive
/// violation window keep a wide ball while a window that brushes clean code gets a tiny
/// one, instead of one near-clean window collapsing the whole rule.
struct RuleProto {
    id: String,
    exemplars: Vec<(Hv, u32)>,
}

impl RuleProto {
    /// Distance to the nearest exemplar whose own radius admits `hv`, if any — the
    /// rule's match score (lower = closer). `None` ⇒ no exemplar claims this window.
    fn best(&self, hv: &Hv) -> Option<u32> {
        self.exemplars
            .iter()
            .filter_map(|(e, r)| {
                let d = hv.distance(e);
                (d <= *r).then_some(d)
            })
            .min()
    }
}

/// The trained linter: per-rule violation prototypes, each with a radius calibrated so
/// no window of the clean repo falls inside it. Judgment is pure association — no
/// grammar, no per-rule code — and the calibration is what holds false positives down.
pub struct Model {
    window: usize,
    ambig: u32,
    rules: Vec<RuleProto>,
}

/// Slide a `window`-token window over `toks`, yielding each window's code and start
/// line. Short token streams yield a single whole-stream window so nothing is missed.
fn windows(toks: &[Tok], window: usize) -> Vec<(Hv, usize)> {
    if toks.is_empty() {
        return Vec::new();
    }
    let texts: Vec<&str> = toks.iter().map(|t| t.text.as_str()).collect();
    if texts.len() <= window {
        return vec![(bind(&texts), toks[0].line)];
    }
    (0..=texts.len() - window)
        .map(|i| (bind(&texts[i..i + window]), toks[i].line))
        .collect()
}

impl Model {
    /// Train from the docs. `rules` is `(id, exampleBad, exampleGood)`; `clean` is
    /// known-good source — the repo itself — used to calibrate boundaries. For each
    /// rule: keep the windows of its bad example that are novel vs its good example (the
    /// violation) as exemplars, then set the decision radius to the smaller of `cap` and
    /// **just below the nearest clean-repo window**, so the rule cannot fire on
    /// known-good code. `cap` is the one knob trading recall (larger ⇒ catches more
    /// variants) against false flags (smaller ⇒ stricter). A rule whose radius collapses
    /// to zero (clean code sits on its violation) is dropped — never a guess. The result
    /// false-flags on none of `clean` by construction. `ambig` is the judging margin: a
    /// window is only flagged when its best rule beats the runner-up by that many bits.
    pub fn train(
        window: usize,
        cap: u32,
        ambig: u32,
        rules: &[(String, String, String)],
        clean: &[&str],
    ) -> Model {
        let clean_windows: Vec<Hv> = clean
            .iter()
            .flat_map(|src| windows(&tokenize(src), window))
            .map(|(hv, _)| hv)
            .collect();

        let mut trained = Vec::new();
        for (id, bad_src, good) in rules {
            if bad_src.is_empty() || good.is_empty() {
                continue; // ungroundable: needs both examples to find the novel windows
            }
            let good_windows: Vec<Hv> = windows(&tokenize(good), window)
                .into_iter()
                .map(|(hv, _)| hv)
                .collect();
            // the violation = bad windows with no near-equivalent in the good example
            let novel: Vec<Hv> = windows(&tokenize(bad_src), window)
                .into_iter()
                .map(|(hv, _)| hv)
                .filter(|hv| {
                    good_windows
                        .iter()
                        .map(|g| hv.distance(g))
                        .min()
                        .map(|d| d > NOVEL_THRESH)
                        .unwrap_or(true)
                })
                .collect();
            if novel.is_empty() {
                continue; // bad and good are structurally the same here — nothing to flag
            }
            // Give each violation window its OWN radius: just under its nearest clean
            // window, capped at `cap`. A window distinctive from clean code earns a wide
            // ball (up to `cap`); one that brushes clean code gets a tiny ball and so
            // effectively only fires on near-exact repeats. Windows that sit on clean
            // code (radius 0) are dropped — they're indistinguishable from known-good and
            // can't be a trustworthy fingerprint. No clean calibration window lies inside
            // any kept ball, by construction.
            let exemplars: Vec<(Hv, u32)> = novel
                .into_iter()
                .filter_map(|e| {
                    let nearest_clean = clean_windows
                        .iter()
                        .map(|c| c.distance(&e))
                        .min()
                        .unwrap_or(u32::MAX);
                    let radius = nearest_clean.saturating_sub(1).min(cap);
                    (radius >= 1).then_some((e, radius))
                })
                .collect();
            if exemplars.is_empty() {
                continue; // every window of this violation sits on known-good code
            }
            trained.push(RuleProto {
                id: id.clone(),
                exemplars,
            });
        }
        Model {
            window,
            ambig,
            rules: trained,
        }
    }

    /// How many rules survived training as separable, repo-calibrated prototypes.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Judge `source`: flag each window that lies within some rule's radius of its
    /// prototype, attributing the window to its nearest such rule. By calibration, no
    /// window matching the clean repo can fire.
    pub fn judge(&self, source: &str) -> Vec<Flag> {
        let mut flags = Vec::new();
        for (hv, line) in windows(&tokenize(source), self.window) {
            // every rule that claims this window, nearest first
            let mut claims: Vec<(u32, &str)> = self
                .rules
                .iter()
                .filter_map(|r| r.best(&hv).map(|d| (d, r.id.as_str())))
                .collect();
            claims.sort_by_key(|(d, _)| *d);
            // AMBIGUITY REJECTION: flag only when one rule owns the window distinctly —
            // the runner-up must be at least `self.ambig` bits farther. If two rules
            // match almost equally, the window isn't a fingerprint of either, so the
            // attribution can't be trusted and we stay silent (never a wrong-rule flag).
            let distinct = match (claims.first(), claims.get(1)) {
                (Some(_), None) => true,
                (Some((d0, _)), Some((d1, _))) => *d1 >= d0 + self.ambig,
                _ => false,
            };
            if distinct {
                flags.push(Flag {
                    line,
                    rule_id: claims[0].1.to_string(),
                });
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
        // unbinding b recovers a exactly — the property all association relies on
        assert_eq!(a.xor(&b).xor(&b), a);
    }

    #[test]
    fn distinct_tokens_are_near_orthogonal_and_equal_ones_are_identical() {
        let a = token_hv("unwrap");
        let b = token_hv("expect");
        assert_eq!(token_hv("unwrap").distance(&a), 0); // same token ⇒ same code
        let d = a.distance(&b);
        // independent codes differ in roughly half their bits (DIM/2 = 4096)
        assert!((3500..4700).contains(&d), "distance {d} not near DIM/2");
    }

    #[test]
    fn order_matters_in_a_window() {
        // same tokens, different order ⇒ different code (position rotation works)
        assert_ne!(bind(&["a", "==", "true"]), bind(&["true", "==", "a"]));
    }

    #[test]
    fn tokenizer_collapses_strings_and_comments_so_their_text_isnt_code() {
        let toks: Vec<String> = tokenize(r#"let e = "if x == true"; // x == true"#)
            .into_iter()
            .map(|t| t.text)
            .collect();
        // the `== true` inside the string and the comment never appear as tokens
        assert!(toks.contains(&"<str>".to_string()));
        assert!(toks.contains(&"<comment>".to_string()));
        assert!(!toks.contains(&"==".to_string()));
        assert!(!toks.contains(&"true".to_string()));
        // real code tokens survive
        assert!(toks.contains(&"let".to_string()));
    }

    #[test]
    fn trained_model_flags_the_bad_pattern_not_clean_or_strings() {
        let rules = vec![(
            "bool_comparison".to_string(),
            "fn f(x: bool) { if x == true { g() } }".to_string(),
            "fn f(x: bool) { if x { g() } }".to_string(),
        )];
        let clean = [
            "fn a(x: i32) -> i32 { x + 1 }",
            "fn b(v: Vec<i32>) { for e in v { use_it(e) } }",
            "fn c(s: String) { print(s) }",
        ];
        let model = Model::train(4, 2048, 0, &rules, &clean);
        assert_eq!(model.rule_count(), 1, "the rule should be separable");
        // real violation in unseen code is flagged
        let bad = model.judge("fn h(flag: bool) { if flag == true { do_thing() } }");
        assert!(bad.iter().any(|f| f.rule_id == "bool_comparison"));
        // clean code is not flagged
        assert!(model.judge("fn h(flag: bool) { if flag { do_thing() } }").is_empty());
        // the pattern only inside a string is not code, so not flagged
        assert!(model.judge(r#"fn h() { let msg = "if flag == true"; log(msg); }"#).is_empty());
    }

    #[test]
    fn a_bundled_prototype_recognizes_its_own_pattern() {
        // "train" a bad-prototype on windows that share `== true`, a clean one without.
        let mut bad = Bundler::new();
        for ctx in [["x", "==", "true"], ["y", "==", "true"], ["z", "==", "true"]] {
            bad.add(&bind(&ctx));
        }
        let mut clean = Bundler::new();
        for ctx in [["x", "&&", "y"], ["a", "+", "b"], ["p", "||", "q"]] {
            clean.add(&bind(&ctx));
        }
        let bad_p = bad.finalize();
        let clean_p = clean.finalize();
        // an unseen `== true` window is nearer the bad prototype than the clean one
        let probe = bind(&["w", "==", "true"]);
        assert!(probe.distance(&bad_p) < probe.distance(&clean_p));
        // and a clean window is nearer the clean prototype
        let ok = bind(&["m", "+", "n"]);
        assert!(ok.distance(&clean_p) < ok.distance(&bad_p));
    }
}
