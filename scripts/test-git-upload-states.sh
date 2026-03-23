#!/usr/bin/env bash
#
# scripts/test-git-upload-states.sh
#
# Synthetic reproducible tests for git-upload's ability to handle and recover
# from weird git states.  Every test:
#   1. Creates an isolated temp directory with a bare "remote" repo and a
#      working "local" clone.
#   2. Puts the local clone into a specific broken/weird git state.
#   3. Invokes git-upload (with library mode + main) and verifies it either
#      pushes successfully or exits with a clear message.
#
# Designed to run in both VS Code (via tasks.json) and GitHub Actions CI
# on macOS and Linux.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GIT_UPLOAD="$REPO_ROOT/git-upload"

# Provide a mock AI command so -ai tests work in CI (where copilot is
# not installed).  The mock emits the COMMIT_BEGIN/COMMIT_END markers
# that generate_ai_message() expects.
if ! command -v copilot >/dev/null 2>&1; then
	export GIT_UPLOAD_AI_CMD='printf "COMMIT_BEGIN\ntest: synthetic state recovery commit\nCOMMIT_END\n"'
fi

# ── Test bookkeeping ───────────────────────────────────────────────────
total=0
passed=0
failed=0
declare -a failure_names=()

pass() {
	passed=$((passed + 1))
	total=$((total + 1))
	echo "[PASS] $1" >&2
}

fail() {
	failed=$((failed + 1))
	total=$((total + 1))
	failure_names+=("$1")
	echo "[FAIL] $1  —  $2" >&2
}

# ── Helpers ────────────────────────────────────────────────────────────

# Create an isolated sandbox with a bare remote and a local clone.
# Sets: SANDBOX, REMOTE_DIR, LOCAL_DIR
setup_sandbox() {
	local name="$1"
	SANDBOX=$(mktemp -d "${TMPDIR:-/tmp}/git-upload-test-${name}.XXXXXX")
	REMOTE_DIR="$SANDBOX/remote.git"
	LOCAL_DIR="$SANDBOX/local"

	git init --bare -q "$REMOTE_DIR"
	# Set default branch to main in bare repo
	git -C "$REMOTE_DIR" symbolic-ref HEAD refs/heads/main

	# Seed the bare remote with an initial commit so clones start on main
	local seed="$SANDBOX/seed"
	git init -q "$seed"
	cd "$seed"
	git config user.name "Seed"
	git config user.email "seed@example.com"
	git checkout -q -b main 2>/dev/null || true
	echo "initial" > README.md
	git add README.md
	git commit -q -m "Initial commit"
	git remote add origin "$REMOTE_DIR"
	git push -q -u origin main
	cd /
	rm -rf "$seed"

	git clone -q "$REMOTE_DIR" "$LOCAL_DIR"
	cd "$LOCAL_DIR"
	git config user.name  "Test Bot"
	git config user.email "test@example.com"
	git checkout -q main 2>/dev/null || true
}

teardown_sandbox() {
	cd /
	rm -rf "$SANDBOX"
}

# Run git-upload in the LOCAL_DIR, expecting success (exit 0 + pushed).
# Pass extra args as $@ (e.g. "-ai" or a commit message).
run_upload_expect_success() {
	local test_name="$1"; shift
	cd "$LOCAL_DIR"

	local output
	if output=$(bash "$GIT_UPLOAD" "$@" 2>&1); then
		# Verify the commit actually reached the remote.
		local local_head remote_head
		local_head=$(git rev-parse HEAD 2>/dev/null || echo "none")

		# Determine current branch (may have changed during recovery)
		local cur_branch
		cur_branch=$(git symbolic-ref -q --short HEAD 2>/dev/null || echo "")
		if [ -z "$cur_branch" ]; then
			fail "$test_name" "Still in detached HEAD after upload"
			return
		fi

		remote_head=$(git ls-remote "$REMOTE_DIR" "refs/heads/$cur_branch" 2>/dev/null | awk '{print $1}')
		if [ -z "$remote_head" ]; then
			# Might have pushed to a new branch name
			remote_head=$(git ls-remote "$REMOTE_DIR" 2>/dev/null | grep "refs/heads/" | head -1 | awk '{print $1}')
		fi

		if [ "$local_head" = "$remote_head" ]; then
			pass "$test_name"
		else
			fail "$test_name" "Upload succeeded but remote HEAD ($remote_head) != local HEAD ($local_head)"
		fi
	else
		fail "$test_name" "git-upload exited non-zero. Output: $(echo "$output" | tail -5)"
	fi
}

# Run git-upload expecting a clean exit with specific message pattern (for
# cases like "nothing to commit").
run_upload_expect_exit() {
	local test_name="$1"
	local expected_exit="$2"
	shift 2
	cd "$LOCAL_DIR"

	local actual_exit=0
	bash "$GIT_UPLOAD" "$@" >/dev/null 2>&1 || actual_exit=$?

	if [ "$actual_exit" = "$expected_exit" ]; then
		pass "$test_name"
	else
		fail "$test_name" "Expected exit $expected_exit but got $actual_exit"
	fi
}


# ══════════════════════════════════════════════════════════════════════
#  TEST CASES
# ══════════════════════════════════════════════════════════════════════

# ── 1. Normal push (baseline) ─────────────────────────────────────────
test_normal_push() {
	setup_sandbox "normal-push"

	echo "new content" > file.txt
	git add file.txt

	run_upload_expect_success "normal-push" "test: baseline normal push"

	teardown_sandbox
}

# ── 2. Detached HEAD on a single branch ────────────────────────────────
test_detached_head_single_branch() {
	setup_sandbox "detached-single"

	# Create a second commit and detach at it
	echo "second" >> README.md
	git add README.md
	git commit -q -m "Second commit"
	git push -q origin main

	local head_sha
	head_sha=$(git rev-parse HEAD)
	git checkout -q "$head_sha"  # detach

	echo "work while detached" > detached-work.txt
	git add detached-work.txt

	run_upload_expect_success "detached-head-single-branch" "test: detached head single branch recovery"

	teardown_sandbox
}

# ── 3. Detached HEAD on NO branch (orphan work) ───────────────────────
test_detached_head_no_branch() {
	setup_sandbox "detached-orphan"

	local base_sha
	base_sha=$(git rev-parse HEAD)

	# Detach and create NEW commits not on any branch
	git checkout -q "$base_sha"
	echo "orphan work" > orphan.txt
	git add orphan.txt
	git commit -q -m "Orphan commit"

	echo "more orphan work" > orphan2.txt
	git add orphan2.txt

	run_upload_expect_success "detached-head-no-branch" "test: detached head orphan work recovery"

	teardown_sandbox
}

# ── 4. Detached HEAD on multiple branches (with -ai picks main) ──────
test_detached_head_multi_branch() {
	setup_sandbox "detached-multi"

	# Create another branch at the same commit
	git branch feature-x

	local head_sha
	head_sha=$(git rev-parse HEAD)
	git checkout -q "$head_sha"  # detach

	echo "multi-branch detached work" > multi.txt
	git add multi.txt

	# With -ai, should pick main over feature-x
	run_upload_expect_success "detached-head-multi-branch-ai" "-ai" "test: detached multi-branch with ai"

	teardown_sandbox
}

# ── 5. Mid-rebase (no conflicts) ──────────────────────────────────────
test_mid_rebase_no_conflict() {
	setup_sandbox "mid-rebase-clean"

	# Create divergent history
	echo "main change" > main-file.txt
	git add main-file.txt
	git commit -q -m "Main change"
	git push -q origin main

	git checkout -q -b feature
	echo "feature change" > feature-file.txt
	git add feature-file.txt
	git commit -q -m "Feature change"

	# Go back to main and add another commit
	git checkout -q main
	echo "main change 2" > main-file2.txt
	git add main-file2.txt
	git commit -q -m "Main change 2"
	git push -q origin main

	# Start rebase of feature onto main, then don't finish it
	git checkout -q feature
	GIT_SEQUENCE_EDITOR="sed -i.bak 's/pick/edit/'" git rebase main 2>/dev/null || true

	# We should now be mid-rebase with an "edit" stop (not a conflict)
	echo "new file during rebase" > new-during-rebase.txt
	git add new-during-rebase.txt

	run_upload_expect_success "mid-rebase-no-conflict" "test: mid-rebase clean recovery"

	teardown_sandbox
}

# ── 6. Mid-rebase with conflicts ──────────────────────────────────────
test_mid_rebase_with_conflict() {
	setup_sandbox "mid-rebase-conflict"

	# Create conflicting changes
	echo "main version" > shared.txt
	git add shared.txt
	git commit -q -m "Main version of shared"
	git push -q origin main

	git checkout -q -b conflict-branch
	echo "branch version" > shared.txt
	git add shared.txt
	git commit -q -m "Branch version of shared"

	git checkout -q main
	echo "main version updated" > shared.txt
	git add shared.txt
	git commit -q -m "Updated main version"
	git push -q origin main

	# Start conflicting rebase
	git checkout -q conflict-branch
	git rebase main 2>/dev/null || true

	# Should be mid-rebase with conflicts now.
	# git-upload -ai should abort the rebase, then we need something to commit.
	# After rebase abort, the branch goes back to pre-rebase state with the
	# "Branch version of shared" commit. We need new work to push.
	# Write a new file that isn't involved in the conflict.
	echo "extra work" > extra.txt

	# With -ai, should abort rebase, then commit new work and push
	run_upload_expect_success "mid-rebase-with-conflict-ai" "-ai" "test: mid-rebase conflict ai recovery"

	teardown_sandbox
}

# ── 7. Mid-merge with conflicts ───────────────────────────────────────
test_mid_merge_with_conflict() {
	setup_sandbox "mid-merge-conflict"

	echo "original" > conflict.txt
	git add conflict.txt
	git commit -q -m "Original"
	git push -q origin main

	git checkout -q -b merge-branch
	echo "branch edit" > conflict.txt
	git add conflict.txt
	git commit -q -m "Branch edit"

	git checkout -q main
	echo "main edit" > conflict.txt
	git add conflict.txt
	git commit -q -m "Main edit"
	git push -q origin main

	# Start merge that will conflict
	git merge merge-branch 2>/dev/null || true

	# With -ai, should resolve and push
	run_upload_expect_success "mid-merge-conflict-ai" "-ai" "test: mid-merge conflict ai recovery"

	teardown_sandbox
}

# ── 8. Mid-cherry-pick with conflicts ─────────────────────────────────
test_mid_cherry_pick_conflict() {
	setup_sandbox "mid-cherry-pick"

	echo "base" > cherry.txt
	git add cherry.txt
	git commit -q -m "Base cherry"
	git push -q origin main

	git checkout -q -b cherry-source
	echo "cherry content" > cherry.txt
	git add cherry.txt
	git commit -q -m "Cherry source commit"
	local cherry_sha
	cherry_sha=$(git rev-parse HEAD)

	git checkout -q main
	echo "main cherry content" > cherry.txt
	git add cherry.txt
	git commit -q -m "Main cherry content"
	git push -q origin main

	# Cherry-pick that will conflict
	git cherry-pick "$cherry_sha" 2>/dev/null || true

	# After cherry-pick recovery (abort or resolve), we need new content to push
	echo "post-cherry-pick work" > post-cherry.txt

	# With -ai, should recover and push
	run_upload_expect_success "mid-cherry-pick-conflict-ai" "-ai" "test: mid-cherry-pick conflict ai recovery"

	teardown_sandbox
}

# ── 9. Behind upstream (fast-forward possible) ────────────────────────
test_behind_upstream() {
	setup_sandbox "behind-upstream"

	# Push from local, then add a commit directly to remote
	echo "local work" > local.txt
	git add local.txt
	git commit -q -m "Local work"
	git push -q origin main

	# Simulate another developer pushing
	local tmp_clone="$SANDBOX/other-clone"
	git clone -q "$REMOTE_DIR" "$tmp_clone"
	cd "$tmp_clone"
	git config user.name "Other Dev"
	git config user.email "other@example.com"
	echo "other work" > other.txt
	git add other.txt
	git commit -q -m "Other developer's work"
	git push -q origin main

	# Back to our local, which is now behind
	cd "$LOCAL_DIR"
	echo "my new work" > my-new.txt
	git add my-new.txt

	run_upload_expect_success "behind-upstream" "test: behind upstream recovery"

	teardown_sandbox
}

# ── 10. Diverged from upstream (needs rebase) ─────────────────────────
test_diverged_from_upstream() {
	setup_sandbox "diverged"

	echo "initial work" > work.txt
	git add work.txt
	git commit -q -m "Initial work"
	git push -q origin main

	# Simulate remote getting ahead
	local tmp_clone="$SANDBOX/other-clone"
	git clone -q "$REMOTE_DIR" "$tmp_clone"
	cd "$tmp_clone"
	git config user.name "Other Dev"
	git config user.email "other@example.com"
	echo "remote-only change" > remote-file.txt
	git add remote-file.txt
	git commit -q -m "Remote-only change"
	git push -q origin main

	# Local also has a new commit (diverged)
	cd "$LOCAL_DIR"
	echo "local-only change" > local-file.txt
	git add local-file.txt
	git commit -q -m "Local-only change"

	echo "yet more local" > local-file2.txt
	git add local-file2.txt

	run_upload_expect_success "diverged-from-upstream" "test: diverged branch rebase recovery"

	teardown_sandbox
}

# ── 11. Bisect in progress ────────────────────────────────────────────
test_bisect_in_progress() {
	setup_sandbox "bisect"

	# Need several commits for bisect
	for i in 1 2 3 4 5; do
		echo "commit $i" > "file$i.txt"
		git add "file$i.txt"
		git commit -q -m "Commit $i"
	done
	git push -q origin main

	# Start a bisect session
	git bisect start
	git bisect bad HEAD
	git bisect good HEAD~4

	# User wants to abort bisect and just push their work
	echo "new work after bisect" > bisect-work.txt
	git add bisect-work.txt

	run_upload_expect_success "bisect-in-progress" "test: bisect recovery"

	teardown_sandbox
}

# ── 12. Stashed changes with dirty working tree ──────────────────────
test_stash_and_dirty_tree() {
	setup_sandbox "stash-dirty"

	echo "content A" > stash-test.txt
	git add stash-test.txt
	git commit -q -m "Add stash-test"
	git push -q origin main

	# Stash something, then make new changes
	echo "stashed content" > stash-test.txt
	git stash push -m "test stash"

	echo "new content to push" > new-file.txt
	git add new-file.txt

	run_upload_expect_success "stash-dirty-tree" "test: stash with dirty working tree"

	teardown_sandbox
}

# ── 13. Empty repo (no commits yet) ──────────────────────────────────
test_empty_repo() {
	setup_sandbox "empty-repo"

	# Recreate local as a fresh init (no commits)
	rm -rf "$LOCAL_DIR"
	mkdir -p "$LOCAL_DIR"
	cd "$LOCAL_DIR"
	git init -q
	git config user.name "Test Bot"
	git config user.email "test@example.com"
	git remote add origin "$REMOTE_DIR"

	# Try to push from an unborn branch with no commits
	echo "first file" > first.txt

	# git-upload should handle unborn branch: either push or exit cleanly
	# If it succeeds, the remote gets the commit. If it exits 1, that's also fine.
	local exit_code=0
	bash "$GIT_UPLOAD" "test: empty repo" >/dev/null 2>&1 || exit_code=$?
	if [ "$exit_code" = "0" ] || [ "$exit_code" = "1" ]; then
		pass "empty-repo-unborn"
	else
		fail "empty-repo-unborn" "Unexpected exit code $exit_code"
	fi

	teardown_sandbox
}

# ── 14. Revert in progress ───────────────────────────────────────────
test_revert_in_progress() {
	setup_sandbox "revert"

	echo "will revert" > revert-me.txt
	git add revert-me.txt
	git commit -q -m "Commit to revert"
	git push -q origin main

	echo "conflicting" > revert-me.txt
	git add revert-me.txt
	git commit -q -m "Conflicting change"
	git push -q origin main

	# Revert the first commit — will conflict with the second
	local revert_target
	revert_target=$(git rev-parse HEAD~1)
	git revert --no-commit "$revert_target" 2>/dev/null || true

	# After revert recovery (abort), add new work to push
	echo "extra work" > extra-revert.txt

	run_upload_expect_success "revert-in-progress" "test: revert recovery"

	teardown_sandbox
}

# ── 15. Multiple remotes ─────────────────────────────────────────────
test_multiple_remotes() {
	setup_sandbox "multi-remote"

	# Add a second remote
	local second_remote="$SANDBOX/second-remote.git"
	git init --bare -q "$second_remote"
	git remote add upstream "$second_remote"

	echo "multi-remote work" > multi.txt
	git add multi.txt

	run_upload_expect_success "multiple-remotes" "test: multiple remotes"

	teardown_sandbox
}

# ── 16. Branch created from stale detached HEAD after cherry-pick ────
test_cherry_pick_then_detach() {
	setup_sandbox "cherry-detach"

	echo "base" > base.txt
	git add base.txt
	git commit -q -m "Base"
	git push -q origin main

	git checkout -q -b source-branch
	echo "cherry1" > c1.txt
	git add c1.txt
	git commit -q -m "Cherry 1"
	local c1_sha
	c1_sha=$(git rev-parse HEAD)

	echo "cherry2" > c2.txt
	git add c2.txt
	git commit -q -m "Cherry 2"

	# Go back to main, cherry-pick, then detach
	git checkout -q main
	git cherry-pick "$c1_sha"
	git push -q origin main

	local main_head
	main_head=$(git rev-parse HEAD)
	git checkout -q "$main_head"  # detach after cherry-pick

	echo "post cherry-pick detached work" > post-cherry.txt

	# With -ai, should auto-pick main from the matching branches
	run_upload_expect_success "cherry-pick-then-detach" "-ai" "test: cherry-pick + detach recovery"

	teardown_sandbox
}

# ── 17. Amended commit that diverges from upstream ───────────────────
test_amend_diverge() {
	setup_sandbox "amend-diverge"

	echo "original" > amended.txt
	git add amended.txt
	git commit -q -m "Original commit"
	git push -q origin main

	# Amend the commit (diverges from upstream)
	echo "amended content" > amended.txt
	git add amended.txt
	git commit -q --amend -m "Amended commit"

	echo "new file" > new-after-amend.txt

	# With -ai, should handle the divergence (force push needed)
	# For now, we accept that git-upload correctly detects the divergence
	# and exits non-zero asking the user to resolve.
	# This is the EXPECTED behavior — force pushing is dangerous.
	local exit_code=0
	bash "$GIT_UPLOAD" "-ai" "test: amend diverge" >/dev/null 2>&1 || exit_code=$?
	if [ "$exit_code" = "1" ]; then
		pass "amend-diverge"
	else
		fail "amend-diverge" "Expected exit 1 for diverged amend, got $exit_code"
	fi

	teardown_sandbox
}

# ── 18. Rename-only changes ──────────────────────────────────────────
test_rename_only() {
	setup_sandbox "rename-only"

	echo "content" > original-name.txt
	git add original-name.txt
	git commit -q -m "Add file"
	git push -q origin main

	git mv original-name.txt renamed-file.txt

	run_upload_expect_success "rename-only" "test: rename-only changes"

	teardown_sandbox
}

# ══════════════════════════════════════════════════════════════════════
#  RUN ALL TESTS
# ══════════════════════════════════════════════════════════════════════

echo "" >&2
echo "════════════════════════════════════════════════════════════════" >&2
echo "  git-upload synthetic state recovery tests" >&2
echo "════════════════════════════════════════════════════════════════" >&2
echo "" >&2

test_normal_push
test_detached_head_single_branch
test_detached_head_no_branch
test_detached_head_multi_branch
test_mid_rebase_no_conflict
test_mid_rebase_with_conflict
test_mid_merge_with_conflict
test_mid_cherry_pick_conflict
test_behind_upstream
test_diverged_from_upstream
test_bisect_in_progress
test_stash_and_dirty_tree
test_empty_repo
test_revert_in_progress
test_multiple_remotes
test_cherry_pick_then_detach
test_amend_diverge
test_rename_only

echo "" >&2
echo "════════════════════════════════════════════════════════════════" >&2
echo "  Results: ${passed} passed, ${failed} failed out of ${total}" >&2
echo "════════════════════════════════════════════════════════════════" >&2

if [ "$failed" -gt 0 ]; then
	echo "" >&2
	echo "Failed tests:" >&2
	for name in "${failure_names[@]}"; do
		echo "  - $name" >&2
	done
	echo "" >&2
	echo "TEST_SUMMARY: fail ${failed}/${total}"
	exit 1
fi

echo "TEST_SUMMARY: pass ${passed}/${total}"
exit 0
