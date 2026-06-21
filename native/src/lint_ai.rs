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
        for i in 0..WORDS {
            w[i] = self.0[i] ^ other.0[i];
        }
        Hv(w)
    }

    /// Number of differing bits (`popcount` of the XOR). 0 ⇒ identical; ~`DIM/2` ⇒
    /// unrelated. This is the only "score" in the engine — smaller is more similar.
    pub fn distance(&self, other: &Hv) -> u32 {
        let mut d = 0;
        for i in 0..WORDS {
            d += (self.0[i] ^ other.0[i]).count_ones();
        }
        d
    }

    /// Rotate all `DIM` bits left by one. Composed `k` times this encodes position `k`,
    /// so a token bound at slot `k` is distinguishable from the same token at slot `j`.
    fn rotl1(&self) -> Hv {
        let mut w = [0u64; WORDS];
        let top = self.0[WORDS - 1] >> 63; // wraps around into bit 0
        for i in 0..WORDS {
            let carry_in = if i == 0 { top } else { self.0[i - 1] >> 63 };
            w[i] = (self.0[i] << 1) | carry_in;
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
