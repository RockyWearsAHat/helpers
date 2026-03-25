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

The cardinal rule: **describe behavior, not code**. The diff shows what code changed — the message explains what the program does differently now and why.

### Subject Line

- Use imperative mood: "Fix", "Add", "Stop", "Let" — not past tense
- Describe the EFFECT on behavior, not the mechanism or file names
- Must pass: "If applied, this commit will \_\_\_"
- ≤ 50 chars ideal, 72 hard max. No trailing period

### Body

- First sentence: the situation or problem BEFORE this change
- Then: what you did and why (not how — the code shows how)
- Last: consequences, side effects, things to know going forward
- NO section headers like "What changed:" / "Why:" — just write naturally
- If there's a number (speed, count, size), include it
- Group large changes by behavior, not by file

### Examples of Good Subjects

- `Stop spinner from persisting after Ctrl-C`
- `Checkpoint commits now generate their own AI message`
- `Reduce AI timeout from 300s to 60s for faster failures`
- `Fix install button showing up after app is already installed`

### Examples of Bad Subjects

- `Update git-upload` (what changed about it?)
- `Refactor prompt construction` (no behavior described)
- `Modify extension.js chip handler` (file names, not effects)
- `Improve error handling` (improve HOW?)

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
