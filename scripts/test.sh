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
	# git-upload, git-get, git-initialize, git-fucked-the-push, git-remerge,
	# git-resolve, git-scan-for-leaked-envs, git-checkpoint, git-help-i-pushed-an-env
	# are native Rust (native/src/gitcli/), exercised by cargo + the gitcli smoke test.
	"bash -n git-copilot-quickstart"
	"bash -n Helpers-Installer.sh"
	"bash -n install-helpers"
	"bash -n scripts/community-cache-submit.sh"
	"bash -n scripts/community-cache-pull.sh"
	"bash -n scripts/community-research-submit.sh"
	"bash -n scripts/build-native-binaries.sh"
	"bash -n scripts/verify-install.sh"
	"bash -n scripts/build-pkg.sh"
	"bash -n scripts/pkg/postinstall"
	"bash -n scripts/pkg/core-scripts/postinstall"
	"bash -n scripts/pkg/audit-scripts/postinstall"
	"bash -n scripts/pkg/vscode-scripts/postinstall"
	"bash ./scripts/test-gitcli.sh"
	"node ./scripts/test-patch-vscode-argv.js"
	"node --check ./scripts/patch-vscode-argv.js"
	"node --check ./scripts/patch-vscode-runsubagent-model.js"
	"node ./scripts/build-pages-search-site.js"
	"node ./scripts/test-project-index.js"
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
	if output="$(eval "$check" 2>&1)"; then
		passed=$((passed + 1))
	else
		failed=$((failed + 1))
		failures+=("$check")
		# Surface why it failed (the suite otherwise hides output), so CI logs
		# show the root cause instead of just the failing command name.
		echo "[test] FAILED: $check" >&2
		printf '%s\n' "$output" | tail -25 | sed 's/^/[test]   /' >&2
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
