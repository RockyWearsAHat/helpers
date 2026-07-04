# The AI Linter — how it works

> The single authoritative description of the lint system. Module headers in `native/src/`
> summarize their own file and point here; this document owns the cross-module theory. **No
> semantic change lands without updating this file first** — every regression this system has
> had came from editing behavior without a written model of it.

## Thesis

**English is read; code is linted.** The system builds a baseline understanding of English by
reading (that understanding is the substrate), then learns each code language *through* that
understanding by reading its documentation, and finally enforces only things that state a
violation. Comprehension and enforcement are different acts: teaching material, concept prose,
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

Measured (Apple Silicon, `cargo run --release --example reader_bench`): reading ≈ 2.5M
tokens/s; training 2,221 labeled sentences ≈ 0.07s; classification ≈ 220k sentences/s. The AI
is never the slow part.

## What is learned vs what is programmed

This boundary is the answer to "is it an AI or a program?" — kept honest by inspection.

**Learned (data; changes by reading, never by code edit):**
vocabulary and its frequencies (self-defining: any string hashes to a code; nothing is
enumerated); the polarity prototypes; every rule (read from docs/law files); every compiled
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

## Sources of law (exactly two) and sources of reading

**Enforcement grows purely from reading.** There is no curated rule catalog anywhere — the
former `extraDocs/lint-corpus.jsonl` (hand-maintained linter rules) is deleted; correctness is
learned by reading official documentation and grounded against the installed toolchain, never
authored. The polarity classifier's only labels are toolchain verdicts on real code.

| Law | Trust |
|---|---|
| Project rule files: `.helpers/lint-rules/*.{md,txt}`, root `lintPref.{md,txt}` | Absolute ("law by location"): never polarity-gated, never Hv-gated, never quarantined |
| Official language documentation (crawled; registered in `lint-index/sources.json` or discovered on the fly; normative style guides — PEP8, api-guidelines, effective_go — are where the practice rules live) | Gated: prohibition reading + grounding + self-fire + reference-fire + Hv gate + quarantine |

**Reading only (never rules):** `extraDocs/*.md` teaching prose, `lint-index/reading-sources.json`
corpora (Stack Overflow, Urban Dictionary — coder register), and all doc prose around examples.
Enforcing teaching material was a repeated noise source (see ledger) and is structurally off.

## The per-language training pipeline

`sitemap → level-parallel map → read → ground → bind → compile → cache/save`

0. **Languages train in parallel** (one thread per language) and every crawled source is
   cached **once per machine** in `~/.cache/helpers/lint-index/crawls/<tool>.json` (the
   extracted per-page prose + example pairs, keyed by toolchain version). A source shared by
   two languages (TypeScript ⊇ JavaScript both read MDN) hits the network once — the second
   reader replays the cached pages at memory speed; a process-wide once-map prevents two
   parallel languages from double-crawling the same source. `HELPERS_LINT_REFRESH` recrawls.
1. **Map**: try `<origin>/sitemap.xml` (one request can enumerate the site), then balloon
   outward by levels — every link of a level fetched concurrently (64-wide waves), visited-set
   dedup, until the in-scope site is mapped. No pacing. Measured: the 123-page Rust reference +
   api-guidelines → **2.7s for the entire pipeline**.
2. **Read**: the Reader learns the prose; `(governing prose, code example)` pairs are sliced at
   *tag boundaries* (between `</pre>` and the next `<pre`) — never at byte offsets.
3. **Ground**: a bounded sample of examples is checked against the installed toolchain
   (parse/compile check only, never executed, parallel): flagged → prose feeds the bad
   prototype, clean → good. Docs' claims tested against reality.
4. **Bind**: prose⊗code hypervector bindings + the reference corpus ("what is normal here").
5. **Compile** (`RuleSet::build`): examples → lossless generalized AST patterns via
   `bad ∧ ¬good` tree-diff (operations exact, operands bound wildcards, literals typed
   wildcards) or discriminating token regexes; prose-only rules → a detector derived by the
   evidence hierarchy below.
6. **Save**: version-keyed user cache (`~/.cache/helpers/lint-models/`) — **machine-global,
   shared by every project on the system**; a language learned once is never relearned for
   another project. Nothing model-shaped is ever written into a project folder or committed
   to the repo (the former `lint-models/` committed-module path is deleted). *Always checked,
   retrained only when the toolchain version or `TRAIN_VERSION` changed.*

## The evidence hierarchy (construct selection for prose rules)

A prose law's detector token is chosen from the sentence's whitespace-delimited words (edge
punctuation trimmed — `console.log`, `8080`, `lock(this)` survive verbatim). Candidates are
ranked, best first, by:

1. **Grounding** — occurs in real code: the language's documented (comment-stripped) examples
   ground anyone; the *project's own sources* additionally ground the project's law only
   (comments/data strings must not launder teaching vocabulary). Existence leads: a word that
   never occurs in code can never fire, and prohibition register words ("Never") read as
   decisively forbidding without being anyone's construct — grounding is what keeps them from
   hijacking selection.
2. **Forbidding context** — the word's polarity context along the reading (nearest decisive
   lean); remedy-context words are *ineligible* ("…; use the logging module instead" can never
   compile `logging`).
3. **Not connective** — corpus-head words (Zipf top-half mass; scale-free) rank last.
4. **Order/rarity** — project law: document order among grounded content words (the author
   names the violation before the remedy); learned rules: rarity (fewest reads).

Entry gates: a **learned** rule (example-backed or not) compiles only if its description
carries a forbidding sentence, and a prose-derived detector must be grounded in documented
code. Every learned detector (AST or text) must also pass the **reference-fire gate**: it is
run against the language's reference corpus (the docs' own *normal* code) at compile time,
and a detector that fires on more than 1% of that normal code's lines is over-general — the
rule's real meaning is semantic (borrow usage, macro context) and tree shape cannot carry
it — so it abstains. The bar is two-tier, by how much the detector's own shape can vouch for
it: a **structured** AST pattern (depth ≥ 2 with at least one exact token kept from the
example) gets the 1% bar; a **degenerate** detector — a single-leaf pattern (a bare `null`
literal), an all-wildcard shape (any method call), or any single-token text regex — carries
no discriminating structure, so the reference corpus is the only witness left and the bar is
0.1%. A construct the docs genuinely ban (`goto`) is near-absent from the docs' own normal
examples and passes; a construct normal code uses constantly (`null`, `trap`) cannot be a
violation marker and dies. The gate is statistical and only activates when the corpus is
large enough to testify (≥500 lines); grounding-scale corpora and discovery probes skip it.
Measured live: without it, semantically-meant rules (clippy's `needless_pass_by_ref_mut` —
its diff reduces to "any `&mut` parameter") and error-page leaf patterns (MDN's "operand
can't be null" — its diff reduces to the `null` literal) produced 1,432 and then 204
findings on this repo; with it the same models keep only real ones. Project law is exempt
(its file is the label) and, when unenforceable, is **reported** ("Project law not yet
enforceable…") — law never vanishes silently.

## The live path (per run, milliseconds)

fire → guard → gate → quarantine → config/feedback → report.

- **Fire**: each file parsed once; all AST rules run over the same tree; text regexes compiled
  once. Whole-repo (1,462 files) match+gate ≈ 6ms with light models, ≈ 330ms with all language
  models; total warm run < 1s (`HELPERS_LINT_TRACE=1` prints the stage split).
- **Restatement guard**: a line sharing ≥3 and ≥half of the rule's own words is quoting the
  law, not breaking it.
- **Hv concept gate**: imprecise (regex / container-only) findings are kept only if the fired
  rule's fingerprint is the nearest concept to the matched construct — one batched
  popcount-grid dispatch per language.
- **Quarantine**: a doc rule firing like scrape noise (>1% of all lines, or ≥20 hits covering
  >10% of one file) is quarantined and reported.
- **Docs are reading material**: md/txt files are linted only by rules written *for* them;
  `any`-language law governs code languages.
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
11. **Register vocabulary hijacking construct selection** (weighted contexts made "Never"
   read decisively forbidding → laws watched `Never`; "use"'s endorsement lean poisoned
   neighboring `unsafe`/`any`; docs-grounded "never" beat project-grounded "unsafe" on
   document order; prose capitalization compiled a case-sensitive `\bUnsafe\b`) → *the law
   hierarchy now leads with existence — project-code grounding, then docs grounding, then
   forbidding context; only informative (non-common) words project context onto neighbors; a
   grounded law word survives remedy-context ineligibility; detectors are case-insensitive on
   the lowercased surface.*

## Open problems (honest)

- **Per-token polarity evidence.** Span prototypes cannot say what one word means; the
  side-count design (tally which label each token appeared under — labels now come only from
  toolchain grounding verdicts, the curated seed is gone; lean = 2:1 majority) fixed
  "use"-class noise in probes but regressed fence orientation and contract tests when rushed.
  Land it with: grounded-only tallies, orientation reading words→sentences→order, and a
  regenerated bootstrap — and update this file first.
- **Two kinds of law the linter cannot watch yet — and says so.**
  (1) A law about pure punctuation: "do not compare types with `==`". The linter watches
  *words*, and `==` has no letters or digits, so there is nothing it can watch for. (2) A law
  whose construct only ever appears inside a quoted string: "never hardcode port 8080" where
  the code says `Listen(":8080")`. The linter checks that a law's word really appears in the
  project's code before watching it, and it deliberately skips the insides of strings and
  comments during that check (otherwise English text in strings would count as "code").
  So `8080`-inside-quotes doesn't count, and the law compiles against a different word or
  reports as not-yet-enforceable. In both cases the output tells you what it is actually
  watching ("Your law, as understood") instead of silently guessing.
- **Per-language law instances can diverge.** An `any`-language law compiles once per
  language against that language's corpus, so picks differ (ruby's instance of a "port 8080"
  law once compiled `from`); "Your law, as understood" currently shows one instance — show
  the divergent ones so the author sees which language misread.
- **Tutorial prose still mints an occasional junk rule** (go.dev tutorial narration became
  `that_the_set_of_albums_w`); the weighted forbidding gate, reference-fire, quarantine, and
  the 2-flag feedback loop are the defense-in-depth — the residue is single rules caught by
  the loop, not storms.
- **Doc-rule recall.** Reference manuals are descriptive; normative style guides (PEP8,
  api-guidelines, effective_go) yield the real rules. More registered sources per language is
  a data edit in `sources.json`.
- **Latent-sequence reasoning ("brain waves").** Inference is already Hv-native end to end;
  a rolling-context classifier (prototypes over context space, not bag space) is the designed
  next step for clause understanding without any typography.

## Operational notes

- Stage timing: `HELPERS_LINT_TRACE=1`. Offline: `HELPERS_LINT_OFFLINE=1`. Force re-learn:
  `HELPERS_LINT_REFRESH=1`. Model cache override: `HELPERS_LINT_MODELS`.
- Caches live in `~/.cache/helpers/`; deleting them is always safe (cold relearn is seconds
  per language, online).
- The polarity bootstrap (`lint-index/polarity-bootstrap.json`) is machine-generated:
  `cargo test --release --lib generate_polarity_bootstrap -- --ignored` — regenerate whenever
  the tokenizer, salience, or seed labeling changes (train/inference consistency).
