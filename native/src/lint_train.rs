//! `lint_train` — self-setup for the AI linter: reads two documentation sources and compiles, per
//! language, the [`LangModel`] a lint run needs — the [`RuleSet`] firing engine and the
//! [`ConceptModel`] confirmation gate. One call to [`ensure_models`] does everything the lint
//! tool needs.
//!
//! Cross-module theory, evidence hierarchy, and the failure ledger live in `LINTER.md` at the
//! repo root — the single authoritative doc; update it BEFORE changing semantics here.
//!
//! Law comes from exactly two places; everything else is READING:
//!
//!   1. **Official web documentation** — each language's official docs, crawled live and cached
//!      (a committed `lint-index/` snapshot seeds the offline case).
//!   2. **The project's own law** — every `*.md`/`*.txt` under `.helpers/lint-rules/` plus a
//!      root-level `lintPref.md`/`lintPref.txt`, READ as plain English
//!      ([`crate::linter::Knowledge::read_document`]) with no required format.
//!
//! There is no curated rule catalog: enforcement grows purely from READING — the linter reads
//! official docs and the project's law, understands them, and enforces what they forbid, with
//! every reading grounded against the installed toolchain. Never authored, never hand-coded.
//! `extraDocs/` prose and the registered reading sources are teaching material: they build the
//! understanding that reading happens through, but only a statement of a violation becomes law.
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

/// Pages to crawl per source — a runaway safety valve (a mis-scoped seed must not eat a whole
/// wiki), never a working limit: the WHOLE in-scope docs tree is crawled and read (LINTER.md,
/// "Map"). The learned catalog is cached and registry-shared, so the cost is paid once per
/// machine per toolchain version.
#[cfg(feature = "crawl")]
const MAX_CRAWL_PAGES: usize = 20_000;

/// Bump when the training logic changes so existing caches are treated as stale and relearned.
const TRAIN_VERSION: &str = "docs-v45-abbreviations-abbreviate";

/// Process latch: network acquisition (registry pull, crawl, discovery, grammar download) is
/// allowed only when a SETUP verb set it — `lint_config action=train` and nothing else. A lint
/// run never sets it, so linting is replay-only by construction (LINTER.md, "Lint never
/// touches the network"). `HELPERS_LINT_OFFLINE` keeps setup off the real network in the
/// hermetic contract tests.
static NETWORK_SETUP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Enter setup mode: network acquisition is allowed for the rest of this process.
pub fn allow_network_setup() {
    NETWORK_SETUP.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether this process may touch the network for acquisition — setup mode, minus the
/// hermetic test switch.
pub(crate) fn network_allowed() -> bool {
    NETWORK_SETUP.load(std::sync::atomic::Ordering::Relaxed)
        && std::env::var_os("HELPERS_LINT_OFFLINE").is_none()
}

/// The current [`TRAIN_VERSION`] — public so feedback flags can be version-scoped
/// ([`crate::lint_feedback`]): a suppression earned under one training version must not
/// silently carry into the next.
pub fn train_version() -> &'static str {
    TRAIN_VERSION
}

/// The committed rule catalogs, embedded so an installed binary far from the checkout still has a
/// documentation seed to learn from offline. The live crawl (when reachable) and the on-disk
/// `lint-index/` are both preferred over this.
static EMBEDDED_LINT_INDEX: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../lint-index");

/// The CS principles folder document, embedded as the offline fallback (the on-disk copy is
/// preferred so editing it relearns on the next run). Points to the actual course principles
/// document (prose-only reading material; pattern rules come from crawled official docs).
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
    /// Fingerprint of the SOURCE SET the catalog was read from (sorted seed URLs, hashed).
    /// Adding or changing a docs source re-reads the language instead of reusing a catalog
    /// that never saw the new source — locally and through the registry alike. Serialized
    /// early so the prefix probe and the registry publisher read it without a full parse.
    #[serde(default)]
    sources_fp: String,
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
    fn current(&self, toolchain: &str, sources_fp: &str) -> bool {
        self.train_version == TRAIN_VERSION
            && self.version == toolchain
            && self.sources_fp == sources_fp
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
    /// Languages whose official docs resolved to NOTHING this run (offline with a cold page
    /// cache, or no sources known) — enforcing project law only. Knowledge, like law, never
    /// vanishes silently: the report names them, and the next online run retries.
    pub unlearned: Vec<String>,
    /// Languages whose learned catalog was downloaded from the GitHub model registry this run.
    pub pulled: Vec<String>,
    /// A network request failed at the TRANSPORT level this run (or the hermetic
    /// `HELPERS_LINT_OFFLINE` switch simulated it): whatever is `unlearned` stayed that way
    /// because the wire was down, so the report asks to reconnect instead of to rephrase.
    pub net_down: bool,
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
    crate::lint_match::prose_lang(lang)
}

pub(crate) fn rule_documents(project_root: &Path) -> Vec<(PathBuf, String)> {
    let is_text = |p: &Path| matches!(p.extension().and_then(|x| x.to_str()), Some("md" | "txt"));
    let mut docs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(project_root.join(".helpers/lint-rules")) {
        for e in entries.flatten() {
            let p = e.path();
            if is_text(&p) {
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("any").to_lowercase();
                // `js.md` means JavaScript: extension aliases resolve through the same map the
                // file walker detects languages with, or the law governs nothing (ledger #16).
                let stem = resolve_language(&stem);
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
    rules_in_documents(&rule_documents(project_root), data_root, lang, "project-rule")
}

/// The machine-global CS-principles rule documents: `<data_root>/corpus/*.{md,txt}`. A stem
/// naming a language (aliases resolve like rule-file stems do) scopes the file; any other stem
/// (`cs-principles`) means every code language. These are DATA — the CS2420/CS3500 canon lives
/// in files, never in code — and they are GATED like learned rules (prohibition-sentence entry,
/// grounding, self/reference-fire, quarantine): global principles must earn each firing; they
/// are not the user's own law.
fn corpus_documents(data_root: &Path) -> Vec<(PathBuf, String)> {
    let mut docs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(data_root.join("corpus")) {
        for e in entries.flatten() {
            let p = e.path();
            if !matches!(p.extension().and_then(|x| x.to_str()), Some("md" | "txt")) {
                continue;
            }
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("any").to_lowercase();
            let resolved = resolve_language(&stem);
            let lang = if resolved != stem || crate::lint_match::bundled_language(&stem) {
                resolved
            } else {
                "any".into()
            };
            docs.push((p, lang));
        }
    }
    docs.sort();
    docs
}

/// The corpus folder's rules that govern `lang` — see [`corpus_documents`].
pub(crate) fn corpus_rules(data_root: &Path, lang: &str) -> Vec<DocRule> {
    rules_in_documents(&corpus_documents(data_root), data_root, lang, "corpus-rule")
}

/// Read rule documents through the ONE document reader and keep the rules that govern `lang`
/// (`lang` itself, or `any` for code languages). `slice` labels the rules' origin tier.
/// Prose-only rules (no bad example) are valid: the pattern is derived from the English
/// description.
fn rules_in_documents(
    docs: &[(PathBuf, String)],
    data_root: &Path,
    lang: &str,
    slice: &str,
) -> Vec<DocRule> {
    let polarity = crate::lint_docs::document_polarity(data_root);
    let mut out = Vec::new();
    let allow_any = !is_document_language(lang);
    for (path, default_lang) in docs {
        let Ok(doc) = std::fs::read_to_string(path) else { continue };
        let source = path.to_string_lossy().into_owned();
        for r in crate::linter::Knowledge::read_document(default_lang, &doc, polarity.as_ref()).rules {
            let any = r.language == "any" || r.language.is_empty();
            if !(r.language == lang || (any && allow_any)) {
                continue;
            }
            out.push(DocRule {
                id: r.id,
                slice: slice.to_string(),
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

/// The model cache directory — exposed for sibling modules that persist shared learned
/// artifacts beside the per-language models (e.g. the transferred polarity classifier).
pub(crate) fn model_dir_pub() -> PathBuf {
    model_dir()
}

/// An embedded `lint-index/` artifact alone — for substrate loads that have no project
/// `data_root` in hand (the common-language brain is consulted from deep inside construct
/// selection, where plumbing a root through every caller would couple the matcher to setup).
pub(crate) fn embedded_lint_index_file(name: &str) -> Option<String> {
    EMBEDDED_LINT_INDEX.get_file(name).and_then(|f| f.contents_utf8().map(str::to_string))
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
    project_code: &std::collections::BTreeMap<String, Vec<(String, String)>>,
) -> (TrainReport, HashMap<String, LangModel>) {
    // Languages are independent (own toolchain, own sources, own cache files), so they train in
    // PARALLEL — cold setup costs the slowest language, not the sum. Shared crawled sources are
    // deduplicated by the per-source crawl cache (`lint_docs`), so two languages reading the same
    // site never fetch it twice. Results merge in `langs` order, keeping the report deterministic.
    let results: Vec<(TrainReport, Option<(String, LangModel)>)> = std::thread::scope(|s| {
        let handles: Vec<_> = langs
            .iter()
            .map(|lang| {
                s.spawn(move || {
                    let t0 = std::time::Instant::now();
                    let out = train_language(lang, data_root, project_root, project_code);
                    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
                        eprintln!("[lint-train] {lang}: {:.1}ms", t0.elapsed().as_secs_f64() * 1e3);
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("language training thread panicked"))
            .collect()
    });
    let mut report = TrainReport::default();
    let mut models = HashMap::new();
    for (r, model) in results {
        report.trained.extend(r.trained);
        report.reused.extend(r.reused);
        report.skipped.extend(r.skipped);
        report.crawled.extend(r.crawled);
        report.unenforced.extend(r.unenforced);
        report.unlearned.extend(r.unlearned);
        report.pulled.extend(r.pulled);
        if let Some((lang, m)) = model {
            models.insert(lang, m);
        }
    }
    // Meaningful only in setup mode (a lint run is replay-only and never networks): the
    // hermetic offline switch and a real transport failure report identically — the wire was
    // down, so whatever stayed unlearned needs a reconnect.
    report.net_down = (NETWORK_SETUP.load(std::sync::atomic::Ordering::Relaxed)
        && std::env::var_os("HELPERS_LINT_OFFLINE").is_some())
        || crate::doc_crawler::network_down();
    (report, models)
}

/// Train or load ONE language's model — the per-language body of [`ensure_models`], isolated so
/// languages can run on their own threads. Returns the language's report slice and its model
/// (`None` when the language is skipped).
///
/// The model is `overlay ⊕ module` (LINTER.md, "Save"): the shared AI MODULE (doc-trained,
/// project-independent, registry-shareable) merged under the PROJECT OVERLAY (the project's
/// law + the machine corpus principles, compiled locally). Documentation is purely training
/// input — the module carries only the compiled result and its provenance timestamp.
fn train_language(
    lang: &str,
    data_root: &Path,
    project_root: &Path,
    project_code: &std::collections::BTreeMap<String, Vec<(String, String)>>,
) -> (TrainReport, Option<(String, LangModel)>) {
    let mut report = TrainReport::default();
    let lang = &lang.to_string();
    let version = crate::lint_checkers::detect_version(lang).unwrap_or_default();
    let sources_fp = sources_fingerprint(data_root, lang);

    // ── 1) The AI MODULE: fresh on disk → registry → read the docs → none (law-only). ──
    let mut module = load_module(lang)
        .filter(|m| m.version == version && m.train_version == TRAIN_VERSION && m.sources_fp == sources_fp);
    if network_allowed() {
        // 100% VERIFIED CURRENT: past the verification window, every inventoried page is
        // conditionally revalidated against the live site; only real movement retrains —
        // an all-304 sweep just restarts the window (LINTER.md, "Save").
        if let Some(m) = &mut module {
            if unix_now().saturating_sub(m.verified_at.max(m.trained_at)) > MODULE_MAX_AGE {
                if crate::lint_docs::refresh_language_pages(data_root, lang, &version) {
                    module = None;
                } else {
                    m.verified_at = unix_now();
                    save_module(lang, m);
                }
            }
        }
        if module.is_none() {
            if let Some(m) = registry_fetch(data_root, lang, &version, &sources_fp) {
                save_module(lang, &m);
                report.pulled.push(lang.clone());
                module = Some(m);
            }
        }
    }
    let mut freshly_trained = false;
    if module.is_none() {
        let (doc_rules, reference, extensions, learned_from, flagged) =
            resolve_rules(data_root, lang, &version, &mut report);
        if learned_from == "nothing" {
            report.unlearned.push(lang.clone());
        } else {
            let tuples: Vec<(String, String, String, String, String, String)> = doc_rules
                .iter()
                .map(|r| (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone(), r.source.clone()))
                .collect();
            let concept_tuples: Vec<(String, String, String)> =
                doc_rules.iter().map(|r| (r.id.clone(), r.description.clone(), r.bad.clone())).collect();
            let ground = crate::lint_match::Grounding {
                reference,
                project: Vec::new(),
                polarity: crate::lint_docs::document_polarity(data_root),
                trusted: std::collections::HashSet::new(),
                flagged,
            };
            let m = Module {
                version: version.clone(),
                train_version: TRAIN_VERSION.to_string(),
                sources_fp: sources_fp.clone(),
                trained_at: unix_now(),
                verified_at: unix_now(),
                learned_from: learned_from.clone(),
                extensions,
                rules: RuleSet::build(lang, &tuples, &ground),
                concept: ConceptModel::compile(&concept_tuples, lang),
            };
            save_module(lang, &m);
            report.trained.push(format!("{lang} ({} rules, from {learned_from})", m.rules.rule_count()));
            freshly_trained = true;
            module = Some(m);
        }
    }
    if module.is_some() && !freshly_trained && !report.pulled.contains(lang) {
        report.reused.push(lang.clone());
    }

    // ── 2) The PROJECT OVERLAY: law + machine corpus, compiled against the project itself. ──
    let law_rules = project_rules(data_root, project_root, lang);
    let mut local_rules = law_rules;
    local_rules.extend(corpus_rules(data_root, lang));
    let trusted: std::collections::HashSet<String> =
        local_rules.iter().map(|r| r.id.clone()).collect();
    let project_fp = project_code
        .get(lang)
        .map(|files| {
            files.iter().map(|(_, src)| crate::lint_ai::token_seed(src)).fold(0u64, |acc, h| acc ^ h)
        })
        .unwrap_or(0);
    let module_id = module
        .as_ref()
        .map(|m| format!("{}@{}@{}@{}", m.version, m.sources_fp, m.train_version, m.trained_at))
        .unwrap_or_default();
    let stamp = overlay_stamp_of(lang, data_root, &version, &local_rules, project_fp, &module_id);
    let overlay = match load_overlay(lang, project_fp).filter(|o| o.stamp == stamp) {
        Some(o) => o,
        None => {
            // Law grounds in the project's own code first (its primary universe), then in
            // whatever reading memory THIS machine has — a machine that only pulled the
            // module compiles law without a docs corpus, by design: documentation is never
            // shipped, and the evidence hierarchy leads with project grounding anyway.
            let reference = load_cache(lang).map(|c| c.doc_rules(lang).1).unwrap_or_default();
            let ground = crate::lint_match::Grounding {
                reference,
                project: project_code
                    .get(lang)
                    .map(|files| files.iter().map(|(_, src)| src.clone()).collect())
                    .unwrap_or_default(),
                polarity: crate::lint_docs::document_polarity(data_root),
                trusted: trusted.clone(),
                flagged: Default::default(),
            };
            let tuples: Vec<(String, String, String, String, String, String)> = local_rules
                .iter()
                .map(|r| (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone(), r.source.clone()))
                .collect();
            let concept_tuples: Vec<(String, String, String)> =
                local_rules.iter().map(|r| (r.id.clone(), r.description.clone(), r.bad.clone())).collect();
            let rules = RuleSet::build(lang, &tuples, &ground);
            let compiled: std::collections::HashSet<String> =
                rules.rule_ids().map(str::to_string).collect();
            let o = Overlay {
                stamp,
                unenforced: trusted.iter().filter(|id| !compiled.contains(*id)).cloned().collect(),
                concept: ConceptModel::compile(&concept_tuples, lang),
                rules,
            };
            save_overlay(lang, project_fp, &o);
            o
        }
    };
    // The user's law never vanishes silently — unenforced ids persist with the overlay so
    // warm runs keep reporting them.
    for id in &overlay.unenforced {
        report.unenforced.push((lang.clone(), id.clone()));
    }

    // ── 3) overlay ⊕ module (overlay first — trust order). ──
    // A prose language's module contributes READING only (LINTER.md, "Docs are reading
    // material"): its files are raw English end to end — the universe ledger #12 excludes —
    // so learned detectors never fire there; only the project's own law governs them.
    let (module_rules, module_concept) = match module {
        Some(m) if !crate::lint_match::prose_lang(lang) => (m.rules, m.concept),
        _ => (RuleSet::empty(lang), ConceptModel { rules: Vec::new() }),
    };
    let rules = RuleSet::merged(overlay.rules, module_rules);
    let concept = ConceptModel::merged(overlay.concept, module_concept);
    if rules.rule_count() == 0 {
        report.skipped.push((lang.clone(), "no rules found for this language".to_string()));
        return (report, None);
    }
    (report, Some((lang.clone(), LangModel { rules, concept })))
}

/// Now, in unix seconds — module provenance timestamps.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// How old a module may grow before setup probes the live sources' `Last-Modified` — the
/// "always ensured up to date" window. A probe, never a re-read: the docs must actually have
/// moved.
const MODULE_MAX_AGE: u64 = 24 * 60 * 60;

/// The shareable, runnable AI MODULE for one language: the compiled doc-rule pattern engine,
/// its hypervector concept gate, and provenance (`toolchain @ sources @ TRAIN_VERSION @
/// trained_at`). This — and only this — is what the registry shares: no documentation in any
/// form (LINTER.md, "Save"). Small header fields serialize first so a prefix probe reads
/// provenance without parsing the engines.
#[derive(Serialize, Deserialize)]
struct Module {
    version: String,
    train_version: String,
    sources_fp: String,
    trained_at: u64,
    /// Last time the full page inventory was VERIFIED current against the live sites (every
    /// page conditionally revalidated, nothing moved). Verification restarts the window
    /// without retraining.
    #[serde(default)]
    verified_at: u64,
    learned_from: String,
    /// The language's learned file-extension claims (LINTER.md, "File types are learned by
    /// reading") — folded into the machine-global extension map at save.
    #[serde(default)]
    extensions: std::collections::BTreeMap<String, u32>,
    rules: RuleSet,
    concept: ConceptModel,
}

/// The per-project overlay: the project's law + machine corpus principles compiled locally,
/// with the ids that could not compile (reported every run — law never vanishes silently).
#[derive(Serialize, Deserialize)]
struct Overlay {
    stamp: String,
    unenforced: Vec<String>,
    concept: ConceptModel,
    rules: RuleSet,
}

fn module_path(lang: &str) -> PathBuf {
    model_dir().join(format!("{lang}.module.json"))
}

fn overlay_path(lang: &str, project_fp: u64) -> PathBuf {
    model_dir().join(format!("{lang}.overlay-{project_fp:016x}.json"))
}

fn load_module(lang: &str) -> Option<Module> {
    serde_json::from_str(&std::fs::read_to_string(module_path(lang)).ok()?).ok()
}

fn save_module(lang: &str, module: &Module) {
    if let Some(parent) = module_path(lang).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(module) {
        let _ = std::fs::write(module_path(lang), json);
    }
    fold_extension_claims(lang, &module.extensions);
}

// ── File types are learned by reading (LINTER.md) ─────────────────────────────────────────
//
// The machine-global extension map: `{language → {extension → mention count}}`, folded from
// every saved module. Resolution and law-stem aliasing both read it; the committed bootstrap
// (`lint-index/extensions-bootstrap.json`, machine-generated learned data) wires cold
// machines; an extension nothing claims IS the language name. No extension→language table
// exists in code.

/// A language's file-extension claims: `{extension → mention count}` learned from its docs.
pub(crate) type ExtClaims = std::collections::BTreeMap<String, u32>;

/// Where the machine-global extension map lives — beside the modules it is folded from.
fn extension_map_path() -> PathBuf {
    model_dir().join("extensions.json")
}

/// Merge one language's learned claims into the machine-global extension map. An empty claim
/// set still writes the language's entry, so "read the docs, saw no filenames" is recorded
/// rather than re-derived.
fn fold_extension_claims(lang: &str, claims: &ExtClaims) {
    let path = extension_map_path();
    let mut map: std::collections::BTreeMap<String, ExtClaims> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    map.insert(lang.to_string(), claims.clone());
    if let Ok(json) = serde_json::to_string(&map) {
        let _ = std::fs::write(path, json);
    }
}

/// The full claims universe: the committed learned bootstrap under the machine map (a
/// machine's own reading overrides the shipped reading, per language). Cached per process by
/// the machine map's mtime — resolution runs once per project file, and a training run that
/// refolds the map mid-process (the MCP server is long-lived) invalidates it naturally.
fn extension_claims_universe() -> std::sync::Arc<std::collections::BTreeMap<String, ExtClaims>> {
    type Universe = std::collections::BTreeMap<String, ExtClaims>;
    type CacheKey = (PathBuf, Option<std::time::SystemTime>);
    static CACHE: std::sync::Mutex<Option<(CacheKey, std::sync::Arc<Universe>)>> =
        std::sync::Mutex::new(None);
    let path = extension_map_path();
    let key = (path.clone(), std::fs::metadata(&path).and_then(|m| m.modified()).ok());
    let mut cache = CACHE.lock().expect("extension map cache lock");
    if let Some((cached_key, universe)) = cache.as_ref() {
        if *cached_key == key {
            return universe.clone();
        }
    }
    let mut map: Universe = EMBEDDED_LINT_INDEX
        .get_file("extensions-bootstrap.json")
        .and_then(|f| f.contents_utf8())
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_default();
    if let Some(machine) =
        std::fs::read_to_string(&path).ok().and_then(|raw| serde_json::from_str::<Universe>(&raw).ok())
    {
        map.extend(machine);
    }
    let universe = std::sync::Arc::new(map);
    *cache = Some((key, universe.clone()));
    universe
}

/// Resolve a file extension (or a law-file stem — ledger #16: same map, by construction) to
/// its canonical language name, from the learned claims:
///
///   1. among claiming languages, one for which this extension is the PRIMARY claim (its own
///      top-counted extension) wins — the TS handbook mentions `.js` often, but `.ts` is its
///      primary, so javascript keeps `.js`;
///   2. else the highest mention count, ties broken lexicographically (determinism);
///   3. a document extension is never claimed by a code language (`open("file.txt")` examples
///      cannot make `.txt` python — prose stays reading material);
///   4. an extension nothing claims IS the language name — unknown languages surface by name
///      and the run asks for their docs.
pub fn resolve_language(name_or_ext: &str) -> String {
    resolve_in(&extension_claims_universe(), name_or_ext)
}

/// The KNOWN language a docs code-block hint declares, or `None` when the label resolves to
/// nothing this machine knows (LINTER.md, ledger #18): junk fence labels ("output", "plain")
/// are not hints, and treating them as languages would silently discard real examples. A label
/// is knowledge when resolution transformed it (an alias/typography match — "js" ⇒ javascript)
/// or the resolved name itself holds a claims entry.
pub fn hint_language(hint: &str) -> Option<String> {
    let h = hint.trim().to_lowercase();
    if h.is_empty() || h == "any" {
        return None;
    }
    let universe = extension_claims_universe();
    let resolved = resolve_in(&universe, &h);
    (resolved != h || universe.contains_key(&h)).then_some(resolved)
}

/// Whether a code block hinted `hint` belongs to a DIFFERENT known language than the one
/// training — the gate that keeps a polyglot page's foreign examples (an MDN JavaScript page's
/// HTML block) out of this language's bindings, grounding, and reference corpus. The block's
/// prose is still read; only its claim to be THIS language's code is refused.
pub fn foreign_example(lang: &str, hint: &str) -> bool {
    hint_language(hint).is_some_and(|h| h != lang)
}

/// [`resolve_language`] over an explicit claims universe — the pure core, unit-testable
/// against the committed bootstrap.
fn resolve_in(universe: &std::collections::BTreeMap<String, ExtClaims>, name_or_ext: &str) -> String {
    let ext = name_or_ext.to_lowercase();
    // "any" is the law system's own word (a rule file governing every language), never an
    // extension — ruby's docs claim ".any" (`.any?`) and must not swallow it.
    if ext == "any" {
        return ext;
    }
    // Already a known language name (a claims entry exists under it) — canonical as-is.
    if universe.contains_key(&ext) {
        return ext;
    }
    let mut best: Option<(&str, (bool, bool, u32))> = None; // lang → (primary, affix, count)
    for (lang, claims) in universe.iter() {
        // A language is a candidate through a real claim, or through its own NAME: an
        // extension that begins the name ("rs" → rust) or elides it ("yml" → yaml — a
        // first-letter-anchored subsequence, the classic vowel-dropping abbreviation) is
        // candidate typography even when the docs never wrote the extension out.
        // A claim owns nothing when it is noise beside the language's own primary claim
        // (<1% of it — the reference-fire idea): the PHP manual's few `.cs` mentions must
        // not swallow an extension no registered language owns.
        let top = claims.values().copied().max().unwrap_or(0);
        let claim = claims
            .get(&ext)
            .copied()
            .filter(|&c| c.saturating_mul(100) >= top);
        let prefix = lang.starts_with(&ext);
        let elision = ext.len() >= 2
            && ext.len() < lang.len()
            && lang.starts_with(&ext[..1])
            && is_subsequence(&ext, lang);
        // Claimless candidacy needs a PROPER elision — interior letters dropped ("yml" from
        // "yaml"), never a bare prefix: "cs" must not be swallowed by "css" when nothing
        // named csharp is registered; an unclaimed prefix stays an unknown language that the
        // run asks about.
        if claim.is_none() && !(elision && !prefix) {
            continue;
        }
        if crate::lint_match::prose_lang(&ext) && !crate::lint_match::prose_lang(lang) {
            continue;
        }
        let count = claim.unwrap_or(0);
        let primary = claim.is_some() && claims.values().all(|&c| c <= count);
        // Name typography ("rs" starts "rust", "sh" ends "bash", "yml" elides "yaml")
        // outranks raw mention counts: ruby's docs mention `.sh` scripts more than bash's
        // one-page manual ever names itself. A suffix counts only backed by a real claim —
        // otherwise every `.in` file would resolve to kotlin by its tail.
        let affix = prefix || elision || (claim.is_some() && lang.ends_with(&ext));
        let score = (primary, affix, count);
        let better = match &best {
            None => true,
            Some((b_lang, b_score)) => {
                score > *b_score || (score == *b_score && lang.as_str() < *b_lang)
            }
        };
        if better {
            best = Some((lang.as_str(), score));
        }
    }
    best.map(|(lang, _)| lang.to_string()).unwrap_or(ext)
}

/// Whether `needle`'s characters appear in `hay` in order — the elision test ("yml" ⊂ "yaml").
fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut it = hay.chars();
    needle.chars().all(|c| it.by_ref().any(|h| h == c))
}

fn load_overlay(lang: &str, project_fp: u64) -> Option<Overlay> {
    serde_json::from_str(&std::fs::read_to_string(overlay_path(lang, project_fp)).ok()?).ok()
}

fn save_overlay(lang: &str, project_fp: u64, overlay: &Overlay) {
    if let Ok(json) = serde_json::to_string(overlay) {
        let _ = std::fs::write(overlay_path(lang, project_fp), json);
    }
}

/// Drop a language's AI module (and any overlays) so the next setup re-acquires it — the
/// invalidation `add_source` uses when a new docs URL lands.
pub fn invalidate_module(lang: &str) {
    let _ = std::fs::remove_file(module_path(lang));
    if let Ok(entries) = std::fs::read_dir(model_dir()) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with(&format!("{lang}.overlay-")) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
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

// ── rule resolution: the AI learns its own rules, cached and version-current ──

/// Resolve a language's documented rules, in order of freshness:
///
///   1. the linter's own learned cache (`~/.cache/helpers/lint-models/`, machine-global — one
///      learning serves every project on the system), when it matches the detected toolchain
///      version;
///   2. the committed/embedded `lint-index/` snapshot, when it covers that version (fast, and
///      carries doc categories) — so a present, current seed avoids a needless crawl;
///   3. a **live crawl of the official docs** otherwise (stale/absent seed, or
///      `HELPERS_LINT_REFRESH` set) — this is the AI learning the rules itself and is what keeps
///      it current; the result is cached, version-keyed, so later runs are fast and only relearn
///      on a version bump;
///   4. the seed again as the offline fallback when a crawl is unavailable.
///
/// Records crawl activity in `report`. Returns the rules and a short provenance label.
fn resolve_rules(
    data_root: &Path,
    lang: &str,
    version: &str,
    report: &mut TrainReport,
) -> (Vec<DocRule>, Vec<String>, ExtClaims, String, std::collections::HashSet<u64>) {
    let refresh = std::env::var_os("HELPERS_LINT_REFRESH").is_some();
    let sources_fp = sources_fingerprint(data_root, lang);
    if !refresh {
        if let Some(cat) = load_cache(lang) {
            if cat.current(version, &sources_fp) {
                let (rules, reference) = cat.doc_rules(lang);
                if !rules.is_empty() {
                    let exts = cat.memory.as_ref().map(|m| m.extensions.clone()).unwrap_or_default();
                    let flagged = cat.memory.as_ref().map(|m| m.flagged.clone()).unwrap_or_default();
                    return (rules, reference, exts, format!("cache:{}", cat.learned_from), flagged);
                }
            }
        }
    }
    let (seed, seed_version) = seed_with_version(data_root, lang);
    // A present seed that covers the detected version (or when no version is detectable / the seed
    // is unpinned) is used directly — no reason to crawl docs we already mirror. The seed carries
    // no reference code (its caps lean on the rules' own good examples).
    let seed_current = !seed.is_empty() && (version.is_empty() || seed_version.is_empty() || seed_version == version);
    if !refresh && seed_current {
        return (seed, Vec::new(), ExtClaims::new(), "committed snapshot".to_string(), Default::default());
    }
    // READ it ourselves from the live docs. Cache the MEMORY we read (not pre-extracted rules),
    // keyed by the toolchain version, so the next run queries the same reading and only re-reads on
    // a version bump.
    if let Some(memory) = crawl_learn(data_root, lang, version) {
        let cat = LearnedCatalog {
            version: version.to_string(),
            train_version: TRAIN_VERSION.to_string(),
            sources_fp: sources_fp.clone(),
            learned_from: "docs".to_string(),
            rules: Vec::new(),
            reference: Vec::new(),
            memory: Some(memory),
        };
        let (rules, reference) = cat.doc_rules(lang);
        // Reading IS the module (LINTER.md): a descriptive spec that yields ZERO prohibition
        // rules still delivers the reference corpus and comprehension — the language is set
        // up, not "unlearned". Only a source that could not be READ falls through.
        report.crawled.push(lang.to_string());
        let exts = cat.memory.as_ref().map(|m| m.extensions.clone()).unwrap_or_default();
        let flagged = cat.memory.as_ref().map(|m| m.flagged.clone()).unwrap_or_default();
        save_cache(lang, &cat);
        return (rules, reference, exts, "live docs".to_string(), flagged);
    }
    // Offline or crawl-disabled: fall back to the snapshot (stale is better than nothing).
    if !seed.is_empty() {
        return (seed, Vec::new(), ExtClaims::new(), "committed snapshot".to_string(), Default::default());
    }
    (Vec::new(), Vec::new(), ExtClaims::new(), "nothing".to_string(), Default::default())
}

/// READ `lang`'s official language documentation into an association [`crate::lint_read::Memory`].
/// A language may have several registered documents (reference + style guide + its own linter's
/// rule docs); ALL are read into one memory, grounded once against the installed toolchain.
/// Sources come from the registry and the `add_source` store — never a web search. `None` when
/// nothing could be read (no network, no sources, empty read) or the crawler is not compiled in.
#[cfg(feature = "crawl")]
fn crawl_learn(data_root: &Path, lang: &str, version: &str) -> Option<crate::lint_read::Memory> {
    // `HELPERS_LINT_OFFLINE` gates the NETWORK, not the learning: registered sources still
    // replay from the machine-global page cache (a `TRAIN_VERSION` bump re-reads from disk).
    // A cold cache with no network reads nothing and the caller reports the language as not
    // yet learned. Sources are the registered registry plus whatever a user or agent handed
    // over via `add_source` — there is no web search: an unknown language is ASKED for.
    let sources = resolved_sources(data_root, lang);
    if sources.is_empty() {
        return None;
    }
    let memory = crate::lint_docs::read_language(lang, &sources, MAX_CRAWL_PAGES, data_root, Some(version));
    // A read succeeded when ANY page's prose was read (LINTER.md, "reading IS the module") —
    // bindings and reference code are riches, never the bar. Requiring bindings∨reference
    // threw away prose-only spec sites with no code blocks at all (json.org presents its
    // grammar as diagrams), reporting a cleanly-read language as "docs not learned".
    (memory.pages_read > 0 || !memory.bindings.is_empty() || !memory.reference.is_empty())
        .then_some(memory)
}

/// Every registered docs source for `lang` from `sources.json` (on-disk preferred, embedded
/// fallback) — a language may list several official documents and all of them are learned.
/// `kind:"crawl"` uses `seed`; `kind:"agent"` uses `docsBase` as a best-effort crawl target.
/// Public accessor for [`crawl_sources_from_config`] — the freshness probe in
/// [`crate::lint_docs::sources_changed_since`] walks the same registered source set.
#[cfg(feature = "crawl")]
pub(crate) fn registered_docs_sources(data_root: &Path, lang: &str) -> Vec<crate::lint_docs::DocsSource> {
    resolved_sources(data_root, lang)
}

/// The documentation URLs resolved for `lang` — what the setup report probes when a NEEDED
/// language failed to learn, to tell "site not answering" apart from "link answers but has
/// nothing readable" (LINTER.md, "Online to set up").
pub(crate) fn source_urls(data_root: &Path, lang: &str) -> Vec<String> {
    resolved_sources(data_root, lang).into_iter().map(|s| s.url).collect()
}

// ── The language manifest (LINTER.md, "The language manifest") ────────────────
//
// One user-owned file — `~/.config/helpers/languages.json` — says where every language's
// instructions come from. Setup backfills it from the committed registry; the user's edits
// override; `[]` disables (the run then asks). Every source resolution reads it FIRST.

/// Where the manifest lives (under `$HOME`, so hermetic tests redirect it with everything else).
pub fn manifest_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".config/helpers/languages.json")
}

/// The manifest's `languages` map as written (language → doc URLs; `[]` = disabled).
fn manifest_map() -> std::collections::BTreeMap<String, Vec<String>> {
    std::fs::read_to_string(manifest_path())
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|json| serde_json::from_value(json["languages"].clone()).ok())
        .unwrap_or_default()
}

/// The manifest's `sites` list — whole WEBSITES to learn from ("A site is a source"): each is
/// crawled once and every language its pages attribute to gets a module. User-owned, like the
/// language entries.
fn manifest_sites() -> Vec<String> {
    std::fs::read_to_string(manifest_path())
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|json| serde_json::from_value(json["sites"].clone()).ok())
        .unwrap_or_default()
}

/// Write the manifest back, pretty-printed for hand editing, with its contract in `_note`.
/// The user's `sites` list rides through untouched.
fn manifest_write(map: &std::collections::BTreeMap<String, Vec<String>>) {
    let path = manifest_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let doc = serde_json::json!({
        "_note": "Where each language's lint instructions come from (LINTER.md, 'The language manifest'). Yours to edit: change a language's URLs to retrain it from those docs at the next setup; set [] to disable its docs (the linter will ask); a language you delete is re-added from the committed registry. `sites` lists whole websites to learn from — every language a site's pages document gets a module. `lint_config action=add_source` writes here.",
        "languages": map,
        "sites": manifest_sites(),
    });
    if let Ok(json) = serde_json::to_string_pretty(&doc) {
        let _ = std::fs::write(path, json);
    }
}

/// Every SITE source this machine learns from: registry `kind:"site"` entries plus the
/// manifest's `sites` list. The tool id is `site-<host>` — the marker that keys its page
/// cache by TRAIN_VERSION (language-independent) instead of any toolchain.
pub(crate) fn site_sources(data_root: &Path) -> Vec<crate::lint_docs::DocsSource> {
    let mut out: Vec<crate::lint_docs::DocsSource> = Vec::new();
    let mut push = |url: &str| {
        let host = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .and_then(|rest| rest.split('/').next())
            .unwrap_or("site")
            .trim_start_matches("www.");
        let tool = format!("site-{host}");
        if !out.iter().any(|s| s.url == url) {
            out.push(crate::lint_docs::DocsSource { url: url.to_string(), crawl: true, tool });
        }
    };
    if let Some(raw) = std::fs::read_to_string(data_root.join("lint-index/sources.json")).ok().or_else(|| {
        EMBEDDED_LINT_INDEX.get_file("sources.json").and_then(|f| f.contents_utf8().map(str::to_string))
    }) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
            for e in json["sources"].as_array().into_iter().flatten() {
                if e["kind"].as_str() == Some("site") {
                    if let Some(u) = e["seed"].as_str() {
                        push(u);
                    }
                }
            }
        }
    }
    for u in manifest_sites() {
        push(&u);
    }
    out
}

/// SETUP step for site sources: make sure each site's page cache exists (crawling once when
/// the network is allowed) and report the languages it teaches. Returns one report line per
/// site.
#[cfg(feature = "crawl")]
pub(crate) fn prepare_sites(data_root: &Path, project_langs: &[String]) -> Vec<String> {
    let sites = site_sources(data_root);
    if sites.is_empty() {
        return Vec::new();
    }
    let mut extra: std::collections::HashSet<String> =
        registered_languages(data_root).into_iter().collect();
    extra.extend(project_langs.iter().map(|l| l.to_lowercase()));
    let mut lines = Vec::new();
    for src in sites {
        let langs = crate::lint_docs::ensure_site_cache(&src, MAX_CRAWL_PAGES, &extra);
        if langs.is_empty() {
            lines.push(format!(
                "site {} → no language recognized yet (pages read as prose only)",
                src.url
            ));
        } else {
            lines.push(format!("site {} → teaches: {}", src.url, langs.join(", ")));
        }
    }
    lines
}

#[cfg(not(feature = "crawl"))]
pub(crate) fn prepare_sites(_data_root: &Path, _project_langs: &[String]) -> Vec<String> {
    Vec::new()
}

/// Record `urls` as `lang`'s documentation in the manifest — the `add_source` write path.
pub(crate) fn manifest_set(lang: &str, urls: Vec<String>) {
    let mut map = manifest_map();
    map.insert(lang.to_lowercase(), urls);
    manifest_write(&map);
}

/// Backfill the manifest with every registry/added-store language it does not name yet, so
/// the file always shows the full picture. Existing entries (the user's word) are never
/// touched.
pub(crate) fn manifest_sync(data_root: &Path) {
    let mut map = manifest_map();
    let mut changed = false;
    for lang in registered_languages(data_root) {
        if !map.contains_key(&lang) {
            let mut urls: Vec<String> =
                crawl_sources_from_config(data_root, &lang).into_iter().map(|s| s.url).collect();
            if urls.is_empty() {
                urls.extend(crate::lint_docs::learned_source(&lang).map(|s| s.url));
            }
            // A language taught only by a SITE has no per-language URL; an empty entry
            // would read as the user disabling it. Its provenance is the manifest's own
            // `sites` list — leave the key absent.
            if urls.is_empty() {
                continue;
            }
            map.insert(lang, urls);
            changed = true;
        }
    }
    if changed || !manifest_path().exists() {
        manifest_write(&map);
    }
}

/// A stable per-URL source identity for manifest-provided docs — host plus a short hash of
/// the full URL, so the crawl page cache stays keyed consistently across runs and two sources
/// on one host stay distinct.
fn manifest_tool(url: &str) -> String {
    let host = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("docs")
        .trim_start_matches("www.");
    format!("{host}-{:08x}", crate::lint_ai::token_seed(url) as u32)
}

/// The single source-resolution seam (LINTER.md, "The language manifest"): every consumer —
/// training, fingerprints, freshness probes, "needs a docs URL" asks — reads THIS, so the
/// manifest, the registry, and the staleness stamps can never disagree about where a
/// language's docs live. A manifest entry equal to the registry's URL set returns the
/// registry's own source identities so existing page caches keep serving.
pub(crate) fn resolved_sources(data_root: &Path, lang: &str) -> Vec<crate::lint_docs::DocsSource> {
    let registry = crawl_sources_from_config(data_root, lang);
    let mut out = match manifest_map().get(&lang.to_lowercase()) {
        // `[]` is the user disabling this language's docs OUTRIGHT — sites included; the run
        // asks instead.
        Some(urls) if urls.is_empty() => return Vec::new(),
        Some(urls) => {
            let reg_urls: std::collections::BTreeSet<&str> =
                registry.iter().map(|s| s.url.as_str()).collect();
            let man_urls: std::collections::BTreeSet<&str> =
                urls.iter().map(|u| u.as_str()).collect();
            if man_urls == reg_urls {
                registry
            } else {
                urls.iter()
                    .map(|u| crate::lint_docs::DocsSource {
                        url: u.clone(),
                        crawl: true,
                        tool: manifest_tool(u),
                    })
                    .collect()
            }
        }
        None => registry,
    };
    // Site sources whose cached pages teach this language join in ("A site is a source").
    for site in site_sources(data_root) {
        if crate::lint_docs::cached_site_langs(&site.tool).contains(&lang.to_lowercase()) {
            out.push(site);
        }
    }
    if !out.is_empty() {
        return out;
    }
    crate::lint_docs::learned_source(lang).into_iter().collect()
}

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

/// The corpus folder's PROSE — teaching material the reader learns from (`extraDocs/*.md`/`.txt`,
/// embedded principles doc as the offline fallback). Teaching trains the reader, the classifier,
/// and the concept space; it never mints rules — enforcing a section's illustrative fences turned
/// reading material into law, which is exactly the confusion the comprehension/enforcement split
/// exists to prevent.
pub(crate) fn corpus_prose(data_root: &Path) -> Vec<String> {
    let corpus_dir = data_root.join("extraDocs");
    match std::fs::read_dir(&corpus_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| matches!(e.path().extension().and_then(|x| x.to_str()), Some("md" | "txt")))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .collect(),
        Err(_) => vec![EMBEDDED_CS_PRINCIPLES.to_string()],
    }
}

// ── public training API ──────────────────────────────────────────────────────

// ── cache + checksum plumbing ────────────────────────────────────────────────

/// Path to a language's learned-rule cache (`<lang>.learned.json`, beside its model).
/// Pull `lang`'s learned catalog from the GitHub model registry, when one is published for the
/// EXACT toolchain version and [`TRAIN_VERSION`] — anything else is "not available" and the
/// caller trains from docs instead (or asks). The registry base URL is DATA: the top-level
/// `registry` key of `sources.json`; no key, no registry. The registry serves `index.json`
/// (`[{language, toolchain, train_version, file}]`) beside the catalog files themselves.
/// Offline (hermetic switch) or transport failure returns `None` — the failure latches
/// [`crate::doc_crawler::NET_DOWN`] inside `fetch` and the run stays honest, never broken.
#[cfg(feature = "crawl")]
fn registry_fetch(data_root: &Path, lang: &str, version: &str, sources_fp: &str) -> Option<Module> {
    if !network_allowed() || crate::doc_crawler::network_down() {
        return None;
    }
    let base = registry_url(data_root)?;
    let index = registry_index(&base, data_root)?;
    let entry = index.as_array()?.iter().find(|e| {
        e["language"].as_str() == Some(lang)
            && e["train_version"].as_str() == Some(TRAIN_VERSION)
            && e["toolchain"].as_str() == Some(version)
            && e["sources"].as_str() == Some(sources_fp)
    })?;
    let file = entry["module"].as_str().or_else(|| entry["file"].as_str())?;
    let expected_hash = entry["sha256"].as_str()?;
    let (_, body) = crate::doc_crawler::fetch(&format!("{base}/{file}"))?;
    // The signed index pins each module's exact bytes: a hash mismatch means tampering or
    // corruption, and unverified bits must never reach the loaded engine.
    if crate::lint_sign::sha256_hex(body.as_bytes()) != expected_hash {
        return None;
    }
    let module: Module = serde_json::from_str(&body).ok()?;
    (module.train_version == TRAIN_VERSION
        && module.version == version
        && module.sources_fp == sources_fp)
        .then_some(module)
}

#[cfg(not(feature = "crawl"))]
fn registry_fetch(_data_root: &Path, _lang: &str, _version: &str, _sources_fp: &str) -> Option<Module> {
    None
}

/// The registry's `index.json`, fetched AT MOST once per run (process-wide `OnceLock` — every
/// language shares one answer) and disk-cached for a day (`registry-index.json` beside the
/// discovery cache). Without this, every warm run on a repo full of unlearnable
/// pseudo-languages (json, yml, …) paid one network round-trip per language, breaking the
/// "assume offline" default. `HELPERS_LINT_REFRESH` bypasses the day.
#[cfg(feature = "crawl")]
fn registry_index(base: &str, data_root: &Path) -> Option<serde_json::Value> {
    static INDEX: std::sync::OnceLock<Option<serde_json::Value>> = std::sync::OnceLock::new();
    INDEX
        .get_or_init(|| {
            let cache = lint_index_cache_dir().join("registry-index.json");
            let day = std::time::Duration::from_secs(24 * 60 * 60);
            let fresh = std::env::var_os("HELPERS_LINT_REFRESH").is_none()
                && std::fs::metadata(&cache)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|age| age < day);
            if fresh {
                // The disk cache holds an index that already verified once; still bound to
                // the trusted keys below on every use.
                if let Some((body, sig)) = std::fs::read_to_string(&cache)
                    .ok()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .and_then(|v| {
                        Some((v["index"].as_str()?.to_string(), v["signature"].as_str()?.to_string()))
                    })
                {
                    if let Some(v) = verify_index(&body, &sig, data_root) {
                        return Some(v);
                    }
                }
            }
            let (_, body) = crate::doc_crawler::fetch(&format!("{base}/index.json"))?;
            let (_, sig) = crate::doc_crawler::fetch(&format!("{base}/index.sig"))?;
            let sig = sig.trim().to_string();
            let v = verify_index(&body, &sig, data_root)?;
            let _ = std::fs::create_dir_all(lint_index_cache_dir());
            let _ = std::fs::write(
                &cache,
                serde_json::json!({ "index": body, "signature": sig }).to_string(),
            );
            Some(v)
        })
        .clone()
}

/// An index is real only when a TRUSTED key signed exactly these bytes — otherwise the
/// registry does not exist for this run and the machine reads the documentation itself.
#[cfg(feature = "crawl")]
fn verify_index(body: &str, signature: &str, data_root: &Path) -> Option<serde_json::Value> {
    trusted_registry_keys(data_root)
        .iter()
        .any(|key| crate::lint_sign::verify(body.as_bytes(), signature, key))
        .then(|| serde_json::from_str(body).ok())?
}

/// Fingerprint of `lang`'s resolved documentation source set: the sorted seed URLs (registry
/// file plus the `add_source` store), hashed. The learned-catalog cache key's third leg.
fn sources_fingerprint(data_root: &Path, lang: &str) -> String {
    let mut urls: Vec<String> =
        resolved_sources(data_root, lang).into_iter().map(|s| s.url).collect();
    urls.sort();
    urls.dedup();
    format!("{:016x}", crate::lint_ai::token_seed(&urls.join("\u{1f}")))
}

/// The Ed25519 public keys allowed to sign the consumed registry index —
/// `lint-index/trusted-keys.json` (on-disk preferred, embedded fallback). Data, but a security
/// anchor: an index signed by no trusted key means the registry does not exist for this run.
pub(crate) fn trusted_registry_keys(data_root: &Path) -> Vec<String> {
    let raw = std::fs::read_to_string(data_root.join("lint-index/trusted-keys.json")).ok().or_else(|| {
        EMBEDDED_LINT_INDEX.get_file("trusted-keys.json").and_then(|f| f.contents_utf8().map(str::to_string))
    });
    let Some(raw) = raw else { return Vec::new() };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else { return Vec::new() };
    json["registry"]
        .as_array()
        .map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// The GitHub model registry's base URL — the `registry` key of `sources.json` (on-disk
/// preferred, embedded fallback). Pure data; absent means no registry.
fn registry_url(data_root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(data_root.join("lint-index/sources.json")).ok().or_else(|| {
        EMBEDDED_LINT_INDEX.get_file("sources.json").and_then(|f| f.contents_utf8().map(str::to_string))
    })?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    json["registry"].as_str().map(|u| u.trim_end_matches('/').to_string())
}

/// Every language named in the sources registry (on-disk preferred, embedded fallback) plus
/// every language a URL was handed over for (`add_source`) — the full set `lint_config
/// action=train` batch-trains so the whole machine is live for every known language at once.
pub fn registered_languages(data_root: &Path) -> Vec<String> {
    let mut langs: Vec<String> = Vec::new();
    let mut push = |l: &str| {
        let l = l.to_lowercase();
        if !l.is_empty() && !langs.iter().any(|x| x == &l) {
            langs.push(l);
        }
    };
    if let Some(raw) = std::fs::read_to_string(data_root.join("lint-index/sources.json")).ok().or_else(|| {
        EMBEDDED_LINT_INDEX.get_file("sources.json").and_then(|f| f.contents_utf8().map(str::to_string))
    }) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
            for e in json["sources"].as_array().into_iter().flatten() {
                if let Some(l) = e["language"].as_str() {
                    push(l);
                }
            }
        }
    }
    // Languages the machine's SITE caches teach ("A site is a source" discovery).
    for site in site_sources(data_root) {
        for l in crate::lint_docs::cached_site_langs(&site.tool) {
            push(&l);
        }
    }
    // The manifest's own languages (the user may have added one by hand; `[]` = disabled).
    for (lang, urls) in manifest_map() {
        if !urls.is_empty() {
            push(&lang);
        }
    }
    // Added sources remembered per machine (~/.cache): kind "crawl" entries only — a legacy
    // negative marker (kind "none") is not a language to train.
    if let Ok(raw) = std::fs::read_to_string(crate::lint_docs::learned_sources_path_pub()) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
            for e in json["sources"].as_array().into_iter().flatten() {
                if e["kind"].as_str() == Some("crawl") {
                    if let Some(l) = e["language"].as_str() {
                        push(l);
                    }
                }
            }
        }
    }
    langs.sort();
    langs
}

/// Whether ANY documentation source is known for `lang` — registered in the registry file or
/// handed over via `add_source`. The training report uses it to ask precisely: a language
/// without a source needs a URL, not another train run.
pub fn has_docs_source(data_root: &Path, lang: &str) -> bool {
    !resolved_sources(data_root, lang).is_empty()
}

fn cache_path(lang: &str) -> PathBuf {
    model_dir().join(format!("{lang}.learned.json"))
}

/// Load a language's cached learned catalog, or `None` if absent/unreadable/stale.
///
/// Staleness is probed on the raw PREFIX before the full parse: the catalog is multi-megabyte,
/// and `version`/`train_version` serialize first (struct order, compact JSON), so a catalog
/// from older reading logic is rejected for the cost of a `read` — deserializing 13 MB just to
/// discover `current()` is false was a full third of every warm run after a `TRAIN_VERSION`
/// bump. Callers still call [`LearnedCatalog::current`] for the toolchain-version half.
fn load_cache(lang: &str) -> Option<LearnedCatalog> {
    let raw = std::fs::read_to_string(cache_path(lang)).ok()?;
    let head = raw.get(..512).unwrap_or(&raw);
    if !head.contains(&format!("\"train_version\":\"{TRAIN_VERSION}\"")) {
        return None;
    }
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
/// Cache stamp from FILE STATE, not parsed content: the multi-megabyte learned catalog is
/// covered by its `(mtime, len)` fingerprint (any recrawl rewrites the file), the on-disk seed
/// catalogs and the polarity classifiers by theirs, the project's law by its full (small) rows,
/// and the project's grounding corpus by its token fingerprint. A warm run therefore proves
/// freshness without deserializing anything — that parse was the dominant cost of every warm
/// lint. The embedded fallback catalogs are fixed per binary and covered by `TRAIN_VERSION`.
fn overlay_stamp_of(lang: &str, data_root: &Path, version: &str, law: &[DocRule], project_fp: u64, module_id: &str) -> String {
    let mut h = Sha256::new();
    h.update(TRAIN_VERSION.as_bytes());
    h.update(version.as_bytes());
    h.update(module_id.as_bytes());
    h.update(project_fp.to_le_bytes());
    h.update(file_state(&cache_path(lang)).to_le_bytes());
    h.update(file_state(&crate::lint_docs::global_polarity_path()).to_le_bytes());
    for dir in [data_root.join("lint-index"), lint_index_cache_dir()] {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut states: Vec<u128> = entries
            .flatten()
            .filter(|e| e.file_name().to_str().is_some_and(|n| n.ends_with(".json")))
            .map(|e| file_state(&e.path()))
            .collect();
        states.sort_unstable();
        for s in states {
            h.update(s.to_le_bytes());
        }
    }
    let mut rows: Vec<String> = law
        .iter()
        .map(|r| format!("{}\u{1f}{}\u{1f}{}\u{1f}{}", r.id, r.bad, r.good, r.description))
        .collect();
    rows.sort();
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

/// A file's `(mtime, len)` folded into one value; `0` when absent. The stamp's cheap witness
/// that a learned artifact changed on disk.
fn file_state(p: &Path) -> u128 {
    std::fs::metadata(p)
        .map(|m| {
            let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
            t.map(|d| d.as_nanos()).unwrap_or(0) ^ ((m.len() as u128) << 64)
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {

    /// Ledger #18's gate, as a table: a hint that resolves to a KNOWN different language is
    /// foreign; the training language's own aliases are not; junk fence labels are no hint at
    /// all (excluding on them would silently discard real examples).
    #[test]
    fn foreign_example_gate_trusts_only_known_language_hints() {
        for (lang, hint, want) in [
            ("javascript", "html", true),   // MDN js page's html block
            ("javascript", "css", true),    // …or css block
            ("javascript", "bash", true),   // …or a curl example
            ("javascript", "js", false),    // its own alias
            ("javascript", "javascript", false),
            ("html", "html", false),
            ("html", "js", true),
            ("rust", "rust", false),
            ("javascript", "", false),      // undeclared ⇒ the page's language
            ("javascript", "output", false), // junk label ⇒ no hint
            ("javascript", "plain", false),
        ] {
            assert_eq!(
                super::foreign_example(lang, hint),
                want,
                "foreign_example({lang:?}, {hint:?})"
            );
        }
    }

    /// The committed extensions bootstrap must wire every registered language's canonical
    /// extension to it — the learned replacement for the deleted `file_lang` table, asserted
    /// as data so a regenerated bootstrap that breaks a wiring fails here, not in the field.
    #[test]
    fn committed_bootstrap_resolves_the_canonical_extensions() {
        let raw = EMBEDDED_LINT_INDEX
            .get_file("extensions-bootstrap.json")
            .and_then(|f| f.contents_utf8())
            .expect("extensions-bootstrap.json is committed and embedded");
        let universe: std::collections::BTreeMap<String, ExtClaims> =
            serde_json::from_str(raw).expect("bootstrap parses");
        for (ext, lang) in [
            ("rs", "rust"),
            ("py", "python"),
            ("js", "javascript"),
            ("ts", "typescript"),
            ("go", "go"),
            ("java", "java"),
            ("rb", "ruby"),
            ("sh", "bash"),
            ("kt", "kotlin"),
            ("php", "php"),
            ("md", "markdown"),
            ("yaml", "yaml"),
            ("yml", "yaml"),
            ("json", "json"),
            ("css", "css"),
            ("html", "html"),
            ("xml", "xml"),
            ("toml", "toml"),
            ("svg", "svg"),
            ("zig", "zig"),
            ("c", "c"),
            // CommonMark's own spec claims .txt (markdown IS plain text there) — both are
            // prose languages with identical runtime handling, so the learned answer stands.
            ("txt", "markdown"),
            ("swift", "swift"),
        ] {
            assert_eq!(
                resolve_in(&universe, ext),
                lang,
                "the learned claims must resolve .{ext} to {lang}"
            );
        }
    }

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
    fn a_cached_catalog_from_older_reading_logic_or_other_sources_is_stale() {
        let cat = |train_version: &str| LearnedCatalog {
            version: "1.0".to_string(),
            train_version: train_version.to_string(),
            sources_fp: "abc123".to_string(),
            learned_from: "docs".to_string(),
            rules: Vec::new(),
            reference: Vec::new(),
            memory: None,
        };
        assert!(cat(TRAIN_VERSION).current("1.0", "abc123"), "same logic + toolchain + sources → fresh");
        assert!(!cat("").current("1.0", "abc123"), "a pre-versioning catalog must relearn");
        assert!(!cat("docs-v1-ancient").current("1.0", "abc123"), "older reading logic must relearn");
        assert!(!cat(TRAIN_VERSION).current("2.0", "abc123"), "a toolchain bump still relearns");
        assert!(
            !cat(TRAIN_VERSION).current("1.0", "other"),
            "an added or changed docs source must relearn — a catalog that never saw the new source is stale"
        );
    }
}

