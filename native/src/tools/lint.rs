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
use crate::util::file_lang;

/// Per-project linter preferences loaded from `.helpers/lint.json`.
///
/// Agents and users write this file (via `lint_config`) to tailor which rules fire,
/// what languages are reviewed, and how severe each finding is reported.
#[derive(Default, serde::Deserialize)]
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
/// Extension-like aliases ("ts", "py") are resolved via `file_lang` to their canonical name.
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
            other => { set.insert(file_lang(other).unwrap_or(other).to_string()); }
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
    let root = root_arg(args);
    if !root.exists() {
        return Err(format!("lint: path not found: {}", root.display()));
    }
    let max = args.get("max").and_then(Value::as_u64).unwrap_or(80).clamp(1, 500) as usize;
    let filter = parse_lang_filter(args);
    let data = data_root();

    // Load per-project config, then merge in feedback-driven auto-suppressions: rules a developer
    // has flagged as false positives enough times (see `crate::lint_feedback`) are folded into the
    // same `ignore_rules` list, so they are suppressed through the normal config path below.
    let mut config = load_config(&root);
    let auto_suppressed = crate::lint_feedback::merge_auto_suppressed(&mut config, &root);
    let ignore_set: HashSet<&str> = config.ignore_rules.iter().map(String::as_str).collect();

    // 1) Walk the project and partition by language: those with a tree-sitter grammar are analyzed
    //    with the AST engine; the rest are still analyzed via the token-regex fallback, so nothing
    //    is dropped for lacking a grammar.
    let files = walk_repo(&root);
    // The project's own rule documents (root lintPref, .helpers/lint-rules) are instructions TO
    // the linter, not source to be linted — never analyze the law as if it were code.
    let law: HashSet<PathBuf> = lint_train::rule_documents(&root).into_iter().map(|(p, _)| p).collect();
    // Select the lintable files first (cheap), then read them ALL in parallel — the read pass
    // is pure I/O over independent files and was the warm run's single largest stage when
    // sequential. Order is preserved through the indexed collect, so the grouping (and every
    // report downstream) stays deterministic.
    let selected: Vec<(String, &crate::index::walk::WalkedFile)> = files
        .iter()
        .filter(|f| !law.contains(&f.abs))
        .filter_map(|f| {
            // A known extension resolves to its canonical grammar name; an UNKNOWN one becomes a
            // language named by the extension itself — no built-in language list. Such files ride
            // the token-regex engine, project rules (`.helpers/lint-rules/<ext>.md`, `any.md`),
            // and on-the-fly docs discovery; unreadable (non-UTF-8) files fall out at the read.
            let ext = f.ext.to_lowercase();
            if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
                return None;
            }
            let l = file_lang(&ext).map(str::to_string).unwrap_or(ext);
            if filter.as_ref().is_some_and(|set| !set.contains(l.as_str())) {
                return None;
            }
            if config.languages.as_ref().is_some_and(|set| !set.contains(&l)) {
                return None;
            }
            Some((l, f))
        })
        .collect();
    use rayon::prelude::*;
    let bodies: Vec<Option<String>> = selected
        .par_iter()
        .map(|(_, f)| std::fs::read_to_string(&f.abs).ok())
        .collect();
    let mut by_language: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for ((l, f), body) in selected.into_iter().zip(bodies) {
        if let Some(src) = body {
            by_language.entry(l).or_default().push((f.rel.clone(), src));
        }
    }

    // 2) Train / load one model per detected language (checksum-cached), and the advice map that
    //    carries each rule's English description and severity for rendering. The file map itself
    //    is the grounding corpus (`lang → (path, body)`): a law names constructs that live in the
    //    code it governs, so the project grounds construct selection for ANY language, shape-free
    //    — passed by reference, never duplicated.
    let langs: Vec<String> = by_language.keys().cloned().collect();
    mark(&mut stages, "walk+read");
    let (report, models) = lint_train::ensure_models(&langs, &data, &root, &by_language);
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
    //    Every fire-rate denominator derives from ONE parallel line count per file.
    let file_lines: HashMap<&str, usize> = {
        use rayon::prelude::*;
        let files: Vec<(&str, &str)> =
            by_language.values().flatten().map(|(p, s)| (p.as_str(), s.as_str())).collect();
        files.par_iter().map(|(p, s)| (*p, s.lines().count())).collect()
    };
    let total_lines: usize = file_lines.values().sum();
    // Languages fire in PARALLEL — each language's pass (trusted-law lookup, matching, concept
    // gate) is independent, so the stage costs the slowest language, not the sum. Results fold
    // back in language order, and each language's findings stay in file order, so the report
    // is deterministic. Within a language, files also match in parallel (rayon nests fine).
    struct LangPass {
        staged: Vec<(String, Hit, bool)>, // (path, hit, unverified doc rule?)
        law_watch: Vec<(String, String)>,
        trace: String,
    }
    let passes: Vec<LangPass> = {
        use rayon::prelude::*;
        let entries: Vec<(&String, &Vec<(String, String)>)> = by_language.iter().collect();
        entries
            .par_iter()
            .filter_map(|(lang, lang_files)| {
                let model = models.get(*lang)?;
                let t0 = std::time::Instant::now();
                // Rules the project itself authored are the user's explicit law for this
                // codebase — trusted fully, never gated, however weak their compiled anchor is.
                let trusted = lint_train::project_rule_ids(&data, &root, lang);
                let law_watch: Vec<(String, String)> = trusted
                    .iter()
                    .filter_map(|id| model.rules.detector_of(id).map(|w| (id.clone(), w)))
                    .collect();
                // Precise AST matches are exact and staged directly. Imprecise matches —
                // text-fallback regexes and container-only AST patterns (several distinct rules
                // compile to the same bare `list`) — must clear the Hv concept gate: the matched
                // construct's tokens must agree with the fired rule's fingerprint more than with
                // any other rule's, so a match that belongs to a different concept is dropped —
                // no word blocklist involved. All of a language's gated findings go through ONE
                // batched query ([`crate::lint_ai::ConceptModel::confirms_batch`]), a single
                // popcount-grid dispatch instead of a scalar scan per finding.
                let mut pending: Vec<(String, crate::lint_match::Finding, bool)> = Vec::new();
                let mut gate_tokens: Vec<(usize, Vec<String>)> = Vec::new(); // (pending idx, construct tokens)
                let per_file: Vec<Vec<crate::lint_match::Finding>> =
                    lang_files.par_iter().map(|(_, src)| model.rules.flag(src)).collect();
                for ((path, src), findings) in lang_files.iter().zip(per_file) {
                    for finding in findings {
                        let doc_rule = !trusted.contains(&finding.rule);
                        if !finding.precise {
                            let toks = line_tokens(src, finding.line);
                            // Restatement guard (all imprecise findings, trusted law included):
                            // a line repeating the rule's own words is quoting the law, not
                            // breaking it.
                            let desc =
                                model.rules.info_of(&finding.rule).map(|(_, d, _)| d).unwrap_or("");
                            if restates_rule(&toks, desc) {
                                continue;
                            }
                            if doc_rule {
                                gate_tokens.push((pending.len(), toks));
                            }
                        }
                        pending.push((path.clone(), finding, doc_rule));
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
                let mut staged: Vec<(String, Hit, bool)> = Vec::new();
                for (i, (path, finding, doc_rule)) in pending.into_iter().enumerate() {
                    if rejected.contains(&i) {
                        continue;
                    }
                    // The compiled model carries each rule's reporting facts — no catalog re-read.
                    let (severity, advice_text, source) = model
                        .rules
                        .info_of(&finding.rule)
                        .map(|(sev, desc, src)| {
                            let sev = if sev.is_empty() { finding.severity.clone() } else { sev.to_string() };
                            let adv = if desc.is_empty() { format!("violates `{}`", finding.rule) } else { desc.to_string() };
                            // A law file inside the project cites as a relative path; docs cite their URL.
                            let src = src
                                .strip_prefix(&format!("{}/", root.display()))
                                .unwrap_or(src)
                                .to_string();
                            (sev, adv, src)
                        })
                        .unwrap_or_else(|| (finding.severity.clone(), format!("violates `{}`", finding.rule), String::new()));
                    staged.push((path, Hit { line: finding.line, rule: finding.rule, severity, advice: advice_text, source }, doc_rule));
                }
                let trace = format!(
                    "[lint-match] {lang}: {} files, {} rules, {} finding(s), {:.1}ms",
                    lang_files.len(),
                    model.rules.rule_count(),
                    staged.len(),
                    t0.elapsed().as_secs_f64() * 1e3
                );
                Some(LangPass { staged, law_watch, trace })
            })
            .collect()
    };
    let mut staged: Vec<(String, Hit, bool)> = Vec::new();
    let trace_on = std::env::var_os("HELPERS_LINT_TRACE").is_some();
    for pass in passes {
        if trace_on {
            eprintln!("{}", pass.trace);
        }
        staged.extend(pass.staged);
        law_watch.extend(pass.law_watch);
    }
    mark(&mut stages, "match+gate");

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
        .flat_map(|(l, files)| files.iter().map(move |(p, _)| (p.as_str(), l.as_str())))
        .collect();
    let lang_lines: HashMap<&str, usize> = by_language
        .iter()
        .map(|(l, files)| {
            (l.as_str(), files.iter().map(|(p, _)| file_lines[p.as_str()]).sum())
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
    // projects (`path` passed compile on 8 corpus lines, then fired 305× here).
    let degenerate =
        |id: &str| models.values().any(|m| m.rules.degenerate_detector(id));
    let quarantined: std::collections::BTreeSet<String> = fires.iter()
        .filter(|((id, lang), &n)| {
            let lines = lang_lines.get(*lang).copied().unwrap_or(total_lines);
            let per_mille = if degenerate(id) { 1000 } else { 100 };
            (n >= 20 && n * per_mille > lines) || concentrated.contains(*id)
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
    let mut body = render(&root, &reports, &analyzed, &unanalyzed, &sources, max);
    if !law_watch.is_empty() {
        body.push_str("\nYour law, as understood:\n");
        for (id, watching) in &law_watch {
            body.push_str(&format!("  {id} → watching for {watching}\n"));
        }
    }
    body.push_str(&render_unenforced(&report.unenforced));
    // Law files whose language matches no analyzed file are INERT — reported, never skipped
    // silently (ledger #16): a typo'd stem must surface here, not as a CLEAN verdict.
    let inert: Vec<String> = lint_train::rule_documents(&root)
        .into_iter()
        .filter(|(_, lang)| lang != "any" && !by_language.contains_key(lang.as_str()))
        .map(|(p, lang)| {
            let rel = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
            format!("{rel} (governs '{lang}')")
        })
        .collect();
    if !inert.is_empty() {
        body.push_str(&format!(
            "\nInert law file(s) — the language they govern matches no analyzed file, so their \
             rules did not run: {}.\n",
            inert.join(", ")
        ));
    }
    // Knowledge never vanishes silently: name every language whose docs resolved to nothing
    // this run (offline with a cold page cache, or no sources known) — project law still
    // applies there, and the next online run retries the docs.
    // Prose languages (md/txt, man sections) are reading material with no doc-learning path —
    // "run once online to learn" would be a false promise for them, so they are not listed.
    let unlearned: std::collections::BTreeSet<&str> = report
        .unlearned
        .iter()
        .map(String::as_str)
        .filter(|l| !crate::lint_match::prose_lang(l))
        .collect();
    if !unlearned.is_empty() {
        let names = unlearned.into_iter().collect::<Vec<_>>().join(", ");
        // A lint run is replay-only — it never sets anything up, it ASKS for the documentation
        // (LINTER.md, "Lint never touches the network"): the user — far more often the agent
        // acting for them — answers with a URL. One message, two verbs, nothing else.
        body.push_str(&format!(
            "\nNot yet set up (project law still enforced there): {names}. Everything else \
             was fully checked. Hand me each unknown language's official documentation \
             (`lint_config action=add_source lang=<language> url=<docs URL>`), then run \
             `lint_config action=train` — setup needs internet, linting never does.\n"
        ));
    }
    body.push_str(&render_quarantine(&quarantined));
    body.push_str(&render_feedback(&root, &auto_suppressed));
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

/// Collapse a file's hits into readable lines: one per distinct rule, carrying the advice once and
/// the lines it occurred on (capped), highest-severity first.
fn group_hits(hits: &[Hit]) -> Vec<String> {
    let mut groups: Vec<(String, String, String, String, Vec<usize>)> = Vec::new(); // (rule, sev, advice, source, lines)
    for h in hits {
        let advice = if h.advice.is_empty() { format!("violates `{}`", h.rule) } else { h.advice.clone() };
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
            format!("[{sev}] [{rule}] {advice}  {occ}{cite}")
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
) -> String {
    let mut s = String::new();
    let analyzed: usize = by_language.values().sum();
    let langs: Vec<String> = by_language.iter().map(|(l, n)| format!("{l} ({n})")).collect();
    s.push_str(&format!(
        "I read {} and analyzed {analyzed} source file(s): {}.\n\n",
        root.display(),
        if langs.is_empty() { "none".to_string() } else { langs.join(", ") }
    ));

    let total: usize = reports.iter().map(|f| f.hits.len()).sum();
    if total == 0 {
        s.push_str("Verdict: CLEAN. No violations of the learned rules or the project's law.\n");
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
            "Verdict: {total} issue(s) across {} of {analyzed} file(s) — {hi} high, {me} medium, {lo} low.\n",
            reports.len()
        ));
        let mut shown = 0usize;
        for f in reports {
            if shown >= max { break; }
            s.push_str(&format!("\n{}\n", f.path));
            for line in group_hits(&f.hits) {
                if shown >= max {
                    s.push_str("  …raise `max` to see more.\n");
                    break;
                }
                s.push_str(&format!("  {line}\n"));
                shown += 1;
            }
        }
    }

    if !unanalyzed.is_empty() {
        let u: Vec<String> = unanalyzed.iter().map(|(l, n)| format!("{l} ({n})")).collect();
        s.push_str(&format!("\nLanguages without AST support (not analyzed): {}.\n", u.join(", ")));
    }

    if !sources.is_empty() {
        s.push_str(&format!("\nTrained from: {}.\n", sources.join(", ")));
    }
    s
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
    for f in walk_repo(root) {
        if law.contains(&f.abs) {
            continue;
        }
        let ext = f.ext.to_lowercase();
        if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let lang = file_lang(&ext).map(str::to_string).unwrap_or(ext);
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
        let lines = group_hits(&hits);
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
