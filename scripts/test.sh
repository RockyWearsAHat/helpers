#!/usr/bin/env bash

# scripts/test.sh
# Repo sanity checks intended to match what CI verifies.
#
# This script is also used by git-upload -ai to produce an authoritative
# Testing: line in commit messages.

set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

declare -a checks
checks=(
	"bash -n git-upload"
	"bash -n git-get"
	"bash -n git-initialize"
	"bash -n git-checkpoint"
	"bash -n git-fucked-the-push"
	"bash -n git-remerge"
	"bash -n git-resolve"
	"bash -n git-copilot-quickstart"
	"bash -n git-copilot-devops-audit"
	"bash -n git-help-i-pushed-an-env"
	"bash -n git-scan-for-leaked-envs"
	"bash -n Git-Shell-Helpers-Installer.sh"
	"bash -n install-git-shell-helpers"
	"bash -n scripts/community-cache-submit.sh"
	"bash -n scripts/community-cache-pull.sh"
	"bash -n scripts/community-research-submit.sh"
	"bash -n scripts/build-pkg.sh"
	"bash -n scripts/pkg/postinstall"
	"bash -n scripts/pkg/core-scripts/postinstall"
	"bash -n scripts/pkg/mcp-scripts/postinstall"
	"bash -n scripts/pkg/audit-scripts/postinstall"
	"bash -n scripts/pkg/vscode-scripts/postinstall"
	"bash ./scripts/test-git-upload-detect.sh"
	"node ./scripts/test-knowledge-rw.js"
	"node ./scripts/test-list-language-models.js"
	"node ./scripts/test-pdf-extract.js"
	"node ./scripts/test-mcp-research.js"
	"node ./scripts/test-patch-vscode-argv.js"
	"node --check ./scripts/patch-vscode-argv.js"
	"node --check ./scripts/patch-vscode-runsubagent-model.js"
	"node ./scripts/test-resolve-repo-root.js"
	"bash ./scripts/test-node-coverage.sh"
	"node ./scripts/test-search-auto-scrape.js"
	"node ./scripts/test-chat-history-archive.js"
	"node ./scripts/test-chat-archive-mcp.js"
	"node ./scripts/test-install-health.js"
	"node ./scripts/test-worktree-manager.js"
	"node ./scripts/build-pages-search-site.js"
	"node ./scripts/test-session-memory.js"
	"bash ./scripts/build-dist.sh"
)

if command -v pkgbuild >/dev/null 2>&1; then
	checks+=("bash ./scripts/build-pkg.sh")
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