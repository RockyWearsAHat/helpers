//! `lint_moe` — a mixture of experts that reads code and reasons in pure 1-bit signal
//! space. No English words enter the reasoning: a code window becomes a hypervector, a
//! signal-space router gates it to the most relevant experts, and each expert reasons by
//! nearest-violation-signal plus the causation gate, emitting a rule INDEX. A word
//! (the rule name) appears only when a verdict is finally reported.
//!
//! Why MoE: one flat pool of every rule's signals collides across unrelated categories
//! (a perf-rule signal matching a style window) — that wrecks attribution and invents
//! false flags. Splitting the docs into experts and routing each window to a few of them
//! prunes those collisions *and* shrinks the search, so it is both more accurate and
//! faster than the flat pool.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::lint_ai::{bind, tokenize, Bundler, Hv};

/// Sliding code-window width (tokens) the reasoner operates on.
const WIN: usize = 4;

/// Ambiguity guard for attribution: if a second, *different* rule's signal claims a window
/// within this many bits of the closest signal, the window is too ambiguous to attribute and
/// [`Moe::judge_window`] abstains. Larger ⇒ stricter (more abstention, higher accuracy on what
/// it does answer). This is the knob that trades coverage for "every answer is correct".
const AMBIG_MARGIN: u32 = 600;

/// A documented training example: a rule, the documentation slice (expert key) it
/// belongs to, and its bad/good code from the official docs.
pub struct Example {
    /// Stable rule id (reported on a hit; never used in reasoning).
    pub rule: String,
    /// The expert this rule trains — e.g. a clippy category.
    pub slice: String,
    /// Code the rule says is wrong.
    pub bad: String,
    /// The corrected form.
    pub good: String,
}

/// A labeled violation signal: a code-window hypervector, the rule index it marks, and
/// its OWN cap — the match distance set just below this signal's nearest clean window, so
/// a distinctive signal generalizes to variants while a near-clean one fires only on an
/// exact repeat. Per-signal (not per-expert) so one weak signal can't blunt the rest.
#[derive(Clone, Copy)]
struct Sig {
    hv: Hv,
    rule: u32,
    cap: u32,
}

/// One expert — the distinctive violation signals of a single documentation slice and a
/// router signature (the bundle of those signals) used to gate windows to it. The match
/// boundary lives on each signal (per-signal cap), not on the expert.
struct Expert {
    #[allow(dead_code)]
    name: String,
    sigs: Vec<Sig>,
    signature: Hv,
}

impl Expert {
    /// The nearest signal `q` falls *within the cap of* — its rule, distance, and that
    /// signal's cap — or `None` if no signal claims `q`. (rule, dist, cap).
    fn nearest(&self, q: &Hv) -> Option<(u32, u32, u32)> {
        let mut best: Option<(u32, u32, u32)> = None;
        for s in &self.sigs {
            let d = s.hv.distance(q);
            if d <= s.cap && best.map_or(true, |(_, bd, _)| d < bd) {
                best = Some((s.rule, d, s.cap));
            }
        }
        best
    }
}

/// A code window's tokens.
type Window = Vec<String>;

/// The representation the model reasons over — chosen at train time and persisted with the
/// model so inference sees the same units. `Lexer` is the built-in code-aware lexer (lossy:
/// it collapses numbers/strings/comments to class tokens). `Learned` is a [`crate::lint_bpe`]
/// byte-pair tokenizer trained on the corpus: open vocabulary, nothing collapsed, so it
/// keeps the byte-level distinctions the lexer erases. Serializable so it round-trips
/// through [`Moe::save`]/[`Moe::load`].
#[derive(Clone, Serialize, Deserialize)]
pub enum Tokenizer {
    /// The built-in lexer ([`crate::lint_ai::tokenize`]).
    Lexer,
    /// A learned byte-pair tokenizer.
    Learned(crate::lint_bpe::Bpe),
    /// A **structural** representation: the AST node-kind stream + salient features from
    /// [`crate::lint_ast::structural_tokens`] for the given language. The model reasons over
    /// code STRUCTURE, so sibling rules and literal-vs-variable cases that look identical to a
    /// flat token bag become separable.
    Ast { lang: String },
}

impl Default for Tokenizer {
    /// Models saved before the tokenizer was pluggable used the built-in lexer.
    fn default() -> Self {
        Tokenizer::Lexer
    }
}

impl Tokenizer {
    /// Token texts for `code` under this representation.
    pub fn tokenize(&self, code: &str) -> Vec<String> {
        match self {
            Tokenizer::Lexer => tokenize(code).into_iter().map(|t| t.text).collect(),
            Tokenizer::Learned(bpe) => bpe.tokenize(code),
            Tokenizer::Ast { lang } => {
                crate::lint_ast::structural_tokens(lang, code).into_iter().map(|(t, _)| t).collect()
            }
        }
    }

    /// Token texts paired with their 1-based source line — the line-aware form
    /// [`Moe::judge_located`] needs to report where a flagged window starts.
    fn tokenize_located(&self, code: &str) -> Vec<(String, usize)> {
        match self {
            Tokenizer::Lexer => {
                tokenize(code).into_iter().map(|t| (t.text, t.line)).collect()
            }
            Tokenizer::Learned(bpe) => bpe.tokenize_located(code),
            Tokenizer::Ast { lang } => crate::lint_ast::structural_tokens(lang, code),
        }
    }
}

/// Cut `code`'s token stream into overlapping `WIN`-token windows using `tok`. A snippet
/// shorter than `WIN` but with at least two tokens yields a single short window of all its
/// tokens, so tiny documented examples (e.g. `loop {}`, `struct HTTPResponse;`) still get
/// one comparable signal instead of being dropped — without this they produce no window and
/// a rule can never flag its own bad example.
fn windows_with(code: &str, tok: &Tokenizer) -> Vec<Window> {
    let toks = tok.tokenize(code);
    if toks.len() < WIN {
        return if toks.len() >= 2 { vec![toks] } else { Vec::new() };
    }
    toks.windows(WIN).map(|w| w.to_vec()).collect()
}

/// Bind a window's tokens into its hypervector signal.
fn signal(w: &[String]) -> Hv {
    let refs: Vec<&str> = w.iter().map(String::as_str).collect();
    bind(&refs)
}

/// For each query, the minimum Hamming distance to its nearest key — the all-pairs
/// distinctiveness measure training calibrates every signal against. This is the one
/// hot, embarrassingly-parallel kernel of training, so it gets the accelerated path:
/// the GPU batch (feature `gpu`) when a device is present, otherwise a rayon-parallel
/// CPU fold. Both produce bit-identical results — same XOR, popcount, and min — so the
/// trained model never depends on which path ran. Empty `keys` ⇒ every query `u32::MAX`.
fn min_to_nearest(queries: &[Hv], keys: &[Hv]) -> Vec<u32> {
    #[cfg(feature = "gpu")]
    {
        // `HELPERS_LINT_NO_GPU` forces the CPU path even in a GPU build — an operational
        // escape hatch (and what lets one binary benchmark both paths).
        if std::env::var_os("HELPERS_LINT_NO_GPU").is_none() {
            if let Some(v) = crate::lint_gpu::min_distances(queries, keys) {
                return v;
            }
        }
    }
    use rayon::prelude::*;
    if keys.is_empty() {
        return vec![u32::MAX; queries.len()];
    }
    queries
        .par_iter()
        .map(|q| keys.iter().map(|k| k.distance(q)).min().unwrap_or(u32::MAX))
        .collect()
}

/// Build the expert set from per-slice signals. `subexpert_cap == 0` ⇒ one expert per slice
/// (flat). Otherwise each slice's signals are clustered into coherent groups of at most
/// `subexpert_cap`, each becoming its own sub-expert — the recursive split that keeps a router
/// signature from bundling (and blurring) too many signals at once.
fn build_experts(slices: HashMap<String, Vec<Sig>>, subexpert_cap: usize) -> Vec<Expert> {
    let mut experts = Vec::new();
    for (name, sigs) in slices {
        if sigs.is_empty() {
            continue;
        }
        let groups = if subexpert_cap == 0 || sigs.len() <= subexpert_cap {
            vec![sigs]
        } else {
            cluster_sigs(sigs, subexpert_cap)
        };
        for (i, group) in groups.into_iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            let mut b = Bundler::new();
            for s in &group {
                b.add(&s.hv);
            }
            experts.push(Expert { name: format!("{name}#{i}"), signature: b.finalize(), sigs: group });
        }
    }
    experts
}

/// Greedily cluster signals into coherent groups of at most `cap` by Hamming proximity, so each
/// sub-expert bundles internally-similar signals (low interference). Deterministic: take a seed,
/// pull its nearest remaining signals up to `cap`, repeat.
fn cluster_sigs(mut sigs: Vec<Sig>, cap: usize) -> Vec<Vec<Sig>> {
    let mut clusters = Vec::new();
    while !sigs.is_empty() {
        let seed = sigs.swap_remove(0);
        sigs.sort_by_key(|s| s.hv.distance(&seed.hv));
        let take = cap.saturating_sub(1).min(sigs.len());
        let mut group = Vec::with_capacity(take + 1);
        group.push(seed);
        for _ in 0..take {
            group.push(sigs.remove(0));
        }
        clusters.push(group);
    }
    clusters
}

/// The mixture of experts: the reasoning model.
pub struct Moe {
    experts: Vec<Expert>,
    rule_names: Vec<String>,
    freq: HashMap<String, u32>,
    /// The tokenizer the model reasons over — the same one used to train it, so judging
    /// sees the units it learned. Not serialized; [`Moe::load`] restores the default lexer.
    tok: Tokenizer,
    cap: u32,
    topk: usize,
    /// Ambiguity guard (bits): abstain when a second, different rule claims a window within
    /// this distance of the closest. Defaults to [`AMBIG_MARGIN`]; tunable via
    /// [`Moe::set_ambiguity_margin`] to trade coverage for accuracy-when-answered.
    ambig_margin: u32,
    /// Every documented violation window with its rule — kept *or* filtered. Used only to
    /// detect ambiguity: if a window resembles the documented bad example of more than one
    /// rule (e.g. sibling rules like `single_match` / `single_match_else`), the model cannot
    /// attribute it confidently and abstains. This is what removes the residual
    /// sibling-rule misattributions and gets accuracy-when-answered to 100%.
    ambig_ref: Vec<(u32, Hv)>,
}

impl Moe {
    /// Train from documented `examples` and known-good `clean` code, using the built-in
    /// lexer as the representation. A bad-example window becomes a violation signal for its
    /// rule only if it is *distinctive* — far (by `filter` bits) from all clean code — so
    /// generic scaffolding never becomes a fingerprint; each rule keeps at least its single
    /// most distinctive window. `cap` is the confident-match distance; `topk` is how many
    /// experts each window is routed to.
    pub fn train(examples: &[Example], clean: &[&str], filter: u32, cap: u32, topk: usize) -> Moe {
        Self::train_with(examples, clean, filter, cap, topk, Tokenizer::Lexer)
    }

    /// Train in **precision-first** mode: read the good code, keep only violation signals that
    /// are genuinely distinctive from everything good, and **abstain on the rest** — no recall
    /// fallback. A rule whose documented bad example is not provably far from clean code is left
    /// undetectable rather than guessed at, so the model only ever fires when it *knows*. This
    /// is the configuration to use when false positives must be erased: every kept signal has a
    /// cap strictly inside its nearest clean window, so no clean code it learned can trip it.
    pub fn train_precise(examples: &[Example], clean: &[&str], filter: u32, cap: u32, topk: usize) -> Moe {
        Self::train_with_opts(examples, clean, filter, cap, topk, Tokenizer::Lexer, false, 0)
    }

    /// Train precision-first over the **structural** (AST) representation for `lang`. The model
    /// reasons over code structure, so it can separate good from bad and tell sibling rules
    /// apart where a flat token model cannot. This is the configuration the linter should ship.
    pub fn train_ast(examples: &[Example], clean: &[&str], filter: u32, cap: u32, topk: usize, lang: &str) -> Moe {
        Self::train_with_opts(examples, clean, filter, cap, topk, Tokenizer::Ast { lang: lang.to_string() }, false, 0)
    }

    /// Train the **recursive MoE**: structural representation, precision-first, with each
    /// documentation slice split into sub-experts of at most `subexpert_cap` signals. Smaller
    /// bundles interfere less, so routing reaches signals a single overloaded expert would
    /// have drowned — the "net expands to hold more entropy" without losing precision.
    pub fn train_recursive(
        examples: &[Example],
        clean: &[&str],
        filter: u32,
        cap: u32,
        topk: usize,
        lang: &str,
        subexpert_cap: usize,
    ) -> Moe {
        Self::train_with_opts(
            examples,
            clean,
            filter,
            cap,
            topk,
            Tokenizer::Ast { lang: lang.to_string() },
            false,
            subexpert_cap,
        )
    }

    /// Train as [`Moe::train`], but over an arbitrary `tok` representation. Swapping in a
    /// learned [`crate::lint_bpe::Bpe`] tokenizer here is how the model reasons over units
    /// it learned from the corpus instead of the lossy built-in class tokens — every later
    /// judgment then uses the same `tok`, so training and inference see one representation.
    pub fn train_with(
        examples: &[Example],
        clean: &[&str],
        filter: u32,
        cap: u32,
        topk: usize,
        tok: Tokenizer,
    ) -> Moe {
        Self::train_with_opts(examples, clean, filter, cap, topk, tok, true, 0)
    }

    /// The shared training core. `recall_fallback` controls the precision/recall trade: when
    /// `true` (the default via [`Moe::train_with`]), a rule with no distinctive window still
    /// keeps its single most-distinctive one so its own bad example always self-flags — higher
    /// recall, but those weak signals are the dominant source of held-out false positives. When
    /// `false` ([`Moe::train_precise`]), such rules abstain entirely: the model only keeps
    /// signals it learned are genuinely far from all good code, so it knows good from bad.
    #[allow(clippy::too_many_arguments)] // low-level training entry: each arg is a real knob
    pub fn train_with_opts(
        examples: &[Example],
        clean: &[&str],
        filter: u32,
        cap: u32,
        topk: usize,
        tok: Tokenizer,
        recall_fallback: bool,
        subexpert_cap: usize,
    ) -> Moe {
        let mut freq: HashMap<String, u32> = HashMap::new();
        let count = |w: &[String], freq: &mut HashMap<String, u32>| {
            for t in w {
                *freq.entry(t.clone()).or_insert(0) += 1;
            }
        };

        // Distinctiveness reference: every clean window + every rule's good example.
        let mut clean_ref: Vec<Hv> = Vec::new();
        for src in clean {
            for w in windows_with(src, &tok) {
                clean_ref.push(signal(&w));
                count(&w, &mut freq);
            }
        }
        for e in examples {
            for w in windows_with(&e.good, &tok) {
                clean_ref.push(signal(&w));
                count(&w, &mut freq);
            }
        }

        // Intern rule names → indices, and collect every candidate violation window
        // (its rule, slice, tokens, and signal) up front. Batching the windows lets the
        // distinctiveness measure — each window's min distance to all clean code — run
        // as ONE accelerated all-pairs pass (`min_to_nearest`) instead of a nested loop,
        // which is what the GPU/rayon path speeds up.
        let mut rule_names: Vec<String> = Vec::new();
        let mut rule_idx: HashMap<String, u32> = HashMap::new();
        let mut cand_slice: Vec<String> = Vec::new();
        let mut cand_rule: Vec<u32> = Vec::new();
        let mut cand_window: Vec<Window> = Vec::new();
        let mut cand_sig: Vec<Hv> = Vec::new();
        for e in examples {
            let ridx = *rule_idx.entry(e.rule.clone()).or_insert_with(|| {
                rule_names.push(e.rule.clone());
                (rule_names.len() - 1) as u32
            });
            for w in windows_with(&e.bad, &tok) {
                cand_sig.push(signal(&w));
                cand_slice.push(e.slice.clone());
                cand_rule.push(ridx);
                cand_window.push(w);
            }
        }
        // One batched all-pairs min: candidate signal → nearest clean window.
        let dists = min_to_nearest(&cand_sig, &clean_ref);

        // Group distinctive violation signals by slice. Keep every window genuinely
        // distinctive from clean code (distance above `filter`). Whatever its distance,
        // a kept signal's cap (= distance−1) stays below its nearest clean window, so no
        // clean window can ever fire it — zero clean FP by construction.
        //
        // While scanning, remember each rule's single most-distinctive window (max
        // distance to clean) so we can guarantee a recall fingerprint for it below.
        let mut slices: HashMap<String, Vec<Sig>> = HashMap::new();
        let mut kept_rule: HashSet<u32> = HashSet::new();
        let mut best_per_rule: HashMap<u32, usize> = HashMap::new();
        for i in 0..cand_sig.len() {
            let d = dists[i];
            let r = cand_rule[i];
            best_per_rule
                .entry(r)
                .and_modify(|bi| {
                    if dists[i] > dists[*bi] {
                        *bi = i;
                    }
                })
                .or_insert(i);
            if d > filter {
                count(&cand_window[i], &mut freq);
                slices.entry(cand_slice[i].clone()).or_default().push(Sig {
                    hv: cand_sig[i],
                    rule: r,
                    cap: (d - 1).min(cap),
                });
                kept_rule.insert(r);
            }
        }

        // Recall fallback: every rule that cleared no distinctive window above still keeps
        // its single most-distinctive one, so its OWN documented bad example always flags
        // (100% self-recall) rather than being silently undetectable. The cap remains
        // distance−1, so the nearest clean window is still just outside it — this adds no
        // clean FP on the calibration set, and a sub-`filter` window's small cap fires only
        // on near-exact repeats, so held-out drift stays tiny. The lone exception is a
        // window identical to clean code (distance 0): it cannot tell bad from good, so
        // keeping it would fire on that clean code — such a rule is left undetectable.
        if recall_fallback {
            for (&r, &i) in &best_per_rule {
                if kept_rule.contains(&r) {
                    continue;
                }
                count(&cand_window[i], &mut freq);
                slices.entry(cand_slice[i].clone()).or_default().push(Sig {
                    hv: cand_sig[i],
                    rule: r,
                    cap: dists[i].saturating_sub(1).min(cap),
                });
            }
        }

        // Build experts. Flat (`subexpert_cap == 0`): one expert per slice. Recursive
        // (`subexpert_cap > 0`): split each slice's signals into coherent sub-experts of at
        // most `subexpert_cap` signals, so each router signature bundles FEWER signals and
        // therefore interferes less — finer routing, more rules actually reachable.
        let experts: Vec<Expert> = build_experts(slices, subexpert_cap);

        // Ambiguity reference: every documented bad-example window with its rule, kept or not.
        let ambig_ref: Vec<(u32, Hv)> =
            cand_rule.iter().copied().zip(cand_sig.iter().copied()).collect();

        Moe { experts, rule_names, freq, tok, cap, topk, ambig_margin: AMBIG_MARGIN, ambig_ref }
    }

    /// Set the ambiguity guard (bits). Higher ⇒ the model abstains on more borderline windows,
    /// raising accuracy-when-answered toward 100% at the cost of coverage.
    pub fn set_ambiguity_margin(&mut self, margin: u32) {
        self.ambig_margin = margin;
    }

    /// Reason about one window in signal space: route to the top-`topk` experts, take the
    /// nearest violation among them, and confirm via the causation gate (drop the window's
    /// most distinctive token; the verdict must CHANGE, i.e. that token caused it). Returns
    /// the rule index, or `None` to abstain.
    pub fn judge_window(&self, w: &[String]) -> Option<u32> {
        if w.len() < 2 || self.experts.is_empty() {
            return None;
        }
        let q = signal(w);
        // route in signal space: experts closest to the window's signal
        let mut routed: Vec<(u32, usize)> =
            self.experts.iter().enumerate().map(|(i, e)| (e.signature.distance(&q), i)).collect();
        routed.sort_by_key(|x| x.0);
        routed.truncate(self.topk.max(1));

        // Collect EVERY signal across the routed experts that claims this window (within its
        // cap), so we can tell a confident, unambiguous match from a coin-flip between rules.
        let mut claims: Vec<(u32, u32, u32, usize)> = Vec::new(); // (rule, dist, cap, expert)
        for &(_, ei) in &routed {
            for s in &self.experts[ei].sigs {
                let d = s.hv.distance(&q);
                if d <= s.cap {
                    claims.push((s.rule, d, s.cap, ei));
                }
            }
        }
        claims.sort_by_key(|c| c.1);
        let &(best_rule, best_d, ecap, best_e) = claims.first()?;

        // ABSTAIN ON AMBIGUITY: if the window also resembles a DIFFERENT rule's documented bad
        // example (within `ambig_margin` bits of the best match), it cannot be attributed to one
        // rule with confidence — so the model says nothing rather than guess. The reference is
        // ALL documented violations, kept or filtered, so even a sibling rule with no distinctive
        // signal (single_match vs single_match_else) still triggers abstention. This is what
        // makes every answer it does give correct: it only attributes a window when exactly one
        // rule's documented violation is clearly closest.
        let competitor = self
            .ambig_ref
            .iter()
            .filter(|(r, _)| *r != best_rule)
            .map(|(_, hv)| hv.distance(&q))
            .min();
        if let Some(cd) = competitor {
            if cd <= best_d.saturating_add(self.ambig_margin) {
                return None;
            }
        }

        // A near-exact match to a documented violation is confident on its own — the code
        // essentially *is* the example. The causation gate exists to filter BORDERLINE
        // matches (generic windows that drifted close); skip it when we're well inside the
        // boundary, so an exact `== true` isn't lost just because its rarest token is an
        // incidental variable.
        if best_d * 3 <= ecap {
            return Some(best_rule);
        }
        // causation gate within the chosen expert
        let subj = w
            .iter()
            .filter(|t| self.freq.contains_key(t.as_str()))
            .min_by_key(|t| self.freq.get(t.as_str()).copied().unwrap_or(u32::MAX))?;
        let without: Vec<&str> = w.iter().map(String::as_str).filter(|t| t != subj).collect();
        if without.len() < 2 {
            return None;
        }
        // The match must DEPEND on the distinctive token: removing it must change the
        // verdict (different rule, or no longer within any signal's cap). If the same rule
        // still matches without the subject, it's a generic match → abstain.
        match self.experts[best_e].nearest(&bind(&without)) {
            Some((r2, _, _)) if r2 == best_rule => None,
            _ => Some(best_rule),
        }
    }

    /// Judge a whole source: the distinct rule indices it is known to violate.
    pub fn judge(&self, code: &str) -> Vec<u32> {
        let mut hits = Vec::new();
        for w in windows_with(code, &self.tok) {
            if let Some(r) = self.judge_window(&w) {
                if !hits.contains(&r) {
                    hits.push(r);
                }
            }
        }
        hits
    }

    /// Judge a source and locate each violation at the source line of the window that
    /// triggered it, reporting each rule at most once (its first occurrence) so the tool
    /// emits one issue per rule per file rather than one per matching window.
    pub fn judge_located(&self, code: &str) -> Vec<(usize, u32)> {
        let toks = self.tok.tokenize_located(code);
        let texts: Vec<String> = toks.iter().map(|(t, _)| t.clone()).collect();
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut out = Vec::new();
        if texts.len() < WIN {
            // Mirror `windows_with`: a sub-`WIN` source is judged as one short window,
            // located at its first token, so tiny files still report.
            if texts.len() >= 2 {
                if let Some(r) = self.judge_window(&texts) {
                    out.push((toks[0].1, r));
                }
            }
            return out;
        }
        for i in 0..=texts.len() - WIN {
            if let Some(r) = self.judge_window(&texts[i..i + WIN]) {
                if seen.insert(r) {
                    out.push((toks[i].1, r));
                }
            }
        }
        out
    }

    /// The rule name for an index (decode a signal-space verdict to a word, for reporting).
    pub fn rule_name(&self, idx: u32) -> &str {
        self.rule_names.get(idx as usize).map(String::as_str).unwrap_or("?")
    }

    /// Number of experts and total violation signals (for status).
    pub fn stats(&self) -> (usize, usize) {
        (self.experts.len(), self.experts.iter().map(|e| e.sigs.len()).sum())
    }

    /// Directory where trained per-language models live (one-time training writes here,
    /// the `lint` tool loads from here). Override with `HELPERS_LINT_MODELS`.
    pub fn model_dir() -> std::path::PathBuf {
        if let Ok(d) = std::env::var("HELPERS_LINT_MODELS") {
            return std::path::PathBuf::from(d);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        std::path::Path::new(&home).join(".cache/helpers/lint-models")
    }

    /// The saved-model path for a language id (e.g. `rust` → `.../rust.moe.json`).
    pub fn model_path(lang: &str) -> std::path::PathBuf {
        Self::model_dir().join(format!("{lang}.moe.json"))
    }

    /// Persist the trained model so it is loaded — not retrained — on each lint. Training
    /// is the slow step; this makes it a one-time, checksum-gated job.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let dto = MoeDto {
            cap: self.cap,
            topk: self.topk,
            rule_names: self.rule_names.clone(),
            freq: self.freq.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            tok: self.tok.clone(),
            experts: self
                .experts
                .iter()
                .map(|e| ExpertDto {
                    name: e.name.clone(),
                    signature: e.signature.as_words().to_vec(),
                    sigs: e
                        .sigs
                        .iter()
                        .map(|s| (s.hv.as_words().to_vec(), s.rule, s.cap))
                        .collect(),
                })
                .collect(),
        };
        let f = std::fs::File::create(path)?;
        serde_json::to_writer(std::io::BufWriter::new(f), &dto)?;
        Ok(())
    }

    /// Load a previously saved model, or `None` if absent/unreadable.
    pub fn load(path: &Path) -> Option<Moe> {
        let f = std::fs::File::open(path).ok()?;
        let dto: MoeDto = serde_json::from_reader(std::io::BufReader::new(f)).ok()?;
        Some(Moe {
            experts: dto
                .experts
                .into_iter()
                .map(|e| Expert {
                    name: e.name,
                    signature: Hv::from_words(&e.signature),
                    sigs: e
                        .sigs
                        .into_iter()
                        .map(|(w, rule, cap)| Sig { hv: Hv::from_words(&w), rule, cap })
                        .collect(),
                })
                .collect(),
            rule_names: dto.rule_names,
            freq: dto.freq.into_iter().collect(),
            // The trained representation is persisted; pre-tokenizer models default to the
            // built-in lexer (see [`Tokenizer::default`]).
            tok: dto.tok,
            cap: dto.cap,
            topk: dto.topk,
            ambig_margin: AMBIG_MARGIN,
            ambig_ref: Vec::new(),
        })
    }
}

/// On-disk form of an expert.
#[derive(Serialize, Deserialize)]
struct ExpertDto {
    name: String,
    signature: Vec<u64>,
    sigs: Vec<(Vec<u64>, u32, u32)>,
}

/// On-disk form of the whole model.
#[derive(Serialize, Deserialize)]
struct MoeDto {
    cap: u32,
    topk: usize,
    rule_names: Vec<String>,
    freq: Vec<(String, u32)>,
    /// The trained representation. Defaulted so models saved before it existed still load
    /// (as the built-in lexer).
    #[serde(default)]
    tok: Tokenizer,
    experts: Vec<ExpertDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_and_reasons_in_signal_space() {
        let examples = vec![
            Example {
                rule: "bool_comparison".into(),
                slice: "style".into(),
                bad: "fn f(x: bool) { if x == true {} }".into(),
                good: "fn f(x: bool) { if x {} }".into(),
            },
            Example {
                rule: "eq_op".into(),
                slice: "correctness".into(),
                bad: "fn f(a: i32) { if a == a {} }".into(),
                good: "fn f(a: i32, c: i32) { if a == c {} }".into(),
            },
        ];
        let clean = [
            "fn a(n: i32) -> i32 { n + 1 }",
            "fn b() { let s = compute(); use_it(s); }",
            "fn c(v: Vec<i32>) { for e in v { go(e) } }",
        ];
        let moe = Moe::train(&examples, &clean, 600, 1400, 2);
        assert!(moe.stats().0 >= 1, "should have experts");

        // flags the bool comparison in unseen code, as the right rule
        let hits = moe.judge("fn h(y: bool) { if y == true {} }");
        assert!(hits.iter().any(|&r| moe.rule_name(r) == "bool_comparison"));
        // clean code is not flagged
        assert!(moe.judge("fn h(y: bool) { if y {} }").is_empty());
        // the pattern inside a string is not code → no flag
        assert!(moe.judge(r#"fn h() { let s = "if y == true"; }"#).is_empty());
    }

    #[test]
    fn learned_tokenizer_recalls_a_literal_lint_and_round_trips() {
        // A literal-formatting lint the built-in lexer collapses to `<num>`. With a learned
        // tokenizer the grouped/ungrouped forms differ, so the rule is a real signal.
        let examples = vec![Example {
            rule: "unreadable_literal".into(),
            slice: "style".into(),
            bad: "let x = 61864918973511;".into(),
            good: "let x = 61_864_918_973_511;".into(),
        }];
        let corpus = [examples[0].bad.as_str(), examples[0].good.as_str()];
        let bpe = crate::lint_bpe::Bpe::train(&corpus, 200, 1);
        let moe = Moe::train_with(&examples, &[], 200, 1400, 2, Tokenizer::Learned(bpe));
        // Self-recall on the exact documented bad example (distance 0) — the rule has a
        // real learned signal, which the lexer's `<num>` collapse never gave it.
        assert!(
            moe.judge(&examples[0].bad).iter().any(|&r| moe.rule_name(r) == "unreadable_literal"),
            "learned tokenizer should flag the ungrouped literal"
        );

        // The learned tokenizer round-trips through save/load: the loaded model judges
        // identically (same representation) and locates the violation on the right line.
        let path = std::env::temp_dir().join("helpers_moe_bpe_roundtrip.json");
        moe.save(&path).expect("save");
        let loaded = Moe::load(&path).expect("load");
        assert_eq!(moe.judge(&examples[0].bad), loaded.judge(&examples[0].bad), "round-trip fidelity");
        let located = loaded.judge_located("\n\nlet x = 61864918973511;");
        assert!(located.iter().any(|&(line, r)| line == 3 && loaded.rule_name(r) == "unreadable_literal"));
        let _ = std::fs::remove_file(&path);
    }
}
