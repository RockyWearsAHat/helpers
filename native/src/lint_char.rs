//! The character-level substrate (owner directive 2026-07-07: the atom is a UTF-8 character —
//! no token vocabulary, no frequency-by-word table, no cap). The reader ingests a stream of
//! Unicode scalars and learns to PREDICT the next one from the rolling context; the prediction
//! error (SURPRISE) is the one signal everything downstream stands on.
//!
//! This replaces the word-token substrate: "does English account for this?" becomes "does the
//! reader predict this character run with low surprise" (reading the dictionary entirely teaches
//! the model to spell English), and segmentation of a raw documentation page falls out of the
//! same signal — code runs are high-surprise, prose is low-surprise, a section boundary is a
//! surprise spike. No HTML parser, no tag list, no `Gap`.
//!
//! Unlike the old word reader, the prediction memory is PERSISTED (LINTER.md, "Markup second"):
//! a loaded reader must be able to read a page back, so its context→next-char associations ride
//! the artifact. The memory is bounded ([`MEM_CAP`] slots), so the substrate stays small.

use std::collections::HashMap;

use crate::lint_ai::{Bundler, Hv, DIM};

/// Prediction memory capacity — how many context→next-char associations the reader keeps. A
/// character model's useful context is the last handful of scalars, so the reachable context
/// space is far smaller than a word model's; this bounds the persisted artifact.
const MEM_CAP: usize = 1 << 20;

/// Two predicted/actual character codes closer than this (Hamming) count as a correct
/// prediction — the reader only rewrites a slot on a real miss. A quarter of the space is the
/// same conservative radius the word reader used.
const SURPRISE_RADIUS: u32 = (DIM / 4) as u32;

/// The character-level predictive reader. All operations are 1-bit (XOR / rotate / Hamming);
/// deterministic, no floats in the learned state.
#[derive(Clone, Default)]
pub struct CharReader {
    /// Context address → predicted next-character code. Bounded by [`MEM_CAP`]. PERSISTED — a
    /// loaded reader reads pages by this memory.
    mem: HashMap<u32, Hv>,
    /// Total characters read — the mass surprise averages are measured against.
    total: u64,
}

/// The fixed basis: one hypervector per Unicode scalar. No vocabulary is stored — the code is a
/// pure function of the character, so any scalar (any language, any symbol) is encodable, and
/// the "basis" is the character set itself.
pub fn char_hv(c: char) -> Hv {
    // Salt keeps char codes clear of token-seed collisions in shared structures.
    Hv::random((c as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ 0xC0FFEE)
}

/// A stable u32 address for a context — the slot the predictor reads and writes (same hash the
/// word reader used; conservative, not an LSH, so the reader only "knows" a continuation in a
/// context it has literally seen).
fn ctx_key(ctx: &Hv) -> u32 {
    let mut h = 0xCBF29CE484222325u64;
    for w in ctx.as_words() {
        h = h.rotate_left(5) ^ *w;
        h = h.wrapping_mul(0x100000001B3);
    }
    (h ^ (h >> 32)) as u32
}

/// How many preceding characters form the prediction context — a fixed-order model (the last
/// [`ORDER`] characters predict the next). Bounded so the context GENERALIZES: "th" predicts
/// "e" wherever it occurs, not only after an exact prefix the reader saw before. A rolling
/// hash of the whole history never generalizes and every unseen prefix reads as maximal
/// surprise — measured, and the reason the first cut barely separated English from code.
const ORDER: usize = 4;

/// The context address for the characters preceding position `i` in `chars` — the XOR of the
/// last [`ORDER`] character codes, each bound to its distance back by rotation (so order
/// matters and a shorter available history still addresses a slot).
fn context(chars: &[char], i: usize) -> Hv {
    let mut ctx = Hv::zero();
    let start = i.saturating_sub(ORDER);
    for (dist, c) in chars[start..i].iter().rev().enumerate() {
        ctx = ctx.xor(&rotate_by(&char_hv(*c), dist + 1));
    }
    ctx
}

impl CharReader {
    pub fn new() -> CharReader {
        CharReader { mem: HashMap::new(), total: 0 }
    }

    /// Characters read so far.
    pub fn total_read(&self) -> u64 {
        self.total
    }

    /// Learned context slots — how much spelling the reader has comprehended.
    pub fn learned(&self) -> usize {
        self.mem.len()
    }

    /// READ a span and LEARN from it (predictive coding): at each character, when the memory's
    /// prediction for the current context is wrong or absent, rewrite that one slot to expect
    /// this character. Correct predictions touch nothing.
    pub fn learn(&mut self, text: &str) {
        let chars: Vec<char> = text.chars().collect();
        for i in 0..chars.len() {
            let h = char_hv(chars[i]);
            let key = ctx_key(&context(&chars, i));
            let miss = self.mem.get(&key).is_none_or(|p| p.distance(&h) > SURPRISE_RADIUS);
            if miss && (self.mem.len() < MEM_CAP || self.mem.contains_key(&key)) {
                self.mem.insert(key, h);
            }
            self.total += 1;
        }
    }

    /// The SURPRISE of reading `text` under the learned model, in mean bits of prediction error
    /// per character (0 = every character was predicted exactly; ~DIM/2 = wholly unpredictable).
    /// This is the English-vs-code signal: prose the reader learned scores low, code and unseen
    /// identifiers score high. Read-only — never learns.
    pub fn surprise(&self, text: &str) -> u32 {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return 0;
        }
        let mut sum = 0u64;
        for i in 0..chars.len() {
            let h = char_hv(chars[i]);
            let key = ctx_key(&context(&chars, i));
            // No prediction in this context = maximally surprising (half the space).
            let d = self.mem.get(&key).map_or((DIM / 2) as u32, |p| p.distance(&h));
            sum += u64::from(d);
        }
        (sum / chars.len() as u64) as u32
    }

    /// Encode a span into ONE hypervector self-encoded from its characters: the position-bound
    /// bundle of its character codes (a spelling centroid). Any string — word, identifier,
    /// phrase — maps deterministically; strings that share spelling share geometry. This is the
    /// representation polarity and concept matching stand on.
    pub fn encode(&self, text: &str) -> Option<Hv> {
        let mut b = Bundler::new();
        let mut any = false;
        for (i, c) in text.chars().enumerate() {
            // Position-bound so order matters (`ab` ≠ `ba`); rotation is the positional role.
            b.add(&rotate_by(&char_hv(c), i));
            any = true;
        }
        any.then(|| b.finalize())
    }
}

/// Rotate an Hv left by `k` bits (positional binding) — `k` capped to the dimension.
fn rotate_by(hv: &Hv, k: usize) -> Hv {
    let mut v = *hv;
    for _ in 0..(k % DIM) {
        v = v.rotl1_pub();
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English prose the reader has read must be far LESS surprising than code it has not —
    /// the hermetic floor of the whole rewrite (the real-dictionary gate is the ignored test
    /// below). The reader trains on English sentences, then judges unseen English vs code.
    #[test]
    fn english_reads_calmer_than_code() {
        let mut r = CharReader::new();
        // A small English corpus — ordinary words, ordinary letter sequences.
        let corpus = "the quick brown fox jumps over the lazy dog. \
             a function returns a value to the caller when it is done. \
             the reader learns to predict the next letter in a word. \
             common english words have common letter sequences and endings. \
             she sells sea shells by the sea shore in the summer season. ";
        // Read it enough times that the letter statistics settle.
        for _ in 0..40 {
            r.learn(corpus);
        }
        let prose = r.surprise("the reader returns a common value to the caller");
        let code = r.surprise("xq7_frobnicate(items[0], cfg={k:v}); z9$->w=~q;");
        assert!(
            prose + prose / 2 < code,
            "English prose ({prose} bits) must read far calmer than code ({code} bits)"
        );
        // A single English-shaped word vs a symbol-laden identifier.
        assert!(
            r.surprise("season") < r.surprise("z9$_qx=[]"),
            "an English word ({}) reads calmer than a code token ({})",
            r.surprise("season"),
            r.surprise("z9$_qx=[]"),
        );
    }

    /// The CURRICULUM property (owner directive 2026-07-07): one brain, trained cumulatively —
    /// reading HTML must RETAIN the English it already learned while GAINING HTML. Because
    /// learning only adds context→prediction slots, prior knowledge is never overwritten; new
    /// structure layers on top. This is what lets the brain read English → HTML → CSS → JS and
    /// then read any documentation directly.
    #[test]
    fn reading_html_retains_english_and_gains_html() {
        let english = "the quick brown fox jumps over the lazy dog. a function returns a \
            value to the caller. common english words share common letter sequences. she \
            sells sea shells by the sea shore every single summer season without fail. ";
        let html = "<div class=\"box\"><p>the value</p></div><span id=\"x\">text</span>\
            <ul><li>one</li><li>two</li></ul><a href=\"/page\">link</a><h2>Title</h2>";
        let mut r = CharReader::new();
        for _ in 0..40 {
            r.learn(english);
        }
        let e0 = r.surprise("a common function returns the value to the caller");
        let h0 = r.surprise("<div class=\"row\"><p>hello</p></div>");
        // Now layer HTML on top — English is never re-read.
        for _ in 0..40 {
            r.learn(html);
        }
        let e1 = r.surprise("a common function returns the value to the caller");
        let h1 = r.surprise("<div class=\"row\"><p>hello</p></div>");
        // RETAINS: English surprise barely moves (knowledge is not overwritten).
        assert!(e1 <= e0 + e0 / 8, "English retained: was {e0}, now {e1}");
        // GAINS: HTML surprise drops materially (new structure learned).
        assert!(h1 + h1 / 4 < h0, "HTML learned: was {h0}, now {h1}");
    }

    /// THE CURRICULUM, on real data (owner directive 2026-07-07): reading is uniform character
    /// prediction, English is the general base, and each language is the SAME method continued
    /// from there. Code is surprising only UNTIL the reader reads it — so the property to prove
    /// is not a permanent English/code split but that training on a language DROPS its surprise
    /// while English is RETAINED. Ignored (needs the local dictionary); run:
    /// `cargo test --release --lib char_curriculum_on_real_dictionary -- --ignored --nocapture`
    #[test]
    #[ignore = "reads the local dictionary; demonstrates the cumulative curriculum on real data"]
    fn char_curriculum_on_real_dictionary() {
        let prose = crate::lint_english::dictionary_prose_sample()
            .expect("this machine has a readable dictionary");
        let mut r = CharReader::new();
        r.learn(&prose); // the general base: English
        eprintln!("English base: {} chars, {} slots", r.total_read(), r.learned());
        let english = "the value is returned to the caller when the function is done";
        let code_sample = "let x = arr[0]; for (const k of items) { obj[k] = fn(k) ?? 0; } \
            function step(a, b) { return a === b ? a : b; } const y = eval(cfg._k);";
        let e_before = r.surprise(english);
        let c_before = r.surprise("const z = arr[1] ?? fn(items[0]);");
        eprintln!("before code: english ~{e_before}  code ~{c_before}");
        // Continue the SAME training on code — the specific layered onto the general.
        for _ in 0..200 {
            r.learn(code_sample);
        }
        let e_after = r.surprise(english);
        let c_after = r.surprise("const z = arr[1] ?? fn(items[0]);");
        eprintln!("after code:  english ~{e_after}  code ~{c_after}");
        assert!(c_after + c_after / 8 < c_before, "the language was learned: code {c_before} -> {c_after}");
        assert!(e_after <= e_before + e_before / 8, "English retained: {e_before} -> {e_after}");
    }
}
