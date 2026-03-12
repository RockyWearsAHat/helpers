#!/bin/zsh

# Git-Shell-Helpers-Installer.sh
#
# Standalone installer script that fetches the latest versions of the
# helper commands and man pages from GitHub and installs them into:
#   - ~/bin
#   - ~/man/man1
# then wires PATH and MANPATH in ~/.zshrc.
#
# Usage (one line):
#   curl -fsSL \
#     https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh \
#     | zsh

set -euo pipefail

REPO_RAW_BASE="https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main"

BIN_DIR="${HOME}/bin"
MAN_DIR="${HOME}/man/man1"
ZSHRC="${HOME}/.zshrc"

ensure_dir() {
  local dir="$1"
  if [ ! -d "$dir" ]; then
    mkdir -p "$dir"
  fi
}

ensure_line_in_file() {
  local file="$1"
  local line="$2"

  if [ ! -f "$file" ]; then
    touch "$file"
  fi

  if ! grep -qxF "$line" "$file" 2>/dev/null; then
    printf '%s\n' "$line" >>"$file"
  fi
}

fetch() {
  local src="$1"
  local dest="$2"

  echo "[Git-Shell-Helpers-Installer] Fetching $src -> $dest"
  curl -fsSL "$src" -o "$dest"
}

install_all() {
  echo "[Git-Shell-Helpers-Installer] Installing git shell helpers..."

  ensure_dir "$BIN_DIR"
  ensure_dir "$MAN_DIR"

  # Scripts
  fetch "$REPO_RAW_BASE/git-upload"     "$BIN_DIR/git-upload"
  fetch "$REPO_RAW_BASE/git-get"        "$BIN_DIR/git-get"
  fetch "$REPO_RAW_BASE/git-initialize" "$BIN_DIR/git-initialize"
  fetch "$REPO_RAW_BASE/git-fucked-the-push" "$BIN_DIR/git-fucked-the-push"
  fetch "$REPO_RAW_BASE/git-copilot-devops-audit" "$BIN_DIR/git-copilot-devops-audit"
  chmod +x "$BIN_DIR/git-upload" "$BIN_DIR/git-get" "$BIN_DIR/git-initialize" "$BIN_DIR/git-fucked-the-push" "$BIN_DIR/git-copilot-devops-audit"

  # Man pages (from repo's man/man1)
  fetch "$REPO_RAW_BASE/man/man1/git-upload.1"     "$MAN_DIR/git-upload.1"
  fetch "$REPO_RAW_BASE/man/man1/git-get.1"        "$MAN_DIR/git-get.1"
  fetch "$REPO_RAW_BASE/man/man1/git-initialize.1" "$MAN_DIR/git-initialize.1"
  fetch "$REPO_RAW_BASE/man/man1/git-fucked-the-push.1" "$MAN_DIR/git-fucked-the-push.1"
  fetch "$REPO_RAW_BASE/man/man1/git-copilot-devops-audit.1" "$MAN_DIR/git-copilot-devops-audit.1"

  # Ensure PATH and MANPATH are wired in ~/.zshrc (idempotent)
  ensure_line_in_file "$ZSHRC" 'export PATH="$HOME/bin:$PATH"'
  ensure_line_in_file "$ZSHRC" 'export MANPATH="$HOME/man:$MANPATH"'

  # Optionally help the user install gh and copilot-cli if missing.
  # We keep this macOS/Homebrew-centric and ask for confirmation.
  if ! command -v gh >/dev/null 2>&1 || ! command -v copilot >/dev/null 2>&1; then
    echo "[Git-Shell-Helpers-Installer] Detected missing dependencies for AI commits." >&2
    echo "  - GitHub CLI (gh) present:        $(command -v gh >/dev/null 2>&1 && echo yes || echo no)" >&2
    echo "  - GitHub Copilot CLI (copilot) present: $(command -v copilot >/dev/null 2>&1 && echo yes || echo no)" >&2

    if [[ "$OSTYPE" == darwin* ]] && command -v brew >/dev/null 2>&1; then
      printf "[Git-Shell-Helpers-Installer] Install missing tools via Homebrew now? [y/N]: " >&2
      read -r reply || reply=""
      if [[ "$reply" == "y" || "$reply" == "Y" ]]; then
        if ! command -v gh >/dev/null 2>&1; then
          echo "[Git-Shell-Helpers-Installer] Installing GitHub CLI (gh) via Homebrew..." >&2
          brew install gh || echo "[Git-Shell-Helpers-Installer] Failed to install gh; please install it manually." >&2
        fi
        if ! command -v copilot >/dev/null 2>&1; then
          echo "[Git-Shell-Helpers-Installer] Installing GitHub Copilot CLI (copilot) via Homebrew..." >&2
          brew install copilot-cli || echo "[Git-Shell-Helpers-Installer] Failed to install copilot-cli; please install it manually." >&2
        fi
      else
        echo "[Git-Shell-Helpers-Installer] To use AI commit messages, please install:" >&2
        echo "  - GitHub CLI: https://cli.github.com/" >&2
        echo "  - GitHub Copilot CLI: https://docs.github.com/en/copilot/concepts/agents/about-copilot-cli" >&2
      fi
      fi
  fi

  # Optional: install private audit agents and the slash command into standard user-level Copilot locations
  VSCODE_USER_DIR="$HOME/Library/Application Support/Code/User"
  if [ -d "$VSCODE_USER_DIR" ]; then
    echo ""
    printf '[Git-Shell-Helpers-Installer] Install private Copilot audit agents + /copilot-devops-audit globally in VS Code? [Y/n]: '
    read -r vscode_reply || vscode_reply=""
    if [[ -z "$vscode_reply" || "$vscode_reply" == "y" || "$vscode_reply" == "Y" ]]; then
      "${BIN_DIR}/git-copilot-devops-audit" --update-agent --force >/dev/null 2>&1 || true
      echo "[Git-Shell-Helpers-Installer] Installed DevOpsAudit agents and skills into ~/.copilot, and the prompt into the VS Code user profile."
      echo "[Git-Shell-Helpers-Installer] Reload VS Code window (Cmd+Shift+P → 'Developer: Reload Window') to activate."
    else
      echo "[Git-Shell-Helpers-Installer] Skipped VS Code global install. Run 'git copilot-devops-audit' in any repo to install later."
    fi
  fi

  echo "[Git-Shell-Helpers-Installer] Done. Open a new terminal or run:"
  echo "  source $ZSHRC"
  echo "Then you can use: git upload, git get, git initialize, git fucked-the-push,"
  echo "  git copilot-devops-audit, and view docs via git help <command>."
}

install_all
