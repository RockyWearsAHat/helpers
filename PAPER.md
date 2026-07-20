# A Linter That Learns: Documentation-Grounded Rule Acquisition with a Total Conservation Ledger

**System paper — helpers-native AI linter. 2026-07-20. Status: deployed, measured; every claim below is reproduced by a tracked test or a committed measurement.**

## Abstract

We describe a linter whose rules are not written but **learned**: it reads a programming
language's own documentation, understands prohibition/deprecation statements through a frozen
English meaning network, verifies each candidate rule against the docs' own examples, and
enforces only what it can prove — while accounting for every fact it declines to enforce in a
single conservation ledger. On HTML it enforces **every documented obsolete feature in its
registered corpus (200/201 → all-nine ancient elements after source self-correction), at exact
lines, citing the documentation's own sentences, with zero false positives on modern files and
zero hand-written rules.**

## 1. Design laws (owner-ruled, permanent)

1. **Two axes.** LEARNING is 100% mandatory — the machine never declines to know a documented
   fact. Only ENFORCEMENT may tier down (proven / graded-LOW / quiet), and only with a named,
   queryable reason. "Withheld" vocabulary is banned from the learning path.
2. **Nothing vanishes, at any stage.** Every pipeline gate that drops a fact records a
   stage-prefixed `(id, reason)` row in one conservation ledger (`lint_query kind=rules →
   withheld[]`). "Silently lost" is a failure class with a test.
3. **Understanding over memorization.** A bad/good contrast compiles the *minimal differing
   element in its construct context* (the anchored diff), never the verbatim example bytes.
   Novel-instance probes are mandatory: `packvex("[7,8,9,]")` — never spelled in any doc —
   must fire; its clean twin must not.
4. **No cross-language attestation.** A fact enters a language's web only from that language's
   own registered documentation. A JavaScript page can never teach HTML law.
5. **Coverage comes from learning, not code.** A coverage miss is a label or a data gap, never
   a new reader arm. Hand shape-laws are frozen as label generators until the learned reader
   (PASS 39) dissolves them. Code is legitimate only in the learner, the verification stack,
   and the measuring instruments.

## 2. Architecture

**Substrate.** Binary hypervectors (XOR/popcount, no floats) carry all learned structure: a
frozen dictionary meaning network (English understanding: negation, prohibition, supersession,
polarity — derived from dictionary definitions, never word lists), a character/markup brain
(pages read as one token stream — tags are vocabulary), and per-language webs of
`ConstructNode`s (construct, governing prose, sources, roles, rule/graded tiers, referee state).

**Pipeline.** crawl (per-tool HLM1 page caches, revalidated) → read (facts: subject + the
docs' own forbidding sentence; every prohibition fact retained into the web) → verify
(grammar-as-referee demos, corroboration referee over the meaning net, subject-binding) →
compile (detectors as reader-token sequences with ONE containment matcher — no regex, no
enumerated shapes; anchored-diff pairs; tag-scoped pairs for markup) → enforce (one parse +
one walk per file; findings cite ⟨source⟩ verbatim).

**Reading laws** (frozen, label-generating): page-level status banners; per-attribute badge
entries (`host@attr` constructs); index sections read by repeating-entry structure (tables,
definition lists, shared-description runs) under a status-joined heading; interface pages
mapped by their own stated `<x>` typography; the one-hand-datum store (`deprecation-status.json`)
holds the only human-provided facts: which author status words denote prohibition.

## 3. The measuring instruments

- **Recall census** (`tests/recall_census.rs`): an authored fact manifest generates a fixture
  documentation site in two renders (clean / messy-chrome), trained through the real binary
  hermetically; every fact classified learned / named-quiet / silently-lost. Green =
  knowledge 12/12, silent 0, novel-instance probes fire, on both renders.
- **Ground-truth scoreboard**: 201 corpus-documented HTML obsolete features (25 elements,
  176 attributes, HTML 3.2 → living standard), independently referee'd, re-probed after every
  fresh train.
- **Acceptance fixtures**: per-language planted-violation files verified at exact lines
  (js 86/86, ts 110/110, css 24/24, html 14/14, rust 9/9), clean twins at zero.

## 4. Measured results (2026-07-17 → 20)

| Instrument | Before | After |
|---|---|---|
| Silent losses (census, both renders) | 9/9 facts silently lost | **0** |
| Census enforcement | — | proven 9 + quiet 0 + silent 0 (12/12 knowledge) |
| HTML ground truth (fresh train) | 46/201 fired | **200/201** (only gap: a source-registration gap, then closed) |
| Attribute-level rules | 0/176 | **176/176** |
| Ancient elements (`bgsound isindex keygen listing multicol nextid spacer blink`) | undocumented in corpus | **all fire**, citing WHATWG obsolete.html |
| Clean-file false positives | 1 | **0** |
| Junk rules | 5 | **0** |
| Era pages (1998/2005/2010) | — | 88 findings, all true, zero junk |
| Real languages regression | — | byte-identical module counts to prior baselines, zero junk from new paths |

**Root-cause discipline highlights:** the clean render was *punished for being clean* (page
titles polluting the reference corpus — fixed by the good-contrast discriminator); a
long-standing "drops between surfaces" mystery (`center`) was a referee matching the bare word
in a VR API sentence — fixed by element-typography refereeing; MDN's 2025 reorg silently
deleted pages — detected as live 404s, closed by registering the WHATWG living standard, whose
obsolete list the machine reads through its own structural grammar (unquoted attributes,
definitional-majority lists, shared-description runs).

## 5. What is next (specs committed, not yet built)

- **PASS 38 — citation closure:** registered docs' own citations propose new sources
  (verified same-subject, adoption-as-data, 404 successor hunt). Ends manual registration.
- **PASS 39 — the one-bit predictive reader:** a one-bit predictive coder over the markup
  token stream (designed FOR GPU batch training; prediction-error spikes = segmentation),
  trained on the thousands of grounded labels the frozen arms produced. Shadow-first;
  dissolves the hand arms one measured step at a time.
- **PASS 40 — cross-source ground truth:** independent sources audit each other every
  retrain; whole-web version-over-version conservation.
- **Speaking overlay:** findings composed from the meaning net in any configured human
  language (the docs' verbatim sentence retained as cited proof); retires hand-written
  message templates.

## 6. Reproduction

Everything is a command: `cargo test --lib` (273), `cargo test --test ai_linter_behaviors`
(21, diversity contract), `cargo test --test recall_census` (2, the census law), and
`lint_config action=train` then `lint` on any project. Authoritative spec: `LINTER.md`.
Agent-facing usage: `LINT_AGENTS.md`.
