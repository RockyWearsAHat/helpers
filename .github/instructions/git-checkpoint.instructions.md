---
description: "Guidance for when and how to use git-checkpoint during coding sessions."
applyTo: "**"
---

# Git Checkpoint — When to Commit

`git checkpoint` creates a local commit with a proper AI-generated message.
It does NOT push. Use it at meaningful moments during development.

## When to checkpoint

- **A fix is verified and working.** You changed something, tested it, it works now. That's a checkpoint.
- **A logical unit of work is complete.** You added a feature, wired up a new endpoint, finished a refactor. Done? Checkpoint.
- **You're about to switch context.** Moving to a different file, feature, or problem. Checkpoint what you have first.
- **Before a risky change.** About to rewrite something that might break? Checkpoint the working state.
- **Before deploying or pushing.** Code needs to go to a server? Checkpoint, then push.

## When NOT to checkpoint

- **Mid-change.** Don't commit half-finished work unless you're explicitly saving a WIP state.
- **After every small edit.** Changing one typo doesn't need its own commit unless it fixes a real bug.
- **Automatically on a timer.** Commits mark meaningful versions, not clock ticks.

## How to use

```bash
git checkpoint              # Stage + AI message + local commit
git checkpoint -a           # Stage all tracked + AI message + commit
git checkpoint -m "message" # Manual message
git checkpoint --push       # Commit and push
git checkpoint "context"    # Extra context for the AI prompt
```

## Configuration

```bash
git checkpoint --enable     # Enable for this repo (default: enabled)
git checkpoint --disable    # Disable for this repo
git checkpoint --status     # Show current config
```

Per-repo git config keys:

- `checkpoint.enabled` — gate for the command (default: true)
- `checkpoint.push` — always push after commit (default: false)
- `checkpoint.sign` — GPG-sign commits (default: false)

## For AI assistants

When you've completed a task and the changes are verified working, suggest:

> "These changes look good — want me to checkpoint this? `git checkpoint -a`"

Do NOT checkpoint without the user's knowledge. Always confirm first.
