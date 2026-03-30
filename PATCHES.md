# VS Code Patches & Proposed API Usage

Branch-per-chat navigation requires three non-standard VS Code integrations:

1. **Proposed API** (`chatParticipantPrivate`) — for chat session focus events. Enabled via `~/.vscode/argv.json`. No bundle patch needed.
2. **Workbench patch** (folder-switch) — to suppress the workspace folder switch confirmation dialog. Applied to the workbench bundle.
3. **Git extension patch** (git-head-display) — to support branch name display override via `.git/gsh-head-override`. Applied to the git extension bundle.

## Chat Session Focus: Proposed API

VS Code already has `vscode.window.onDidChangeActiveChatPanelSessionResource` in the `chatParticipantPrivate` proposed API. This fires whenever the user switches between chat conversations — exactly what we need.

**How it's enabled:**

1. `~/.vscode/argv.json` includes `"enable-proposed-api": ["RockyWearsAHat.git-shell-helpers"]`
2. `vscode-extension/package.json` includes `"enabledApiProposals": ["chatParticipantPrivate"]`
3. The extension subscribes to the event in `activate()` and maps session URIs to worktree bindings.

**Why not a bundle patch?** We originally patched the workbench to write session focus to a JSON file (`~/.cache/gsh/active-chat-session.json`), but the renderer is sandboxed — `import("fs")` silently fails. The proposed API routes through the extension host via `$acceptActiveChatSession` IPC, which is the correct architecture.

**Survives VS Code updates**: `argv.json` is user config, not part of the application bundle. The `enable-proposed-api` flag persists across updates.

**Upstream candidate**: Promote `onDidChangeActiveChatPanelSessionResource` to stable API so extensions don't need the proposed API flag.

## Patch Management

```bash
node scripts/patch-vscode-apply-all.js --check    # check status of all patches
node scripts/patch-vscode-apply-all.js             # apply all patches
node scripts/patch-vscode-apply-all.js --revert    # revert all bundles to pristine
```

**Restart requirements**:

- **Workbench patches** (folder-switch): Quit and restart VS Code (Cmd+Q → reopen). Reload Window is NOT sufficient — Electron caches the workbench bundle.
- **Git extension patches** (git-head-display): Reload Window is sufficient.

The extension checks patch status on activation and offers to apply if missing.

## Workbench Patch: Folder Switch

### What it changes (`patch-vscode-folder-switch.js`)

**Type**: Requested behavior change
**Upstream candidate**: Add `suppressDialogs` option to `updateWorkspaceFolders()`

**Problem**: `updateWorkspaceFolders()` triggers a blocking confirmation dialog in `enterWorkspace()`. This prevents automated worktree switching.

In `enterWorkspace()`, replaces:

```
if (!await this.extensionService.stopExtensionHosts(reason)) return;
```

with:

```
await this.extensionService._doStopExtensionHosts();
```

Skips the veto/dialog chain. Extension hosts still restart cleanly.

**Lines changed**: 1 line modified.

## Git Extension Patch: Head Display Override

### What it changes (`patch-vscode-git-head-display.js`)

**Type**: Display-only hook — no git state is modified
**Target**: `/Applications/Visual Studio Code.app/Contents/Resources/app/extensions/git/dist/main.js`
**Upstream candidate**: Add `headLabelOverride` to the Git Extension API

**Problem**: When a branch session is active, we want the status bar to show the worktree's branch name (e.g. `feature/my-work`) even though the main repo's HEAD stays on `dev`. Previously this required `git checkout` + stash/detach, which was slow (~100ms+), risked stash conflicts, and mutated working tree state.

**Solution**: Patch the `headLabel` getter to check for a `.git/gsh-head-override` file before reading the real `HEAD.name`. When present, its contents (trimmed) are returned as the branch label. All internal git operations (status, diff, index) continue to use the real HEAD.

In `get headLabel()`, prepends:

```javascript
try {
  let g = require("fs")
    .readFileSync(
      require("path").join(this.repository.root, ".git", "gsh-head-override"),
      "utf8",
    )
    .trim();
  if (g) return g;
} catch {}
```

**How the extension uses it**:

- **Focus**: Writes `feature/my-work\n` to `.git/gsh-head-override`, then triggers `repo.status()` to refresh the Git extension.
- **Unfocus**: Deletes `.git/gsh-head-override`, triggers refresh. Status bar reverts to real branch.
- **Crash recovery**: On activation, deletes any stale `.git/gsh-head-override` files.

**Advantages over checkout/stash approach**:

- **Instant**: Single file write (~0.1ms) vs git checkout (~100ms) + stash (~50ms)
- **No stash conflicts**: Working tree and index are never touched
- **No branch lock contention**: Worktree stays on its branch, main repo stays on dev
- **Survives main branch progression**: `dev` can advance (new commits, merges) without any interaction with focus state

**Activation**: Reload Window is sufficient (git extension runs in extension host, not cached by Electron like the workbench bundle).

**Lines changed**: 1 getter augmented with a ~100-char prefix.

## VS Code Update Behavior

When VS Code auto-updates, both bundle patches are lost. The extension detects this and prompts re-application. The proposed API (argv.json) is unaffected by updates.

## File Inventory

| File                                    | Purpose                                           |
| --------------------------------------- | ------------------------------------------------- |
| `scripts/patch-vscode-apply-all.js`     | Coordinator: backup, apply, check, revert         |
| `scripts/patch-vscode-folder-switch.js` | Folder switch patch definition                    |
| `PATCHES.md`                            | This document                                     |
| `~/.vscode/argv.json`                   | Proposed API enablement (user-local, not in repo) |

## Backup

A single pristine (pre-any-patch) backup is maintained at:

```
/Applications/Visual Studio Code.app/.../workbench.desktop.main.js.pristine
```

Legacy per-patch backup files (`.bak`, `.folder-switch.bak`) are cleaned up automatically.
