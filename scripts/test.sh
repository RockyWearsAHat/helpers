#!/bin/zsh

# scripts/test.sh
# Repo sanity checks intended to match what CI verifies.
#
# This script is also used by git-upload -ai to produce an authoritative
# Testing: line in commit messages.

set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

typeset -a checks
checks=(
	"zsh -n git-upload"
	"zsh -n git-get"
	"zsh -n git-initialize"
	"zsh -n git-resolve"
	"zsh -n git-fucked-the-push"
	"zsh -n git-copilot-quickstart"
	"zsh -n git-copilot-devops-audit"
	"bash -n scripts/community-cache-submit.sh"
	"bash -n scripts/community-cache-pull.sh"
	"zsh -n git-help-i-pushed-an-env"
	"zsh -n git-scan-for-leaked-envs"
	"zsh -n Git-Shell-Helpers-Installer.sh"
	"zsh -n install-git-shell-helpers"
	"zsh ./scripts/test-git-upload-detect.sh"
	"bash ./scripts/build-dist.sh"
)

if command -v pkgbuild >/dev/null 2>&1; then
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