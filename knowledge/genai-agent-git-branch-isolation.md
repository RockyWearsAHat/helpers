# Multi-Agent Git Branch and Worktree Isolation — Enabling True Parallel Execution

## Related
genai-multiagent-orchestration.md, genai-agent-architecture.md, genai-agentic-coding.md, git-branching-strategies.md, git-advanced-operations.md, tools-git-internals.md

## The Fundamental Principle: Isolation as a Data Race Prevention Mechanism

Git branches are **not** primarily an organizational tool. They are a runtime isolation mechanism that makes true parallel agent execution physically possible.

The core insight: **Without branch isolation, agents don't run in parallel — they run concurrently on shared mutable state with no synchronization.** This is a filesystem-level data race.

### The Problem: Shared Mutable State

Without branches, every agent writing to the same codebase shares one ground truth:

```
WITHOUT isolation:
  main (shared state)
    ↓ [Agent A writes broken file, incomplete test setup]
    ↓ [Agent A's state: broken build, dangling imports]
    ↓
  Agent B reads same files → sees Agent A's half-finished changes
  Agent B's test suite → FAILS due to Agent A's incomplete work
  Agent B is now BLOCKED
  Orchestrator receives: "Agent B failed" (but Agent A caused it)
```

This is indistinguishable from a traditional data race in concurrent programming. The symptom (Agent B failure) is causally disconnected from the source (Agent A's change). Debugging is a nightmare.

### The Solution: Branch Isolation

Each agent gets its own isolated branch — a complete, independent snapshot of the codebase:

```
WITH branch isolation:
  main (stable, reviewed)
   
   ├─ feature/agent-a
   │  └─ Agent A writes broken file → its build fails
   │     Only Agent A sees this failure
   │
   └─ feature/agent-b  
      └─ Agent B is completely unaware of Agent A's state
         B's branch operates on a fresh, clean codebase
         B can test, build, and validate independently
```

**This is the difference between agents that genuinely run in parallel and agents that are merely queued.**

### Concrete Failure Modes Without Isolation

Real scenarios that happen when agents share branches:

1. **Mid-edit file snapshots** — Agent B reads a file Agent A is still modifying; gets an inconsistent state (partial write)
2. **Cascade failures** — Agent B's test references a utility Agent A deleted; test fails for a reason outside B's control
3. **Silent erasure** — Agent A's revert of a shared file silently overwrites B's recent changes
4. **Non-deterministic CI** — CI picks up an interleaved state that neither agent individually produced
5. **Blame confusion** — Orchestrator sees "Agent B's changes broke the build" when actually Agent A's broken state in the shared branch caused it

These are concurrency bugs, not logic bugs. Non-deterministic, context-dependent, hard to reproduce.

---

## How `git worktree` Works: The Mechanical Implementation

### The Architecture

`git worktree` solves: **how to check out multiple branches simultaneously without full clones.**

Git separates storage into two parts:

- **`.git/` repository** — Objects, refs, history. Content-addressable storage. Single source of truth.
- **Working tree** — Checked-out files visible to the agent. Editors and builds happen here.

Normally, `git checkout` changes the working tree globally, making simultaneous branches impossible without full clones. A full clone for each agent wastes disk space and creates synchronization headaches.

`git worktree` enables multiple independent working trees from a single `.git/`:

```bash
# Orchestrator creates worktrees for parallel agents
git worktree add ../workspace-agent-a feature/agent-a
git worktree add ../workspace-agent-b feature/agent-b
git worktree add ../workspace-agent-c feature/agent-c

# Result:
# repo/                          (original checkout, main branch)
#   .git/                        (shared object store, refs, history)
#
# ../workspace-agent-a/          (separate working tree)
#   .git   <-- TEXT FILE         (points back to ../repo/.git)
#   [source files for feature/agent-a branch]
#
# ../workspace-agent-b/          (separate working tree)
#   .git   <-- TEXT FILE
#   [source files for feature/agent-b branch]
```

### The `.git` File vs `.git` Directory

Normally, `.git` is a directory containing objects, refs, history. In a linked worktree, `.git` is a **text file**:

```
$ cat ../workspace-agent-a/.git
gitdir: /full/path/to/repo/.git/worktrees/agent-a
```

This file points back to the main repository's object store. All worktrees share the same objects, but each maintains separate:

- **HEAD** (which branch is checked out)
- **Index** (staging area for next commit)
- **Working tree files** (no overlap, no interference)

### Disk Efficiency: Shared Object Store

**Without worktrees (5 full clones of a 500MB repo):**
```
5 × .git/objects    =  2500MB
5 × working trees   =     5MB
TOTAL:              =  2505MB
```

**With worktrees (1 repo + 5 worktrees):**
```
1 × .git/objects    =   500MB  (shared by all)
6 × working trees   =     6MB
TOTAL:              =   506MB
```

**~82% savings.** The object store (commits, history, file blobs) is content-addressed and deduplicated automatically.

Real-world caveat: Build artifacts multiply per worktree. On a ~2GB codebase with build output, 5 worktrees + artifacts can reach ~9-10GB (Cursor forum reports, 2025). Still much better than 5 full clones.

### Core Commands

```bash
# Create worktree with new branch from main
git worktree add -b feature/auth ../workspace-auth main

# Create worktree for existing branch
git worktree add ../workspace-auth feature/auth

# List all worktrees
git worktree list

# Remove when done (branch must not be checked out in another worktree)
git worktree remove ../workspace-auth
git worktree remove --force ../workspace-broken  # force if corrupted

# Clean up stale references after manual directory deletion
git worktree prune

# Lock to prevent accidental removal
git worktree lock ../workspace-auth

# Repair corrupted worktree metadata
git worktree repair
```

---

## Official Adoption: Production Evidence (2025-2026)

### Anthropic / Claude Code — Native Built-in Support (Feb 2026)

Claude Code shipped built-in git worktree support in February 2026 (announced @boris_cherny, Feb 21, 2026). Now documented in Claude Code's official "Common workflows" section for multi-agent parallel development. Prior to this, developers had to wire worktrees manually. Claude Code can now:

- Accept a worktree directory as its working context
- Run concurrently in multiple terminals against different branches
- Commit changes to their assigned branch without interfering with other sessions

### Cursor — Parallel Agents Feature on Worktrees (Nov 2025)

Cursor built their "Parallel Agents" feature directly on git worktrees (documented Nov 2025, dev.to). When Cursor opens parallel agent mode:

- Auto-creates `git worktree` per agent session
- Agents work simultaneously without blocking each other
- Cursor handles merge logic through its UI

### incident.io — Production Deployment (June 2025)

Incident response platform incident.io publicly documented running 4-5 parallel Claude agents routinely:

- Each agent gets its own branch + worktree
- Agents handle separate incident domains (alerts, workflows, reporting)
- Work concurrently; synchronized only at merge
- Measurable improvement in output quality vs sequential single-agent

---

## The Parallel Execution Pattern: Orchestration Workflow

### Orchestrator Pre-Dispatch Setup

Before agents begin work:

```python
# Pseudocode: Orchestrator setup phase

agents = [
    { "id": "auth",  "task": "Implement OIDC provider",  "model": "opus" },
    { "id": "api",   "task": "REST API v2 refactor",     "model": "sonnet" },
    { "id": "tests", "task": "Integration test suite",   "model": "sonnet" }
]

for i, agent in enumerate(agents):
    branch = f"feature/{agent['id']}"
    worktree = f"../workspace-{agent['id']}"

    # Create branch from clean main
    run("git checkout main && git pull")
    run(f"git checkout -b {branch}")
    run(f"git push -u origin {branch}")

    # Create worktree
    run(f"git worktree add {worktree} {branch}")

    # Write per-worktree environment (port offsets, etc.)
    write_env(worktree, agent_index=i, base_port=3000)

    # Install dependencies
    run(f"cd {worktree} && npm ci")

    # Dispatch agent
    dispatch_agent(agent['id'], worktree=worktree, branch=branch, task=agent['task'])

# All 3 agents now running independently in parallel
```

### Each Agent in Its Worktree

In `../workspace-auth`, Agent A works completely independently:

```bash
cd ../workspace-auth
git status         # On branch feature/auth — only this branch's files exist
npm test           # Tests against clean state, no other agent's changes visible
# ... implement OIDC ...
git add auth.py
git commit -m "[auth-agent][step:3/7] Implement OIDC callback endpoint"
# Agent B and Agent C are completely unaware this commit happened
```

### Branch as Progress Ledger

Each branch is a complete, timestamped, attributable record of what an agent did:

```bash
git log feature/auth --oneline
# 4a3c2d1 [auth-agent][step:5/7] Fix OIDC token refresh race condition
# 3b2c1a0 [auth-agent][step:4/7] Add OIDC callback endpoint
# 2a1b0f9 [auth-agent][step:3/7] Install oidc-client library
# 1b0a0e8 [auth-agent][step:2/7] Create OIDC configuration schema

git diff main..feature/auth --stat
# auth.py               | 280 ++
# config/oidc.yaml      |  45 ++
# tests/test_auth.py    | 120 ++

# Agent A's work is wrong → discard without touching anything else:
git branch -D feature/auth
git push origin --delete feature/auth
# feature/api and feature/tests: still intact, still progressing
```

This gives you:
- **Blame** — exactly what did Agent A change, in what order
- **Reversibility** — discard a branch = revert that agent's entire contribution
- **Replay** — `git checkout feature/auth~2` to restore state before last 2 commits
- **Attribution** — in multi-day workflows, know which agent produced which output

---

## Hard vs Soft Dependencies: Modeling the Execution Graph

### True Parallelism: Independent Work

When agents' file scopes don't overlap:

```
main
 ├─ feature/auth       adds: login endpoint, JWT middleware
 ├─ feature/api-v2     refactors: API endpoints (no new auth dependency)
 └─ feature/tests      covers: existing stable utilities
```

All three work simultaneously. Only sync point: merge order to main.

### Soft Dependencies: Start Early, Pull Later

Agent B needs Agent A's output eventually but can start productive work in the meantime:

```
feature/auth (Agent A)
  └─ In progress...

feature/integration (Agent B, started from main)
  └─ Doing: test scaffolding, mocking auth interface, planning
  └─ When Agent A merges: git merge main (pulls auth)
  └─ Finishes: integration with real auth
```

Reduces idle time. Explicitly model this in the orchestration manifest.

### Hard Dependencies: Sequential

Agent B cannot start until Agent A **completes and merges**:

```
feature/db-migration (Agent A)
  └─ Created, tested, merged to main
     ↓
main (now includes new schema)
     ↓
feature/backend (Agent B)
  └─ Branches from updated main
  └─ Uses new schema Agent A created
```

Orchestration manifest:

```json
{
  "id": "backend-agent",
  "branch": "feature/backend",
  "status": "waiting",
  "blocked_by": ["db-agent"]
}
```

Orchestrator holds `backend-agent` until `db-agent` merges, then creates its branch from the updated main.

---

## Real Failure Modes and Mitigations

### 1. Port Conflicts (High Probability)

Multiple agents run dev servers in parallel. All default to the same ports: 3000, 5432, 8080.

**Failure:** `EADDRINUSE: Port 3000 already in use` — Agent B's environment startup fails entirely.

**Solution — Port offset formula:**

```bash
# In ../workspace-agent-a/.env.local:
SERVICE_PORT=3000        # BASE + 0 * 10

# In ../workspace-agent-b/.env.local:
SERVICE_PORT=3010        # BASE + 1 * 10

# In ../workspace-agent-c/.env.local:
SERVICE_PORT=3020        # BASE + 2 * 10

# General: SERVICE_PORT = BASE_PORT + (AGENT_INDEX * 10) + SERVICE_OFFSET
# DB_PORT = 5432 + AGENT_INDEX
```

Orchestrator writes per-worktree `.env` files before dispatch.

### 2. Dependencies Not Carried Over

`node_modules` is in the working tree, not `.git`. New worktrees start empty.

```bash
git worktree add ../workspace-b feature/b
cd ../workspace-b && npm start
# Error: Cannot find module 'react'
```

**Fix:** Run `npm ci` (or `pnpm install`) inside each worktree after creation. With `pnpm`, packages are symlinked from a shared store — much faster than full reinstalls.

### 3. Database and Runtime Services Not Isolated

Worktrees are **filesystem isolated**, not **runtime isolated**. They share: local databases, Docker daemon + volumes, cache directories, message queues.

**Failure:** Agent A's test truncates a table. Agent B's concurrent test queries that table and sees wrong data. B fails for a reason outside B's control.

**Fix:** Per-agent databases. Write each worktree's `.env` with a different `DATABASE_URL` pointing to its own database instance or schema. Or use Docker Compose with agent-indexed volume names and port numbers.

### 4. IDE Recognition Failures

- **Claude Code `/ide`**: Fails with "No available IDEs detected" in a worktree. The scanner doesn't recognize the `.git` text file structure.
- **VS Code**: Added native worktree support July 2025. Open the worktree directory explicitly.
- **JetBrains**: Command-line only; no native worktree creation UI.

For agent-automated workflows: rely on CLI tools, not IDE integration. Claude Code's file editing and terminal tools work fine in worktrees — only the `/ide` bridge fails.

### 5. Disk Space Accumulation

Build artifacts multiply per worktree. Forgotten worktrees silently stack:

**Observed (2025):** 15 forgotten worktrees consuming ~120GB. Cursor forum: 20-minute session on ~2GB codebase → 9.82GB from automatic worktree creation.

**Fix:** Orchestrator cleanup script on agent completion:

```bash
git worktree remove ../workspace-agent-a
rm -rf ../workspace-agent-a/node_modules ../workspace-agent-a/dist
git worktree prune
```

Monitor with `du -sh ../workspace-*/` daily in long-running environments.

### 6. Self-Merge Conflicts

Two agents edit overlapping lines of the same file on separate branches. Both branches merge to main sequentially.

**Prevention:** Task decomposition with non-overlapping file ownership is the correct solution. CODEOWNERS enforced at review time can help.

**Resolution:** Human review, or orchestrator automated strategy (`--strategy-option=ours/theirs` for known-safe patterns).

The "What 371 Git Worktrees Taught Me" article (Level Up Coding, Feb 2026) identifies this as the core operational insight: **multi-agent git workflows are not a coding problem, they are a management problem.** The git mechanics are trivial. The dependency graph, merge sequencing, and conflict ownership are where teams struggle.

---

## Tooling Ecosystem (2025-2026)

| Tool | Purpose | Notes |
|------|---------|-------|
| **git native** | `git worktree` | Ships with all modern git (Git 2.7+, 2015) |
| **agentree** | Python multi-agent orchestration | Dependency graph, dispatch, cleanup |
| **git-worktree-runner** (CodeRabbit) | CLI orchestration | Supports Claude, Cursor, Copilot, Gemini |
| **worktree-cli** | Worktree management + templates | `.env` copying, hook script support |
| **gwq** | Dashboard + tmux integration | Visual status across all active worktrees |
| **ccswarm** | Specialized agent pool coordinator | Frontend/Backend/DevOps/QA pools |
| **Crystal** | Desktop app for parallel agent management | Visual parallel Claude Code + Codex sessions |
| **parallel-cc** | Auto-creates isolated worktrees | Detects parallel Claude Code sessions, auto-provisions |

No single tool covers the full stack (port allocation + DB isolation + IDE integration + conflict prediction + merge orchestration). Most production teams build custom Python or shell orchestration around native `git worktree`.

---

## Orchestrator Responsibilities: The Full Pattern

### Branch Provisioning

```python
def provision_agent(agent_id, task, agent_index, base="main"):
    branch = f"feature/{agent_id}"
    worktree = f"../workspace-{agent_id}"
    
    run(f"git fetch origin {base}")
    run(f"git checkout -b {branch} origin/{base}")
    run(f"git push -u origin {branch}")
    run(f"git worktree add {worktree} {branch}")
    
    write_env(worktree, {
        "AGENT_ID": agent_id,
        "AGENT_INDEX": agent_index,
        "SERVICE_PORT": 3000 + agent_index * 10,
        "DATABASE_URL": f"postgresql://localhost:{5432 + agent_index}/app_{agent_id}"
    })
    run(f"cd {worktree} && npm ci")
    return {"branch": branch, "worktree": worktree}
```

### Assignment Manifest

```json
{
  "agents": [
    {
      "id": "auth-agent",
      "branch": "feature/oidc-integration",
      "worktree": "../workspace-auth",
      "status": "in-progress",
      "blocked_by": [],
      "blocks": ["integration-tests"],
      "last_commit": "3b2c1a0",
      "last_updated": "2026-03-26T10:15:00Z"
    }
  ]
}
```

### Merge Sequencing

```python
def run_merge_pipeline(agents):
    merge_order = topological_sort(agents)  # Respects blocked_by graph
    
    for agent in merge_order:
        if agent["status"] != "complete":
            continue
        
        run(f"git checkout {agent['branch']}")
        if run("npm test").returncode != 0:
            agent["status"] = "failed"  # Escalate, don't merge broken branch
            continue
        
        run("git checkout main")
        if run(f"git merge --no-ff {agent['branch']}").returncode != 0:
            resolve_conflict_or_escalate(agent['branch'])
        else:
            run("git push origin main")
            agent["status"] = "merged"
            unblock_dependents(agent, agents)
```

### Cleanup After Merge

```python
def cleanup_agent(agent_id):
    agent = get_agent(agent_id)
    run(f"git worktree remove {agent['worktree']}")
    run(f"git branch -d {agent['branch']}")
    run(f"git push origin --delete {agent['branch']}")
    run("git worktree prune")
```

---

## Preview Environments as an Alternative

Local worktrees solve **code isolation** but not **runtime isolation** (databases, ports, Docker). Cloud preview environments solve both — each branch gets a completely isolated stack:

```
Branch push: feature/oidc-integration
  ↓ Preview provisioned:
  ├─ Isolated application container
  ├─ Independent PostgreSQL instance (cloned + sanitized from prod)
  ├─ Unique URL: oidc-integration--pr-123.preview.company.com
  └─ Full integration tests run against real infrastructure
```

**Trade-offs:**

| | Local Worktree | Cloud Preview |
|--|--|--|
| Startup | Seconds | 2–10 minutes |
| Cost | Disk space only | $5–50/day/branch |
| Database isolation | Manual setup | Automatic |
| Production fidelity | Approximate | Exact |
| Fast iteration | Yes | No |

**Use worktrees** for: fast iteration, single codebase, stateless or easily mockable services.  
**Use preview environments** for: microservices, complex stateful infrastructure, final pre-merge validation.

---

## When NOT to Use Branch Isolation

1. **Single-agent sequential work** — No parallelism, no benefit
2. **One-file trivial changes** — Setup overhead exceeds value
3. **Agents need real-time state sharing** — Tasks so coupled they must react to each other's current (not merged) changes
4. **Merge conflicts are constant** — Task decomposition is wrong; fix the decomposition before adding worktrees
5. **Resource-constrained machines** — Even minimal worktrees + build artifacts can overwhelm low-disk environments

---

## Quick Reference Commands

```bash
# Setup
git worktree add -b feature/x ../workspace-x main   # New branch + worktree
git worktree add ../workspace-x feature/x            # Existing branch

# Inspect
git worktree list
du -sh ../workspace-*/

# Within worktree
git status
git log --oneline -5
git diff main..

# Push from worktree
git push origin HEAD

# Merge (from main repo)
git checkout main && git pull
git merge --no-ff feature/x && git push origin main

# Cleanup
git worktree remove ../workspace-x
git worktree prune
git branch -d feature/x && git push origin --delete feature/x
```

---

## Summary

| Property | Value |
|----------|-------|
| **Primary problem solved** | Filesystem-level data race in concurrent agents on shared mutable state |
| **Isolation mechanism** | Separate HEAD + index + working tree per worktree; shared `.git` object store |
| **Disk savings** | ~82% vs full clones |
| **Official adoption** | Claude Code native (Feb 2026), Cursor Parallel Agents (Nov 2025), incident.io production (Jun 2025) |
| **Runtime isolation** | NOT included — database, Docker, ports require manual per-worktree setup |
| **Key tools** | `agentree`, `git-worktree-runner`, `gwq`, `ccswarm`, `Crystal`, `parallel-cc` |
| **Orchestrator burden** | Branch provisioning, assignment tracking, merge sequencing, conflict resolution, cleanup |
| **Best fit** | 2–10 parallel agents on independent tasks in a single codebase |
| **Alternative** | Cloud preview environments for full runtime isolation |

**The primary value is not organizational elegance or clean history. It is that branch isolation makes independent parallel progress physically possible in a shared codebase without filesystem-level data races.**