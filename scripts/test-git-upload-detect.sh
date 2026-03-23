#!/usr/bin/env bash
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

run_release_notes_case() {
	local tmp
	tmp=$(mktemp -d -t "git-upload-release-notes.XXXXXX")
	trap 'rm -rf "$tmp"' EXIT

	cd "$tmp"
	git init -q
	git config user.name "Test User"
	git config user.email "test@example.com"
	mkdir -p release-notes
	printf '0.1.0\n' > VERSION
	cat > release-notes/v0.1.0.md <<'EOF'
# Git Shell Helpers v0.1.0

## Highlights

- Initial release notes.
EOF
	git add VERSION release-notes/v0.1.0.md
	git commit -qm "Initial release"
	git tag v0.1.0

	printf '0.1.1\n' > VERSION
	printf 'new behavior\n' > CHANGELOG.txt
	git add VERSION CHANGELOG.txt

	export GIT_UPLOAD_LIBRARY_ONLY=1
	source "$repo_root/git-upload" >/dev/null
	use_ai=true
	GIT_UPLOAD_AI_CMD='printf "%s\n" NOTES_BEGIN "- Auto-create missing release notes" "- Reuse repo guidance during upload" NOTES_END'

	if ! ensure_version_bump_release_notes "$tmp"; then
		echo "[test] expected version-bump release notes generation to succeed" >&2
		return 1
	fi

	if [ ! -f release-notes/v0.1.1.md ]; then
		echo "[test] expected release-notes/v0.1.1.md to be created" >&2
		return 1
	fi

	if ! grep -q "Auto-create missing release notes" release-notes/v0.1.1.md; then
		echo "[test] expected generated highlights in release notes file" >&2
		return 1
	fi

	if ! git diff --cached --name-only | grep -qx 'release-notes/v0.1.1.md'; then
		echo "[test] expected generated release notes to be staged" >&2
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
run_release_notes_case
