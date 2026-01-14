# Git Shell Helpers

Small quality-of-life helpers wrapped as git subcommands:

- `git upload` – stage, commit, and push, with optional AI-generated commit messages using GitHub Copilot CLI.
- `git get` – initialize a local repo from a remote (like a lightweight `git clone` flow).
- `git initialize` – initialize the current directory as a repo, create an initial commit, set `origin`, and push.
- `git fucked-the-push` – destructive recovery helper to undo the last pushed commit while keeping changes staged.

## Installation options

### 1. macOS .pkg installer (recommended)

On macOS, the preferred way to install is via the native `.pkg` installer:

- Installs binaries into `/usr/local/bin` and man pages into `/usr/local/share/man/man1`.
- Does **not** modify your shell config files.

Grab the latest packaged macOS installer from the latest release:

- [github-shell-helpers latest release](https://github.com/RockyWearsAHat/github-shell-helpers/releases/latest)

After downloading `github-shell-helpers-<version>.pkg`:

1. Double-click the `.pkg`.
2. Follow the standard macOS Installer flow.

Once complete, the commands and man pages should be available immediately in any new shell:

- `git upload`
- `git get`
- `git initialize`
- `git fucked-the-push`
- `git help upload|get|initialize|fucked-the-push`

### 2. One-line script installer (portable alternative)

If you prefer a script-based install into your home directory, use the installer script. This works well on macOS and other Unixy environments where you control your shell config.

Direct download of the installer script:

- [Git-Shell-Helpers-Installer.sh](https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh)

```sh
curl -fsSL \
  https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh \
  | zsh
```

After installation, either open a new terminal or run:

```sh
source ~/.zshrc
```

Then you can use the same commands and help pages as with the `.pkg` installer.

## Why two installer methods?

- **Script installer (`Git-Shell-Helpers-Installer.sh`)**
  - Cross-shell friendly, very easy to share as a copy-pastable command.
  - Installs into your home directory and updates your shell config.
- **macOS `.pkg` installer**
  - Integrates with the native macOS Installer UI.
  - Installs into system-level locations (`/usr/local/...`) without touching your `~/.zshrc`.

Both install the same commands and man pages; they just target different installation styles.

## Development

- Update the version number in `VERSION` before cutting a new release.
- Build artifacts locally:
  - Script installer dist: `./scripts/build-dist.sh` → `dist/Git-Shell-Helpers-Installer.sh`
  - macOS pkg: `./scripts/build-pkg.sh` → `dist/github-shell-helpers-<version>.pkg`
- VS Code tasks:
  - **Build installer** – runs `./scripts/build-dist.sh`.
  - **Build macOS pkg** – runs `./scripts/build-pkg.sh`.

CI (see `.github/workflows/build-installer.yml`) ensures both installers build cleanly on each push to `main`.
