# Handoff — remove the dead AI scaffolding from the linter

**Branch:** `feat/lint-index-system`
**Prereq state:** verified working (see "Verification" below). Bug fix committed as `0f6c9c1`.
**Your job:** make the codebase honest — delete the unwired "AI" modules so what's on disk
matches what actually runs. **Do NOT change linter behavior.** The live engine stays exactly as-is.

---

## 0. The one-paragraph mental model

The linter's judgment is **100% deterministic**: a documented rule's `bad`/`good` example is
diffed with tree-sitter, the distinguishing subtree is compiled to a generalized AST pattern
(`lint_match`), and code is matched by exact subtree containment. Behavioral principles
(`lint_practice`) measure each function against the project's own statistical norm. **Knowledge is
learned entirely from curated sources** (`corpus/cs-principles.md` + `lint-index/*.json` + live doc
crawl) — *no rule is hardcoded in Rust*. That part is correct and proven; leave it alone.

What's wrong is the **pile of AI-flavored modules that look like the engine but aren't wired into
judgment**: a mixture-of-experts (`lint_moe`), a BPE tokenizer (`lint_bpe`), a lint GPU kernel
(`lint_gpu`), and an experimental dictionary brain (`lint_lang`). They survive only through
incidental couplings. Remove them.

---

## 1. KEEP — do not touch (these are live)

| Module | Why it's live |
|---|---|
| `lint_match` | **the linter engine** (AST pattern compile + match) |
| `lint_practice` | behavioral/"idea" detector (responsibility/complexity/length vs project norm) |
| `lint_train` | orchestration: resolve rules → build/cache `RuleSet` |
| `lint_docs` | live documentation crawler (clippy/ruff/eslint/staticcheck) |
| `lint_checkers` | toolchain version detection (cache-key freshness) |
| `linter` | `Knowledge::from_text` — parses a markdown doc into rules |
| `lint_ai` | **NOT dead.** The hypervector engine is the substrate of the **memory subsystem** (`memory/embed.rs`, `memory/gpu.rs`, `memory/retriever.rs`, `memory/store.rs`) and the crawler's link-dedup. Keep it. |
| `lint_ast` | **keep `language_of`** — `lint_practice` depends on it. Its MoE-only bits may be trimmed (see step 6). |

---

## 2. REMOVE — dead-for-everything (coupled chain; follow the order)

`lint_moe`, `lint_bpe`, `lint_gpu`, `lint_lang`. Do it in this order so the build never breaks:

### Step A — relocate `model_dir()` (the ONLY live use of `lint_moe`)
`lint_train.rs` imports `lint_moe::Moe` solely for `Moe::model_dir()`, a trivial path helper:
```rust
// currently lint_moe.rs:624
pub fn model_dir() -> std::path::PathBuf {
    if let Ok(d) = std::env::var("HELPERS_LINT_MODELS") { return std::path::PathBuf::from(d); }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".cache/helpers/lint-models")
}
```
Move it into `lint_train.rs` as a private free fn `model_dir()`. Update its 4 call sites in
`lint_train.rs` (`patterns_path`, `cache_path`, `stamp_path`, and the `read_dir(Moe::model_dir())`
in `advice`). Drop `use crate::lint_moe::Moe;`.

### Step B — delete the chain
After Step A, these have no remaining non-test references:
- `lint_moe.rs` (used only by `lint_train` [now gone] and `lint_lang` [also being deleted])
- `lint_bpe.rs` (used only by `lint_moe`)
- `lint_gpu.rs` (used only by `lint_moe`; the memory subsystem has its OWN `memory/gpu.rs` which
  explicitly supersedes it — see its module doc: *"What's different from the older lint_gpu kernel…"*)
- `lint_lang.rs` (referenced by nobody live; its only tests are `#[ignore]`d)

Delete the files and their `pub mod …;` lines in `src/lib.rs` (lines 13–25 region).

> ⚠️ **`lint_lang` is the experimental "dictionary-grounded LangBrain"** referenced in the user's
> memory note as a *future* direction. It is currently unwired. **Confirm with the user before
> deleting it** — they may want it parked rather than removed. Everything else in this list is safe.

### Step C — keep the `gpu` Cargo feature
Do **not** remove the `gpu` feature or the `wgpu`/`pollster`/`bytemuck` deps from `Cargo.toml`.
`memory/gpu.rs` uses `#[cfg(feature = "gpu")]` + `wgpu`. Only the *lint* GPU kernel
(`lint_gpu.rs`) is dead; the *memory* GPU kernel stays.

---

## 3. Optional cleanup (step 6)

`lint_ast` is now used only by `lint_practice::language_of`. You may trim `lint_ast` down to
`language_of` (drop the MoE-only `structural_tokens`, `RuleExample`, `AstHit`, `Signature` if
nothing else references them), or inline `language_of` into `lint_practice` and delete `lint_ast`
entirely. Verify `lint_match` does **not** need it (it has its own private `language()`), then decide.
Also fix the pre-existing warning: `lint_docs.rs:18` imports `LearnedRule` unused.

---

## 4. Validation — run after EACH removal

```sh
cd native
cargo build --no-default-features --features crawl   # the linter's normal build
cargo build                                           # default (includes gpu) — memory must still compile
cargo test  --no-default-features --features crawl --lib   # expect: 80 passed, 2 ignored, 0 failed
```
If any count drops or a test fails, you broke a coupling — revert that step and re-check the map above.

---

## 5. Verification already performed (the bar this handoff was gated on — all PASS)

Tested with the freshly-built binary against scratch projects. All three claims hold:

1. **100% on what it's trained on** — over EVERY real catalog rule, each *compiled* rule fires on
   its own `bad` and clears its own `good`:
   - rust: 525/525 compiled = 100%  ·  python: 468/468 = 100%  ·  javascript: 174/174 = 100%
   - Abstentions (rust 205, python 462, js 137) are **correct**: a semantic/dataflow rule can't be
     learned from one syntactic example, so the engine abstains instead of guessing.
2. **Unknown language, learned purely from a curated doc** — a custom Go rule (in *no* official
   catalog) written as plain markdown `go:bad`/`go:good` trained and fired on the violation
   (robust to extra body lines — not over-fit) and left the idiomatic form clean. `go: 1 rules, from nothing`.
3. **CS behavioral rules, any language** — the corpus prose principles (`single_responsibility`,
   `complexity`) flag the outlier function in Go **and** Python projects with zero false positives on
   the tidy units. Fenced CS code-rules also work cross-language (rust + custom go proven).

### Bug fixed this session (commit `0f6c9c1`, `lint_match.rs`)
When a documented fix *inserts* siblings into a construct (a `None`-guard, early return, try/except
wrap), the unchanged body looked novel, so `novel_root` over-captured the whole unit into a pattern
so literal a stray docstring defeated the match (e.g. "mutable default argument"). Fix: recognize
fix-inserted context and localize to the real violation; also anchor patterns on collection-literal
kinds so `param = []`-style rules are learnable. Recovered ~10 previously-abstained ruff rules.
Regression test: `lint_match::tests::fix_that_adds_a_guard_localizes_to_the_violation_not_the_whole_unit`.

---

## 6. Separate issue (NOT cleanup — data, flag to user)

`lint-index/staticcheck.json` (Go) has 160 rules but **zero code examples** (`exampleBad` empty), so
Go has **no official code rules from the seed** until a live crawl populates them (the practice rules
and custom corpus rules work regardless). Either re-crawl staticcheck capturing examples, or accept
Go as practice-only + custom-rule for now. This is a data-completeness task, independent of the
cleanup above.
