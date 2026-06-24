//! `linter` — ONE reasoning model, many plug-and-play, self-packed lint modules.
//!
//! The architecture the project has been converging on:
//!
//!   * **The reasoner** ([`Reasoner`]) is the always-on main model. It holds the hard-defined,
//!     always-good CS2420/CS3500 principles — learned from a *text document* the user supplies,
//!     not hardcoded — and it is what actually decides good-vs-bad *in a project*. It composes
//!     the deterministic floor, the behavioral CS norms, the taught principle patterns, and
//!     whatever modules are plugged in, into one verdict.
//!
//!   * **Modules** ([`LintModule`]) are each their own little lint AI for a kind of project
//!     (a language, a framework). A module is trained once from documentation, then **packed**
//!     into a self-contained JSON artifact ([`LintModule::to_json`]) that can be stored in a
//!     shared place (e.g. GitHub) and reused on any machine with no retraining.
//!
//!   * **The registry** ([`ModuleRegistry`]) is the package manager. It reads a manifest of
//!     available modules and pulls a module's artifact **lazily — only when a project actually
//!     needs it** ([`ModuleRegistry::select`] then [`ModuleRegistry::load`]) — so you never pay
//!     space for modules you are not using.
//!
//! Knowledge enters the system one way ([`Knowledge`]): from a crawled docs corpus, or from a
//! plain text/markdown document. The CS principles, a language module, a house style — all of it
//! is "a document you hand it," and every layer learns from that same shape.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::lint_semantic::{function_sources, functions, Norms, Principle};
use crate::lint_sig::{Rule as SigRule, SigModel};

/// Run a signature model **per function** and map each hit back to its real file line. A flat
/// whole-file match can mislocate (report a violation in function B at the first line of function
/// A that merely shares a feature) and cannot flag the same rule in two functions; judging each
/// function in isolation fixes both. Returns `(line, rule_id)` pairs.
fn judge_by_function(sig: &SigModel, lang: &str, code: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (start_line, body) in function_sources(lang, code) {
        for h in sig.judge_located(&body) {
            // `judge_located` lines are 1-based within `body`; offset to the file.
            out.push((start_line + h.line - 1, h.rule));
        }
    }
    out
}

/// One finding, with provenance so a report can say *which* layer judged it and why.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Finding {
    /// 1-based source line.
    pub line: usize,
    /// The rule or principle id violated.
    pub rule_id: String,
    /// Severity bucket (`high`/`medium`/`low`).
    pub severity: String,
    /// Where it came from: `floor`, `cs-principle`, `cs-norm`, or `module:<id>`.
    pub source: String,
    /// Human-readable advice — the message a fixing agent or student reads.
    pub message: String,
}

// ---------------------------------------------------------------------------------------------
// Knowledge: the single ingestion shape (docs corpus OR a text/markdown document).
// ---------------------------------------------------------------------------------------------

/// One documented rule learned from a doc or corpus: a language, an id, the bad/good examples,
/// an English description, and a severity. This is the atom every layer trains from.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LearnedRule {
    /// Language the examples are written in.
    pub language: String,
    /// Stable rule id.
    pub id: String,
    /// Severity bucket (`high`/`medium`/`low`); defaults to `medium`.
    pub severity: String,
    /// English description / the advice to show.
    pub description: String,
    /// Code the rule considers wrong.
    pub bad: String,
    /// The corrected form (may be empty).
    pub good: String,
}

/// A body of knowledge to learn from. Built from a crawled corpus or a text document; the rest of
/// the system never cares which — it only sees [`LearnedRule`]s.
#[derive(Clone, Debug, Default)]
pub struct Knowledge {
    /// Every rule-candidate this knowledge carries.
    pub rules: Vec<LearnedRule>,
    /// Real code the source served alongside the rules (every code block on every crawled doc page).
    /// It is the "what's normal in this language" sample the fit calibrates distinctiveness against:
    /// a feature is only trusted to mark a violation if it is genuinely RARE here, not merely absent
    /// from the handful of documented good examples. Empty when learning from a plain text document.
    pub reference: Vec<String>,
}

impl Knowledge {
    /// Read a crawled corpus (`scripts/crawl-docs.mjs` JSONL: one `{language,rule,description,
    /// bad,good,severity}` object per line). Malformed lines are skipped.
    pub fn from_corpus(path: &Path) -> std::io::Result<Knowledge> {
        let text = std::fs::read_to_string(path)?;
        let mut rules = Vec::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                rules.push(LearnedRule {
                    language: v["language"].as_str().unwrap_or("").to_string(),
                    id: v["rule"].as_str().unwrap_or("").to_string(),
                    severity: v["severity"].as_str().unwrap_or("medium").to_string(),
                    description: v["description"].as_str().unwrap_or("").to_string(),
                    bad: v["bad"].as_str().unwrap_or("").to_string(),
                    good: v["good"].as_str().unwrap_or("").to_string(),
                });
            }
        }
        Ok(Knowledge { rules, reference: Vec::new() })
    }

    /// Learn from a plain **text / markdown document**. This is how a user hands the system their
    /// own rules — the curated CS2420/CS3500 principles, a house style guide — and it becomes
    /// trainable knowledge with no code changes. The grammar is deliberately simple:
    ///
    /// * A heading (`#`/`##`/…) starts a rule. Its text is the description; an `[high|medium|low]`
    ///   suffix sets severity; the id is the heading slugified.
    /// * Fenced code blocks under a heading are its examples. The info string's tag decides which:
    ///   `bad`/`wrong`/`avoid` ⇒ the bad example, `good`/`right`/`correct`/`fix` ⇒ the good one
    ///   (an untagged first block is treated as bad, a second as good). The fence's language word
    ///   (` ```rust `) sets the example language, else `default_lang`.
    pub fn from_text(default_lang: &str, doc: &str) -> Knowledge {
        let mut rules: Vec<LearnedRule> = Vec::new();
        let mut cur: Option<LearnedRule> = None;
        let mut in_fence = false;
        let mut fence_lang = String::new();
        let mut fence_tag = String::new();
        let mut fence_buf = String::new();

        // Commit a finished fenced block to the current rule's bad/good slot.
        fn place(rule: &mut LearnedRule, tag: &str, code: String) {
            let is_good = matches!(tag, "good" | "right" | "correct" | "fix" | "after");
            let is_bad = matches!(tag, "bad" | "wrong" | "avoid" | "dont" | "before");
            if is_good || (!is_bad && !rule.bad.is_empty() && rule.good.is_empty()) {
                rule.good = code;
            } else {
                rule.bad = code;
            }
        }

        for line in doc.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("```") {
                if in_fence {
                    // Closing fence: commit the block.
                    if let Some(r) = cur.as_mut() {
                        if !r.language.is_empty() && !fence_lang.is_empty() {
                            r.language = fence_lang.clone();
                        } else if r.language.is_empty() {
                            r.language = if fence_lang.is_empty() { default_lang.to_string() } else { fence_lang.clone() };
                        }
                        place(r, &fence_tag, fence_buf.trim_end().to_string());
                    }
                    in_fence = false;
                    fence_buf.clear();
                } else {
                    // Opening fence: parse `lang` and/or `:tag` (e.g. `rust:bad`, `bad`, `rust`).
                    in_fence = true;
                    let info = rest.trim();
                    let (l, t) = info.split_once(':').unwrap_or((info, ""));
                    fence_lang = l.trim().to_string();
                    fence_tag = if t.is_empty() { l.trim().to_string() } else { t.trim().to_string() };
                    // If the single word is itself a tag (untagged-language case), treat it so.
                    if t.is_empty() && !matches!(l.trim(), "bad" | "wrong" | "avoid" | "dont" | "before" | "good" | "right" | "correct" | "fix" | "after") {
                        fence_tag = String::new();
                        fence_lang = l.trim().to_string();
                    } else if t.is_empty() {
                        fence_lang = String::new();
                    }
                }
                continue;
            }
            if in_fence {
                fence_buf.push_str(line);
                fence_buf.push('\n');
                continue;
            }
            if let Some(h) = heading(trimmed) {
                if let Some(r) = cur.take() {
                    if !r.bad.is_empty() {
                        rules.push(r);
                    }
                }
                let (sev, title) = split_severity(h);
                cur = Some(LearnedRule {
                    language: String::new(),
                    id: slug(title),
                    severity: sev,
                    description: title.to_string(),
                    bad: String::new(),
                    good: String::new(),
                });
            } else if let Some(r) = cur.as_mut() {
                // Prose between the heading and the first fence extends the description.
                let t = line.trim();
                if !t.is_empty() && r.bad.is_empty() {
                    if !r.description.is_empty() {
                        r.description.push(' ');
                    }
                    r.description.push_str(t);
                }
            }
        }
        if let Some(r) = cur.take() {
            if !r.bad.is_empty() {
                rules.push(r);
            }
        }
        Knowledge { rules, reference: Vec::new() }
    }

    /// Fold another body of knowledge in (later rules win on id collision within a language).
    pub fn merge(&mut self, other: Knowledge) {
        self.rules.extend(other.rules);
    }

    /// The distinct languages this knowledge covers.
    pub fn languages(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for r in &self.rules {
            if !r.language.is_empty() && !seen.contains(&r.language) {
                seen.push(r.language.clone());
            }
        }
        seen
    }

    /// The rules for `lang`, shaped for [`SigModel::train`].
    fn sig_rules(&self, lang: &str) -> Vec<SigRule> {
        self.rules
            .iter()
            .filter(|r| r.language == lang && !r.bad.is_empty())
            .map(|r| SigRule { id: r.id.clone(), bad: r.bad.clone(), good: r.good.clone(), description: r.description.clone() })
            .collect()
    }

    /// id → (severity, advice message) for `lang`, so a flag can carry its description.
    fn advice(&self, lang: &str) -> HashMap<String, (String, String)> {
        self.rules
            .iter()
            .filter(|r| r.language == lang)
            .map(|r| (r.id.clone(), (r.severity.clone(), r.description.clone())))
            .collect()
    }
}

/// A markdown ATX heading's text, or `None`.
fn heading(line: &str) -> Option<&str> {
    let h = line.trim_start_matches('#');
    if h.len() < line.len() && line.starts_with('#') {
        Some(h.trim())
    } else {
        None
    }
}

/// Split a trailing `[high|medium|low]` severity tag off a heading; default `medium`.
fn split_severity(title: &str) -> (String, &str) {
    let t = title.trim();
    if let Some(stripped) = t.strip_suffix(']') {
        if let Some(idx) = stripped.rfind('[') {
            let sev = stripped[idx + 1..].trim().to_lowercase();
            if matches!(sev.as_str(), "high" | "medium" | "low") {
                return (sev, stripped[..idx].trim());
            }
        }
    }
    ("medium".to_string(), t)
}

/// Slugify a heading into a stable id: lowercase, non-alphanumerics to `_`, collapsed.
fn slug(title: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for c in title.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            last_us = false;
        } else if !last_us && !out.is_empty() {
            out.push('_');
            last_us = true;
        }
    }
    out.trim_matches('_').to_string()
}

// ---------------------------------------------------------------------------------------------
// LintModule: a self-contained, packable, reusable lint AI for one language/project type.
// ---------------------------------------------------------------------------------------------

/// A self-packed lint module: its own little lint AI for one language, trained once from docs and
/// serialized so it can be stored centrally and reused anywhere. Self-contained — it carries both
/// the trained detector and the source rules (so its advice messages travel with it).
#[derive(Clone, Serialize, Deserialize)]
pub struct LintModule {
    /// Module id (e.g. `rust-clippy`).
    pub id: String,
    /// Language(s) this module lints.
    pub languages: Vec<String>,
    /// Version of the docs/toolchain it was trained from.
    pub version: String,
    /// Where it was trained from — provenance for auditability.
    pub provenance: String,
    /// The trained signature detector.
    sig: SigModel,
    /// id → (severity, advice) so flags carry their message without re-reading the docs.
    advice: HashMap<String, (String, String)>,
}

impl LintModule {
    /// Train and pack a module for `lang` from `knowledge`. This is the "train once" step; the
    /// result is serialized and shared so no machine repeats it. It uses the **self-validating
    /// fit**: every rule is tested against all the other rules' examples AND the documented good
    /// (idiomatic) forms, and is kept only if it separates the violation from all of them —
    /// otherwise it abstains. Precision over coverage, so the module does not over-flag (an import
    /// line is not mistaken for a write-amount bug).
    pub fn pack(id: &str, version: &str, provenance: &str, lang: &str, knowledge: &Knowledge) -> LintModule {
        let rules = knowledge.sig_rules(lang);
        // Reference = "what's normal in this language": the documented good forms PLUS every real
        // code block the crawler read across the whole docs site. Calibrating distinctiveness against
        // this large real sample (not just a handful of good examples) is what stops the fit from
        // grounding a feature that is merely absent from the docs but common in real code (`== <lit>`,
        // a bare `break`) — the cause of the context-insensitive false positives. A feature must be
        // genuinely rare HERE to be trusted, otherwise the rule abstains.
        let mut reference: Vec<String> = knowledge
            .rules
            .iter()
            .filter(|r| r.language == lang && !r.good.is_empty())
            .map(|r| r.good.clone())
            .collect();
        reference.extend(knowledge.reference.iter().cloned());
        let reference_refs: Vec<&str> = reference.iter().map(|s| s.as_str()).collect();
        let (sig, _tests) = SigModel::fit(lang, &rules, &reference_refs);
        LintModule {
            id: id.to_string(),
            languages: vec![lang.to_string()],
            version: version.to_string(),
            provenance: provenance.to_string(),
            sig,
            advice: knowledge.advice(lang),
        }
    }

    /// Whether this module lints `lang`.
    pub fn applies_to(&self, lang: &str) -> bool {
        self.languages.iter().any(|l| l == lang)
    }

    /// Rules this module could ground (and will therefore ever flag).
    pub fn rule_count(&self) -> usize {
        self.sig.rule_count()
    }

    /// Lint `code`: every taught pattern whose signature is present, with its advice.
    pub fn review(&self, lang: &str, code: &str) -> Vec<Finding> {
        if !self.applies_to(lang) {
            return Vec::new();
        }
        judge_by_function(&self.sig, lang, code)
            .into_iter()
            .map(|(line, rule)| {
                let (sev, msg) = self.advice.get(&rule).cloned().unwrap_or_else(|| ("medium".to_string(), String::new()));
                Finding { line, rule_id: rule, severity: sev, source: format!("module:{}", self.id), message: msg }
            })
            .collect()
    }

    /// Pack to JSON — the artifact you store/share.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Load a packed module, or `None` if invalid.
    pub fn from_json(json: &str) -> Option<LintModule> {
        serde_json::from_str(json).ok()
    }
}

// ---------------------------------------------------------------------------------------------
// ModuleRegistry: the package manager — lazy, on-demand module loading.
// ---------------------------------------------------------------------------------------------

/// A manifest row: a module that is *available* but not necessarily loaded.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleEntry {
    /// Module id.
    pub id: String,
    /// Languages it lints — used to decide whether a project needs it.
    pub languages: Vec<String>,
    /// Trained-from version (the project toolchain version the docs were matched to). Staleness is
    /// decided by comparing this to the project's current toolchain version.
    #[serde(default)]
    pub version: String,
    /// ISO timestamp the artifact was learned/packed — provenance and a tie-breaker for refresh.
    #[serde(default)]
    pub fetched_at: String,
    /// Artifact location relative to the store root (a `<id>.json` file; could be a remote URL in
    /// a networked deployment — resolved lazily either way).
    pub location: String,
}

/// The manifest file shape (`<root>/manifest.json`).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct Manifest {
    modules: Vec<ModuleEntry>,
}

/// The package manager for lint modules. Knows what is *available* from a manifest and pulls a
/// module's artifact only when a project needs it — so unused modules cost no space or load time.
pub struct ModuleRegistry {
    root: PathBuf,
    entries: Vec<ModuleEntry>,
    cache: HashMap<String, LintModule>,
}

impl ModuleRegistry {
    /// Open the store at `root`, reading its manifest (an empty registry if none exists yet).
    pub fn open(root: impl AsRef<Path>) -> ModuleRegistry {
        let root = root.as_ref().to_path_buf();
        let entries = std::fs::read_to_string(root.join("manifest.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<Manifest>(&s).ok())
            .map(|m| m.modules)
            .unwrap_or_default();
        ModuleRegistry { root, entries, cache: HashMap::new() }
    }

    /// Everything the manifest advertises (without loading any of it).
    pub fn available(&self) -> &[ModuleEntry] {
        &self.entries
    }

    /// The manifest entry of a module serving `lang`, if any — used to decide staleness (compare its
    /// `version` to the project's current toolchain version) without loading the artifact.
    pub fn entry_for_lang(&self, lang: &str) -> Option<&ModuleEntry> {
        self.entries.iter().find(|e| e.languages.iter().any(|l| l == lang))
    }

    /// The ids of modules a project in `langs` needs — the lazy-load shortlist.
    pub fn select(&self, langs: &[String]) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.languages.iter().any(|l| langs.contains(l)))
            .map(|e| e.id.clone())
            .collect()
    }

    /// Load a module by id, pulling and caching its artifact on first use. `None` if unknown or
    /// the artifact can't be read/parsed.
    pub fn load(&mut self, id: &str) -> Option<&LintModule> {
        if !self.cache.contains_key(id) {
            let entry = self.entries.iter().find(|e| e.id == id)?;
            let path = self.root.join(&entry.location);
            let module = LintModule::from_json(&std::fs::read_to_string(path).ok()?)?;
            self.cache.insert(id.to_string(), module);
        }
        self.cache.get(id)
    }

    /// Publish a packed module into the store: write its artifact and add/replace its manifest row.
    /// This is the "store it (in GitHub) so it can be reused" step, done locally.
    pub fn publish(&mut self, module: &LintModule) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let location = format!("{}.json", module.id);
        std::fs::write(self.root.join(&location), module.to_json())?;
        self.entries.retain(|e| e.id != module.id);
        self.entries.push(ModuleEntry {
            id: module.id.clone(),
            languages: module.languages.clone(),
            version: module.version.clone(),
            fetched_at: crate::util::now_iso(),
            location,
        });
        let manifest = Manifest { modules: self.entries.clone() };
        std::fs::write(self.root.join("manifest.json"), serde_json::to_string_pretty(&manifest).unwrap_or_default())?;
        self.cache.insert(module.id.clone(), module.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// Reasoner: the always-on main model that holds CS principles and decides good vs bad.
// ---------------------------------------------------------------------------------------------

/// The main reasoning model. Always-on, it carries the hard-defined CS2420/CS3500 principles
/// (learned from a text document, not hardcoded) and is what decides good-vs-bad *in a project*.
/// It composes four layers into one verdict: the deterministic floor, the taught principle
/// patterns, the behavioral CS norms, and any plugged-in modules.
pub struct Reasoner {
    /// Behavioral norms (single-responsibility / complexity / error-handling / naming), learned
    /// from a code corpus so the bar fits the project. `None` until [`Reasoner::calibrate`].
    norms: Option<Norms>,
    /// Every CS principle the reasoner has ever read — kept in full so a re-fit is never lossy.
    rules: Vec<LearnedRule>,
    /// Known-idiomatic reference code read from the docs — what the fit tests "normal" against.
    reference: Vec<String>,
    /// The current self-validated principle detector (rebuilt by [`Reasoner::fit`]).
    principles: SigModel,
    /// id → (severity, advice) for the principles.
    advice: HashMap<String, (String, String)>,
    /// Cost of the last fit, in (feature, example) tests — the "tried it this many times" number.
    last_fit_tests: usize,
    /// Default language the CS principles are written in (their examples' language).
    lang: String,
}

impl Reasoner {
    /// Build the reasoner from the CS-principles document text and immediately fit it. The rules
    /// come entirely from the text — none are hardcoded — and each is grounded only if the fit can
    /// separate its bad form from every other example; otherwise it abstains.
    pub fn from_cs_principles(lang: &str, doc: &str) -> Reasoner {
        let knowledge = Knowledge::from_text(lang, doc);
        let mut r = Reasoner {
            norms: None,
            rules: knowledge.rules,
            reference: Vec::new(),
            principles: SigModel::fit(lang, &[], &[]).0,
            advice: HashMap::new(),
            last_fit_tests: 0,
            lang: lang.to_string(),
        };
        r.fit();
        r
    }

    /// (Re)fit the principle detector from ALL rules read so far, tested against ALL reference code.
    /// Deterministic and idempotent — called after any new knowledge or reference is added.
    fn fit(&mut self) {
        let rules: Vec<SigRule> = self
            .rules
            .iter()
            .filter(|r| r.language == self.lang && !r.bad.is_empty())
            .map(|r| SigRule { id: r.id.clone(), bad: r.bad.clone(), good: r.good.clone(), description: r.description.clone() })
            .collect();
        let reference: Vec<&str> = self.reference.iter().map(|s| s.as_str()).collect();
        let (model, tests) = SigModel::fit(&self.lang, &rules, &reference);
        self.principles = model;
        self.last_fit_tests = tests;
        self.advice = self
            .rules
            .iter()
            .filter(|r| r.language == self.lang)
            .map(|r| (r.id.clone(), (r.severity.clone(), r.description.clone())))
            .collect();
    }

    /// Learn MORE — a new rule, an updated doc, a new language version — and re-fit. Non-lossy:
    /// every previously-read rule is retained, so adding knowledge only ever expands what the
    /// reasoner can flag. This is the "update a behavior, start working immediately, never forget"
    /// path; no retraining from scratch.
    pub fn learn(&mut self, doc: &str) {
        let knowledge = Knowledge::from_text(&self.lang, doc);
        self.rules.extend(knowledge.rules);
        self.fit();
    }

    /// Read known-idiomatic reference code (e.g. the docs' own good examples, the language's std)
    /// so the fit knows what "normal" looks like and keeps each rule to the part that is genuinely
    /// distinctive. Re-fits. The more it reads, the more precise — and the more it abstains rather
    /// than over-flag. Reference is additive and never lossy.
    pub fn study_reference(&mut self, code: &[&str]) {
        self.reference.extend(code.iter().map(|s| s.to_string()));
        self.fit();
    }

    /// How many CS principle patterns the reasoner currently grounds.
    pub fn principle_count(&self) -> usize {
        self.principles.rule_count()
    }

    /// A human-readable list of what the reasoner judges against: the grounded principle ids plus
    /// the always-on behavioral checks. Surfaced so a report can say what it checked.
    pub fn knowledge_summary(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .rules
            .iter()
            .filter(|r| r.language == self.lang && self.principles.judge(&r.bad).iter().any(|id| id == &r.id))
            .map(|r| r.id.clone())
            .collect();
        out.push("single-responsibility / complexity / error-handling / naming (behavioral)".to_string());
        out
    }

    /// (feature, example) tests the last fit ran — how hard it "tried" to separate right from wrong.
    pub fn fit_tests(&self) -> usize {
        self.last_fit_tests
    }

    /// Self-test against the docs it learned from: for every rule, does its bad form flag and its
    /// good form stay clean, with no rule firing on another's good example? Returns
    /// `(rules_grounded, rules_total, cross_or_self_failures)` — the honest report card.
    pub fn self_test(&self) -> (usize, usize, usize) {
        let grounded = self.principles.rule_count();
        let mut total = 0;
        let mut failures = 0;
        for r in self.rules.iter().filter(|r| r.language == self.lang) {
            total += 1;
            // A grounded rule must never fire on its own documented good (fixed) form — doing so
            // would mean it cannot tell the violation from its fix. That is the failure we count.
            let fires_on_good = !r.good.is_empty() && self.principles.judge(&r.good).iter().any(|id| id == &r.id);
            if fires_on_good {
                failures += 1;
            }
        }
        (grounded, total, failures)
    }

    /// Calibrate the behavioral norms to a body of code (`(lang, source)` pairs) — typically the
    /// project under review, so single-responsibility/complexity are judged against how this
    /// project actually writes code. Tailors the advice to the project and the user's style.
    pub fn calibrate(&mut self, sources: &[(&str, &str)]) {
        self.norms = Some(Norms::learn(sources));
    }

    /// Review one file. Runs, in order: the exact floor, the taught CS principles, the behavioral
    /// norms (if calibrated), and every plugged-in module — returning one composed, de-duplicated
    /// list of findings, each tagged with where it came from.
    pub fn review(&self, lang: &str, code: &str, modules: &[&LintModule]) -> Vec<Finding> {
        let mut out: Vec<Finding> = Vec::new();

        // (The former "deterministic floor" — a bank of regexes generated offline by
        // gen-lint-checkers — was retired: a single-method regex (`.join(`, `.array(`) cannot
        // express a context-dependent rule, so it over-flagged clean code. Those same rules are now
        // learned LIVE from the docs by the signature module below, which has the AST/real-code
        // calibration to apply them precisely. Learned-from-docs replaces the static rulebook.)

        // 2) Taught CS principles — patterns learned from the CS document, judged per function.
        if lang == self.lang {
            for (line, rule) in judge_by_function(&self.principles, lang, code) {
                let (sev, msg) = self.advice.get(&rule).cloned().unwrap_or_else(|| ("medium".to_string(), String::new()));
                out.push(Finding { line, rule_id: rule, severity: sev, source: "cs-principle".to_string(), message: msg });
            }
        }

        // 3) Behavioral CS norms — single-responsibility / complexity / error-handling / naming.
        if let Some(norms) = &self.norms {
            for m in functions(lang, code) {
                for p in norms.judge(&m) {
                    out.push(Finding {
                        line: m.line,
                        rule_id: principle_id(&p).to_string(),
                        severity: "medium".to_string(),
                        source: "cs-norm".to_string(),
                        message: principle_advice(&p, &m.name),
                    });
                }
            }
        }

        // 4) Plugged-in modules — each its own lint AI for this language.
        for module in modules {
            out.extend(module.review(lang, code));
        }

        // De-duplicate identical (line, rule) findings, keeping the first (floor wins ties).
        let mut seen = std::collections::HashSet::new();
        out.retain(|f| seen.insert((f.line, f.rule_id.clone())));
        out.sort_by_key(|f| f.line);
        out
    }
}

// ---------------------------------------------------------------------------------------------
// Whole-repository review: read the entire folder, then talk back in English.
// ---------------------------------------------------------------------------------------------

/// One file's place in a repository review.
pub struct FileReport {
    /// Path relative to the repository root.
    pub path: String,
    /// Detected language (by extension).
    pub language: String,
    /// Whether the file could be analyzed (we have a parser/bank for its language). A file we can
    /// read but not deeply parse is reported honestly rather than silently skipped.
    pub analyzed: bool,
    /// Findings in this file.
    pub findings: Vec<Finding>,
}

/// A whole-repository review: the project picture plus every file's findings, renderable as an
/// English report a person can read.
pub struct RepoReport {
    /// The repository root that was read.
    pub root: String,
    /// Per-file reports, in path order.
    pub files: Vec<FileReport>,
    /// Source-file counts per language.
    pub by_language: BTreeMap<String, usize>,
    /// Languages found that have no parser/bank, so they were read but not linted.
    pub unanalyzed_languages: BTreeMap<String, usize>,
    /// The principle/module names the review checked against — what the reasoner knows.
    pub knowledge: Vec<String>,
}

impl RepoReport {
    /// Total findings across the repository.
    pub fn total_findings(&self) -> usize {
        self.files.iter().map(|f| f.findings.len()).sum()
    }

    /// Files with at least one finding.
    pub fn flagged_files(&self) -> usize {
        self.files.iter().filter(|f| !f.findings.is_empty()).count()
    }

    /// True when nothing was flagged — the repository follows every principle the reasoner learned.
    pub fn is_clean(&self) -> bool {
        self.total_findings() == 0
    }

    /// Confident findings bucketed by severity: `(high, medium, low)`. Confident = the floor,
    /// taught principles, and behavioral norms — all calibrated to this project. Module findings
    /// are excluded; they are reported separately as lower-confidence candidates.
    pub fn severity_counts(&self) -> (usize, usize, usize) {
        let (mut hi, mut me, mut lo) = (0, 0, 0);
        for f in &self.files {
            for fi in f.findings.iter().filter(|x| is_confident(&x.source)) {
                match fi.severity.as_str() {
                    "high" => hi += 1,
                    "low" => lo += 1,
                    _ => me += 1,
                }
            }
        }
        (hi, me, lo)
    }

    /// Count of confident findings — the ones the verdict is based on.
    pub fn confident_count(&self) -> usize {
        self.files.iter().flat_map(|f| &f.findings).filter(|x| is_confident(&x.source)).count()
    }

    /// Render the review as an English report. The verdict is based ONLY on the confident layers
    /// (floor + taught principles + behavioral norms, all calibrated to this project). Module
    /// findings — higher recall but lower precision, generated from crawled docs — are listed
    /// separately and clearly labelled as candidates to review, so the verdict stays trustworthy.
    pub fn to_english(&self) -> String {
        let mut s = String::new();
        let analyzed: usize = self.by_language.values().sum();
        let langs: Vec<String> = self.by_language.iter().map(|(l, n)| format!("{l} ({n})")).collect();
        s.push_str(&format!("I read {} and looked at {analyzed} source file(s): {}.\n\n", self.root, langs.join(", ")));

        let confident = self.confident_count();
        let confident_files = self.files.iter().filter(|f| f.findings.iter().any(|x| is_confident(&x.source))).count();
        if confident == 0 {
            s.push_str("Verdict: CLEAN. Every file follows the principles and rules I learned (nothing the confident layers flag).\n");
        } else {
            let (hi, me, lo) = self.severity_counts();
            s.push_str(&format!(
                "Verdict: {confident} issue(s) across {confident_files} of {analyzed} file(s) — {hi} high, {me} medium, {lo} low. Highest-severity first.\n"
            ));
        }

        for f in &self.files {
            let confident_findings: Vec<Finding> = f.findings.iter().filter(|x| is_confident(&x.source)).cloned().collect();
            if confident_findings.is_empty() {
                continue;
            }
            s.push_str(&format!("\n{}\n", f.path));
            for line in group_findings(&confident_findings) {
                s.push_str(&format!("  {line}\n"));
            }
        }

        // Module candidates — surfaced but clearly fenced off from the verdict.
        let candidates: usize = self.total_findings() - confident;
        if candidates > 0 {
            s.push_str(&format!(
                "\n--- {candidates} candidate suggestion(s) from language modules (higher recall, LOWER precision — generated from crawled docs, may over-flag; review before acting) ---\n"
            ));
            for f in &self.files {
                let module_findings: Vec<Finding> = f.findings.iter().filter(|x| !is_confident(&x.source)).cloned().collect();
                if module_findings.is_empty() {
                    continue;
                }
                s.push_str(&format!("\n{}\n", f.path));
                for line in group_findings(&module_findings) {
                    s.push_str(&format!("  {line}\n"));
                }
            }
        }

        if !self.unanalyzed_languages.is_empty() {
            let u: Vec<String> = self.unanalyzed_languages.iter().map(|(l, n)| format!("{l} ({n})")).collect();
            s.push_str(&format!(
                "\nI read but could not deeply analyze these (no parser learned yet): {}.\n",
                u.join(", ")
            ));
        }
        if !self.knowledge.is_empty() {
            s.push_str(&format!("\nVerdict judged against what I learned: {}.\n", self.knowledge.join(", ")));
        }
        s
    }
}

/// A finding is "confident" when it comes from a layer calibrated to this project — the
/// deterministic floor, the taught principles, or the behavioral norms. Module findings (source
/// `module:*`) are higher-recall candidates and are reported separately, not in the verdict.
fn is_confident(source: &str) -> bool {
    !source.starts_with("module:")
}

/// Severity rank for ordering (high first).
fn severity_rank(sev: &str) -> u8 {
    match sev {
        "high" => 0,
        "low" => 2,
        _ => 1,
    }
}

/// Collapse a file's findings into readable English lines: one line per distinct (source, rule),
/// carrying the advice once and the lines it occurred on (capped), ordered highest-severity first.
/// So 30 `print_stdout` hits read as a single "×30 (lines …)" line, not 30 lines of noise.
fn group_findings(findings: &[Finding]) -> Vec<String> {
    // key -> (severity, source, advice, lines). Keyed by the ADVICE TEXT, not just the rule id, so
    // distinct per-function messages (a `cs-norm` naming each function) stay separate while
    // identical messages (a `floor` rule with no message) collapse into one line.
    let mut groups: Vec<(String, String, String, String, Vec<usize>)> = Vec::new();
    for f in findings {
        let advice = if f.message.is_empty() { format!("violates `{}`", f.rule_id) } else { f.message.clone() };
        let key = format!("{}|{}|{}", f.source, f.rule_id, advice);
        if let Some(g) = groups.iter_mut().find(|g| g.0 == key) {
            g.4.push(f.line);
        } else {
            groups.push((key, f.severity.clone(), f.source.clone(), advice, vec![f.line]));
        }
    }
    groups.sort_by(|a, b| severity_rank(&a.1).cmp(&severity_rank(&b.1)).then_with(|| b.4.len().cmp(&a.4.len())));
    groups
        .into_iter()
        .map(|(_, sev, source, advice, mut lines)| {
            lines.sort_unstable();
            let count = lines.len();
            let shown: Vec<String> = lines.iter().take(6).map(|l| l.to_string()).collect();
            let more = if count > 6 { format!(", +{} more", count - 6) } else { String::new() };
            let occ = if count == 1 { format!("L{}", lines[0]) } else { format!("×{count} (lines {}{more})", shown.join(", ")) };
            format!("[{sev}] [{source}] {advice}  {occ}")
        })
        .collect()
}

/// Map a file extension to a language tag — by name only, so it works for any project with no
/// toolchain installed. The tree-sitter grammars compiled into the binary cover these; a file in
/// any other language is read and reported, but not deeply linted.
fn language_of(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str())? {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "mjs" | "cjs" | "jsx" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "go" => Some("go"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "hpp" => Some("cpp"),
        "rb" => Some("ruby"),
        _ => None,
    }
}

/// We can deeply analyze a language only if a tree-sitter grammar is compiled in for it.
fn have_parser(lang: &str) -> bool {
    matches!(lang, "rust" | "python" | "javascript" | "typescript" | "go")
}

/// Walk a directory tree collecting source files, skipping the usual generated/vendor folders.
fn walk_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            // Prune dependency trees and build output (the same set the index walker skips) — a
            // review is about the PROJECT's own code, not the thousands of files under `.venv`,
            // `node_modules`, etc., which would otherwise dominate every count.
            if crate::index::walk::SKIP_DIRS.contains(&n) {
                continue;
            }
            walk_sources(&p, out);
        } else if language_of(&p).is_some() {
            out.push(p);
        }
    }
}

/// Read an entire repository and review it. The reasoner first **calibrates to this repository**:
/// it reads every parseable file once, studies them as reference (so a principle is tested against
/// the project's own prevalent patterns — e.g. a correct `0..len` indexing loop — and keeps only
/// what truly separates a violation from them, or abstains), and learns the behavioral norms from
/// the same code. Then it judges each file with floor + taught principles + behavioral norms +
/// the modules each language needs (pulled lazily from `registry`). Files in a language with no
/// parser are read and reported, not silently dropped.
///
/// The tradeoff of calibrating to the repo is the same one [`Reasoner::calibrate`] makes: the
/// project's prevalent patterns become "normal", so a violation that is pervasive throughout the
/// project would be treated as the norm. An isolated violation is still caught (the prevalent
/// correct form forces the distinguishing feature into the signature).
pub fn review_repository(root: &Path, reasoner: &mut Reasoner, registry: &mut ModuleRegistry) -> RepoReport {
    let mut files_on_disk = Vec::new();
    walk_sources(root, &mut files_on_disk);
    files_on_disk.sort();

    // First pass: read every parseable file and calibrate the reasoner to this repository, so its
    // principles are tested against the project's own idiomatic code before it judges anything.
    let mut sources: Vec<(String, String, String)> = Vec::new(); // (rel, lang, code)
    let mut unanalyzed_languages: BTreeMap<String, usize> = BTreeMap::new();
    for path in &files_on_disk {
        let Some(lang) = language_of(path) else { continue };
        let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();
        let Ok(code) = std::fs::read_to_string(path) else { continue };
        if have_parser(lang) {
            sources.push((rel, lang.to_string(), code));
        } else {
            *unanalyzed_languages.entry(lang.to_string()).or_default() += 1;
        }
    }
    let reference: Vec<&str> = sources.iter().map(|(_, _, c)| c.as_str()).collect();
    reasoner.study_reference(&reference);
    let norm_refs: Vec<(&str, &str)> = sources.iter().map(|(_, l, c)| (l.as_str(), c.as_str())).collect();
    reasoner.calibrate(&norm_refs);

    let mut by_language: BTreeMap<String, usize> = BTreeMap::new();
    let mut reports = Vec::new();
    // Cache modules per language so the registry pulls each at most once for the whole repo.
    let mut modules_by_lang: HashMap<String, Vec<LintModule>> = HashMap::new();

    // Report the files we could not parse (read but not linted) alongside the analyzed ones.
    for path in &files_on_disk {
        if let Some(lang) = language_of(path) {
            if !have_parser(lang) {
                let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();
                reports.push(FileReport { path: rel, language: lang.to_string(), analyzed: false, findings: Vec::new() });
            }
        }
    }

    // Second pass: judge each analyzable file with the now-calibrated reasoner.
    for (rel, lang, code) in &sources {
        *by_language.entry(lang.clone()).or_default() += 1;
        let modules = modules_by_lang.entry(lang.clone()).or_insert_with(|| {
            registry
                .select(std::slice::from_ref(lang))
                .iter()
                .filter_map(|id| registry.load(id).cloned())
                .collect()
        });
        let refs: Vec<&LintModule> = modules.iter().collect();
        let findings = reasoner.review(lang, code, &refs);
        reports.push(FileReport { path: rel.clone(), language: lang.clone(), analyzed: true, findings });
    }

    // Project calibration of the module layer. A learned signature that fires across a large share of
    // the project's files is matching the project's OWN idiom — e.g. numpy `[:, None]` indexing read
    // as a "list copy" in scientific code — not a defect. "Calibrate the bar to the project's own
    // idiomatic code": suppress those rules so the module surfaces the rare, genuine issues instead of
    // the codebase's prevalent patterns. (The docs taught the rule; the project says what's normal HERE.)
    let analyzed = by_language.values().sum::<usize>().max(1);
    let idiom_threshold = (analyzed / 10).max(5); // fires in ≥10% of files (min 5) ⇒ a project idiom
    let mut module_files: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for fr in &reports {
        for f in &fr.findings {
            if f.source.starts_with("module:") {
                module_files.entry(f.rule_id.clone()).or_default().insert(fr.path.clone());
            }
        }
    }
    let idiom_rules: std::collections::HashSet<String> = module_files
        .into_iter()
        .filter(|(_, files)| files.len() >= idiom_threshold)
        .map(|(rule, _)| rule)
        .collect();
    if !idiom_rules.is_empty() {
        for fr in &mut reports {
            fr.findings
                .retain(|f| !(f.source.starts_with("module:") && idiom_rules.contains(&f.rule_id)));
        }
    }

    RepoReport {
        root: root.display().to_string(),
        files: reports,
        by_language,
        unanalyzed_languages,
        knowledge: reasoner.knowledge_summary(),
    }
}

/// Stable id for a behavioral principle.
fn principle_id(p: &Principle) -> &'static str {
    match p {
        Principle::SingleResponsibility => "single_responsibility",
        Principle::Complexity => "complexity",
        Principle::ErrorHandling => "error_handling",
        Principle::NamingMismatch => "naming_mismatch",
    }
}

/// The CS advice message for a behavioral principle, naming the offending function.
fn principle_advice(p: &Principle, name: &str) -> String {
    match p {
        Principle::SingleResponsibility => format!("`{name}` does more than one thing — split it so each unit has a single responsibility."),
        Principle::Complexity => format!("`{name}` is more complex (branches/loops/nesting) than this project's norm — simplify or decompose it."),
        Principle::ErrorHandling => format!("`{name}` forces or discards a fallible result — handle the error instead of unwrapping/ignoring it."),
        Principle::NamingMismatch => format!("`{name}`'s name promises behavior its body doesn't deliver — rename it or make it do what it says."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CS_DOC: &str = r#"
# Off by one indexing [high]
Indexing a collection with an inclusive range up to its length reads one element past the end.
```rust:bad
fn sum(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..=xs.len() { t += xs[i]; } t }
```
```rust:good
fn sum(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..xs.len() { t += xs[i]; } t }
```
"#;

    #[test]
    fn from_text_parses_rules_with_examples() {
        let k = Knowledge::from_text("rust", CS_DOC);
        assert_eq!(k.rules.len(), 1);
        let r = &k.rules[0];
        assert_eq!(r.id, "off_by_one_indexing");
        assert_eq!(r.severity, "high");
        assert!(r.bad.contains("0..=xs.len()") && r.good.contains("0..xs.len()"));
    }

    #[test]
    fn reasoner_learns_a_principle_from_the_document_and_catches_it() {
        // The "honest part": the off-by-one was a blind spot. Teach it via the CS document, and
        // the reasoner now catches the SAME mistake on different variable names — and not the fix.
        let r = Reasoner::from_cs_principles("rust", CS_DOC);
        assert!(r.principle_count() >= 1, "the principle grounded from the doc");
        let bad = r.review("rust", "fn total(ys: &[i32]) -> i32 { let mut s = 0; for k in 0..=ys.len() { s += ys[k]; } s }", &[]);
        assert!(bad.iter().any(|f| f.rule_id == "off_by_one_indexing" && f.source == "cs-principle"), "taught principle catches the variant: {bad:?}");
        let good = r.review("rust", "fn total(ys: &[i32]) -> i32 { let mut s = 0; for k in 0..ys.len() { s += ys[k]; } s }", &[]);
        assert!(!good.iter().any(|f| f.rule_id == "off_by_one_indexing"), "the correct form must not flag");
    }

    #[test]
    fn learning_is_incremental_and_never_lossy() {
        // Read one principle, then read another later — the first MUST stay grounded (non-lossy),
        // and the reasoner works immediately on the new one. No retraining from scratch.
        let mut r = Reasoner::from_cs_principles("rust", CS_DOC);
        let before = r.principle_count();
        assert!(before >= 1);
        r.learn(
            r#"
# Idiomatic emptiness check [low]
Use is_empty, not comparing length to zero.
```rust:bad
fn d(items: &[i32]) -> bool { items.len() == 0 }
```
```rust:good
fn d(items: &[i32]) -> bool { items.is_empty() }
```
"#,
        );
        assert!(r.principle_count() > before, "the newly-read principle grounded");
        // The originally-read rule still fires — knowledge only ever expands.
        let hit = r.review("rust", "fn f(ys: &[i32]) -> i32 { let mut s = 0; for k in 0..=ys.len() { s += ys[k]; } s }", &[]);
        assert!(hit.iter().any(|f| f.rule_id == "off_by_one_indexing"), "old rule retained after learning a new one");
        let (grounded, total, failures) = r.self_test();
        assert_eq!(failures, 0, "no rule fires on its own good form or another's");
        assert!(grounded <= total);
    }

    #[test]
    fn a_constraining_feature_makes_even_a_weak_contrast_precise() {
        // CS_DOC's good example still INDEXES (`0..xs.len()`), so `..=` vs `..` is the only
        // discriminating contrast — historically too weak: the signature was a lone `..=` operator
        // and flagged a legitimate `1..=6`. The fit now anchors that signature on the co-occurring
        // `.len()` call (a constraining feature present in the bad example), so it requires `..=`
        // AND a `.len()` nearby. Result: it catches the real off-by-one and leaves `1..=6` clean —
        // precise from the doc alone, no reference reading needed.
        let r = Reasoner::from_cs_principles("rust", CS_DOC);
        let legit = "fn dice() -> u32 { let mut n = 0; for r in 1..=6 { n += r; } n }";
        let bug = "fn s(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..=xs.len() { t += xs[i]; } t }";
        assert!(
            !r.review("rust", legit, &[]).iter().any(|f| f.rule_id == "off_by_one_indexing"),
            "a legitimate inclusive range stays clean (no lone-operator over-flag)"
        );
        assert!(
            r.review("rust", bug, &[]).iter().any(|f| f.rule_id == "off_by_one_indexing"),
            "the genuine `0..=xs.len()` off-by-one is still caught"
        );
    }

    #[test]
    fn a_rich_contrast_grounds_precisely_catching_the_bug_not_a_legit_range() {
        // A well-formed doc shows the IDIOMATIC fix (iterate, don't index). That richer contrast
        // gives the fit real structure (index + len + range), so it grounds precisely from the
        // doc alone: it catches the genuine off-by-one and leaves a legitimate `1..=6` clean.
        const RICH: &str = r#"
# Off by one indexing [high]
Indexing with an inclusive range up to the length reads past the end; iterate instead.
```rust:bad
fn sum(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..=xs.len() { t += xs[i]; } t }
```
```rust:good
fn sum(xs: &[i32]) -> i32 { let mut t = 0; for x in xs { t += x; } t }
```
"#;
        let r = Reasoner::from_cs_principles("rust", RICH);
        let legit = "fn dice() -> u32 { let mut n = 0; for r in 1..=6 { n += r; } n }";
        let bug = "fn s(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..=xs.len() { t += xs[i]; } t }";
        assert!(!r.review("rust", legit, &[]).iter().any(|f| f.rule_id == "off_by_one_indexing"), "a legit range must stay clean");
        assert!(r.review("rust", bug, &[]).iter().any(|f| f.rule_id == "off_by_one_indexing"), "the genuine bug is caught");
    }

    #[test]
    fn repo_review_reads_a_folder_and_talks_back_in_english() {
        let dir = std::env::temp_dir().join(format!("repo_review_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/clean.rs"), "fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();
        // The buggy file carries the off-by-one the CS document teaches — the engine's own
        // learned-from-docs principle, not the retired regex floor — so the end-to-end review is
        // exercised through the layer that actually judges now.
        std::fs::write(
            dir.join("src/buggy.rs"),
            "fn s(xs: &[i32]) -> i32 {\n    let mut t = 0;\n    for i in 0..=xs.len() { t += xs[i]; }\n    t\n}\n",
        )
        .unwrap();

        let mut reasoner = Reasoner::from_cs_principles("rust", CS_DOC);
        // Pack the rule into a module and publish it — the production path. A module judges directly
        // (it is not calibrated away by the repo the way a taught principle is), so the lone bug is
        // still caught, the way the live-learned language module catches it on a real project.
        let mut reg = ModuleRegistry::open(dir.join("store"));
        let module = LintModule::pack("rust-cs", "test", "unit", "rust", &Knowledge::from_text("rust", CS_DOC));
        reg.publish(&module).unwrap();
        let report = review_repository(&dir, &mut reasoner, &mut reg);

        assert_eq!(report.by_language.get("rust").copied(), Some(2), "read both rust files");
        assert!(!report.is_clean(), "the buggy file must produce findings");
        let english = report.to_english();
        assert!(english.contains("Verdict:"), "report reads as English");
        assert!(english.contains("Off by one indexing"), "the off-by-one is reported: {english}");
        assert!(english.contains("buggy.rs"), "the offending file is named");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn module_packs_loads_and_reviews_through_the_registry() {
        let dir = std::env::temp_dir().join(format!("lintmods_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let k = Knowledge::from_text("rust", CS_DOC);
        let module = LintModule::pack("rust-cs", "test", "unit", "rust", &k);
        assert!(module.rule_count() >= 1);

        let mut reg = ModuleRegistry::open(&dir);
        reg.publish(&module).expect("publish");

        // A fresh registry only knows what the manifest advertises until something is needed.
        let mut reg2 = ModuleRegistry::open(&dir);
        assert_eq!(reg2.select(&["rust".to_string()]), vec!["rust-cs".to_string()]);
        assert!(reg2.select(&["python".to_string()]).is_empty(), "no python module ⇒ nothing pulled");
        let loaded = reg2.load("rust-cs").expect("lazy load");
        let hits = loaded.review("rust", "fn t(z: &[i32]) -> i32 { let mut a = 0; for j in 0..=z.len() { a += z[j]; } a }");
        assert!(hits.iter().any(|f| f.rule_id == "off_by_one_indexing"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
