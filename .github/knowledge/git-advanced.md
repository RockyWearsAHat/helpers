# Git Advanced — Beyond the Basics

## The Object Model
Git stores 4 object types, all content-addressed by SHA-1 (transitioning to SHA-256):
- **blob**: File contents (no filename — just raw bytes)
- **tree**: Directory listing (maps names → blobs/trees with mode bits)
- **commit**: Points to a tree + parent commit(s) + author/committer + message
- **tag**: Annotated tag pointing to a commit with metadata

Every ref (branch, tag, HEAD) is just a file containing a SHA.

```bash
# Inspect any object
git cat-file -t abc1234    # type
git cat-file -p abc1234    # pretty-print contents
git rev-parse HEAD         # resolve ref to SHA
```

## Rebase Strategies

### Interactive Rebase
```bash
git rebase -i HEAD~5       # Rewrite last 5 commits
git rebase -i main         # Rebase onto main
```

**Commands in `git rebase -i`:**
```
pick    — use commit as-is
reword  — use commit but edit message
edit    — pause for amending (add files, split commit)
squash  — meld into previous commit, combine messages
fixup   — meld into previous, discard this message
drop    — remove commit entirely
exec    — run shell command
```

### Rebase onto (transplant commits)
```bash
# Move commits from feature that branched off old-base onto new-base
git rebase --onto new-base old-base feature
```

### Autosquash Workflow
```bash
# While working, create fixup commits targeting earlier commits
git commit --fixup=abc1234
git commit --squash=abc1234

# Later, interactive rebase auto-arranges them
git rebase -i --autosquash main
```

### When NOT to Rebase
- **Published/shared branches**: Never rebase commits others have based work on
- **Merge commits you want to preserve**: Rebase linearizes history by default
- If the branch has been force-pushed and others have pulled — coordinate first

## Cherry-Pick

```bash
git cherry-pick abc1234              # Apply one commit
git cherry-pick abc..def             # Apply range (exclusive of abc)
git cherry-pick abc^..def            # Apply range (inclusive of abc)
git cherry-pick -n abc1234           # Stage changes without committing
git cherry-pick --no-commit a b c    # Stage multiple, commit manually
git cherry-pick -x abc1234           # Append "(cherry picked from ...)" to message
```

**Conflict resolution:**
```bash
# Fix conflicts, then:
git add .
git cherry-pick --continue
# Or abort:
git cherry-pick --abort
```

## Reflog — Your Safety Net

The reflog records every HEAD movement. It's local-only and expires (default 90 days).

```bash
git reflog                    # Show HEAD reflog
git reflog show feature       # Show reflog for a branch
git reflog --date=relative    # With timestamps

# Recover "lost" commits
git checkout HEAD@{3}         # Go back 3 HEAD positions
git branch recovered HEAD@{5} # Create branch from reflog entry
git reset --hard HEAD@{1}    # Undo the last operation

# Find when a branch was at a certain state
git log -g --grep="keyword" # Search reflog entries
```

**Reflog saves you from:**
- Accidental `git reset --hard`
- Bad rebase
- Deleted branches (`git branch -D` — the commits still exist until GC)
- Accidental `git commit --amend`

## Worktrees — Multiple Working Directories

```bash
# Create a new worktree for a branch (no stashing needed!)
git worktree add ../hotfix-tree hotfix-branch
git worktree add ../review-tree origin/pr-123

# List worktrees
git worktree list

# Remove when done
git worktree remove ../hotfix-tree
```

**Use cases:**
- Review a PR while your main worktree has uncommitted changes
- Run tests on one branch while developing on another
- Compare behavior between branches side-by-side

## Submodules vs Subtrees

### Submodules
```bash
# Add a submodule
git submodule add https://github.com/user/lib.git lib/

# Clone a repo with submodules
git clone --recurse-submodules repo-url

# Update submodules to latest
git submodule update --remote --merge

# Status
git submodule status
```
**Gotchas:** Submodules pin to a specific commit. `git pull` doesn't update them. Easy to get into detached HEAD state inside submodule.

### Subtrees (simpler alternative)
```bash
# Add a subtree
git subtree add --prefix=lib https://github.com/user/lib.git main --squash

# Pull updates
git subtree pull --prefix=lib https://github.com/user/lib.git main --squash

# Push changes back upstream
git subtree push --prefix=lib https://github.com/user/lib.git main
```
**Advantage:** No special clone steps. The code is just there. Contributors don't need to know it's a subtree.

## Advanced Log & Search

```bash
# Search commit messages
git log --grep="bug fix" --oneline

# Search diffs (pickaxe — find when a string was added/removed)
git log -S "function_name" --oneline
git log -S "TODO" -p                    # Show patches

# Regex search in diffs
git log -G "def\s+my_func" --oneline

# Who changed this line? (blame with ignore whitespace)
git blame -w file.py
git blame -L 10,20 file.py             # Lines 10-20

# Log with file rename tracking
git log --follow -- old-name.py

# Show merge-base (common ancestor)
git merge-base main feature

# Diff between branches (what feature adds vs main)
git diff main...feature                 # Three-dot = diff from merge-base
```

## Git Bisect — Binary Search for Bugs

```bash
git bisect start
git bisect bad                 # Current commit is broken
git bisect good v1.0.0         # This tag was working

# Git checks out a midpoint — test it, then:
git bisect good    # or
git bisect bad

# Repeat until Git identifies the first bad commit
# Then reset:
git bisect reset
```

### Automated Bisect
```bash
# Run a test script automatically at each step
git bisect start HEAD v1.0.0
git bisect run ./test-script.sh
# Exit 0 = good, exit 1 = bad, exit 125 = skip (can't test this commit)
```

## Stash Advanced Usage

```bash
git stash                          # Stash tracked changes
git stash -u                       # Include untracked files
git stash -a                       # Include ignored files too
git stash push -m "description"    # Named stash
git stash push -- path/to/file     # Stash specific files

git stash list                     # List all stashes
git stash show stash@{0} -p       # Show diff of a stash
git stash pop                      # Apply and remove
git stash apply                    # Apply but keep in stash list
git stash drop stash@{2}          # Remove specific stash
git stash branch new-branch        # Create branch from stash
```

## Hooks

### Client-Side Hooks (in `.git/hooks/` or via `core.hooksPath`)
```
pre-commit       Before commit (lint, format, run fast tests)
prepare-commit-msg  Before editor opens (add ticket number)
commit-msg       After message written (validate format)
pre-push         Before push (run full test suite)
post-checkout    After checkout (rebuild, install deps)
post-merge       After merge (install deps if lockfile changed)
```

### Useful Hook Patterns
```bash
#!/bin/sh
# .git/hooks/pre-commit — prevent committing to main
branch=$(git symbolic-ref HEAD 2>/dev/null)
if [ "$branch" = "refs/heads/main" ]; then
    echo "Direct commits to main are not allowed. Use a feature branch."
    exit 1
fi
```

```bash
#!/bin/sh
# .git/hooks/commit-msg — enforce conventional commits
if ! head -1 "$1" | grep -qE '^(feat|fix|docs|style|refactor|test|chore|ci|build|perf|revert)(\(.+\))?: .{1,}$'; then
    echo "Invalid commit message format. Use: type(scope): description"
    exit 1
fi
```

## Useful Git Config

```bash
# Auto-setup rebase for pulls
git config --global pull.rebase true

# Autosquash by default
git config --global rebase.autoSquash true

# Better diff algorithm
git config --global diff.algorithm histogram

# Sign commits
git config --global commit.gpgsign true
git config --global user.signingkey YOUR_KEY_ID

# Reuse Recorded Resolution — auto-resolve recurring conflicts
git config --global rerere.enabled true

# Default branch name
git config --global init.defaultBranch main

# Fast-forward only merges (explicit merge required otherwise)
git config --global merge.ff only

# Color moved lines differently in diff
git config --global diff.colorMoved zebra
```

## Dangerous Commands — Know Before You Run

| Command | What it does | Recovery |
|---------|-------------|----------|
| `git reset --hard HEAD~n` | Discards last n commits AND working changes | Reflog (commits); working changes are gone |
| `git push --force` | Overwrites remote history | Others may lose work; use `--force-with-lease` instead |
| `git clean -fd` | Deletes all untracked files/dirs | Gone forever (not in git) |
| `git checkout -- file` | Discards uncommitted changes to file | Gone forever |
| `git rebase` (on shared branch) | Rewrites commit SHAs | Others must `git pull --rebase` or re-clone |
| `git filter-branch` / `git filter-repo` | Rewrites entire history | Force-push required; all clones invalidated |

**Safe alternatives:**
- `git push --force-with-lease` — Refuses if remote has new commits you haven't seen
- `git revert` instead of `git reset` for shared history — creates an undo commit
- `git stash` before risky operations

---

*This guide covers advanced Git operations. For basics, see `git help tutorial`. For internals, see `git help gitcore-tutorial`.*
