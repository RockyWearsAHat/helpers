---
description: "Branch lifecycle rules for feature work. Ensures agents commit to the correct branch, merge completed work back to the baseline, and clean up."
applyTo: "**"
---

# Branch Lifecycle — Feature Branch Workflow

Agents working on feature branches must follow this lifecycle to prevent orphaned branches and wrong-branch commits.

## Branch Awareness

At the start of any session involving feature work, call the `workspace_context` MCP tool to orient yourself. It returns the workspace root, current branch, worktree status, and remote for each workspace folder. Use this before making changes.

Before making any commit, confirm which branch you are on:

1. **Call `workspace_context`** to get the current branch and workspace root. This is more reliable than `git branch --show-current` because it covers all workspace roots.
2. **Use the `branch` guard.** Always pass `branch` to the checkpoint tool when working on a named feature branch. This aborts the commit if HEAD has drifted. Example: `{ "all": true, "branch": "feature/my-work" }`.

## When to Use a Feature Branch

**Use a feature branch when:**

- The change spans multiple files or involves structural changes
- The user explicitly asks to branch, or says "create a branch," "feature branch," etc.
- The work may take multiple rounds of edits and testing before it's ready
- There's meaningful risk of breaking existing behavior
- The user wants to review the diff before it lands on the baseline

**Commit directly to the baseline when:**

- The change is a one-file fix, typo correction, or config tweak
- The user says "just fix it" or clearly expects immediate application
- The change is trivially verifiable (syntax check, lint fix)

When in doubt, branch. The cost of branching is one extra merge; the cost of a bad commit on the baseline is a revert.

## Preferred Method: Branch Sessions (Worktree Isolation)

When the `gitShellHelpers.branchSessions.enabled` setting is on (check via `workspace_context`), **use branch sessions for feature work**. This is the preferred workflow because:

- Each chat gets its own isolated worktree — no interference between conversations
- The main repo automatically checks out the feature branch when the chat is focused, so `git branch` shows the feature branch name locally
- Switching between chats automatically switches the visible branch
- The user sees normal branch names — the worktree mechanics are invisible
- Original branch and uncommitted work are automatically stashed and restored

**Starting feature work:**

```
branch_session_start({ branch: "feature/my-work" })
```

**Committing:**

```
checkpoint({ all: true })
```

**Ending the session:**

```
branch_session_end({ branch: "feature/my-work" })
```

**Ending and merging into the baseline in one step:**

```
branch_session_end({ branch: "feature/my-work", merge: true })
```

The extension automatically restores the original branch and pops any stash.

### How it looks to the user

When a branch session is active, the main repository checkout reflects the feature branch. Running `git branch` in the repo root shows the feature branch as current. This is **not** a traditional worktree workflow where changes are hidden in a cache directory — the extension transparently manages focus so the user's terminal, Explorer, and git status all show the right branch.

When the user switches to a different chat, the extension switches the visible branch to match that chat's session (or restores the baseline if the chat has no session). This enables parallel branch work across conversations.

## Fallback: Direct Branching

When branch sessions are **disabled** (or unavailable), fall back to direct branching:

```
git checkout -b feature/my-work dev
```

This is simpler but doesn't support per-chat isolation — all chats share one checkout.

## Session Exit Rule — ALWAYS Return to Baseline

**When a chat session ends, task completes, or the user stops engaging, the workspace MUST be left on the baseline branch** (`dev`, `main`, or whatever the user started on). This is mandatory, not optional.

- With branch sessions: call `branch_session_end` — the extension handles the restore automatically.
- With direct branching: checkpoint, then `git checkout dev`.
- Never leave the user on a feature branch after the conversation ends.

This prevents the user from unknowingly making subsequent work on a stale feature branch.

## Incremental Development Cycle

The expected workflow for feature work is:

1. **Develop on the feature branch.** Make changes, test, checkpoint.
2. **Push the feature branch.** Use `{ "all": true, "push": true, "branch": "feature/..." }` when the work is ready to share.
3. **Merge back to the baseline.** Once the feature is complete and validated, merge it into the team's baseline branch (`dev`, `main`, or whatever the repo uses).
4. **Clean up.** Remove the feature branch and any worktrees after a successful merge.

Do not leave completed feature branches unmerged. A branch that has been validated and accepted should be merged promptly.

## Merging Back

**Preferred: merge-on-end.** When ending a branch session, pass `merge: true` to merge the feature branch into the baseline in one step:

```
branch_session_end({ branch: "feature/branch-name", merge: true })
```

This auto-commits dirty changes, removes the worktree, merges with `--no-ff`, and deletes the feature branch. If merge conflicts occur, the merge is left **in progress** with conflict markers in the workspace files. Resolve them with normal file editing tools (`read_file` to see markers, `replace_string_in_file` to fix), then `git add` + `git commit --no-edit` to complete.

**Fallback: manual merge** (direct branching or when merge-on-end isn't available):

1. Switch to the baseline: `git checkout dev`
2. Merge: `git merge --no-ff feature/branch-name`
3. Resolve conflicts if any.
4. Run tests: `bash ./scripts/test.sh`
5. Checkpoint: `{ "all": true, "context": "merge feature/branch-name into dev" }`

## After Merge — Cleanup

With `merge: true`, cleanup is automatic — the branch is deleted on successful merge.

For manual merges:

1. **Delete the local branch.** `git branch -d feature/branch-name`
2. **Delete the remote branch.** `git push origin --delete feature/branch-name` if pushed.

These are destructive — ask for confirmation when required.

## When NOT to Merge

- **Work is still in progress.** Don't merge partial features.
- **Tests are failing.** Fix first, then merge.
- **The baseline has diverged significantly.** Rebase or merge the baseline into the feature branch first, re-test, then merge forward.
- **You're unsure which branch is the baseline.** Ask the user.

## Branch Naming

Use descriptive prefixes:

- `feature/` — new functionality
- `fix/` — bug fixes
- `refactor/` — structural changes with no behavior change
- `chore/` — maintenance, dependency updates, CI changes

## For Long-Running Branches

If a feature branch lives for more than one session:

1. Periodically merge the baseline INTO the feature branch to stay current.
2. Resolve conflicts incrementally rather than letting them accumulate.
3. Keep the branch focused — avoid scope creep that makes merging harder.
