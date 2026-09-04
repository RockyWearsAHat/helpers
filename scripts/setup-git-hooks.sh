#!/bin/bash
# Setup git hooks for the project

HOOKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/git-hooks"
GIT_HOOKS_DIR="$(git rev-parse --git-dir)/hooks"

if [ ! -d "$GIT_HOOKS_DIR" ]; then
  mkdir -p "$GIT_HOOKS_DIR"
fi

# Install pre-commit hook
if [ -f "$HOOKS_DIR/pre-commit" ]; then
  cp "$HOOKS_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit"
  chmod +x "$GIT_HOOKS_DIR/pre-commit"
  echo "[setup-git-hooks] installed pre-commit hook"
fi
