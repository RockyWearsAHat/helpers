# VS Code Contributions

We modify **3 things** in VS Code to enable branch-per-chat (each Copilot Chat gets its own git worktree). Two are bundle patches, one uses a proposed API.

---

## At a Glance

| #   | What We Change                         | Where in VS Code Source                                                                                                                                                                                 | Lines Changed         | How Applied       |
| --- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- | ----------------- |
| 1   | **Skip folder-switch dialog**          | [`src/vs/workbench/browser/workbench.ts`](https://github.com/microsoft/vscode/blob/main/src/vs/workbench/browser/workbench.ts) → `enterWorkspace()`                                                     | 1 line                | Bundle patch      |
| 2   | **Show worktree branch in status bar** | [`extensions/git/src/repository.ts`](https://github.com/microsoft/vscode/blob/main/extensions/git/src/repository.ts) → `get headLabel()`                                                                | 1 getter prefix       | Bundle patch      |
| 3   | **Detect chat conversation switches**  | [`vscode.proposed.chatParticipantPrivate.d.ts`](https://github.com/microsoft/vscode/blob/main/src/vscode-dts/vscode.proposed.chatParticipantPrivate.d.ts) → `onDidChangeActiveChatPanelSessionResource` | 0 (uses existing API) | Proposed API flag |

---

## Change 1: Skip Folder-Switch Dialog

**Problem**: When an extension calls `updateWorkspaceFolders()` to swap the open folder, VS Code shows a confirmation dialog. This blocks automated worktree switching.

**What we change**: In `enterWorkspace()`, replace the dialog-gated `stopExtensionHosts()` with a direct `_doStopExtensionHosts()`. Extension hosts still restart normally — only the dialog is removed.

```diff
  // workbench.desktop.main.js (minified) → source: workbench.ts
  async enterWorkspace(workspace) {
-   if (!await this.extensionService.stopExtensionHosts(reason)) return;
+   await this.extensionService._doStopExtensionHosts();
    // ...extension hosts restart, workspace folder switches cleanly
  }
```

**Why it's needed**: Branch-per-chat switches folders every time you click a different chat. A dialog on every switch defeats the purpose.

**Prior art**: [@bpasero already did this](https://github.com/microsoft/vscode/pull/292783) for agent session windows (merged Feb 2026). We're asking for the same behavior as a public API option.

**Proposed upstream fix**: Add `{ suppressConfirmation: true }` option to `updateWorkspaceFolders()`. See [`proposals/001`](proposals/001-suppress-folder-switch-dialog.md).

| Detail                   | Value                                   |
| ------------------------ | --------------------------------------- |
| Bundle patched           | `workbench.desktop.main.js`             |
| Source file              | `src/vs/workbench/browser/workbench.ts` |
| Patch script             | `scripts/patch-vscode-folder-switch.js` |
| Restart needed           | Full quit + reopen (Cmd+Q)              |
| Survives VS Code updates | No — must reapply                       |

---

## Change 2: Show Worktree Branch in Status Bar

**Problem**: When a branch session is active, the status bar shows `dev` (the real HEAD) instead of `feature/my-work` (the worktree's branch). The only way to change it is `git checkout`, which is slow (~150ms), risks stash conflicts, and mutates the working tree.

**What we change**: Prefix the `headLabel` getter with a file check. If `.git/gsh-head-override` exists, return its contents as the branch label. Everything else (git operations, status, diff) uses the real HEAD.

```diff
  // extensions/git/dist/main.js (minified) → source: repository.ts
  get headLabel() {
+   // Display override — cosmetic only, no git state modified
+   try {
+     const override = fs.readFileSync(
+       path.join(this.repository.root, '.git', 'gsh-head-override'), 'utf8'
+     ).trim();
+     if (override) return override;
+   } catch {}
    const e = this.HEAD;
    return e ? (e.name || ...) : ...;
  }
```

Also hides the misleading "Sync" button when override is active (it would say "Sync to origin/dev" while showing `feature/my-work`).

**Why it's needed**: Instant branch display (~0.1ms file write) vs git checkout (~150ms + stash + lock contention). No working tree mutation. No stash conflicts.

**Prior art**: [@sbatten requested better worktree distinction](https://github.com/microsoft/vscode/issues/260706) (Aug 2025). [@lszomoru is scaffolding the Git extension API](https://github.com/microsoft/vscode/pull/305643) (Mar 2026).

**Proposed upstream fix**: Add `headLabelOverride` property to the Git Extension API. See [`proposals/002`](proposals/002-git-head-label-override.md).

| Detail                   | Value                                      |
| ------------------------ | ------------------------------------------ |
| Bundle patched           | `extensions/git/dist/main.js`              |
| Source file              | `extensions/git/src/repository.ts`         |
| Patch script             | `scripts/patch-vscode-git-head-display.js` |
| Restart needed           | Reload Window (Cmd+Shift+P → Reload)       |
| Survives VS Code updates | No — must reapply                          |

---

## Change 3: Detect Chat Conversation Switches

**Problem**: Extensions need to know when the user switches between Copilot Chat conversations (to swap branch context). There's no stable API for this.

**What we use**: The `chatParticipantPrivate` proposed API provides `onDidChangeActiveChatPanelSessionResource` — exactly what we need. No bundle patch required.

```typescript
// Our extension subscribes to chat session focus changes:
vscode.window.onDidChangeActiveChatPanelSessionResource((sessionUri) => {
  // sessionUri identifies which chat conversation is now active
  // → look up the worktree bound to this session → switch branch display
});
```

**How it's enabled** (user setup, not a patch):

1. `~/.vscode/argv.json` → `"enable-proposed-api": ["RockyWearsAHat.git-shell-helpers"]`
2. `package.json` → `"enabledApiProposals": ["chatParticipantPrivate"]`

**Why it's needed**: Chat panels don't fire tab-change events when switching conversations. Without this, we can't detect which chat the user is looking at.

**Prior art**: Active chat session infrastructure work — [PR #304532](https://github.com/microsoft/vscode/pull/304532), [PR #305297](https://github.com/microsoft/vscode/pull/305297), [PR #305730](https://github.com/microsoft/vscode/pull/305730). [@wycats requested session exposure](https://github.com/microsoft/vscode/issues/305853) (Mar 2026).

**Proposed upstream fix**: Promote to stable `vscode.window.onDidChangeActiveChatSession`. See [`proposals/003`](proposals/003-chat-session-focus-stable.md).

| Detail                   | Value                                                        |
| ------------------------ | ------------------------------------------------------------ |
| Bundle patched           | None                                                         |
| Source file              | `src/vscode-dts/vscode.proposed.chatParticipantPrivate.d.ts` |
| Restart needed           | Reload Window                                                |
| Survives VS Code updates | Yes (argv.json is user config)                               |

---

## How It All Fits Together

```
User switches chat conversations
         │
         ▼
  ┌─────────────────────────┐
  │ Change 3: Chat Focus    │  ← detects which chat is active
  │ (proposed API)          │
  └──────────┬──────────────┘
             │
             ▼
  ┌─────────────────────────┐
  │ Change 1: Folder Switch │  ← swaps workspace to worktree folder
  │ (workbench patch)       │     (no confirmation dialog)
  └──────────┬──────────────┘
             │
             ▼
  ┌─────────────────────────┐
  │ Change 2: Branch Label  │  ← shows "feature/my-work" in status bar
  │ (git extension patch)   │     (no git checkout needed)
  └─────────────────────────┘
```

---

## Patch Management

```bash
node scripts/patch-vscode-apply-all.js --check   # show patch status
node scripts/patch-vscode-apply-all.js            # apply all patches
node scripts/patch-vscode-apply-all.js --revert   # restore original bundles
node scripts/patch-vscode-apply-all.js --json     # status as JSON (for extension)
```

The extension checks patches on startup and offers to apply if missing.

When VS Code auto-updates, bundle patches are lost. The extension detects this and prompts reapplication. The proposed API (Change 3) is unaffected.

---

## File Map

```
scripts/
├── patch-vscode-apply-all.js          # Coordinator: backup, apply, check, revert
├── patch-vscode-folder-switch.js      # Change 1: folder switch patch definition
└── patch-vscode-git-head-display.js   # Change 2: git head display patch definition

proposals/
├── README.md                          # Index of upstream proposals
├── 001-suppress-folder-switch-dialog.md   # → microsoft/vscode PR proposal
├── 002-git-head-label-override.md         # → microsoft/vscode PR proposal
├── 003-chat-session-focus-stable.md       # → microsoft/vscode PR proposal
└── OBSOLESCENCE-STRATEGY.md           # How patches auto-retire when upstream lands
```

---

## Upstream Proposals

Each change has a corresponding proposal to land the feature natively in VS Code so our patches become unnecessary:

| Change           | Proposal                                                          | Target                       | Upstream References                                                            |
| ---------------- | ----------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------ |
| 1. Folder switch | [`proposals/001`](proposals/001-suppress-folder-switch-dialog.md) | `microsoft/vscode`           | [PR #292783](https://github.com/microsoft/vscode/pull/292783) by @bpasero      |
| 2. Branch label  | [`proposals/002`](proposals/002-git-head-label-override.md)       | `microsoft/vscode` (git ext) | [Issue #260706](https://github.com/microsoft/vscode/issues/260706) by @sbatten |
| 3. Chat focus    | [`proposals/003`](proposals/003-chat-session-focus-stable.md)     | `microsoft/vscode`           | [Issue #305853](https://github.com/microsoft/vscode/issues/305853) by @wycats  |

Our code is designed to detect native APIs and stop using patches automatically. See [`proposals/OBSOLESCENCE-STRATEGY.md`](proposals/OBSOLESCENCE-STRATEGY.md).
