---
description: "Engram-inspired session learning system. Agents log actions and outcomes to per-workspace session memory with surprise-weighted retrieval. Ensures agents learn from past mistakes and avoid repeating failed approaches."
applyTo: "**"
---

# Session Learning — Engram-Inspired Agent Memory

When session memory is enabled (`gitShellHelpers.sessionMemory.enabled`), agents have access to a per-workspace learning system that records actions, outcomes, and surprise levels. This system uses TF-IDF indexing with surprise-weighted retrieval — high-surprise events (mistakes, unexpected failures) surface preferentially in searches, similar to how DeepSeek's Engram module uses dopamine-style learning to weight surprising training signals.

## Agent Identity and Accountability

You are GitHub Copilot. Before any non-trivial action, state what you are about to do and why. After completing an action, state what you validated and what you assumed. If an approach fails twice, stop and explicitly reconsider — do not retry the same strategy. Never declare success without running the compiler or test suite.

## The Learning Loop

### Before Acting — Search First (Both Sources)

Before any non-trivial action (refactoring, debugging, multi-file edits), run **both** searches:

**1. Session log — failed approaches from this workspace:**

```
search_session_log({
  query: "refactoring bash monolith extraction",
  current_model: "claude-sonnet-4.6"
})
```

**2. Knowledge base — established patterns and prior art:**

```
search_knowledge_index({ query: "bash script extraction modular" })
```

If session log results show a failed approach with high surprise, **do not repeat that approach** — choose a different strategy. If knowledge base results show an established pattern, use it rather than inventing one.

Both searches are mandatory for non-trivial actions. Skipping either is skipping institutional memory.

### After Acting — Log the Outcome

After every significant action completes, log what happened:

```
log_session_event({
  action: "extracted test detection functions from git-upload to lib/upload-test-detection.sh",
  outcome: "success — all tests pass, lint clean",
  surprise: 0.1,
  model: "claude-sonnet-4.6",
  tags: ["refactor", "bash", "git-upload", "extraction"],
  context: "moved detect_vscode_test_task, detect_test_cmd, summarize_test_output to lib file"
})
```

### Surprise Scoring Guide

| Score   | Meaning               | When to use                               |
| ------- | --------------------- | ----------------------------------------- |
| 0.0     | Completely expected   | Routine task completed as planned         |
| 0.1–0.3 | Slightly unexpected   | Minor complications, easy fix             |
| 0.4–0.6 | Moderately unexpected | Approach needed adjustment mid-task       |
| 0.7–0.9 | Very unexpected       | Complete approach failure, had to restart |
| 1.0     | Totally unexpected    | Fundamental assumption was wrong          |

**Log failures with high surprise.** The system learns most from mistakes, not successes. A successful refactor with surprise=0.1 is routine. A failed refactor where the approach was completely wrong is surprise=0.9 — and that's the entry that will save the next agent from wasting time on the same dead end.

### Model-Tier Gating

Always pass `current_model` when searching. The system boosts same-model matches (1.3x) because model-specific failure patterns are real — what trips up GPT-5.2 may not trip up Sonnet, and vice versa.

Always pass `model` when logging. Future agents running the same model will see your entries surface more prominently.

## Request-Boundary Context Preservation

VS Code may compact older conversation messages to fit context window limits. **Anything not logged to session memory can be lost.** To ensure the full context of a request survives compaction:

**At the START of any multi-step coding task**, log the intent:

```
log_session_event({
  action: "starting: <brief description of what user asked>",
  outcome: "in-progress",
  surprise: 0.0,
  model: "...",
  tags: ["request-start", ...relevant tags],
  context: "user wants: <key details>. plan: <phases>. tests: <how success is measured>"
})
```

**At each loop iteration boundary** (after write, after test run), log the result:

```
log_session_event({
  action: "iteration N: edited X, ran tests",
  outcome: "tests failed: <which tests, error summary> | tests passed",
  surprise: 0.2,  // higher if unexpected
  ...
})
```

This means even if VS Code compacts the 30 prior messages down to a summary, the session log holds the full record and `search_session_log` will surface it for the NEXT iteration.

**Minimum retention guarantee**: Every request gets at least one `request-start` log entry and one `request-end` (or `in-progress`) entry. These two anchor points ensure the full intent is always recoverable.

**User-configurable**: The `gitShellHelpers.sessionMemory.enabled` setting controls whether logging is on. When on, the append-only log at `.github/session-memory/session-log.jsonl` is never compacted — it grows until the user clears it. For full workspace history with indexing, enable the setting and use `build_session_index` (if available) or rely on the auto-rebuild that happens after every write.

## When to Log

**Always log:**

- Any action that failed or produced an unexpected result
- Any approach that was tried and abandoned
- Successful completion of multi-step tasks
- Discovery of a non-obvious codebase constraint or pattern
- Build/test failures and the fix that resolved them

**Skip logging:**

- Trivial reads or searches with no action taken
- Routine single-file edits with expected outcomes
- Answering conversational questions

## Session Summary

At the start of a new session, use `get_session_summary` to orient yourself:

```
get_session_summary({ limit: 10 })
```

This returns aggregate stats (total entries, average surprise, model usage, top tags, outcome breakdown) and the most recent events. Use it to understand the project's history before diving in.

## Anti-Patterns

- **Logging everything** — Only log actions with outcomes, not reads or searches.
- **Always logging surprise=0** — Be honest. If it surprised you, say so.
- **Ignoring search results** — If session memory says "approach X failed last time," don't try approach X again without a specific reason why this time is different.
- **Never searching** — If you skip the pre-action search, you lose the entire benefit of the system.
