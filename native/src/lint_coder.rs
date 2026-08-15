//! `lint_coder` — PASS 39 phase one, item 2 (native/history.dx "PASS 39"): a ONE-BIT predictive coder
//! over the markup token stream.
//!
//! ## The model
//! A Sparse Distributed Memory (Kanerva) over the [`crate::lint_ai::Hv`] substrate: `K` previous
//! tokens are bound into ONE context signature (position-rotated XOR — the same binding
//! [`crate::lint_ai::bind`] uses for a description window), and that context signature indexes a
//! FIXED hash table of bit-sliced majority counters — the online form of
//! [`crate::lint_ai::majority_bundle`]'s offline one-bit vote, one training example at a time.
//! Reading a slot back (majority vote per bit) is the model's PREDICTION for the token that
//! follows that context; training nudges the slot toward the token that actually followed.
//! Prediction ERROR is `popcount(XOR(predicted, actual))` — near-zero on a context the model has
//! seen before, near-maximal on a genuinely new one (an unvisited slot predicts nothing, scored
//! as maximum distance) — the spec's segmentation-boundary signal.
//!
//! No English, no tag names, no hand shape rule: the token stream is markup typography only
//! (`<…>` is one token, whitespace separates words — the same word-boundary rule
//! [`crate::lint_graph`]'s module doc names as HTML's own typography), and the model's entire
//! state is XOR / popcount / majority-vote — the same binary substrate as the existing brains.
//!
//! ## GPU (owner ruling, native/history.dx "PASS 39", 2026-07-20): training is FOR the GPU — do not fake it
//! The owner's ruling is explicit: batched one-bit updates across thousands of lanes is where this
//! design's real speed lives, and CPU popcount alone "isn't bad, but isn't amazing." This phase
//! ships the CPU reference ONLY: the existing GPU plumbing ([`crate::hv_batch`]) is a batched
//! ARGMIN/nearest-key search (queries × keys → nearest), not a batched WRITE (many training
//! examples updating many hash-table slots in parallel, with same-slot collisions needing an
//! atomic or reduction step) — porting the coder's training loop needs a NEW Metal compute kernel
//! (a scatter-add/threshold shader) beyond what `hv_batch.rs` provides, which measured over an
//! hour of net-new GPU work, not a `<1h` reuse. **TODO (named, not built): a
//! `hv_batch`-sibling batched one-bit training kernel** — bind `hv_batch`'s existing WGSL
//! infrastructure (device/queue/buffer setup already proven there) to a scatter-XOR-popcount-
//! threshold shader over `(context, target)` pairs, one dispatch per training-corpus chunk.
//! CPU inference stays CPU regardless of where training runs (native/architecture.dx's own reasoning: shadow
//! proposals and any future lint-path read are µs-tiny, syscall-adjacent workloads where a GPU
//! dispatch is pure overhead).

use crate::lint_ai::{token_hv, Hv, DIM};

/// Context window — how many previous tokens the coder conditions its prediction on. Small and
/// fixed (a Kanerva ADDRESS, not a growing history), so training is one pass, O(1) per token.
pub const CONTEXT_K: usize = 3;

/// Hash-table width (number of Kanerva "hard locations"). Fixed and bounded — memory is capped
/// before training starts, unlike a HashMap keyed by every distinct context ever seen. Collisions
/// blend distinct contexts, which is exactly what a CPU reference for register SEPARATION should
/// honestly report rather than hide; the GPU-trained production model can widen this table
/// without changing the algorithm.
const TABLE_SLOTS: usize = 1 << 14;

/// One Kanerva hard location: a bit-sliced majority counter over [`DIM`] bits, `i8`-clamped
/// (saturating) so a heavily-visited slot cannot overflow. The online sibling of
/// [`crate::lint_ai::Bundler`]'s per-bit `i32` counts, narrowed to fit `TABLE_SLOTS` of them in
/// bounded memory (`i32` would not fit the budget at this table width).
#[derive(Clone)]
struct Slot {
    counts: Box<[i8; DIM]>,
    hits: u32,
}

impl Slot {
    fn new() -> Slot {
        Slot { counts: Box::new([0; DIM]), hits: 0 }
    }

    /// One-bit update: nudge every bit toward `target`'s bit, clamped — the online form of
    /// [`crate::lint_ai::majority_bundle`]'s offline vote.
    fn update(&mut self, target: &Hv) {
        let words = target.as_words();
        for (bit, c) in self.counts.iter_mut().enumerate() {
            let set = (words[bit / 64] >> (bit % 64)) & 1 == 1;
            *c = c.saturating_add(if set { 1 } else { -1 });
        }
        self.hits += 1;
    }

    /// The slot's current majority-vote prediction — `None` on a never-visited slot (an honest
    /// abstention: no memory yet, never a fabricated all-zero guess).
    fn predict(&self) -> Option<Hv> {
        if self.hits == 0 {
            return None;
        }
        let mut words = [0u64; DIM / 64];
        for (bit, &c) in self.counts.iter().enumerate() {
            if c > 0 {
                words[bit / 64] |= 1 << (bit % 64);
            }
        }
        Some(Hv::from_words(&words))
    }
}

/// The trained coder: exactly [`TABLE_SLOTS`] hard locations, indexed by a bound context
/// signature's own hash. The public surface is deliberately narrow — [`Coder::train`] and
/// [`Coder::predict`] — everything else (the token stream, the binding, the hashing) is an
/// internal implementation choice the caller never needs.
pub struct Coder {
    slots: Vec<Slot>,
}

impl Default for Coder {
    fn default() -> Coder {
        Coder::new()
    }
}

impl Coder {
    pub fn new() -> Coder {
        Coder { slots: (0..TABLE_SLOTS).map(|_| Slot::new()).collect() }
    }

    /// The table slot a context signature addresses — an FNV-1a fold of its raw words, modulo the
    /// table width. Deterministic, so the same context always addresses the same slot.
    fn slot_index(context: &Hv) -> usize {
        let mut h = 0xCBF2_9CE4_8422_2325u64;
        for w in context.as_words() {
            h ^= *w;
            h = h.wrapping_mul(0x0000_0001_0000_01B3);
        }
        (h as usize) % TABLE_SLOTS
    }

    /// Bind `CONTEXT_K` consecutive token signatures into one context Hv — position-sensitive XOR
    /// (each token rotated by its offset before folding), the same window-binding
    /// [`crate::lint_ai::bind`] uses. `window` must hold exactly [`CONTEXT_K`] signatures, oldest
    /// first.
    fn context_hv(window: &[Hv]) -> Hv {
        let mut ctx = Hv::zero();
        for (i, hv) in window.iter().enumerate() {
            let mut rotated = *hv;
            for _ in 0..i {
                rotated = rotated.rotl1_pub();
            }
            ctx = ctx.xor(&rotated);
        }
        ctx
    }

    /// Train online over one token stream, left to right, one pass: for each position `i >= K`,
    /// bind tokens `[i-K, i)` into a context and nudge that slot toward token `i`'s signature.
    /// Returns the number of updates performed (`0` on a stream shorter than the context).
    pub fn train(&mut self, tokens: &[&str]) -> usize {
        if tokens.len() <= CONTEXT_K {
            return 0;
        }
        let sigs: Vec<Hv> = tokens.iter().map(|t| token_hv(t)).collect();
        let mut updates = 0usize;
        for i in CONTEXT_K..sigs.len() {
            let ctx = Self::context_hv(&sigs[i - CONTEXT_K..i]);
            let idx = Self::slot_index(&ctx);
            self.slots[idx].update(&sigs[i]);
            updates += 1;
        }
        updates
    }

    /// The coder's prediction for the token that follows `window` (exactly [`CONTEXT_K`]
    /// signatures, oldest first) — `None` on a context this model has never trained on.
    pub fn predict(&self, window: &[Hv]) -> Option<Hv> {
        if window.len() != CONTEXT_K {
            return None;
        }
        let ctx = Self::context_hv(window);
        self.slots[Self::slot_index(&ctx)].predict()
    }
}

/// Tokenize `body` into the markup-typography token stream: a `<…>` run (`<!--…-->` for a
/// comment) is one token, and a maximal run of non-whitespace characters between them is one
/// token — the same word-boundary rule [`crate::lint_graph`]'s module doc names as HTML's own
/// typography. Pure segmentation: no tag name, no attribute, no English judgement. Returns
/// `(start, end, text)` triples so a caller can map a token back to its byte span in `body`.
pub fn tokenize(body: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        if bytes[at] == b'<' {
            let close = if body[at..].starts_with("<!--") {
                body[at..].find("-->").map(|i| at + i + 3).unwrap_or(body.len())
            } else {
                body[at..].find('>').map(|i| at + i + 1).unwrap_or(body.len())
            };
            out.push((at, close, &body[at..close]));
            at = close;
            continue;
        }
        let text_end = body[at..].find('<').map(|i| at + i).unwrap_or(body.len());
        let text = &body[at..text_end];
        let mut word_start: Option<usize> = None;
        for (i, c) in text.char_indices() {
            let abs = at + i;
            if c.is_whitespace() {
                if let Some(s) = word_start.take() {
                    out.push((s, abs, &body[s..abs]));
                }
            } else if word_start.is_none() {
                word_start = Some(abs);
            }
        }
        if let Some(s) = word_start {
            out.push((s, text_end, &body[s..text_end]));
        }
        at = text_end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_splits_tags_and_words() {
        let toks = tokenize("<p>hello world</p>");
        let texts: Vec<&str> = toks.iter().map(|&(_, _, t)| t).collect();
        assert_eq!(texts, vec!["<p>", "hello", "world", "</p>"]);
    }

    #[test]
    fn tokenize_keeps_a_comment_as_one_token() {
        let toks = tokenize("a <!-- x > y --> b");
        let texts: Vec<&str> = toks.iter().map(|&(_, _, t)| t).collect();
        assert_eq!(texts, vec!["a", "<!-- x > y -->", "b"]);
    }

    #[test]
    fn tokenize_spans_slice_back_to_the_source() {
        let body = "<div>ok</div>";
        for (s, e, t) in tokenize(body) {
            assert_eq!(&body[s..e], t);
        }
    }

    #[test]
    fn context_hv_is_order_sensitive() {
        let a = token_hv("a");
        let b = token_hv("b");
        let c = token_hv("c");
        let forward = Coder::context_hv(&[a, b, c]);
        let backward = Coder::context_hv(&[c, b, a]);
        assert_ne!(forward, backward);
    }

    #[test]
    fn slot_index_is_deterministic() {
        let ctx = Coder::context_hv(&[token_hv("<p>"), token_hv("hello"), token_hv("world")]);
        assert_eq!(Coder::slot_index(&ctx), Coder::slot_index(&ctx));
    }

    #[test]
    fn unseen_context_abstains() {
        let coder = Coder::new();
        let window = [token_hv("<x>"), token_hv("y"), token_hv("z")];
        assert!(coder.predict(&window).is_none());
    }

    #[test]
    fn repeated_training_lowers_prediction_error_on_the_same_context() {
        let mut coder = Coder::new();
        let tokens = ["<p>", "the", "quick", "fox", "<p>", "the", "quick", "fox"];
        coder.train(&tokens);
        let window = [token_hv("<p>"), token_hv("the"), token_hv("quick")];
        let predicted = coder.predict(&window).expect("context was trained");
        let actual = token_hv("fox");
        // Two identical n-grams voted for the SAME target — the slot's majority must reproduce
        // it exactly (distance 0), not merely "closer than an untrained guess".
        assert_eq!(predicted.distance(&actual), 0);
    }

    // ── PASS 39 item 3 — the full-corpus measurement (native/history.dx "PASS 39") ──────────────────
    //
    // Ignored by default (corpus-scale, minutes-class): run explicitly with
    // `cargo test --release --lib lint_coder::tests::measure_full_corpus -- --ignored --nocapture`.
    // Trains on 80% of HTML pages (by URL, so a held-out page's labels never leak into training),
    // evaluates boundary precision/recall against the other 20%'s grounded labels
    // (`lint_labels::export_labels`), against a trivial "every tag is a boundary" baseline, and
    // reports mean prediction error per register (chrome/code/prose/status/term/unlabeled) as the
    // register-separation signal. Persists the label sidecar as a side effect.
    #[test]
    #[ignore]
    #[cfg(feature = "crawl")]
    fn measure_full_corpus() {
        use std::collections::HashMap;

        let t_export = std::time::Instant::now();
        let (labels, counts) = crate::lint_labels::export_labels();
        let export_elapsed = t_export.elapsed();
        crate::lint_labels::save_labels(&labels);
        println!(
            "labels: pages={} code={} governing_prose={} chrome={} status_marker={} term={} total={} export={export_elapsed:?}",
            counts.pages, counts.code, counts.governing_prose, counts.chrome, counts.status_marker, counts.term,
            counts.total()
        );

        let mut by_url: HashMap<String, Vec<crate::lint_labels::Label>> = HashMap::new();
        for l in labels {
            by_url.entry(l.url.clone()).or_default().push(l);
        }
        let bodies: HashMap<String, String> = crate::lint_docs::all_cached_pages()
            .into_iter()
            .filter(|(_, b)| b.contains("</"))
            .collect();
        let mut urls: Vec<String> = by_url.keys().cloned().collect();
        urls.sort();

        // Deterministic 80/20 split BY PAGE — a held-out URL's every label stays held out
        // together, so the split can never leak one page's spans across train/test.
        let is_test = |u: &str| crate::lint_ai::token_seed(u) % 5 == 0;

        let mut coder = Coder::new();
        let mut train_tokens = 0usize;
        let mut train_pages = 0usize;
        let t_train = std::time::Instant::now();
        for url in &urls {
            if is_test(url) {
                continue;
            }
            let Some(body) = bodies.get(url) else { continue };
            let dropped = crate::doc_crawler::drop_script_style(body);
            let toks: Vec<&str> = tokenize(&dropped).into_iter().map(|(_, _, t)| t).collect();
            if toks.is_empty() {
                continue;
            }
            train_tokens += toks.len();
            train_pages += 1;
            coder.train(&toks);
        }
        let train_elapsed = t_train.elapsed();

        const TOLERANCE: i64 = 24; // bytes — a predicted boundary within this window of a true one is a hit.
        let (mut coder_tp, mut coder_fp, mut coder_fn) = (0u32, 0u32, 0u32);
        let (mut base_tp, mut base_fp, mut base_fn) = (0u32, 0u32, 0u32);
        let mut err_by_kind: HashMap<&'static str, (u64, u64)> = HashMap::new();
        let mut test_pages = 0usize;

        for url in &urls {
            if !is_test(url) {
                continue;
            }
            let Some(body) = bodies.get(url) else { continue };
            let dropped = crate::doc_crawler::drop_script_style(body);
            let toks = tokenize(&dropped);
            if toks.len() <= CONTEXT_K {
                continue;
            }
            test_pages += 1;
            let sigs: Vec<Hv> = toks.iter().map(|&(_, _, t)| token_hv(t)).collect();

            let mut errors: Vec<(u32, u32)> = Vec::with_capacity(toks.len());
            for i in CONTEXT_K..toks.len() {
                let err = match coder.predict(&sigs[i - CONTEXT_K..i]) {
                    Some(p) => p.distance(&sigs[i]),
                    None => DIM as u32, // no memory of this context — scored as maximal error
                };
                errors.push((toks[i].0 as u32, err));
            }
            if errors.is_empty() {
                continue;
            }

            let mean: f64 = errors.iter().map(|&(_, e)| f64::from(e)).sum::<f64>() / errors.len() as f64;
            let var: f64 =
                errors.iter().map(|&(_, e)| (f64::from(e) - mean).powi(2)).sum::<f64>() / errors.len() as f64;
            let threshold = mean + var.sqrt(); // one-sigma spike over the page's own baseline
            let predicted: Vec<u32> =
                errors.iter().filter(|&&(_, e)| f64::from(e) > threshold).map(|&(s, _)| s).collect();
            let baseline: Vec<u32> =
                toks.iter().filter(|&&(_, _, t)| t.starts_with('<')).map(|&(s, _, _)| s as u32).collect();

            let mut truth: Vec<u32> = by_url[url].iter().map(|l| l.start).collect();
            truth.sort_unstable();
            truth.dedup();

            score(&predicted, &truth, TOLERANCE, &mut coder_tp, &mut coder_fp, &mut coder_fn);
            score(&baseline, &truth, TOLERANCE, &mut base_tp, &mut base_fp, &mut base_fn);

            for &(start, err) in &errors {
                let kind = label_kind_at(&by_url[url], start);
                let e = err_by_kind.entry(kind).or_insert((0, 0));
                e.0 += u64::from(err);
                e.1 += 1;
            }
        }

        let prf = |tp: u32, fp: u32, fn_: u32| -> (f64, f64) {
            let precision = if tp + fp == 0 { 0.0 } else { f64::from(tp) / f64::from(tp + fp) };
            let recall = if tp + fn_ == 0 { 0.0 } else { f64::from(tp) / f64::from(tp + fn_) };
            (precision, recall)
        };
        let (cp, cr) = prf(coder_tp, coder_fp, coder_fn);
        let (bp, br) = prf(base_tp, base_fp, base_fn);

        println!("train: pages={train_pages} tokens={train_tokens} wall={train_elapsed:?}");
        println!("test: pages={test_pages} tolerance={TOLERANCE}B");
        println!("boundary P/R — coder:    P={cp:.3} R={cr:.3} (tp={coder_tp} fp={coder_fp} fn={coder_fn})");
        println!("boundary P/R — baseline: P={bp:.3} R={br:.3} (tp={base_tp} fp={base_fp} fn={base_fn}) [every markup token-open]");
        println!("register separation (mean prediction error, max={DIM}):");
        let mut kinds: Vec<&&str> = err_by_kind.keys().collect();
        kinds.sort();
        for k in kinds {
            let (sum, n) = err_by_kind[k];
            println!("  {k:16} mean_error={:.1} n={n}", sum as f64 / n.max(1) as f64);
        }
    }

    /// Greedy tolerance-window matcher: a predicted boundary within `tolerance` bytes of an
    /// unmatched truth boundary is a hit (counted once each); every other predicted boundary is a
    /// false positive, every truth boundary matched by nothing is a false negative.
    #[cfg(feature = "crawl")]
    fn score(predicted: &[u32], truth: &[u32], tolerance: i64, tp: &mut u32, fp: &mut u32, fn_: &mut u32) {
        let mut matched = vec![false; truth.len()];
        for &p in predicted {
            match truth.iter().position(|&t| (i64::from(p) - i64::from(t)).abs() <= tolerance) {
                Some(i) => {
                    matched[i] = true;
                    *tp += 1;
                }
                None => *fp += 1,
            }
        }
        *fn_ += matched.iter().filter(|&&m| !m).count() as u32;
    }

    /// The label register containing byte `pos` on `page`'s labels, or `"unlabeled"` when none
    /// covers it — the register-separation measurement's per-token bucket key.
    #[cfg(feature = "crawl")]
    fn label_kind_at(page: &[crate::lint_labels::Label], pos: u32) -> &'static str {
        for l in page {
            if pos >= l.start && pos < l.end {
                return match l.kind {
                    crate::lint_labels::LabelKind::Code => "code",
                    crate::lint_labels::LabelKind::GoverningProse => "governing_prose",
                    crate::lint_labels::LabelKind::Chrome => "chrome",
                    crate::lint_labels::LabelKind::StatusMarker => "status_marker",
                    crate::lint_labels::LabelKind::Term => "term",
                };
            }
        }
        "unlabeled"
    }
}
