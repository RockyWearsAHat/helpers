//! `lint_train` — self-setup for the AI linter: it **learns the rules itself** from the two
//! knowledge sources, compiles one exact-pattern rule set per language, and caches the result so a
//! lint run loads instead of relearning.
//!
//! Knowledge enters from exactly two places — never from memory, never from a rule hardcoded in
//! Rust, and never requiring an external indexer to be run first:
//!
//!   1. **The documentation from the links** — the official rule docs for each language
//!      (clippy / ruff / eslint / staticcheck). The linter learns them *itself* by crawling the
//!      live docs ([`crate::lint_docs`]): it reads each rule's anti-pattern, fix, and lesson. What
//!      it learns is cached and refreshed when the toolchain version changes, so it stays current —
//!      never stale, never dependent on someone pre-building an index. A committed `lint-index/`
//!      snapshot, when present, is used as a fast seed; the embedded copy is the offline fallback.
//!   2. **Files in a folder** — `corpus/` (`cs-principles.md`): the CS2420 / CS3500 rules. Each
//!      fenced `bad`/`good` pair becomes a pattern the model detects.
//!
//! Training is the slow step, so [`ensure_models`] does it once and caches it, gated on a checksum
//! of the resolved rules + toolchain version: it relearns only when the docs or the folder change.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::lint_match::RuleSet;

/// How many doc pages to crawl when learning a language whose docs are a site (ruff/eslint publish
/// ~1000 rule pages). High enough to read the whole rule set; the learned catalog is cached, so the
/// crawl cost is paid once per toolchain version.
#[cfg(feature = "crawl")]
const MAX_CRAWL_PAGES: usize = 2000;

/// Bump when the training logic changes so existing caches are treated as stale and relearned.
const TRAIN_VERSION: &str = "pattern-v1-lossless";

/// The committed rule catalogs, embedded so an installed binary far from the checkout still has a
/// documentation seed to learn from offline. The live crawl (when reachable) and the on-disk
/// `lint-index/` are both preferred over this.
static EMBEDDED_LINT_INDEX: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../lint-index");

/// The committed per-language modules, embedded so an installed binary far from the checkout still
/// ships every language the linter has learned (Go, and the example-rich rust/python/js catalogs).
/// The on-disk `lint-models/` is preferred (editing/adding a module takes effect on pull).
static EMBEDDED_LINT_MODELS: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../lint-models");

/// The CS principles folder document, embedded as the offline fallback (the on-disk copy is
/// preferred so editing it relearns on the next run).
const EMBEDDED_CS_PRINCIPLES: &str = include_str!("../../corpus/cs-principles.md");

/// One documented rule, normalized across all sources into the shape the engine compiles from: an id, a
/// routing `slice` (the doc category, or severity when the source has no category), severity,
/// English advice, the anti-pattern, its fix, and a doc URL for citation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DocRule {
    id: String,
    slice: String,
    severity: String,
    description: String,
    bad: String,
    good: String,
    #[serde(default)]
    source: String,
}

/// A language's learned rule catalog, cached so the linter does not relearn every run. Keyed by the
/// toolchain `version` it was learned for, so a version bump triggers a fresh crawl ("stay current").
#[derive(Serialize, Deserialize)]
struct LearnedCatalog {
    /// Toolchain version the rules were learned for (empty when undetectable).
    version: String,
    /// Where the rules came from (a tool name, `committed`, or `embedded`) — provenance.
    learned_from: String,
    /// The normalized rules.
    rules: Vec<DocRule>,
    /// Real idiomatic code the docs served — the clean reference that calibrates each signal so
    /// only the genuinely-distinctive part of a rule fires. Empty for the seed (it carries no
    /// reference code).
    #[serde(default)]
    reference: Vec<String>,
}

/// The reportable facts about a rule the compiled pattern itself does not carry: severity, the English
/// advice (its description), and the doc URL it was sourced from. Looked up by rule id when
/// rendering a finding, so the verdict can explain itself and cite its source.
#[derive(Clone, Debug, Default)]
pub struct RuleInfo {
    /// Severity bucket (`high`/`medium`/`low`).
    pub severity: String,
    /// English description — the advice a reader or fixing agent acts on.
    pub description: String,
    /// Direct URL to the rule's official documentation (empty for folder rules).
    pub source: String,
}

/// What [`ensure_models`] did this run — so the tool can report self-setup honestly.
#[derive(Default, Debug)]
pub struct TrainReport {
    /// Languages whose model was (re)trained and cached this run.
    pub trained: Vec<String>,
    /// Languages whose cached model was already fresh and reused.
    pub reused: Vec<String>,
    /// Languages skipped, with the reason (no documented rules, no learnable signal, …).
    pub skipped: Vec<(String, String)>,
    /// Languages whose rules were (re)learned from the live docs this run.
    pub crawled: Vec<String>,
}

/// Ensure a fresh, cached compiled [`RuleSet`] exists for each requested language, learning from the
/// docs + the corpus folder only. Idempotent and checksum-gated: a language whose resolved rules and
/// toolchain version are unchanged is reused, not relearned. `data_root` holds `lint-index/` (the
/// seed) and `corpus/` (the folder); missing on-disk sources fall back to the embedded copies, and a
/// stale/absent catalog is relearned from the live docs when the crawler is available. Each rule is
/// compiled to its exact tree pattern from its own bad/good example — no thresholds, no statistics —
/// so a match is the rule's structure occurring verbatim, with scope and co-reference intact.
/// Load a language's rules from the project's own `.helpers/lint-rules/` directory.
///
/// Reads `<lang>.md` (rules specific to that language) and `any.md` (rules that apply to every
/// language). These are authored in the same markdown format as `corpus/cs-principles.md`:
/// a `## rule-id [severity]` heading followed by a description and optional `bad`/`good` fenced
/// blocks. Project rules are merged AFTER the global corpus, so they take priority over it.
pub(crate) fn project_rules(project_root: &Path, lang: &str) -> Vec<DocRule> {
    let dir = project_root.join(".helpers/lint-rules");
    let mut out = Vec::new();
    for filename in [format!("{lang}.md"), "any.md".to_string()] {
        let path = dir.join(&filename);
        let Ok(doc) = std::fs::read_to_string(&path) else { continue };
        let source = path.to_string_lossy().into_owned();
        for r in crate::linter::Knowledge::from_text(lang, &doc).rules {
            if r.bad.is_empty() {
                continue;
            }
            out.push(DocRule {
                id: r.id,
                slice: "project-rule".to_string(),
                severity: r.severity,
                description: r.description,
                bad: r.bad,
                good: r.good,
                source: source.clone(),
            });
        }
    }
    out
}

/// Expose the stamp file path so external tools (e.g. `lint_rule`) can invalidate it,
/// forcing a retrain on the next `lint` call without requiring a version bump.
pub fn stamp_path_pub(lang: &str) -> PathBuf {
    stamp_path(lang)
}

pub fn ensure_models(langs: &[String], data_root: &Path, project_root: &Path) -> TrainReport {
    let mut report = TrainReport::default();
    let folder = corpus_rules(data_root);
    for lang in langs {
        let version = crate::lint_checkers::detect_version(lang).unwrap_or_default();
        let (mut rules, _reference, learned_from) = resolve_rules(data_root, lang, &version, &mut report);
        // CS-principles folder rules for this language.
        rules.extend(folder.iter().filter(|(l, _)| l == lang).map(|(_, r)| r.clone()));
        // Project-local rules — higher priority than global corpus, so appended last.
        rules.extend(project_rules(project_root, lang));
        if rules.iter().all(|r| r.bad.is_empty()) {
            report.skipped.push((lang.clone(), "no rule has a bad example to learn the pattern from".to_string()));
            continue;
        }
        let stamp = stamp_of(&version, &rules);
        if model_fresh(&patterns_path(lang), &stamp_path(lang), &stamp) {
            report.reused.push(lang.clone());
            continue;
        }
        let tuples: Vec<(String, String, String, String, String)> = rules
            .iter()
            .map(|r| (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone()))
            .collect();
        let model = RuleSet::build(lang, &tuples);
        if model.rule_count() == 0 {
            report.skipped.push((lang.clone(), "no rule carried a distinctive pattern to match".to_string()));
            continue;
        }
        if let Some(parent) = patterns_path(lang).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(patterns_path(lang), model.to_json()).is_ok() {
            let _ = std::fs::write(stamp_path(lang), &stamp);
            report.trained.push(format!("{lang} ({} rules, from {learned_from})", model.rule_count()));
        } else {
            report.skipped.push((lang.clone(), "could not write the cached model".to_string()));
        }
    }
    report
}

/// Load a language's cached compiled rule set, or `None` if absent/unreadable.
pub fn load_patterns(lang: &str) -> Option<RuleSet> {
    RuleSet::from_json(&std::fs::read_to_string(patterns_path(lang)).ok()?)
}

/// Directory where trained per-language models live: a committed `lint-models/` in the repo (so a
/// `git pull` ships every language's compiled patterns) is preferred, then the user cache. One-time
/// training writes to the cache; the `lint` tool loads from whichever is present. Override with
/// `HELPERS_LINT_MODELS`.
fn model_dir() -> PathBuf {
    if let Ok(d) = std::env::var("HELPERS_LINT_MODELS") {
        return PathBuf::from(d);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Path::new(&home).join(".cache/helpers/lint-models")
}

/// Path to a language's cached compiled patterns (`<lang>.patterns.json`, beside the cache root).
fn patterns_path(lang: &str) -> PathBuf {
    model_dir().join(format!("{lang}.patterns.json"))
}

// ── rule resolution: the AI learns its own rules, cached and version-current ──

/// Resolve a language's documented rules, in order of freshness:
///
///   1. the linter's own learned cache, when it matches the detected toolchain version;
///   2. a **committed module** (`lint-models/<lang>.learned.json`) — a catalog already crawled and
///      checked in to the repo, so a `git pull` ships every language's rules (and the reference code
///      that calibrates them) to everyone, working offline and instantly with no per-machine crawl.
///      This is how a language learned once is shared: ingest a link, commit the module, others pull;
///   3. the committed/embedded `lint-index/` snapshot, when it covers that version (fast, and carries
///      doc categories) — so a present, current seed avoids a needless crawl;
///   4. a **live crawl of the official docs** otherwise (stale/absent seed, or `HELPERS_LINT_REFRESH`
///      set) — this is the AI learning the rules itself and is what keeps it current; the result is
///      cached, version-keyed, so later runs are fast and only relearn on a version bump;
///   5. the seed again as the offline fallback when a crawl is unavailable.
///
/// Records crawl activity in `report`. Returns the rules and a short provenance label.
fn resolve_rules(
    data_root: &Path,
    lang: &str,
    version: &str,
    report: &mut TrainReport,
) -> (Vec<DocRule>, Vec<String>, String) {
    let refresh = std::env::var_os("HELPERS_LINT_REFRESH").is_some();
    if !refresh {
        if let Some(cat) = load_cache(lang) {
            if cat.version == version && !cat.rules.is_empty() {
                return (cat.rules, cat.reference, format!("cache:{}", cat.learned_from));
            }
        }
        // A committed module is a high-quality seed (real bad/good pairs + reference code). It is
        // used regardless of toolchain version — like the snapshot, it is a starting point that an
        // explicit `HELPERS_LINT_REFRESH` re-crawls. Preferred over the bare `lint-index/` snapshot.
        if let Some(cat) = load_committed_module(data_root, lang) {
            if !cat.rules.is_empty() {
                let (seed_rules, _) = seed_with_version(data_root, lang);
                let existing: std::collections::HashSet<String> =
                    cat.rules.iter().map(|r| r.id.clone()).collect();
                let mut rules = cat.rules;
                rules.extend(seed_rules.into_iter().filter(|r| !existing.contains(&r.id)));
                return (rules, cat.reference, "committed module".to_string());
            }
        }
    }
    let (seed, seed_version) = seed_with_version(data_root, lang);
    // A present seed that covers the detected version (or when no version is detectable / the seed
    // is unpinned) is used directly — no reason to crawl docs we already mirror. The seed carries
    // no reference code (its caps lean on the rules' own good examples).
    let seed_current = !seed.is_empty() && (version.is_empty() || seed_version.is_empty() || seed_version == version);
    if !refresh && seed_current {
        return (seed, Vec::new(), "committed snapshot".to_string());
    }
    // Learn it ourselves from the live docs. Cache what we learn (rules + reference), keyed by the
    // toolchain version, so the next run is fast and only relearns on a bump.
    if let Some((rules, reference)) = crawl_learn(data_root, lang, version) {
        if !rules.is_empty() {
            report.crawled.push(lang.to_string());
            save_cache(
                lang,
                &LearnedCatalog {
                    version: version.to_string(),
                    learned_from: "docs".to_string(),
                    rules: rules.clone(),
                    reference: reference.clone(),
                },
            );
            return (rules, reference, "live docs".to_string());
        }
    }
    // Offline or crawl-disabled: fall back to the snapshot (stale is better than nothing).
    if !seed.is_empty() {
        return (seed, Vec::new(), "committed snapshot".to_string());
    }
    (Vec::new(), Vec::new(), "nothing".to_string())
}

/// Crawl the official docs for `lang` and normalize what is learned into [`DocRule`]s plus the
/// crawl's `reference` — every real code block the docs served, the "what's normal in this
/// language" sample that calibrates each signal so an incidental token never becomes a violation.
/// `None` when the language has no known docs URL or the crawler is not compiled in.
#[cfg(feature = "crawl")]
fn crawl_learn(data_root: &Path, lang: &str, version: &str) -> Option<(Vec<DocRule>, Vec<String>)> {
    // Operational escape hatch: skip all network learning (air-gapped runs, and deterministic
    // tests) — the resolver then uses the committed/embedded seed instead.
    if std::env::var_os("HELPERS_LINT_OFFLINE").is_some() {
        return None;
    }
    let src = crate::lint_docs::known_docs_url(lang, version)
        .or_else(|| crawl_source_from_config(data_root, lang))?;
    let knowledge = if lang == "rust" {
        crate::lint_docs::learn_clippy(lang, version, MAX_CRAWL_PAGES)
    } else {
        crate::lint_docs::learn_from_url(lang, &src, MAX_CRAWL_PAGES)
    };
    if knowledge.rules.is_empty() {
        return None;
    }
    let rules = knowledge
        .rules
        .into_iter()
        .map(|r| DocRule {
            slice: r.severity.clone(),
            source: src.url.clone(),
            id: r.id,
            severity: r.severity,
            description: r.description,
            bad: r.bad,
            good: r.good,
        })
        .collect();
    Some((rules, knowledge.reference))
}

/// Read `sources.json` from the data root (prefer on-disk, fall back to embedded) and return a
/// [`crate::lint_docs::DocsSource`] for `lang` when the source kind supports crawling. Skips
/// `kind:"builtin"` entries (handled by `known_docs_url`). For `kind:"crawl"` uses the `seed`
/// field; for `kind:"agent"` uses `docsBase` as a best-effort crawl target.
#[cfg(feature = "crawl")]
fn crawl_source_from_config(data_root: &Path, lang: &str) -> Option<crate::lint_docs::DocsSource> {
    let raw = std::fs::read_to_string(data_root.join("lint-index/sources.json"))
        .ok()
        .or_else(|| {
            EMBEDDED_LINT_INDEX
                .get_file("sources.json")
                .and_then(|f| f.contents_utf8().map(str::to_string))
        })?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    for entry in json["sources"].as_array()?.iter() {
        let entry_lang = entry["language"].as_str().unwrap_or("");
        if !lang_matches(entry_lang, lang) {
            continue;
        }
        let kind = entry["kind"].as_str().unwrap_or("");
        let tool = entry["tool"].as_str().unwrap_or("").to_string();
        match kind {
            "crawl" => {
                let url = entry["seed"].as_str()?.to_string();
                return Some(crate::lint_docs::DocsSource { url, crawl: true, tool });
            }
            "agent" => {
                let url = entry["docsBase"].as_str()?.to_string();
                return Some(crate::lint_docs::DocsSource { url, crawl: true, tool });
            }
            _ => continue,
        }
    }
    None
}

#[cfg(not(feature = "crawl"))]
fn crawl_learn(_data_root: &Path, _lang: &str, _version: &str) -> Option<(Vec<DocRule>, Vec<String>)> {
    None
}

/// The committed/embedded rule snapshot for `lang` — the offline seed — plus the toolchain version
/// it was built for (so the resolver can tell whether it is current). Reads every
/// `lint-index/<tool>.json` whose `language` matches, preferring the on-disk copies and falling
/// back to the embedded ones. These carry a doc `category`, used as the routing slice. The version
/// is the first matching catalog's `toolchainVersion` (else `docsVersion`).
fn seed_with_version(data_root: &Path, lang: &str) -> (Vec<DocRule>, String) {
    let mut out = Vec::new();
    let mut version = String::new();
    for raw in seed_catalogs(data_root) {
        let Ok(idx) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        if !idx["language"].as_str().is_some_and(|l| lang_matches(l, lang)) {
            continue;
        }
        if version.is_empty() {
            version = idx["toolchainVersion"]
                .as_str()
                .or_else(|| idx["docsVersion"].as_str())
                .unwrap_or("")
                .to_string();
        }
        for r in idx["rules"].as_array().into_iter().flatten() {
            let bad = r["exampleBad"].as_str().unwrap_or("");
            if bad.is_empty() {
                continue;
            }
            out.push(DocRule {
                id: r["id"].as_str().unwrap_or("").to_string(),
                slice: r["category"].as_str().unwrap_or("other").to_string(),
                severity: r["severity"].as_str().unwrap_or("medium").to_string(),
                description: r["description"].as_str().unwrap_or("").to_string(),
                bad: bad.to_string(),
                good: r["exampleGood"].as_str().unwrap_or("").to_string(),
                source: r["source"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    (out, version)
}

/// Whether a catalog's `language` serves the requested model language.
/// TypeScript extends JavaScript: a TypeScript model learns all JavaScript rules too.
fn lang_matches(catalog: &str, want: &str) -> bool {
    // Normalize short aliases to canonical language names.
    let norm = |s: &str| match s.to_ascii_lowercase().as_str() {
        "js" | "jsx" => "javascript".to_string(),
        "ts" | "tsx" => "typescript".to_string(),
        other => other.to_ascii_lowercase(),
    };
    let c = norm(catalog);
    if c == want {
        return true;
    }
    // TypeScript is a superset of JavaScript: include all JavaScript rules in the TypeScript model.
    if want == "typescript" && c == "javascript" {
        return true;
    }
    false
}

/// The raw JSON of every committed/embedded rule catalog (excluding `sources.json`).
fn seed_catalogs(data_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(data_root.join("lint-index")) {
        for entry in rd.flatten() {
            if is_catalog_name(entry.path().file_name().and_then(|n| n.to_str())) {
                if let Ok(s) = std::fs::read_to_string(entry.path()) {
                    out.push(s);
                }
            }
        }
    }
    if out.is_empty() {
        for f in EMBEDDED_LINT_INDEX.files() {
            if is_catalog_name(f.path().file_name().and_then(|n| n.to_str())) {
                if let Some(s) = f.contents_utf8() {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

/// A `lint-index` entry is a rule catalog if it is a `*.json` and not the source registry.
fn is_catalog_name(name: Option<&str>) -> bool {
    matches!(name, Some(n) if n.ends_with(".json") && n != "sources.json")
}

/// The CS-principles folder rules as `(language, DocRule)` — the second knowledge source. Prefers
/// the on-disk document (edit it and the next lint relearns) and falls back to the embedded copy.
fn corpus_rules(data_root: &Path) -> Vec<(String, DocRule)> {
    let doc = std::fs::read_to_string(data_root.join("corpus/cs-principles.md"))
        .unwrap_or_else(|_| EMBEDDED_CS_PRINCIPLES.to_string());
    crate::linter::Knowledge::from_text("rust", &doc)
        .rules
        .into_iter()
        .filter(|r| !r.bad.is_empty())
        .map(|r| {
            (
                r.language.clone(),
                DocRule {
                    id: r.id,
                    slice: "cs-principle".to_string(),
                    severity: r.severity,
                    description: r.description,
                    bad: r.bad,
                    good: r.good,
                    source: String::new(),
                },
            )
        })
        .collect()
}

/// Build the rule-id → [`RuleInfo`] map for rendering findings, from the SAME sources the models
/// learned from (cached learned catalogs + committed seed + corpus folder + project rules), so every
/// finding's advice and citation trace back to a doc link or a rule file and nothing else.
/// Read-only — never crawls (that already happened during [`ensure_models`]).
pub fn advice(data_root: &Path, project_root: Option<&Path>) -> HashMap<String, RuleInfo> {
    /// Record a rule's reportable facts, later sources overriding earlier (more-current) ones.
    fn put(out: &mut HashMap<String, RuleInfo>, r: &DocRule) {
        out.insert(
            r.id.clone(),
            RuleInfo { severity: r.severity.clone(), description: r.description.clone(), source: r.source.clone() },
        );
    }
    let mut out: HashMap<String, RuleInfo> = HashMap::new();
    // Committed/embedded seed (all languages).
    for raw in seed_catalogs(data_root) {
        if let Ok(idx) = serde_json::from_str::<serde_json::Value>(&raw) {
            for r in idx["rules"].as_array().into_iter().flatten() {
                if let Some(id) = r["id"].as_str() {
                    out.insert(
                        id.to_string(),
                        RuleInfo {
                            severity: r["severity"].as_str().unwrap_or("medium").to_string(),
                            description: r["description"].as_str().unwrap_or("").to_string(),
                            source: r["source"].as_str().unwrap_or("").to_string(),
                        },
                    );
                }
            }
        }
    }
    // Committed modules override the bare seed (they carry full descriptions + sources).
    if let Ok(rd) = std::fs::read_dir(committed_modules_dir(data_root)) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.to_string_lossy().ends_with(".learned.json") {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    if let Ok(cat) = serde_json::from_str::<LearnedCatalog>(&s) {
                        for r in &cat.rules {
                            put(&mut out, r);
                        }
                    }
                }
            }
        }
    }
    // Anything the linter learned itself and cached overrides the seed (it is more current).
    if let Ok(rd) = std::fs::read_dir(model_dir()) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.to_string_lossy().ends_with(".learned.json") {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    if let Ok(cat) = serde_json::from_str::<LearnedCatalog>(&s) {
                        for r in &cat.rules {
                            put(&mut out, r);
                        }
                    }
                }
            }
        }
    }
    // Folder rules (the CS principles).
    for (_, r) in corpus_rules(data_root) {
        put(&mut out, &r);
    }
    // Project-local rules — highest priority; their descriptions override everything else
    // so a user's custom advice appears verbatim in lint output.
    if let Some(pr) = project_root {
        let dir = pr.join(".helpers/lint-rules");
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let lang_hint = path.file_stem().and_then(|s| s.to_str()).unwrap_or("any");
                if let Ok(doc) = std::fs::read_to_string(&path) {
                    let src = path.to_string_lossy().into_owned();
                    for r in crate::linter::Knowledge::from_text(lang_hint, &doc).rules {
                        out.insert(
                            r.id.clone(),
                            RuleInfo {
                                severity: r.severity,
                                description: r.description,
                                source: src.clone(),
                            },
                        );
                    }
                }
            }
        }
    }
    out
}

// ── public training API ──────────────────────────────────────────────────────

/// The result of a successful `learn_and_commit` call.
pub struct LearnResult {
    /// The language that was trained.
    pub lang: String,
    /// Number of rules learned from the docs.
    pub rule_count: usize,
    /// Number of those rules that compiled to a matchable tree pattern.
    pub pattern_count: usize,
    /// Path of the committed module that was written.
    pub module_path: PathBuf,
}

/// Force-crawl a language's registered docs URL, compile the model, and persist it as a
/// committed module (`<data_root>/lint-models/<lang>.learned.json`). This is how a trained
/// language is shared: commit the module, push, open a PR — others get it on `git pull` with
/// no per-machine crawl. Also updates the user's local pattern cache so the next `lint` run
/// loads immediately. Returns an error when no docs URL is registered for the language or the
/// crawl returns no rules.
#[cfg(feature = "crawl")]
pub fn learn_and_commit(lang: &str, data_root: &Path) -> Result<LearnResult, String> {
    let version = crate::lint_checkers::detect_version(lang).unwrap_or_default();
    let (rules, reference) =
        crawl_learn(data_root, lang, &version).ok_or_else(|| {
            format!(
                "no docs URL configured for `{lang}` — add one with `lint_add_source` first, \
                 or set HELPERS_LINT_OFFLINE to use a committed module"
            )
        })?;
    if rules.is_empty() {
        return Err(format!("crawled docs for `{lang}` but found no rules with examples"));
    }
    let rule_count = rules.len();
    let catalog = LearnedCatalog {
        version: version.clone(),
        learned_from: "docs".to_string(),
        rules: rules.clone(),
        reference,
    };
    // Save to user cache.
    save_cache(lang, &catalog);
    // Compile the pattern model and cache it.
    let stamp = stamp_of(&version, &rules);
    let tuples: Vec<(String, String, String, String, String)> = rules
        .iter()
        .map(|r| (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone()))
        .collect();
    let model = crate::lint_match::RuleSet::build(lang, &tuples);
    let pattern_count = model.rule_count();
    let _ = std::fs::write(patterns_path(lang), model.to_json());
    let _ = std::fs::write(stamp_path(lang), &stamp);
    // Persist as a committed module so `git pull` ships it to others.
    let module_dir = committed_modules_dir(data_root);
    let _ = std::fs::create_dir_all(&module_dir);
    let module_path = module_dir.join(format!("{lang}.learned.json"));
    let json = serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?;
    std::fs::write(&module_path, json).map_err(|e| format!("could not write module: {e}"))?;
    Ok(LearnResult { lang: lang.to_string(), rule_count, pattern_count, module_path })
}

#[cfg(not(feature = "crawl"))]
pub fn learn_and_commit(lang: &str, _data_root: &Path) -> Result<LearnResult, String> {
    Err(format!(
        "learn_and_commit requires the `crawl` feature; \
         rebuild with `cargo build --features crawl` to enable doc-crawling for `{lang}`"
    ))
}

// ── cache + checksum plumbing ────────────────────────────────────────────────

/// Path to a language's learned-rule cache (`<lang>.learned.json`, beside its model).
fn cache_path(lang: &str) -> PathBuf {
    model_dir().join(format!("{lang}.learned.json"))
}

/// Load a language's cached learned catalog, or `None` if absent/unreadable.
fn load_cache(lang: &str) -> Option<LearnedCatalog> {
    serde_json::from_str(&std::fs::read_to_string(cache_path(lang)).ok()?).ok()
}

/// The committed per-language modules directory: `lint-models/` beside `lint-index/` and `corpus/`.
/// A module here is checked into the repo, so it ships with a `git pull` — the shared, pullable form
/// of a language the linter has already learned.
fn committed_modules_dir(data_root: &Path) -> PathBuf {
    data_root.join("lint-models")
}

/// Load a committed module (`lint-models/<lang>.learned.json`) — a crawled catalog checked into the
/// repo so every clone has the language's rules offline. Prefers the on-disk copy (so editing/adding
/// a module takes effect on pull) and falls back to the embedded copy for a binary far from the
/// checkout. `None` when neither is present/readable.
fn load_committed_module(data_root: &Path, lang: &str) -> Option<LearnedCatalog> {
    let name = format!("{lang}.learned.json");
    let raw = std::fs::read_to_string(committed_modules_dir(data_root).join(&name))
        .ok()
        .or_else(|| EMBEDDED_LINT_MODELS.get_file(&name).and_then(|f| f.contents_utf8().map(str::to_string)))?;
    serde_json::from_str(&raw).ok()
}

/// Persist a learned catalog so the next run loads instead of relearning.
fn save_cache(lang: &str, cat: &LearnedCatalog) {
    if let Some(parent) = cache_path(lang).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cat) {
        let _ = std::fs::write(cache_path(lang), json);
    }
}

/// A stable checksum of a language's resolved rules + toolchain version — the model cache key.
/// Order-independent (rows are sorted) and salted with [`TRAIN_VERSION`].
fn stamp_of(version: &str, rules: &[DocRule]) -> String {
    let mut rows: Vec<String> = rules
        .iter()
        .map(|r| format!("{}\u{1f}{}\u{1f}{}", r.id, r.bad, r.good))
        .collect();
    rows.sort();
    let mut h = Sha256::new();
    h.update(TRAIN_VERSION.as_bytes());
    h.update(version.as_bytes());
    for r in &rows {
        h.update(r.as_bytes());
        h.update([0u8]);
    }
    let mut s = String::from("sha256:");
    for b in h.finalize() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// A model is fresh when both it and a matching stamp file exist on disk.
fn model_fresh(model: &Path, stamp: &Path, want: &str) -> bool {
    model.exists() && std::fs::read_to_string(stamp).map(|s| s.trim() == want).unwrap_or(false)
}

/// Path to a language's model cache stamp (`<lang>.patterns.stamp`, beside its model).
fn stamp_path(lang: &str) -> PathBuf {
    model_dir().join(format!("{lang}.patterns.stamp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A self-contained data root with a committed seed catalog and a tiny principles doc — proves
    /// both knowledge sources flow into a trained, cached, reloadable model. `tag` keeps the path
    /// unique per test so parallel tests never share (and clobber) one directory.
    fn fixture_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lint_train_{}_{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lint-index")).unwrap();
        std::fs::create_dir_all(dir.join("corpus")).unwrap();
        let catalog = serde_json::json!({
            "tool": "clippy", "language": "rust", "docsVersion": "1.0.0", "source": "test",
            "checksum": "sha256:0", "ruleCount": 1,
            "rules": [{
                "id": "bool_comparison", "category": "style", "severity": "low",
                "description": "Comparing a bool to true is redundant.",
                "exampleBad": "fn f(x: bool) { if x == true {} }",
                "exampleGood": "fn f(x: bool) { if x {} }",
                "source": "test#bool_comparison"
            }]
        });
        std::fs::write(dir.join("lint-index/clippy.json"), catalog.to_string()).unwrap();
        std::fs::write(dir.join("lint-index/sources.json"), "{\"sources\":[]}").unwrap();
        std::fs::write(
            dir.join("corpus/cs-principles.md"),
            "# Off by one indexing [high]\n```rust:bad\nfn s(xs: &[i32]) -> i32 { let mut t = 0; for i in 0..=xs.len() { t += xs[i]; } t }\n```\n```rust:good\nfn s(xs: &[i32]) -> i32 { xs.iter().sum() }\n```\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn seed_and_folder_rules_both_resolve_for_rust() {
        let root = fixture_root("seed");
        let (seed, _version) = seed_with_version(&root, "rust");
        assert!(seed.iter().any(|r| r.id == "bool_comparison"), "the seed (docs-link) rule resolves");
        let folder = corpus_rules(&root);
        assert!(folder.iter().any(|(l, r)| l == "rust" && r.id == "off_by_one_indexing"), "the folder rule resolves");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn trains_caches_and_reuses_from_seed_offline() {
        let root = fixture_root("train");
        let models = root.join("models");
        std::env::set_var("HELPERS_LINT_MODELS", &models);
        // Deterministic + offline: force the seed path (no live crawl) so the fixture's own rule
        // is what trains, independent of the machine's toolchain version or network.
        std::env::set_var("HELPERS_LINT_OFFLINE", "1");
        std::env::remove_var("HELPERS_LINT_REFRESH");

        let first = ensure_models(&["rust".to_string()], &root, &root);
        assert!(
            first.trained.iter().any(|s| s.starts_with("rust")),
            "first run trains from the resolved rules: {first:?}"
        );
        assert!(patterns_path("rust").exists(), "model cached to disk");

        let second = ensure_models(&["rust".to_string()], &root, &root);
        assert!(second.reused.contains(&"rust".to_string()), "second run reuses the fresh cache: {second:?}");

        let model = load_patterns("rust").expect("reload");
        let flags = |code: &str| -> Vec<String> { model.flag(code).into_iter().map(|f| f.rule).collect() };
        assert!(flags("fn g(y: bool) { if y == true {} }").iter().any(|r| r == "bool_comparison"), "flags the doc rule");
        assert!(flags("fn g(y: bool) { if y {} }").is_empty(), "the fix stays clean");

        std::env::remove_var("HELPERS_LINT_MODELS");
        std::env::remove_var("HELPERS_LINT_OFFLINE");
        let _ = std::fs::remove_dir_all(&root);
    }
}
