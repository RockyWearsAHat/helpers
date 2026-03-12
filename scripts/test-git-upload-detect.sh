#!/bin/zsh
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"

run_case() {
	local name="$1"
	local pkg_json="$2"

	local tmp
	tmp=$(mktemp -d -t "git-upload-detect-${name}.XXXXXX")
	trap 'rm -rf "$tmp"' EXIT

	cd "$tmp"
	git init -q
	printf '%s\n' "$pkg_json" > package.json

	# Source git-upload as a library so we can call compute_testing_status.
	export GIT_UPLOAD_LIBRARY_ONLY=1
	source "$repo_root/git-upload" >/dev/null

	local got
	got=$(compute_testing_status)
	if [ "$got" != "Testing: not configured" ]; then
		echo "[test] expected 'Testing: not configured' but got: $got" >&2
		return 1
	fi

	rm -rf "$tmp"
	trap - EXIT
}

run_guidance_case() {
	local tmp
	tmp=$(mktemp -d -t "git-upload-guidance.XXXXXX")
	trap 'rm -rf "$tmp"' EXIT

	cd "$tmp"
	git init -q
	mkdir -p .github
	cat > .github/COMMIT_GUIDELINES.md <<'EOF'
# Commit rules

- Mention git-upload explicitly.
EOF
	cat > AGENTS.md <<'EOF'
# Agent notes

- Prefer direct, specific commit subjects.
EOF
	cat > README.md <<'EOF'
# Example repo

This repo uses git upload for staged commits.
EOF

	export GIT_UPLOAD_LIBRARY_ONLY=1
	source "$repo_root/git-upload" >/dev/null

	local got
	got=$(collect_repo_ai_guidance "$tmp")

	if [[ "$got" != *"Project commit guidelines (.github/COMMIT_GUIDELINES.md):"* ]]; then
		echo "[test] expected commit guidelines block in repo guidance output" >&2
		return 1
	fi

	if [[ "$got" != *"Repository agent instructions (AGENTS.md):"* ]]; then
		echo "[test] expected AGENTS.md block in repo guidance output" >&2
		return 1
	fi

	if [[ "$got" != *"Repository overview (README.md):"* ]]; then
		echo "[test] expected README.md block in repo guidance output" >&2
		return 1
	fi

	rm -rf "$tmp"
	trap - EXIT
}

run_case \
	"npm-no-test-script" \
	'{"name":"x","version":"1.0.0"}'

run_case \
	"npm-placeholder-test" \
	'{"name":"x","version":"1.0.0","scripts":{"test":"echo \"Error: no test specified\" && exit 1"}}'

run_guidance_case
