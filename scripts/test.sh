#!/usr/bin/env bash

# scripts/test.sh
# Repo sanity checks intended to match what CI verifies.
#
# This script is also used by git-upload -ai to produce an authoritative
# Testing: line in commit messages.

set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

# Determine which shell to use for syntax checks. On systems without zsh
# (e.g. most Linux CI images), skip zsh-specific checks gracefully.
has_zsh=false
if command -v zsh >/dev/null 2>&1; then
	has_zsh=true
fi

declare -a checks
checks=(
	"bash -n git-upload"
	"bash -n git-checkpoint"
	"bash -n scripts/community-cache-submit.sh"
	"bash -n scripts/community-cache-pull.sh"
	"bash -n scripts/community-research-submit.sh"
	"bash ./scripts/test-git-upload-detect.sh"
	"node ./scripts/test-knowledge-rw.js"
	"node ./scripts/test-list-language-models.js"
	"node ./scripts/test-patch-vscode-argv.js"
	"node --check ./scripts/patch-vscode-argv.js"
	"node --check ./scripts/patch-vscode-runsubagent-model.js"
	"node ./scripts/test-resolve-repo-root.js"
	"bash ./scripts/test-node-coverage.sh"
	"node ./scripts/test-search-auto-scrape.js"
	"node ./scripts/test-chat-history-archive.js"
	"node ./scripts/test-worktree-manager.js"
	"node ./scripts/build-pages-search-site.js"
	"node ./scripts/test-session-memory.js"
	"bash ./scripts/build-dist.sh"
)

# Scripts that still require zsh — only check when zsh is available
if [ "$has_zsh" = true ]; then
	checks+=(
		"zsh -n git-get"
		"zsh -n git-initialize"
		"zsh -n git-resolve"
		"zsh -n git-fucked-the-push"
		"zsh -n git-copilot-quickstart"
		"zsh -n git-copilot-devops-audit"
		"zsh -n git-help-i-pushed-an-env"
		"zsh -n git-scan-for-leaked-envs"
		"zsh -n Git-Shell-Helpers-Installer.sh"
		"zsh -n install-git-shell-helpers"
	)
fi

if command -v pkgbuild >/dev/null 2>&1 && [ "$has_zsh" = true ]; then
	checks+=("zsh ./scripts/build-pkg.sh")
fi

total=${#checks[@]}
passed=0
failed=0

failures=()

for check in "${checks[@]}"; do
	echo "[test] run: $check" >&2
	if eval "$check" >/dev/null 2>&1; then
		passed=$((passed + 1))
	else
		failed=$((failed + 1))
		failures+=("$check")
	fi
done

if [ "$failed" -eq 0 ]; then
	echo "TEST_SUMMARY: pass ${passed}/${total}"
	exit 0
fi

echo "TEST_SUMMARY: fail ${failed}/${total}"
for f in "${failures[@]}"; do
	echo "TEST_FAIL: $f"
done

exit 1