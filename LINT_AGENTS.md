# LINT_AGENTS.md — how an agent uses the AI linter correctly

The linter is an MCP server (`helpers`) + CLI (`helpers-native call <tool>`). It LEARNS rules
from documentation; you never write rules. Read this before touching lint anything.

## Daily use (any repo)

1. **Lint:** MCP tool `lint` (or `helpers-native call lint` with `{"root": "<repo>"}`).
   Offline, milliseconds-warm, never touches the network. Findings cite the documentation's
   own sentence and ⟨source⟩ URL at exact `file:line` — treat them like compiler errors.
2. **A language shows "Not yet set up":** register its official docs once, then train:
   `lint_config {"action":"add_source","lang":"<lang>","url":"<official docs URL>"}` then
   `lint_config {"action":"train"}`. Training needs network; linting never does.
3. **Retrain** (after source changes or a version bump): `lint_config {"action":"train"}` —
   project languages only; `"all":true` for every registered language. Models are
   machine-global (`~/.cache/helpers/lint-models`); every repo benefits.

## Interrogating the machine (do this instead of guessing)

- `lint_query {"kind":"rules","arg":"<lang>"}` — every enforced rule + the CONSERVATION
  LEDGER `withheld: [{id, reason}]`: every fact the machine knows but declines to enforce,
  with its stage-prefixed reason. Nothing is ever silently dropped — if you think a rule is
  "missing", it is either here with a reason, or it is a documentation/source gap.
- `lint_query {"kind":"web","language":"<lang>","arg":"<construct>"}` — what the machine KNOWS
  about a construct: state (PROVEN/GRADED/READ), the docs' governing sentence, sources,
  meaning links. `found:false` ⇒ no registered source documents it.
- Verdict tiers: `[medium]` = proven (verified against the docs' own bad/good examples);
  `[low]`/`graded-*` = attested by documentation, page-cited, weaker proof. Both are real.

## Laws you must not violate (owner-ruled, permanent — see LINTER.md)

1. **NEVER hand-write rules** — not into the corpus, not into code, not "just one". A missing
   rule is a docs/source/label gap. Fix by registering the right source or reporting the miss.
2. **No new reader/shape code.** Coverage misses are labels for the learner (PASS 39), never
   new Rust arms. The three legitimate code zones: learner, verification, measuring
   instruments.
3. **No cross-language attestation.** Never register language A's docs under language B.
4. **Spec-first:** any semantic change updates `LINTER.md` BEFORE the code. Bump
   `TRAIN_VERSION` (lint_train.rs) on logic changes.
5. **Project law** (per-repo rules): put rule docs in `.helpers/lint-rules/<lang>.md` — plain
   English with a forbidding sentence + bad/good examples. The machine compiles them like any
   documentation. This is the ONLY sanctioned way to add a rule by hand, and it lives in
   DATA, in your repo, cited back to your file.

## Validation gates (before claiming any lint-related change works)

- `cd native && cargo test --lib` — unit suites.
- `cargo test --test ai_linter_behaviors` — the diversity contract (~6 min).
- `cargo test --test recall_census` — the census law: knowledge 100%, silent 0 (~12 min).
- After replacing a deployed binary: `codesign --force --sign -` then kill the
  `helpers-native` daemons (they respawn; models apply on restart).

## Cost discipline

Targeted tests first; full census only before a commit. No multi-agent orchestration on this
project without the owner's explicit ask. Probes are throwaway `native/examples/*.rs` —
delete them when done.
