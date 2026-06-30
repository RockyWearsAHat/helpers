# Handoff — linter: deterministic tree-brain shipped clean, comprehension model is the next research

**Branch:** `feat/lint-index-system`
**Supersedes:** `HANDOFF-lint-cleanup.md` (that cleanup is now DONE — see §1).
**Status:** working tree builds, **78 lib tests pass**, linter verified across rust/python/js/go.
Nothing committed this session — see §5 before you commit or push.

---

## 0. One-paragraph mental model (the shared vocabulary the owner uses)

The linter is a **deterministic "tree-brain"**, not a statistical classifier. A documented rule is
compiled to an **emergent AST pattern** — whatever the example's own structure *is*, not a shape we
hardcode (`lint_match`). Linting a project = parse the project into its own tree and **probe it with
the referenced brain parts** (match each rule-pattern by exact sub-tree containment). It is right
100% of the time *on what it emits* and **abstains** otherwise — zero false positives by construction.
"Works 100%" means *never wrong*, not *catches everything*. The owner's words for this: "the tree
brain allows proper linting; the tree is built from documentation understanding, not hardcoded,
purely emergent; referenced for the probes against the project tree." That half is built and proven.

---

## 1. DONE this session (in the working tree, not yet committed)

### A. Dead AI scaffolding removed (the `HANDOFF-lint-cleanup.md` job)
Deleted `lint_moe` (mixture-of-experts), `lint_bpe`, `lint_gpu`, `lint_lang` — they looked like the
engine but never judged anything (only `Moe::model_dir()` was live). Relocated `model_dir()` into
`lint_train.rs` as a private fn. Removed their `pub mod` lines from `lib.rs`. Kept `lint_ai` (the
hypervector substrate — used by the memory subsystem AND the comprehension spike) and the `gpu`
Cargo feature (memory's `memory/gpu.rs` uses it; fixed the stale comment that said the linter judged
on GPU — it does not). **Result: ~270 MB of dead `*.moe.json` / `lang-brain.bin` artifacts no longer
produced; live engine artifacts are <1 MB/lang.** This is the "faster + smaller" the owner asked for.

### B. Recall fix — empty-good repeated-instance collapse (`lint_match.rs`)
Clippy shows ~37% of rules (281/749) with an EMPTY `good` and a multi-statement `bad`
(`if x == true {}` / `if y == false {}`). The old code kept the whole multi-statement tree, so the
pattern demanded *every* instance at once and didn't fire even on its own example. New
`collapse_repeated()`: when `good` is empty and all top-level children share one shape, they are the
same anti-pattern repeated → compile a single instance. Test:
`empty_good_repeated_instances_collapse_to_one_and_fire`.

### C. **Faithful doc reading — the heart of this session** (`lint_docs.rs`)
The owner's key insight: *the rendered docs are ground truth with zero errors; the only bad training
data is what our lossy extraction MANUFACTURES.* The old extractor took `good = blocks.get(1)` (the
2nd code block by POSITION) whenever a page had no explicit marker — fabricating fixes that were never
fixes (it paired a Go `http.Header` violation with an unrelated `mu.Lock()` block, grabbed `// Output:`
comments as "fixes", and treated eslint's option-allowed `== null` as a clean example). Garbage in →
garbage rules. Replaced with **`governed_polarity()`**: for each code block, read the polarity the page
itself asserts in the English that *governs* that block (the prose/heading/`class="…"` immediately
above it), substring-safe ("incorrect" ≠ "correct"). A `good` is ONLY ever a block the page positively
labels a fix — **never a positional guess**. When the page doesn't assert a violation+fix, emit no pair.
The section-fallback (`rules_from_sections`) likewise pairs a fix only when it IMMEDIATELY follows the
bad (adjacency = the page asserting "this fixes that"). Tests: `good_is_never_fabricated_from_position`,
`good_is_taken_only_from_an_explicit_correct_label`, `sections_do_not_pair_a_fix_across_an_unrelated_section`.

> ⚠️ This is a deterministic **cue-vocabulary** reader (general English, not site-specific markers).
> It is the BRIDGE, not the destination — the owner wants this replaced by the comprehension model in
> §3. It is correct and faithful, just not "understanding."

### D. Committed per-language modules (`lint_train.rs`, `lint-models/`)
New `lint-models/<lang>.learned.json` — crawled catalogs checked into the repo so a `git pull` ships
every language's rules offline/instantly (the owner's "the module for that language, pulled here").
`resolve_rules` reads them as a high-quality seed (before the bare `lint-index/` snapshot);
`load_committed_module` prefers on-disk then an **embedded** copy (`EMBEDDED_LINT_MODELS`) for binaries
far from a checkout. `advice()` reads them for descriptions/citations. Present: rust, python,
javascript, typescript (`reference` stripped — the matcher doesn't use it; keeps the repo lean).

### E. Stale "MoE" comments fixed across `linter.rs`, `lint_checkers.rs`, `lint_train.rs`,
`tools/lint.rs`, `Cargo.toml`; `.moe.stamp` → `.patterns.stamp`.

---

## 2. VERIFIED behavior (freshly-built release binary, real scratch projects)

| Claim | Evidence |
|---|---|
| Near-instant | first run trains+caches (~9s); **cached re-run = 0.07s** whole project |
| Precise, never wrong | **0 false positives** across rust/python/js/go on every test |
| Recall on learnable rules | of rules that COMPILE: rust 570/572, python 485/485, js 209/209, go 106/106 **self-fire (≈100%)**; the abstained remainder are semantic/dataflow rules un-learnable from one syntactic example (correct to abstain) |
| Learns a new language from a link | **Go went 0 → 106 rules** purely from the live staticcheck crawl (seed had zero examples); committed-module path makes a learned language pullable |
| Arbitrary CS rules work | appended a brand-new `## No panicking unwrap…` rule to `corpus/cs-principles.md` → it fired on `.unwrap()` with **no code change**, alongside the off-by-one rule |

Run the linter:
```sh
echo '{"root":"/path/to/project"}' | helpers-native call lint
# refresh a language from live docs: HELPERS_LINT_REFRESH=1 ; air-gapped: HELPERS_LINT_OFFLINE=1
```

---

## 3. THE NEXT RESEARCH — dictionary-grounded 1-bit comprehension (owner's chosen direction)

The owner explicitly chose to **research-spike a learned comprehension model** over shipping the
deterministic reader as the final word. The vision, in their words: *"train English first, give it the
task of LEARNING… feed the full HTML, train it over and over until it can recite forward and backward,
then it generates its own test corpus."* The comprehension model READS a doc page and identifies the
violation/fix from **meaning**, so we never depend on cue words or page format. It cannot call an LLM
(Helpers tools are deterministic native Rust) — it must be a **local 1-bit hypervector model**.

### The base we already have (do NOT rebuild)
- `native/src/lint_ai.rs` — the hypervector substrate: `Hv` (8192-bit), `token_hv`, `bind`,
  `Bundler` (majority superposition), `rotate` (position). This is "the base" the owner means.
- **`~/Desktop/OneBit-PC-AI/`** — the sibling project where the 1-bit brain + dictionary grounding
  were already built. Reuse these instead of starting over:
  - `data/dictionary.txt` (**34 MB, the macOS dictionary ALREADY PARSED to text**) ·
    `data/system_dictionary_terms.txt` (651 KB headwords) · `src/generated_dictionary.inc` (35 MB baked).
  - Baker/instiller tools: `tools/pc_bake_dictionary_fast.cpp`, `tools/pc_dict_instill.c`,
    `tools/pc_dictionary_multimodal_seed_fast.cpp`, `tools/pc_foundation_absorb.cpp`.
  - Already-instilled brains: `data/grownet.bin.dict` (10 MB), `data/onebit_brain.bin` (2 MB),
    `data/grownet.bin` (21 MB); brain code `tools/pc_onebit_brain.c`, `tools/pc_grownet.c`.
  - Already-trained onebit LINT models: `models/clippy.*.k3.onebit`, `models/ruff.*.k3.onebit`.
  - Deep prior handoffs (large, read first): `CURRENT_STATE.md` (156 KB),
    `NEXT_AGENT_HANDOFF.md` (141 KB), `CLAUDE.md`.

### Suggested path (English-first, then learn)
1. **Ground English from the parsed dictionary** — map each headword to a grounded `Hv` by bundling
   its definition's word-vectors (`data/dictionary.txt` is the input; `pc_dict_instill` / the
   `grownet.bin.dict` artifact is prior art). Goal: semantically-related words land near each other,
   so "incorrect / wrong / discouraged / triggers a warning" cluster vs "correct / instead / fixed".
2. **Comprehend a doc** — represent each code block's governing prose in the grounded space; polarity
   = grounded similarity to the violation vs fix concept, not keyword presence. Degrade to "no pair"
   when neither is close. This drops `governed_polarity` (§1C) in favour of meaning.
3. **Self-supervise on the page** — train to reconstruct/recite the page so structure is internalized,
   then let the model emit its own (bad, good) corpus, which the deterministic `lint_match` compiles
   and SELF-TESTS (a generated pair that fires on its own good is rejected — the existing safety net).
4. The deterministic tree-brain (`lint_match`) stays the JUDGE; comprehension only improves what feeds
   it. Same discipline both sides: emit only what's certain.

### Honest risk (tell the owner if it stalls)
"Recite the page" ≠ "extract the right rule." Memorization is not structure identification — the leap
is the research, and it may need many iterations. This will NOT be a one-shot clean publish; it is a
spike on top of the known-good deterministic base.

---

## 4. Known data-quality edges (not bugs — honest limits)

- **eslint `eqeqeq` / `no-var` abstain.** Their docs' "correct" examples genuinely include `== null` /
  `var` (option-dependent). A purely-syntactic rule that flagged them would over-fire, so the engine
  correctly drops them. Real semantic rules; comprehension won't change that they're option-dependent.
- **Go / staticcheck is prose, not labeled code.** `staticcheck.dev/docs/checks/` is one JS-rendered
  page; extraction yields fragments + messy ids (`staticcheck_olor_000_font_weight_700_…`). The Go
  committed module was therefore **NOT shipped** (would misbehave). Go relies on the CS corpus +
  practice rules + on-demand crawl until a structured Go source or the comprehension model exists.
- **python ids** are ruff rule *names* (slightly slugified, e.g. `p-print`) — valid, not garbage.

---

## 5. BEFORE you commit or push (owner's hard rule: "must fully work, no followup tweaks")

1. The committed `lint-models/*.learned.json` were generated by the **pre-faithfulness** crawl
   (positional-good era). **Regenerate them** once the extraction approach is final (faithful reader
   OR comprehension model), so the shipped modules contain no fabricated pairs:
   ```sh
   HELPERS_LINT_MODELS=/tmp/m HELPERS_LINT_REFRESH=1 helpers-native call lint   # per language
   # then copy /tmp/m/<lang>.learned.json into lint-models/ with reference stripped
   ```
2. `lint_practice.rs` and `lint_train.rs` are still **untracked** (`??`) on this branch — `git add`
   them with the rest; don't leave the linter half-tracked.
3. Re-run `cd native && cargo build && cargo test --no-default-features --features crawl --lib`
   (expect all green) and re-lint a scratch project to confirm 0 FP before any checkpoint.
4. Do **not** commit `~/Desktop/OneBit-PC-AI` artifacts into this repo (the multi-GB `data/*` are
   gitignored there for a reason).
