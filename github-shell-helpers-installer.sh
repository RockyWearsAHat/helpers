#!/bin/zsh

# github-shell-helpers-installer.sh
#
# Standalone installer script that fetches the latest versions of the
# helper commands and man pages from GitHub and installs them into:
#   - ~/bin
#   - ~/man/man1
# then wires PATH and MANPATH in ~/.zshrc.
#
# Usage (one line):
#   curl -fsSL \
#     https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/github-shell-helpers-installer.sh \
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

  echo "[github-shell-helpers-installer] Fetching $src -> $dest"
  curl -fsSL "$src" -o "$dest"
}

install_all() {
  echo "[github-shell-helpers-installer] Installing git shell helpers..."

  ensure_dir "$BIN_DIR"
  ensure_dir "$MAN_DIR"

  # Scripts
  fetch "$REPO_RAW_BASE/git-upload"    "$BIN_DIR/git-upload"
  fetch "$REPO_RAW_BASE/git-get"       "$BIN_DIR/git-get"
  fetch "$REPO_RAW_BASE/git-initialize" "$BIN_DIR/git-initialize"
  fetch "$REPO_RAW_BASE/git-fucked-the-push" "$BIN_DIR/git-fucked-the-push"
  chmod +x "$BIN_DIR/git-upload" "$BIN_DIR/git-get" "$BIN_DIR/git-initialize" "$BIN_DIR/git-fucked-the-push"

  # Man pages (from repo's man/man1)
  fetch "$REPO_RAW_BASE/man/man1/git-upload.1"    "$MAN_DIR/git-upload.1"
  fetch "$REPO_RAW_BASE/man/man1/git-get.1"       "$MAN_DIR/git-get.1"
  fetch "$REPO_RAW_BASE/man/man1/git-initialize.1" "$MAN_DIR/git-initialize.1"
  fetch "$REPO_RAW_BASE/man/man1/git-fucked-the-push.1" "$MAN_DIR/git-fucked-the-push.1"

  # Ensure PATH and MANPATH are wired in ~/.zshrc (idempotent)
  ensure_line_in_file "$ZSHRC" 'export PATH="$HOME/bin:$PATH"'
  ensure_line_in_file "$ZSHRC" 'export MANPATH="$HOME/man:$MANPATH"'

  echo "[github-shell-helpers-installer] Done. Open a new terminal or run:"
  echo "  source $ZSHRC"
  echo "Then you can use: git upload, git get, git initialize, git fucked-the-push, and view docs via git help <command>."
}

install_all
