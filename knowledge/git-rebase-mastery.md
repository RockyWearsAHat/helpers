# Git Rebase Mastery — Interactive Rebase, Autosquash, and Recovery

## Interactive Rebase: Editing History

Interactive rebase rewrites commits as they are replayed onto a new base. This enables amending messages, squashing, reordering, or dropping commits—all before pushing.

### Basic Usage

```bash
git rebase -i HEAD~5       # Edit last 5 commits
git rebase -i main         # Rebase current branch onto main
git rebase -i --root       # Edit all commits (from root)
```

Git opens an editor with a todo list:

```
pick abc1234 Add user auth
pick def5678 Fix login bug
pick ghi9012 Update docs
pick jkl3456 Add tests
pick mno7890 Refactor auth module

# Commands:
# p, pick = use commit
# r, reword = use commit, but edit message
# e, edit = use commit, but stop for amending
# s, squash = meld into previous, keep both messages
# f, fixup = meld, discard this commit's message
# d, drop = remove commit entirely
# x, exec = run shell command
```

**Reordering:** Change the order of lines to reorder commits:

```
pick jkl3456 Add tests  # (moved up)
pick abc1234 Add user auth
pick def5678 Fix login bug
pick ghi9012 Update docs
pick mno7890 Refactor auth module
```

### Operations

**reword**: Edit commit message interactively

```
reword abc1234 Add user auth
```

Git stops with an editor for the message. Re-save and exit.

**edit**: Stop rebase to amend the commit (add/remove files, modify content)

```
edit def5678 Fix login bug
```

Git stops. You can:

```bash
# Modify files
git add/rm/reset
git commit --amend  # Modify the current commit
git rebase --continue
```

**squash**: Combine commit into the previous one, keeping both messages

```
pick abc1234 Add user auth
squash def5678 Fix login bug
```

Git combines both messages (interactive editor), allowing you to edit the result.

**fixup**: Like squash but discard the current commit's message (keep only the previous)

```
pick abc1234 Add user auth
fixup def5678 Fix login bug (message discarded)
```

**drop**: Completely remove the commit

```
drop ghi9012 Update docs  # This commit vanishes
```

**exec**: Run a shell command between commits (useful for validating history)

```
pick abc1234 Add user auth
exec npm test
pick def5678 Fix login bug
exec npm test
```

If any command fails, rebase stops. Fix the issue, then `git rebase --continue`.

### Abort and Recovery

If rebase goes wrong:

```bash
git rebase --abort      # Cancel rebase, return to original state
git rebase --continue   # After resolving conflicts, continue
```

The original commits are preserved in the reflog:

```bash
git reflog              # Find the original HEAD before rebase
git reset --hard HEAD@{1}  # Return to pre-rebase state
```

## Autosquash Workflow

The autosquash feature automatically arranges fixup commits during rebase, streamlining code review workflows.

### Creating Fixup Commits

While coding, create commits targeting specific earlier commits:

```bash
# During a PR review, you notice the auth module has a bug
git commit --fixup=abc1234  # Creates: "fixup! Add user auth"
git commit --squash=abc1234 # Creates: "squash! Add user auth"
```

These follow a convention: the commit message starts with `fixup! [original message]` or `squash! [original message]`.

### Autosquashing

Later, when rebasing:

```bash
git rebase -i --autosquash main
```

Git automatically arranges fixup/squash commits next to their targets and marks them for squashing:

```
pick abc1234 Add user auth
fixup def5678 fixup! Add user auth   (auto-arranged)
fixup ghi9012 fixup! Add user auth   (auto-arranged)
pick jkl3456 Fix login bug
squash mno7890 squash! Fix login bug (auto-arranged)
```

No manual reordering needed. Just save and exit.

### Workflow Integration

**Typical PR workflow with autosquash:**

1. Write feature commits
2. Open PR for review
3. Reviewers request changes
4. Make additional commits with `--fixup` targeting specific commits
5. Before merge, `git rebase -i --autosquash main` to clean up
6. Force-push to PR (history rewritten locally, not shared yet)
7. Merge squashed/fixed history to main

This keeps the main branch clean (each PR appears as 1-3 logical commits) while preserving review history in the PR comments.

## Rebase vs. Merge: Tradeoffs

### Merge Preserves History

A merge commit records the integration:

```
main: ────────────────────M (merge commit)
          ↙              ↗
feature: ────────────────
```

**merge.log:**

```
commit M: Merge branch 'feature' into 'main'
  Parents: previous-main, feature-tip
```

**Advantages:**

- Preserves exact integration point
- Easy to understand "when did this branch integrate?"
- Supports reverting entire branches (revert -m)
- Clear history for audits

**Disadvantages:**

- Creates merge commits (visual clutter)
- Non-linear history
- Difficult to bisect or understand individual changes

### Rebase Linearizes

Replaying commits on top of main:

```
main: ────────────M
           ↓↓↓ (rebased commits)
feature: ────────────────
```

**log output:**

```
commit A: Original feature commit 1
commit B: Original feature commit 2
commit C: Original feature commit 3
commit M: Merge to main (fast-forward)
```

Linear, clean history—each commit is visible.

**Advantages:**

- Clean, linear history
- Easy to understand progression
- `git bisect` works naturally (half-cull commits)
- Simpler `git log`

**Disadvantages:**

- Rewrites history (commits get new SHAs)
- Can't push rebased commits if others are based on them
- Loses information about when integration occurred
- Harder to understand "where did this code come from?"

### Team Conventions

**Merge-heavy teams:**

- Value clear integration history
- Accept merge commits as necessary overhead
- Easier onboarding (rebase is less intuitive)
- Common in enterprise environments

**Rebase-heavy teams:**

- Value clean history
- Rebasing is a discipline (don't rebase shared branches)
- Steeper learning curve
- Common in open-source (clean main, reviewable commits)

**Hybrid (common in practice):**

- Rebase feature branches locally before opening PR
- Merge PR to main (or squash-merge for single commit)
- Maintains both clean history and integration record

```bash
git rebase main           # Local cleanup before PR
git push origin feature
# On GitHub: "Squash and merge" or "Create merge commit"
```

## Rebase Onto: Transplanting Commits

`git rebase --onto` moves commits from one branch to another, skipping the original base.

### Syntax

```bash
git rebase --onto <new-base> <old-base> <branch>
```

**Example:** Feature branch `feature/auth` branched from `develop` three commits ago, but you want to rebase it onto latest `main`:

```
develop: ────────────D (old base)
         ↓
feature: ────────────F1─→F2─→F3 (3 feature commits)

main:    ────────────────M (newer than develop)
```

To move feature commits onto main:

```bash
git rebase --onto main develop feature/auth
```

Result:

```
main:    ────────────────M
         ↓↓↓ (feature commits replayed here)
feature: ────────────────M─→F1'─→F2'─→F3'
```

### Use Cases

- **Feature from stale branch**: Rebasing feature branch from old release onto latest main
- **Cherry-picking a series**: Moving a series of commits without bringing the rest of the branch
- **Branch forest management**: Organizing interdependent branches

## Updating Refs During Rebase

`git rebase --update-refs` (Git 2.38+) updates dependent branches automatically during rebase.

### Scenario

```
main: ────────────────────
feature1: ────F1─→F2 (based on main)
feature2: ────F1─→F2─→F3─→F4 (based on feature1)
```

Rebase feature1 onto latest main:

```bash
git rebase --update-refs main feature1
```

Result: Both feature1 and feature2 are updated (feature2 is now based on the rebased feature1).

Without `--update-refs`, feature2 would be broken or need manual rebasing.

## Recovering from Bad Rebases

Rebase gone wrong? Use the reflog to recover.

### Abort During Rebase

```bash
git rebase --abort    # Return to pre-rebase state immediately
```

### Recovery After Rebase

If you've completed a bad rebase and realized the mistake:

```bash
git reflog              # Find the original HEAD before rebase
git reset --hard HEAD@{3}  # Return to that state (commits are restored)
```

The original commits aren't lost—they're dangling but in the reflog until garbage collection (default 90 days).

### Reverting Just One Commit

If rebase succeeded but one commit is corrupted:

```bash
git rebase -i HEAD~[count]  # Reopen the rebase
# Mark the problematic commit with 'drop'
git rebase --continue
```

Or manually edit and continue without dropping:

```bash
git commit --amend  # Fix the current commit
git rebase --continue
```

## Rebasing Published Branches: a Caution

Never rebase commits that others have based work on:

```
main (shared):      ────────A───B───C

Developer 1:        ────────A───B───C───D (local branch)
Developer 2:        ────────A───B───C─────────E (local branch based on your C)
```

If Developer 1 rebases and force-pushes:

```
Developer 1:        ────────A───B'───C'───D'   (rebased, force-pushed)
Developer 2 pulls:  CONFLICT (C no longer exists; C' is different)
```

Developer 2 must:

```bash
git fetch origin
git rebase origin/main  # Rebase their work onto the new history
# Or if willing to accept remote changes:
git pull --rebase
```

**Rule:** Rebase is safe for local branches or branches you're the sole owner of. For shared branches (main, develop), use merge.

---

_Sources: Pro Git v2 (https://git-scm.com/book/en/v2), Atlassian Git Tutorials, git-scm.com documentation, Andrew Lock (autosquash tutorial)_