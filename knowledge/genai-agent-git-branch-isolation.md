# Git Branch Isolation for Parallel AI Agents — Independent Sandboxes, Not Just Organization

## The Fundamental Point

Git branches are not primarily an organizational tool for agents. They are a **runtime isolation mechanism** that makes true parallel agent execution possible.

Without branch isolation, every agent writing to the same codebase shares a single ground truth. When one agent introduces a broken state — a failing build, a broken import, a bad type — that broken state is immediately visible to every other agent. Any agent that subsequently reads files, runs tests, or checks build output is now operating on a poisoned codebase. It may fail for reasons that have nothing to do with its own work.

**Branch isolation solves this at the root.** Each agent's branch is a complete, independent copy of the codebase state at the moment it branched. Agent B cannot see Agent A's failures. Agent A cannot pollute Agent B's environment. They are not sharing a reality — they each have their own.

```
WITHOUT isolation:
  main ← Agent A writes broken file
               ↓
         Agent B tries to build → FAILS because of Agent A's mess
         Agent B is now BLOCKED, waiting for Agent A to fix it

WITH branch isolation:
  main
   ├── feature/agent-a  ← Agent A writes broken file → its build fails
   └── feature/agent-b  ← Agent B is completely unaware → continues working
```

This is the difference between agents that can genuinely run in parallel and agents that are merely queued.

## Why This Matters More Than It Seems

In a sequential pipeline (Agent A finishes → Agent B starts), branch isolation is optional. The agents never overlap.

In a parallel pipeline (Agent A and Agent B both working simultaneously), branch isolation is **mandatory for correctness**. Without it, you don't have parallel agents — you have concurrent agents sharing mutable state with no synchronization, which is a data race at the filesystem level.

Common failure modes on a shared branch with parallel agents:
- Agent B reads a file Agent A half-wrote (mid-edit snapshot)
- Agent B's test suite fails because Agent A deleted a shared utility it needed
- Agent A reverts a file Agent B had already modified, silently erasing B's work
- CI runs against an interleaved state neither agent intended
- The orchestrator receives a test failure that wasn't caused by the agent it asked to run

These failures are hard to diagnose because the symptom (Agent B failing) is causally disconnected from the source (Agent A's change).

## The Model: One Branch Per Agent, One Feature Per Branch

The correct mental model maps directly to professional software engineering practice:

```
main (stable, reviewed, integrated)
 │
 ├── feature/auth-service      ← Agent A owns this
 ├── feature/payment-api       ← Agent B owns this
 └── feature/notification-worker ← Agent C owns this
```

Each agent:
1. Branches from a known-good `main` at task start
2. Commits its own incremental progress to its own branch (`git checkpoint` as it works)
3. Can pull updates from `main` if it needs something another agent already landed
4. Merges back to `main` only when its work is complete and green

Nobody else's failures are on their branch. Nobody else's in-progress work is in their way.

## Branches as a Progress Ledger, Not Just an Isolation Mechanism

A secondary but real benefit: each agent's branch is a complete, attributable record of everything that agent did, in order, with timestamps.

This gives you:
- **Auditability** — `git log feature/agent-a` shows the exact evolution of that agent's work
- **Reversibility** — if Agent A's output is wrong, discard the branch without touching anything else
- **Comparison** — `git diff main..feature/agent-a` shows exactly what the agent changed vs. the baseline
- **Replay** — the branch can be re-run from any prior commit if the agent needs to backtrack
- **Attribution** — in a multi-day or multi-session workflow, you know exactly what each agent produced

Without branches, all of this is lost. You have a flat commit history with contributions from multiple agents interleaved, no clean way to see "what did Agent A do," and no way to revert Agent A's work without touching Agent B's.

## Using `git worktree` for Simultaneous Branch Checkout

A practical concern: multiple agents working in parallel on the same repo would normally require separate clones, because `git checkout` changes the working directory globally.

`git worktree` solves this. It lets you check out multiple branches simultaneously into separate directories from a single repo:

```bash
# Set up parallel worktrees for 3 agents
git worktree add ../workspace-agent-a feature/auth-service
git worktree add ../workspace-agent-b feature/payment-api
git worktree add ../workspace-agent-c feature/notification-worker
```

Each agent gets its own directory with its own branch checked out. They share the `.git` repo object store (efficient), but their working trees are completely separate. No agent's file operations touch another agent's working directory.

Teardown after merge:
```bash
git worktree remove ../workspace-agent-a
```

This is the correct infrastructure primitive for running parallel agents on a single codebase.

## The Dependency Problem: When Branches Are Sequential, Not Parallel

Branch isolation solves the parallel case. It introduces a coordination requirement for the sequential case.

If Agent B's work genuinely depends on Agent A's output:
- Agent B **cannot start** until Agent A's branch is merged to main
- Or Agent B branches from Agent A's branch (creating a dependency chain)
- Or Agent B pulls Agent A's branch directly (creating a soft dependency)

This is not a limitation of branches — it's an honest representation of the actual dependency. If Agent B needs Agent A's code, that dependency exists whether you use branches or not. Branches make the dependency explicit and manageable rather than hidden and racy.

```
Correct dependency modeling:
  main → feature/agent-a (completes + merges)
                ↓
         main (now includes A's work)
                ↓
         feature/agent-b (branches from updated main)
```

Or for tighter coupling:
```
  main → feature/agent-a
              ↓ (Agent B branches from A, not main)
         feature/agent-b (has A's work, starts before A merges)
```

Design the dependency graph explicitly. Run truly independent work in parallel branches. Run dependent work in sequence or as a chain.

## Orchestrator Responsibilities in a Branched Workflow

The orchestrator managing parallel agents needs to:

1. **Create branches before dispatching** — `git checkout -b feature/agent-name` (or `git worktree add`) before the agent runs
2. **Tell each agent its branch** — agents must know which branch they're on so they commit to the right place
3. **Track branch status** — monitor which branches are green, which are in-progress, which have failed
4. **Manage merge order** — when multiple branches are ready, merge in dependency order
5. **Handle conflict resolution** — if two branches touch the same file, merge conflicts require resolution before the second lands
6. **Clean up worktrees/branches** — delete stale branches after merge to avoid accumulation

A minimal orchestrator branch manifest:
```json
{
  "agents": [
    { "id": "agent-a", "branch": "feature/auth-service", "status": "in-progress", "worktree": "../workspace-agent-a" },
    { "id": "agent-b", "branch": "feature/payment-api", "status": "complete", "worktree": "../workspace-agent-b" },
    { "id": "agent-c", "branch": "feature/notifications", "status": "blocked-by": "agent-a" }
  ]
}
```

## When to Merge Back

Merge to main when:
- The agent's branch builds cleanly
- Tests pass on the branch
- A human review gate is satisfied (if required)
- No unresolved dependencies on other branches

**Do not merge a broken branch to main.** The entire point is that main stays clean so new agents can branch from a known-good state.

If an agent's branch is broken and another agent needs to start work that depends on it, one of three things must happen:
1. Fix the broken branch first
2. Start the dependent agent from the last known-good commit on the broken branch
3. Redesign the dependency so it isn't needed yet

## Summary

| Without branch isolation | With branch isolation |
|---|---|
| One agent's broken state blocks all others | Each agent's broken state is contained to its branch |
| Parallel execution is unsafe | Parallel execution is correct |
| No clear attribution of changes | Every agent's work is separately auditable |
| Reverting one agent's work disrupts others | Discard a branch without touching anything else |
| Progress is entangled | Progress is independently tracked per agent |

The primary value is not organizational, not aesthetic, and not about version history. It is that **branch isolation makes independent parallel progress physically possible** in a shared codebase.

## See Also

- **genai-multiagent-orchestration.md** — patterns for coordinating parallel agents
- **genai-agentic-coding.md** — multi-stage code generation pipelines
- **git-branching-strategies.md** — branching models (GitHub Flow, trunk-based, Git Flow)
- **git-advanced-operations.md** — `git worktree`, bisect, and history tools
- **tools-git-internals.md** — how git stores objects and branches at the object level