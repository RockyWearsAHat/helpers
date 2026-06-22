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

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::lint_ai::{bind, tokenize, Bundler, Hv};

/// Sliding code-window width (tokens) the reasoner operates on.
const WIN: usize = 4;

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

/// A labeled violation signal: a code-window hypervector and the rule index it marks.
struct Sig {
    hv: Hv,
    rule: u32,
}

/// One expert — the distinctive violation signals of a single documentation slice, a
/// router signature (the bundle of those signals) used to gate windows to it, and its
/// OWN calibrated cap: the confident-match distance set just below the nearest clean
/// window, so this expert can never fire on known-good code in its category.
struct Expert {
    #[allow(dead_code)]
    name: String,
    sigs: Vec<Sig>,
    signature: Hv,
    cap: u32,
}

impl Expert {
    /// Nearest violation signal to `q`: its rule index and bit-distance.
    fn nearest(&self, q: &Hv) -> (u32, u32) {
        let mut best = u32::MAX;
        let mut bd = u32::MAX;
        for s in &self.sigs {
            let d = s.hv.distance(q);
            if d < bd {
                bd = d;
                best = s.rule;
            }
        }
        (best, bd)
    }
}

/// A code window's tokens.
type Window = Vec<String>;

/// Tokenize `code` into overlapping `WIN`-token windows (code-aware, so operators count).
fn windows_of(code: &str) -> Vec<Window> {
    let toks: Vec<String> = tokenize(code).into_iter().map(|t| t.text).collect();
    toks.windows(WIN).map(|w| w.to_vec()).collect()
}

/// Bind a window's tokens into its hypervector signal.
fn signal(w: &[String]) -> Hv {
    let refs: Vec<&str> = w.iter().map(String::as_str).collect();
    bind(&refs)
}

/// The mixture of experts: the reasoning model.
pub struct Moe {
    experts: Vec<Expert>,
    rule_names: Vec<String>,
    freq: HashMap<String, u32>,
    cap: u32,
    topk: usize,
}

impl Moe {
    /// Train from documented `examples` and known-good `clean` code. A bad-example window
    /// becomes a violation signal for its rule only if it is *distinctive* — far (by
    /// `filter` bits) from all clean code — so generic scaffolding never becomes a
    /// fingerprint; each rule keeps at least its single most distinctive window. `cap` is
    /// the confident-match distance; `topk` is how many experts each window is routed to.
    pub fn train(
        examples: &[Example],
        clean: &[&str],
        filter: u32,
        cap: u32,
        topk: usize,
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
            for w in windows_of(src) {
                clean_ref.push(signal(&w));
                count(&w, &mut freq);
            }
        }
        for e in examples {
            for w in windows_of(&e.good) {
                clean_ref.push(signal(&w));
                count(&w, &mut freq);
            }
        }
        let min_clean = |hv: &Hv, r: &[Hv]| r.iter().map(|c| c.distance(hv)).min().unwrap_or(u32::MAX);

        // Intern rule names → indices; group distinctive violation signals by slice.
        let mut rule_names: Vec<String> = Vec::new();
        let mut rule_idx: HashMap<String, u32> = HashMap::new();
        let mut slices: HashMap<String, Vec<Sig>> = HashMap::new();
        for e in examples {
            let ridx = *rule_idx.entry(e.rule.clone()).or_insert_with(|| {
                rule_names.push(e.rule.clone());
                (rule_names.len() - 1) as u32
            });
            // score windows by distinctiveness; keep those above `filter`, else the best.
            let scored: Vec<(Window, u32)> = windows_of(&e.bad)
                .into_iter()
                .map(|w| {
                    let d = min_clean(&signal(&w), &clean_ref);
                    (w, d)
                })
                .collect();
            if scored.is_empty() {
                continue;
            }
            let any = scored.iter().any(|(_, d)| *d > filter);
            let best = scored.iter().enumerate().max_by_key(|(_, (_, d))| *d).map(|(i, _)| i);
            let bucket = slices.entry(e.slice.clone()).or_default();
            for (i, (w, d)) in scored.iter().enumerate() {
                let keep = if any { *d > filter } else { Some(i) == best };
                if keep {
                    count(w, &mut freq);
                    bucket.push(Sig { hv: signal(w), rule: ridx });
                }
            }
        }

        // Build experts: router signature = bundle of signals; per-expert cap calibrated
        // just below the nearest clean window to any of the expert's signals (clamped to
        // the global `cap`), so the expert cannot fire on known-good code in its category.
        let experts: Vec<Expert> = slices
            .into_iter()
            .filter(|(_, sigs)| !sigs.is_empty())
            .map(|(name, sigs)| {
                let mut b = Bundler::new();
                for s in &sigs {
                    b.add(&s.hv);
                }
                let nearest_clean = clean_ref
                    .iter()
                    .map(|c| sigs.iter().map(|s| s.hv.distance(c)).min().unwrap_or(u32::MAX))
                    .min()
                    .unwrap_or(u32::MAX);
                let ecap = nearest_clean.saturating_sub(1).min(cap);
                Expert { name, signature: b.finalize(), sigs, cap: ecap }
            })
            .collect();

        Moe { experts, rule_names, freq, cap, topk }
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

        let mut best_rule = u32::MAX;
        let mut best_d = u32::MAX;
        let mut best_e = usize::MAX;
        for &(_, ei) in &routed {
            let e = &self.experts[ei];
            let (r, d) = e.nearest(&q);
            // accept only within THIS expert's calibrated cap — a window can't match an
            // expert whose real violations sit farther than its boundary.
            if d <= e.cap && d < best_d {
                best_d = d;
                best_rule = r;
                best_e = ei;
            }
        }
        if best_rule == u32::MAX {
            return None;
        }
        let ecap = self.experts[best_e].cap;
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
        let (r2, d2) = self.experts[best_e].nearest(&bind(&without));
        // The match must DEPEND on the distinctive token: removing it either changes the
        // rule or pushes the window out of this expert's confident range. If the same rule
        // still matches just as closely without the subject, it's a generic match → abstain.
        if r2 == best_rule && d2 <= ecap {
            return None;
        }
        Some(best_rule)
    }

    /// Judge a whole source: the distinct rule indices it is known to violate.
    pub fn judge(&self, code: &str) -> Vec<u32> {
        let mut hits = Vec::new();
        for w in windows_of(code) {
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
        let toks = tokenize(code);
        if toks.len() < WIN {
            return Vec::new();
        }
        let texts: Vec<String> = toks.iter().map(|t| t.text.clone()).collect();
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut out = Vec::new();
        for i in 0..=texts.len() - WIN {
            if let Some(r) = self.judge_window(&texts[i..i + WIN]) {
                if seen.insert(r) {
                    out.push((toks[i].line, r));
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
            experts: self
                .experts
                .iter()
                .map(|e| ExpertDto {
                    name: e.name.clone(),
                    cap: e.cap,
                    signature: e.signature.as_words().to_vec(),
                    sigs: e.sigs.iter().map(|s| (s.hv.as_words().to_vec(), s.rule)).collect(),
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
                    cap: e.cap,
                    signature: Hv::from_words(&e.signature),
                    sigs: e
                        .sigs
                        .into_iter()
                        .map(|(w, rule)| Sig { hv: Hv::from_words(&w), rule })
                        .collect(),
                })
                .collect(),
            rule_names: dto.rule_names,
            freq: dto.freq.into_iter().collect(),
            cap: dto.cap,
            topk: dto.topk,
        })
    }
}

/// On-disk form of an expert.
#[derive(Serialize, Deserialize)]
struct ExpertDto {
    name: String,
    cap: u32,
    signature: Vec<u64>,
    sigs: Vec<(Vec<u64>, u32)>,
}

/// On-disk form of the whole model.
#[derive(Serialize, Deserialize)]
struct MoeDto {
    cap: u32,
    topk: usize,
    rule_names: Vec<String>,
    freq: Vec<(String, u32)>,
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
}
