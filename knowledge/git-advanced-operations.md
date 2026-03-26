# Git Advanced Operations — Bisect, Archaeology, and History Rewriting

## Bisect: Automated Bug Hunting

`git bisect` performs binary search through commit history to identify which commit introduced a bug. Instead of manually checking out commits one by one, bisect automates the search.

### Manual Bisect Workflow

```bash
git bisect start
git bisect bad HEAD          # Current commit is bad (has bug)
git bisect good v1.0         # Older commit is good (no bug)
```

Git checks out the midpoint commit and waits:

```bash
# Test the code
npm test  # or manual testing
# Result: commit abc123 is GOOD or BAD

git bisect good   # Mark as good, continue search
# or
git bisect bad    # Mark as bad, continue search
```

Git repeatedly bisects until it isolates the exact commit:

```
After 5-7 iterations (for ~100 commits):
git bisect result: abc1234 is the first bad commit
```

The identified commit is the one that introduced the bug. Inspect it:

```bash
git show abc1234
git log -1 abc1234
```

Resume normal work:

```bash
git bisect reset    # Return to original branch
```

### Automated Bisect with Scripts

For bugs that can be detected programmatically:

```bash
git bisect start
git bisect bad HEAD
git bisect good v1.0
git bisect run ./test-script.sh
```

The script runs on each bisect candidate. Exit code 0 = good, 1 = bad, 125 = skip (e.g., build failed).

**Example test script:**

```bash
#!/bin/bash
npm run build && npm test
exit $?  # Propagate test result
```

Bisect then fully automates, running the script and marking bisect candidates until convergence.

### Limitations and Edge Cases

- **Skipped commits**: If a commit won't compile or test fails for environmental reasons, mark it `git bisect skip`. Bisect continues with other path.
- **Merge commits**: `git bisect` can traverse merge commits. Use `git bisect --first-parent` to skip merge commits and follow main development line only.
- **Multiple bugs**: If bugs are on different commits, bisect finds one. Repeat for others.

## Git Blame and Log: Code Archaeology

### Blame: Line-by-Line History

`git blame` shows the last commit that touched each line of a file:

```bash
git blame src/main.py
```

Output:

```
abc1234 (Author Name 2025-01-15 10:23:45) line 1:  import sys
def5678 (Other Dev  2025-01-14 09:12:34) line 2:  import os
abc1234 (Author Name 2025-01-15 10:23:45) line 3:  
jkl3456 (Reviewer   2025-01-16 14:55:22) line 4:  def main():
```

**Flag variants:**

```bash
git blame -L 10,20 file.py      # Blame lines 10–20 only
git blame --date=short file.py  # Abbreviated dates
git blame -C file.py            # Track code moved/copied between files
git blame -M file.py            # Track code moved within file
```

**Use cases:**

- Find who last modified problematic code (for discussion)
- Understand the commit context (use `git show abc1234`)
- Identify when a line was added (useful for correlating with deployments)

**Limitation:** Blame shows the last modification, not the original introduction. A refactoring might mark a line as modified by a recent unrelated commit.

### Log with Filtering and Searching

`git log` offers powerful search and filtering:

```bash
# Commits touching specific file
git log -- src/main.py

# Commits modifying lines near N in file
git log -L 10,20:src/main.py

# Commits introducing/removing regex pattern
git log -S "old_function" --follow -- src/

# Commits with message matching pattern
git log --grep="bugfix" --oneline

# Commits by author
git log --author="Name"

# Commits in date range
git log --since="2025-01-01" --until="2025-02-01"

# Commits not on main (branches only)
git log main..feature
```

**Powerful combination:**

```bash
# Find all commits that mention "login" touching auth module
git log --grep="login" -- src/auth.py

# Show the diffs for those commits
git log -p --grep="login" -- src/auth.py
```

### Pickaxe: Find When a String Was Added/Removed

`git log -S` (pickaxe) finds commits that changed the count of a specific string:

```bash
git log -S "TODO: fix this" -- src/
```

Output: All commits where the number of occurrences of "TODO: fix this" changed (added or removed). Combined with `-p`, shows the exact diff:

```bash
git log -S "deprecated_function" -p -- src/utils.py
```

**Difference from grep:**

- `--grep` searches commit messages
- `-S` searches code content
- `-p` shows actual code changes

## Worktrees: Multiple Working Directories

Worktrees allow multiple simultaneous checkouts of the same repository without stashing or committing incomplete work.

### Creating and Using Worktrees

```bash
# Create a worktree for a branch
git worktree add ../feature-tree feature-branch
cd ../feature-tree
# You now have a separate checkout
```

List all worktrees:

```bash
git worktree list
```

Output:

```
/repo              (detached HEAD abc1234)
/repo-feature      feature-branch
/repo-hotfix       (detached HEAD def5678)
```

**Use case:** Working on main branch while needing to review a PR on another branch:

```bash
# In main checkout with uncommitted work
git worktree add ../pr-review origin/pull/42/head
cd ../pr-review
# Test PR without stashing main workarea
git show  # Inspect PR commits
npm test
cd ../repo
# Uncommitted work still in main checkout, untouched
```

### Removing Worktrees

```bash
git worktree remove ../pr-review
```

**Gotcha:** Deleting the directory without `git worktree remove` can corrupt the worktree list. Always use the command.

## Subtree vs Submodule: Vendoring Dependencies

Both approaches include external repositories within a project; they differ fundamentally.

### Subtree

Merges an external repository's history into a subdirectory of the current repo:

```bash
git subtree add --prefix=lib/external https://github.com/user/lib.git main
```

Result: External repo's commits are merged into main repo history under `lib/external/`. All files are now part of main repo.

**Advantages:**

- Subfoldered code is part of normal git operations (blame, log, commit)
- No separate `git submodule` commands
- Cloning automatically includes external code
- Easy for nested dependencies

**Disadvantages:**

- History is merged (can be complex)
- Updating external code is manual (`git subtree pull`)
- Large repos can bloat quickly

### Submodule

References an external repository at a specific commit, without merging history:

```bash
git submodule add https://github.com/user/lib.git lib/
git commit -m "Add lib submodule"
```

Result: `.gitmodules` file records the URL and path; `lib/` is a submodule (checked out at a pinned commit).

**Cloning with submodules:**

```bash
git clone --recurse-submodules https://github.com/user/repo.git
```

**Disadvantages:**

- Separate commands to update (`git submodule update --remote`)
- Easy to forget to initialize on clone (results in empty directories)
- Each developer must understand submodule workflow
- Dangling HEAD state inside submodule is a gotcha

**Advantages:**

- Keeps external repo's history separate
- External updates don't bloat main repo
- Explicit pinning prevents accidental upstream changes

**Rule of thumb:** Subtree for libraries you own or control entire history of; submodule for loosely-coupled dependencies.

## Sparse-Checkout: Partial Repository Clone

Sparse-checkout reduces the working tree to a subset of files, without downloading skipped objects when using partial clone.

### Basic Sparse-Checkout

```bash
git clone <repo>
cd repo
git sparse-checkout init --cone
git sparse-checkout set src/module-a docs
```

Result: Only `src/module-a/` and `docs/` are checked out locally. Other directories are not in the working tree (but still in `.git/objects/`).

Useful for monorepos where developers work on specific subprojects:

```bash
# Checkout only frontend and shared modules
git sparse-checkout set frontend shared/ configs/
```

### Combined with Partial Clone

```bash
git clone --filter=blob:none --sparse https://github.com/user/large-repo.git
cd large-repo
git sparse-checkout init --cone
git sparse-checkout set apps/web
```

This combination:

1. **Partial clone** (`--filter=blob:none`): Downloads commit/tree objects, fetches blobs on-demand
2. **Sparse-checkout** (`--sparse`): Checks out only specified paths

Result: Minimal initial download + targeted worktree. Blobs are fetched as you access them.

**Limitation:** Merging sparse checkouts can cause issues if merges touch ignored directories. Most teams handle this via CI (full checkout for merges).

## Filter-Repo: History Rewriting

`git filter-repo` is a modern replacement for `git filter-branch`. It rewrites the entire history, recomputing commit SHAs.

### Common Use Cases

**Move files to subdirectory:**

```bash
git filter-repo --to-subdirectory-filter lib
```

All files now live under `lib/`; commits are rewritten.

**Remove sensitive files:**

```bash
git filter-repo --invert-paths --items-to-remove secrets.txt passwords.json
```

All commits are rewritten to exclude these files entirely. Use with `git push --force` (coordinate with team).

**Preserve history, change author:**

```bash
git filter-repo --email-callback 'return email.replace("old.com", "new.com")'
```

Rewrites all commits, changing email domain.

### Safety and Aftermath

Rewriting history invalidates all refs:

```bash
git filter-repo --to-subdirectory-filter lib
# All branches, tags, SHAs are new
git push origin --force --all --tags  # Force-push rewritten history
```

**Consequences:**

- All developers must `git pull --rebase` to integrate (their branches now based on old history)
- Tags change (old refs become dangling)
- Requires team coordination (announce planned rewrite)

**Recovery if filter-repo went wrong:**

```bash
git reflog              # Find original HEAD
git reset --hard HEAD@{1}
git push origin --force --all  # Force-push original history back
```

## Cherry-Pick: Selective Commit Application

`git cherry-pick` applies a commit from one branch to another:

```bash
git cherry-pick abc1234              # Apply single commit
git cherry-pick abc..def             # Apply range (exclusive of abc)
git cherry-pick abc^..def            # Apply range (inclusive of abc)
```

**Conflict resolution:**

```bash
# Fix conflicts
git add .
git cherry-pick --continue
# Or abort:
git cherry-pick --abort
```

**Use cases:**

- Backporting bug fixes from main to release branch
- Applying hotfix to multiple branches
- Selectively merging from feature branches

**Gotcha:** Cherry-pick changes commit SHA (new parent), creating duplication if the commit is later merged normally. Use with discipline or rebase when possible.

---

_Sources: Pro Git v2 (https://git-scm.com/book/en/v2), git-scm.com documentation, Gun.io (bisect guide), Andrew Lock (filter-repo, autosquash), xargs.io (archaeology)_