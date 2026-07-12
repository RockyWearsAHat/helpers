//! `lint` — the AI code reviewer.
//!
//! Cross-module theory, evidence hierarchy, and the failure ledger live in `LINTER.md` at the
//! repo root — the single authoritative doc; update it BEFORE changing semantics here.
//!
//! Training: [`crate::lint_train::ensure_models`] compiles law from the project's rule files
//! (`.helpers/lint-rules/`, root `lintPref`), the curated catalog, and official docs — teaching
//! prose is read, never enforced — and returns, per language, a
//! [`crate::lint_train::LangModel`] — the [`crate::lint_match::RuleSet`] firing engine, the
//! [`crate::lint_ai::ConceptModel`] confirmation gate. One call, checksum-cached.
//!
//! Analysis, per file, per language:
//!   1. **Rule firing** — `RuleSet::flag` matches each documented rule's lossless AST pattern
//!      (or, for grammarless languages, its discriminating token regex) against the file: a
//!      finding is the rule's structure occurring, on the construct's line.
//!   2. **Restatement guard** — an imprecise finding whose line shares half the rule's own
//!      description words is quoting the law, not breaking it ([`restates_rule`]) — dropped
//!      before gating, trusted project law included.
//!   3. **Confirmation gate** — precise AST matches report directly; imprecise text-fallback
//!      matches (grammarless languages, description-derived regexes) pass through
//!      `ConceptModel::confirms`, which bundles the matched construct's tokens and keeps the
//!      finding only when the fired rule is the concept it is closest to — so a regex that hit a
//!      token belonging to a different rule is dropped, with no hand-kept word list.
//!   4. **Self-validation** — doc rules that fire like scrape noise (>1% of all scanned lines, or
//!      concentrated in one file) are quarantined and reported, never shown as findings.
//!
//! Documentation formats (md/txt) are reading material, not code: they are linted only by rules
//! written FOR them — `any`-language law governs code languages
//! ([`crate::lint_train::is_document_language`]).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::git::workspace_root;
use crate::index::walk::walk_repo;
use crate::lint_train;
use crate::proto::{text, ToolResult};
use crate::lint_train::resolve_language;

/// Per-project linter preferences loaded from `.helpers/lint.json`.
///
/// Agents and users write this file (via `lint_config`) to tailor which rules fire,
/// what languages are reviewed, and how severe each finding is reported.
#[derive(Default, Clone, serde::Deserialize)]
pub struct LintConfig {
    /// Rule ids to suppress entirely — they will never appear in lint output.
    #[serde(default)]
    pub ignore_rules: Vec<String>,
    /// Override severity for specific rules: `{"rule-id": "high"|"medium"|"low"}`.
    #[serde(default)]
    pub severity_overrides: HashMap<String, String>,
    /// When set, only these languages are reviewed (in addition to any `--lang` CLI flag).
    #[serde(default)]
    pub languages: Option<Vec<String>>,
    /// The HUMAN language findings are rendered in (and, ahead, foreign docs are read in) — the I/O
    /// overlay of LINTER.md, "The human-language I/O overlay". Default `"english"` (empty ⇒ same):
    /// output is byte-for-byte unchanged. `"fr"`/`"french"` renders findings through the French
    /// concept lexicon. `HELPERS_LINT_LANG` overrides this for a one-off run. Data/config only — no
    /// translation lives in code.
    #[serde(default)]
    pub io_language: String,
}

/// Load `.helpers/lint.json` from the project root, returning defaults on any read/parse error.
pub fn load_config(project_root: &Path) -> LintConfig {
    let path = project_root.join(".helpers/lint.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// The project root to review, from the `root` arg or the resolved workspace.
fn root_arg(args: &Value) -> PathBuf {
    match args.get("root").and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => workspace_root(),
    }
}

/// Optional language filter from the `modules` arg. Absent / empty / `all` ⇒ every language.
///
/// Extension-like aliases ("ts", "py") resolve through the learned extension claims
/// (`lint_train::resolve_language`) to their canonical name.
/// Canonical names ("typescript", "python") and unknown names pass through unchanged — an
/// unknown language produces no files in the output rather than being silently discarded,
/// which surfaces the typo instead of hiding it.
fn parse_lang_filter(args: &Value) -> Option<BTreeSet<String>> {
    let arr = args.get("modules").and_then(Value::as_array)?;
    let mut set = BTreeSet::new();
    for tok in arr.iter().filter_map(Value::as_str) {
        let s = tok.trim().to_ascii_lowercase();
        match s.as_str() {
            "all" | "" => return None,
            other => { set.insert(resolve_language(other)); }
        }
    }
    if set.is_empty() { None } else { Some(set) }
}

/// One reported violation in a file.
struct Hit {
    /// 1-based source line.
    line: usize,
    /// The rule id the model attributed.
    rule: String,
    /// Severity bucket (`high`/`medium`/`low`).
    severity: String,
    /// English advice — the rule's description from its source.
    advice: String,
    /// Where the rule came from (doc URL or rule-file path) — every finding cites its origin,
    /// so "did this come from documentation or from our own law?" is answered in the output.
    source: String,
}

/// A file's place in the review.
struct FileReport {
    /// Repo-relative path.
    path: String,
    /// Findings in this file.
    hits: Vec<Hit>,
}

/// Extract the alphanumeric/underscore tokens of one 1-based source line — the "construct" the Hv
/// gate confirms a text-fallback finding against. Out-of-range lines yield no tokens (gate abstains).
fn line_tokens(src: &str, line: usize) -> Vec<String> {
    let Some(text) = src.lines().nth(line.saturating_sub(1)) else { return Vec::new() };
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|t| t.len() >= 2)
        .map(str::to_string)
        .collect()
}

/// Whether a matched line RESTATES the rule's own English rather than violating it: it shares at
/// least 3 — and at least half — of the description's distinct word tokens. Documentation that
/// quotes or discusses the law is the model's reading material (English is the comprehension
/// substrate, not a lintable language), so the law must never fire on its own words. Only
/// imprecise text matches need this guard; a precise AST match is parsed code, not prose.
fn restates_rule(line_tokens: &[String], desc: &str) -> bool {
    let words = |s: &str| -> HashSet<String> {
        s.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|t| t.len() >= 3)
            .map(str::to_lowercase)
            .collect()
    };
    let desc_words = words(desc);
    if desc_words.is_empty() {
        return false;
    }
    let line_words: HashSet<String> =
        line_tokens.iter().filter(|t| t.len() >= 3).map(|t| t.to_lowercase()).collect();
    let shared = desc_words.intersection(&line_words).count();
    shared >= 3 && shared * 2 >= desc_words.len()
}

// ── The verdict replay cache (LINTER.md, "Warm runs replay per-file verdicts") ──────────────

/// One file's cached verdict: the `(mtime, len)` state it was computed under, its content
/// seed and line count (grounding-fingerprint and quarantine inputs — reusable without a
/// read), the merged-model identity, and the post-gate findings `(rule, line, doc_rule)`.
/// A file whose language has no model caches with `model_id = 0` and no findings, so its
/// seed still replays; the first model for that language misses the id and re-lints.
#[derive(Clone)]
struct CachedVerdict {
    state: u128,
    seed: u64,
    lines: u64,
    model_id: u64,
    findings: Vec<(String, u64, bool)>,
}

/// The per-project verdict store — an `HLM1` container beside the models, keyed by the
/// project root's path hash. Machine-global, never in the repo, always safe to delete.
fn verdict_cache_path(root: &Path) -> PathBuf {
    lint_train::model_dir_pub()
        .join("lint-verdicts")
        .join(format!("{:016x}.bin", crate::lint_ai::token_seed(&root.display().to_string())))
}

/// Load the project's verdicts; a container from another `TRAIN_VERSION` (its stamp) is
/// empty — verdicts are engine products and expire with the engine's reading logic.
fn load_verdicts(root: &Path) -> BTreeMap<String, CachedVerdict> {
    let Ok(bytes) = std::fs::read(verdict_cache_path(root)) else { return BTreeMap::new() };
    let Some((stamp, mut d)) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::VERDICT)
    else {
        return BTreeMap::new();
    };
    if stamp != lint_train::train_version() {
        return BTreeMap::new();
    }
    // Rule ids repeat across files, so they live once in a table and findings index into it.
    let decode = |d: &mut crate::lint_codec::Dec| -> Option<BTreeMap<String, CachedVerdict>> {
        let table_len = d.u()? as usize;
        let mut table: Vec<String> = Vec::with_capacity(table_len.min(1 << 16));
        for _ in 0..table_len {
            table.push(d.str()?);
        }
        let files = d.u()? as usize;
        let mut out = BTreeMap::new();
        for _ in 0..files {
            let rel = d.str()?;
            let state = (u128::from(d.fixed_u64()?) << 64) | u128::from(d.fixed_u64()?);
            let seed = d.fixed_u64()?;
            let lines = d.u()?;
            let model_id = d.fixed_u64()?;
            let n = d.u()? as usize;
            let mut findings = Vec::with_capacity(n.min(1 << 12));
            for _ in 0..n {
                let rule = table.get(d.u()? as usize)?.clone();
                findings.push((rule, d.u()?, d.boolean()?));
            }
            out.insert(rel, CachedVerdict { state, seed, lines, model_id, findings });
        }
        Some(out)
    };
    decode(&mut d).unwrap_or_default()
}

/// Persist the project's verdicts (rule table + per-file entries).
fn save_verdicts(root: &Path, verdicts: &BTreeMap<String, CachedVerdict>) {
    let mut table: Vec<String> = Vec::new();
    let mut index: HashMap<&str, u64> = HashMap::new();
    for v in verdicts.values() {
        for (rule, _, _) in &v.findings {
            if !index.contains_key(rule.as_str()) {
                index.insert(rule.as_str(), table.len() as u64);
                table.push(rule.clone());
            }
        }
    }
    let mut e = crate::lint_codec::Enc::new();
    e.u(table.len() as u64);
    for rule in &table {
        e.str(rule);
    }
    e.u(verdicts.len() as u64);
    for (rel, v) in verdicts {
        e.str(rel);
        e.fixed_u64((v.state >> 64) as u64);
        e.fixed_u64(v.state as u64);
        e.fixed_u64(v.seed);
        e.u(v.lines);
        e.fixed_u64(v.model_id);
        e.u(v.findings.len() as u64);
        for (rule, line, doc) in &v.findings {
            e.u(index[rule.as_str()]);
            e.u(*line);
            e.boolean(*doc);
        }
    }
    let path = verdict_cache_path(root);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, e.finish(crate::lint_codec::kind::VERDICT, lint_train::train_version()));
}

/// A walked, selected file: its language, repo-relative path, absolute path, and `(mtime,
/// len)` state — everything the replay decision needs without reading the file.
#[derive(Clone)]
struct FileMeta {
    rel: String,
    abs: PathBuf,
    state: u128,
}

/// The lazily-read project code behind [`lint_train::ProjectSource`]: fingerprints reuse
/// cached content seeds; full sources read from disk only when an overlay recompiles.
struct LazyProject<'a> {
    by_language: &'a BTreeMap<String, Vec<FileMeta>>,
    /// Complete after wave 1 (every selected file has a seed — cached or freshly read), so
    /// fingerprints are pure lock-free reads across the parallel training threads.
    seeds: &'a HashMap<String, u64>,
    contents: &'a std::sync::Mutex<HashMap<String, std::sync::Arc<String>>>,
}

impl LazyProject<'_> {
    /// The file's content, from the run's store or freshly read (memoized). `None` = unreadable.
    fn content(&self, meta: &FileMeta) -> Option<std::sync::Arc<String>> {
        if let Some(src) = self.contents.lock().unwrap_or_else(|e| e.into_inner()).get(&meta.rel) {
            return Some(src.clone());
        }
        let src = std::sync::Arc::new(std::fs::read_to_string(&meta.abs).ok()?);
        self.contents
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(meta.rel.clone(), src.clone());
        Some(src)
    }

    /// The file's content seed — wave 1 seeded every selected file, so this is a plain read.
    fn seed(&self, meta: &FileMeta) -> u64 {
        self.seeds.get(&meta.rel).copied().unwrap_or(0)
    }
}

impl lint_train::ProjectSource for LazyProject<'_> {
    fn fingerprint(&self, lang: &str) -> u64 {
        let Some(metas) = self.by_language.get(lang) else { return 0 };
        metas.iter().map(|m| self.seed(m)).fold(0u64, |acc, h| acc ^ h)
    }
    fn sources(&self, lang: &str) -> Vec<(String, String)> {
        let Some(metas) = self.by_language.get(lang) else { return Vec::new() };
        metas
            .iter()
            .filter_map(|m| self.content(m).map(|src| (m.rel.clone(), src.as_str().to_string())))
            .collect()
    }
}

/// Review the whole project: detect its languages, build one `LangModel` per language (rule set +
/// concept gate + principles), match each file with the rule set, confirm text-fallback findings
/// through the concept gate, and report the result in English.
pub fn run(args: &Value) -> ToolResult {
    let t0 = std::time::Instant::now();
    let mut stages: Vec<(&'static str, u128)> = Vec::new();
    let mut mark = {
        let mut last = std::time::Instant::now();
        move |stages: &mut Vec<(&'static str, u128)>, name: &'static str| {
            stages.push((name, last.elapsed().as_micros()));
            last = std::time::Instant::now();
        }
    };
    let t_sub = std::time::Instant::now();
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("lint: path not found: {}", root.display()));
    }
    let max = args.get("max").and_then(Value::as_u64).unwrap_or(80).clamp(1, 500) as usize;
    let filter = parse_lang_filter(args);
    let data = data_root();
    let memo_key = format!("max={max}|langs={filter:?}");

    // The kqueue tier (LINTER.md): when the kernel reports every watched input quiet since
    // the memo was committed, the stored body IS the answer — one kevent drain, no walk,
    // no stat, microseconds. Any doubt falls through to the stat tier below.
    if std::env::var_os("HELPERS_LINT_REFRESH").is_none() {
        if let Some(body) = crate::lint_kq::replay(&root, &memo_key) {
            if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
                let us = t0.elapsed().as_micros();
                eprintln!("[lint-kq] whole-project replay in {us}µs");
                return Ok(vec![text(format!(
                    "{body}\nTiming: kqueue whole-project replay — total {us}µs\n"
                ))]);
            }
            return Ok(vec![text(body)]);
        }
    }

    // The INCREMENTAL tier: the kernel names what fired; lint exactly that over the
    // daemon's cached state (LINTER.md, "The incremental tier").
    if std::env::var_os("HELPERS_LINT_REFRESH").is_none() {
        if let Some(body) = incremental_run(&root, &data, max, &filter, &memo_key) {
            if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
                let us = t0.elapsed().as_micros();
                return Ok(vec![text(format!(
                    "{body}\nTiming: incremental lint — total {us}µs\n"
                ))]);
            }
            return Ok(vec![text(body)]);
        }
    }

    // 1) Walk the project and partition by language: those with a tree-sitter grammar are analyzed
    //    with the AST engine; the rest are still analyzed via the token-regex fallback, so nothing
    //    is dropped for lacking a grammar.
    let t_walk = std::time::Instant::now();
    let (files, walked_dirs) = crate::index::walk::walk_repo_full(&root);
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-walk] walk_repo {:.1}ms, {} files", t_walk.elapsed().as_secs_f64() * 1e3, files.len());
    }
    // Whole-project replay (LINTER.md, "An unchanged project replays the whole report"):
    // the walk just verified every file's state by statting — kernel-synchronous, so an
    // edit made the instant before this call is already in the fold — and the auxiliary
    // fold covers every input outside the tree. Equal witness ⇒ the stored body IS this
    // run's body: no models, no verdicts, no selection. A `HELPERS_LINT_REFRESH` run must
    // re-read the world, so it never replays (it still stores).
    let walk_fold = crate::lint_replay::walk_witness(&files);
    let witness = crate::lint_replay::combine(walk_fold, crate::lint_replay::aux_witness(&root, &data));
    if std::env::var_os("HELPERS_LINT_REFRESH").is_none() {
        if let Some(body) = crate::lint_replay::replay(&root, &memo_key, witness) {
            // A stat-tier hit still arms the kqueue tier, so the NEXT call takes the
            // microsecond path — this is how a fresh daemon converges.
            kq_arm_and_commit(&root, &data, &files, &walked_dirs, &witness, walk_fold.fold, &memo_key, &body);
            if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
                let us = t0.elapsed().as_micros();
                eprintln!("[lint-replay] whole-project replay in {us}µs");
                return Ok(vec![text(format!(
                    "{body}\nTiming: whole-project replay — total {us}µs\n"
                ))]);
            }
            return Ok(vec![text(body)]);
        }
    }
    // Load per-project config, then merge in feedback-driven auto-suppressions: rules a developer
    // has flagged as false positives enough times (see `crate::lint_feedback`) are folded into the
    // same `ignore_rules` list, so they are suppressed through the normal config path below. Loaded
    // AFTER the replay check — both files' states are already in the witness, so a replayed run
    // never needs their contents.
    let mut config = load_config(&root);
    let auto_suppressed = crate::lint_feedback::merge_auto_suppressed(&mut config, &root);
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-walk] config+feedback {:.1}ms", t_sub.elapsed().as_secs_f64() * 1e3);
    }
    let ignore_set: HashSet<String> = config.ignore_rules.iter().cloned().collect();

    // The project's own rule documents (root lintPref, .helpers/lint-rules) are instructions TO
    // the linter, not source to be linted — never analyze the law as if it were code.
    let t_law = std::time::Instant::now();
    let law: HashSet<PathBuf> = lint_train::rule_documents(&root).into_iter().map(|(p, _)| p).collect();
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-walk] law set {:.1}ms", t_law.elapsed().as_secs_f64() * 1e3);
    }
    // Selection is shared with the incremental tier (`select_by_language`); contents are
    // read in waves — changed files now (their seeds feed the grounding fingerprint),
    // model-invalidated files after the models load.
    let by_language = select_by_language(&files, &law, &config, &filter);
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-walk] select+group {:.1}ms", t_law.elapsed().as_secs_f64() * 1e3);
    }
    use rayon::prelude::*;
    let t_v = std::time::Instant::now();
    let verdicts = load_verdicts(&root);
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-walk] load_verdicts {:.1}ms ({} entries)", t_v.elapsed().as_secs_f64() * 1e3, verdicts.len());
    }
    let mut seeds: HashMap<String, u64> = by_language
        .values()
        .flatten()
        .filter_map(|m| {
            let v = verdicts.get(&m.rel)?;
            (v.state == m.state).then(|| (m.rel.clone(), v.seed))
        })
        .collect();
    let contents: std::sync::Mutex<HashMap<String, std::sync::Arc<String>>> =
        std::sync::Mutex::new(HashMap::new());
    {
        // Wave 1: every state-changed (or never-seen) file — read in parallel, seed for the
        // grounding fingerprints below. After this block every selected file has a seed.
        let changed: Vec<&FileMeta> =
            by_language.values().flatten().filter(|m| !seeds.contains_key(&m.rel)).collect();
        let read: Vec<(String, Option<String>)> = changed
            .par_iter()
            .map(|m| (m.rel.clone(), std::fs::read_to_string(&m.abs).ok()))
            .collect();
        if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
            eprintln!("[lint-walk] wave1 read {} changed file(s)", read.len());
        }
        let mut store = contents.lock().unwrap_or_else(|e| e.into_inner());
        for (rel, body) in read {
            if let Some(src) = body {
                seeds.insert(rel.clone(), crate::lint_ai::token_seed(&src));
                store.insert(rel, std::sync::Arc::new(src));
            } else {
                seeds.insert(rel, 0);
            }
        }
    }
    let project = LazyProject { by_language: &by_language, seeds: &seeds, contents: &contents };
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-walk] to-project total {:.1}ms", t_sub.elapsed().as_secs_f64() * 1e3);
    }

    // 2) Train / load one model per detected language (checksum-cached). The project reaches
    //    training as a LAZY source: fingerprints from seeds, full grounding text on demand.
    let langs: Vec<String> = by_language.keys().cloned().collect();
    mark(&mut stages, "walk+read");
    let (report, models) = lint_train::ensure_models(&langs, &data, &root, &project);
    let models = std::sync::Arc::new(models);
    mark(&mut stages, "train/load");

    let mut sources: Vec<String> = Vec::new();
    if !models.is_empty() {
        let total: usize = models.values().map(|m| m.rules.rule_count()).sum();
        sources.push(format!("{total} rules across {} language(s)", models.len()));
    }
    // Name the languages whose rules were (re)learned from the live official docs this run, so the
    // report shows the reading-and-testing path actually ran — not just that rules exist.
    if !report.crawled.is_empty() {
        sources.push(format!("learned live from official docs: {}", report.crawled.join(", ")));
    }
    if !report.pulled.is_empty() {
        sources.push(format!("downloaded from the model registry: {}", report.pulled.join(", ")));
    }

    // Run-level footers that depend only on the TRAINING report and the law listing —
    // pre-rendered so the incremental tier can reuse them verbatim (its inputs are
    // provably unchanged when no aux event fired).
    let mut run_footer = String::new();
    run_footer.push_str(&render_unenforced(&report.unenforced));
    let inert: Vec<String> = lint_train::rule_documents(&root)
        .into_iter()
        .filter(|(_, lang)| lang != "any" && !by_language.contains_key(lang.as_str()))
        .map(|(p, lang)| {
            let rel = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
            format!("{rel} (governs '{lang}')")
        })
        .collect();
    if !inert.is_empty() {
        run_footer.push_str(&format!(
            "\nInert law file(s) — the language they govern matches no analyzed file, so their \
             rules did not run: {}.\n",
            inert.join(", ")
        ));
    }
    let unlearned: std::collections::BTreeSet<&str> = report
        .unlearned
        .iter()
        .map(String::as_str)
        .filter(|l| !crate::lint_match::prose_lang(l))
        .collect();
    if !unlearned.is_empty() {
        let names = unlearned.into_iter().collect::<Vec<_>>().join(", ");
        run_footer.push_str(&format!(
            "\nNot yet set up (project law still enforced there): {names}. Everything else \
             was fully checked. Hand me each unknown language's official documentation \
             (`lint_config action=add_source lang=<language> url=<docs URL>`), then run \
             `lint_config action=train` — setup needs internet, linting never does.\n"
        ));
    }
    if !report.outdated.is_empty() {
        let mut outdated = report.outdated.clone();
        outdated.sort();
        outdated.dedup();
        let names = outdated.join(", ");
        if lint_train::heal_outdated_modules(&outdated, &data, std::time::Duration::from_millis(900)) {
            run_footer.push_str(&format!(
                "\nOut of date at the start of this run (still enforced with the last known \
                 rules): {names}. Current modules were fetched during the run — the next \
                 lint is fully current.\n"
            ));
        } else {
            run_footer.push_str(&format!(
                "\nValidation not completed — results may be out of date for: {names}. The \
                 last known rules were still enforced. Please check your connection and \
                 connect to the internet soon (or run `lint_config action=train`) to ensure \
                 up-to-date linting of all languages and validate the latest rules.\n"
            ));
        }
    }
    // Item 3c — contradiction-driven reshape: a proven rule whose source page was re-read this run and
    // failed to re-prove is DROPPED, never silently kept. Name each one so judgment learning is visible.
    if !report.contradicted.is_empty() {
        let mut names: Vec<String> =
            report.contradicted.iter().map(|(lang, what)| format!("{lang}: {what}")).collect();
        names.sort();
        names.dedup();
        run_footer.push_str(&format!(
            "\nReshaped this run — a previously-proven rule was re-read from its own docs and no longer \
             re-proves, so it was dropped: {}.\n",
            names.join("; ")
        ));
    }
    let feedback_footer = render_feedback(&root, &auto_suppressed);
    let (mut body, fresh_updates, _quarantined, law_watch_block) = fire_shape_render(
        &root,
        max,
        &by_language,
        &verdicts,
        &models,
        contents,
        &seeds,
        &config,
        &ignore_set,
        &sources,
        &run_footer,
        &feedback_footer,
    );
    // Persist the replay cache: prune files gone from the walk, fold in fresh verdicts, write
    // only when something actually changed.
    {
        let walked: HashSet<&str> =
            by_language.values().flatten().map(|m| m.rel.as_str()).collect();
        let mut next = verdicts;
        let before = next.len();
        next.retain(|rel, _| walked.contains(rel.as_str()));
        let dirty = next.len() != before || !fresh_updates.is_empty();
        for (rel, v) in fresh_updates {
            next.insert(rel, v);
        }
        if dirty {
            save_verdicts(&root, &next);
        }
        // Feed the INCREMENTAL tier: with these caches, the next fired-set run touches
        // only the change (LINTER.md, "The incremental tier").
        let aux_now = crate::lint_replay::aux_witness(&root, &data);
        daemon_state().lock().unwrap_or_else(|e| e.into_inner()).insert(
            root.clone(),
            DaemonState {
                files: files.clone(),
                dirs: walked_dirs.clone(),
                verdicts: next,
                models: models.clone(),
                config: config.clone(),
                ignore_set: ignore_set.clone(),
                sources: sources.clone(),
                run_footer: run_footer.clone(),
                feedback_footer: feedback_footer.clone(),
                law_abs: law.clone(),
                walk_w: crate::lint_replay::walk_witness(&files),
                aux_w: aux_now,
                bodies: HashMap::from([(memo_key.clone(), body.clone())]),
                rel_lang: by_language
                    .iter()
                    .flat_map(|(l, ms)| ms.iter().map(move |m| (m.rel.clone(), l.clone())))
                    .collect(),
                info: models
                    .iter()
                    .map(|(lang, m)| {
                        let per: HashMap<String, (String, String, String)> = m
                            .rules
                            .rule_ids()
                            .filter_map(|id| {
                                m.rules.info_of(id).map(|(sev, desc, src)| {
                                    (id.to_string(), (sev.to_string(), desc.to_string(), src.to_string()))
                                })
                            })
                            .collect();
                        (lang.clone(), per)
                    })
                    .collect(),
                trusted: by_language
                    .keys()
                    .map(|lang| (lang.clone(), lint_train::project_rule_ids(&root, lang)))
                    .collect(),
                law_watch_block,
            },
        );
        spawn_eager_pump(root.clone());
    }
    mark(&mut stages, "match+gate");
    // Store the finished body for the whole-project replay. The walk fold is the PRE-run
    // one (a file edited mid-run differs from it next time — a conservative miss); the
    // auxiliary fold is recomputed because the run's own training writes land in the model
    // dir. The trace footer below is per-run and appended after, so a replay never shows a
    // stale timing line.
    let final_witness =
        crate::lint_replay::combine(walk_fold, crate::lint_replay::aux_witness(&root, &data));
    crate::lint_replay::store(&root, &memo_key, final_witness, &body);
    kq_arm_and_commit(&root, &data, &files, &walked_dirs, &final_witness, walk_fold.fold, &memo_key, &body);
    // Honest latency accounting on demand: where a run's time actually went, stage by stage.
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        mark(&mut stages, "render");
        let parts: Vec<String> =
            stages.iter().map(|(n, us)| format!("{n} {:.1}ms", *us as f64 / 1000.0)).collect();
        body.push_str(&format!(
            "\nTiming: {} — total {:.1}ms\n",
            parts.join(", "),
            t0.elapsed().as_micros() as f64 / 1000.0
        ));
    }
    Ok(vec![text(body)])
}

/// Report project-authored rules no detector could be compiled for. The user's law must never
/// vanish silently — this is the difference between integrating an AI (it says what it cannot do
/// yet and why) and a compiler dropping input on the floor.
fn render_unenforced(unenforced: &[(String, String)]) -> String {
    if unenforced.is_empty() {
        return String::new();
    }
    let ids: Vec<String> = unenforced.iter().map(|(lang, id)| format!("{lang}/{id}")).collect();
    format!(
        "\nProject law not yet enforceable ({}): {}.\nThe rule compiled no detector — its words are \
         all ordinary English to the reader and its examples (if any) do not differ at token level. \
         Name the construct in the rule's sentence, give a bad/good example pair that differs in \
         tokens, or run once online so the language's grammar and docs can be learned.\n",
        ids.len(),
        ids.join(", ")
    )
}

/// Report doc-learned rules the self-validation pass quarantined: their fire rate against the real
/// project marks the learned pattern as noise (a badly scraped docs page), not a real convention.
/// Empty when nothing was quarantined.
fn render_quarantine(quarantined: &std::collections::BTreeSet<String>) -> String {
    if quarantined.is_empty() {
        return String::new();
    }
    let ids: Vec<&str> = quarantined.iter().map(String::as_str).collect();
    format!(
        "\nQuarantined {} mislearned rule(s) (fired on >1% of all scanned lines, or >10% of one \
         file — noisy docs scrape, not real findings): {}.\nRe-learn the language (delete its cached model; the next lint retrains) \
         to refresh them.\n",
        ids.len(),
        ids.join(", ")
    )
}

/// Render the feedback footer: rules auto-suppressed from false-positive flags, and missed findings
/// still pending formalization. Empty string when there is no feedback, so clean projects stay quiet.
fn render_feedback(root: &Path, auto_suppressed: &BTreeSet<String>) -> String {
    let mut s = String::new();
    if !auto_suppressed.is_empty() {
        let rules: Vec<&str> = auto_suppressed.iter().map(String::as_str).collect();
        s.push_str(&format!(
            "\nAuto-suppressed from your feedback ({}+ false-positive flags each): {}.\n\
             Re-enable with `lint_config action=unignore rule=<id>`.\n",
            crate::lint_feedback::SUPPRESS_THRESHOLD,
            rules.join(", ")
        ));
    }
    let records = crate::lint_feedback::read_all(root);
    let pending = crate::lint_feedback::pending_missed(&records);
    if !pending.is_empty() {
        s.push_str(&format!("\nPending rules ({} missed finding(s) you flagged — formalize with `lint_rule`):\n", pending.len()));
        for r in pending {
            let loc = r.line.map(|l| format!("{}:{}", r.file, l)).unwrap_or_else(|| r.file.clone());
            let desc = r.description.as_deref().unwrap_or("(no description)");
            let draft = if r.bad.is_some() && r.language.is_some() { " [draft seeded]" } else { "" };
            s.push_str(&format!("  - {loc}: {desc}{draft}\n"));
        }
    }
    s
}

// ── English report ────────────────────────────────────────────────────────────

/// Severity ordering for display: high first.
fn severity_rank(sev: &str) -> u8 {
    match sev {
        "high" => 0,
        "low" => 2,
        _ => 1,
    }
}

/// Condense a rule's advice to the ONE enforceable sentence for display. A canon principle's
/// description is its whole Markdown section — a heading line joined to several body sentences —
/// which is training input for understanding, not a finding message. Dumping all of it made a
/// single DRY hit render as six lines of rubric. This drops leading heading lines (a `#`/`N.`/
/// bullet-marked line) and returns the first real body sentence, capped — the concise statement of
/// the violation. Prose that is already a single short sentence passes through unchanged.
fn condense_advice(advice: &str) -> String {
    let body = advice
        .lines()
        .map(str::trim)
        .find(|l| {
            !l.is_empty()
                && !l.starts_with('#')
                && !l.starts_with(['-', '*'])
                // A "12. DRY — …" heading line: digits then a dot. A real sentence never opens so.
                && !l.split_whitespace().next().is_some_and(|w| w.trim_end_matches('.').chars().all(|c| c.is_ascii_digit()) && w.ends_with('.'))
        })
        .unwrap_or(advice.trim());
    // First sentence: up to a terminal '.', '!' or '?' followed by space/end — not a decimal point.
    let bytes = body.as_bytes();
    let mut end = body.len();
    for (i, &b) in bytes.iter().enumerate() {
        if matches!(b, b'.' | b'!' | b'?')
            && bytes.get(i + 1).is_none_or(|n| n.is_ascii_whitespace())
            && !bytes.get(i.wrapping_sub(1)).is_some_and(u8::is_ascii_digit)
        {
            end = i + 1;
            break;
        }
    }
    let first = body[..end].trim();
    // A defensive length cap for a pathological run-on sentence with no early terminator.
    if first.chars().count() > 200 {
        let cut: String = first.chars().take(197).collect();
        format!("{}…", cut.trim_end())
    } else {
        first.to_string()
    }
}

/// Translate human-readable finding/verdict text into the selected I/O language, when one is set —
/// a word-level concept gloss through the bilingual overlay (LINTER.md, "The human-language I/O
/// overlay"). `None` (English default) returns the text unchanged, so English output is identical.
fn tr(lex: Option<&crate::lint_lang::Lexicon>, text: &str) -> String {
    match lex {
        Some(l) => l.render(text),
        None => text.to_string(),
    }
}

/// Collapse a file's hits into readable lines: one per distinct rule, carrying the advice once and
/// the lines it occurred on (capped), highest-severity first. When `lex` is set the severity label
/// and advice render in the selected I/O language; the rule id and source citation stay verbatim.
fn group_hits(hits: &[Hit], lex: Option<&crate::lint_lang::Lexicon>) -> Vec<String> {
    let mut groups: Vec<(String, String, String, String, Vec<usize>)> = Vec::new(); // (rule, sev, advice, source, lines)
    for h in hits {
        let advice = if h.advice.is_empty() { format!("violates `{}`", h.rule) } else { condense_advice(&h.advice) };
        if let Some(g) = groups.iter_mut().find(|g| g.0 == h.rule) {
            g.4.push(h.line);
        } else {
            groups.push((h.rule.clone(), h.severity.clone(), advice, h.source.clone(), vec![h.line]));
        }
    }
    groups.sort_by(|a, b| severity_rank(&a.1).cmp(&severity_rank(&b.1)).then_with(|| b.4.len().cmp(&a.4.len())));
    groups
        .into_iter()
        .map(|(rule, sev, advice, source, mut lines)| {
            lines.sort_unstable();
            let count = lines.len();
            let shown: Vec<String> = lines.iter().take(6).map(usize::to_string).collect();
            let more = if count > 6 { format!(", +{} more", count - 6) } else { String::new() };
            let occ = if count == 1 { format!("L{}", lines[0]) } else { format!("×{count} (lines {}{more})", shown.join(", ")) };
            let cite = if source.is_empty() { String::new() } else { format!("  ⟨{source}⟩") };
            format!("[{}] [{rule}] {}  {occ}{cite}", tr(lex, &sev), tr(lex, &advice))
        })
        .collect()
}

/// Render the review: verdict, per-file findings, what could not be analyzed, training sources.
fn render(
    root: &Path,
    reports: &[FileReport],
    by_language: &BTreeMap<String, usize>,
    unanalyzed: &BTreeMap<String, usize>,
    sources: &[String],
    max: usize,
    lex: Option<&crate::lint_lang::Lexicon>,
) -> String {
    let mut s = String::new();
    let analyzed: usize = by_language.values().sum();
    let langs: Vec<String> = by_language.iter().map(|(l, n)| format!("{l} ({n})")).collect();
    // The header carries a filesystem PATH and language names — glossing it word-by-word would
    // corrupt them (`/private/` → `/privé/`, `rust` → `rouille`), so it stays English by design.
    // Only the human-language FINDINGS and verdict labels below are rendered through the overlay.
    s.push_str(&format!(
        "I read {} and analyzed {analyzed} source file(s): {}.\n\n",
        root.display(),
        if langs.is_empty() { "none".to_string() } else { langs.join(", ") }
    ));

    let total: usize = reports.iter().map(|f| f.hits.len()).sum();
    if total == 0 {
        s.push_str(&format!("{}\n", tr(lex, "Verdict: CLEAN. No violations of the learned rules or the project's law.")));
    } else {
        let (mut hi, mut me, mut lo) = (0usize, 0usize, 0usize);
        for f in reports {
            for h in &f.hits {
                match h.severity.as_str() {
                    "high" => hi += 1,
                    "low" => lo += 1,
                    _ => me += 1,
                }
            }
        }
        s.push_str(&format!(
            "{}\n",
            tr(lex, &format!(
                "Verdict: {total} issue(s) across {} of {analyzed} file(s) — {hi} high, {me} medium, {lo} low.",
                reports.len()
            ))
        ));
        let mut shown = 0usize;
        for f in reports {
            if shown >= max { break; }
            s.push_str(&format!("\n{}\n", f.path));
            for line in group_hits(&f.hits, lex) {
                if shown >= max {
                    s.push_str("  …raise `max` to see more.\n");
                    break;
                }
                s.push_str(&format!("  {line}\n"));
                shown += 1;
            }
        }
    }

    // Language lists and the training-source line carry language names / identifiers, so they stay
    // English (glossing them would translate `rust` → `rouille`); only the findings render in the
    // I/O language.
    if !unanalyzed.is_empty() {
        let u: Vec<String> = unanalyzed.iter().map(|(l, n)| format!("{l} ({n})")).collect();
        s.push_str(&format!("\nLanguages without AST support (not analyzed): {}.\n", u.join(", ")));
    }

    if !sources.is_empty() {
        s.push_str(&format!("\nTrained from: {}.\n", sources.join(", ")));
    }
    // Cite the I/O overlay whenever findings are rendered in a non-English language (LINTER.md,
    // "The human-language I/O overlay"): the reader sees WHICH lexicon translated the output, and
    // that untranslated words are an honest gap in that bilingual dictionary, not a bug.
    if let Some(l) = lex {
        s.push_str(&format!(
            "\n[I/O language: {} — findings rendered word-level through the {}; terms the bilingual dictionary lacks stay English.]\n",
            l.lang(),
            l.source(),
        ));
    }
    s
}



/// The armed daemon's cached inputs for the INCREMENTAL tier (LINTER.md, "The incremental
/// tier"): everything a fired-set run needs without a walk, a container decode, or a
/// model load. Repopulated by every full run; invalidated by falling back (aux events).
struct DaemonState {
    files: Vec<crate::index::walk::WalkedFile>,
    dirs: Vec<PathBuf>,
    verdicts: BTreeMap<String, CachedVerdict>,
    models: std::sync::Arc<HashMap<String, lint_train::LangModel>>,
    config: LintConfig,
    ignore_set: HashSet<String>,
    sources: Vec<String>,
    run_footer: String,
    feedback_footer: String,
    law_abs: HashSet<PathBuf>,
    /// Patched walk witness (order-independent fold — `lint_replay::file_term`).
    walk_w: crate::lint_replay::Witness,
    /// Aux witness cached from the last full run — provably current on the incremental
    /// path (any aux event falls back before reaching it).
    aux_w: crate::lint_replay::Witness,
    /// Rendered bodies per memo key — the zero-change incremental answer.
    bodies: HashMap<String, String>,
    /// rel → language for every SELECTED file (default filter) — analyzed counts and the
    /// quarantine denominators come from this, no re-selection.
    rel_lang: HashMap<String, String>,
    /// lang → rule → (severity, advice, source): O(1) finding rendering (the full run's
    /// `info_of` linear scans charged ~0.5ms per call across a few hundred findings).
    info: HashMap<String, HashMap<String, (String, String, String)>>,
    /// lang → the project's own rule ids (trusted law — never gated).
    trusted: HashMap<String, HashSet<String>>,
    /// The rendered "Your law, as understood" block, cached with the models it reflects.
    law_watch_block: String,
}

/// The EAGER PUMP (LINTER.md, "The incremental tier"): parse during the trace. One thread
/// per root blocks on the kernel event stream; the moment an edit lands it runs the
/// incremental pipeline in the background — so the work happens in the dead time between
/// the edit and the next lint request, and that request lands on the committed memo in
/// microseconds. A structural event (incremental declines) runs the full pipeline
/// instead, so even those are pre-digested. In-flight and spawn guards keep exactly one
/// pump and one run per root; a one-shot process's pump dies with it, harmlessly.
fn spawn_eager_pump(root: PathBuf) {
    static PUMPS: std::sync::OnceLock<std::sync::Mutex<HashSet<PathBuf>>> =
        std::sync::OnceLock::new();
    {
        let mut pumps =
            PUMPS.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
        if !pumps.insert(root.clone()) {
            return; // already pumping this root
        }
    }
    std::thread::spawn(move || {
        let trace = std::env::var_os("HELPERS_LINT_TRACE").is_some();
        loop {
            if !crate::lint_kq::wait_event(&root, 60_000) {
                continue;
            }
            // Coalesce the editor's burst (write+rename pairs, multi-file saves).
            std::thread::sleep(std::time::Duration::from_millis(15));
            let t0 = std::time::Instant::now();
            let data = data_root();
            let key = format!("max={}|langs=None", 80);
            let done = incremental_run(&root, &data, 80, &None, &key).is_some();
            if !done {
                // Structural change — pre-digest with the full pipeline.
                let _ = run(&json!({ "root": root.display().to_string() }));
            }
            if trace {
                eprintln!(
                    "[lint-pump] {} pre-digested in {:.0}µs ({})",
                    root.display(),
                    t0.elapsed().as_micros(),
                    if done { "incremental" } else { "full" }
                );
            }
        }
    });
}

/// Per-root daemon state (long-lived process only; a one-shot `call` populates and exits).
fn daemon_state() -> &'static std::sync::Mutex<HashMap<PathBuf, DaemonState>> {
    static STATE: std::sync::OnceLock<std::sync::Mutex<HashMap<PathBuf, DaemonState>>> =
        std::sync::OnceLock::new();
    STATE.get_or_init(Default::default)
}

/// Group lintable files by language — the ONE selection the full and incremental paths
/// share (law files excluded, extensions resolved once per distinct ext, filter + config
/// languages applied).
fn select_by_language(
    files: &[crate::index::walk::WalkedFile],
    law: &HashSet<PathBuf>,
    config: &LintConfig,
    filter: &Option<BTreeSet<String>>,
) -> BTreeMap<String, Vec<FileMeta>> {
    let mut lang_of_ext: HashMap<&str, String> = HashMap::new();
    let mut by_language: BTreeMap<String, Vec<FileMeta>> = BTreeMap::new();
    for f in files.iter().filter(|f| !law.contains(&f.abs)) {
        let ext = f.ext.as_str();
        if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let l = lang_of_ext.entry(ext).or_insert_with(|| resolve_language(ext));
        if filter.as_ref().is_some_and(|set| !set.contains(l.as_str())) {
            continue;
        }
        if config.languages.as_ref().is_some_and(|set| !set.contains(l)) {
            continue;
        }
        by_language
            .entry(l.clone())
            .or_default()
            .push(FileMeta { rel: f.rel.clone(), abs: f.abs.clone(), state: f.state });
    }
    by_language
}

/// The shared FIRE → SHAPE → RENDER core (LINTER.md, "The live path"): per-file replay or
/// re-lint through the given models, restatement guard, Hv gate, quarantine, config
/// suppression, and the rendered body. Both the full pipeline and the incremental tier
/// call this — the incremental tier with cached inputs, so its cost is the CHANGE plus
/// this function's aggregation, never a walk or a model load.
#[allow(clippy::too_many_arguments)]
fn fire_shape_render(
    root: &Path,
    max: usize,
    by_language: &BTreeMap<String, Vec<FileMeta>>,
    verdicts: &BTreeMap<String, CachedVerdict>,
    models: &HashMap<String, lint_train::LangModel>,
    contents: std::sync::Mutex<HashMap<String, std::sync::Arc<String>>>,
    seeds: &HashMap<String, u64>,
    config: &LintConfig,
    ignore_set: &HashSet<String>,
    sources: &[String],
    run_footer: &str,
    feedback_footer: &str,
) -> (String, Vec<(String, CachedVerdict)>, std::collections::BTreeSet<String>, String) {
    use rayon::prelude::*;
    let mut reports: Vec<FileReport> = Vec::new();
    // What each project law compiled to — rendered so the author SEES the comprehension and can
    // correct a mis-read law by rephrasing, not by debugging missing findings.
    let mut law_watch: BTreeMap<String, String> = BTreeMap::new();
    let push_hit = |reports: &mut Vec<FileReport>, path: &str, hit: Hit| {
        if let Some(r) = reports.iter_mut().find(|r| r.path == path) {
            r.hits.push(hit);
        } else {
            reports.push(FileReport { path: path.to_string(), hits: vec![hit] });
        }
    };

    // 3) Rule firing + confirmation gate. Findings are staged (not reported yet) so the
    //    self-validation pass below can measure each rule's fire rate against reality first.
    //    Wave 2: a file whose state matched but whose language's MODEL changed must re-lint —
    //    read those now so every language pass below has its fresh files' contents in hand.
    {
        let verdicts = &verdicts;
        let need: Vec<&FileMeta> = by_language
            .iter()
            .flat_map(|(lang, metas)| {
                let model_id = models.get(lang).map(|m| m.id).unwrap_or(0);
                metas.iter().filter(move |m| {
                    !matches!(verdicts.get(&m.rel), Some(v) if v.state == m.state && v.model_id == model_id)
                })
            })
            .collect();
        let store = contents.lock().unwrap_or_else(|e| e.into_inner());
        let missing: Vec<&FileMeta> =
            need.into_iter().filter(|m| !store.contains_key(&m.rel)).collect();
        drop(store);
        let read: Vec<(String, Option<String>)> = missing
            .par_iter()
            .map(|m| (m.rel.clone(), std::fs::read_to_string(&m.abs).ok()))
            .collect();
        let mut store = contents.lock().unwrap_or_else(|e| e.into_inner());
        for (rel, body) in read {
            if let Some(src) = body {
                store.insert(rel, std::sync::Arc::new(src));
            }
        }
    }
    let contents = contents.into_inner().unwrap_or_else(|e| e.into_inner());
    // Every fire-rate denominator derives from one line count per file — replayed from the
    // verdict cache for unchanged files, counted from the fresh read otherwise.
    let file_lines: HashMap<&str, usize> = by_language
        .values()
        .flatten()
        .map(|m| {
            let lines = match contents.get(&m.rel) {
                Some(src) => src.lines().count(),
                None => verdicts.get(&m.rel).map(|v| v.lines as usize).unwrap_or(0),
            };
            (m.rel.as_str(), lines)
        })
        .collect();
    let total_lines: usize = file_lines.values().sum();
    // Languages fire in PARALLEL — each language's pass (trusted-law lookup, matching, concept
    // gate) is independent, so the stage costs the slowest language, not the sum. Results fold
    // back in language order, and each language's findings stay in file order, so the report
    // is deterministic. Within a language, files also match in parallel (rayon nests fine).
    struct LangPass {
        staged: Vec<(String, Hit, bool)>, // (path, hit, unverified doc rule?)
        law_watch: Vec<(String, String)>,
        trace: String,
        /// Fresh files' new verdict-cache entries (LINTER.md, "Warm runs replay per-file verdicts").
        updates: Vec<(String, CachedVerdict)>,
    }
    let passes: Vec<LangPass> = {
        use rayon::prelude::*;
        let entries: Vec<(&String, &Vec<FileMeta>)> = by_language.iter().collect();
        entries
            .par_iter()
            .map(|(lang, metas)| {
                let t0 = std::time::Instant::now();
                let model = models.get(*lang);
                let model_id = model.map(|m| m.id).unwrap_or(0);
                // Replay or re-lint, per file: an unchanged file under an unchanged model
                // replays its cached verdict — no read, no parse, no gate.
                let (fresh, replayed): (Vec<&FileMeta>, Vec<&FileMeta>) = metas.iter().partition(|m| {
                    !matches!(verdicts.get(&m.rel), Some(v) if v.state == m.state && v.model_id == model_id)
                });
                let mut updates: Vec<(String, CachedVerdict)> = Vec::new();
                let mut staged: Vec<(String, Hit, bool)> = Vec::new();
                // The finding-to-Hit rendering facts come from the compiled model either way.
                let hit_of = |model: &lint_train::LangModel, rule: &str, line: usize| -> (Hit, String) {
                    let (severity, advice_text, source) = model
                        .rules
                        .info_of(rule)
                        .map(|(sev, desc, src)| {
                            let sev = if sev.is_empty() { "medium".to_string() } else { sev.to_string() };
                            let adv = if desc.is_empty() { format!("violates `{rule}`") } else { desc.to_string() };
                            // A law file inside the project cites as a relative path; docs cite their URL.
                            let src = src
                                .strip_prefix(&format!("{}/", root.display()))
                                .unwrap_or(src)
                                .to_string();
                            (sev, adv, src)
                        })
                        .unwrap_or_else(|| ("medium".to_string(), format!("violates `{rule}`"), String::new()));
                    (Hit { line, rule: rule.to_string(), severity: severity.clone(), advice: advice_text, source }, severity)
                };
                let (law_watch, rule_count) = match model {
                    Some(model) => {
                        // Rules the project itself authored are the user's explicit law for this
                        // codebase — trusted fully, never gated, however weak their compiled anchor is.
                        let trusted = lint_train::project_rule_ids(root, lang);
                        let law_watch: Vec<(String, String)> = trusted
                            .iter()
                            .filter_map(|id| model.rules.detector_of(id).map(|w| (id.clone(), w)))
                            .collect();
                        // Precise AST matches are exact and staged directly. Imprecise matches —
                        // text-fallback token detectors and container-only AST patterns — must
                        // clear the Hv concept gate: the matched construct's tokens must agree
                        // with the fired rule's fingerprint more than with any other rule's. All
                        // of a language's gated findings go through ONE batched query
                        // ([`crate::lint_ai::ConceptModel::confirms_batch`]) — a single
                        // popcount-grid dispatch instead of a scalar scan per finding.
                        let mut pending: Vec<(String, crate::lint_match::Finding, bool)> = Vec::new();
                        let mut gate_tokens: Vec<(usize, Vec<String>)> = Vec::new();
                        let with_src: Vec<(&FileMeta, std::sync::Arc<String>)> = fresh
                            .iter()
                            .filter_map(|m| contents.get(&m.rel).map(|src| (*m, src.clone())))
                            .collect();
                        let per_file: Vec<Vec<crate::lint_match::Finding>> =
                            with_src.par_iter().map(|(_, src)| model.rules.flag(src)).collect();
                        let mut fresh_findings: HashMap<&str, Vec<(String, u64, bool)>> =
                            with_src.iter().map(|(m, _)| (m.rel.as_str(), Vec::new())).collect();
                        for ((meta, src), findings) in with_src.iter().zip(per_file) {
                            for finding in findings {
                                let doc_rule = !trusted.contains(&finding.rule);
                                if !finding.precise {
                                    let toks = line_tokens(src, finding.line);
                                    // Restatement guard (all imprecise findings, trusted law
                                    // included): a line repeating the rule's own words is
                                    // quoting the law, not breaking it.
                                    let desc = model
                                        .rules
                                        .info_of(&finding.rule)
                                        .map(|(_, d, _)| d)
                                        .unwrap_or("");
                                    if restates_rule(&toks, desc) {
                                        continue;
                                    }
                                    if doc_rule {
                                        gate_tokens.push((pending.len(), toks));
                                    }
                                }
                                pending.push((meta.rel.clone(), finding, doc_rule));
                            }
                        }
                        let items: Vec<(&str, Vec<&str>)> = gate_tokens
                            .iter()
                            .map(|(i, toks)| {
                                (pending[*i].1.rule.as_str(), toks.iter().map(String::as_str).collect())
                            })
                            .collect();
                        let mut rejected: HashSet<usize> = HashSet::new();
                        for (kept, (i, _)) in
                            model.concept.confirms_batch(&items).into_iter().zip(&gate_tokens)
                        {
                            if !kept {
                                rejected.insert(*i);
                            }
                        }
                        for (i, (path, finding, doc_rule)) in pending.into_iter().enumerate() {
                            if rejected.contains(&i) {
                                continue;
                            }
                            if let Some(list) = fresh_findings.get_mut(path.as_str()) {
                                list.push((finding.rule.clone(), finding.line as u64, doc_rule));
                            }
                            let (hit, _) = hit_of(model, &finding.rule, finding.line);
                            staged.push((path, hit, doc_rule));
                        }
                        // Replays: the cached post-gate findings, rendered through the same model.
                        for meta in &replayed {
                            if let Some(v) = verdicts.get(&meta.rel) {
                                for (rule, line, doc_rule) in &v.findings {
                                    let (hit, _) = hit_of(model, rule, *line as usize);
                                    staged.push((meta.rel.clone(), hit, *doc_rule));
                                }
                            }
                        }
                        // Every fresh file earns a verdict entry — findings or none — so the
                        // next run replays it. Unreadable files record state with no findings.
                        for (meta, src) in &with_src {
                            let findings = fresh_findings.remove(meta.rel.as_str()).unwrap_or_default();
                            updates.push((
                                meta.rel.clone(),
                                CachedVerdict {
                                    state: meta.state,
                                    seed: seeds.get(&meta.rel).copied().unwrap_or(0),
                                    lines: src.lines().count() as u64,
                                    model_id,
                                    findings,
                                },
                            ));
                        }
                        (law_watch, model.rules.rule_count())
                    }
                    None => {
                        // No model for this language — nothing fires, but each fresh file still
                        // caches its state and seed so warm fingerprints never re-read it.
                        for meta in &fresh {
                            let (seed, lines) = match contents.get(&meta.rel) {
                                Some(src) => (
                                    seeds.get(&meta.rel).copied().unwrap_or(0),
                                    src.lines().count() as u64,
                                ),
                                None => (seeds.get(&meta.rel).copied().unwrap_or(0), 0),
                            };
                            updates.push((
                                meta.rel.clone(),
                                CachedVerdict { state: meta.state, seed, lines, model_id: 0, findings: Vec::new() },
                            ));
                        }
                        (Vec::new(), 0)
                    }
                };
                let trace = format!(
                    "[lint-match] {lang}: {} files ({} fresh), {} rules, {} finding(s), {:.1}ms",
                    metas.len(),
                    fresh.len(),
                    rule_count,
                    staged.len(),
                    t0.elapsed().as_secs_f64() * 1e3
                );
                LangPass { staged, law_watch, trace, updates }
            })
            .collect()
    };
    let mut staged: Vec<(String, Hit, bool)> = Vec::new();
    let trace_on = std::env::var_os("HELPERS_LINT_TRACE").is_some();
    let mut fresh_updates: Vec<(String, CachedVerdict)> = Vec::new();
    for pass in passes {
        if trace_on {
            eprintln!("{}", pass.trace);
        }
        staged.extend(pass.staged);
        law_watch.extend(pass.law_watch);
        fresh_updates.extend(pass.updates);
    }

    // 3b) Self-validation against reality: a doc-learned rule that fires on more than 1% of every
    //     line scanned (and at least 20 lines) is a mislearned pattern — noisy docs scraping, not
    //     hundreds of real mistakes. Quarantine it wholesale and say so, instead of flooding the
    //     report. Project-authored rules are never quarantined: a project-wide convention
    //     violation legitimately fires everywhere.
    // Fire rates are judged against the LINES OF THE RULE'S OWN LANGUAGE, not the whole
    // project: a rust rule's noise must not be diluted to invisibility by a thousand markdown
    // files that it never even ran against.
    let path_lang: HashMap<&str, &str> = by_language
        .iter()
        .flat_map(|(l, metas)| metas.iter().map(move |m| (m.rel.as_str(), l.as_str())))
        .collect();
    let lang_lines: HashMap<&str, usize> = by_language
        .iter()
        .map(|(l, metas)| {
            (l.as_str(), metas.iter().map(|m| file_lines[m.rel.as_str()]).sum())
        })
        .collect();
    let mut fires: HashMap<(&str, &str), usize> = HashMap::new(); // (rule, lang) → hits
    let mut file_fires: HashMap<(&str, &str), usize> = HashMap::new();
    for (path, hit, doc_rule) in &staged {
        if *doc_rule {
            let lang = path_lang.get(path.as_str()).copied().unwrap_or("");
            *fires.entry((hit.rule.as_str(), lang)).or_default() += 1;
            *file_fires.entry((hit.rule.as_str(), path.as_str())).or_default() += 1;
        }
    }
    // Concentrated mislearning: ≥20 fires inside one file covering >10% of its lines is a pattern
    // matching the file's fabric (every `[`, every backtick), not 20+ separate mistakes.
    let concentrated: HashSet<&str> = file_fires.iter()
        .filter(|((_, path), &n)| n >= 20 && n * 10 > file_lines.get(path).copied().unwrap_or(usize::MAX))
        .map(|((rule, _), _)| *rule)
        .collect();
    // Two-tier by detector structure, mirroring the compile-time reference-fire bar: a
    // detector whose own shape can vouch for it gets 1%; a DEGENERATE one (single token, bare
    // leaf) has only fire statistics as witness and gets 0.1% — the reference corpus can be
    // too small to testify about a token that is rare in doc examples but pervasive in real
    // projects (`path` passed compile on 8 corpus lines, then fired 305× here). The
    // degenerate tier's FLOOR is 50 fires, not 20: its incident class fires in the hundreds
    // (`path`, 305×), while a legitimately much-violated single-token convention fires in
    // the twenties (`no_var_declaration`, 20+ real `var` declarations on one repo) — a
    // floor of 20 quarantined the true rule exactly when it was most violated.
    let degenerate =
        |id: &str| models.values().any(|m| m.rules.degenerate_detector(id));
    let quarantined: std::collections::BTreeSet<String> = fires.iter()
        .filter(|((id, lang), &n)| {
            let lines = lang_lines.get(*lang).copied().unwrap_or(total_lines);
            let (floor, per_mille) = if degenerate(id) { (50, 1000) } else { (20, 100) };
            (n >= floor && n * per_mille > lines) || concentrated.contains(*id)
        })
        .map(|((id, _), _)| id.to_string())
        .collect();
    for (path, hit, _) in staged.into_iter().filter(|(_, h, _)| !quarantined.contains(&h.rule)) {
        push_hit(&mut reports, &path, hit);
    }

    // 5) Apply per-project config: suppress ignored rules, apply severity overrides.
    for report in &mut reports {
        report.hits.retain(|h| !ignore_set.contains(h.rule.as_str()));
        for hit in &mut report.hits {
            if let Some(sev) = config.severity_overrides.get(&hit.rule) {
                hit.severity = sev.clone();
            }
        }
    }

    // A file whose every finding was suppressed above has nothing to report: drop it, or the
    // verdict counts it and the body prints a bare path with no findings under it.
    reports.retain(|r| !r.hits.is_empty());

    let analyzed: BTreeMap<String, usize> = by_language.iter().map(|(l, fs)| (l.clone(), fs.len())).collect();
    let unanalyzed: BTreeMap<String, usize> = BTreeMap::new();
    reports.sort_by(|a, b| a.path.cmp(&b.path));
    let lex = io_lexicon(config);
    let mut body = render(root, &reports, &analyzed, &unanalyzed, &sources, max, lex.as_ref());
    let mut law_watch_block = String::new();
    if !law_watch.is_empty() {
        law_watch_block.push_str("\nYour law, as understood:\n");
        for (id, watching) in &law_watch {
            law_watch_block.push_str(&format!("  {id} → watching for {watching}\n"));
        }
    }
    body.push_str(&law_watch_block);

    body.push_str(run_footer);
    body.push_str(&render_quarantine(&quarantined));
    body.push_str(feedback_footer);

    (body, fresh_updates, quarantined, law_watch_block)
}

/// Arm the kqueue tier and commit `body` under the soundness protocol (LINTER.md, "The
/// kqueue tier"): racy-window gate, ARM first, RE-SWEEP and require the walk fold
/// unchanged (an edit racing the arming lands in the fold difference or as a pending
/// event), require a quiet drain, then commit. Runs after every answer — full runs AND
/// stat-tier replays — so the steady state converges to the microsecond path.
#[allow(clippy::too_many_arguments)]
fn kq_arm_and_commit(
    root: &Path,
    data: &Path,
    files: &[crate::index::walk::WalkedFile],
    walked_dirs: &[PathBuf],
    witness: &crate::lint_replay::Witness,
    walk_fold: u128,
    memo_key: &str,
    body: &str,
) {
    let trace = std::env::var_os("HELPERS_LINT_TRACE").is_some();
    // The kq tier needs NO mtime racy window: its invalidation is content-true events,
    // not (mtime, len) folds — a same-tick same-length rewrite still posts NOTE_WRITE.
    // The stat tier's on-disk store keeps the window; gating kq commits on it only
    // delayed convergence after every edit (measured). Kept as a trace note only.
    if false && !crate::lint_replay::replay_safe(witness) {
        if trace {
            let culprit = files
                .iter()
                .find(|f| f.mtime == witness.newest)
                .map(|f| f.rel.clone())
                .or_else(|| {
                    let mut hit = None;
                    for d in [
                        root.join(".helpers"),
                        root.join(".helpers/lint-rules"),
                        data.join("corpus"),
                        data.join("lint-index"),
                        lint_train::model_dir_pub(),
                    ] {
                        for e in crate::index::walk::scan_dir(&d) {
                            if e.mtime == witness.newest {
                                hit = Some(format!("{}/{}", d.display(), e.name));
                            }
                        }
                    }
                    hit
                });
            eprintln!(
                "[lint-kq] no commit: inside the racy window (newest input: {culprit:?})"
            );
        }
        return;
    }
    let watch = kq_watch_set(root, data, files, walked_dirs);
    if !crate::lint_kq::arm(root, &watch) {
        if trace {
            eprintln!("[lint-kq] no commit: watch set incomplete ({} paths)", watch.len());
        }
        return;
    }
    let refreshed = crate::lint_replay::walk_witness(&crate::index::walk::walk_repo(root));
    if refreshed.fold != walk_fold {
        if trace {
            eprintln!("[lint-kq] no commit: tree changed during arming");
        }
        return;
    }
    if !crate::lint_kq::confirm_quiet(root) {
        if trace {
            eprintln!("[lint-kq] no commit: events during arming");
        }
        return;
    }
    crate::lint_kq::commit(root, memo_key, body);
    if trace {
        eprintln!("[lint-kq] committed ({} watched paths)", watch.len());
    }
}


/// The INCREMENTAL tier (LINTER.md, "The incremental tier"): a fired-set run over the
/// daemon's cached state — the lint IS the change. Only content edits to already-known
/// files ride this path in v1; anything structural (new files/dirs, gitignore edits, any
/// aux/law/config event, a cold cache) returns `None` and the stat tier's full run — which
/// repopulates the cache — takes over. Every fallback is the slower sound path.
fn incremental_run(
    root: &Path,
    _data: &Path,
    max: usize,
    filter: &Option<BTreeSet<String>>,
    memo_key: &str,
) -> Option<String> {
        let t0 = std::time::Instant::now();
    let trace = std::env::var_os("HELPERS_LINT_TRACE").is_some();
    let fired = crate::lint_kq::fired_paths(root)?;
    let mut mark_last = std::time::Instant::now();
    let mut mark = move |name: &str| {
        if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
            eprintln!("[lint-inc] {name} {:.0}µs", mark_last.elapsed().as_micros());
            mark_last = std::time::Instant::now();
        }
    };
    let mut states = daemon_state().lock().unwrap_or_else(|e| e.into_inner());
    let st = states.get_mut(root)?;
    // AUX events reprice the world (models, law, config, feedback, ignore semantics).
    let helpers_dir = root.join(".helpers");
    for p in &fired {
        if !p.starts_with(root) || p.starts_with(&helpers_dir) || st.law_abs.contains(p) {
            return None;
        }
        if matches!(p.file_name().and_then(|n| n.to_str()), Some(".gitignore") | Some(".ignore")) {
            return None;
        }
    }
    // Refresh exactly the fired directories (a fired file refreshes via its parent dir —
    // one bulk syscall re-verifies every sibling for free).
    let dir_set: std::collections::BTreeSet<&PathBuf> = st.dirs.iter().collect();
    let mut rescan: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
    for p in &fired {
        if dir_set.contains(p) {
            rescan.insert(p.clone());
        } else {
            rescan.insert(p.parent()?.to_path_buf());
        }
    }
    let mut changed_any = false;
    let mut changed_idx: Vec<usize> = Vec::new();
    for dir in &rescan {
        if !dir_set.contains(dir) {
            return None; // a fired path under an unknown dir — structure changed
        }
        // Lazy per-dir index: only this directory's cached files, never a full clone.
        let mut in_dir: HashMap<&Path, usize> = HashMap::new();
        for (i, f) in st.files.iter().enumerate() {
            if f.abs.parent() == Some(dir.as_path()) {
                in_dir.insert(f.abs.as_path(), i);
            }
        }
        let entries = crate::index::walk::scan_dir(dir);
        let mut updates: Vec<(usize, u128, u128)> = Vec::new(); // (idx, mtime, state)
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for e in &entries {
            let abs = dir.join(&e.name);
            if e.is_dir {
                if !dir_set.contains(&abs)
                    && !crate::index::walk::SKIP_DIRS.contains(&e.name.as_str())
                {
                    return None; // a NEW directory — the walk owns recursion + ignore law
                }
                continue;
            }
            if !e.is_file {
                continue;
            }
            match in_dir.get(abs.as_path()) {
                Some(&i) => {
                    seen.insert(abs);
                    if st.files[i].state != e.state {
                        updates.push((i, e.mtime, e.state));
                    }
                }
                // A NEW file needs the walk's per-dir ignore chain — defer to the full run.
                None => return None,
            }
        }
        if in_dir.iter().any(|(abs, _)| !seen.contains(*abs)) {
            return None; // a deletion — membership changed; the full walk re-owns the set
        }
        for (i, mtime, state) in updates {
            let f = &mut st.files[i];
            st.walk_w.fold ^= crate::lint_replay::file_term(&f.rel, f.state);
            f.mtime = mtime;
            f.state = state;
            st.walk_w.fold ^= crate::lint_replay::file_term(&f.rel, f.state);
            st.walk_w.newest = st.walk_w.newest.max(mtime);
            changed_any = true;
            changed_idx.push(i);
        }
    }
    mark("rescan");
    // ZERO-CHANGE fast path (a fired vnode whose content folded identical — editors save
    // twice, our own reopen races): the cached body IS the answer.
    if !changed_any {
        let body = st.bodies.get(memo_key)?.clone();
        if crate::lint_kq::rearm_fired(root) {
            crate::lint_kq::commit(root, memo_key, &body);
        }
        if trace {
            eprintln!("[lint-inc] zero-change replay in {}µs", t0.elapsed().as_micros());
        }
        return Some(body);
    }
    // Single-file passes for exactly the CHANGED files; everything else renders straight
    // from the verdict cache — the verdicts ARE the staged findings (LINTER.md, "The
    // incremental tier").
    let n_changed = changed_idx.len();
    let mut dirty = false;
    for i in changed_idx {
        let (rel, abs, state) = {
            let f = &st.files[i];
            (f.rel.clone(), f.abs.clone(), f.state)
        };
        let Some(lang) = st.rel_lang.get(&rel).cloned() else { continue }; // unselected
        if filter.as_ref().is_some_and(|set| !set.contains(&lang)) {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(&abs) else {
            st.verdicts.insert(
                rel,
                CachedVerdict { state, seed: 0, lines: 0, model_id: 0, findings: Vec::new() },
            );
            dirty = true;
            continue;
        };
        let seed = crate::lint_ai::token_seed(&src);
        let lines = src.lines().count() as u64;
        let empty_info = HashMap::new();
        let empty_trust = HashSet::new();
        let (model_id, findings) = match st.models.get(&lang) {
            Some(model) => (
                model.id,
                lint_file_findings(
                    model,
                    st.trusted.get(&lang).unwrap_or(&empty_trust),
                    st.info.get(&lang).unwrap_or(&empty_info),
                    &src,
                ),
            ),
            None => (0, Vec::new()),
        };
        st.verdicts.insert(rel, CachedVerdict { state, seed, lines, model_id, findings });
        dirty = true;
    }
    mark("relint");
    let quarantined = quarantine_from_state(st, filter);
    let body = render_from_state(root, st, max, filter, &quarantined);
    mark("render");
    let witness = crate::lint_replay::combine(st.walk_w, st.aux_w);
    if dirty {
        let root2 = root.to_path_buf();
        let verd = st.verdicts.clone();
        let key = memo_key.to_string();
        let body2 = body.clone();
        std::thread::spawn(move || {
            save_verdicts(&root2, &verd);
            crate::lint_replay::store(&root2, &key, witness, &body2);
        });
    }
    st.bodies.insert(memo_key.to_string(), body.clone());
    mark("persist");
    // Membership is unchanged on this path by construction — reopen only what fired
    // (open-new-before-close-old inside, so a racing edit is never lost). Events are
    // content-true: no mtime racy window applies to kq commits.
    if crate::lint_kq::rearm_fired(root) {
        crate::lint_kq::commit(root, memo_key, &body);
    }
    mark("rearm+commit");
    if trace {
        eprintln!(
            "[lint-inc] incremental lint in {}µs ({} fired, {} changed file(s))",
            t0.elapsed().as_micros(),
            fired.len(),
            n_changed
        );
    }
    Some(body)
}


/// Lint ONE file through the full per-file discipline — flag, restatement guard, Hv gate —
/// mirroring the language pass exactly, for the incremental tier's changed files. Returns
/// the post-gate findings in staging order.
fn lint_file_findings(
    model: &lint_train::LangModel,
    trusted: &HashSet<String>,
    info: &HashMap<String, (String, String, String)>,
    src: &str,
) -> Vec<(String, u64, bool)> {
    let mut pending: Vec<(crate::lint_match::Finding, bool)> = Vec::new();
    let mut gate_tokens: Vec<(usize, Vec<String>)> = Vec::new();
    for finding in model.rules.flag(src) {
        let doc_rule = !trusted.contains(&finding.rule);
        if !finding.precise {
            let toks = line_tokens(src, finding.line);
            let desc = info.get(&finding.rule).map(|(_, d, _)| d.as_str()).unwrap_or("");
            if restates_rule(&toks, desc) {
                continue;
            }
            if doc_rule {
                gate_tokens.push((pending.len(), toks));
            }
        }
        pending.push((finding, doc_rule));
    }
    let items: Vec<(&str, Vec<&str>)> = gate_tokens
        .iter()
        .map(|(i, toks)| (pending[*i].0.rule.as_str(), toks.iter().map(String::as_str).collect()))
        .collect();
    let mut rejected: HashSet<usize> = HashSet::new();
    for (kept, (i, _)) in model.concept.confirms_batch(&items).into_iter().zip(&gate_tokens) {
        if !kept {
            rejected.insert(*i);
        }
    }
    pending
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !rejected.contains(i))
        .map(|(_, (f, doc))| (f.rule, f.line as u64, doc))
        .collect()
}

/// The self-validation quarantine, computed straight from the verdict cache — identical
/// thresholds to the full run's pass (per-language denominators, degenerate two-tier,
/// concentration), because the verdicts ARE the staged findings.
fn quarantine_from_state(
    st: &DaemonState,
    filter: &Option<BTreeSet<String>>,
) -> std::collections::BTreeSet<String> {
    let mut fires: HashMap<(&str, &str), usize> = HashMap::new();
    let mut file_fires: HashMap<(&str, &str), usize> = HashMap::new();
    let mut lang_lines: HashMap<&str, usize> = HashMap::new();
    let mut total_lines = 0usize;
    for (rel, v) in &st.verdicts {
        let Some(lang) = st.rel_lang.get(rel) else { continue };
        if filter.as_ref().is_some_and(|set| !set.contains(lang)) {
            continue;
        }
        *lang_lines.entry(lang.as_str()).or_default() += v.lines as usize;
        total_lines += v.lines as usize;
        for (rule, _, doc) in &v.findings {
            if *doc {
                *fires.entry((rule.as_str(), lang.as_str())).or_default() += 1;
                *file_fires.entry((rule.as_str(), rel.as_str())).or_default() += 1;
            }
        }
    }
    let concentrated: HashSet<&str> = file_fires
        .iter()
        .filter(|((_, rel), &n)| {
            n >= 20
                && n * 10
                    > st.verdicts.get(**&rel).map(|v| v.lines as usize).unwrap_or(usize::MAX)
        })
        .map(|((rule, _), _)| *rule)
        .collect();
    let degenerate =
        |id: &str| st.models.values().any(|m| m.rules.degenerate_detector(id));
    fires
        .iter()
        .filter(|((id, lang), &n)| {
            let lines = lang_lines.get(*lang).copied().unwrap_or(total_lines);
            let (floor, per_mille) = if degenerate(id) { (50, 1000) } else { (20, 100) };
            (n >= floor && n * per_mille > lines) || concentrated.contains(*id)
        })
        .map(|((id, _), _)| id.to_string())
        .collect()
}

/// Render the report straight from the verdict cache + the state's cached blocks — the
/// same sections in the same order as the full run's body, so the two tiers are
/// indistinguishable to the reader.
fn render_from_state(
    root: &Path,
    st: &DaemonState,
    max: usize,
    filter: &Option<BTreeSet<String>>,
    quarantined: &std::collections::BTreeSet<String>,
) -> String {
    let mut reports: Vec<FileReport> = Vec::new();
    for (rel, v) in &st.verdicts {
        let Some(lang) = st.rel_lang.get(rel) else { continue };
        if filter.as_ref().is_some_and(|set| !set.contains(lang)) {
            continue;
        }
        let empty = HashMap::new();
        let info = st.info.get(lang).unwrap_or(&empty);
        let hits: Vec<Hit> = v
            .findings
            .iter()
            .filter(|(rule, _, _)| !quarantined.contains(rule) && !st.ignore_set.contains(rule.as_str()))
            .map(|(rule, line, _)| {
                let (sev, adv, src) = info.get(rule).cloned().unwrap_or_else(|| {
                    ("medium".to_string(), format!("violates `{rule}`"), String::new())
                });
                let sev = st.config.severity_overrides.get(rule).cloned().unwrap_or(if sev.is_empty() { "medium".into() } else { sev });
                let adv = if adv.is_empty() { format!("violates `{rule}`") } else { adv };
                let src = src.strip_prefix(&format!("{}/", root.display())).unwrap_or(&src).to_string();
                Hit { line: *line as usize, rule: rule.clone(), severity: sev, advice: adv, source: src }
            })
            .collect();
        if !hits.is_empty() {
            reports.push(FileReport { path: rel.clone(), hits });
        }
    }
    let mut analyzed: BTreeMap<String, usize> = BTreeMap::new();
    for lang in st.rel_lang.values() {
        if filter.as_ref().is_some_and(|set| !set.contains(lang)) {
            continue;
        }
        *analyzed.entry(lang.clone()).or_default() += 1;
    }
    let lex = io_lexicon(&st.config);
    let mut body = render(root, &reports, &analyzed, &BTreeMap::new(), &st.sources, max, lex.as_ref());
    body.push_str(&st.law_watch_block);
    body.push_str(&st.run_footer);
    body.push_str(&render_quarantine(quarantined));
    body.push_str(&st.feedback_footer);
    body
}

/// The kqueue tier's complete watch set: every walked file and directory, plus every
/// auxiliary input the witness folds — law and config under `.helpers/`, the corpus, both
/// lint-index directories (the project data one and the machine one where `add_source`
/// writes), and the model dir's top level. Derived caches (`lint-verdicts/`,
/// `lint-replay/`) are excluded exactly as they are from the stat witness, so a run never
/// invalidates its own memo (LINTER.md, "The kqueue tier").
fn kq_watch_set(
    root: &Path,
    data: &Path,
    files: &[crate::index::walk::WalkedFile],
    dirs: &[PathBuf],
) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = files.iter().map(|f| f.abs.clone()).collect();
    v.extend(dirs.iter().cloned());
    let mut dir_and_entries = |d: PathBuf| {
        if !d.is_dir() {
            return;
        }
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let name = e.file_name();
                if name == "lint-verdicts" || name == "lint-replay" {
                    continue;
                }
                v.push(e.path());
            }
        }
        v.push(d);
    };
    dir_and_entries(root.join(".helpers"));
    dir_and_entries(root.join(".helpers/lint-rules"));
    dir_and_entries(data.join("corpus"));
    dir_and_entries(data.join("lint-index"));
    if let Some(machine_index) = crate::lint_docs::learned_sources_path_pub().parent() {
        dir_and_entries(machine_index.to_path_buf());
    }
    dir_and_entries(lint_train::model_dir_pub());
    v.sort();
    v.dedup();
    v
}

// ── runtime resource resolution ──────────────────────────────────────────────

/// Public for sibling tools that need the same data root.
pub(crate) fn data_root_pub() -> PathBuf { data_root() }

/// The languages of the project's own files — the same extension→language resolution the lint
/// walk uses (unknown extensions are languages named by the extension; law documents are not
/// source). `lint_config action=train` unions these with the registry so the current project
/// is always covered by one training run.
pub(crate) fn project_languages(root: &Path) -> Vec<String> {
    let law: HashSet<PathBuf> =
        lint_train::rule_documents(root).into_iter().map(|(p, _)| p).collect();
    let mut langs: Vec<String> = Vec::new();
    let mut lang_of_ext: HashMap<String, String> = HashMap::new();
    for f in walk_repo(root) {
        if law.contains(&f.abs) {
            continue;
        }
        let ext = f.ext;
        if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let lang = lang_of_ext.entry(ext.clone()).or_insert_with(|| resolve_language(&ext)).clone();
        // Prose formats are reading material, never doc-trained modules — asking for "man
        // page documentation" would be noise, not setup.
        if crate::lint_match::prose_lang(&lang) {
            continue;
        }
        if !langs.contains(&lang) {
            langs.push(lang);
        }
    }
    langs
}

/// Locate the directory that holds the linter's knowledge sources (`extraDocs/`, `lint-index/`).
/// Prefers the resolved workspace root (the dev checkout); otherwise walks up from the executable.
/// Always returns a path — missing files fall back to the embedded copies in [`crate::lint_train`].
fn data_root() -> PathBuf {
    // Memoized per HELPERS_WORKSPACE_ROOTS value: the discovery below stats several
    // directories, and the daemon paid it on every call before even reaching the
    // microsecond tier.
    type Cache = std::sync::Mutex<HashMap<String, PathBuf>>;
    static CACHE: std::sync::OnceLock<Cache> = std::sync::OnceLock::new();
    let key = std::env::var("HELPERS_WORKSPACE_ROOTS").unwrap_or_default();
    if let Some(hit) = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return hit.clone();
    }
    let out = data_root_uncached();
    CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, out.clone());
    out
}

/// Build the human-language I/O overlay for a run, or `None` for the English default (LINTER.md,
/// "The human-language I/O overlay"). Reads `HELPERS_LINT_LANG` then the project's `io_language`;
/// loads the bilingual lexicon from the machine cache or the crate asset. `None` ⇒ output is
/// byte-for-byte English, and no lexicon file is even read.
fn io_lexicon(config: &LintConfig) -> Option<crate::lint_lang::Lexicon> {
    let lang = crate::lint_lang::Lexicon::selected(&config.io_language)?;
    crate::lint_lang::Lexicon::load(&data_root(), &lang)
}

fn data_root_uncached() -> PathBuf {
    let ws = workspace_root();
    if ws.join("extraDocs").exists() || ws.join("lint-index").exists() {
        return ws;
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(d) = dir {
            if d.join("extraDocs").exists() || d.join("lint-index").exists() {
                return d;
            }
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    ws
}


// ── schema ───────────────────────────────────────────────────────────────────

/// MCP schema for the `lint` tool.
pub fn schema() -> Value {
    json!({
        "name": "lint",
        "description": "AI lint for the whole project. Law = the project's own rule files (.helpers/lint-rules/, root lintPref — plain English) + live official docs per language + the curated catalog; everything else it reads is comprehension, never enforcement. Fires learned AST patterns via tree-sitter, confirms imprecise matches with the Hv concept gate, reports law it could not compile, and self-quarantines mislearned rules. Flag wrong/missing findings with lint_flag.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "root": { "type": "string", "description": "Project root. Defaults to the current workspace." },
                "max": { "type": "integer", "description": "Max finding lines to list (1-500). Default 80." },
                "modules": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional language filter: `rust`, `python`, `js`/`ts`, `go`. `all` or omitted reviews every language."
                }
            },
            "required": []
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_quoting_the_rules_own_words_is_a_restatement_not_a_violation() {
        let desc = "Never call `eval` anywhere in this project; parse the input explicitly.";
        // A doc line quoting the law shares most of its words → restatement.
        let quote = line_tokens("Never call eval anywhere in this project; parse the input explicitly.", 1);
        assert!(restates_rule(&quote, desc));
        // A real violation shares only the construct itself → finding stands.
        let code = line_tokens("    value = eval(expr)", 1);
        assert!(!restates_rule(&code, desc));
        // A code comment that borrows a few of the rule's words is still code, not a quote.
        let comment = line_tokens("result = eval(s)  # never use in prod", 1);
        assert!(!restates_rule(&comment, desc));
        // A short description cannot mark ordinary lines as restatements.
        assert!(!restates_rule(&line_tokens("x = eval(y)", 1), "avoid `eval`"));
    }

    #[test]
    fn group_hits_orders_by_severity_and_collapses() {
        let hits = vec![
            Hit { line: 9, rule: "a".into(), severity: "low".into(), advice: "x".into(), source: String::new() },
            Hit { line: 3, rule: "b".into(), severity: "high".into(), advice: "y".into(), source: "https://d/r".into() },
            Hit { line: 5, rule: "b".into(), severity: "high".into(), advice: "y".into(), source: "https://d/r".into() },
        ];
        let lines = group_hits(&hits, None);
        assert!(lines[0].contains("[high]") && lines[0].contains("×2"), "high collapses first: {lines:?}");
        assert!(lines[1].contains("[low]"));
    }

    #[test]
    fn data_root_resolves_to_a_dir_with_sources_or_workspace() {
        let d = data_root();
        assert!(d.join("extraDocs").exists() || d.join("lint-index").exists() || d.exists());
    }

    #[test]
    fn unknown_lang_in_filter_passes_through_not_silently_dropped() {
        // An unrecognised language name should reach the filter set unchanged so
        // the caller sees zero files for it rather than "all languages" being reviewed.
        let f = parse_lang_filter(&json!({ "modules": ["elixir"] })).unwrap();
        assert!(f.contains("elixir"), "unknown lang passes through: {f:?}");
    }

    #[test]
    fn extension_aliases_resolve_to_canonical_names() {
        let f = parse_lang_filter(&json!({ "modules": ["ts", "py"] })).unwrap();
        assert!(f.contains("typescript") && f.contains("python"));
    }
}
