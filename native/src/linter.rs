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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::lint_checkers;
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
        Ok(Knowledge { rules })
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
        Knowledge { rules }
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

    /// The corpus (bad examples) for `lang` — what rarity is measured against during training.
    fn corpus(&self, lang: &str) -> Vec<String> {
        self.rules.iter().filter(|r| r.language == lang).map(|r| r.bad.clone()).collect()
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
    /// result is serialized and shared so no machine repeats it.
    pub fn pack(id: &str, version: &str, provenance: &str, lang: &str, knowledge: &Knowledge) -> LintModule {
        let rules = knowledge.sig_rules(lang);
        let corpus = knowledge.corpus(lang);
        let corpus_refs: Vec<&str> = corpus.iter().map(|s| s.as_str()).collect();
        let sig = SigModel::train(lang, &rules, &corpus_refs);
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
    /// Trained-from version.
    #[serde(default)]
    pub version: String,
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
    /// Pattern-style CS principles taught from the CS document (e.g. off-by-one indexing).
    principles: SigModel,
    /// id → (severity, advice) for the taught principles.
    advice: HashMap<String, (String, String)>,
    /// Default language the CS principles are written in (their examples' language).
    lang: String,
}

impl Reasoner {
    /// Build the reasoner from the CS-principles document text. The principles' bad/good examples
    /// become a taught signature detector (so the rules are learned from the doc, not coded), and
    /// their descriptions become the advice the reasoner gives.
    ///
    /// Each principle in the document must supply both a bad and a good example; the reasoner
    /// learns the pattern from their structural difference ([`SigModel::train_trusted`]), so the
    /// rules come entirely from the text — none are hardcoded — and a principle that lacks a clear
    /// structural contrast abstains rather than guess.
    pub fn from_cs_principles(lang: &str, doc: &str) -> Reasoner {
        let knowledge = Knowledge::from_text(lang, doc);
        let rules = knowledge.sig_rules(lang);
        Reasoner {
            norms: None,
            principles: SigModel::train_trusted(lang, &rules),
            advice: knowledge.advice(lang),
            lang: lang.to_string(),
        }
    }

    /// How many CS principle patterns the reasoner grounded from the document.
    pub fn principle_count(&self) -> usize {
        self.principles.rule_count()
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
        let lines: Vec<&str> = code.lines().collect();
        let mut out: Vec<Finding> = Vec::new();

        // 1) Deterministic floor — exact, zero false positives.
        if let Some(set) = lint_checkers::assemble(lang) {
            for h in set.run(&lines) {
                out.push(Finding {
                    line: h.line,
                    rule_id: h.rule_id,
                    severity: h.severity,
                    source: "floor".to_string(),
                    message: String::new(),
                });
            }
        }

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
