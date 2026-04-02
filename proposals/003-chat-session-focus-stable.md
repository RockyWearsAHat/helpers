# Proposal: Promote Chat Session Focus Events to Stable API

**Target**: `microsoft/vscode`
**Component**: Chat / Extension API
**Type**: API Promotion (Proposed → Stable)

## Summary

Promote `vscode.window.onDidChangeActiveChatPanelSessionResource` from the `chatParticipantPrivate` proposed API to the stable extension API, enabling extensions to react to chat conversation switches without requiring `enable-proposed-api` flags.

## Motivation

Extensions that maintain per-conversation state — branch sessions, context files, tool configurations, memory scopes — need to know when the user switches between chat conversations. Today, the only mechanism is the `chatParticipantPrivate` proposed API, which requires:

1. Adding `"chatParticipantPrivate"` to `enabledApiProposals` in `package.json`
2. The user manually editing `~/.vscode/argv.json` to include `"enable-proposed-api": ["publisher.extension-id"]`
3. Restarting VS Code

This is a high barrier for a fundamental capability. It's analogous to `onDidChangeActiveTextEditor` — extensions need it for basic state management, and it shouldn't require proposed API access.

### Real-world use case: Branch-per-chat state management

Our extension binds git worktrees to chat conversations. When the user switches from Chat A to Chat B, we need to:

- Update the file explorer to show Chat B's worktree files
- Write a display override so the status bar shows Chat B's branch
- Log the focus change for debugging

Without `onDidChangeActiveChatPanelSessionResource`, we'd have no way to detect conversation switches. The tab-based heuristics we use as fallbacks are unreliable — chat panels share a single tab group, and VS Code frequently flickers focus during tool calls and inline code rendering.

### Active upstream work

The chat session infrastructure is being actively rebuilt:

- **[PR #304532](https://github.com/microsoft/vscode/pull/304532)** — "Chat Session Customizations initial Sketch" (merged ~Mar 25, 2026)
- **[PR #305297](https://github.com/microsoft/vscode/pull/305297)** — "Multi chat support" (merged ~Mar 26, 2026)
- **[PR #305730](https://github.com/microsoft/vscode/pull/305730)** — "Session types changes and smarter chat to session mapping" (merged ~Mar 27, 2026)
- **[Issue #305853](https://github.com/microsoft/vscode/issues/305853)** by @wycats — requesting `chatSessionResource` exposure to `LanguageModelChatProvider` (opened Mar 27, 2026, assigned to @jrieken)

This indicates strong momentum toward making chat session identity a first-class concept in the extension API.

## Proposed Stable API

```typescript
export namespace window {
  /**
   * An event that fires when the active chat panel session changes.
   * This occurs when the user switches between chat conversations
   * in the chat panel, or when a new conversation is started.
   *
   * The event payload is the URI of the newly active chat session,
   * or `undefined` if no chat session is active (e.g., chat panel closed).
   *
   * Extensions can use this to maintain per-conversation state such as
   * workspace context, tool configurations, or branch bindings.
   */
  export const onDidChangeActiveChatSession: Event<Uri | undefined>;

  /**
   * The URI of the currently active chat panel session, or `undefined`
   * if no chat session is active.
   */
  export const activeChatSessionUri: Uri | undefined;
}
```

### Naming note

We suggest `onDidChangeActiveChatSession` (dropping "PanelResource" from the proposed name) for consistency with `onDidChangeActiveTextEditor`, `onDidChangeActiveTerminal`, etc.

## Implementation Notes

The proposed API already provides the full implementation via `$acceptActiveChatSession` IPC from the renderer to the extension host. Promoting to stable requires:

1. Move the event registration from `chatParticipantPrivate` to the stable `window` namespace
2. Remove the proposed API gate
3. Add the `activeChatSessionUri` getter (current value, not just events)
4. Update `vscode.d.ts` with the stable declarations

No new IPC, no new renderer code, no new infrastructure. The hard work is done — this is a promotion, not a new feature.

## Safety Considerations

- **Read-only** — the API only exposes events and the current session URI. Extensions cannot modify, create, or delete chat sessions through this API.
- **Privacy** — the session URI is an opaque identifier (scheme `vscode-chat-session`). It does not expose conversation content, user messages, or model responses.
- **No abuse potential** — knowing which chat session is active has no security implications. It's equivalent to knowing which file tab is active.

## Current Workaround

We use the `chatParticipantPrivate` proposed API with `enable-proposed-api` in `~/.vscode/argv.json`:

```json
// ~/.vscode/argv.json
{
  "enable-proposed-api": ["RockyWearsAHat.git-shell-helpers"]
}
```

```json
// package.json
{
  "enabledApiProposals": ["chatParticipantPrivate"]
}
```

```typescript
// extension.ts
if (vscode.window.onDidChangeActiveChatPanelSessionResource) {
  context.subscriptions.push(
    vscode.window.onDidChangeActiveChatPanelSessionResource(onSessionChanged),
  );
}
```

This works but requires manual `argv.json` setup, cannot be distributed via the Marketplace (proposed APIs are blocked), and is subject to breaking changes without notice.

## Backward Compatibility

- The proposed API event would remain as an alias during a deprecation period
- Extensions currently using the proposed API would work unchanged
- New extensions can use the stable API without any setup

## References

- [Issue #305853](https://github.com/microsoft/vscode/issues/305853) — @wycats requesting chat session resource exposure (assigned to @jrieken)
- [PR #305730](https://github.com/microsoft/vscode/pull/305730) — Session types changes and smarter mapping (merged)
- [PR #305297](https://github.com/microsoft/vscode/pull/305297) — Multi chat support (merged)
- [PR #304532](https://github.com/microsoft/vscode/pull/304532) — Chat session customizations (merged)
- [git-shell-helpers](https://github.com/RockyWearsAHat/github-shell-helpers) — extension consuming this API for branch-per-chat sessions
