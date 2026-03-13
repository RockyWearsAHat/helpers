# Git Shell Helpers VS Code Extension

Surfaces community cache participation settings in VS Code's native Settings UI with GitHub authentication integration.

## Settings

| Setting                                           | Default     | Description                                                                                            |
| ------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------ |
| `gitShellHelpers.communityCache.mode`             | `pull-only` | `disabled`, `pull-only`, `pull-and-auto-submit`, `auto-submit-only-public`, or `auto-submit-whitelist` |
| `gitShellHelpers.communityCache.whitelistedRepos` | `[]`        | Repos allowed to submit when mode is `auto-submit-whitelist`                                           |
| `gitShellHelpers.communityCache.availableRepos`   | _(auto)_    | Your GitHub repos (populated by Refresh command)                                                       |
| `gitShellHelpers.communityCache.githubUser`       | _(auto)_    | Authenticated GitHub username                                                                          |

## How it works

- **User settings** (machine-wide) sync to `~/.copilot/devops-audit-community-settings.json`
- **Workspace settings** sync to `.github/devops-audit-community-settings.json`
- On activation, imports existing JSON settings and detects GitHub auth
- On change, writes back to the appropriate JSON file

Community repo, base branch, and branch prefix are predefined and not user-configurable.

## Commands

- **Git Shell Helpers: Log in to GitHub** — opens a terminal for `gh auth login` if not authenticated
- **Git Shell Helpers: Refresh Repository List** — fetches your repos via `gh` and shows a multi-select picker for the whitelist
- **Git Shell Helpers: Show Community Cache Status** — displays current settings and JSON file state

## Install

```bash
cd vscode-extension
npx @vscode/vsce package --no-dependencies
code --install-extension git-shell-helpers-*.vsix
```

Or via the project installer: `./install-git-shell-helpers`

- **Git Shell Helpers: Show Community Cache Status** — displays current settings and JSON file state
