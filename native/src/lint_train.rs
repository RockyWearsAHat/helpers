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
use crate::lint_codec::Bin;
use crate::lint_match::RuleSet;

/// A trained model for one language: the pattern rule set (the firing engine) and the Hv concept
/// gate (confirms imprecise text-fallback findings). Both train from the same two sources;
/// [`ensure_models`] builds them together so the lint tool makes one call and gets everything.
pub struct LangModel {
    /// Pattern-matching rules compiled from documentation bad/good examples — the firing engine.
    pub rules: RuleSet,
    /// Concept fingerprints for the same rules; the gate that confirms text-fallback findings.
    pub concept: ConceptModel,
    /// Identity of the merged engine — module provenance ⊕ overlay stamp, hashed. The verdict
    /// replay cache keys per-file findings on it (LINTER.md, "Warm runs replay per-file
    /// verdicts"): any retrain, law edit, or project change lands in one of the two stamps.
    pub id: u64,
}

/// Pages to crawl per source — a runaway safety valve (a mis-scoped seed must not eat a whole
/// wiki), never a working limit: the WHOLE in-scope docs tree is crawled and read (LINTER.md,
/// "Map"). The learned catalog is cached and registry-shared, so the cost is paid once per
/// machine per toolchain version.
#[cfg(feature = "crawl")]
pub(crate) const MAX_CRAWL_PAGES: usize = 20_000;

/// Bump when the training logic changes so existing caches are treated as stale and relearned.
pub(crate) const TRAIN_VERSION: &str = "docs-v97-pseudo-shape";

/// The minimum number of PROVEN construct rules the construct-module workflow
/// ([`crate::lint_module::graduated_rules`]) must graduate for a language before the MODULE seam flips
/// from the legacy token-miner to the proven set (THE FLIP, LINTER.md). A language the workflow OWNS —
/// one whose docs are per-construct rule pages / deprecation-notecard reference pages (the web stack) —
/// proves a module's worth of rules (MEASURED: javascript 5, css 31, html 8); an incidental cross-reader
/// (typescript 1) or a language with no such pages (rust 0) falls below the floor and STAYS on the miner,
/// so the flip is scoped BEHAVIORALLY to what the new workflow actually proves — no language named in code.
pub(crate) const GRADUATED_MODULE_FLOOR: usize = 3;

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
    /// The construct a graduated construct-module rule forbids ([`crate::linter::LearnedRule::construct`]):
    /// `Some(c)` compiles the rule to its proven `uses_construct(c)` plan directly; `None` is a legacy
    /// example/token rule. Optional and defaulted so pre-extracted committed catalogs are unaffected.
    #[serde(default)]
    construct: Option<String>,
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
    fn doc_rules(&self, lang: &str, data_root: &Path) -> (Vec<DocRule>, Vec<String>, Vec<Contradiction>) {
        match &self.memory {
            Some(memory) => {
                // THE FLIP (2026-07-11, LINTER.md "The flip pass"): a language's MODULE rules are the
                // PROVEN construct rules from the construct-module workflow ([`crate::lint_module`]) —
                // each graduated through the frozen self-generated test loop (or the notecard path) over
                // the docs' OWN prohibition/deprecation pages — for every language the new workflow OWNS.
                // A language OWNS the workflow when its structural doc reading (rule pages / deprecation
                // notecards) proves a MODULE's worth of construct rules ([`GRADUATED_MODULE_FLOOR`]); a
                // language that only shares the meaning graph (an incidental cross-reader) or has no such
                // pages graduates 0–1 and STAYS on the legacy token-miner ([`crate::lint_docs::rules_from_memory`]),
                // so no other language's rules disappear. MEASURED 2026-07-11: javascript 5 / css 31 /
                // html 8 flip to the proven set; typescript 1, rust 0 keep the miner. Behavioral scope,
                // no language named. The workflow's per-language measurement is `examples/web_module_train.rs`.
                let module = crate::lint_module::graduated_rules(lang, memory);
                let flip = module.rules.len() >= GRADUATED_MODULE_FLOOR;
                let source_rules = if flip {
                    module.rules
                } else {
                    crate::lint_docs::rules_from_memory(lang, memory)
                };
                let mut rules: Vec<DocRule> = source_rules
                    .into_iter()
                    .map(|(r, url)| DocRule {
                        id: r.id,
                        slice: r.severity.clone(),
                        severity: r.severity,
                        description: r.description,
                        bad: r.bad,
                        good: r.good,
                        source: url,
                        construct: r.construct,
                    })
                    .collect();
                // PROVEN-STATE PERSISTENCE + CONTRADICTION-DRIVEN RESHAPE (owner corrections 2026-07-12,
                // points 4 and 3c). A flip language's module RETAINS every construct rule proven in a past
                // retrain whose source page LEFT this crawl's corpus (retain-and-grow); a rule whose page is
                // STILL in the corpus but did not re-prove this crawl is a CONTRADICTION and is DROPPED, never
                // silently kept ([`merge_graduated`] — the fresh pass IS the re-check). SOURCE-SCOPED (owner
                // directive 2026-07-12): a retained rule whose source is no longer a registered documentation
                // source for this language is DROPPED first ([`registered_ledger`]) — an owner-removed source
                // (a third-party linter catalog) cannot leak its proven rules back through the ledger.
                // Structural (host match against the registry), no domain name in code. The write side
                // ([`persist_graduated_ledger`]) runs after the module is built.
                let mut contradictions = Vec::new();
                if flip {
                    let prior = registered_ledger(data_root, lang, load_graduated_ledger(lang));
                    let (merged, dropped) = merge_graduated(rules, prior, &module.corpus_urls);
                    rules = merged;
                    contradictions = dropped;
                    // PASS 27 — the GRADED (LOW-severity) tier: append the evidence-graded findings AFTER the
                    // proven merge, so the proven set and its order are byte-identical and the contradiction
                    // re-check (which is a PROVEN-rule mechanism) never touches them. Graded rules carry a
                    // `graded-<construct>` id and an empty bad/good; the module compile fires them via the
                    // same `uses_construct(fire)` plan at LOW severity ([`crate::lint_module::graded_forms`]).
                    for (r, url) in module.graded {
                        rules.push(DocRule {
                            id: r.id,
                            slice: r.severity.clone(),
                            severity: r.severity,
                            description: r.description,
                            bad: r.bad,
                            good: r.good,
                            source: url,
                            construct: r.construct,
                        });
                    }
                }
                (rules, memory.reference.clone(), contradictions)
            }
            None => (self.rules.clone(), self.reference.clone(), Vec::new()),
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
    /// Languages a replay-only run served from a STALE module (engine/toolchain/sources
    /// stamp mismatch) — outdated knowledge still enforces (LINTER.md, "Lint never learns
    /// from the network"), and the report owes the user the out-of-date footer.
    pub outdated: Vec<String>,
    /// A network request failed at the TRANSPORT level this run (or the hermetic
    /// `HELPERS_LINT_OFFLINE` switch simulated it): whatever is `unlearned` stayed that way
    /// because the wire was down, so the report asks to reconnect instead of to rephrase.
    pub net_down: bool,
    /// Ledger rules DROPPED this run because their source page was re-read and the rule failed to
    /// re-prove (Item 3c — contradiction-driven reshape). Recorded as `(language, "construct @ source")`
    /// so a contradiction is surfaced, never a silent drop.
    pub contradicted: Vec<(String, String)>,
}

/// Record every contradiction a language's [`LearnedCatalog::doc_rules`] re-check dropped, so the run
/// report can surface it (Item 3c — never a silent drop). No-op when the re-check found none.
fn record_contradictions(report: &mut TrainReport, lang: &str, contradictions: Vec<Contradiction>) {
    for (id, source) in contradictions {
        report.contradicted.push((lang.to_string(), format!("{id} @ {source}")));
    }
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

/// Memoize a document LISTING behind its directories' `(mtime, len)` states plus the
/// extension map's (stems resolve through the learned claims). A listing changes only when
/// an entry is added, removed, or renamed — each of which touches its directory's mtime —
/// while content edits are caught downstream by every document's own file state, so a
/// witness hit is always sound. Kills the per-language re-listing: seventeen parallel
/// passes each re-read the same two directories per run.
fn memoized_listing(
    key: &str,
    witnesses: &[&Path],
    compute: impl FnOnce() -> Vec<(PathBuf, String)>,
) -> Vec<(PathBuf, String)> {
    type Table = std::collections::HashMap<String, (u128, std::sync::Arc<Vec<(PathBuf, String)>>)>;
    static MEMO: std::sync::Mutex<Option<Table>> = std::sync::Mutex::new(None);
    let state = witnesses
        .iter()
        .map(|p| file_state(p))
        .chain(std::iter::once(file_state(&extension_map_path())))
        .fold(0u128, |acc, st| acc.rotate_left(11) ^ st);
    // Computed inside the lock deliberately, like the document read below: the first
    // parallel wave must produce one listing, not seventeen racing ones.
    let mut memo = MEMO.lock().unwrap_or_else(|e| e.into_inner());
    let table = memo.get_or_insert_with(Default::default);
    if let Some((have, hit)) = table.get(key) {
        if *have == state {
            return hit.as_ref().clone();
        }
    }
    let out = std::sync::Arc::new(compute());
    table.insert(key.to_string(), (state, out.clone()));
    out.as_ref().clone()
}

/// The directories law is read from when linting `project_root`: the root itself and every
/// ancestor up to and INCLUDING the enclosing repository root (the first ancestor holding a
/// `.git`). Linting a SUBDIRECTORY of a project must still see the project's `.helpers/lint-rules`
/// and root `lintPref` — before this, `rule_documents` read only the exact lint root, so law
/// silently vanished the moment an agent linted a subfolder. The walk stops at the repo boundary
/// (never climbing into unrelated parents or `$HOME`); a root with no `.git` scans only itself.
fn law_search_dirs(project_root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![project_root.to_path_buf()];
    let mut cur = project_root;
    loop {
        if cur.join(".git").exists() {
            break; // the repo root is the outermost project boundary (inclusive)
        }
        match cur.parent() {
            Some(parent) if parent != cur => {
                dirs.push(parent.to_path_buf());
                cur = parent;
            }
            _ => break,
        }
    }
    dirs
}

pub(crate) fn rule_documents(project_root: &Path) -> Vec<(PathBuf, String)> {
    let dirs = law_search_dirs(project_root);
    // Witness every scanned directory and its `.helpers/lint-rules` — the listing changes only
    // when a law file is added, removed, or renamed, each of which touches one of these dirs'
    // mtime (content edits are caught downstream by each document's own file state).
    let mut witnesses: Vec<PathBuf> = Vec::with_capacity(dirs.len() * 2);
    for d in &dirs {
        witnesses.push(d.join(".helpers/lint-rules"));
        witnesses.push(d.clone());
    }
    let witness_refs: Vec<&Path> = witnesses.iter().map(PathBuf::as_path).collect();
    memoized_listing(
        &format!("law\u{1f}{}", project_root.display()),
        &witness_refs,
        || rule_documents_uncached(&dirs),
    )
}

fn rule_documents_uncached(dirs: &[PathBuf]) -> Vec<(PathBuf, String)> {
    let is_text = |p: &Path| matches!(p.extension().and_then(|x| x.to_str()), Some("md" | "txt"));
    let mut docs = Vec::new();
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir.join(".helpers/lint-rules")) {
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
        if let Ok(entries) = std::fs::read_dir(dir) {
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
    }
    docs.sort();
    docs.dedup();
    docs
}

/// Load the project's own rules that govern `lang`: every rule read from [`rule_documents`] whose
/// language is `lang` or `any`. Project rules are merged BEFORE the global corpus and the crawled
/// docs, so they take priority over both.
pub(crate) fn project_rules(project_root: &Path, lang: &str) -> Vec<DocRule> {
    rules_in_documents(&rule_documents(project_root), lang, "project-rule", false)
}

/// The machine-global CS-principles rule documents: `<data_root>/corpus/*.{md,txt}`. A stem
/// naming a language (aliases resolve like rule-file stems do) scopes the file; any other stem
/// (`cs-principles`) means every code language. These are DATA — the CS2420/CS3500 canon lives
/// in files, never in code — and they are GATED like learned rules (prohibition-sentence entry,
/// grounding, self/reference-fire, quarantine): global principles must earn each firing; they
/// are not the user's own law.
fn corpus_documents(data_root: &Path) -> Vec<(PathBuf, String)> {
    memoized_listing(
        &format!("corpus\u{1f}{}", data_root.display()),
        &[&data_root.join("corpus")],
        || corpus_documents_uncached(data_root),
    )
}

fn corpus_documents_uncached(data_root: &Path) -> Vec<(PathBuf, String)> {
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

/// Whether `token` IS the name of a language this machine knows — a bundled grammar or a language
/// registered in the learned extension claims. Deliberately stricter than [`resolve_language`]
/// (which routes any stem to a best-effort language) and [`hint_language`] (which admits a language
/// reached only by an incidental mention count): only a token that IS a known language name trips
/// it, so an agnostic principle's own words ("Big-O", "One Concept", "graph") never read as a
/// language. There is NO coded language list here — the set is exactly the machine's learned and
/// bundled languages, so it grows with the machine and stays swappable.
fn token_names_language(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let t = token.to_lowercase();
    crate::lint_match::bundled_language(&t) || extension_claims_universe().contains_key(&t)
}

/// The language-AGNOSTIC portion of a canon corpus document — the ONLY portion the CS-principles
/// module wires (LINTER.md: "language-agnostic sections only … a canon's language-specific appendix
/// is excluded"). The canon states its principles as Markdown sections; a section whose HEADING
/// names a known language ([`token_names_language`] — never a coded list) is a language appendix and
/// is dropped together with its nested (deeper-heading) subsections, so `## Language-Specific: C# and
/// .NET` and its `###` members never mint cross-language junk like `uses_construct(lock)`. Headings
/// are recognised only OUTSIDE fenced code, mirroring the document reader, so a `# comment` inside a
/// Python example is never mistaken for a heading. The rule is general and keeps the canon swappable
/// with ZERO code change: a future `## Rust: …` canon section drops out the same way, and a different
/// rubric file is read by whatever agnostic sections it holds. A document naming no language is
/// returned whole; trailing agnostic sections AFTER a language section survive (only the language
/// section and its subsections are removed).
pub(crate) fn canon_agnostic(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut in_fence = false;
    let mut excluding: Option<usize> = None; // the heading LEVEL the language section began at
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence {
            let level = trimmed.len() - trimmed.trim_start_matches('#').len();
            if level > 0 {
                // A sibling or shallower heading closes an open language section.
                if excluding.is_some_and(|start| level <= start) {
                    excluding = None;
                }
                if excluding.is_none()
                    && trimmed[level..]
                        .split(|c: char| !c.is_ascii_alphanumeric())
                        .any(token_names_language)
                {
                    excluding = Some(level);
                }
            }
        }
        if excluding.is_none() {
            kept.push_str(line);
            kept.push('\n');
        }
    }
    kept
}

/// The corpus folder's rules that govern `lang` — see [`corpus_documents`]. Read as CANON: each
/// document's language-specific appendix is excluded first ([`canon_agnostic`]).
pub(crate) fn corpus_rules(data_root: &Path, lang: &str) -> Vec<DocRule> {
    rules_in_documents(&corpus_documents(data_root), lang, "corpus-rule", true)
}

/// Read rule documents through the ONE document reader and keep the rules that govern `lang`
/// (`lang` itself, or `any` for code languages). `slice` labels the rules' origin tier. `canon`
/// reads each document as owner CANON — its language-specific appendix excluded ([`canon_agnostic`]);
/// project law (`canon = false`) is read whole. Prose-only rules (no bad example) are valid: the
/// pattern is derived from the English description.
fn rules_in_documents(docs: &[(PathBuf, String)], lang: &str, slice: &str, canon: bool) -> Vec<DocRule> {
    let all = read_rule_documents(docs, canon);
    let allow_any = !is_document_language(lang);
    let mut out = Vec::new();
    for (source, r) in all.iter() {
        let any = r.language == "any" || r.language.is_empty();
        if !(r.language == lang || (any && allow_any)) {
            continue;
        }
        out.push(DocRule {
            id: r.id.clone(),
            slice: slice.to_string(),
            severity: r.severity.clone(),
            description: r.description.clone(),
            bad: r.bad.clone(),
            good: r.good.clone(),
            source: source.clone(),
            construct: r.construct.clone(),
        });
    }
    out
}

/// Every rule the given documents carry, read ONCE through the one document reader —
/// lang-AGNOSTIC (each rule already carries its language from its doc's stem and fences) and
/// memoized per doc-set state: seventeen languages once re-read identical documents per run,
/// and the read (sentence classification through the polarity classifier) was the training
/// stage's remaining cost. Computed INSIDE the lock deliberately: the first parallel wave
/// must produce ONE read, not seventeen racing ones.
fn read_rule_documents(
    docs: &[(PathBuf, String)],
    canon: bool,
) -> std::sync::Arc<Vec<(String, crate::linter::LearnedRule)>> {
    type Table = std::collections::HashMap<String, std::sync::Arc<Vec<(String, crate::linter::LearnedRule)>>>;
    static MEMO: std::sync::Mutex<Option<Table>> = std::sync::Mutex::new(None);
    let state = docs
        .iter()
        .map(|(p, _)| file_state(p))
        .fold(0u128, |acc, st| acc.rotate_left(11) ^ st);
    let names: Vec<&str> = docs.iter().filter_map(|(p, _)| p.to_str()).collect();
    let key = format!("{}\u{1f}{state:x}\u{1f}{}", canon as u8, names.join("\u{1f}"));
    let mut memo = MEMO.lock().unwrap_or_else(|e| e.into_inner());
    let table = memo.get_or_insert_with(Default::default);
    if let Some(hit) = table.get(&key) {
        return hit.clone();
    }
    let polarity = crate::lint_docs::document_polarity();
    let mut out = Vec::new();
    for (path, default_lang) in docs {
        let Ok(doc) = std::fs::read_to_string(path) else { continue };
        // Canon documents wire only their language-agnostic sections; project law is read whole.
        let doc = if canon { canon_agnostic(&doc) } else { doc };
        let source = path.to_string_lossy().into_owned();
        for r in crate::linter::Knowledge::read_document(default_lang, &doc, polarity.as_deref()).rules {
            out.push((source.clone(), r));
        }
    }
    let out = std::sync::Arc::new(out);
    table.insert(key, out.clone());
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
#[cfg_attr(not(test), allow(dead_code))] // consumed by the dev extensions-bootstrap generator
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
pub fn project_rule_ids(project_root: &Path, lang: &str) -> std::collections::HashSet<String> {
    project_rules(project_root, lang).into_iter().map(|r| r.id).collect()
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
/// The project's code as grounding input, served LAZILY (LINTER.md, "Warm runs replay
/// per-file verdicts"): fingerprints come from cached content seeds — no file reads — and the
/// full sources are pulled only on the rare path that actually needs them (an overlay
/// recompiling against its grounding universe).
pub trait ProjectSource: Sync {
    /// XOR of the language's file content seeds — the grounding fingerprint that keys the
    /// overlay to the project's code.
    fn fingerprint(&self, lang: &str) -> u64;
    /// The language's full sources `(rel path, contents)` — the overlay's grounding universe.
    fn sources(&self, lang: &str) -> Vec<(String, String)>;
}

/// A run with no project grounding (the machine-wide train batch): fingerprint zero,
/// no sources — law still compiles, through docs grounding and the evidence hierarchy.
pub struct NoProject;

impl ProjectSource for NoProject {
    fn fingerprint(&self, _lang: &str) -> u64 {
        0
    }
    fn sources(&self, _lang: &str) -> Vec<(String, String)> {
        Vec::new()
    }
}

pub fn ensure_models(
    langs: &[String],
    data_root: &Path,
    project_root: &Path,
    project_code: &dyn ProjectSource,
) -> (TrainReport, HashMap<String, LangModel>) {
    // Languages are independent (own toolchain, own sources, own cache files), so they train in
    // PARALLEL — cold setup costs the slowest language, not the sum. Shared crawled sources are
    // deduplicated by the per-source crawl cache (`lint_docs`), so two languages reading the same
    // site never fetch it twice. Results merge in `langs` order, keeping the report deterministic.
    let t_spawn = std::time::Instant::now();
    // Warm the SHARED reads once on the calling thread before the fan-out: the law and corpus
    // documents are language-agnostic and memoized behind one lock, so cold memos inside the
    // parallel wave would convoy every language on the first reader (measured: each of 17
    // languages reported the one ~1.5ms read as its own "law" time).
    let _ = read_rule_documents(&rule_documents(project_root), false);
    let _ = read_rule_documents(&corpus_documents(data_root), true);
    let results: Vec<(TrainReport, Option<(String, LangModel)>)> = {
        use rayon::prelude::*;
        langs
            .par_iter()
            .map(|lang| {
                let t0 = std::time::Instant::now();
                let out = train_language(lang, data_root, project_root, project_code);
                if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
                    eprintln!("[lint-train] {lang}: {:.1}ms", t0.elapsed().as_secs_f64() * 1e3);
                }
                out
            })
            .collect()
    };
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-train] scope total {:.1}ms", t_spawn.elapsed().as_secs_f64() * 1e3);
    }
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
        report.outdated.extend(r.outdated);
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
    project_code: &dyn ProjectSource,
) -> (TrainReport, Option<(String, LangModel)>) {
    let mut report = TrainReport::default();
    let lang = &lang.to_string();
    // Honest sub-stage accounting under HELPERS_LINT_TRACE — where a language's load time goes.
    let trace_on = std::env::var_os("HELPERS_LINT_TRACE").is_some();
    let mut splits: Vec<(&'static str, u128)> = Vec::new();
    let mut last = std::time::Instant::now();
    let mut mark = |splits: &mut Vec<(&'static str, u128)>, name: &'static str| {
        let now = std::time::Instant::now();
        splits.push((name, (now - last).as_micros()));
        last = now;
    };
    let version = crate::lint_checkers::detect_version(lang).unwrap_or_default();
    mark(&mut splits, "version");
    let sources_fp = sources_fingerprint(data_root, lang);
    mark(&mut splits, "sources_fp");

    // ── 1) The AI MODULE: fresh on disk → registry → read the docs → none (law-only). ──
    let on_disk = load_module(lang);
    // Item 3d — COMPLETE against a knowledge snapshot: a module is current only under the SAME
    // (toolchain ⊕ train logic ⊕ sources ⊕ BRAIN) it was proven on. The brain axis reopens refinement
    // when this machine's understanding changes (a rebuilt brain → the module re-proves through the 3c
    // re-check). Skipped when this machine has NO brain (a pull-only machine — `brain_fingerprint` is
    // `None`): a foreign module's brain stamp must not force a local retrain the machine cannot do.
    let local_brain = crate::lint_char::brain_fingerprint();
    let is_current = |m: &Module| {
        m.version == version
            && m.train_version == TRAIN_VERSION
            && m.sources_fp == sources_fp
            && local_brain.map_or(true, |fp| m.brain_fp == fp)
    };
    let stale = on_disk.as_ref().is_some_and(|m| !is_current(m));
    let mut module = on_disk.filter(is_current);
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
    } else if module.is_none() && stale {
        // OUTDATED KNOWLEDGE STILL ENFORCES (LINTER.md, "Lint never learns from the
        // network"): a replay-only run uses the stale module AS-IS — old reading beats no
        // reading — reports the language as out of date, and the bounded validation pass
        // (tools/lint.rs) tries the one cheap fix. Rebuilding from cached pages is real
        // work (seconds per language) and belongs to setup or the background healer,
        // never inline in a lint.
        module = load_module(lang);
        report.outdated.push(lang.clone());
    }
    let mut freshly_trained = false;
    if module.is_none() {
        let (doc_rules, reference, extensions, learned_from, flagged) =
            resolve_rules(data_root, lang, &version, &mut report);
        if learned_from == "nothing" {
            report.unlearned.push(lang.clone());
        } else {
            let tuples: Vec<(String, String, String, String, String, String, Option<String>)> = doc_rules
                .iter()
                .map(|r| (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone(), r.source.clone(), r.construct.clone()))
                .collect();
            let ground = crate::lint_match::Grounding {
                reference,
                project: Vec::new(),
                polarity: crate::lint_docs::document_polarity(),
                trusted: std::collections::HashSet::new(),
                flagged,
            };
            let rules = RuleSet::build(lang, &tuples, &ground);
            // PASS 31 — THE CONSERVATION INVARIANT (owner ruling: understanding drives linting;
            // "proven but silently unenforced" is a failure class, not a bug to rediscover). Every
            // PROVEN construct rule must be compiled, or withheld for a NAMED, ACCEPTED reason
            // (flood-unsafe shape; duplicate id/pattern — identical enforcement already exists).
            // Any other loss of a proven rule means the pipeline disagrees with the web: the train
            // FAILS LOUDLY for this language and refuses to ship the disagreeing module.
            let violations = proven_conservation_violations(&doc_rules, &rules);
            if !violations.is_empty() {
                report.skipped.push((
                    lang.clone(),
                    format!(
                        "INVARIANT VIOLATION — proven rules lost without an accepted reason: {} (module NOT saved; the web and the compile disagree)",
                        violations.join("; ")
                    ),
                ));
                mark(&mut splits, "module");
                return (report, None);
            }
            // Concepts exist only for rules that can FIRE (LINTER.md, "Hv concept gate"):
            // a rule that compiled no detector can never be confirmed, and its fingerprint
            // would only serve to veto other rules' true findings — measured: a
            // detector-less concept outranked `no_var_declaration` on its own construct.
            let compiled: std::collections::HashSet<&str> = rules.rule_ids().collect();
            let concept_tuples: Vec<(String, String, String)> = doc_rules
                .iter()
                .filter(|r| compiled.contains(r.id.as_str()))
                .map(|r| (r.id.clone(), r.description.clone(), r.bad.clone()))
                .collect();
            let m = Module {
                version: version.clone(),
                train_version: TRAIN_VERSION.to_string(),
                sources_fp: sources_fp.clone(),
                trained_at: unix_now(),
                verified_at: unix_now(),
                learned_from: learned_from.clone(),
                extensions,
                // COMPLETE against this machine's current understanding (Item 3d) — 0 when brain-less.
                brain_fp: crate::lint_char::brain_fingerprint().unwrap_or(0),
                concept: ConceptModel::compile(&concept_tuples, lang),
                rules,
            };
            save_module(lang, &m);
            // Persist the graduated construct rules this module was built from, so the next retrain
            // retains them (owner point 4). `doc_rules` already merged in any prior ledger, so this
            // grows the ledger monotonically; a non-flip language (no construct rules) leaves it untouched.
            persist_graduated_ledger(lang, &doc_rules);
            report.trained.push(format!("{lang} ({} rules, from {learned_from})", m.rules.rule_count()));
            freshly_trained = true;
            module = Some(m);
        }
    }
    if module.is_some() && !freshly_trained && !report.pulled.contains(lang) {
        report.reused.push(lang.clone());
    }
    mark(&mut splits, "module");

    // ── 2) The PROJECT OVERLAY: law + machine corpus, compiled against the project itself. ──
    let law_rules = project_rules(project_root, lang);
    mark(&mut splits, "law");
    let mut local_rules = law_rules;
    local_rules.extend(corpus_rules(data_root, lang));
    mark(&mut splits, "corpus");
    let trusted: std::collections::HashSet<String> =
        local_rules.iter().map(|r| r.id.clone()).collect();
    let project_fp = project_code.fingerprint(lang);
    let module_id = module
        .as_ref()
        .map(|m| format!("{}@{}@{}@{}", m.version, m.sources_fp, m.train_version, m.trained_at))
        .unwrap_or_default();
    let stamp = overlay_stamp_of(lang, data_root, &version, &local_rules, project_fp, &module_id);
    mark(&mut splits, "stamp");
    let overlay = match load_overlay(lang, project_fp).filter(|o| o.stamp == stamp) {
        Some(o) => o,
        None => {
            // Law grounds in the project's own code first (its primary universe), then in
            // whatever reading memory THIS machine has — a machine that only pulled the
            // module compiles law without a docs corpus, by design: documentation is never
            // shipped, and the evidence hierarchy leads with project grounding anyway.
            let reference = load_cache(lang).map(|c| c.doc_rules(lang, data_root).1).unwrap_or_default();
            let ground = crate::lint_match::Grounding {
                reference,
                project: project_code.sources(lang).into_iter().map(|(_, src)| src).collect(),
                polarity: crate::lint_docs::document_polarity(),
                trusted: trusted.clone(),
                flagged: Default::default(),
            };
            let tuples: Vec<(String, String, String, String, String, String, Option<String>)> = local_rules
                .iter()
                .map(|r| (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone(), r.source.clone(), r.construct.clone()))
                .collect();
            let rules = RuleSet::build(lang, &tuples, &ground);
            let compiled: std::collections::HashSet<String> =
                rules.rule_ids().map(str::to_string).collect();
            // Concepts only for rules that can fire — same argument as the module path.
            let concept_tuples: Vec<(String, String, String)> = local_rules
                .iter()
                .filter(|r| compiled.contains(&r.id) && !rules.is_probe(&r.id))
                .map(|r| (r.id.clone(), r.description.clone(), r.bad.clone()))
                .collect();
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
    let model_id = crate::lint_ai::token_seed(&format!("{module_id}\u{1f}{}", overlay.stamp));
    let rules = RuleSet::merged(overlay.rules, module_rules);
    let concept = ConceptModel::merged(overlay.concept, module_concept);
    mark(&mut splits, "overlay+merge");
    if trace_on {
        let parts: Vec<String> =
            splits.iter().map(|(n, us)| format!("{n} {:.1}ms", *us as f64 / 1000.0)).collect();
        eprintln!("[lint-train {lang}] {}", parts.join(", "));
    }
    if rules.rule_count() == 0 {
        report.skipped.push((lang.clone(), "no rules found for this language".to_string()));
        return (report, None);
    }
    (report, Some((lang.clone(), LangModel { rules, concept, id: model_id })))
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
    /// The knowledge-snapshot fingerprint this module was COMPLETED against (LINTER.md → Item 3d):
    /// the brain's [`crate::lint_char::brain_fingerprint`] at train time (0 when no brain existed).
    /// Together with `train_version` + `sources_fp` it is the completion stamp — a changed brain
    /// reopens refinement (the module goes stale and its rules re-prove through the 3c re-check).
    #[serde(default)]
    brain_fp: u64,
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

impl Bin for Module {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.version);
        e.str(&self.train_version);
        e.str(&self.sources_fp);
        e.u(self.trained_at);
        e.u(self.verified_at);
        e.str(&self.learned_from);
        self.extensions.enc(e);
        // The completion snapshot rides the wire; a `TRAIN_VERSION` bump relearns any older blob.
        e.u(self.brain_fp);
        self.rules.enc(e);
        self.concept.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Module> {
        Some(Module {
            version: d.str()?,
            train_version: d.str()?,
            sources_fp: d.str()?,
            trained_at: d.u()?,
            verified_at: d.u()?,
            learned_from: d.str()?,
            extensions: Bin::dec(d)?,
            brain_fp: d.u()?,
            rules: Bin::dec(d)?,
            concept: Bin::dec(d)?,
        })
    }
}

impl Module {
    /// The container's probe-readable provenance — what the registry publisher and staleness
    /// checks read from the file prefix: `train_version ␟ toolchain version ␟ sources_fp`.
    fn probe_stamp(&self) -> String {
        format!("{}\u{1f}{}\u{1f}{}", self.train_version, self.version, self.sources_fp)
    }
}

impl Bin for Overlay {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.stamp);
        self.unenforced.enc(e);
        self.concept.enc(e);
        self.rules.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<Overlay> {
        Some(Overlay { stamp: d.str()?, unenforced: Vec::dec(d)?, concept: Bin::dec(d)?, rules: Bin::dec(d)? })
    }
}

/// Decode one `HLM1` artifact file of the expected kind. `None` on absence, wrong kind, or any
/// malformed byte — every caller treats that as "artifact not there".
fn load_bin<T: Bin>(path: &Path, kind: u8) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;
    let (_, mut d) = crate::lint_codec::Dec::open(&bytes, kind)?;
    T::dec(&mut d)
}

/// The `docs-vNN` train ordinal a stamp carries, or `None` when it names none (a toolchain
/// version, a foreign stamp) — the comparable core of [`TRAIN_VERSION`]-family stamps.
fn train_ordinal(stamp: &str) -> Option<u32> {
    let rest = &stamp[stamp.find("docs-v")? + "docs-v".len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Whether writing an artifact stamped `stamp` at `path` would ROLL KNOWLEDGE BACKWARDS: the
/// artifact already on disk carries a NEWER `docs-vNN` ordinal than the writer (PASS 33 — the
/// stale-daemon class: a long-lived process whose binary was replaced on disk must never
/// resurrect its older knowledge over a newer store). Abstains (`false`, write allowed) when
/// either side carries no ordinal; reads only the container header prefix, never the payload.
pub(crate) fn stamp_regression(path: &Path, stamp: &str) -> bool {
    let Some(new) = train_ordinal(stamp) else { return false };
    let mut prefix = [0u8; 512];
    let have = std::fs::File::open(path)
        .and_then(|mut f| std::io::Read::read(&mut f, &mut prefix))
        .unwrap_or(0);
    crate::lint_codec::probe(&prefix[..have])
        .and_then(|h| train_ordinal(&h.stamp))
        .is_some_and(|existing| existing > new)
}

/// Encode one `HLM1` artifact file, creating the directory and deleting the legacy `.json`
/// twin so migrated machines keep exactly one copy (LINTER.md, "Save"). Refuses a
/// train-ordinal REGRESSION ([`stamp_regression`]) — an outlived process keeps the newer store.
fn save_bin<T: Bin>(path: &Path, kind: u8, stamp: &str, value: &T) {
    if stamp_regression(path, stamp) {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut e = crate::lint_codec::Enc::new();
    value.enc(&mut e);
    let _ = std::fs::write(path, e.finish(kind, stamp));
    let _ = std::fs::remove_file(path.with_extension("json"));
}

fn module_path(lang: &str) -> PathBuf {
    model_dir().join(format!("{lang}.module.bin"))
}

fn overlay_path(lang: &str, project_fp: u64) -> PathBuf {
    model_dir().join(format!("{lang}.overlay-{project_fp:016x}.bin"))
}

/// The compiled machine-global [`RuleSet`] cached for `lang`, or `None` when the language has no
/// trained module — the crawled-DOC rules the `lint_query rules` interrogation enumerates. Loads
/// the cached module; never trains (a query must not mutate machine state).
pub fn cached_ruleset(lang: &str) -> Option<crate::lint_match::RuleSet> {
    load_module(lang).map(|m| m.rules)
}

/// A trained module's COMPLETION state (LINTER.md → Item 3d): the knowledge snapshot it was proven
/// against and whether that snapshot is STILL current on this machine. `complete == true` means the
/// module was written under today's train logic, the current registered sources, and (when a brain
/// exists) this machine's current understanding — its proven set is at fixpoint and needs no refinement.
/// `complete == false` means a changed corpus or brain has REOPENED it: the next `train` re-proves its
/// rules through the 3c re-check.
pub struct ModuleCompletion {
    /// The train-logic version the module was written under.
    pub train_version: String,
    /// The source-set fingerprint (the corpus stamp) at train time.
    pub sources_fp: String,
    /// The brain knowledge-snapshot fingerprint at train time (0 when built brain-less).
    pub brain_fp: u64,
    /// Unix seconds the module was trained.
    pub trained_at: u64,
    /// Whether the snapshot is still current on this machine (no reopening pending).
    pub complete: bool,
}

/// The COMPLETION state of `lang`'s trained module, or `None` when no module is on disk. Recomputes the
/// live corpus stamp and reads the live brain fingerprint to decide `complete`, so the answer reflects
/// TODAY's knowledge, not just what was stamped. A pure read; never trains, never touches the network.
pub fn module_completion(lang: &str) -> Option<ModuleCompletion> {
    let m = load_module(lang)?;
    let data_root = crate::tools::lint::data_root_pub();
    let version = crate::lint_checkers::detect_version(lang).unwrap_or_default();
    let live_sources_fp = sources_fingerprint(&data_root, lang);
    let live_brain = crate::lint_char::brain_fingerprint();
    let complete = m.train_version == TRAIN_VERSION
        && m.version == version
        && m.sources_fp == live_sources_fp
        && live_brain.map_or(true, |fp| m.brain_fp == fp);
    Some(ModuleCompletion {
        train_version: m.train_version,
        sources_fp: m.sources_fp,
        brain_fp: m.brain_fp,
        trained_at: m.trained_at,
        complete,
    })
}

/// The association [`crate::lint_read::Memory`] a language's docs were read into — the source the
/// construct-module training workflow ([`crate::lint_module`]) derives its loop inputs from, and what
/// `examples/js_module_train.rs` measures against the real crawls. Loads the cached learned catalog;
/// `None` when the language has not been read yet. Never trains, never touches the network.
pub fn cached_memory(lang: &str) -> Option<crate::lint_read::Memory> {
    load_cache(lang).and_then(|c| c.memory)
}

/// The machine-global CORPUS rules compiled for `lang` — the understanding→trace (and probe
/// fallback) rules the live overlay derives from `<data_root>/corpus/*.md`, read FRESH (LINTER.md,
/// "the corpus is read fresh each run"). The `lint_query rules` interrogation enumerates these
/// ALONGSIDE the crawled-doc module so the listing reflects what GENUINELY enforces, not just the
/// module. Built with empty grounding: a trace/probe rule binds from UNDERSTANDING alone — grounding
/// gates a doc EXAMPLE, which a prose-only corpus principle has none of — so the same rules the
/// overlay compiles appear here without a project. Never trains; a pure read of the corpus files.
pub fn corpus_ruleset(lang: &str) -> crate::lint_match::RuleSet {
    let data_root = crate::tools::lint::data_root_pub();
    let rules = corpus_rules(&data_root, lang);
    let trusted: std::collections::HashSet<String> = rules.iter().map(|r| r.id.clone()).collect();
    let ground = crate::lint_match::Grounding { trusted, ..Default::default() };
    let tuples: Vec<(String, String, String, String, String, String, Option<String>)> = rules
        .iter()
        .map(|r| {
            (r.id.clone(), r.severity.clone(), r.bad.clone(), r.good.clone(), r.description.clone(), r.source.clone(), r.construct.clone())
        })
        .collect();
    crate::lint_match::RuleSet::build(lang, &tuples, &ground)
}

fn load_module(lang: &str) -> Option<Module> {
    if let Some(m) = load_bin::<Module>(&module_path(lang), crate::lint_codec::kind::MODULE) {
        return Some(m);
    }
    // Legacy JSON module: migrate on sight — decode once, save the container, drop the JSON.
    let module: Module =
        serde_json::from_str(&std::fs::read_to_string(module_path(lang).with_extension("json")).ok()?).ok()?;
    save_module(lang, &module);
    Some(module)
}

fn save_module(lang: &str, module: &Module) {
    save_bin(&module_path(lang), crate::lint_codec::kind::MODULE, &module.probe_stamp(), module);
    fold_extension_claims(lang, &module.extensions);
}

/// The per-language GRADUATED-rule ledger path (`<lang>.graduated.bin`, beside the module). A SEPARATE
/// sidecar so it survives the module rebuild every retrain does — the store of PROVEN construct rules
/// kept retain-and-grow (owner correction 2026-07-12, point 4).
fn graduated_ledger_path(lang: &str) -> PathBuf {
    model_dir().join(format!("{lang}.graduated.bin"))
}

/// The construct rules PROVEN in past retrains for `lang`, or empty when none is persisted. The ledger
/// is stamped with the [`TRAIN_VERSION`] it was written under and DISCARDED on a mismatch: a ledger from
/// a different version may carry rule ids/semantics this version changed (e.g. the pre-2026-07-12 slugged
/// `uses--` collision), so persistence is retain-and-grow WITHIN a `TRAIN_VERSION`, reset on a semantic
/// bump. Never trains; a pure read.
fn load_graduated_ledger(lang: &str) -> Vec<DocRule> {
    let Ok(bytes) = std::fs::read(graduated_ledger_path(lang)) else {
        return Vec::new();
    };
    let Some((stamp, mut d)) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::GRADUATED) else {
        return Vec::new();
    };
    if stamp != TRAIN_VERSION {
        return Vec::new();
    }
    Vec::<DocRule>::dec(&mut d).unwrap_or_default()
}

/// One dropped ledger rule: its byte-preserved construct id and the source page whose re-check
/// contradicted it — recorded so a contradiction is NEVER a silent drop (LINTER.md → Item 3c).
type Contradiction = (String, String);

/// PASS 31 — the CONSERVATION INVARIANT check: every PROVEN construct rule (a `uses-…` graduated rule,
/// never the evidence-graded LOW tier) must be COMPILED into the rule set, or WITHHELD for a NAMED,
/// ACCEPTED reason — a flood-unsafe firing shape, or a duplicate id/pattern (identical enforcement
/// already exists). Returns the violations (`id: reason` / `id: VANISHED`); an empty return is the
/// invariant holding. This is the pipeline refereeing ITSELF: the web (source of truth) and the compile
/// may never silently disagree — the measured `document.write` class (a proven deprecation unenforced
/// because a presentation gate judged its display sentence) becomes a loud training failure.
fn proven_conservation_violations(doc_rules: &[DocRule], rules: &crate::lint_match::RuleSet) -> Vec<String> {
    let accepted = |gate: &str| {
        gate.contains("flood-unsafe") || gate.contains("duplicate id") || gate.contains("duplicate compiled pattern")
    };
    let compiled: std::collections::HashSet<&str> = rules.rule_ids().collect();
    let withheld: std::collections::HashMap<&str, &str> =
        rules.withheld().iter().map(|(id, gate)| (id.as_str(), gate.as_str())).collect();
    doc_rules
        .iter()
        .filter(|r| r.construct.is_some() && !r.id.starts_with("graded-"))
        .filter(|r| !compiled.contains(r.id.as_str()))
        .filter_map(|r| match withheld.get(r.id.as_str()) {
            Some(gate) if accepted(gate) => None,
            Some(gate) => Some(format!("{}: {}", r.id, gate)),
            None => Some(format!("{}: VANISHED (no compile record at all)", r.id)),
        })
        .collect()
}

/// CONTRADICTION-DRIVEN RESHAPE merge of freshly-graduated construct rules with the persisted ledger
/// (owner correction 2026-07-12, Item 3c — judgment LEARNS, the missing half of point 4). The fresh
/// graduation pass IS the re-check: it re-ran the blind self-generated loop over the CURRENT (grown)
/// brain + corpus. So for each PRIOR ledger rule, keyed by its byte-preserved construct id (point 1):
/// - **Re-proven** — its construct is in `fresh`: fresh WINS (a reshaped understanding from the grown
///   brain replaces the old one; the stamp refreshes on the next persist). Agreement, retained.
/// - **Contradiction** — its construct is ABSENT from `fresh` but its `source` page is STILL in
///   `corpus_urls`: the page was re-read and re-tested this crawl and the rule FAILED to re-prove. Never
///   a silent keep — the rule is DROPPED and the contradiction is recorded for the caller to surface.
/// - **Unrefreshed retain** — its construct is absent from `fresh` AND its `source` page has LEFT the
///   corpus (a subset crawl that did not fetch it): the last proof is RETAINED (retain-and-grow), never
///   re-litigated against a corpus that never saw it (the MEASURED eqeqeq subset-variance case).
///
/// Returns the merged rule set and every contradiction dropped, so nothing vanishes silently.
fn merge_graduated(
    mut fresh: Vec<DocRule>,
    prior: Vec<DocRule>,
    corpus_urls: &std::collections::HashSet<String>,
) -> (Vec<DocRule>, Vec<Contradiction>) {
    let have: std::collections::HashSet<String> = fresh.iter().map(|r| r.id.clone()).collect();
    let mut dropped = Vec::new();
    for p in prior {
        if have.contains(&p.id) {
            continue; // re-proven this crawl — fresh (possibly reshaped) already carries it
        }
        if corpus_urls.contains(&p.source) {
            dropped.push((p.id, p.source)); // page re-read, rule did not re-prove — contradiction
        } else {
            fresh.push(p); // page left the corpus — retain the last proof, unrefreshed
        }
    }
    (fresh, dropped)
}

/// Drop ledger rules whose SOURCE is no longer a registered documentation source for `lang` (owner
/// directive 2026-07-12: a language's module learns ONLY from its own registered documentation; a
/// source the owner removes from the registry — a third-party linter's rule catalog, say — must not
/// leak its proven rules back through the retain-and-grow ledger). STRUCTURAL: a rule is retained only
/// when its source URL's host ([`crate::lint_docs::url_host`]) matches a currently-registered source's
/// host ([`resolved_sources`]). Names no domain — the registry (`sources.json`/manifest) is the DATA
/// that decides.
fn registered_ledger(data_root: &Path, lang: &str, prior: Vec<DocRule>) -> Vec<DocRule> {
    let host = crate::lint_docs::url_host;
    let allowed: std::collections::HashSet<String> =
        resolved_sources(data_root, lang).iter().filter_map(|s| host(&s.url)).collect();
    prior.into_iter().filter(|r| host(&r.source).is_some_and(|h| allowed.contains(&h))).collect()
}

/// Persist the graduated construct rules a fresh module was built from, so the next retrain retains them
/// (owner point 4). Only the construct-carrying (graduated) rules are stored, and ONLY when there are any:
/// a language whose module did NOT engage the flip (legacy miner rules, `construct == None`) must never
/// overwrite an existing ledger with emptiness. Stamped with the current [`TRAIN_VERSION`].
fn persist_graduated_ledger(lang: &str, rules: &[DocRule]) {
    let graduated: Vec<DocRule> = rules.iter().filter(|r| r.construct.is_some()).cloned().collect();
    if !graduated.is_empty() {
        save_bin(&graduated_ledger_path(lang), crate::lint_codec::kind::GRADUATED, TRAIN_VERSION, &graduated);
    }
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
/// `HLM1` binary like every machine artifact (LINTER.md, "The live path": nothing on the
/// hot path parses JSON); the `.json` spelling survives only as a legacy read fallback.
fn extension_map_path() -> PathBuf {
    model_dir().join("extensions.bin")
}

/// Read the machine map: the binary artifact, else the legacy JSON (migrated on the next
/// fold). Empty when neither exists.
fn read_extension_map(path: &Path) -> std::collections::BTreeMap<String, ExtClaims> {
    if let Some(map) = std::fs::read(path).ok().and_then(|bytes| {
        let (_, mut d) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::EXTMAP)?;
        let langs = d.u()? as usize;
        let mut map = std::collections::BTreeMap::new();
        for _ in 0..langs {
            let lang = d.str()?;
            let n = d.u()? as usize;
            let mut claims = ExtClaims::new();
            for _ in 0..n {
                let ext = d.str()?;
                claims.insert(ext, d.u()? as u32);
            }
            map.insert(lang, claims);
        }
        Some(map)
    }) {
        return map;
    }
    std::fs::read_to_string(path.with_extension("json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Merge one language's learned claims into the machine-global extension map. An empty claim
/// set still writes the language's entry, so "read the docs, saw no filenames" is recorded
/// rather than re-derived.
fn fold_extension_claims(lang: &str, claims: &ExtClaims) {
    let path = extension_map_path();
    let mut map = read_extension_map(&path);
    map.insert(lang.to_string(), claims.clone());
    let mut e = crate::lint_codec::Enc::new();
    e.u(map.len() as u64);
    for (lang, claims) in &map {
        e.str(lang);
        e.u(claims.len() as u64);
        for (ext, count) in claims {
            e.str(ext);
            e.u(u64::from(*count));
        }
    }
    if std::fs::write(&path, e.finish(crate::lint_codec::kind::EXTMAP, TRAIN_VERSION)).is_ok() {
        let _ = std::fs::remove_file(path.with_extension("json"));
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
    // The committed bootstrap is CONSTANT for the process lifetime (embedded, reviewable
    // JSON by law) — parse it exactly once, not once per machine-map generation: its parse
    // was the measured multi-ms slice of every cold resolution.
    static BOOTSTRAP: std::sync::OnceLock<Universe> = std::sync::OnceLock::new();
    let bootstrap = BOOTSTRAP.get_or_init(|| {
        EMBEDDED_LINT_INDEX
            .get_file("extensions-bootstrap.json")
            .and_then(|f| f.contents_utf8())
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default()
    });
    let mut map = bootstrap.clone();
    map.extend(read_extension_map(&path));
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
    resolve_graded_memo(&extension_claims_universe(), name_or_ext).0
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
    let (resolved, label_grade) = resolve_graded_memo(&universe, &h);
    // Ledger #21: an incidental count-claim may route a FILE, never validate a LABEL — a
    // junk fence word that happens to appear after dots in some language's prose is no hint.
    if !label_grade {
        return None;
    }
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
#[cfg_attr(not(test), allow(dead_code))] // the bootstrap contract tests' explicit-universe entry
fn resolve_in(universe: &std::collections::BTreeMap<String, ExtClaims>, name_or_ext: &str) -> String {
    // Pure — no memo: tests hand this explicit universes the memo's generation cannot see.
    resolve_in_graded(universe, name_or_ext).0
}

/// [`resolve_in_graded`] memoized per universe GENERATION — the extension map's `(path,
/// mtime)` key, the same witness [`extension_claims_universe`] caches by, so identical key
/// means identical universe. The resolver is called once per walked file, and its linear
/// scan over every language's claims was a measurable slice of the walk stage.
fn resolve_graded_memo(
    universe: &std::collections::BTreeMap<String, ExtClaims>,
    name_or_ext: &str,
) -> (String, bool) {
    type Generation = (PathBuf, Option<std::time::SystemTime>);
    type Memo =
        std::sync::Mutex<Option<(Generation, std::collections::HashMap<String, (String, bool)>)>>;
    static MEMO: Memo = std::sync::Mutex::new(None);
    let path = extension_map_path();
    let generation = (path.clone(), std::fs::metadata(&path).and_then(|m| m.modified()).ok());
    let mut memo = MEMO.lock().unwrap_or_else(|e| e.into_inner());
    let table = match memo.as_mut() {
        Some((have, table)) if *have == generation => table,
        _ => {
            *memo = Some((generation, std::collections::HashMap::new()));
            &mut memo.as_mut().expect("just set").1
        }
    };
    if let Some(hit) = table.get(name_or_ext) {
        return hit.clone();
    }
    let answer = resolve_in_graded(universe, name_or_ext);
    table.insert(name_or_ext.to_string(), answer.clone());
    answer
}

/// [`resolve_in`] plus the resolution's GRADE (ledger #21): `true` when it resolved through
/// the language's own identity, a PRIMARY claim, or name typography — label-grade, trustable
/// by the fence-hint gate — and `false` when an incidental mention count alone carried it:
/// file-grade, good enough to route a file on disk, never to validate a label.
fn resolve_in_graded(universe: &std::collections::BTreeMap<String, ExtClaims>, name_or_ext: &str) -> (String, bool) {
    let ext = name_or_ext.to_lowercase();
    // "any" is the law system's own word (a rule file governing every language), never an
    // extension — ruby's docs claim ".any" (`.any?`) and must not swallow it.
    if ext == "any" {
        return (ext, true);
    }
    // Already a known language name (a claims entry exists under it) — canonical as-is.
    if universe.contains_key(&ext) {
        return (ext, true);
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
        // First-letter anchoring compares CHARS: a docs fence hint can open with a multibyte
        // character, and a byte slice there panicked the whole training batch.
        let elision = ext.len() >= 2
            && ext.len() < lang.len()
            && ext.chars().next() == lang.chars().next()
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
    match best {
        Some((lang, (primary, affix, _))) => (lang.to_string(), primary || affix),
        None => (ext, false),
    }
}

/// Whether `needle`'s characters appear in `hay` in order — the elision test ("yml" ⊂ "yaml").
fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut it = hay.chars();
    needle.chars().all(|c| it.by_ref().any(|h| h == c))
}

fn load_overlay(lang: &str, project_fp: u64) -> Option<Overlay> {
    // No JSON fallback: an overlay is cheap to recompile and its stamp would miss anyway
    // whenever its module migrated (the module identity is part of the stamp).
    load_bin(&overlay_path(lang, project_fp), crate::lint_codec::kind::OVERLAY)
}

fn save_overlay(lang: &str, project_fp: u64, overlay: &Overlay) {
    save_bin(&overlay_path(lang, project_fp), crate::lint_codec::kind::OVERLAY, &overlay.stamp, overlay);
}

/// Drop a language's AI module (and any overlays) so the next setup re-acquires it — the
/// invalidation `add_source` uses when a new docs URL lands.
pub fn invalidate_module(lang: &str) {
    let _ = std::fs::remove_file(module_path(lang));
    let _ = std::fs::remove_file(module_path(lang).with_extension("json"));
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
                let (rules, reference, contradictions) = cat.doc_rules(lang, data_root);
                if !rules.is_empty() {
                    record_contradictions(report, lang, contradictions);
                    let exts = cat.memory.as_ref().map(|m| m.extensions.clone()).unwrap_or_default();
                    let flagged = cat.memory.as_ref().map(|m| m.flagged.clone()).unwrap_or_default();
                    return (rules, reference, exts, format!("cache:{}", cat.learned_from), flagged);
                }
            }
        }
    }
    let t_seed = std::time::Instant::now();
    let (seed, seed_version) = seed_with_version(data_root, lang);
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-resolve {lang}] seed {:.1}ms", t_seed.elapsed().as_secs_f64() * 1e3);
    }
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
    let t_crawl = std::time::Instant::now();
    let crawled = crawl_learn(data_root, lang, version);
    if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
        eprintln!("[lint-resolve {lang}] crawl_learn {:.1}ms", t_crawl.elapsed().as_secs_f64() * 1e3);
    }
    if let Some(memory) = crawled {
        let cat = LearnedCatalog {
            version: version.to_string(),
            train_version: TRAIN_VERSION.to_string(),
            sources_fp: sources_fp.clone(),
            learned_from: "docs".to_string(),
            rules: Vec::new(),
            reference: Vec::new(),
            memory: Some(memory),
        };
        let (rules, reference, contradictions) = cat.doc_rules(lang, data_root);
        record_contradictions(report, lang, contradictions);
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
    // Memoized per (lang, input file states): three files are read and parsed per call, and a
    // warm run makes two calls per language for identical answers.
    type Memo = std::sync::Mutex<
        Option<std::collections::HashMap<String, Vec<crate::lint_docs::DocsSource>>>,
    >;
    static MEMO: Memo = std::sync::Mutex::new(None);
    let state = file_state(&data_root.join("lint-index/sources.json"))
        .rotate_left(1)
        ^ file_state(&manifest_path()).rotate_left(2)
        ^ file_state(&crate::lint_docs::learned_sources_path_pub()).rotate_left(3);
    let key = format!("{}\u{1f}{lang}\u{1f}{state:x}", data_root.display());
    {
        let mut memo = MEMO.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(hit) = memo.get_or_insert_with(Default::default).get(&key) {
            return hit.clone();
        }
    }
    let answer = resolved_sources_uncached(data_root, lang);
    let mut memo = MEMO.lock().unwrap_or_else(|e| e.into_inner());
    memo.get_or_insert_with(Default::default).insert(key, answer.clone());
    answer
}

/// [`resolved_sources`] proper — the manifest → registry → added-sources resolution.
fn resolved_sources_uncached(data_root: &Path, lang: &str) -> Vec<crate::lint_docs::DocsSource> {
    let registry = crawl_sources_from_config(data_root, lang);
    let mut out = match manifest_map().get(&lang.to_lowercase()) {
        // `[]` is the user disabling this language's docs OUTRIGHT — sites included; the run
        // asks instead.
        Some(urls) if urls.is_empty() => return Vec::new(),
        // PER-URL identity (PASS 33): a manifest URL the registry also names keeps the registry's
        // tool id — its crawl cache name and toolchain keying stay stable when the registry gains
        // or loses a SIBLING source. Only a URL the registry does not carry is manifest-keyed. The
        // manifest stays the user's word: a registry URL absent from it is still not crawled.
        Some(urls) => urls
            .iter()
            .map(|u| {
                registry.iter().find(|s| &s.url == u).cloned().unwrap_or_else(|| {
                    crate::lint_docs::DocsSource { url: u.clone(), crawl: true, tool: manifest_tool(u) }
                })
            })
            .collect(),
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
                construct: None,
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

/// A `lint-index` entry is a rule catalog if it is a `*.json` and not one of our OTHER data
/// families that live beside it: the source registry, the trusted-key anchor, the reading
/// corpus list, and the machine-generated bootstraps (parsing the multi-megabyte English
/// bootstrap as a candidate catalog cost every run of a seed-tier language ~19 ms — for a
/// file that can never match).
fn is_catalog_name(name: Option<&str>) -> bool {
    matches!(name, Some(n) if n.ends_with(".json")
        && n != "sources.json"
        && n != "trusted-keys.json"
        && n != "reading-sources.json"
        && !n.ends_with("-bootstrap.json"))
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

// ── HLM1 binary codecs (LINTER.md, "Save") — field order is wire order. ──────

impl Bin for DocRule {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.id);
        e.str(&self.slice);
        e.str(&self.severity);
        e.str(&self.description);
        e.str(&self.bad);
        e.str(&self.good);
        e.str(&self.source);
        // Empty string encodes `None` — a legacy example/token rule (the common case). Appended
        // last so the wire form stays additive; older blobs predate `TRAIN_VERSION`, so they are
        // relearned rather than decoded.
        e.str(self.construct.as_deref().unwrap_or(""));
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<DocRule> {
        Some(DocRule {
            id: d.str()?,
            slice: d.str()?,
            severity: d.str()?,
            description: d.str()?,
            bad: d.str()?,
            good: d.str()?,
            source: d.str()?,
            construct: {
                let c = d.str()?;
                (!c.is_empty()).then_some(c)
            },
        })
    }
}

impl Bin for LearnedCatalog {
    fn enc(&self, e: &mut crate::lint_codec::Enc) {
        e.str(&self.version);
        e.str(&self.train_version);
        e.str(&self.sources_fp);
        e.str(&self.learned_from);
        self.rules.enc(e);
        self.reference.enc(e);
        self.memory.enc(e);
    }
    fn dec(d: &mut crate::lint_codec::Dec) -> Option<LearnedCatalog> {
        Some(LearnedCatalog {
            version: d.str()?,
            train_version: d.str()?,
            sources_fp: d.str()?,
            learned_from: d.str()?,
            rules: Vec::dec(d)?,
            reference: Vec::dec(d)?,
            memory: Option::dec(d)?,
        })
    }
}

// ── public training API ──────────────────────────────────────────────────────

// ── cache + checksum plumbing ────────────────────────────────────────────────

/// Path to a language's learned-rule cache (`<lang>.learned.bin`, beside its model).
/// Pull `lang`'s learned catalog from the GitHub model registry, when one is published for the
/// EXACT toolchain version and [`TRAIN_VERSION`] — anything else is "not available" and the
/// caller trains from docs instead (or asks). The registry base URL is DATA: the top-level
/// `registry` key of `sources.json`; no key, no registry. The registry serves `index.json`
/// (`[{language, toolchain, train_version, file}]`) beside the catalog files themselves.
/// Offline (hermetic switch) or transport failure returns `None` — the failure latches
/// [`crate::doc_crawler::NET_DOWN`] inside `fetch` and the run stays honest, never broken.
/// Ceiling on a fetched registry module (defense in depth: the signed index pins real sizes
/// far below this — a mis-pointed URL must not balloon memory).
#[cfg(feature = "crawl")]
const MAX_REGISTRY_MODULE_BYTES: u64 = 64 << 20;

#[cfg(feature = "crawl")]
fn registry_fetch(data_root: &Path, lang: &str, version: &str, sources_fp: &str) -> Option<Module> {
    if !network_allowed() || crate::doc_crawler::network_down() {
        return None;
    }
    registry_fetch_inner(data_root, lang, version, sources_fp)
}

/// [`registry_fetch`] WITHOUT the setup latch — the bounded validation pass's entry
/// (LINTER.md, "Lint may VALIDATE, never learn"): a replay-only run that served outdated
/// knowledge may pull the current module, and nothing else. The hermetic offline switch
/// and the transport-down latch still apply.
#[cfg(feature = "crawl")]
fn registry_fetch_validation(
    data_root: &Path,
    lang: &str,
    version: &str,
    sources_fp: &str,
) -> Option<Module> {
    if std::env::var_os("HELPERS_LINT_OFFLINE").is_some() || crate::doc_crawler::network_down() {
        return None;
    }
    registry_fetch_inner(data_root, lang, version, sources_fp)
}

#[cfg(not(feature = "crawl"))]
fn registry_fetch_validation(
    _data_root: &Path,
    _lang: &str,
    _version: &str,
    _sources_fp: &str,
) -> Option<Module> {
    None
}

#[cfg(feature = "crawl")]
fn registry_fetch_inner(data_root: &Path, lang: &str, version: &str, sources_fp: &str) -> Option<Module> {
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
    let bytes = crate::doc_crawler::fetch_bytes(&format!("{base}/{file}"), MAX_REGISTRY_MODULE_BYTES)?;
    // The signed index pins each module's exact bytes: a hash mismatch means tampering or
    // corruption, and unverified bits must never reach the loaded engine.
    if crate::lint_sign::sha256_hex(&bytes) != expected_hash {
        return None;
    }
    // Current registries serve `HLM1` containers; legacy entries are JSON — the magic decides.
    let module: Module = if bytes.starts_with(&crate::lint_codec::MAGIC) {
        let (_, mut d) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::MODULE)?;
        Module::dec(&mut d)?
    } else {
        serde_json::from_str(std::str::from_utf8(&bytes).ok()?).ok()?
    };
    (module.train_version == TRAIN_VERSION
        && module.version == version
        && module.sources_fp == sources_fp)
        .then_some(module)
}

#[cfg(not(feature = "crawl"))]
fn registry_fetch(_data_root: &Path, _lang: &str, _version: &str, _sources_fp: &str) -> Option<Module> {
    None
}

/// The bounded validation pass (LINTER.md, "Lint may VALIDATE, never learn"): for every
/// language a replay-only run served from a stale module, try the one cheap fix — a
/// registry pull of the current module — on a background thread, waiting at most `budget`.
/// Returns whether EVERY such language came current within the budget. The thread keeps
/// working past the budget in a long-lived process, so the next lint benefits either way;
/// a one-shot process that exits first simply leaves the footer's advice standing. One
/// attempt per language per process — a dead registry must not be re-probed every lint.
pub fn heal_outdated_modules(
    langs: &[String],
    data_root: &Path,
    budget: std::time::Duration,
) -> bool {
    static ATTEMPTED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    let fresh: Vec<String> = {
        let mut seen = ATTEMPTED
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        langs.iter().filter(|l| seen.insert((*l).clone())).cloned().collect()
    };
    if fresh.is_empty() {
        return false; // already attempted this process — the footer keeps standing
    }
    let data = data_root.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut all = true;
        for lang in &fresh {
            let version = crate::lint_checkers::detect_version(lang).unwrap_or_default();
            let fp = sources_fingerprint(&data, lang);
            match registry_fetch_validation(&data, lang, &version, &fp) {
                Some(m) => save_module(lang, &m),
                None => all = false,
            }
        }
        let _ = tx.send(all);
    });
    rx.recv_timeout(budget).unwrap_or(false)
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
    // Fold the machine-global corpus CONTENT into the fingerprint: the corpus principles compile
    // into every language's module (understanding→trace bridge / probe fallback), so editing a
    // principle — or adding a new one — must rebuild the module and enforce with ZERO code change
    // (LINTER.md, "the understanding→trace bridge"). Hashing content (not just the file listing)
    // is what catches an in-place edit to an existing corpus file.
    let fp = crate::lint_ai::token_seed(&urls.join("\u{1f}")) ^ corpus_content_fp(data_root);
    format!("{fp:016x}")
}

/// A content fingerprint of the machine-global corpus folder (`corpus/*.{md,txt}`), read FRESH from
/// disk (not the listing memo, which is keyed by directory mtime and would miss an in-place content
/// edit). Deterministic in sorted path order so the fingerprint is reproducible.
fn corpus_content_fp(data_root: &Path) -> u64 {
    let mut files: Vec<PathBuf> = std::fs::read_dir(data_root.join("corpus"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|x| x.to_str()).is_some_and(|x| x == "md" || x == "txt")
        })
        .collect();
    files.sort();
    let mut h = 0u64;
    for p in files {
        if let Ok(text) = std::fs::read_to_string(&p) {
            h = h.rotate_left(17) ^ crate::lint_ai::token_seed(&text);
        }
    }
    h
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
    model_dir().join(format!("{lang}.learned.bin"))
}

/// Load a language's cached learned catalog, or `None` if absent/unreadable/stale.
///
/// Staleness is probed on the raw PREFIX before the full parse: the catalog is multi-megabyte,
/// and `version`/`train_version` serialize first (struct order, compact JSON), so a catalog
/// from older reading logic is rejected for the cost of a `read` — deserializing 13 MB just to
/// discover `current()` is false was a full third of every warm run after a `TRAIN_VERSION`
/// bump. Callers still call [`LearnedCatalog::current`] for the toolchain-version half.
fn load_cache(lang: &str) -> Option<LearnedCatalog> {
    if let Ok(bytes) = std::fs::read(cache_path(lang)) {
        // The container stamp leads with the training version: a stale catalog is rejected
        // for the cost of the header parse, before any inflation.
        let header = crate::lint_codec::probe(&bytes)?;
        if header.stamp.split('\u{1f}').next() != Some(TRAIN_VERSION) {
            return None;
        }
        let (_, mut d) = crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::LEARNED)?;
        return LearnedCatalog::dec(&mut d);
    }
    // Legacy JSON catalog: same prefix discipline, then migrate on sight.
    let raw = std::fs::read_to_string(cache_path(lang).with_extension("json")).ok()?;
    let head = raw.get(..512).unwrap_or(&raw);
    if !head.contains(&format!("\"train_version\":\"{TRAIN_VERSION}\"")) {
        return None;
    }
    let cat: LearnedCatalog = serde_json::from_str(&raw).ok()?;
    save_cache(lang, &cat);
    Some(cat)
}

/// Persist a learned catalog so the next run loads instead of relearning.
fn save_cache(lang: &str, cat: &LearnedCatalog) {
    let stamp = format!("{}\u{1f}{}\u{1f}{}", cat.train_version, cat.version, cat.sources_fp);
    save_bin(&cache_path(lang), crate::lint_codec::kind::LEARNED, &stamp, cat);
}

/// SETUP-TIME sweep of the model cache: migrate or delete every legacy artifact the load
/// paths would otherwise never touch again — a fresh machine never re-reads a stale file, so
/// without this the JSON era would sit on disk forever (LINTER.md, "Save": exactly one copy).
/// Owned file families only; returns how many files were migrated away or deleted. Lint runs
/// never call this — setup mutates, lint replays.
pub fn sweep_legacy_cache(data_root: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(model_dir()) else { return 0 };
    let registered = registered_languages(data_root);
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let module_lang = name.strip_suffix(".module.json").map(str::to_string);
        let learned_lang = name.strip_suffix(".learned.json").map(str::to_string);
        let gone = if name.ends_with(".json") && name.contains(".overlay-") {
            // Overlays recompile on demand; a legacy JSON overlay has no reader anymore.
            std::fs::remove_file(&path).is_ok()
        } else if name.ends_with(".patterns.json") || name.ends_with(".patterns.stamp") || name == "index.json" {
            // File families from pre-module eras — no code reads them.
            std::fs::remove_file(&path).is_ok()
        } else if let Some(lang) = module_lang {
            // A registered language's module migrates (the load path saves the container and
            // deletes the JSON); an unregistered language's is dead — its successor name
            // retrains from the shared page cache, never from this file.
            if registered.contains(&lang) {
                let _ = load_module(&lang);
            }
            if path.exists() {
                let _ = std::fs::remove_file(&path);
            }
            !path.exists()
        } else if let Some(lang) = learned_lang {
            if registered.contains(&lang) {
                let _ = load_cache(&lang);
            }
            // A stale or unregistered catalog is point-in-time reading the crawl page cache
            // can always regenerate — the file is dead either way.
            if path.exists() {
                let _ = std::fs::remove_file(&path);
            }
            !path.exists()
        } else if name == "polarity.global.json" {
            crate::lint_docs::migrate_global_polarity()
        } else {
            false
        };
        if gone {
            removed += 1;
        }
    }
    removed
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
    // The substrates shape comprehension (page reading forms units through the character
    // brain's meaning network and learned structural roles — LINTER.md, "Reading a page is
    // UNDERSTANDING"), so a rebuilt brain must recompile what was read through the old one.
    h.update(file_state(&model_dir().join("char.global.bin")).to_le_bytes());
    h.update(file_state(&model_dir().join("english.global.bin")).to_le_bytes());
    for s in lint_index_states(data_root).iter() {
        h.update(s.to_le_bytes());
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

/// The sorted `(mtime, len)` states of every JSON data file under the two lint-index
/// directories — the overlay stamp's shared input. Computed ONCE per data root and memoized
/// (fingerprinted by the directories' own mtimes so a long-lived process stays correct):
/// every language's stamp reads the same directories, and re-walking them per language was a
/// measurable slice of every warm run.
fn lint_index_states(data_root: &Path) -> std::sync::Arc<Vec<u128>> {
    type Cache = std::sync::Mutex<Option<((PathBuf, u128), std::sync::Arc<Vec<u128>>)>>;
    static CACHE: Cache = std::sync::Mutex::new(None);
    let dirs = [data_root.join("lint-index"), lint_index_cache_dir()];
    let dir_mtimes: u128 = dirs
        .iter()
        .map(|d| {
            std::fs::metadata(d)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|t| t.as_nanos())
                .unwrap_or(0)
        })
        .fold(0u128, |acc, t| acc.rotate_left(9) ^ t);
    let key = (data_root.to_path_buf(), dir_mtimes);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((have, states)) = cache.as_ref() {
        if *have == key {
            return states.clone();
        }
    }
    let mut states: Vec<u128> = Vec::new();
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        states.extend(
            entries
                .flatten()
                .filter(|e| e.file_name().to_str().is_some_and(|n| n.ends_with(".json")))
                .map(|e| file_state(&e.path())),
        );
    }
    states.sort_unstable();
    let states = std::sync::Arc::new(states);
    *cache = Some((key, states.clone()));
    states
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
    use super::{merge_graduated, DocRule};

    /// A minimal ledger-shaped [`DocRule`] keyed by construct `id` and its `source` page.
    fn ledger_rule(id: &str, source: &str) -> DocRule {
        DocRule {
            id: id.to_string(),
            slice: "medium".into(),
            severity: "medium".into(),
            description: format!("Never use `{id}`."),
            bad: format!("{id} x;"),
            good: "y;".into(),
            source: source.into(),
            construct: Some(id.to_string()),
        }
    }

    /// Item 3c — the re-check's three outcomes, driven by a PERTURBED corpus. The fresh pass carries
    /// only `A` (re-proven). Two prior ledger rules — `B` whose page is still in the corpus (a
    /// contradiction: re-read, did not re-prove) and `C` whose page has LEFT the corpus (a subset
    /// crawl) — must resolve as DROP and RETAIN respectively, and never the reverse.
    #[test]
    fn merge_drops_a_contradicted_rule_and_retains_one_whose_page_left_the_corpus() {
        let fresh = vec![ledger_rule("A", "https://docs.test/a")];
        let prior = vec![
            ledger_rule("B", "https://docs.test/b"),
            ledger_rule("C", "https://docs.test/c"),
        ];
        // The PERTURBATION: B's page is still read this crawl (so its absence from `fresh` is a genuine
        // failure to re-prove); C's page was NOT fetched this crawl.
        let corpus: std::collections::HashSet<String> =
            ["https://docs.test/a", "https://docs.test/b"].iter().map(|s| s.to_string()).collect();

        let (merged, dropped) = merge_graduated(fresh, prior, &corpus);
        let kept: std::collections::HashSet<&str> = merged.iter().map(|r| r.id.as_str()).collect();

        assert!(kept.contains("A"), "re-proven rule retained (fresh wins)");
        assert!(!kept.contains("B"), "contradicted rule (page re-read, no re-prove) DROPPED");
        assert!(kept.contains("C"), "rule whose page left the corpus RETAINED (retain-and-grow)");
        assert_eq!(
            dropped,
            vec![("B".to_string(), "https://docs.test/b".to_string())],
            "the contradiction is recorded, never a silent drop"
        );
    }

    /// The reshape half: when the fresh pass RE-PROVES a construct with a changed understanding, the
    /// fresh (reshaped) rule WINS over the stale ledger copy — never the old text kept, never a duplicate.
    #[test]
    fn merge_lets_a_reshaped_fresh_rule_win_over_the_stale_ledger_copy() {
        let mut reshaped = ledger_rule("A", "https://docs.test/a");
        reshaped.description = "A reshaped understanding of `A`.".into();
        let prior = vec![ledger_rule("A", "https://docs.test/a")]; // stale text, same construct id
        let corpus: std::collections::HashSet<String> =
            ["https://docs.test/a"].iter().map(|s| s.to_string()).collect();

        let (merged, dropped) = merge_graduated(vec![reshaped], prior, &corpus);
        assert_eq!(merged.len(), 1, "no duplicate — one construct, one rule");
        assert_eq!(merged[0].description, "A reshaped understanding of `A`.", "fresh reshape wins");
        assert!(dropped.is_empty(), "a re-proven reshape is agreement, not a contradiction");
    }

    /// Item 3d — the TRAINING loop is at fixpoint. Re-running training over an unchanged corpus re-proves
    /// the same set: merging a fresh pass with a prior ledger EQUAL to it (every construct re-proven, every
    /// source page still in the corpus) returns that set unchanged with ZERO contradictions. The proven set
    /// does not oscillate — a second retrain against the same knowledge is a no-op.
    #[test]
    fn re_training_over_an_unchanged_corpus_is_a_fixpoint() {
        let fresh = vec![ledger_rule("A", "https://docs.test/a"), ledger_rule("B", "https://docs.test/b")];
        let prior = fresh.clone(); // last retrain's ledger, identical because the corpus did not change
        let corpus: std::collections::HashSet<String> =
            ["https://docs.test/a", "https://docs.test/b"].iter().map(|s| s.to_string()).collect();

        let (merged, dropped) = merge_graduated(fresh.clone(), prior, &corpus);
        let mut merged_ids: Vec<&str> = merged.iter().map(|r| r.id.as_str()).collect();
        merged_ids.sort();
        assert_eq!(merged_ids, vec!["A", "B"], "the proven set is stable across an unchanged retrain");
        assert_eq!(merged.len(), fresh.len(), "no growth, no duplication — a fixpoint");
        assert!(dropped.is_empty(), "nothing contradicts itself over the same corpus");
    }

    /// Law is found from a SUBDIRECTORY: a project's `.helpers/lint-rules` and root `lintPref`
    /// govern a lint run rooted deep inside the project, and the walk stops at the repo root.
    #[test]
    fn law_is_found_walking_up_to_the_repo_root() {
        let base = std::env::temp_dir().join(format!("law-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        let deep = repo.join("crates").join("inner").join("src");
        std::fs::create_dir_all(&deep).expect("deep dir");
        std::fs::create_dir_all(repo.join(".git")).expect(".git marks the repo root");
        std::fs::create_dir_all(repo.join(".helpers/lint-rules")).expect("law dir");
        std::fs::write(repo.join(".helpers/lint-rules/any.md"), "## no_foo\nNever use foo.\n")
            .expect("law file");
        std::fs::write(repo.join("lintPref.md"), "Never use bar anywhere.\n").expect("lintpref");
        // A sibling ABOVE the repo root must never be swept in.
        std::fs::create_dir_all(base.join(".helpers/lint-rules")).expect("outside dir");
        std::fs::write(base.join(".helpers/lint-rules/any.md"), "## outside\nNever use outside.\n")
            .expect("outside law");

        let found = super::rule_documents(&deep);
        let paths: Vec<String> = found.iter().map(|(p, _)| p.display().to_string()).collect();
        assert!(
            paths.iter().any(|p| p.ends_with("repo/.helpers/lint-rules/any.md")),
            "project law must be found from a subdir: {paths:?}"
        );
        assert!(
            paths.iter().any(|p| p.ends_with("repo/lintPref.md")),
            "root lintPref must be found from a subdir: {paths:?}"
        );
        assert!(
            !paths.iter().any(|p| p.contains("outside")),
            "the walk must stop at the repo root, not climb into unrelated parents: {paths:?}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// DEV TOOL — print a cached module's compiled rules and its learned memory's verdicts,
    /// grouped by source page: the "what did the reading actually extract?" probe live-docs
    /// validation reads. `PROBE_LANG=<lang> cargo test --release --lib probe_module_rules
    /// -- --ignored --nocapture` (honors `HELPERS_LINT_MODELS`).
    #[test]
    #[ignore = "dev tool: inspect a trained module's rules per source page"]
    fn probe_module_rules() {
        let lang = std::env::var("PROBE_LANG").expect("PROBE_LANG");
        let bytes = std::fs::read(super::module_path(&lang)).expect("module file");
        let (stamp, mut d) =
            crate::lint_codec::Dec::open(&bytes, crate::lint_codec::kind::MODULE).expect("container");
        let module = <super::Module as crate::lint_codec::Bin>::dec(&mut d).expect("decodes");
        println!("stamp={stamp}");
        println!("{}", serde_json::to_string(&module.rules).expect("serializes"));
        // The learned memory behind the module: classifier state, per-word tallies
        // (`PROBE_WORDS=a,b,c`), and each binding's classify verdict — the live-docs
        // quality view ("which prose minted, and why").
        let lb = std::fs::read(super::cache_path(&lang)).expect("learned file");
        let (_, mut ld) =
            crate::lint_codec::Dec::open(&lb, crate::lint_codec::kind::LEARNED).expect("open");
        let cat = <super::LearnedCatalog as crate::lint_codec::Bin>::dec(&mut ld).expect("dec");
        let Some(mem) = &cat.memory else { return };
        let Some(pol) = &mem.polarity else { return };
        eprintln!(
            "memory: bindings={} reference={} flagged={} ready={} votes={}",
            mem.bindings.len(),
            mem.reference.len(),
            mem.flagged.len(),
            pol.is_ready(),
            pol.votes()
        );
        if let Ok(words) = std::env::var("PROBE_WORDS") {
            for w in words.split(',') {
                eprintln!("tally {w:?}: {:?} lean={:?}", pol.tally_of(w), pol.tally_lean(w));
            }
        }
        let mut minted = 0usize;
        for b in &mem.bindings {
            if pol.classify(&b.prose) == Some(true) {
                minted += 1;
                if minted <= 10 {
                    eprintln!("MINT {:?}", &b.prose[..b.prose.len().min(90)]);
                }
            }
        }
        eprintln!("bindings classifying prohibition: {minted}/{}", mem.bindings.len());
        // Per-domain binding/mint counts — the crawl-coverage view.
        {
            let mut by: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
            for b in &mem.bindings {
                let dom = b.url.split('/').nth(2).unwrap_or("?").to_string();
                let e = by.entry(dom).or_insert((0, 0));
                e.0 += 1;
                if pol.classify(&b.prose) == Some(true) {
                    e.1 += 1;
                }
            }
            let pages: std::collections::HashSet<&str> =
                mem.bindings.iter().map(|b| b.url.as_str()).collect();
            eprintln!("distinct pages: {}", pages.len());
            for (d, (n, m)) in by {
                eprintln!("domain {d}: bindings={n} minting={m}");
            }
        }
        // Sample of NON-minting bindings from each source (PROBE_MISS=n).
        if let Ok(n) = std::env::var("PROBE_MISS") {
            let n: usize = n.parse().unwrap_or(0);
            for b in mem.bindings.iter().filter(|b| b.url.contains("eslint")).take(n) {
                eprintln!("MISS {:?} -> {:?}", &b.prose[..b.prose.len().min(110)], pol.classify(&b.prose));
            }
        }
        // Verdict distribution over a fresh grounding sample (PROBE_GROUND=n).
        if let Ok(n) = std::env::var("PROBE_GROUND") {
            let n: usize = n.parse().unwrap_or(0);
            let data_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
            let (mut f, mut c, mut u) = (0, 0, 0);
            for b in mem.bindings.iter().take(n) {
                match crate::lint_toolchain::check(&lang, &b.code, data_root) {
                    crate::lint_toolchain::Verdict::Flagged => f += 1,
                    crate::lint_toolchain::Verdict::Clean => c += 1,
                    crate::lint_toolchain::Verdict::Unknown => u += 1,
                }
            }
            eprintln!("fresh grounding over first {n}: flagged={f} clean={c} unknown={u}");
        }
    }





    use super::*;

    /// Encode → decode → compare as serde JSON values: semantic equality over every field,
    /// hypervectors included, without hand-writing `PartialEq` across the model types.
    fn assert_round_trip<T: crate::lint_codec::Bin + serde::Serialize>(value: &T, kind: u8, what: &str) -> T {
        let mut e = crate::lint_codec::Enc::new();
        value.enc(&mut e);
        let bytes = e.finish(kind, "stamp");
        let (_, mut d) = crate::lint_codec::Dec::open(&bytes, kind).expect("container opens");
        let decoded = T::dec(&mut d).unwrap_or_else(|| panic!("{what} decodes"));
        assert_eq!(
            serde_json::to_value(value).expect("fixture serializes"),
            serde_json::to_value(&decoded).expect("decoded serializes"),
            "{what} must survive its HLM1 round trip with every field intact"
        );
        decoded
    }

    /// One fixture per artifact struct, round-tripped through its `HLM1` container — the
    /// "100% verified" contract for the wire format itself (LINTER.md, "Save"). The module
    /// fixture carries a real compiled detector, a real concept gate, and a trained polarity
    /// classifier so every stream (raw hypervectors, integer arrays, deflated text) is hit.
    #[test]
    fn hlm1_round_trips_every_artifact_struct() {
        use crate::lint_codec::kind;
        let rules = crate::lint_match::RuleSet::build(
            "zetalang",
            &[(
                "no_zorkle".to_string(),
                "high".to_string(),
                "zorkle cleanup".to_string(),
                "loop { step() }".to_string(),
                "Never use the zorkle statement anywhere; it is deprecated.".to_string(),
                "https://registry.example/zetalang".to_string(),
                None,
            )],
            &crate::lint_match::Grounding {
                reference: vec!["loop { step() }".to_string(), "emit(\"done\")".to_string()],
                ..Default::default()
            },
        );
        assert!(rules.rule_count() > 0, "fixture must compile a detector");
        let concept = crate::lint_ai::ConceptModel::compile(
            &[("no_zorkle".to_string(), "Never use zorkle".to_string(), "zorkle cleanup".to_string())],
            "zetalang",
        );
        let polarity = crate::lint_read::Polarity::from_labeled(&[
            ("never use the zorkle statement", true),
            ("the emit call is idiomatic and encouraged", false),
        ]);
        let module = Module {
            version: "1.2.3".to_string(),
            train_version: TRAIN_VERSION.to_string(),
            sources_fp: "abcd".to_string(),
            trained_at: 7,
            verified_at: 9,
            learned_from: "docs".to_string(),
            extensions: [("zl".to_string(), 3u32)].into_iter().collect(),
            brain_fp: 0,
            rules,
            concept,
        };
        assert_round_trip(&module, kind::MODULE, "Module");

        let overlay = Overlay {
            stamp: "sha256:feed".to_string(),
            unenforced: vec!["law_x".to_string()],
            concept: crate::lint_ai::ConceptModel::compile(&[], "zetalang"),
            rules: crate::lint_match::RuleSet::build("zetalang", &[], &Default::default()),
        };
        assert_round_trip(&overlay, kind::OVERLAY, "Overlay");

        let catalog = LearnedCatalog {
            version: "1.2.3".to_string(),
            train_version: TRAIN_VERSION.to_string(),
            sources_fp: "abcd".to_string(),
            learned_from: "docs".to_string(),
            rules: vec![DocRule {
                id: "no_zorkle".to_string(),
                slice: "statements".to_string(),
                severity: "high".to_string(),
                description: "Never use zorkle — naïve ✓ utf8".to_string(),
                bad: "zorkle".to_string(),
                good: "emit()".to_string(),
                source: "https://registry.example".to_string(),
                construct: None,
            }],
            reference: vec!["emit(\"done\")".to_string()],
            memory: Some(crate::lint_read::Memory {
                bindings: Vec::new(),
                reference: vec!["loop { step() }".to_string()],
                polarity: Some(polarity),
                pages_read: 4,
                flagged: [1u64, u64::MAX].into_iter().collect(),
                extensions: [("zl".to_string(), 2u32)].into_iter().collect(),
            }),
        };
        assert_round_trip(&catalog, kind::LEARNED, "LearnedCatalog");
    }

    /// DEV VERIFICATION — sweeps every legacy JSON artifact still in this machine's model
    /// cache through the container and asserts semantic equality, artifact by artifact:
    /// `cargo test --release --lib hlm1_sweeps_machine_cache -- --ignored --nocapture`.
    /// Run BEFORE migrating a machine; it proves the codec against every real trained model,
    /// not just fixtures.
    #[test]
    #[ignore = "dev verification: needs a machine cache with legacy JSON artifacts"]
    fn hlm1_sweeps_machine_cache() {
        use crate::lint_codec::kind;
        let dir = model_dir();
        let mut checked = 0usize;
        for entry in std::fs::read_dir(&dir).expect("model cache exists").flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(raw) = std::fs::read_to_string(entry.path()) else { continue };
            if name.ends_with(".module.json") {
                let m: Module = serde_json::from_str(&raw).expect("module JSON parses");
                assert_round_trip(&m, kind::MODULE, &name);
            } else if name.ends_with(".learned.json") {
                let c: LearnedCatalog = serde_json::from_str(&raw).expect("catalog JSON parses");
                assert_round_trip(&c, kind::LEARNED, &name);
            } else if name == "english.global.json" {
                let e: crate::lint_english::English = serde_json::from_str(&raw).expect("english JSON parses");
                assert_round_trip(&e, kind::ENGLISH, &name);
            } else if name == "polarity.global.json" {
                let p: crate::lint_read::Polarity = serde_json::from_str(&raw).expect("polarity JSON parses");
                assert_round_trip(&p, kind::POLARITY, &name);
            } else {
                continue;
            }
            checked += 1;
            println!("verified {name}");
        }
        println!("verified {checked} artifacts under {}", dir.display());
    }

    /// DEV BENCH — decode latency and size of every artifact on this machine, container vs
    /// legacy JSON: `cargo test --release --lib hlm1_bench_decode -- --ignored --nocapture`.
    #[test]
    #[ignore = "dev bench: needs a machine cache with real artifacts"]
    fn hlm1_bench_decode() {
        use crate::lint_codec::{kind, Bin as _, Dec, Enc};
        fn bench<T: crate::lint_codec::Bin>(name: &str, kind: u8, json: &str, value: &T) {
            let mut e = Enc::new();
            value.enc(&mut e);
            let bytes = e.finish(kind, "stamp");
            let start = std::time::Instant::now();
            let iters = 20u32;
            for _ in 0..iters {
                let (_, mut d) = Dec::open(&bytes, kind).expect("opens");
                assert!(T::dec(&mut d).is_some());
            }
            let bin_us = start.elapsed().as_micros() / u128::from(iters);
            println!(
                "{name}: json {} KB -> bin {} KB ({:.1}x), decode {} µs",
                json.len() / 1024,
                bytes.len() / 1024,
                json.len() as f64 / bytes.len() as f64,
                bin_us,
            );
        }
        for entry in std::fs::read_dir(model_dir()).expect("model cache").flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(raw) = std::fs::read_to_string(entry.path()) else { continue };
            if name.ends_with(".module.json") {
                let m: Module = serde_json::from_str(&raw).expect("parses");
                bench(&name, kind::MODULE, &raw, &m);
            } else if name.ends_with(".learned.json") {
                let c: LearnedCatalog = serde_json::from_str(&raw).expect("parses");
                bench(&name, kind::LEARNED, &raw, &c);
            } else if name == "english.global.json" {
                let e: crate::lint_english::English = serde_json::from_str(&raw).expect("parses");
                bench(&name, kind::ENGLISH, &raw, &e);
            }
        }
    }

    /// Ledger #18's gate, as a table: a hint that resolves to a KNOWN different language is
    /// foreign; the training language's own aliases are not; junk fence labels are no hint at
    /// all (excluding on them would silently discard real examples).
    #[test]
    fn foreign_example_gate_trusts_only_known_language_hints() {
        for (lang, hint, want) in [
            ("javascript", "日本語", false), // multibyte label: no hint, and NEVER a panic
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
    fn the_extension_map_round_trips_binary_and_migrates_legacy_json() {
        let dir = std::env::temp_dir().join(format!("ext-map-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("extensions.bin");
        // Legacy JSON reads as a fallback…
        std::fs::write(
            path.with_extension("json"),
            r#"{"vexlang":{"vex":7,"vx":2}}"#,
        )
        .unwrap();
        let legacy = read_extension_map(&path);
        assert_eq!(legacy["vexlang"]["vex"], 7, "legacy JSON must stay readable");
        // …and the binary form round-trips exactly.
        let mut e = crate::lint_codec::Enc::new();
        e.u(1);
        e.str("vexlang");
        e.u(2);
        e.str("vex");
        e.u(7);
        e.str("vx");
        e.u(2);
        std::fs::write(&path, e.finish(crate::lint_codec::kind::EXTMAP, TRAIN_VERSION)).unwrap();
        let bin = read_extension_map(&path);
        assert_eq!(bin, legacy, "binary and legacy forms decode identically");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PASS 33 — the stale-daemon rollback class: a writer whose stamp carries an OLDER
    /// `docs-vNN` ordinal than the artifact on disk must be refused; every other write
    /// (same, newer, either side ordinal-free, no file yet) proceeds.
    #[test]
    fn knowledge_writes_are_train_ordinal_monotonic() {
        assert_eq!(super::train_ordinal("docs-v97-pseudo-shape"), Some(97));
        assert_eq!(super::train_ordinal("docs-v92-graded-tier\u{1f}23.9.0\u{1f}ab"), Some(92));
        assert_eq!(super::train_ordinal("23.9.0"), None, "a toolchain stamp carries no ordinal");

        let dir = std::env::temp_dir().join(format!("stamp-monotonic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("x.web.bin");
        let write = |stamp: &str| {
            let e = crate::lint_codec::Enc::new();
            std::fs::write(&path, e.finish(crate::lint_codec::kind::WEB, stamp)).unwrap();
        };
        assert!(!super::stamp_regression(&path, "docs-v92-x"), "no file yet — write allowed");
        write("docs-v97-pseudo-shape");
        assert!(super::stamp_regression(&path, "docs-v92-graded-tier"), "older writer REFUSED");
        assert!(!super::stamp_regression(&path, "docs-v97-pseudo-shape"), "same version allowed");
        assert!(!super::stamp_regression(&path, "docs-v98-next"), "newer writer allowed");
        assert!(!super::stamp_regression(&path, "23.9.0"), "ordinal-free writer abstains");
        write("23.9.0");
        assert!(!super::stamp_regression(&path, "docs-v92-x"), "ordinal-free store abstains");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PASS 33 — per-URL source identity: a manifest URL the registry also names keeps the
    /// registry's tool id (stable crawl-cache name); only a URL the registry does not carry is
    /// keyed by [`manifest_tool`]. The registry URL ABSENT from the manifest stays uncrawled
    /// (the manifest is the user's word).
    #[test]
    fn manifest_urls_keep_their_registry_tool_identity() {
        let dir = std::env::temp_dir().join(format!("per-url-tools-{}", std::process::id()));
        let _env = crate::test_env_lock();
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &dir);
        std::fs::create_dir_all(dir.join(".config/helpers")).unwrap();
        std::fs::create_dir_all(dir.join("lint-index")).unwrap();
        std::fs::write(
            dir.join("lint-index/sources.json"),
            r#"{"sources":[
                {"tool":"shard-a","language":"shardlang","kind":"crawl","seed":"https://docs.shard/a/"},
                {"tool":"shard-b","language":"shardlang","kind":"crawl","seed":"https://docs.shard/b/"}
            ]}"#,
        )
        .unwrap();
        // The manifest keeps a/, drops b/ (the user's word), adds a novel URL.
        std::fs::write(
            dir.join(".config/helpers/languages.json"),
            r#"{"languages":{"shardlang":["https://docs.shard/a/","https://docs.shard/extra/"]}}"#,
        )
        .unwrap();
        let sources = super::resolved_sources_uncached(&dir, "shardlang");
        match old_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
        let extra_tool = super::manifest_tool("https://docs.shard/extra/");
        let tools: Vec<(&str, &str)> =
            sources.iter().map(|s| (s.url.as_str(), s.tool.as_str())).collect();
        assert_eq!(
            tools,
            vec![
                ("https://docs.shard/a/", "shard-a"),
                ("https://docs.shard/extra/", extra_tool.as_str()),
            ],
            "registry URL keeps its tool id; novel URL is manifest-keyed; dropped URL stays dropped"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stale_module_still_enforces_offline_and_is_reported_outdated() {
        // LINTER.md, "Lint never learns from the network": a replay-only run must serve a
        // stale module AS-IS (old reading beats no reading) and name the language in
        // `TrainReport::outdated`, never degrade it to "not set up".
        let dir = std::env::temp_dir().join(format!("stale-module-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _env = crate::test_env_lock();
        std::env::set_var("HELPERS_LINT_MODELS", &dir);
        let lang = "stalelang";
        let ground = crate::lint_match::Grounding::default();
        let rules = RuleSet::build(
            lang,
            &[(
                "no_zap".into(),
                "high".into(),
                "zap left".into(),
                "zip left".into(),
                "Never use the zap statement anywhere; it is deprecated and will be removed.".into(),
                "https://d/zap".into(),
                None,
            )],
            &ground,
        );
        assert!(rules.rule_count() > 0, "fixture rule compiles");
        let stale = Module {
            version: String::new(),
            train_version: "docs-v0-ancient".into(), // NOT the current TRAIN_VERSION
            sources_fp: sources_fingerprint(&dir, lang),
            trained_at: 1,
            verified_at: 1,
            learned_from: "docs".into(),
            extensions: Default::default(),
            brain_fp: 0,
            concept: ConceptModel { rules: Vec::new() },
            rules,
        };
        save_module(lang, &stale);
        let (report, models) =
            ensure_models(&[lang.to_string()], &dir, &dir, &NoProject);
        std::env::remove_var("HELPERS_LINT_MODELS");
        assert!(
            report.outdated.contains(&lang.to_string()),
            "stale module must be reported outdated: {report:?}"
        );
        let model = models.get(lang).expect("stale module still loads and enforces");
        assert!(model.rules.rule_count() > 0, "the stale rules still fire");
        let _ = std::fs::remove_dir_all(&dir);
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

