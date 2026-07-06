# The AI Linter — how it works

> The single authoritative description of the lint system. Module headers in `native/src/`
> summarize their own file and point here; this document owns the cross-module theory. **No
> semantic change lands without updating this file first** — every regression this system has
> had came from editing behavior without a written model of it.

## Thesis

**English is read; code is linted — and common language is learned FIRST.** The system builds a
baseline understanding of English by reading (that understanding is the substrate), then learns
each code language *through* that understanding by reading its documentation, and finally
enforces only things that state a violation. The baseline is not optional and not implicit: a
reader fed only technical documentation believes "never" and "import" are rare words (measured:
165 and 402 reads in a 637k-token docs-fed reader whose Zipf head cutoff was 691 — both
escaped every commonness judgment), so before any docs are read, the substrate reads the
machine's own dictionary (see "Common language first" below). Nothing downstream can be smarter
than this baseline: construct selection, polarity, and salience all ask what the reading can
account for, and the answer is only meaningful if common English was read first. Comprehension and enforcement are different acts: teaching material, concept prose,
and remedy clauses are *read* — they train the reader and the classifier — but never fire.

The substrate is a 1-bit hyperdimensional AI, not an LLM and not a rule program:

- 8192-bit binary hypervectors; the only operations are XOR, rotate, per-bit majority, and
  Hamming distance (`lint_ai.rs`). Deterministic, no floats, no backprop.
- The **Reader** (`lint_read.rs`) is a sequential predictive coder: rolling context
  `ctx' = ρ(ctx) ⊕ hv(token)`, an associative memory updated only on prediction error, and
  learned token frequencies (the corpus-derived stop-list).
- The **Polarity classifier**: two prototype hypervectors (prohibition / endorsement)
  accumulated from grounded reading; a span classifies by per-token votes with a margin
  *calibrated from the prototypes' own measured noise floor* — no hand-tuned constant.
  Votes are **information-weighted**: each word weighs its Shannon self-information in
  integer bits (−log₂ of its learned frequency), so the most common words weigh almost
  nothing, the mid-band matters, and rare words carry the meaning — a reverse-logarithmic
  curve learned from reading, applied identically when accumulating the prototypes and when
  voting. This is what stops neutral manual prose ("Interactive shells permit trapping
  signals…") from classifying as law off the back of common register words.

Measured (Apple Silicon, `cargo run --release --example reader_bench`, over the shipped
`extraDocs/*.md` teaching prose): reading ≈ 2.3M tokens/s; polarity training ≈ 80k labeled
sentences/s; classification ≈ 390k sentences/s. `hv_bench`: the batched GPU Hamming grid
overtakes the CPU fold at ≈3M query×key pairs (auto-dispatched; correctness identical either
side). The AI is never the slow part.

## What is learned vs what is programmed

This boundary is the answer to "is it an AI or a program?" — kept honest by inspection.

**Learned (data; changes by reading, never by code edit):**
vocabulary and its frequencies (self-defining: any string hashes to a code; nothing is
enumerated); the common-language brain (dictionary-read frequencies + the set of words the
dictionary defines); the polarity prototypes; every rule (read from docs/law files); every compiled
detector (derived from the rule's own words/examples); the grounding corpora (docs' example
code, the project's own sources); the concept fingerprints; the feedback suppressions.

**Programmed (mechanism; contains no vocabulary, no rule content):**
typography (whitespace/case/digit word boundaries, CJK per-character morphemes, sentence
punctuation followed by whitespace, markdown fences, HTML tag boundaries); the evidence
ordering (below); trust structure (law-by-location; teaching-is-not-law); evidence thresholds
(2-flag suppression, quarantine rates, `MAX_EXAMPLE_BYTES`, Zipf-head mass). Thresholds are
the grey zone — they are hyperparameters, but none encodes content, and every outcome above
them shifts when the reading shifts.

**Anti-cheat instruments** (how we know tests measure learning, not memorization): the
contract suite (`native/tests/ai_linter_behaviors.rs`) runs the *built binary* on **invented
languages** (`zlang`, `qlang`) and a **localhost-served fictional docs site** (`flowlang`) —
nothing in the codebase can know them; the "Your law, as understood" block plus the `⟨source⟩`
citation on every finding make provenance visible at runtime; `TRAIN_VERSION` invalidates all
caches whenever reading logic changes, so nothing stale can masquerade as learning.

**Tests are generative, not hand-fed** — the same principle as the training data: a suite that
needs a new hand-written test per bug is as unmaintainable as a rule catalog that needs a new
entry per rule. Correctness is asserted as *invariants over tables* (construct shapes × law
phrasings × grounding styles × string/comment traps in `lint_match/tests.rs`; a planted
law×violation matrix asserted by exact issue count against the built binary in
`ai_linter_behaviors.rs::planted_violations_are_flagged_exactly_no_more_no_less` — the
zero-FP/zero-FN contract, executable). A new bug class becomes a table row, never a test
function; every ledger dimension stays pinned by the cartesian product.

## Sources of law (exactly three) and sources of reading

**Enforcement grows purely from reading.** There is no curated rule catalog anywhere — the
former `extraDocs/lint-corpus.jsonl` (hand-maintained linter rules) is deleted; correctness is
learned by reading official documentation and grounded against the installed toolchain, never
authored. The polarity classifier's only labels are toolchain verdicts on real code. The
architecture holds the entropy; every source below is DATA — adding a rule, a principle, or a
language is a file edit, never a code change.

| Law | Trust |
|---|---|
| Project rule files: `.helpers/lint-rules/*.{md,txt}`, root `lintPref.{md,txt}` | Absolute ("law by location"): never polarity-gated, never Hv-gated, never quarantined |
| The corpus folder: `<data_root>/corpus/*.{md,txt}` — machine-global CS-principles rule documents (CS2420/CS3500 canon), read through the same document reader as project law (stem = language, `any` = every code language) | Law by location at COMPILE time (a heading-per-rule document is deliberate — ledger #13's tutorial-narration risk does not exist, and measured: the entry gate rejected 3 of 4 curated principles and reference-fire killed the fourth on the docs' own bad-form examples). At RUN time it is NOT the project's own law: quarantinable, imprecise matches Hv-gated, 2-flag suppressible — its blast radius is every project on the machine, so the runtime nets stay |

A rule file's stem is the language it governs, and extension aliases resolve through the same
map the file walker uses (`js.md` ⇒ javascript, `py.md` ⇒ python — ledger #16). A law file whose
language matches no file in the project is REPORTED as inert ("governs 'x' — no x files"), never
silently skipped.
| Official language documentation (crawled; registered in `lint-index/sources.json` or handed over via `add_source` — the system never searches the web for docs; normative style guides — PEP8, api-guidelines, effective_go — are where the practice rules live) | Gated: prohibition reading + grounding + self-fire + reference-fire + Hv gate + quarantine |

**Reading only (never rules):** `extraDocs/*.md` teaching prose, `lint-index/reading-sources.json`
corpora (Stack Overflow, Urban Dictionary — coder register), and all doc prose around examples.
Enforcing teaching material was a repeated noise source (see ledger) and is structurally off.

**The bar is parity with the built linters, from the LANGUAGE'S OWN documentation.** The
target is what ESLint flags for JavaScript and clippy for Rust — derived independently, by
reading the language's live official pages (MDN JavaScript, the Rust reference and
api-guidelines, docs.python.org + PEP8), never by ingesting another linter's rule list.
Sources are ACTUAL PAGE DOCUMENTATION: live official sites, crawled — not hand-written, not
gathered snapshots (linter-docs sources were tried and removed by directive: reading a
linter's catalog is derivative, and its "incorrect" examples are valid syntax that inverts
parse-grounding — see open problems). What the language's docs deprecate, warn about, and
forbid becomes the rule set — potentially ahead of the built linters, because the docs move
first. Then the project's plain-English law lands on top, through the very same reading. What
containment matching cannot carry (dataflow: unused variables, absence rules) stays in open
problems, not in silent false-negative territory.

**Setup guarantees the documentation is CURRENT.** The crawl page cache is re-validated at
setup time: when `action=train` runs (network allowed), any source whose cached pages are
older than a day is re-crawled before modules build — a project is never set up against stale
documentation, and lint runs stay replay-only against what setup ensured. `HELPERS_LINT_REFRESH`
forces it regardless of age.

## The per-language training pipeline

`sitemap → level-parallel map → read → ground → bind → compile → cache/save`

0. **Languages train in parallel** (one thread per language) and every crawled source is
   cached **once per machine** in `~/.cache/helpers/lint-index/crawls/<tool>.json` (the
   extracted per-page prose + example pairs, keyed by toolchain version). A source shared by
   two languages (TypeScript ⊇ JavaScript both read MDN) hits the network once — the second
   reader replays the cached pages at memory speed; a process-wide once-map prevents two
   parallel languages from double-crawling the same source. `HELPERS_LINT_REFRESH` recrawls.
   A language's sources are read **round-robin interleaved**, so the bounded-memory caps
   (`MAX_BINDINGS`, `MAX_REFERENCE`, grounding samples) are source-FAIR: no source can starve
   another by being read first (measured: MDN filled all 4,000 binding slots and ESLint's 300
   rule pages bound NOTHING — read and silently discarded). The learned catalog is keyed by
   toolchain version + `TRAIN_VERSION` + a **sources fingerprint** (the resolved source URLs),
   and registry entries carry the same fingerprint — adding or changing a docs source re-reads
   the language everywhere instead of reusing a catalog that never saw the new source.
1. **Map**: try `<origin>/sitemap.xml` (one request can enumerate the site), then balloon
   outward by levels — every link of a level fetched concurrently (64-wide waves), visited-set
   dedup, until the in-scope site is mapped. No pacing. Measured: the 123-page Rust reference +
   api-guidelines → **2.7s for the entire pipeline**. **The WHOLE in-scope site is crawled and
   every page's prose is read** — the caps (`MAX_CRAWL_PAGES`, `MAX_BINDINGS`,
   `MAX_REFERENCE`, `MAX_GROUND_CHECKS`) are runaway safety valves sized far above any real
   documentation site, never working limits that silently truncate knowledge (measured before
   this held: MDN's JavaScript tree is thousands of pages and a 700-page cap + 200-page read
   budget left most of it unread, making rule counts swing with crawl order). Scope is the seed's
   docs TREE, boundary-safe: a directory-like seed path scopes to itself (`…/c` covers `/c`
   and `/c/…`, never `/cpp`); a file seed (`…/bash.html`) scopes to its folder — the safety
   valves exist for exactly the day a seed mis-scopes.
2. **Read**: the Reader learns the prose; `(governing prose, code example)` pairs are sliced at
   *tag boundaries* (between `</pre>` and the next `<pre`) — never at byte offsets — and a
   HEADING is a hard boundary: the prose that governs a block never crosses into the
   previous section (`<h1>`–`<h6>`, pure typography). Measured on the diversity contract's
   MDN-reference shape: without the heading cut, a section's prohibition window swallowed
   the tail of the neutral intro above it, the mixed span classified as nothing, and the
   rule never minted — the same dilution real reference pages produce. **A
   blockless section cannot teach law yet** (built, measured, reverted — 2026-07-06):
   units exist only per `<pre>` block, so a section that states a prohibition in plain
   prose with no example block forms no unit and cannot mint — MDN's "Never use direct
   eval()!" (a heading, prose, bullets, zero `<pre>`) is structurally invisible, and its
   prose is tail-truncated into the NEXT block's window, cutting exactly the sentence that
   states the law. LEAD units (slug + head-kept section prose + no code, minting prose-only
   rules through the description path) were implemented and measured against the current
   span classifier, which cannot read that section either way: the whole span classifies
   ENDORSEMENT ("malicious … attacks" prose), the bare "Never use direct eval()" sentence
   ABSTAINS ("never"/"use" carry near-zero information weight), while error-register
   reference sections ("cannot be parsed…") classify prohibition as spans AND sentences —
   so lead units minted 8–14 junk prose-only rules (54–105 findings on one repo) and zero
   true ones. Both variants reverted; a regenerated bootstrap likewise failed its own
   contract test (canonical "Never use goto…" → abstain) and was reverted — MORE reading
   under the current accumulation makes the classifier worse, not better, because
   clean-parsing pages' warning prose feeds the endorsement prototype. Re-land lead units
   TOGETHER WITH the per-token side-count classifier (open problems; asymmetric grounding —
   Flagged is evidence, Clean only means "parses"), never before it. What remains landed:
   unit-former changes poison the crawl cache by
   format marker (`UNITS_FORMAT` folded into the cached version): cached pages store formed
   units, not HTML, so a former change re-crawls rather than silently keeping old units.
   **A site is
   never assumed to document one language** (ledger #18): a page's TREE names the language a
   source is registered for (the seed scopes the crawl), but the page's own code blocks declare
   their individual languages — `class="brush: js"`, `language-css`, the fence info string —
   and that declaration is honored. Each block's hint resolves through the same
   extension→language map law-file stems use (#16); a block hinted for a DIFFERENT known
   language is READ as prose context but never bound, never grounded, and never enters this
   language's reference corpus — an MDN JavaScript page's HTML example belongs to html, and
   binding it into javascript minted rules that fired cross-language (an MDN `input/password`
   HTML-page rule firing on `.mjs` files — measured). A hint that resolves to nothing known is
   no hint at all: junk fence labels ("output", "plain") must not silently discard real
   examples. Blocks with no hint attribute to the page's language, as before.
3. **Ground**: a bounded sample of examples is checked against the installed toolchain
   (parse/compile check only, never executed, parallel): flagged → prose feeds the bad
   prototype, clean → good. Docs' claims tested against reality.
4. **Bind**: prose⊗code hypervector bindings + the reference corpus ("what is normal here").
5. **Compile** (`RuleSet::build`): examples → lossless generalized AST patterns via
   `bad ∧ ¬good` tree-diff (operations exact, operands bound wildcards, literals typed
   wildcards) or a discriminating token sequence (a single distinctive token, or an ordered
   same-line pair); prose-only rules → a detector derived by the evidence hierarchy below.
   The tree-diff's descent to the smallest novel subtree stops one level above a CHILDLESS
   node: a bare leaf as pattern root is degenerate by definition (the same classification
   the reference-fire and quarantine tiers use), and stripping the leaf's context is what
   turned `items=[]`-as-default-parameter into "any empty list literal" — the compiled
   pattern flagged the rule's own remedy (`items = []` inside the None-guard). A novel
   subtree that still has children keeps rooting the pattern itself (`scores = [90, 85,
   77]`'s list still generalizes across contexts).
   Sequence selection prefers the most GENERAL detector that still discriminates: a pure
   `bad ∧ ¬good` pair (both tokens absent from the fix) first, then a single distinctive
   token, and only last a relaxed pair anchored on one token the fix shares — measured: with
   the relaxed pair tried before the single token, `no_var_declaration` (bad `var count = 1;`
   good `let count = 1;`) compiled to `var … count`, which fires on the example's own
   identifier and misses every real `var` line without a `count` beside it.
   **There is no regex engine and no shape catalog anywhere in the matcher**: a text detector
   IS its tokens, the tokens come from the reader's ONE tokenizer (`lint_read` word runs —
   ledger #2/#11: every token set the engine compares tokenizes the same way; the old
   example-diff tokenizer's enumerated operators/sigils/flags are deleted), and firing is
   whole-token containment on the lowercased surface (ledger #15) — a token edge that is a
   word character must not touch a word character. A `bad ∧ ¬good` difference that is pure
   punctuation or a bare numeric value yields no watchable word and the compile abstains —
   values and operators are semantics, and the AST diff is the path that carries them. One
   containment function serves the compile gates and the live fire, so the two can never
   disagree about what a detector means. An example-diff text detector must additionally be
   TRACEABLE (#19): its example was reality-FLAGGED by the toolchain, or the law's own words
   name at least one kept token (the anchor; an ordered pair's partner token only narrows
   firing, so it need not be named) — identifiers a Clean-parsing example happens to use are
   not evidence of anything. An example that is still the whole translation unit after
   wrapper-skipping is a **sample program, not a rule** — a rule is a construct a reader can
   point at, never a whole file — and compile abstains (tutorial hello-worlds once minted
   `first_statement_in_a_go`, which fired on any hello-world; ledger #13).
6. **Save — the artifact is an AI MODULE, and documentation is never saved as an artifact.**
   Training distills documentation into runnable bits (`~/.cache/helpers/lint-models/`,
   machine-global):
   - `<lang>.module.bin` — **the AI module**: the compiled doc-rule `RuleSet` (pattern
     engine) + `ConceptModel` (hypervector concept gate) + provenance
     (`toolchain @ sources @ TRAIN_VERSION @ trained_at`). Loaded every run; this — and ONLY
     this — is what the registry shares. Project-independent by construction (doc rules
     only); no prose, no examples, no corpus: the trained result and the timestamp that
     proves it current.
   - `<lang>.overlay-<project>.bin` — the PROJECT overlay: the project's law + the machine
     corpus principles, compiled locally against the project's own code (the law's primary
     grounding universe), this machine's reading memory when it has one, and the transferred
     polarity classifier; stamped by law rows + project fingerprint + module identity. At
     load time `overlay ⊕ module` merge (overlay first — trust order), and that merged
     engine lints.
   - `<lang>.learned.bin` — the local reading memory (bindings + reference corpus +
     grounded polarity): the substrate this machine keeps learning with, and the richer
     grounding its own overlays compile against. **Never shared, never in any repo** — like
     the page cache, it is point-in-time reading, not the module. A read SUCCEEDED when any
     page's prose was read — bindings and reference code are riches, never the bar: a spec
     site with no code blocks at all (json.org presents its grammar as diagrams) still
     trains the reader and mints the module (`Memory.pages_read` is the witness; measured:
     gating on bindings∨reference reported json "docs not learned" off a clean 1-page read).
   Freshness is a probe, not a payload: at setup, a module older than a day checks the live
   sources' `Last-Modified` and re-reads only when the documentation actually moved (no
   header ⇒ conservative re-read). Nothing model-shaped is ever written into a project
   folder or committed to a repo. *The module retrains only when the toolchain version, the
   source set, `TRAIN_VERSION`, or the documentation itself changed; the overlay recompiles
   only when the law, the project, or the module changed.*

   **Every machine-cache artifact is one binary container (`HLM1`), decoded in microseconds.**
   A trained model is mostly hypervectors — already uniform random bits — so the format stores
   them as they are and compresses only what has entropy to give: after a fixed header (magic
   `HLM1`, format version, artifact kind, then the provenance stamp as one length-prefixed
   UTF-8 string — readable by a prefix probe without touching the payload) come two streams.
   The RAW stream holds every hypervector and every integer array (sorted token seeds,
   frequency counts) verbatim, little-endian, in encode order — decoding is a bounds check and
   a bulk copy, and a frozen reader's frequency table stays a sorted array consulted by binary
   search (a write spills it to a map; lint never writes). The DATA stream holds structure and
   text (varint lengths/counts, UTF-8 strings) and is DEFLATE-compressed (`miniz_oxide`, the
   dep the dictionary reader already uses) — rule prose and reference corpora compress several
   ×, hypervectors would not. JSON survives in exactly two places, both deliberate: artifacts
   COMMITTED to the repo (bootstraps — reviewable inputs, diffed in PRs) and the signed
   registry `index.json` (reviewable, signed as text); a legacy `.json` cache artifact is
   migrated on first load (decode JSON → save `.bin` → delete) and the registry loader sniffs
   the magic, so old machines and old registry entries keep working. Because a fresh machine
   never re-reads a stale file, the SETUP verb also sweeps the cache (`sweep_legacy_cache`):
   legacy module/learned JSON is migrated or dropped (the crawl page cache regenerates any
   reading), JSON overlays and dead pre-module families (`*.patterns.json`, an old
   `index.json`) are deleted — exactly one copy of every artifact lives on disk. The same
   keying rules apply unchanged — the container is a wire format, never a cache key.
   Measured (Apple Silicon, 2026-07-05, this machine's full cache of 48 trained languages):
   artifacts shrink 2.0–2.5× (the JSON-era 405 MB cache → 175 MB), a module decodes in
   1–360 µs (cpp 146 µs, typescript 360 µs — vs multi-ms JSON parses), the 11 MB English
   brain loads in ~0.6 ms, and the largest learned catalog (cpp: 63 MB JSON → 25 MB) decodes
   in ~34 ms on the setup path, which is the only path that reads it.

**File types are learned by reading — there is no extension→language table in code.** A
language's own documentation names the files it lives in (`main.rs`, "use the `.py`
extension", `kotlinc hello.kt`), so the association is knowledge, not configuration. Two
typographic readings collect a language's **extension claims** while its docs stream (no
vocabulary, no extension list anywhere):

- **dotted tokens** — `.` + a short alphanumeric run containing a letter, closing its word; a
  run a call opener follows is an invocation, never a filename (`console.log(x)` teaches
  nothing). No commonness filter: a language's docs say their own extension's name in prose
  constantly ("js" all over MDN), so head-wordness is exactly the wrong reason to drop a claim
  (measured: it deleted `.js`/`.php` where they belong and kept `.tostring`).
- **the docs' own name definition** — the parenthetical "JavaScript (JS)" / "TypeScript (TS)"
  is documentation introducing its own short name; the abbreviation (strictly shorter, closing
  the parens itself, and ABBREVIATING — its letters drawn from the name, first-letter-anchored,
  in order; ledger #20) claims maximal strength. This is what keeps `.js` javascript even though
  the TS handbook *mentions* `.js` more than MDN's whole JavaScript tree (103 vs 11 dot-led —
  measured; every pure count hands `.js` to typescript).

Claims live in the module and fold into one machine-global map (`<models>/extensions.json`) at
save. An extension resolves by, in order: the language whose claim is PRIMARY (its own
top-counted claim — `.json` is json.org's primary 8 mentions and beats typescript's 446
tsconfig mentions); then NAME TYPOGRAPHY — the extension begins the language's name ("rs" →
rust), elides it ("yml" → yaml: first-letter-anchored subsequence, the classic vowel-dropping
abbreviation; candidacy even with zero claims), or ends it when backed by a real claim ("sh" →
bash, whose one-page manual names `.sh` once while ruby's docs mention shell scripts
constantly; claim-backed only, or every `.in` file would be kotlin's by its tail); then the
highest count; then lexicographic. A document extension is never claimed by a code language
(`open("file.txt")` examples cannot make `.txt` python — prose stays reading material; `.txt`
does resolve to markdown, whose spec claims plain text as its own). An extension nothing
claims IS the language name (`.go`, `.css` — and any unknown, which the run then asks for).
Law-file stems resolve through the SAME map (ledger #16). Cold machines are wired from
`lint-index/extensions-bootstrap.json` — machine-generated learned data, like the polarity
bootstrap: regenerate with
`cargo test --release --lib generate_extensions_bootstrap -- --ignored`, commit the diff
(`committed_bootstrap_resolves_the_canonical_extensions` pins every canonical wiring), and the
machine map overrides it per language as reading continues. Measured before this held: `.md`
files resolved to a language named "md" while the module trained from CommonMark was named
"markdown" — every machine's markdown module was inert, silently, forever; same for `.yaml`
vs the registry's old "yml" name (the language is registered as "yaml" now — the docs' own
name).

**Lint never touches the network; setup does — no flags, ever.** A lint run is REPLAY-ONLY by
construction: caches, the committed seed, and cached crawl pages (a `TRAIN_VERSION` bump still
re-reads them from disk) — it runs on whatever is set up and ASKS, by name, for what is not.
All acquisition lives in the SETUP verbs, where being online is assumed and a network failure
is reported plainly (it never caches a negative answer and never breaks the run). Setup
acquires per language, in this order: (1) the **GitHub model registry** — published AI
MODULES keyed by `language @ toolchain-version @ sources @ TRAIN_VERSION`: the compiled
runnable artifact plus its `trained_at` timestamp — a couple MB, **never documentation in any
form** (no page snapshots, no example corpus: doc text only ever goes stale, and one cheap
`Last-Modified` probe at setup proves currency better than any stored copy). The reading
memory stays on the machine that read, as the substrate it keeps learning with. A pulled
module is loaded as-is; only the project overlay compiles locally. The registry URL is DATA (the `registry` key of `lint-index/sources.json`);
`lint_submit models=true` distills and publishes this machine's modules; the registry INDEX is
fetched once per run and disk-cached for a day; `HELPERS_LINT_REFRESH` bypasses. (2) the
committed sources snapshot; (3) crawling official docs — registered in `sources.json` or
handed over via `add_source` — into the LOCAL page cache, freshness-checked at setup (below). **There is no web search**: this is a linter,
not a search engine — a language with no known documentation is asked for, and the user (far
more often, the agent acting for them) answers with a URL. In code the mode is one process
latch (`lint_train::allow_network_setup`), set only by the setup verbs; `HELPERS_LINT_OFFLINE`
survives only as the hermetic switch the contract tests use to keep setup off the real
network; no user or agent ever needs to set it.

**The sharing channel assumes every user is the attacker.** A shared module reaches other
machines' running AI, so the threat model is not a man in the middle — it is the submitter:
anyone can run this program, extract their own signing key, patch their own binary, and sign
anything. Every control follows from that:

- **Users can never submit artifacts — only reviewable INPUTS.** A compiled module is
  unreviewable (malice hides in two megabytes of hypervectors), so nothing compiled ever
  crosses the boundary inward. The submission channel (`lint_submit`, opt-in per invocation)
  carries exactly two typed, human-reviewable things: documentation source entries
  (`sources.json` additions — a URL a reviewer can open) and rule-level feedback counts
  (`{rule id → false-positive count, missed count}` — bare numbers, schema-validated,
  size-capped; never paths, never code, never tracking). Submissions arrive as a PR — the
  monitored channel — signed by the submitter's machine key for ATTRIBUTION and revocation,
  never for trust.
- **Modules are built only by trusted infrastructure.** The registry maintainer's machine
  reads the reviewed sources with its own pipeline and publishes what IT trained
  (`lint_submit promote=true`): the consumed `lint-models` branch carries an index SIGNED by
  the registry key with the SHA-256 of every module pinned. No user-built bytes ever reach it
  — a malicious "module" cannot enter the channel because the channel does not accept
  modules, only URLs its owner re-reads independently.
- **Consumers verify or fall through.** `registry_fetch` loads an index only when its
  signature verifies against `lint-index/trusted-keys.json` (data, committed) and a module
  only when its bytes hash to the signed entry; any mismatch and the registry does not exist
  for that run — the machine reads the documentation itself. Unsigned, tampered, or
  self-promoted content is structurally inert: nobody's consumer trusts its key.
- **Defense in depth at load:** even maintainer-signed modules pass size caps and description
  sanitation (advice strings are shown to agents — the prompt-injection surface), plus the
  runtime nets (quarantine, Hv gate, 2-flag feedback) that treat every non-project rule as
  suspect.
- **Honest statement of the guarantee:** the chain ensures nothing reaches a consumer except
  content the maintainer's own pipeline built from reviewed inputs, unmodified since signing.
  The remaining trust decision — "is this URL really the official documentation?" — is
  exactly the human-sized question the PR review exists to answer.

**A site is a source — hand the WEBSITE, get a module per language it teaches.** A source may
be registered for one language (a seed scoped to that language's tree) or as a SITE
(`kind:"site"` in the registry; the `sites` list of the manifest): the whole site is mapped
once into the shared page cache, every PAGE is attributed to the language it documents, and
one module trains per language discovered — nobody feeds per-language subdomain URLs unless
they want to pin one. Attribution is the same learned resolution everything else uses
(ledger #16/#18's resolver — claims + name typography, never a vocabulary), asked at three
levels, first answer wins:

1. **The page's own declarations** — the majority language among its code blocks' hints
   (`brush: js`, `language-css`, fence info): documentation labels its own examples.
2. **URL typography** — a path segment that resolves to a known language (`…/docs/Web/CSS/…`,
   `…/w/cpp/…`): sites file their languages the way projects file their sources.
3. **The host's own name** — a host label resolving through name typography (`docs.python.org`
   ⇒ python, `kotlinlang.org` ⇒ kotlin, `go.dev` ⇒ go): a single-language site says its
   language in its name.

A page attributing to nothing is reading material for every language the site teaches — prose
comprehension, never rules. Within an attributed page the block-hint gate (#18) still routes
each foreign-labeled example out, so an MDN JavaScript page's CSS block stays CSS wherever the
page landed. Unhinted blocks inherit their page's attribution — which is also what keeps a
PER-LANGUAGE source honest: its pages attribute to it or to nothing, and a page that
positively attributes ELSEWHERE is routed out rather than bound. Discovery feeds the same
self-assembly seam: languages a site teaches join the trained set exactly as if each had been
registered by hand, and the manifest backfills to show them.

**The language manifest — one file says where every language's instructions come from.**
`~/.config/helpers/languages.json` maps each language to the documentation URLs it is trained
from: `{ "languages": { "rust": ["https://doc.rust-lang.org/reference/", …], … } }`. It is the
USER'S file: setup backfills it from the committed registry (a language present in
`lint-index/sources.json` but absent from the manifest is copied in, so the file always shows
the full picture), the user edits it to customize — an edited entry OVERRIDES the registry, an
entry emptied to `[]` disables the language's docs (the run then asks, exactly as for an
unknown language), and `add_source` writes into it. Resolution order is manifest → registry →
ask; when a manifest entry matches the registry byte-for-byte the registry's own source
identities are kept so page caches survive. Every stamp that guards freshness (the sources
fingerprint, the module's `Last-Modified` sweep) reads the SAME resolution, so editing the
manifest retrains exactly the languages whose sources actually changed — up-to-dateness is
checked against what the file says, never against a hidden store.

**Online to set up, offline to run — and exactly two setup verbs.** Every report and reply
states the contract in those words. `lint_config action=add_source lang=<x> url=<official
docs>` registers a documentation source — a data write into the machine's added-sources store
(it invalidates the language's model stamp), offline-safe, trains nothing by itself.
`lint_config action=train` is MODULAR: by default it acquires and trains only the CURRENT
PROJECT'S languages (plus an explicit `lang=`), in parallel — a project never pays for a
module it does not use, and a registered language whose site is down costs repos that don't
use it nothing. `all=true` runs the machine-wide batch (every registered language); models
are machine-global either way, and a module retrains only when actually stale (toolchain
version, source set, `TRAIN_VERSION`, or the documentation itself moved — the stamps above).
**A needed language that fails to learn is classified, not shrugged at:** its origins are
probed twice (retry — a one-off handshake hiccup is not a dead site), and the report then
asks for input with the exact command: "docs site not answering (url) — hand me a different
link" when the origin is dead, "docs link answers but nothing readable was learned" when the
site responds but the documentation is unusable. There is no instant hand-teach tool and no
lint-time learning: sources are added, training runs, lint replays — one seam, no shortcuts
to confuse provenance.

**A language it cannot learn is ASKED for at runtime.** The lint report names every language
that is not set up and asks for its documentation link: `add_source` the URL, then `train`.
Every format qualifies — json, svg, a config dialect — because every format has documentation
somewhere, and the asker (usually the agent) knows where. This is the self-assembly seam: the
engine mints one expert module per language, entirely from documentation it is handed or
pulls — at setup time.

**Knowledge survives offline and version bumps.** The crawl page cache (step 0) is the entropy
store and is keyed by TOOLCHAIN version only; `HELPERS_LINT_OFFLINE` means *no network*, never
*no learning* — a `TRAIN_VERSION` bump re-READS the cached pages from disk at memory speed
(setup-mode network is gated by the process latch). **Reading IS the module**: a source that
was read mints the language's module even when ZERO prohibition rules compile out of it — a
descriptive spec (JSON, CommonMark) still yields the reference corpus that grounds law
selection, the reader's comprehension, and a set-up language; "not yet set up" means *could
not read anything*, never *read it and found nothing to ban*. A model whose docs resolved to
NOTHING (unreachable, empty, or no source) is marked so beside its stamp and is retried on
the next setup run instead of masquerading as fresh; the run report names every such language
(prose formats — md/txt, man sections — are not listed: they are reading material with no
doc-learning path) — knowledge, like law, never vanishes silently.
Measured before this held: two offline runs after a version bump silently gutted every model
on the machine to law-only (17 compiled instances where rust alone should carry hundreds).

## Common language first — the LangBrain

**A rule written in natural English can only be understood by something that understands
natural English.** Before any documentation is read, the substrate learns common language from
the machine's own dictionary — on macOS the installed New Oxford American Dictionary
(`Body.data` under the system dictionary assets: 781 zlib chunks, ~225MB of definition XML,
~5.4M prose tokens, ~90k defined words; parsed offline, never network). Two things are learned,
both pure data:

- **The common-language frequency curve** — the reader reads every definition's prose, so its
  frequencies reflect real English ("the" 225k, "use" 3.9k, "never" 740, "import" 31) instead
  of documentation register. This is the curve information weighting and rarity rankings should
  have been standing on all along.
- **The defined-word set (headwords)** — the vocabulary English itself accounts for. This is
  the English-knowledge judgment construct selection uses: a word the dictionary DEFINES is
  common language, whatever its count ("import" at 31 reads is exactly as English as "never" at
  740 — a frequency floor would misjudge both, and `eval` at 4 incidental reads proves any
  floor wrong in the other direction). A word the dictionary does not define (`telnetlib`,
  `xmlhttprequest`, `dbg`, `todo`) is not English — it is the thing the sentence is ABOUT.

The artifact is `english.global.bin` (machine-global, beside the models): the dictionary-fed
reader plus the headword set. It is built once per machine at SETUP time (`action=train`) from
the local dictionary; lint runs only ever load it. Machines without a parseable dictionary load
the committed bootstrap `lint-index/english-bootstrap.json` — machine-generated learned data,
same covenant as the polarity and extensions bootstraps: regenerate with
`cargo test --release --lib generate_english_bootstrap -- --ignored` and commit the diff. The
LangBrain is a substrate, not a rule source: it never fires, never gates a project law's
EXISTENCE, and adding meaning on top of it (definitions as bindings — word ⊗ its definition,
the designed "rules MEAN something" step) extends this section rather than adding a mechanism.

A prose law's detector token is chosen from the sentence's whitespace-delimited words (edge
punctuation trimmed — `console.log`, `8080`, `lock(this)` survive verbatim). Candidates are
ranked, best first, by:

For the **project's law** (ranked, best first):

0. **Not the remedy** — a word in remedy context PAST the first sentence ranks below
   everything, wherever it grounded: "…; use `fetch` with an AbortController" once compiled
   `fetch` because `fetch-depth:` in a workflow file grounded it in the project — existence
   must never promote the endorsed alternative over the named violation. Demotion, never a
   drop: a preventive law still needs *some* watchable word. It applies only past the first
   sentence (the author names the violation before the remedy — the document-order
   convention, #6a), so docs register that paints the construct's own word as endorsement
   ("dbg! is a useful macro…") cannot demote it inside the naming sentence.
1. **The author's marking** — a word the law wrote in backticks IN THE NAMING SENTENCE is the
   named construct (authors backtick their remedies too, so later sentences' marks don't
   count). Optional evidence, never a gate (#2 banned shape *requirements*: an unmarked law
   still compiles through the ranks below) — but when the author did mark, no corpus
   statistics may outvote them ("project", a real identifier in this repo's code, once
   outranked the backticked `XMLHttpRequest` on existence).
2. **Project existence** — occurs in the project's own sources as a WHOLE identifier run
   (sub-word parts are comprehension, not existence — #14): a law names constructs that live
   in the code it governs, and living in the CODE is what proves a word is meant as code even
   when it is also an English word (`panic`, `var` — the common-language judgment below would
   wrongly demote them if it ranked first). Existence is judged in TWO universes: the code
   surface first, and — for a non-English, UNMARKED word only — the project's raw text
   (comment bodies, string interiors). A word that exists only inside comments/strings
   ("TODO", a port `":8080"`) still grounds, and the compiled detector then FIRES in that raw
   universe too: a law fires in the text universe it grounded in (#12, generalized — #12
   stays intact for code-grounded words, whose detectors never enter strings/comments). Two
   exclusions keep the raw universe honest: common-language words never ground through it
   (comments are English; "never" lives in every repo's comments — #14/#15/#17 hold), and a
   BACKTICKED word never does — backticks are inline code markup, the author saying "this is
   a code construct" (`` `todo!` ``), so its law stays preventive on the code surface instead
   of firing on every comment that discusses the construct (measured on this repo: 13
   findings on doc comments the moment marked laws could go raw). A comment-marker law (TODO,
   FIXME) is written unmarked; a code-construct law is backticked — the author's own
   typography is the evidence, never a shape rule about the word itself.
3. **Docs existence** — occurs as a whole identifier run in the language's documented
   (comment-stripped, string-masked) examples: a construct that is also an English word
   (`panic`) may live only in the docs' reference code, and that existence still outranks the
   English demotion below. Project and docs existence OR together here — a project-grounded
   construct never loses this rank.
4. **Not common language** — among words with equal existence, the one English cannot account
   for is the construct: a word the common-language brain knows (a dictionary headword —
   "never", "import", "module", "print") or that sits in the docs corpus head (Zipf top-half
   mass) can never outrank one neither can account for (`telnetlib`, `xmlhttprequest`). This
   is what kills the register hijackers that ride existence — "import" grounds in every
   import line right beside `telnetlib`, and they tie there; English knowledge breaks the tie
   toward the construct. The dictionary judgment is an existence TIE-BREAK, never a veto on
   preventive laws: a construct can itself be an English word and ground nowhere (`panic` in
   a clean repo), so among UNGROUNDED words only the docs-corpus head demotes and the
   sentence's own polarity context (rank 5) stays the deciding evidence — the register
   residual there belongs to the per-token polarity open problem. English is asked about the WHOLE whitespace word: a compound
   identifier (`secret_token`, `document.write`) reads as several English tokens, but the
   compound itself is code typography no dictionary defines — its parts being common never
   demote it (#11's tokenizer split stays a reading aid, not a judgment unit). The judgment
   is learned by reading the dictionary, never enumerated (#17); register words ("Never")
   reading as decisively forbidding (#15) are dictionary words like any other.
5. **Context tier** — forbidding > neutral, by the word's polarity context along the reading
   (nearest decisive lean).
6. **Order/rarity** — document order among grounded content words; rarity (fewest reads) for
   ungrounded words.

For **learned** doc rules: grounding in documented code is an entry *requirement* (not a
rank), remedy-context words are ineligible outright ("…; use the logging module instead" can
never compile `logging`), then forbidding context, not-connective, rarity.

Entry gates: a **learned** rule (example-backed or not) compiles only if some SENTENCE of its
description **classifies as a prohibition** under the information-weighted span classifier —
the sentence is the verdict unit (ledger #6: never the mixed span; ledger #13: never a single
word — one mis-leaning token in a tutorial paragraph must not admit the paragraph). A
prose-derived detector must additionally be grounded in documented code. Every learned detector (AST or text) must also pass the **reference-fire gate**: it is
run against the language's reference corpus (the docs' own *normal* code) at compile time,
and a detector that fires on more than 1% of that normal code's lines is over-general — the
rule's real meaning is semantic (borrow usage, macro context) and tree shape cannot carry
it — so it abstains. The bar is two-tier, by how much the detector's own shape can vouch for
it: a **structured** AST pattern (depth ≥ 2 with at least one exact token kept from the
example) gets the 1% bar; a **degenerate** detector — a single-leaf pattern (a bare `null`
literal), an all-wildcard shape (any method call), or any single-token text detector — carries
no discriminating structure, so the reference corpus is the only witness left and the bar is
0.1%. A construct the docs genuinely ban (`goto`) is near-absent from the docs' own normal
examples and passes; a construct normal code uses constantly (`null`, `trap`) cannot be a
violation marker and dies. The gate is statistical and only activates when the corpus is
large enough to testify (≥500 lines); grounding-scale corpora skip it.
Measured live: without it, semantically-meant rules (clippy's `needless_pass_by_ref_mut` —
its diff reduces to "any `&mut` parameter") and error-page leaf patterns (MDN's "operand
can't be null" — its diff reduces to the `null` literal) produced 1,432 and then 204
findings on this repo; with it the same models keep only real ones. Project law is exempt
(its file is the label) and, when unenforceable, is **reported** ("Project law not yet
enforceable…") — law never vanishes silently.

## The live path (per run, milliseconds cold-warm; microseconds when nothing changed)

fire → guard → gate → quarantine → config/feedback → report.

**An unchanged project replays the whole report (microseconds), warm or cold.** The finished
report body is a pure function of its inputs, so one WITNESS — a fold of every input's
`(mtime, len)` state — decides between "return the stored body" and "run the pipeline". The
witness is verified by STATTING, never by events or daemons: file-system events were tried
and measured unsound (macOS fseventsd ingests kernel events on a ~10ms cadence, and neither
`FSEventStreamFlushSync` nor `FSEventsGetCurrentEventId` can see an edit made microseconds
before the check — an edit-then-lint replayed a stale CLEAN), while mtimes are updated by
the kernel synchronously with the write itself, so a stat sweep can never miss an edit. The
sweep is microseconds because the walk fuses everything into batched syscalls: on macOS one
`getattrlistbulk` call returns name, type, mtime, and size for a whole directory's entries
at once (no per-file `stat`), directories fan out on the shared rayon pool, and ignore
rules match against a per-directory chain of compiled matchers. The witness folds: the walk
(file set + every file's state — deletions change the fold), the law listing and its
documents' states (`.helpers/lint-rules`, root `lintPref` via the walk), `corpus/` and
`lint-index/` (top level), the model dir (modules, overlays, `extensions.json`,
`toolchains.json`), `.helpers/lint.json`, the feedback file, and `TRAIN_VERSION`; the memo
key carries the args (`max`, language filter). Derived caches are EXCLUDED from the fold so
the memo never invalidates itself: `lint-verdicts/` and `lint-replay/` are engine products
of inputs already folded. The body+witness pairs persist in an `HLM1` container
(`lint-replay/<project>.bin` beside the verdicts), so a fresh process replays as fast as
the daemon — warm and cold differ only by `exec`. A witness mismatch falls through to the
per-file verdict replay below and the fresh body is stored with a witness recomputed after
the run's own model writes. `(mtime, len)` alone cannot see a same-length edit landing in
the same mtime tick as the stored state (git's "racy index" problem), so every store also
records whether its newest input mtime cleared a 2s racy window of the store moment: a racy
store is kept but refuses to replay, and the next full run — same inputs, later moment —
re-stores it replayable. The replay changes no verdict — same engine, same output — so it
does not bump `TRAIN_VERSION`; the one residual (documented, self-healing): a module
retrained by ANOTHER process in the milliseconds a run is in flight is masked until any
input changes.

**Warm runs replay per-file verdicts.** Firing, the restatement guard, and the Hv gate are all
deterministic per FILE given the merged model, so their product is cached per file and a warm
run never re-derives it: the verdict cache (`lint-verdicts/<project>.bin` beside the models —
an `HLM1` container, machine-global, never in the repo, always safe to delete) maps each file
to its `(mtime, len)` state, its content seed, its line count, and the post-gate findings
`(rule, line, doc_rule)`, keyed by the model identity (module provenance ⊕ overlay stamp ⊕
`TRAIN_VERSION`). A warm run STATS every file, replays verdicts for unchanged ones — no read,
no parse, no gate — reads and lints only what actually changed, and then applies the
RUN-LEVEL shaping to the union exactly as before: quarantine rates over per-language line
totals, feedback suppressions, severity overrides, and rendering stay whole-run, because they
are functions of the run, not of a file. Grounding fingerprints reuse the cached content
seeds, so the project fingerprint no longer needs every file read either; the full contents
are read lazily — only when an overlay actually recompiles (its grounding universe) or a file
is stale. Everything cheap-but-repeated is memoized per process or per machine by the same
discipline that killed ledger #8: toolchain versions live in `toolchains.json` beside the
models, keyed by the resolved binary's `(path, mtime, len)` (absence keyed by a PATH
fingerprint), so a warm run spawns no processes; extension resolution memoizes per universe
generation; the lint-index directory states behind the overlay stamp are computed once per
run, not once per language.

- **Fire**: each file is parsed once and its tree walked ONCE, and that single walk yields
  everything the run needs from the tree: AST patterns are indexed by the one node kind their
  root can match, so each node tries only its own candidates (never one full-tree walk per
  rule), and the same walk collects the English-bearing spans (string/comment/heredoc/char
  nodes) that blank into the code surface for token detectors — the mask is a byproduct of
  the walk, never a second parse. Token detectors then fire by whole-token containment per
  line in their grounded universe (code surface / raw); no regex engine runs anywhere.
  Languages fire in parallel (the stage costs the slowest language, not the sum), files
  within a language in parallel, and results fold back in language-then-file order so the
  report stays deterministic. Whole-repo (~1,475 files), measured 2026-07-06 in the daemon:
  an UNCHANGED project answers in ≈ 1.8–2.8ms end to end — the batched-syscall walk verifies
  every file's state (~2ms; the sweep IS the correctness proof) and the whole-report replay
  returns the stored body; a changed project pays the per-file replay pipeline, ≈ 7ms warm
  (was ≈ 212ms before verdict replay, ≈ 20ms before the fused walk / decision cache /
  per-extension resolution): train/load ≈ 1–4ms (toolchain-version machine cache — no
  spawns; one shared document read, prewarmed before the language fan-out; dir-state
  memoized listings), match+gate ≈ 1ms (replay; fresh files pay the real parse)
  (`HELPERS_LINT_TRACE=1` prints the stage split, per-language `[lint-train]`/`[lint-match]`
  lines, and the walk sub-stages; `[lint-replay]` marks a whole-report replay). Text detectors match each line's **code surface** (comments dropped, string
  interiors blanked — the same function grounding reads through; ledger #12); prose files
  (md/txt) match raw lines, and a project-law detector whose construct grounded ONLY in the
  raw universe (see the evidence hierarchy) matches raw lines as well — the report says so
  beside the pattern.
- **Restatement guard**: a line sharing ≥3 and ≥half of the rule's own words is quoting the
  law, not breaking it.
- **Hv concept gate**: imprecise (token / container-only) findings are kept only if the fired
  rule's fingerprint is the nearest concept to the matched construct — one batched
  popcount-grid dispatch per language. **Concepts exist only for rules that can FIRE**: a
  rule that compiled no detector can never be confirmed, so its fingerprint could only
  serve to veto other rules' true findings — measured (`var leaky = 2;` in a two-line
  project): a detector-less concept sat at distance 3468 from the construct while the fired
  `no_var_declaration` sat at 3607, and the gate rejected its own rule's true hit, which
  the identical construct with a docs-known identifier (`var count = 2;`) survived. Both
  the module's and the overlay's `ConceptModel` compile from the rules the `RuleSet`
  actually kept; among firing concepts the fired rule's own token mass keeps true hits
  nearest, and no blanket noise-band abstention is needed (tried and reverted: abstaining
  whenever the nearest concept sat in the random-distance band let every low-count scrape
  rule through the gate). **A tie is statistical, not exact**: one line can violate one
  rule while legitimately containing another rule's territory — `var doubled =
  eval("total * 2")` put the eval concepts 85 bits nearer than the fired
  `no_var_declaration` and winner-take-all killed a true finding — so the fired rule loses
  only when the nearest concept beats it by more than 3σ of the code geometry's distance
  noise (σ = `√DIM/2`; within that band the two distances are indistinguishable and the
  finding is kept, the same "ties keep it" the gate always had).
- **Quarantine**: a doc rule firing like scrape noise (≥20 hits and >1% of the lines of the
  RULE'S OWN LANGUAGE — for a *degenerate* detector ≥50 hits and >0.1%, the same two-tier the
  reference-fire gate uses, because a small reference corpus cannot witness a token that is
  rare in doc examples but pervasive in real projects — or ≥20 hits covering >10% of one file)
  is quarantined and reported. The degenerate tier's floor is 50, not 20: its incident class
  fires in the hundreds (`path`, 305×) while a legitimately much-violated single-token
  convention fires in the twenties (`no_var_declaration`, 20+ real `var` declarations on one
  repo), and a floor of 20 quarantined the true rule exactly when it was most violated. The
  denominator is per-language: a rust rule's noise must not be diluted to invisibility by a
  thousand markdown files it never ran against.
- **Docs are reading material**: md/txt files are linted only by rules written *for* them;
  `any`-language law governs code languages. LEARNED rules never fire on a prose language at
  all — a prose file is 100% the English universe ledger #12 excludes (a CommonMark-learned
  `div`+`id` detector fired 200+ times on this repo's own docs the moment the markdown module
  first actually loaded: raw text has no code surface to discipline a token detector), so a
  prose language's module contributes its reading (reference corpus, comprehension) and only
  the project's own law (or corpus law written for prose) may flag its files.
- **Feedback** (runtime shaping): `lint_flag` false-positives auto-suppress a rule per project
  after 2 distinct sites (reversible); missed-findings surface as pending rules. This is the
  designed interaction loop — the linter is shaped to the project while it runs.
  Suppressions are **version-scoped**: each flag records the `TRAIN_VERSION` it was filed
  under, and only same-version flags suppress — when a new training version lands, old
  suppressions clear (the log keeps them as history), because the junk they papered over
  should no longer exist; if it does, two fresh flags re-suppress it and the recurrence is
  visible instead of silently masked.
- **Report**: findings carry the rule's own English and a `⟨source⟩` citation (doc URL or
  rule-file path); "Your law, as understood: id → watching for `token`" shows the compiled
  comprehension of every project law so a misreading is corrected by rephrasing, not by
  debugging silence.

## Failure ledger (what broke, why, and the invariant it left)

1. **Concept prose fired on documentation** (2,358 findings, ~99% noise) → *only a statement of
   a violation may become a detector; teaching feeds understanding.*
2. **Shape-expectant extraction** (backtick/dotted/camel/call-syntax gates; bare `8080` could
   never be a construct) → *one tokenizer, whitespace words, learned-evidence ranking; shapes
   deleted.*
3. **Rarity picked the remedy's word** (`logging` over `print`) → *remedy-context ineligibility
   + document-order fallback for law.*
4. **Sentence splitter cut `string.format` in half** → *a `.` between letters is part of a
   word; boundaries need trailing whitespace.*
5. **Byte-offset context windows opened mid-HTML-tag** (`class="boring"` polluted 40 bindings
   and rule ids) → *prose is sliced at tag boundaries by construction.*
6. **Whole-description polarity flips** ("Do not use X. Use Y instead." reads as endorsement
   when the remedy register dominates) → *labels and verdicts live at the sentence/word level,
   never the mixed span.*
6a. **One-word fence-label calls flip under prototype drift** (a regenerated classifier read
   the fence word `bad` as endorsement — docs prose "that was bad, instead:" governs *good*
   blocks, so label words drift — and a rule's examples swapped; a relative bad-vs-good lean
   comparison inherited the same inverted geometry) → *document order (violation first, fix
   after) is the convention; only POSITIVE evidence overrides it — the trailing block's label
   decisively classifies prohibition while the leading block's does not. Drift then degrades
   to the convention, never to swapped examples.*
7. **Ubiquitous-verb register leans** ("use" decisively bad ⇒ a bash-manual guidance sentence
   minted a firing rule) → caught in production by the feedback loop (2 flags → suppressed);
   root fix is per-token **side counts** from grounded labels — designed, prototyped, and
   **reverted** (see open problems) after it destabilized fence orientation; do not re-land it
   without updating this file with the full design first.
8. **O(n²) shape hashing + per-rule reparse + per-file regex compile + per-language 0.5MB JSON
   parse + npm probe per unknown extension** (hours of CPU; two processes at 100% for
   135 CPU-minutes) → *memoize by construction: one shape pass, one parse per file, one regex
   compile per rule, one classifier load per state, failures cached on disk.*
9. **Stale caches masquerading as current** → *every learned artifact is keyed by
   `TRAIN_VERSION` + toolchain version; bump on any reading/compile logic change* (bitten
   again live: a compile-logic edit without a bump silently reran old models).
10. **The curated catalog as law** (2,221 hand-maintained linter rules briefly minted rules:
   semantically-meant entries tree-diffed to ubiquitous shapes — 1,432 findings) → *the
   catalog is deleted entirely; enforcement grows purely from reading, labels come only from
   toolchain grounding, and the reference-fire gate kills shape-degenerate detectors at
   compile time.*
11. **Two tokenizers disagreed on snake_case** (grounding corpora were tokenized by an ad-hoc
   splitter that kept `secret_token` whole, while the description's candidate words tokenize
   through the reader — which splits at `_` into `secret`/`token`; the set intersection was
   empty, so **no snake_case identifier could ever ground a law**, and "Never hardcode
   `secret_token`…" silently compiled `hardcode` — a word in no one's code — instead of the
   named construct: a silent false negative in the dominant naming style of Python/Rust/C) →
   *grounding corpora tokenize through `lint_read::tokens` — the reader's one tokenizer —
   by construction; ledger #2's "one tokenizer" invariant applies to every token set the
   selector compares, not just the description side.*
12. **Text detectors fired inside strings and comments** (a law watching `secret_token` flagged
   the remedy line `os.environ["SECRET_TOKEN"]`; grounding already treated string/comment
   interiors as English-not-code, so the detector fired in a universe the law never grounded
   against) → *one `code_surface` function — whole-line comments dropped, string interiors
   blanked, trailing `//`/`#` comments cut, a quote with no same-line mate is typography (Rust
   `'a`) not a string — is shared by grounding AND text-rule firing: a law fires only in the
   text universe it grounded in. Prose files (md/txt) are exempt — their text IS the governed
   material. AST rules hold this by construction (a string node is never an identifier).*
13. **One mis-leaning word admitted a whole tutorial paragraph as law** (the entry gate asked
   only "does ANY word sit in forbidding context?", so go.dev tutorial narration and MDN
   error-page remedy prose — "can be fixed via js" — minted rules whose detectors fired on
   hello-world code: `first_statement_in_a_go` watched the very construct the sentence
   prescribes) → *the entry verdict is rendered per SENTENCE by the information-weighted span
   classifier — a learned rule compiles only when some sentence of its description classifies
   as a prohibition; the sentence is the verdict unit, never the mixed span (#6), never a
   single word.*
14. **Multi-line strings and prose "languages" grounded register words as code** (line-based
   masking cannot see that the middle line of a Rust multi-line help string, a bash heredoc
   body, or a man page's entire text is English — so "never" grounded as project code in
   every language and, with existence leading the law hierarchy, EVERY instance of
   `no_xmlhttprequest` compiled `(?i)\bnever\b`; a repo doc's `# Title` heading also minted a
   trusted law watching `rust`) → *masking is AST-exact for grammared languages (string /
   comment / heredoc / char node spans blanked from the parse tree — the same tree firing
   uses), the line masker stays only as the grammarless fallback; man sections (numeric
   extensions) are prose, never code-law targets; a heading with no body prose and no code
   blocks is a title, not a law; and the law hierarchy leads with not-connective (see the
   evidence hierarchy) so no corpus can promote a common word over the named construct.*
15. **Register vocabulary hijacking construct selection** (weighted contexts made "Never"
   read decisively forbidding → laws watched `Never`; "use"'s endorsement lean poisoned
   neighboring `unsafe`/`any`; docs-grounded "never" beat project-grounded "unsafe" on
   document order; prose capitalization compiled a case-sensitive `\bUnsafe\b`) → *the law
   hierarchy now leads with existence — project-code grounding, then docs grounding, then
   forbidding context; only informative (non-common) words project context onto neighbors; a
   grounded law word survives remedy-context ineligibility; detectors are case-insensitive on
   the lowercased surface.*

16. **Rule-file stems taken literally** (`js.md` compiled its rules for a language named "js";
   files detect as "javascript", so the law governed NOTHING — CLEAN verdicts over planted
   violations, no law block, no report: law vanished silently) → *rule-file stems and fence
   language hints resolve through the same extension→language map the file walker uses; a law
   file whose language matches no project file is reported as inert.*

17. **The substrate never learned common English, so commonness judgments answered from
   ignorance** (validated live on never-seen law phrasings: the docs-fed reader's Zipf head
   cutoff sat at 691 reads while "import" had 402 and "never" 165, so neither was ever demoted
   as connective — "Do not import the telnetlib module…" compiled `import`, and with "never"
   in any string of the project, `never`: a false positive on an innocent string plus a false
   negative on the actual `import telnetlib` line. Every extractor patch inherits the same
   lie — the defect was the missing baseline, not the ranking) → *common language first: the
   machine's own dictionary is read into the LangBrain before any docs, and the selection
   judgment is "does English account for this word" (dictionary headword or docs-corpus head),
   learned by reading, never a hand-tuned threshold — a binary frequency cutoff at any scale
   was the exact failure `select.rs` warned itself about.*

18. **Polyglot documentation bound foreign examples into a language's model** (MDN's JavaScript
   tree embeds HTML/CSS example blocks; extraction dropped every block's own language label
   ("the optional info string after the fence is dropped"), so HTML examples became javascript
   bindings — 11 HTML-shaped bindings measured in the js reading memory — and, worse, an HTML
   example on a JS page FAILS `node --check` grounding, feeding its neutral prose to the BAD
   prototype: mislabeled polarity from mere language mixing. An MDN HTML-page rule fired on
   `.mjs` files) → *the docs' own block hint (`brush:`/`language-*` classes, fence info) is
   extracted and resolved through the extension→language map (#16); a foreign-hinted block is
   prose-only for the training language. The hint gate trusts only hints resolving to a KNOWN
   language — junk labels are not hints. Sites are polyglot by default; only the seed TREE is
   per-language.*

19. **Untraceable example tokens fired on innocent code that resembled the docs' examples** (nim,
   trained ungrounded from one link: descriptive tutorial prose slipped the sentence gate under
   the transferred classifier, the example diff fell back to a literal token pair, and the pair
   was `proc greet` — the tutorial's OWN function name. Every user who writes a greeter gets
   flagged by a "rule" whose watched words appear nowhere in its own description and were never
   reality-tested) → *provenance: an example-diff TEXT detector compiles only when the
   toolchain actually FLAGGED its example (the reality label travels with the memory) or the
   law's own words name a kept token — whole-token, through the one matcher; the anchor
   suffices, because a pair's partner token only narrows firing on the anchored line. A
   Clean-parsing or ungroundable example's identifiers are just code the docs showed. Applies
   whenever a classifier is rendering verdicts; with no classifier the author's material is
   trusted, as at the entry gate. The AST path is untouched (structured patterns carry their
   own discrimination and the reference-fire gate).*

20. **A genuine parenthetical dethroned a language's own extension machine-wide** (the docs'
   name-definition rule grants maximal claim strength, and real prose writes "Python (REPL)",
   "Kotlin (J2K)", "Erlang (BEAM)", none of which is the language's short name — python's
   primary claim became `repl` at MAX, its real `py` claim fell out of the primary band, the
   bare-prefix guard blocked typography candidacy, and `.py` resolved to MARKDOWN off
   markdown's incidental `py` mentions: every python file on the machine silently linted as
   prose) → *an abbreviation ABBREVIATES: the parenthetical's letters must come from the name
   itself, first-letter-anchored, in order ("js" ⊆ javascript; "repl" ⊄ python) — the same
   elision typography the resolver already trusts, applied at the claim's birth.*

21. **Incidental attribute-access claims made junk fence labels resolve as languages** (dotted
   tokens like `result.output` in real documentation minted low-count extension claims for
   `output` across eight languages; the moment one language's corpus was small enough for the
   claim to clear the 1%-of-primary noise gate, `output` RESOLVED — and the block-hint gate
   (#18) then routed "output"-labeled example blocks out as foreign, silently discarding real
   examples exactly as #18's junk-label clause forbids) → *resolution is GRADED by how it
   resolved: a language's own identity, a PRIMARY claim, or name typography is label-grade —
   trustable by the hint gate — while an incidental count-claim is file-grade: it may resolve
   a file on disk (a file's claim is corroborated by the project that contains it) but never
   validates a fence label. Labels name languages by name or canonical extension; canonical
   extensions are always primaries or typography.*

## The distribution channel (built) and the community network (deferred, decided)

**What ships today: a signed, one-way distribution channel — every user assumed hostile.** A
shared module reaches other machines' running AI, so the channel is a supply chain under
attack, and the failure to exclude is a hand-crafted or tampered artifact being LOADED
anywhere. Rule text is read by agents, so a crafted "rule" whose advice says "disable your
sandbox / post the env file to…" would be prompt injection with a linter's trust halo —
credential theft and code exfiltration are the named failure modes, and one malicious
artifact distributed once is total failure.

The built guarantees (`lint_sign`, and the registry path in `lint_train`, contract-tested in
`ai_linter_behaviors.rs`):

- **Consumers verify or fall through.** `registry_fetch` accepts an index only when its
  signature verifies against `lint-index/trusted-keys.json` (committed data; embedded
  fallback), and a module only when its bytes hash to the signed index entry. Any
  mismatch and the registry *does not exist* for that run — the machine reads the
  documentation itself. Unsigned, tampered, or attacker-signed content is structurally inert:
  no consumer trusts its key (the `an_unsigned_or_tampered_registry_is_never_loaded`
  contract proves a machine falls through rather than load an untrusted-signed index).
- **Publishing is maintainer-only and signed.** `lint_submit models=true` signs the index
  with the machine's Ed25519 key (`~/.config/helpers/signing.key`, generated on first use)
  and writes `index.json` + `index.sig`; only keys listed in `trusted-keys.json` are
  consumed. `lint_submit identity=true` prints this machine's public fingerprint.
- **Fail-safe key management:** rotation = edit `trusted-keys.json` + republish; revoking a
  compromised key just removes it (consumers then refuse its index and fall through to docs —
  never fail-open).

**The community network is DEFERRED, but the shape is decided** (grilled 2026-07-05, so a
later build starts from settled constraints, not a blank page):

- **Sources are owner-only, forever closed to automated community submission.** No code can
  distinguish "official documentation" from a convincing forgery, and there is no human
  reviewer in the automated path — so the pipeline must never crawl a community-submitted URL
  and sign what it builds (that would let an attacker author "correctness" the registry key
  faithfully signs). New sources enter ONLY by the owner editing `sources.json` in the public
  repo; a community PR there goes through the owner's own merge judgment, which is the only
  gate that exists.
- **If a feedback channel is ever built, it is signal to the OWNER only — never a direct
  community→consumer edge.** The single trust path stays community → aggregated signal →
  owner judgment → new signed module → consumers. Network feedback never auto-changes any
  consumer's enforcement (a consumer's suppressions stay local, from its own runs), because
  free self-minted identities make sybil-suppression of a real rule otherwise trivial. The
  payload, when built, is bare bounded integers (`{rule id → fp, missed}`) — no text, no
  paths, no code: nothing for injection to ride on.
- **The residual, stated plainly:** even a clean owner-built module quotes documentation prose
  to agents; the system bounds it (load-time length caps + control-character stripping on
  advice, `⟨source⟩` citation on every finding) but cannot prove prose harmless — an agent
  that obeys imperative text inside lint advice is a failure of the agent's own hygiene.

The seam is clean: `lint_sign` (crypto), the signed registry (distribution), and the existing
local `lint_feedback` log are the whole foundation a network needs. Adding it later is a new
submission tool + an owner aggregation step — no rearchitecting. Each control lands TDD-first
with an attacker contract, exactly as the two registry contracts already do.

## Open problems (honest)

- **Per-token polarity evidence.** Span prototypes cannot say what one word means; the
  side-count design (tally which label each token appeared under — labels now come only from
  toolchain grounding verdicts, the curated seed is gone; lean = 2:1 majority) fixed
  "use"-class noise in probes but regressed fence orientation and contract tests when rushed.
  Land it with: grounded-only tallies, orientation reading words→sentences→order, and a
  regenerated bootstrap — and update this file first.
- **Error-page remedy prose trains the bad prototype** (measured live, the driver of the
  residual junk class): an MDN error page's example FAILS the toolchain, so the prose around
  it feeds the *bad* prototype — but that prose is remedy language ("can be fixed by
  wrapping…"), so fix-register vocabulary ("fixed", "wrap", "avoid the error") learns a
  prohibition lean, and remedy fragments then pass the sentence gate as law
  (`can_be_fixed_via_js`, `avoid_the_error_wrap_eac`). The defense-in-depth (sentence gate,
  sample-program abstention, reference-fire, quarantine, 2-flag feedback) reduced this from
  storms to single rules that the loop suppresses (demonstrated on this repo: 762 → 343 rules,
  then convergence to CLEAN with every suppression a verified FP) — the root fix is the
  side-count design above, whose grounded-only tallies must separate "prose beside failing
  code" from "prose stating the failure".
- **A law whose violation is an ABSENCE cannot compile.** "Never use a bare `except:`", "no
  empty catch blocks": containment matching cannot assert emptiness (an empty-block pattern
  matches every block — over-fire kills it, correctly), and `bad ∧ ¬good` cannot express a
  missing token. Keep such rules out of `corpus/` until the engine learns absence shapes;
  `HELPERS_LINT_TRACE=1` names the gate that dropped them (`[lint-build]` lines).
- **A prose law about pure punctuation can compile a junk word, not a report.** "Do not
  compare types with `==`": the linter watches *words*, and `==` has no letters or digits, so
  there is nothing it can watch for — the description path can fall back to some other word of
  the sentence ("compare"), which typically never occurs in code: a silent false negative
  dressed as a detector. "Your law, as understood" makes the misread visible, and a bad/good
  example fence fixes it fully when a grammar exists (the AST diff carries operators fine —
  `type(a) == type(b)` compiles losslessly). The EXAMPLE-DIFF path already behaves honestly:
  a `bad ∧ ¬good` difference that is pure punctuation or a bare numeric value yields no
  watchable word, the compile abstains, and project law is reported "not yet enforceable"
  (contract-tested on a grammarless language). The remaining step is the same honesty for the
  description path: detect that the naming sentence's only candidate constructs are non-word
  symbols and report instead of compiling the junk word.
  (String/comment-only constructs — "never hardcode port 8080" with `Listen(":8080")` — were
  this list's second entry; solved by raw-universe grounding, see the evidence hierarchy.)
- **Per-language law instances can diverge.** An `any`-language law compiles once per
  language against that language's corpus, so picks differ (ruby's instance of a "port 8080"
  law once compiled `from`); "Your law, as understood" currently shows one instance — show
  the divergent ones so the author sees which language misread.
- **Tutorial prose can still mint a junk rule when a sentence genuinely reads as law**
  ("Executable commands must always use package main" IS prescriptive English); the
  sentence-level prohibition gate (ledger #13), reference-fire, quarantine, and the 2-flag
  feedback loop are the defense-in-depth — the residue is single rules caught by the loop,
  not storms.
- **Doc-rule recall.** Reference manuals are descriptive; normative style guides (PEP8,
  api-guidelines, effective_go) and the built linters' own rule docs yield the real rules.
  More registered sources per language is a data edit in `sources.json`.
- **Parse-grounding mislabels LINTER documentation** (measured; the blocker for full ESLint
  parity): the polarity classifier's labels come from toolchain verdicts, and a lint rule's
  "incorrect" example is usually VALID SYNTAX — `var x = 1` parses clean — so the prose above
  it ("Examples of incorrect code for this rule") feeds the ENDORSEMENT prototype: the
  grounding actively teaches the opposite of what the page says. Compile-error docs (MDN error
  pages, reference manuals) ground correctly; lint-rule docs need labels the parse check
  cannot give. Only 8 of ESLint's ~290 rules survived to detectors for exactly this reason
  (clippy fares far better: its docs' bad examples often genuinely fail `rustc`). The fix
  belongs to the side-count/asymmetric-grounding design (a Flagged verdict is strong evidence;
  a Clean verdict says "parses", not "endorsed") — design it in this file first, per ledger #7
  discipline; do not special-case linter-doc vocabulary.
- **Governance is association, not adjacency (learned page understanding).** The extraction
  windows have accumulated three heuristics answering one question — which prose governs
  which code (`GOVERNING_CTX` tail, the 40-word lead-in, the heading cut) — and stacked
  hand rules are the tell that the mechanism is wrong. The author's own typography stays
  DATA (`<pre>`/fences mark code, anchors name sections — grounding needs exact code bytes,
  and a learned segmenter that is 95% right poisons 5% of the toolchain labels the whole
  classifier rests on), but OUR windows should dissolve into comprehension: the Reader
  already forms `prose_hv ⊗ code_hv` bindings, so a block's governing prose is the page's
  SENTENCES whose prose-Hv binds strongest to the block's code-Hv — association decides,
  byte distance only breaks ties ("Binding.bind stored but not similarity-queried" is this
  gap). One step further, the reader's own prediction-error spikes are a LEARNED section
  boundary (topic shift = surprise) — no heading tag consulted at all, which folds this
  into the latent-sequence design below. Until then the heading cut above is explicitly
  interim mechanism: typography-only, no vocabulary, and replaced by association when this
  lands (spec-first, with the diversity contract as its acceptance gate).
- **Latent-sequence reasoning ("brain waves").** Inference is already Hv-native end to end;
  a rolling-context classifier (prototypes over context space, not bag space) is the designed
  next step for clause understanding without any typography.

## Operational notes

- Stage timing: `HELPERS_LINT_TRACE=1`. Force re-learn: `HELPERS_LINT_REFRESH=1`. Model cache
  override: `HELPERS_LINT_MODELS`. `HELPERS_LINT_OFFLINE=1` simulates a dead network (hermetic
  contract tests only — no user or agent ever needs it; see "No connectivity flags").
- Setup verbs: `lint_config action=add_source lang=<x> url=<docs>` (register, offline-safe),
  `lint_config action=train` (train everything, needs internet for anything missing). Publish
  this machine's catalogs to the registry: `lint_submit models=true`.
- Caches live in `~/.cache/helpers/`; deleting them is always safe (cold reacquire is a
  registry download or seconds of crawling per language, online). Model-cache artifacts are
  `HLM1` binary containers (see "Save"); `helpers-native` decodes them — they are not for eyes.
- The polarity bootstrap (`lint-index/polarity-bootstrap.json`) is machine-generated:
  `cargo test --release --lib generate_polarity_bootstrap -- --ignored` — regenerate whenever
  the tokenizer, salience, or seed labeling changes (train/inference consistency).
- The English bootstrap (`lint-index/english-bootstrap.json`) is machine-generated from the
  local dictionary: `cargo test --release --lib generate_english_bootstrap -- --ignored` —
  regenerate whenever the tokenizer or the dictionary parser changes. Machines with a local
  dictionary rebuild `english.global.bin` themselves at setup; the bootstrap only covers
  machines without one.
