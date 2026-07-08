# The AI Linter — how it works

> The single authoritative description of the lint system. Module headers in `native/src/`
> summarize their own file and point here; this document owns the cross-module theory. **No
> semantic change lands without updating this file first** — every regression this system has
> had came from editing behavior without a written model of it.

## The character-level substrate (IN PROGRESS — branch `feat/char-level-substrate`)

> Owner directive 2026-07-07. This section describes the substrate the system is being rewritten
> onto; the word-token substrate below it is what ships on `main` until the migration lands. The
> two are kept side by side deliberately — the rewrite replaces the ATOM everything stands on, so
> nothing below is edited until each downstream piece actually moves.

**The atom is a UTF-8 character, and reading is uniform.** There is no token vocabulary, no
word-frequency table, and no cap — the reader (`lint_char::CharReader`) is an order-4 character
predictor: at each position it predicts the next scalar from the last four, and the PREDICTION
ERROR (surprise) is the one signal everything downstream stands on. "Does English account for
this?" becomes "does the reader predict this character run with low surprise." A slot stores the
predicted CHARACTER, not its hypervector (`char_hv` is a pure function), so a real brain is
megabytes, and the prediction memory is PERSISTED — a loaded brain reads pages back, the
capability whose absence had forced hand-parsing.

**Learning is cumulative — one brain, retain-and-grow.** Reading is the same method wherever it
starts; English is the general solution and each language is that same integration continued from
there, adding its specifics. Reading new material RETAINS prior knowledge (measured: English held
within ε) while extending it.

**Surprise is a GAUGE, never the engine (owner directive 2026-07-07 — the correction that matters
most).** The goal is UNDERSTANDING, driven toward 100% by the training pipeline; a lint error
emerges from understanding and from nothing else. Surprise (prediction error) is at most a
*measurement* that understanding is forming — useful to validate the pipeline (it falls as the
brain learns), and legitimately the signal that a construct is NOVEL *while a language is first
being read*. It is NOT a classifier: after training it is ~0 on everything, and even before, it
inverts exactly where it matters — reserved words the brain already knows read calm, and
English-named identifiers (`build`, `clone`) read calm, so "calm ⇒ prose" is wrong in both
directions. Any mechanism that DECIDES something by thresholding surprise is counting the beads'
frame instead of stringing beads. The real substrate is the **knowledge graph**: the associations
the reader binds as it reads — a word to its dictionary meaning, a construct to the English that
governs it, a rule to the prohibition it states — held and computed in the 1-bit HDC space (bind/
bundle/unbind), which a computer holds by the million. Interpretation is a QUERY over that graph
(what does `goto` connect to → "statement, never, prohibition"), not a number. Segmentation as a
separate surprise-thresholded step is deleted from the design: the reader reads the whole page as
learning and the language MODULE (its subgraph — constructs wired to their governing prose and to
prohibition-meaning) is what the rules are read off of.

**The whole system, stated plainly (owner vision, the north star).** This is not "a linter with an
AI in it" — it is a genuinely code-understanding AI onto which linting is thrown, because something
that understands code this deeply checks language rules for free. It fuses LLM-grade language
understanding, 1-bit HDC associative memory, predictive coding, and programmatic checks into one
system that handles ANY language AND arbitrary English rules a user writes (`lintPref`, the
CS-principles corpus). Its reach is meant to exceed syntax: it flags *bad* code from understanding
— CS principles, DRY, dead code, and architectural/security invariants like least-privilege (an
endpoint that mutates account state MUST be authenticated; if the handler shows no auth/repository
guard, that is an error the AI understands, not a pattern it matched). Understanding is the product;
the linter is the surface. Built right, it is the last linter anyone writes. Everything below and
the migration ahead serve that end — comprehension first, enforcement as a consequence.

**The curriculum, in order:** the whole dictionary (English base) → the WEB DELIVERY LAYER
(`html → css → js`) folded into the machine-global base (`char.global.bin`), so that by the time
the brain reads any documentation it already understands the website it arrives in — then every
OTHER language is read against that base, its constructs and their governing prose bound into that
language's subgraph. The reader reads the whole raw page as learning; it does not threshold
anything. What a language's rules are read off of is the GRAPH — a construct wired to prohibition-
meaning (via the dictionary's negation words) is a rule — not a surprise spike. (An earlier cut
segmented by surprise and is retained only as a validation gauge, never as the mechanism; see
"Surprise is a GAUGE".)

**Storage and distribution — machine-global, DELTA modules, never a project copy** (owner
directive, the disk-space covenant). Per-language artifacts are the SAME machine-global,
registry-distributed, never-in-the-repo modules the word substrate already uses (see "Save" and
"The distribution channel"). The one addition the char substrate REQUIRES: a language module
stores only the contexts it ADDS beyond the base — the delta — never a copy of the English+web
base, or every module re-ships the base and disk explodes. Resolution order is unchanged: a
project names its languages → use the machine module in place → else pull the registry module →
else ask for the docs URL, train instantly, and SAVE the module to the computer (not the project)
for reuse. Project law (`lintPref.{md,txt}`) and the machine CS-principles corpus (pure-English
rules) are read through the same brain and compiled into the project OVERLAY, exactly as today.

**The dictionary meaning network — the comprehension backbone (landing now,
`lint_char::MeaningNetwork`).** As the brain reads the dictionary it BINDS EVERY headword the
dictionary defines to the MEANING of its own definition — single words AND multi-word headwords
(`give up`, `null pointer` are constructs too, keyed by their own token seed). The definition's
leading content words are each `encode`d (the char-level spelling centroid, `CharReader::encode`)
and majority-bundled into one meaning hypervector keyed by the headword's token seed. There is NO
caller cap: the whole machine dictionary is bound (owner directive 2026-07-07 — "use the whole
dictionary, make it work fully"). The measured funnel on the shipped New Oxford American body:
**107,945 entries → 103,142 with a parseable definition → 69,691 bound before (the single-word
filter, the "wtf" cap) → 103,142 bound after** (the 33,451 multi-word headwords the old filter
dropped, now bound). The only remaining bound is dictionary typography, not a work cap: the leading
`MAX_MEANING_WORDS` (12) content words carry a definition's genus; the tail is examples and
cross-references. **All senses are FOLDED, not dropped**: when a headword has several dictionary
entries, `seal` unions their leading content words into one meaning (primary sense first, later
senses appended up to the cap) — one word's meaning reflects its whole dictionary range, and the
fold is purely ADDITIVE so retain-and-grow still holds (a repeated headword only ever GAINS words,
never loses its primary sense). Storage is bounded and delta-honest — only the
headword→definition-word list rides `char.global.bin` (the words themselves, capped per entry,
deflated in the DATA stream — a few MB beside the context memory, never one 1KB Hv per headword),
and the meaning vector is REBOUND on query, so the artifact never carries 103k×1KB. The
graph is a pure query: `meaning_of(word)` rebinds the stored definition into its meaning Hv, and
`related(a, b)` is the Hamming proximity of two words' meanings — words whose definitions share
vocabulary land near each other. This whole-dictionary growth does NOT change the lint verdict or
the mint gate, so `TRAIN_VERSION` is UNCHANGED (only `BRAIN_REV` bumps, rebuilding `char.global.bin`):
the sole runtime consumer, `lint_graph::word_is_english`, queries SINGLE words by `meaning_of(...)`
existence — the additions are multi-word phrases never queried as single tokens, and sense-folding
changes meaning VECTORS, not which single-word headwords exist, so page reading is bit-for-bit the
same. Prohibition/negation meaning EMERGES from the definitions alone,
but the SHAPE of the query matters: `related()` Hamming proximity was MEASURED and does NOT
separate disapproval from neutral reference vocabulary (`incorrect`/`unsafe` sit as near the
prohibition anchors as `array`/`element` do — any distance threshold is both a magic constant and
non-separating). What DOES separate, cleanly and with zero neutral false positives, is
definition-COMPOUNDING: a word is negation when its own definition contains a discovered negator
AND another negator-defined word (`English::is_negation`) — this is what the learned-rule entry
gate reads prohibition off of (see "Entry gates"), never `related()`, never a hand list of
negation words (a firing offense here). Reading more material only ADDS entries; prior bindings
are never overwritten (retain-and-grow).

**Reading a page is UNDERSTANDING, not tag-matching (Phase 2 — `lint_graph::read_page`).** A raw
documentation page is scanned by the only typography the covenant grants — a `<…>` run is ONE
markup token, whitespace splits words, sentence terminals close sentences — and every text run is
then judged by MEANING: a word is English when the dictionary meaning network binds it
(`CharReader::meaning_of`), and a run is PROSE when the MAJORITY of its words so resolve, CONSTRUCT
material when they do not (a comparative majority, never an absolute density threshold). Meaning
alone cannot part a section TITLE from a CODE example when both are two unbound words
(`<h1>flowlang statements</h1>` vs `<pre>goto cleanup</pre>`), so the brain also learns, BY EXPOSURE
over the web curriculum, the register that follows each markup token — an association keyed by the
element's own characters (never a tag name written in code): elements whose contained text read as
code become code carriers, the short title-shaped ones become section headings. At read time that
learned association decides FIRST — a code carrier's contents are the example even when its words
are ordinary English (`blink fast`), and a heading opens a section and never welds its own words
into the governing prose (ledger #22); where no element testifies, the meaning majority decides. A
code example is the maximal run of code-carrier and whole-construct gaps stitched back through a
highlighter's shredding; the prose that governs it is the sentences since the last heading, and the
author's `language-*`/`brush:` mark rides along as the block's hint. The curriculum gate holds: no
trained brain ⇒ no units, never a hand parse.

**Landed so far** (validated, committed on the branch): the `CharReader` core with direct
char-code context addressing (n-gram backoff, no per-char hypervector — training the full
dictionary + crawled web is seconds, not minutes); cumulative retain-and-grow; `HLM1` persistence
(round-trips exactly); the setup verb `lint_char::ensure_brain` running the real curriculum
(dictionary → crawled W3/MDN web), read DEDUPED to representative content (`lint_char::novel_blocks`:
the nav/footer chrome repeated across a 20 000-page crawl is learned ONCE, not once per page — a
correctness fix as much as a speed one, since re-reading identical chrome teaches an order-5
predictor nothing and only skews the frequency curve; raw HTML is preserved so the structure-role
learner still sees markup in context) — so the whole base brain builds in SECONDS, not minutes (the
per-word English judgment `lint_graph::word_is_english` asks the meaning network's cheap EXISTENCE
query `CharReader::has_meaning`, a binary search, never rebuilding a hypervector just to test
`is_some`; that one change cut structure-role learning from ~203s to ~0.2s on the measured corpus);
surprise validated as a GAUGE only (English ~11% of max vs
novel code ~73% before the language is read); and the read path fully MOVED onto the char substrate
— `lint_graph::read_page` forms the units both consumers (`doc_crawler::extract_sections_html_hinted`,
`lint_docs::read_crawled_page`) read, on the meaning network and learned structural roles, and
**Phase 3 DELETED the word-substrate `lint_markup.rs` MarkupBrain** (the `markup.global.bin`
artifact, its `markup-bootstrap.json`, and the dead `lint_docs::html_raw_pages`; the overlay cache
stamp now folds `char.global.bin` in its place; codec kind 9 retired-reserved). **Remaining — the
actual work:** the associative KNOWLEDGE GRAPH (bind construct⊗governing-prose⊗meaning; the
dictionary meaning network above is its comprehension backbone; "brain-waves" latent-sequence
reasoning for real prose understanding); language MODULES as subgraphs (delta-stored, machine-global,
never a project copy); rules read off the graph; the full retrain; and **retiring word-level
`english.knows`** — its two live callers both still stand on the word-substrate English brain and
cannot swap cleanly yet, so they are LEFT with a precise migration recorded here:
`lint_match::select.rs`'s `connective` closure (the single-token "common language accounts for this
word" tie-break in SELECTION) and `lint_char::rules_from_understanding`'s construct filter (`!eng.knows(w)`
picking the docs word English cannot account for). Both replace with the meaning-network judgment
`lint_graph::word_is_english(char_brain, w)` (`CharReader::meaning_of` + morphology) — BLOCKED until
the char brain is threaded into both paths AND their hermetic tests carry a meaning-bound brain
(today those tests load only the English bootstrap, so a naive swap returns nothing / breaks
selection). This is the same migration as "construct selection and polarity onto the graph." Understanding
is the product — surprise only ever measures whether it is forming.

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
- **Honest grounding labels** (landed 2026-07-06 — the correction that makes reading raw
  pages sufficient): the asymmetry between the two verdicts lives in the LABELS, never in
  classify-time thresholds. A **Flagged** verdict is reality saying "broken" — that unit's
  prose labels **bad**, as before. A **Clean** verdict only says "parses", never "endorsed"
  — so by itself it labels **nothing**. Clean prose earns a **good** label only when
  structure adds evidence: a clean block that is the SIBLING of a flagged block in the same
  section is the documented FIX (violation first, fix after — the same convention fence
  orientation trusts), and its prose is genuinely endorsement register. Every other clean
  unit stays unlabeled: reference corpus, reading material, never polarity food. This
  un-teaches all three measured mislabel classes at the ROOT: a lint-rule doc's "incorrect"
  example usually PARSES (`var x = 1`), and under honest labels that prose now trains
  nothing instead of training the endorsement prototype (the classifier stops learning the
  opposite of what the page says); neutral tutorial prose no longer floods the good
  prototype (why the previous bootstrap regeneration got WORSE with more reading — that
  failure mode is gone by construction, and the good prototype becomes genuinely remedy/
  endorsement register); and remedy vocabulary earns good weight from fix-position prose to
  balance the bad weight it earns beside failing code. Recall on pages reality cannot flag
  (lint docs whose bad examples parse) comes back through the second pass that already
  exists: an unlabeled unit's `is_bad` at bind time is the classifier's READING of its
  prose — vocabulary learned from reality elsewhere decides, not the unit's own parse
  verdict.
- The **side-count evidence layer** (per-token polarity, landed 2026-07-06): alongside the
  prototypes, training tallies for every salient token the info-bit weight it carried under
  each honest label — `(labeled bad, labeled good)`. The tally is the word's own
  grounded history, so classification reads **words → sentences → order**:
  1. **Words.** Tokens with a decisive tallied lean vote first. A tally's two sides are
     BAD-LABEL weight vs CLEAN-EXPOSURE weight: every clean-grounded unit tallies as
     exposure (reality the word stood next to — NOT an endorsement label; without the
     denominator every ubiquitous word would lean bad by default: "not" beside 6% flagged
     prose is neutral register, "deprecated" beside 75% is prohibition vocabulary). A bad
     lean needs the bad side at 2:1 over exposure; a "reads clean" lean needs exposure at
     4:1. Under 4 bits of total evidence a token abstains. Tallies cover EVERY read token, common
     words included — deliberately: English carries prohibition in its most ubiquitous
     words (the negation primitives "not", "cannot", "never"), and a salience filter here
     would silence exactly the vocabulary reality polarizes fastest (error pages:
     "cannot be parsed", "is not allowed"). Commonness discounts a token's VOTE (its
     info-bit weight), never the existence of its evidence. A span verdict from tallies
     needs ≥8 leaning bits and a 2:1 side majority — one genuinely informative leaning
     word plus change, so feather-weight filler can never decide a span alone.
     **The dictionary diffuses the leans** (definitions as bindings, landed 2026-07-06):
     the LangBrain keeps each headword's definition content-words, and a token the
     grounding never met inherits its lean from its own definition's tallies ("forbid" =
     "order NOT to do" inherits the grounded lean of "not"). Reality polarizes the
     negation primitives; the dictionary carries that polarity across the whole English
     vocabulary — learned end to end, no seed list, no committed classifier.
  2. **Sentences.** When the tallies cannot decide, the span prototypes vote exactly as
     before (calibrated margin, information-weighted).
  3. **Order.** Callers keep their document-order conventions as the final fallback —
     fence orientation's positive-evidence-only swap (ledger #6a) is unchanged, and a
     drifted or legacy artifact (no tallies serialized) degrades to prototypes, then order.
  **The cold floor — negation read from the dictionary alone.** A classifier that has never
  grounded anything (a fresh machine reading an ungroundable language) still reads overt
  prohibition, because the dictionary exposes its own negation primitive: entries with
  negative morphology define a headword as a NEGATOR plus a word of the headword's own
  surface ("invalid" = "not valid", "unsafe" = "not safe"), so the token that keeps
  appearing in exactly those definitions — far above its background rate — is the
  language's negation word, discovered statistically at dictionary-read time (any
  language's dictionary, any prefix system; nothing enumerated). A word is negation-
  clustered when it is a discovered negator or its own definition contains one ("never" =
  "not ever"). An UNREADY classifier classifies a span prohibition when its negation-
  clustered words carry ≥4 info bits — one REAL negation word, the shape prohibition
  sentences actually take ("Never use X") — and everything else abstains. Endorsement has
  no cold floor — only reality can endorse — so pair orientation stays with document order
  until grounding exists.

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
2. **Read**: the raw page is fed to the reader as one token stream (`<…>` runs are single
   markup tokens — typography, never consulted by name), and `(governing prose, code example)`
   pairs are the READING's own segmentation ("Markup second" above): code spans are
   low-English-density register runs judged with the English brain against the W3-calibrated
   split, governing prose is the prose run above a code span, and a title-shaped gap (short,
   unpunctuated) is a hard boundary — the prose that governs a block never crosses into the
   previous section. Measured on the diversity contract's
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
   (parse/compile check only, never executed, parallel), labeled HONESTLY (substrate
   section): flagged → prose feeds the bad prototype and tallies; clean feeds the good
   side only as a fix-sibling of a flagged block in its section; all other clean prose is
   unlabeled. Docs' claims tested against reality — and where reality is silent, nothing
   is invented.
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
`lint-index/extensions-bootstrap.json` — machine-generated learned data: regenerate with
`cargo test --release --lib generate_extensions_bootstrap -- --ignored`, commit the diff
(`committed_bootstrap_resolves_the_canonical_extensions` pins every canonical wiring), and the
machine map overrides it per language as reading continues. Measured before this held: `.md`
files resolved to a language named "md" while the module trained from CommonMark was named
"markdown" — every machine's markdown module was inert, silently, forever; same for `.yaml`
vs the registry's old "yml" name (the language is registered as "yaml" now — the docs' own
name).

**Lint never learns from the network; setup does — no flags, ever.** A lint run is
REPLAY-ONLY for knowledge: caches, the committed seed, and cached crawl pages (a
`TRAIN_VERSION` bump still re-reads them from disk) — it runs on whatever is set up and
ASKS, by name, for what is not. Two staleness rules keep results flowing while staying
honest (user directive: files on disk are knowledge — read and run, never silently degrade):
- **Outdated knowledge still enforces.** A module whose stamp is stale (an engine
  version bump, a toolchain change, a sources edit) is loaded and used AS-IS when it still
  decodes — old reading beats no reading — and the report names every such language in an
  out-of-date footer instead of pretending the language is not set up. A module that no
  longer decodes is genuinely unusable and falls back to the ask, as before.
- **Lint may VALIDATE, never learn.** When (and only when) outdated modules were used, the
  run spends a bounded moment (~1s budget) attempting the one cheap fix — a registry pull
  of the current module (a couple MB of compiled artifact; never a crawl, never a page,
  never a search). Finished in time ⇒ the next run is current and no footer prints.
  Not finished ⇒ the results still return immediately with the footer: "validation not
  completed — results may be out of date; connect to the internet soon so every linted
  language is current." Crawling and all real acquisition stay in the SETUP verbs.
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
- **Definitions as bindings** (landed 2026-07-06) — each single-word headword keeps the
  content words of its own definition (capped, tokenized by the one tokenizer). This is the
  meaning network the side-count polarity layer diffuses through: a word the grounding never
  tallied inherits its lean from its definition's tallies (see "The side-count evidence
  layer"). The dictionary is data read at setup, the leans are reality's, and the hop is one
  binary search — no curated word list anywhere.
- **The substrate is NOT locked to English** (owner directive, 2026-07-06). Nothing in the
  mechanism knows English: the brain reads whatever dictionary the machine has, the tallies
  polarize whatever negation words that language's grounded doc pages actually use, and the
  definition hop diffuses those leans through that dictionary. English is the default
  TRAINING CORPUS — most coding-language sites are written in it — never an assumption in
  code. A machine with a French dictionary reading French docs would learn French
  prohibition the same way, end to end.

**The dictionary is read ENTIRELY, and the read is WITNESSED** (owner directive 2026-07-07):
the chunk walk must consume the whole body — after the last decoded chunk, the remaining bytes
must be the file's zero padding. A walk that stops early (a malformed chunk mid-file, a layout
drift after an OS update) is a PARTIAL read, and a partial read is REFUSED, never saved: half a
frequency curve is a silently wrong baseline for every judgment downstream, and the committed
bootstrap (a complete brain) outranks a fresh incomplete one. The setup report states the
witness ("read entire dictionary — N chunks, N words, N tokens"); measured on this machine:
781/781 chunks, the 291 KB tail pure zero padding.

The artifact is `english.global.bin` (machine-global, beside the models): the dictionary-fed
reader plus the headword set. It is built once per machine at SETUP time (`action=train`) from
the local dictionary; lint runs only ever load it. Machines without a parseable dictionary load
the committed bootstrap `lint-index/english-bootstrap.json` — machine-generated learned data,
same covenant as the extensions bootstrap: regenerate with
`cargo test --release --lib generate_english_bootstrap -- --ignored` and commit the diff. The
LangBrain is a substrate, not a rule source: it never fires, never gates a project law's
EXISTENCE, and adding meaning on top of it (definitions as bindings — word ⊗ its definition,
the designed "rules MEAN something" step) extends this section rather than adding a mechanism.

## Markup second — the MarkupBrain (HTML read from its own documentation)

> **DELETED (Phase 3, branch `feat/char-level-substrate`).** The word-substrate MarkupBrain and its
> `native/src/lint_markup.rs` are gone — page reading now happens on the char substrate through
> `lint_graph::read_page` (the meaning network judges register, learned structural roles part a
> title from an example), and the HTML curriculum is folded into the character brain by
> `lint_char::ensure_brain`. See "The character-level substrate" at the top of this file. This
> section is retained as HISTORY — the model it describes (HTML learned by reading its own docs,
> no tag named, register is the reading's judgment) is exactly what the char substrate continues.

**Documentation is served as HTML, so the thing that reads documentation must understand HTML —
and it learns HTML the same way it learned English: by reading the docs that define it** (owner
directive 2026-07-07: dictionary ENTIRELY → HTML from the W3 docs → only then language pages,
read exactly as served, never through hand-coded parse expectations). The curriculum order is a
hard dependency chain, not a preference: the MarkupBrain's judgments are made WITH the English
brain (below), so English must exist first; the unit former's judgments are made with the
MarkupBrain, so markup must exist before any language documentation is read. Language training
REFUSES — reported, by name, never silently degraded — when either substrate is missing.

**No tag is ever consulted by name — the page is one token stream and the judgments are the
reading's.** The only programmed piece is typography, from the already-allowed list: a `<…>`
run is ONE MARKUP TOKEN (HTML's word-boundary rule, the same class of mechanism as "whitespace
separates words"), an opening markup token CONTAINS the text until its own name closes it
(the element-containment rule — names compared only to each other, the same class as matching
quotes), and sentence punctuation is what it always was. There is no ENUMERATED code-carrier
list, no boundary list, no element vocabulary written into code anywhere; what a tag MEANS is
learned by exposure, exactly as words were. What the reading itself decides:

- **Code vs prose is a REGISTER judgment, made with the English brain.** The text between
  markup tokens (a GAP) reads as English or it doesn't: the fraction of its word tokens the
  dictionary accounts for is its English density. Prose gaps read high; code reads low. This
  is why English must exist first — "how much of this is English" is only answerable by
  something that knows English. Adjacent code-register gaps separated ONLY by markup tokens
  are one code span (syntax highlighters shred examples into `<span>`s; the register survives
  the shredding), and the span's text is exactly the served characters with the markup tokens
  dropped — code preserved verbatim for grounding, no tag names involved.
- **The split point is LEARNED from the W3 reading, not tuned.** At setup, after the English
  brain and before any language trains, the substrate reads the html language's own registered
  documentation raw — the W3-endorsed WHATWG HTML standard plus MDN's HTML tree, `sources.json`
  rows like any other. That corpus contains both registers by nature, so the density
  distribution over its gaps is bimodal, and the valley between the modes is the calibrated
  prose/code split this machine reads every page with. No html reading ⇒ no calibration ⇒
  language documentation REFUSES to be read (asked for by name, never silently hand-parsed).
- **A boundary is a TITLE-SHAPED gap**: short and unpunctuated (no sentence-ending typography),
  set off by markup tokens — headings, captions, nav labels, whatever the site's markup dialect.
  Governing prose never crosses one, and the title's own words never weld onto the section's
  first sentence (ledger #22's class, dissolved rather than special-cased). The shape's word
  ceiling is learned from the same W3 reading (the gap-length distribution's short mode).
  **A gap that belongs to an OPEN sentence is never a boundary**: open on either side — the
  previous prose gap ended without a sentence terminal and flows in, or the gap itself ends
  unterminated and flows into the next. Flow is containment typography: adjacent gaps' stacks
  NEST (one a prefix of the other — `[p]` into `[p, code]` and back), while a real heading
  and its section's prose never nest, so flow stops exactly where sections do. Both fragments
  a mid-sentence mark splits (`Never use the <code>var</code> statement`) stay prose; no tag
  name is involved.
- **TAG ROLES are learned from the W3 reading, never enumerated** (this is the "reading keeps
  their role" clause made concrete). During the same corpus read, every gap testifies for every
  element that CONTAINS it (the containment stack — nesting is typography, so `<pre><code>
  <span>` all receive the code text they wrap): an element whose contained text reads, with ¾
  decisiveness and real support, as code register earns a code-carrier role; one whose text
  reads as heading-shaped (title shape, majority-English, and NOT continuing an open sentence —
  the two guards that keep highlighter shreds and mid-sentence links from testifying as
  headings) earns a boundary role. Everything else stays density-judged. At read time the
  INNERMOST containing element with a decisive learned role decides a gap's register before
  density does — this is what tells `<pre>goto cleanup</pre>` (code) from `<h1>flowlang
  statements</h1>` (heading) when the two are textually identical, and what keeps a code
  example's English comment lines inside the code span. The roles live in the brain as learned
  data (seed → role); no name is ever compared to a list in code.
- **Author marks stay corroborating provenance, never comprehension**: an `id="…"` anchor
  inside a markup token still names the section (rule ids), a `language-*`/`brush:` class still
  declares a block's own language (ledger #18's gate) — attributes the author wrote, read as
  marks, deciding nothing about what is code or where sections lie.

The artifact is `markup.global.bin` (machine-global, `HLM1`, beside the models): the html-fed
reader (tags learned as vocabulary by exposure — ubiquity strips them of meaning-weight, the
reading keeps their role) plus the calibration (density split, title-gap ceiling) and the
learned tag roles (seed → code-carrier/boundary, the containment-stack tallies above). Built at
SETUP after the English brain and before any language trains; machines without a readable html
cache load the committed bootstrap `lint-index/markup-bootstrap.json` — machine-generated
learned data, same covenant as the English and extensions bootstraps (regenerate with
`cargo test --release --lib generate_markup_bootstrap -- --ignored`, commit the diff).

**Pages are cached RAW, exactly as served** (stage 2+3 of the READ-not-split design): the crawl
cache stores each page's body verbatim in an `HLM1` container (DATA-stream deflate — HTML
compresses several ×), and units are FORMED AT READ TIME by the register reading above. The
enumerated `<pre` / `<h1>`–`<h6>` tag lists are DELETED from the unit former. A cached raw page
can always be re-read by a smarter reader — a former change never needs the network again. The
governing-window tail cap stays interim mechanism, to be dissolved by the sequential layer
(open problems).

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

Entry gates: a **learned** rule (example-backed or not) mints only when some SENTENCE of its
governing prose is **COMMANDED BY A NEGATION OPERATOR** — the understanding gate, not the
register. The reading is `English::states_prohibition`: an `is_negation` word (the meaning
network's definition-COMPOUNDING judgment — a word whose own definition reaches negation; never a
word list, never `related()` proximity, which was measured non-separating) GOVERNS the sentence,
standing within its first two words of a sentence the author MARKED ("Never use X", "Do not call
Y", "Deprecated: never use X"). This REPLACED the statistical `classify_tallied` escape, which
admitted neutral/error-register reference prose as law — descriptive Rust Reference sections ("An
array is a fixed-size sequence", "The rules for Send and Sync match those for normal struct
types") classify as prohibition on register drift alone but command nothing, so understanding
refuses them (measured on this repo: 125→46 findings, 36→~24 files; the `r-expr-*`/`r-type-*`
descriptive junk class is gone). The grounded classifier still gates entry (`classify ==
prohibition`) and still reads the fix and ranks the construct — it supplies the register; the
meaning network supplies the *judgment that a prohibition was commanded*. **Why POSITION, not the
word:** the meaning network cannot tell an imperative "Never use X" from a factual "X never
includes Y" — the SAME word `never` — so only a negation that GOVERNS the sentence (leads it)
reads as a command; a negation buried mid-clause ("…it is not allowed to move fields…") or used
as a descriptive adverb ("this representation never includes a CR") states nothing. **Known
residue** the covenant cannot yet kill: a negation LEADING a state description ("Not supported in
Chrome", "Negative values are invalid") reads as a command and still mints — separating "Not
<imperative>" from "Not <state>" needs the per-token side-count classifier (open problems).
**Recall boundary:** prohibitions phrased WITHOUT a leading negation operator — predicate
verdicts ("this code is incorrect", "X is unsafe") and non-operator deprecation ("X is deprecated
and will be removed") — do NOT mint unless the sentence also leads with a covered operator; the
dictionary meaning network is high-precision, low-recall on prohibition, and a hand list of
disapproval words is a firing offense. Doc prose that means to forbid should COMMAND it. The
sentence is the verdict unit (ledger #6:
never the mixed span; ledger #13: never a single word — one mis-leaning token in a tutorial
paragraph must not admit the paragraph). A prose-derived detector must additionally be grounded
in documented code. Every learned detector (AST or text) must also pass the **reference-fire gate**: it is
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

**An unchanged project replays the whole report — microseconds in the daemon, milliseconds
cold.** The finished report body is a pure function of its inputs, so a WITNESS decides
between "return the stored body" and "run the pipeline". Witnesses must be KERNEL-
SYNCHRONOUS with the mutating syscall — daemon-mediated events were tried and measured
unsound (macOS fseventsd ingests kernel events on a ~10ms cadence, and neither
`FSEventStreamFlushSync` nor `FSEventsGetCurrentEventId` can see an edit made microseconds
before the check — an edit-then-lint replayed a stale CLEAN). Two witnesses qualify, and
the replay has one tier per witness:
- **The event tier (steady state, microseconds — kqueue on macOS, inotify on Linux).**
  On macOS the daemon holds an `EVFILT_VNODE` watch (an `O_EVTONLY` fd) on every walked
  file, every directory, and the auxiliary inputs; on Linux one inotify watch per
  DIRECTORY covers its children (event records carry the child name — no fd-per-file),
  with the same masks minus ATTRIB, the same memos, and the same protocol; a queue
  overflow or a watched directory vanishing flips the set incomplete. Platforms without
  an implementation (Windows today — `ReadDirectoryChangesW` is the named port) simply
  run the stat tier: correct at milliseconds, never stale. the knote is enqueued INSIDE `write`/`rename`/`unlink` themselves, so a
  `kevent(timeout=0)` poll after an edit always sees it — no daemon sits in the path.
  A lint call is then one kevent drain + a memo lookup: zero events since the memo's
  generation ⇒ the stored body IS the answer, in single-digit microseconds of engine
  time. The daemon PREWARMS this tier at `initialize` (a detached lint of every announced
  workspace root, falling back to its own resolved workspace) — everything is already on
  disk, so the FIRST user call lands on the microsecond path too, not on an arming bill.
  The racy gate counts only USER-editable inputs: machine-written artifacts (modules,
  overlays) participate in the witness fold but never extend the racy window — counting
  them let a run's own overlay save hold its own memo hostage (measured). Any event, an incomplete watch set (fd budget, open failure), a fresh process, or
  a non-macOS platform falls to the stat tier — the failure mode is always the slower
  sound path, never a stale answer. Events are exact, so this tier needs no mtime racy
  window. The watch set arms right BEFORE the sweep that computes the memo (arm-before-
  scan: an edit racing the sweep lands as a pending event and dirties the memo it raced);
  derived caches (`lint-verdicts/`, `lint-replay/`) are excluded exactly as they are from
  the stat witness, so a run never invalidates itself. Known residual, documented: a
  write that happens purely through a shared `mmap` with no `write`/`msync` posts no
  vnote — agents and editors write files via `write`+`rename`, and the stat tier still
  catches it on the next miss.
- **The incremental tier (an armed daemon WITH changes — the lint is the change).** The
  drain does not just say "dirty": it names exactly which vnodes fired. When every fired
  path lies inside the project tree (no law/config/model/aux event) and the daemon holds
  the previous run's state (files, decoded verdicts, loaded models, rendered footers),
  the run touches ONLY the change: fired directories rescan with one bulk syscall
  (additions/removals), fired files restat and re-lint through the cached models — the
  1-bit core's actual work, microseconds per file — verdicts update in place, run-level
  shaping recomputes from cached counts, and the body re-renders with the cached
  training footers (models, law, and aux are provably untouched — no events). Re-arm
  reopens only what fired (arm-before-read per file: reopen, then read, so a racing edit
  pends on the new vnode); a final drain must be quiet to commit, and a churning tree
  simply returns results without committing. Any fired path OUTSIDE the tree, a missing
  state cache, or an incomplete watch set falls to the stat tier below.
- **The stat tier (cold starts, aux changes, fallback — milliseconds).** Mtimes are
  updated by the kernel synchronously with the write itself, so a stat sweep can never
  miss an edit. The
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
run, not once per language. Nothing on the hot path parses JSON: the machine extension map
is an `HLM1` binary (`extensions.bin`, kind 8; legacy `.json` reads once and migrates on
the next fold), and the embedded extensions bootstrap — committed reviewable JSON by law —
parses exactly once per process, not once per map generation (its parse was a measured
multi-ms slice of every cold resolution).

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
   root fix is per-token **side counts** from grounded labels — designed, prototyped,
   reverted once (it destabilized fence orientation when rushed), then **re-landed
   2026-07-06** together with honest grounding labels (substrate section above):
   Flagged→bad, fix-sibling→good, all other clean prose unlabeled; words→sentences→order
   reading; orientation's document-order convention untouched.
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
22. **The governing window silently ate the FIRST WORD of every prose** (the tail cap's
   word-boundary snap ran even when nothing was cut, so "**Never** use the goto statement…"
   trained, bound, and classified as "use the goto statement…" — for prohibitions the
   operative negation itself; masked for months because the beheaded span still classified
   through register vocabulary) → *snap to a word boundary only when the cap actually cut;
   found the same night the heading's own text was discovered WELDED to the section's first
   sentence ("StatementsNever use…"), hiding the negation from the reader — the heading is
   the boundary, the governing prose is the section BODY after its closing tag. Both are
   exactly the class of silent comprehension damage the READ-not-split direction exists to
   dissolve.*
23. **Dictionary example sentences and litotes poisoned the meaning network** (definitions
   captured per CHUNK first — a hundred headwords sharing one entry's prose — then per entry
   still absorbed usage examples: "instead" learned negation from "do NOT use the phone —
   write instead", and "plain" from its literal definition "NOT decorated") → *definitions
   align per `<d:entry>`; the definition is the text before the dictionary's own example
   separator; and the cold floor reads only negation OPERATORS — words whose definitions are
   negation COMPOUNDED ("never" = "at NO time … NOT ever"), one negated property being a
   description, not an operator.*

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

- **Per-token polarity evidence — LANDED 2026-07-06** (side-count evidence layer, substrate
  section): grounded-only tallies, words→sentences→order, asymmetric leans, and NO committed
  classifier — the polarity bootstrap and its generator are deleted; the classifier
  self-bootstraps from English plus grounded reading. What remains open here: the tallies
  are per-token bags — they cannot read scope ("never use X **except** when Y") or negation
  order; that belongs to the latent-sequence design below.
- **Error-page remedy prose trains the bad prototype** (measured live, the driver of the
  residual junk class): an MDN error page's example FAILS the toolchain, so the prose around
  it feeds the *bad* prototype — but that prose is remedy language ("can be fixed by
  wrapping…"), so fix-register vocabulary ("fixed", "wrap", "avoid the error") learns a
  prohibition lean, and remedy fragments then pass the sentence gate as law
  (`can_be_fixed_via_js`, `avoid_the_error_wrap_eac`). The defense-in-depth (sentence gate,
  sample-program abstention, reference-fire, quarantine, 2-flag feedback) reduced this from
  storms to single rules that the loop suppresses (demonstrated on this repo: 762 → 343 rules,
  then convergence to CLEAN with every suppression a verified FP) — root fix LANDED
  2026-07-06 (honest labels + side-count layer, substrate section): fix-position prose now
  earns GOOD weight from clean siblings, so remedy vocabulary tallies mixed and abstains
  instead of learning a prohibition lean. Measurement gate: retrain must show the
  remedy-fragment junk class gone, not just suppressed.
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
  (clippy fares far better: its docs' bad examples often genuinely fail `rustc`). Root fix
  LANDED 2026-07-06 (honest labels, substrate section): a clean-parsing "incorrect" example
  now trains NOTHING instead of endorsement, and its unit's `is_bad` at bind time is the
  classifier's reading of the prose through vocabulary learned from reality elsewhere — no
  linter-doc special-casing. Measurement gate: ESLint recall must rise well above 8/290 on
  retrain with no junk storm.
- **The page should be READ, not split — ACTIVE DESIGN (owner directive 2026-07-06), staged:**
  the raw page becomes one token stream the reader ingests whole — tags are vocabulary
  (`pre`, `h2` tokenize like words; ubiquity strips them of meaning-weight, the sequential
  coder learns their ROLE), English and markup understanding live in one brain, and
  grounding verdicts double as segmentation labels. Stage order is a correctness
  dependency, not caution: the segmenter trains on grounded labels, so the label pipeline
  must be honest FIRST (stage 1 — honest labels, the dictionary meaning network,
  self-discovered negation: landed 2026-07-06, and it caught the window code eating the first
  word of every governing sentence — a defect a learned segmenter would have laundered
  into its weights). Stages 2+3 core — LANDED 2026-07-07 (see "Markup second"): the crawl
  cache stores RAW pages exactly as served, the MarkupBrain reads the W3 HTML docs whole
  (tags as vocabulary) and calibrates the register split, and the unit former's enumerated
  tag lists are DELETED — segmentation is the reading's own register/boundary judgment.
  Remaining (open): the sequential layer proper — verdicts as segmentation labels refining
  the register runs, and the governing-context tail cap dissolved behind the
  diversity-contract acceptance gate.
  The original problem statement follows.
- **(superseded statement)** The extraction
  windows have accumulated stacked hand heuristics answering one question — which prose
  governs which code (`GOVERNING_CTX` tail, the 40-word lead-in, the heading cut) — and
  stacked hand rules are the tell that the mechanism is wrong. The direction: feed the
  page in AS-IS and let the reader learn markup the way it learned English — tags are
  just tokens (ubiquitous, so they weigh nothing as meaning, but the sequential coder can
  learn their ROLE: what follows `<pre` reads in code register, what follows a heading
  shifts topic), in ADDITION to English understanding, never replacing it. Under that
  reading, today's programmed decisions become judgments of comprehension: code-vs-prose
  is register the reader can already measure, section boundaries are prediction-error
  spikes (topic shift = surprise — the latent-sequence design below), and "which prose
  governs this example" is what the reading says it is. Reality keeps it honest the same
  way it grounds polarity: a span the toolchain parses IS code (a verdict is a
  segmentation label too), and author marks (fences, `<pre>`, anchors) remain
  CORROBORATING signals the reader gets for free — one more thing it read, not a
  mechanism it depends on. Every current window heuristic, the heading cut included, is
  interim mechanism to be dissolved by this; the diversity contract is the acceptance
  gate, and the failure to design against is circularity (judging governance with
  associations that were themselves formed by the old windows — the substrate must
  re-read raw pages, not launder windowed bindings).
- **Fragment examples false-flag and their descriptive prose trains bad — the one junk
  channel left after honest labels (measured 2026-07-06 on this repo: ~400 findings, all
  from reference-manual pages whose snippets are FRAGMENTS — `#[expect(…)]` alone, use-path
  fragments — that fail `rustc --crate-type lib` as written).** Reality is answering a
  different question ("does this compile as a standalone file?") than the one grounding
  asks ("is this code the docs' example of wrongness?"). Designed fix, data not code: each
  toolchain entry grows an optional WRAP template (toolchains.json is registered data, like
  sources.json) — a Flagged snippet is retried wrapped; wrapped-clean means FRAGMENT, which
  tallies as exposure, never as a bad label. Until it lands, the 2-flag feedback loop is the
  dam, exactly as it was for the pre-honest-labels junk classes.
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
- There is NO polarity bootstrap artifact (deleted 2026-07-06, owner directive): the
  classifier self-bootstraps from English knowledge plus grounded web reading, and travels
  only as the machine-global store (`polarity.global.bin`) and inside per-language registry
  modules. A fresh machine's first grounded training run creates it; nothing is committed.
- The English bootstrap (`lint-index/english-bootstrap.json`) is machine-generated from the
  local dictionary: `cargo test --release --lib generate_english_bootstrap -- --ignored` —
  regenerate whenever the tokenizer or the dictionary parser changes. Machines with a local
  dictionary rebuild `english.global.bin` themselves at setup; the bootstrap only covers
  machines without one.
