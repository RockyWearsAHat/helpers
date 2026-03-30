# Proposal: Add `headLabelOverride` support to Git Extension API

**Target**: `microsoft/vscode` (built-in Git extension)
**Component**: Git Extension — Repository Model / Status Bar
**Type**: API Enhancement (Git Extension API)

## Summary

Add a mechanism for extensions to override the branch name displayed in the VS Code status bar and SCM view without modifying the actual `HEAD` reference, index, or working tree. This enables extensions to show contextually meaningful branch names (e.g., the branch of an active worktree session) while leaving git state untouched.

## Motivation

Extensions that manage git worktrees need to communicate which branch the user is "working on" even when the repository's real HEAD points to a different branch. Today, the only way to change the displayed branch name is to actually `git checkout` or `git switch`, which:

1. **Mutates working tree state** — can cause stash conflicts, index lock contention, and file watcher churn
2. **Is slow** — `git checkout` + `git stash` takes ~150ms+ on non-trivial repos
3. **Breaks concurrent operations** — other extensions reading HEAD get the wrong reference during the transition

### Real-world use case: Worktree-based branch sessions

Our extension implements branch-per-chat sessions where each Copilot Chat conversation has its own git worktree. When the user switches between chats, we want the status bar to show `feature/my-work` even though the main repository's HEAD remains on `dev`. The display override is purely cosmetic — all git operations (status, diff, commit) continue using the real HEAD.

### Upstream interest

**[Issue #260706](https://github.com/microsoft/vscode/issues/260706)** by @sbatten (opened Aug 8, 2025, assigned to @lszomoru) requests better visual distinction for worktree windows, noting that worktree branch names can be confusing. This proposal provides the API mechanism to solve that.

**[PR #305643](https://github.com/microsoft/vscode/pull/305643)** by @lszomoru (merged Mar 26, 2026) scaffolds a new Git extension API surface — the foundational infrastructure for adding new public API methods like `headLabelOverride`.

**[Issue #277163](https://github.com/microsoft/vscode/issues/277163)** established Git extension API support for agent/CLI scenarios — the same class of use case that motivates this proposal.

## Proposed API

### Option A: Git Extension API method (preferred)

Extend the Git extension API (exposed via `vscode.extensions.getExtension('vscode.git')`) with a display override:

```typescript
export interface Repository {
  // Existing members...

  /**
   * Override the branch label shown in the status bar and SCM view.
   * When set, this string replaces the real HEAD name in display contexts only.
   * Set to `undefined` to clear the override and revert to the real branch name.
   *
   * This does NOT change the actual HEAD, index, or working tree.
   * All git operations continue using the real HEAD reference.
   */
  headLabelOverride: string | undefined;
}
```

Usage from an extension:

```typescript
const gitExtension = vscode.extensions.getExtension("vscode.git")!.exports;
const api = gitExtension.getAPI(1);
const repo = api.repositories[0];

// Show worktree branch name
repo.headLabelOverride = "feature/my-work";

// Revert to real branch
repo.headLabelOverride = undefined;
```

### Option B: File-based override (simpler, lower-level)

If a full API is premature, a simpler approach: the Git extension checks for a well-known file (e.g., `.git/head-display-override`) and uses its contents as the display label. This requires no API changes — just a small modification to `get headLabel()`:

```typescript
get headLabel() {
  const e = this.HEAD;
  // Check for display override file
  try {
    const override = fs.readFileSync(
      path.join(this.repository.root, '.git', 'head-display-override'),
      'utf8'
    ).trim();
    if (override) return override;
  } catch { /* no override file */ }
  // Original logic
  return e ? (e.name || ...) : ...;
}
```

This is what we currently implement via a bundle patch, using `.git/gsh-head-override` as the file name.

### Supplementary: Hide sync button when override is active

When a display override is active, the "Sync" button in the status bar can be misleading (it would show "Sync to origin/dev" while the display says `feature/my-work`). The override should also suppress the sync button, or the sync button should reflect the override context.

## Implementation Notes

### For Option A (API method)

1. Add `headLabelOverride` property to the `Repository` model
2. In `get headLabel()`, check `this._headLabelOverride` before falling through to real HEAD
3. When the override changes, fire the existing status change event to update the status bar
4. Expose via the Git extension API (`api.d.ts`)

### For Option B (file-based)

1. In `get headLabel()`, add a `try/catch` block reading `.git/head-display-override`
2. In the sync status bar `get command()`, return early when the override file exists
3. No API surface changes needed

Either approach is a ~10-line change to the Git extension.

## Safety Considerations

- **Display only** — no git state is modified. HEAD, index, and working tree are untouched.
- **Ephemeral** — the override is session-scoped (API) or file-based (deleted on cleanup). No persistent state changes.
- **Fail-safe** — if the override file is missing or unreadable, the getter falls through to real HEAD. The `try/catch` ensures no crashes.
- **No security implications** — extensions already have full read/write access to the `.git` directory.

## Current Workaround

We patch the Git extension bundle (`extensions/git/dist/main.js`) to inject override logic into `get headLabel()` and `get command()` (sync button):

```javascript
// headLabel patch — prepends override check:
get headLabel(){let e=this.HEAD;
  try{let g=require("fs").readFileSync(
    require("path").join(this.repository.root,".git","gsh-head-override"),
    "utf8").trim();if(g)return g}catch{}
  return e?(e.name||...

// sync button patch — hides sync when override active:
get command(){
  try{let g=require("fs").readFileSync(
    require("path").join(this.repository.root,".git","gsh-head-override"),
    "utf8").trim();if(g)return}catch{}
  if(!this.state.enabled)return;...
```

This breaks on every VS Code update. A proper API or file convention would eliminate the patch.

## Backward Compatibility

- **Option A**: New optional property on `Repository`. Existing code doesn't reference it and behaves identically.
- **Option B**: New file check in a getter. If the file doesn't exist (the common case), behavior is unchanged. The `try/catch` has negligible performance impact — `readFileSync` on a missing file is ~0.01ms.

## References

- [Issue #260706](https://github.com/microsoft/vscode/issues/260706) — @sbatten requesting better worktree visual distinction (assigned to @lszomoru)
- [PR #305643](https://github.com/microsoft/vscode/pull/305643) — @lszomoru scaffolding the Git extension API (merged Mar 26, 2026)
- [Issue #277163](https://github.com/microsoft/vscode/issues/277163) — Git extension API for agent/CLI scenarios
- [git-shell-helpers](https://github.com/RockyWearsAHat/github-shell-helpers) — real-world extension using this pattern for branch-per-chat sessions
