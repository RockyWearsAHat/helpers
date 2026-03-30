# VS Code Upstream Proposals

This directory contains PR-ready proposals for VS Code features that git-shell-helpers currently implements via bundle patches and proposed APIs. Each proposal is self-contained with motivation, implementation specification, and backward-compatibility design.

## Proposals

| #   | Target Repo        | Proposal                                                                                    | Status | Upstream Issue                                                                                                                                                                                       |
| --- | ------------------ | ------------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `microsoft/vscode` | [Suppress workspace folder switch dialog](001-suppress-folder-switch-dialog.md)             | Filed  | [#306495](https://github.com/microsoft/vscode/issues/306495) — also refs [PR #292783](https://github.com/microsoft/vscode/pull/292783)                                                               |
| 2   | `microsoft/vscode` | [Git extension headLabel override API](002-git-head-label-override.md)                      | Filed  | [#306496](https://github.com/microsoft/vscode/issues/306496) — also refs [#260706](https://github.com/microsoft/vscode/issues/260706), [PR #305643](https://github.com/microsoft/vscode/pull/305643) |
| 3   | `microsoft/vscode` | [Promote chatParticipantPrivate session events to stable](003-chat-session-focus-stable.md) | Filed  | [#306497](https://github.com/microsoft/vscode/issues/306497) — also refs [#305853](https://github.com/microsoft/vscode/issues/305853), [PR #305730](https://github.com/microsoft/vscode/pull/305730) |

## Obsolescence Strategy

See [OBSOLESCENCE-STRATEGY.md](OBSOLESCENCE-STRATEGY.md) for how our patch system gracefully degrades as upstream features land.

## How We Use These Today

Our extension (`RockyWearsAHat.git-shell-helpers`) implements **branch-per-chat** — each Copilot Chat conversation gets its own git worktree, and switching between chats seamlessly switches the visible branch, file explorer, and git status. This requires three non-standard integrations:

1. **Programmatic folder switching** without a confirmation dialog
2. **Branch display override** in the status bar without mutating HEAD
3. **Chat session focus events** to know when the user switches conversations

All three are currently working via patches + proposed API. These proposals aim to land proper API support so patches become unnecessary.
