#!/usr/bin/env bash
#
# test-gitcli.sh — black-box smoke test for the native Rust git-* CLIs.
#
# Exercises the busybox dispatch (argv[0] basename and the explicit `gitcli`
# subcommand) end-to-end against the built gsh-native binary, in a throwaway
# git repo. Deterministic, no network, no AI.

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NB="${REPO_DIR}/native/target/release/gsh-native"

if [ ! -x "$NB" ]; then
	NB="${REPO_DIR}/gsh-native"
fi
if [ ! -x "$NB" ]; then
	echo "GITCLI: skip (gsh-native not built; run 'gsh build')"
	exit 0
fi

fail() {
	echo "GITCLI_FAIL: $1" >&2
	exit 1
}

work="$(mktemp -d -t gsh-gitcli.XXXXXX)"
cleanup() { rm -rf "$work"; }
trap cleanup EXIT

cd "$work"
git init -q
git config user.email test@example.com
git config user.name test
echo "hello" >a.txt
git commit -qm init --allow-empty
git add a.txt && git commit -qm "add a"

# 1) git-resolve on a clean tree exits 0 (nothing to resolve, after a dirty file).
echo "stray" >stray.txt
"$NB" gitcli git-resolve >/dev/null 2>&1 || fail "git-resolve nonzero on dirty tree"

# 2) git-scan-for-leaked-envs finds a planted secret and exits 1.
printf 'AWS_KEY=AKIAIOSFODNN7EXAMPLE\n' >.env
git add .env
if "$NB" gitcli git-scan-for-leaked-envs >/dev/null 2>&1; then
	fail "git-scan-for-leaked-envs should exit 1 when a secret is present"
fi

# 3) Busybox argv[0] dispatch via a symlink behaves identically.
ln -sf "$NB" "$work/git-scan-for-leaked-envs"
if "$work/git-scan-for-leaked-envs" --json >/dev/null 2>&1; then
	fail "symlink dispatch should exit 1 on secret (json)"
fi

# 4) git-remerge rejects a nonexistent source branch (exit 1, no mutation).
if "$NB" gitcli git-remerge does-not-exist >/dev/null 2>&1; then
	fail "git-remerge should reject a nonexistent branch"
fi

# 5) git-initialize validates a missing remote URL (exit 1).
if "$NB" gitcli git-initialize >/dev/null 2>&1; then
	fail "git-initialize should require a remote URL"
fi

# 6) git-upload generates a deterministic commit message (no AI, no remote push).
echo "more" >>a.txt
out="$("$NB" gitcli git-upload "smoke message" 2>&1 || true)"
echo "$out" | grep -q "Using commit message: smoke message" \
	|| fail "git-upload did not use the provided message"
git log -1 --pretty=%s | grep -q "smoke message" \
	|| fail "git-upload did not create the commit"

# 7) git-checkpoint commits locally with a deterministic message (no AI, no push).
echo "checkpoint change" >c.txt
git add c.txt
"$NB" gitcli git-checkpoint >/dev/null 2>&1 || fail "git-checkpoint failed to commit"
git log -1 --pretty=%s | grep -q "checkpoint: update" \
	|| fail "git-checkpoint did not write a deterministic message"

# 8) git-checkpoint --status reports config without committing.
status_out="$("$NB" gitcli git-checkpoint --status 2>&1 || true)"
echo "$status_out" | grep -q "enabled:" \
	|| fail "git-checkpoint --status did not report config"

echo "GITCLI: pass"
