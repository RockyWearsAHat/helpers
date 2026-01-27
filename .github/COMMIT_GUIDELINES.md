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
