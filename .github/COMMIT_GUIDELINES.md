# Commit Message Guidelines for git-upload

This file is automatically referenced when generating AI commit messages via `git upload -ai`.

## Project Context

This repository contains **git-shell-helpers**: a collection of shell scripts that extend Git with user-friendly commands:

- `git-upload` - Stage, commit, and push in one command (with optional AI commit messages)
- `git-get` - Clone or pull with smart defaults
- `git-initialize` - Initialize repos with sensible defaults
- `git-remerge` - Re-apply failed merges safely
- `git-resolve` - Interactive conflict resolution helper
- `git-fucked-the-push` - Recovery tool for push failures

## Commit Message Style

### Subject Line

- Use imperative mood: "Add feature" not "Added feature"
- Be specific: name the script/function affected
- Max 72 characters
- No period at end

### Body Structure

```
Summary:
- Specific change 1 (name files/functions)
- Specific change 2

Why:
- Motivation or problem solved

Breaking changes: none | specific list of what breaks

Risk: low|medium|high (rationale)

Testing: <status from test suite>
```

### Examples of Good Subjects

- `Add early-exit when nothing to commit in git-upload`
- `Fix spinner not stopping on SIGINT in git-upload`
- `Reduce AI timeout from 300s to 60s for faster failures`

### Examples of Bad Subjects

- `Update script` (which script?)
- `Fix bug` (what bug?)
- `Improve performance` (how? where?)

## File Conventions

| Path                 | Purpose                            |
| -------------------- | ---------------------------------- |
| `git-*`              | Main shell scripts (user commands) |
| `scripts/`           | Build and test utilities           |
| `man/man1/`          | Man pages for each command         |
| `build/`             | Build artifacts (not committed)    |
| `.github/workflows/` | CI/CD pipelines                    |

## IF VERSION WAS BUMPED

Compare diffs from the old version, then write new release notes for the new version. These release notes should be concise bullet points outlining new features and quick 5-10 word explinations, really as simple as it can get. Ensure that the markdown file matching the version number (`v!.@.#.md` replace !.@.# with actual version number for release) is created in the release-notes folder BEFORE COMMIT AND FINAL UPLOAD. If upload has started and there is a new version, go back and repair this issue by running this flow, commit a second time (if necessary, e.g. you already commited then realized new version doesn't have release notes, commit ontop of your other commit, no need to revise or change history) then push.
