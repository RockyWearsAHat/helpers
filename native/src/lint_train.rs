//! `lint_train` — self-setup for the AI linter: reads two documentation sources and compiles, per
//! language, the [`LangModel`] a lint run needs — the [`RuleSet`] firing engine and the
//! [`ConceptModel`] confirmation gate. One call to [`ensure_models`] does everything the lint
//! tool needs.
//!
//! The two documentation sources:
//!
//!   1. **Official web documentation** — the official rule docs for each language (clippy / ruff /
//!      eslint / staticcheck / pmd). The linter crawls the live docs and caches what it learns;
//!      a committed `lint-index/` snapshot seeds the offline case.
//!   2. **File documentation** — `extraDocs/` (global principles, shipped with the tool) and the
//!      project's own law: every `*.md`/`*.txt` under `.helpers/lint-rules/` plus a root-level
//!      `lintPref.md`/`lintPref.txt`. Each file is READ ([`crate::linter::Knowledge::read_document`])
//!      — plain English, no required format: examples become rules; prose-only instructions become
//!      description-derived rules.
//!
//! Rules with bad/good examples compile to lossless AST patterns (or discriminating token regexes
//! for grammarless languages) in [`RuleSet`]; the same rules' description + example tokens bundle
//! into concept fingerprints in [`ConceptModel`]. The pattern set is cached to disk with a SHA-256
//! stamp; the concept gate is rebuilt in memory each run (cheap: hashing + popcount).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::lint_ai::ConceptModel;
use crate::lint_match::RuleSet;

/// A trained model for one language: the pattern rule set (the firing engine) and the Hv concept
/// gate (confirms imprecise text-fallback findings). Both train from the same two sources;
/// [`ensure_models`] builds them together so the lint tool makes one call and gets everything.
pub struct LangModel {
    /// Pattern-matching rules compiled from documentation bad/good examples — the firing engine.
    pub rules: RuleSet,
    /// Concept fingerprints for the same rules; the gate that confirms text-fallback findings.
    pub concept: ConceptModel,
}

/// How many doc pages to crawl per source when learning a language whose docs are a site. High
/// enough to read a broad slice of the reference (and, with the reader grounding each example against
/// the toolchain, a rich polarity signal), bounded so the one cold crawl stays inside the tool's time
/// budget; the learned catalog is cached, so the cost is paid once per toolchain version.
#[cfg(feature = "crawl")]
const MAX_CRAWL_PAGES: usize = 700;

/// Bump when the training logic changes so existing caches are treated as stale and relearned.
const TRAIN_VERSION: &str = "docs-v10-grounded-shape-free-selection";

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
/// preferred so editing it relearns on the next run). Points to the actual course principles
/// document (prose-only; pattern rules come from committed modules and crawled official docs).
const EMBEDDED_CS_PRINCIPLES: &str = include_str!("../../extraDocs/software-design.md");

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

/// A language's learned catalog, cached so the linter does not relearn every run. Keyed by the
/// toolchain `version` it was learned for, so a version bump triggers a fresh crawl ("stay current").
///
/// v2 catalogs carry the reader's association [`crate::lint_read::Memory`] — what the model actually
/// read — and rules are QUERIED out of it at train time ([`crate::lint_docs::rules_from_memory`]).
/// The `rules` field remains for older committed modules that shipped pre-extracted tuples; a catalog
/// uses one or the other ([`LearnedCatalog::doc_rules`]).
#[derive(Serialize, Deserialize)]
struct LearnedCatalog {
    /// Toolchain version the rules were learned for (empty when undetectable).
    version: String,
    /// [`TRAIN_VERSION`] the catalog was read under. A user-cache catalog from older reading
    /// logic is stale and relearned; committed modules are deliberate shared seeds and are
    /// exempt (they load whatever their version, like the snapshot).
    #[serde(default)]
    train_version: String,
    /// Where the rules came from (a tool name, `committed`, or `embedded`) — provenance.
    learned_from: String,
    /// Pre-extracted rules (older committed-module form). Empty for v2 memory catalogs.
    #[serde(default)]
    rules: Vec<DocRule>,
    /// Real idiomatic code the docs served (older form; v2 keeps it inside `memory`).
    #[serde(default)]
    reference: Vec<String>,
    /// v2: the association memory the reader built from the docs — bindings, reference corpus, and
    /// the toolchain-grounded polarity classifier. Rules are a query against this.
    #[serde(default)]
    memory: Option<crate::lint_read::Memory>,
}

impl LearnedCatalog {
    /// Whether this user-cache catalog is current: read under today's [`TRAIN_VERSION`] and for
    /// the detected `toolchain` version. A catalog failing either is relearned, so a reading-logic
    /// change refreshes cached knowledge exactly like a toolchain bump does.
    fn current(&self, toolchain: &str) -> bool {
        self.train_version == TRAIN_VERSION && self.version == toolchain
    }

    /// The catalog's rules and reference corpus: queried from the association memory when present
    /// (reading IS the knowledge), else the pre-extracted tuples an older module shipped.
    fn doc_rules(&self, lang: &str) -> (Vec<DocRule>, Vec<String>) {
        match &self.memory {
            Some(memory) => {
                let rules = crate::lint_docs::rules_from_memory(lang, memory)
                    .into_iter()
                    .map(|(r, url)| DocRule {
                        id: r.id,
                        slice: r.severity.clone(),
                        severity: r.severity,
                        description: r.description,
                        bad: r.bad,
                        good: r.good,
                        source: url,
                    })
                    .collect();
                (rules, memory.reference.clone())
            }
            None => (self.rules.clone(), self.reference.clone()),
        }
    }
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
    /// Project-authored rules that could NOT compile a detector, as `(language, rule id)`.
    /// The user's law must never vanish silently — the lint report surfaces these with what
    /// to do about it (add a token-distinctive example, or run online so the language's
    /// grammar/docs can be learned).
    pub unenforced: Vec<(String, String)>,
}

/// Ensure a fresh, cached compiled [`RuleSet`] exists for each requested language, learning from the
/// docs + the corpus folder only. Idempotent and checksum-gated: a language whose resolved rules and
/// toolchain version are unchanged is reused, not relearned. `data_root` holds `lint-index/` (the
/// seed) and `corpus/` (the folder); missing on-disk sources fall back to the embedded copies, and a
/// stale/absent catalog is relearned from the live docs when the crawler is available. Each rule is
/// compiled to its exact tree pattern from its own bad/good example — no thresholds, no statistics —
/// so a match is the rule's structure occurring verbatim, with scope and co-reference intact.
/// Every document the project wrote its law in, as `(path, default language)`: all `*.md`/`*.txt`
/// under `.helpers/lint-rules/` (the filename stem is the language its rules default to; `any`
/// means every language) plus a root-level `lintPref.md`/`lintPref.txt` (any casing, `-`/`_`
/// allowed), whose rules default to every language. Plain English throughout — the files are
/// READ, never parsed against a required format.
/// Whether `lang` is a DOCUMENTATION format — the same formats [`rule_documents`] reads law and
/// teaching material from (`md`/`txt` and their canonical names). Documents are what the linter
/// READS; code is what it LINTS: a rule whose language is `any` governs every CODE language, but
/// never the prose formats themselves — otherwise every design doc that discusses a rule's
/// subject gets flagged by it. Rules written FOR a documentation format (a project `md.md`, a
/// crawled markdown spec) still apply to it.
pub(crate) fn is_document_language(lang: &str) -> bool {
    matches!(lang, "md" | "markdown" | "txt" | "text")
}

pub(crate) fn rule_documents(project_root: &Path) -> Vec<(PathBuf, String)> {
    let is_text = |p: &Path| matches!(p.extension().and_then(|x| x.to_str()), Some("md" | "txt"));
    let mut docs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(project_root.join(".helpers/lint-rules")) {
        for e in entries.flatten() {
            let p = e.path();
            if is_text(&p) {
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("any").to_lowercase();
                docs.push((p, stem));
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(project_root) {
        for e in entries.flatten() {
            let p = e.path();
            if !is_text(&p) {
                continue;
            }
            let stem: String = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            if stem == "lintpref" || stem == "lintprefs" {
                docs.push((p, "any".to_string()));
            }
        }
    }
    docs.sort();
    docs
}

/// Load the project's own rules that govern `lang`: every rule read from [`rule_documents`] whose
/// language is `lang` or `any`. Project rules are merged BEFORE the global corpus and the crawled
/// docs, so they take priority over both.
pub(crate) fn project_rules(data_root: &Path, project_root: &Path, lang: &str) -> Vec<DocRule> {
    let polarity = crate::lint_docs::document_polarity(data_root);
    let mut out = Vec::new();
    let allow_any = !is_document_language(lang);
    for (path, default_lang) in rule_documents(project_root) {
        let Ok(doc) = std::fs::read_to_string(&path) else { continue };
        let source = path.to_string_lossy().into_owned();
        for r in crate::linter::Knowledge::read_document(&default_lang, &doc, polarity.as_ref()).rules {
            let any = r.language == "any" || r.language.is_empty();
            if !(r.language == lang || (any && allow_any)) {
                continue;
            }
            // Prose-only rules (no bad example) are valid: the pattern is derived from the
            // English description.
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
/// Prohibition prose the tool can label HONESTLY, with no authored word list: a CURATED catalog
/// rule that SHOWS a bad example structurally documents a violation, so its description states
/// one — the label is the catalog's own shape, not anyone's vocabulary. Only the shipped
/// `extraDocs/lint-corpus.jsonl` qualifies: it is a catalog of real linter rules. The corpus
/// FOLDER's markdown is teaching material — its fences read as "bad examples" only by document
/// order, so seeding its sections as prohibitions is a mislabel that taught earlier classifiers
/// that ordinary teaching vocabulary means bad. Rules without a bad example ("Apply De Morgan's
/// law") are suggestions and seed nothing. Toolchain grounding supplies the endorsement side and
/// keeps growing both.
pub(crate) fn corpus_prohibition_prose(data_root: &Path) -> Vec<String> {
    let Ok(jsonl) = std::fs::read_to_string(data_root.join("extraDocs/lint-corpus.jsonl")) else {
        return Vec::new();
    };
    jsonl
        .lines()
        .filter_map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).ok()?;
            if v["bad"].as_str().unwrap_or("").trim().is_empty() {
                return None;
            }
            let d = v["description"].as_str()?;
            (d.split_whitespace().count() >= 3).then(|| d.to_string())
        })
        .collect()
}

/// The model cache directory — exposed for sibling modules that persist shared learned
/// artifacts beside the per-language models (e.g. the transferred polarity classifier).
pub(crate) fn model_dir_pub() -> PathBuf {
    model_dir()
}

/// A file from the committed/embedded `lint-index/` data (on-disk copy preferred) — the shape
/// every shipped knowledge artifact is loaded in.
pub(crate) fn lint_index_file(data_root: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(data_root.join("lint-index").join(name))
        .ok()
        .or_else(|| {
            EMBEDDED_LINT_INDEX
                .get_file(name)
                .and_then(|f| f.contents_utf8().map(str::to_string))
        })
}

pub fn stamp_path_pub(lang: &str) -> PathBuf {
    stamp_path(lang)
}

/// The ids of the rules the project itself authored (`.helpers/lint-rules/`, root `lintPref`) for
/// `lang`.
///
/// These are the user's explicit law for their own codebase, so the live path trusts them fully:
/// they are never routed through the Hv concept gate the way learned doc rules with weak
/// (container-only) anchors are.
pub fn project_rule_ids(data_root: &Path, project_root: &Path, lang: &str) -> std::collections::HashSet<String> {
    project_rules(data_root, project_root, lang).into_iter().map(|r| r.id).collect()
}

/// Train from both documentation sources and return one [`LangModel`] per language. Idempotent
/// and checksum-gated: a language whose pattern rules and toolchain version are unchanged reloads
/// from cache; behavioral principles are re-extracted each run (fast file reads, no compilation).
///
/// Source 1 — **official web documentation**: crawled or seeded from `lint-index/`; cached,
/// version-keyed so a toolchain bump triggers a fresh crawl.
/// Source 2 — **file documentation**: `corpus/` (global CS principles) and `.helpers/lint-rules/`
/// (project-local rules). Both feeds BOTH engines: bad/good examples → pattern rules; structural
/// prose → behavioral principles.
/// `project_code` is the project's own sources per language (`lang → file bodies`) — the
/// grounding evidence for construct selection: a law names constructs that live in the code it
/// governs, so the project itself is the one corpus that is ALWAYS available, in any language,
/// with no shapes assumed. Pass an empty map when compiling rules with no project in hand.
pub fn ensure_models(
    langs: &[String],
    data_root: &Path,
    project_root: &Path,
    project_code: &std::collections::BTreeMap<String, Vec<String>>,
) -> (TrainReport, HashMap<String, LangModel>) {
    let mut report = TrainReport::default();
    let mut models = HashMap::new();
    let folder = corpus_rules(data_root);

    for lang in langs {
        let version = crate::lint_checkers::detect_version(lang).unwrap_or_default();
        // Trust order decides who wins a shared pattern signature at dedup time
        // (RuleSet::build keeps the FIRST rule per pattern): the project's own law first,
        // then the corpus folder's principles, then the crawled docs.
        let (doc_rules, reference, learned_from) = resolve_rules(data_root, lang, &version, &mut report);
        let mut rules = project_rules(data_root, project_root, lang);
        // The project's own law is trusted by LOCATION: the user wrote it in a rule file, so it
        // states a violation by construction and bypasses the prohibition gate at compile time.
        let trusted: std::collections::HashSet<String> = rules.iter().map(|r| r.id.clone()).collect();
        // `any` rules govern every CODE language; documentation formats get only rules written
        // FOR them ([`is_document_language`]) — documents are read, not held to code law.
        let allow_any = !is_document_language(lang);
        rules.extend(
            folder.iter()
                .filter(|(l, _)| l == lang || (allow_any && (l == "any" || l.is_empty())))
                .map(|(_, r)| r.clone()),
        );
        rules.extend(doc_rules);

        if rules.is_empty() {
            report.skipped.push((lang.clone(), "no rules found for this language".to_string()));
            continue;
        }

        // Concept fingerprints for the gate — built in memory from the same resolved rules
        // (id, description, example). Cheap (hash + popcount), so it is not cached to disk.
        let concept_tuples: Vec<(String, String, String)> = rules
            .iter()
            .map(|r| (r.id.clone(), r.description.clone(), r.bad.clone()))
            .collect();
        let concept = ConceptModel::compile(&concept_tuples, lang);

        // Grounding = the docs' reference corpus (grounds learned rules) + the project's OWN
        // code (grounds and ranks the project's law). The compiled detectors depend on this
        // evidence, so its fingerprint is part of the model cache stamp.
        let ground = crate::lint_match::Grounding {
            reference,
            project: project_code.get(lang).cloned().unwrap_or_default(),
            polarity: crate::lint_docs::document_polarity(data_root),
            trusted: trusted.clone(),
        };
        let stamp = stamp_of(
            &version,
            &rules,
            ground_fingerprint(&ground.reference) ^ ground_fingerprint(&ground.project).rotate_left(1),
        );

        // Every trusted (project-authored) id that did not survive compilation is REPORTED —
        // the user's law never vanishes silently.
        let note_unenforced =
            |report: &mut TrainReport, compiled: &std::collections::HashSet<String>| {
                for id in trusted.iter().filter(|id| !compiled.contains(*id)) {
                    report.unenforced.push((lang.clone(), id.clone()));
                }
            };

        // Fast path: pattern model already cached and current — load it, attach the concept gate.
        if model_fresh(&patterns_path(lang), &stamp_path(lang), &stamp) {
            if let Some(rule_set) = load_patterns(lang) {
                let compiled: std::collections::HashSet<String> =
                    rule_set.rule_ids().map(str::to_string).collect();
                note_unenforced(&mut report, &compiled);
                models.insert(lang.clone(), LangModel { rules: rule_set, concept });
            }
            report.reused.push(lang.clone());
            continue;
        }

        // Build and cache the pattern model from Source 1 + Source 2 rules. Prose-only rules are
        // read through the language's learned grounding: real code (docs + project) and the
        // transferred polarity classifier (whose reader knows the docs' common words).
        let tuples: Vec<(String, String, String, String, String)> = rules
            .iter()
            .map(|r| (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone()))
            .collect();
        let rule_set = RuleSet::build(lang, &tuples, &ground);
        let compiled: std::collections::HashSet<String> =
            rule_set.rule_ids().map(str::to_string).collect();
        note_unenforced(&mut report, &compiled);

        if rule_set.rule_count() == 0 {
            report.skipped.push((lang.clone(), "no rule carried a distinctive pattern to match".to_string()));
            continue;
        }

        if let Some(parent) = patterns_path(lang).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(patterns_path(lang), rule_set.to_json()).is_ok() {
            let _ = std::fs::write(stamp_path(lang), &stamp);
            report.trained.push(format!("{lang} ({} rules, from {learned_from})", rule_set.rule_count()));
        } else {
            report.skipped.push((lang.clone(), "could not write the cached model".to_string()));
            continue;
        }

        models.insert(lang.clone(), LangModel { rules: rule_set, concept });
    }

    (report, models)
}

/// Order-independent fingerprint of the grounding corpus (docs reference + project code).
/// Construct selection reads descriptions through this evidence, so a grounding change must
/// retrain the cached model exactly like a rule edit does.
fn ground_fingerprint(reference: &[String]) -> u64 {
    reference.iter().map(|s| crate::lint_ai::token_seed(s)).fold(0u64, |acc, h| acc ^ h)
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
            if cat.current(version) {
                let (rules, reference) = cat.doc_rules(lang);
                if !rules.is_empty() {
                    return (rules, reference, format!("cache:{}", cat.learned_from));
                }
            }
        }
        // A committed module is a high-quality seed (a read memory, or pre-extracted pairs). It is
        // used regardless of toolchain version — like the snapshot, it is a starting point that an
        // explicit `HELPERS_LINT_REFRESH` re-crawls. Preferred over the bare `lint-index/` snapshot.
        if let Some(cat) = load_committed_module(data_root, lang) {
            let (mod_rules, reference) = cat.doc_rules(lang);
            if !mod_rules.is_empty() {
                let (seed_rules, _) = seed_with_version(data_root, lang);
                let existing: std::collections::HashSet<String> =
                    mod_rules.iter().map(|r| r.id.clone()).collect();
                let mut rules = mod_rules;
                rules.extend(seed_rules.into_iter().filter(|r| !existing.contains(&r.id)));
                return (rules, reference, "committed module".to_string());
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
    // READ it ourselves from the live docs. Cache the MEMORY we read (not pre-extracted rules),
    // keyed by the toolchain version, so the next run queries the same reading and only re-reads on
    // a version bump.
    if let Some(memory) = crawl_learn(data_root, lang, version) {
        let cat = LearnedCatalog {
            version: version.to_string(),
            train_version: TRAIN_VERSION.to_string(),
            learned_from: "docs".to_string(),
            rules: Vec::new(),
            reference: Vec::new(),
            memory: Some(memory),
        };
        let (rules, reference) = cat.doc_rules(lang);
        if !rules.is_empty() {
            report.crawled.push(lang.to_string());
            save_cache(lang, &cat);
            return (rules, reference, "live docs".to_string());
        }
    }
    // Offline or crawl-disabled: fall back to the snapshot (stale is better than nothing).
    if !seed.is_empty() {
        return (seed, Vec::new(), "committed snapshot".to_string());
    }
    (Vec::new(), Vec::new(), "nothing".to_string())
}

/// READ `lang`'s official language documentation into an association [`crate::lint_read::Memory`].
/// A language may have several registered documents (reference + style guide); ALL are read into one
/// memory, grounded once against the installed toolchain. A language in no registry is discovered on
/// the fly ([`crate::lint_docs::discover_docs`]). `None` when nothing could be read (offline, no
/// sources, empty read) or the crawler is not compiled in.
#[cfg(feature = "crawl")]
fn crawl_learn(data_root: &Path, lang: &str, _version: &str) -> Option<crate::lint_read::Memory> {
    // Operational escape hatch: skip all network learning (air-gapped runs, and deterministic
    // tests) — the resolver then uses the committed/embedded seed instead.
    if std::env::var_os("HELPERS_LINT_OFFLINE").is_some() {
        return None;
    }
    let mut sources = crawl_sources_from_config(data_root, lang);
    if sources.is_empty() {
        sources.extend(crate::lint_docs::discover_docs(lang, data_root));
    }
    if sources.is_empty() {
        return None;
    }
    let memory = crate::lint_docs::read_language(lang, &sources, MAX_CRAWL_PAGES, data_root);
    (!memory.bindings.is_empty()).then_some(memory)
}

/// Every registered docs source for `lang` from `sources.json` (on-disk preferred, embedded
/// fallback) — a language may list several official documents and all of them are learned.
/// `kind:"crawl"` uses `seed`; `kind:"agent"` uses `docsBase` as a best-effort crawl target.
#[cfg(feature = "crawl")]
fn crawl_sources_from_config(data_root: &Path, lang: &str) -> Vec<crate::lint_docs::DocsSource> {
    let Some(raw) = std::fs::read_to_string(data_root.join("lint-index/sources.json"))
        .ok()
        .or_else(|| {
            EMBEDDED_LINT_INDEX
                .get_file("sources.json")
                .and_then(|f| f.contents_utf8().map(str::to_string))
        })
    else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    let Some(entries) = json["sources"].as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries {
        if !lang_matches(entry["language"].as_str().unwrap_or(""), lang) {
            continue;
        }
        let tool = entry["tool"].as_str().unwrap_or("").to_string();
        let url = match entry["kind"].as_str() {
            Some("crawl") => entry["seed"].as_str(),
            Some("agent") => entry["docsBase"].as_str(),
            _ => None,
        };
        if let Some(url) = url {
            out.push(crate::lint_docs::DocsSource { url: url.to_string(), crawl: true, tool });
        }
    }
    out
}

#[cfg(not(feature = "crawl"))]
fn crawl_learn(_data_root: &Path, _lang: &str, _version: &str) -> Option<crate::lint_read::Memory> {
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

/// User-local cache for lint rule catalogs fetched from official docs at training time.
/// Catalogs are generated, not committed; this directory holds the generated artifacts.
fn lint_index_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Path::new(&home).join(".cache/helpers/lint-index")
}

/// Read all catalog JSON files from `dir`, skipping `sources.json` and non-JSON files.
fn load_catalog_dir(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            if is_catalog_name(entry.path().file_name().and_then(|n| n.to_str())) {
                if let Ok(s) = std::fs::read_to_string(entry.path()) {
                    out.push(s);
                }
            }
        }
    }
    out
}

/// The raw JSON of every available rule catalog, in order of freshness:
///   1. workspace `lint-index/` — present in dev checkout (gitignored, not committed);
///   2. user cache `~/.cache/helpers/lint-index/` — written when the native crawler learns;
///   3. embedded fallback. All learning is the native crawler ([`crawl_learn`]); there is no
///      external scraper to shell out to.
fn seed_catalogs(data_root: &Path) -> Vec<String> {
    let mut out = load_catalog_dir(&data_root.join("lint-index"));
    if out.is_empty() { out = load_catalog_dir(&lint_index_cache_dir()); }
    if out.is_empty() {
        for f in EMBEDDED_LINT_INDEX.files() {
            if is_catalog_name(f.path().file_name().and_then(|n| n.to_str())) {
                if let Some(s) = f.contents_utf8() { out.push(s.to_string()); }
            }
        }
    }
    out
}

/// A `lint-index` entry is a rule catalog if it is a `*.json` and not the source registry.
fn is_catalog_name(name: Option<&str>) -> bool {
    matches!(name, Some(n) if n.ends_with(".json") && n != "sources.json")
}

/// The corpus folder rules as `(language, DocRule)` — the second knowledge source. READS every
/// `*.md`/`*.txt` file in `extraDocs/` so adding a new principles document takes effect
/// immediately on the next lint run. Falls back to the embedded principles doc when the directory
/// is absent.
fn corpus_rules(data_root: &Path) -> Vec<(String, DocRule)> {
    let corpus_dir = data_root.join("extraDocs");
    let docs: Vec<(String, String)> = match std::fs::read_dir(&corpus_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| matches!(e.path().extension().and_then(|x| x.to_str()), Some("md" | "txt")))
            .filter_map(|e| {
                let path = e.path();
                let name = path.to_string_lossy().into_owned();
                std::fs::read_to_string(&path).ok().map(|text| (name, text))
            })
            .collect(),
        Err(_) => vec![("embedded".to_string(), EMBEDDED_CS_PRINCIPLES.to_string())],
    };
    let polarity = crate::lint_docs::document_polarity(data_root);
    docs.into_iter()
        .flat_map(|(source, doc)| {
            crate::linter::Knowledge::read_document("any", &doc, polarity.as_ref())
                .rules
                .into_iter()
                // Prose-only rules (no bad example) are valid: pattern comes from description.
                .map(move |r| {
                    (
                        r.language.clone(),
                        DocRule {
                            id: r.id,
                            slice: "cs-principle".to_string(),
                            severity: r.severity,
                            description: r.description,
                            bad: r.bad,
                            good: r.good,
                            source: source.clone(),
                        },
                    )
                })
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
    /// Fold every `<lang>.learned.json` catalog under `dir` into the advice map — the language is
    /// the filename stem, and v2 memory catalogs are queried the same way training queries them.
    fn put_catalogs(out: &mut HashMap<String, RuleInfo>, dir: &Path) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let Some(lang) = name.strip_suffix(".learned.json") else { continue };
            let Ok(s) = std::fs::read_to_string(&p) else { continue };
            let Ok(cat) = serde_json::from_str::<LearnedCatalog>(&s) else { continue };
            for r in cat.doc_rules(lang).0 {
                put(out, &r);
            }
        }
    }
    // Committed modules override the bare seed (they carry full descriptions + sources), and
    // anything the linter learned itself and cached overrides both (it is more current).
    put_catalogs(&mut out, &committed_modules_dir(data_root));
    put_catalogs(&mut out, &model_dir());
    // Folder rules (the CS principles).
    for (_, r) in corpus_rules(data_root) {
        put(&mut out, &r);
    }
    // Project-local rules (`.helpers/lint-rules/`, root `lintPref`) — highest priority; their
    // descriptions override everything else so a user's custom advice appears verbatim in lint
    // output.
    if let Some(pr) = project_root {
        let polarity = crate::lint_docs::document_polarity(data_root);
        for (path, default_lang) in rule_documents(pr) {
            if let Ok(doc) = std::fs::read_to_string(&path) {
                let src = path.to_string_lossy().into_owned();
                for r in crate::linter::Knowledge::read_document(&default_lang, &doc, polarity.as_ref()).rules {
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
    let memory = crawl_learn(data_root, lang, &version).ok_or_else(|| {
        format!(
            "no docs URL configured for `{lang}` — add one with `lint_add_source` first, \
             or set HELPERS_LINT_OFFLINE to use a committed module"
        )
    })?;
    let catalog = LearnedCatalog {
        version: version.clone(),
        train_version: TRAIN_VERSION.to_string(),
        learned_from: "docs".to_string(),
        rules: Vec::new(),
        reference: Vec::new(),
        memory: Some(memory),
    };
    let (rules, reference) = catalog.doc_rules(lang);
    if rules.is_empty() {
        return Err(format!("read docs for `{lang}` but no binding classified as a violation"));
    }
    let rule_count = rules.len();
    // Save to user cache.
    save_cache(lang, &catalog);
    // Compile the pattern model and cache it, grounded in what was just read: the crawl's own
    // reference code and its polarity classifier (falling back to the transferred one).
    let tuples: Vec<(String, String, String, String, String)> = rules
        .iter()
        .map(|r| (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone()))
        .collect();
    let ground = crate::lint_match::Grounding {
        reference,
        project: Vec::new(),
        polarity: catalog
            .memory
            .as_ref()
            .and_then(|m| m.polarity.clone())
            .or_else(|| crate::lint_docs::document_polarity(data_root)),
        trusted: std::collections::HashSet::new(),
    };
    let stamp = stamp_of(&version, &rules, ground_fingerprint(&ground.reference));
    let model = crate::lint_match::RuleSet::build(lang, &tuples, &ground);
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

/// A stable checksum of a language's resolved rules + toolchain version + grounding fingerprint —
/// the model cache key. Order-independent (rows are sorted) and salted with [`TRAIN_VERSION`].
/// The description is part of the row: patterns can be derived from the English prose alone, so
/// editing a rule's wording must retrain the model exactly like editing its examples does.
fn stamp_of(version: &str, rules: &[DocRule], ground_fp: u64) -> String {
    let mut rows: Vec<String> = rules
        .iter()
        .map(|r| format!("{}\u{1f}{}\u{1f}{}\u{1f}{}", r.id, r.bad, r.good, r.description))
        .collect();
    rows.sort();
    let mut h = Sha256::new();
    h.update(TRAIN_VERSION.as_bytes());
    h.update(version.as_bytes());
    h.update(ground_fp.to_le_bytes());
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

    #[test]
    fn any_rules_govern_code_languages_but_not_documentation_formats() {
        assert!(!is_document_language("python"));
        assert!(!is_document_language("qlang"), "an unknown code-like language is governed by `any`");
        assert!(is_document_language("md"));
        assert!(is_document_language("markdown"));
        assert!(is_document_language("txt"));
    }

    #[test]
    fn a_cached_catalog_from_older_reading_logic_is_stale() {
        let cat = |train_version: &str| LearnedCatalog {
            version: "1.0".to_string(),
            train_version: train_version.to_string(),
            learned_from: "docs".to_string(),
            rules: Vec::new(),
            reference: Vec::new(),
            memory: None,
        };
        assert!(cat(TRAIN_VERSION).current("1.0"), "same reading logic + toolchain → fresh");
        assert!(!cat("").current("1.0"), "a pre-versioning catalog must relearn");
        assert!(!cat("docs-v1-ancient").current("1.0"), "older reading logic must relearn");
        assert!(!cat(TRAIN_VERSION).current("2.0"), "a toolchain bump still relearns");
    }
}

