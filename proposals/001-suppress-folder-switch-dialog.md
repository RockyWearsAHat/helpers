# Proposal: Add `suppressConfirmation` option to `updateWorkspaceFolders()`

**Target**: `microsoft/vscode`
**Component**: Workbench — Workspace Service
**Type**: API Enhancement (Extension API)

## Summary

Add an optional `suppressConfirmation` parameter to `vscode.workspace.updateWorkspaceFolders()` that allows extensions to programmatically switch workspace folders without triggering the user-facing confirmation dialog.

## Motivation

Extensions that manage automated workflows — such as git worktree switching, project scaffolding, or branch-per-conversation architectures — need to programmatically replace workspace folders. Today, `updateWorkspaceFolders()` always routes through `enterWorkspace()`, which calls `stopExtensionHosts()` and presents a blocking confirmation dialog. This makes automated folder transitions impossible without user interaction.

### Real-world use case: Branch-per-chat worktree switching

Our extension ([git-shell-helpers](https://github.com/RockyWearsAHat/github-shell-helpers)) implements branch isolation for Copilot Chat conversations. Each chat gets its own git worktree. When the user switches between chats, the extension calls `updateWorkspaceFolders()` to transition to the corresponding worktree. The confirmation dialog breaks this flow — users would see a dialog on every chat switch, defeating the purpose of seamless branch navigation.

### Prior art: VS Code already does this internally

**[PR #292783](https://github.com/microsoft/vscode/pull/292783)** by @bpasero (merged Feb 4, 2026) implements exactly this behavior for "agent sessions workspace" — it skips the extension host restart dialog when workspace folders change in agent session contexts. The implementation calls `_doStopExtensionHosts()` directly instead of going through the `stopExtensionHosts()` veto chain.

This proposal generalizes that pattern into a public API option so any extension can opt into the same behavior when appropriate.

## Proposed API

```typescript
// Existing signature (unchanged for backward compatibility):
// updateWorkspaceFolders(start, deleteCount, ...foldersToAdd): boolean

// New overload with options:
export namespace workspace {
  export function updateWorkspaceFolders(
    start: number,
    deleteCount: number | undefined | null,
    ...workspaceFoldersToAdd: Array<{
      readonly uri: Uri;
      readonly name?: string;
    }>
  ): boolean;

  // Options variant:
  export interface UpdateWorkspaceFoldersOptions {
    /**
     * When true, skip the confirmation dialog that normally appears when
     * workspace folders are replaced. Extension hosts still restart cleanly.
     *
     * Use this when the folder switch is driven by extension logic (not user
     * action) and the user has already consented to the workflow that triggers
     * folder changes (e.g., selecting a branch, starting a session).
     *
     * Default: false
     */
    suppressConfirmation?: boolean;
  }

  export function updateWorkspaceFolders(
    start: number,
    deleteCount: number | undefined | null,
    options: UpdateWorkspaceFoldersOptions,
    ...workspaceFoldersToAdd: Array<{
      readonly uri: Uri;
      readonly name?: string;
    }>
  ): boolean;
}
```

## Implementation Notes

The change is minimal. In `enterWorkspace()` (workbench lifecycle service):

```diff
  async enterWorkspace(workspace, options?) {
-   if (!await this.extensionService.stopExtensionHosts(reason)) return;
+   if (options?.suppressConfirmation) {
+     await this.extensionService._doStopExtensionHosts();
+   } else {
+     if (!await this.extensionService.stopExtensionHosts(reason)) return;
+   }
    // ...rest unchanged
  }
```

This mirrors the approach in PR #292783. Extension hosts still restart. The only difference is whether the user sees a dialog and can veto the transition. The extension takes responsibility for user consent via its own UX (e.g., the user explicitly selected a branch, started a session, or enabled automated switching).

## Safety Considerations

- **Extension hosts still restart cleanly** — only the dialog is suppressed, not the lifecycle transition.
- **Opt-in per call** — extensions must explicitly pass the option. Default behavior is unchanged.
- **User consent is the extension's responsibility** — the option should be used only when the user has already consented to the workflow. Marketplace review can flag abuse.
- **No security implications** — workspace folders are already under the extension's control via the existing API. This only removes a UX gate, not a security boundary.

## Current Workaround

We patch the VS Code workbench bundle (`workbench.desktop.main.js`) to replace the `stopExtensionHosts()` call with `_doStopExtensionHosts()` in `enterWorkspace()`:

```javascript
// Before (original):
async enterWorkspace(e){if(!await this.extensionService.stopExtensionHosts(d(18199,null)))return;let o=xg(...)

// After (patched):
async enterWorkspace(e){await this.extensionService._doStopExtensionHosts();let o=xg(...)
```

This patch breaks on every VS Code update and must be reapplied. A proper API option would eliminate this maintenance burden.

## Backward Compatibility

- Existing callers of `updateWorkspaceFolders()` are unaffected — the new parameter is optional and defaults to current behavior.
- The options object uses a new parameter position that doesn't conflict with the existing `...workspaceFoldersToAdd` rest parameter.

## References

- [PR #292783](https://github.com/microsoft/vscode/pull/292783) — @bpasero's implementation of dialog suppression for agent sessions (merged Feb 4, 2026)
- [git-shell-helpers branch sessions](https://github.com/RockyWearsAHat/github-shell-helpers) — real-world extension using this pattern for branch-per-chat worktree switching
