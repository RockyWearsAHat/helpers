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

Use the `checkpoint` MCP tool — **never** run `git checkpoint` in the terminal.

The tool stages changes, generates a commit message itself via AI (reading the actual diff), and commits. You do not write the message. Just call it.

Optionally pass `context` with brief extra context if it would help the message (e.g. `"this fixes the login race condition from PR #42"`). Never pass `message` unless you need to override the AI output with a specific string.

### When to checkpoint

Call `checkpoint` with `{ "all": true }` when **both** conditions are met:

1. **A meaningful milestone occurred.** This includes explicit confirmation AND implicit satisfaction signals — you do not need to wait for the user to say the words:
   - Short positive replies: "great", "perfect", "nice", "ok", "that works", "👍", "exactly", "yes"
   - The user moving on to a new unrelated task or topic without reporting a new problem
   - The user asking to test, demo, or use what you just built
   - Any reply where the user is clearly satisfied and not raising a new issue
   - A build succeeds, tests pass, or an error that was being fixed is now gone
   - The tone shifts from problem-solving to something new
     Read the emotional intent. You do not need explicit permission.
2. **A checkpoint has not been recently taken** — don't double-commit the same state. If you just checkpointed moments ago and no new changes have been made, skip it.

Do NOT ask for permission — just call the tool. Checkpoints are local-only (no push) and cheap to amend or squash later, so err on the side of committing.

If the MCP tool is not available, first warn the user that the `gsh` MCP server may be stopped, untrusted, or disabled in VS Code. Tell them to open MCP server controls and start or trust `gsh` if needed. Only fall back to `git add -A && git commit -m "..."` in the terminal as a last resort when MCP still is not available.

If the tool returns a `[no-op]` response, the user disabled it mid-session. Do not retry, do not ask about it, just continue your current task.

The one exception: if the user **directly asked** you to checkpoint (e.g. "commit this", "checkpoint now") and the tool returned `[no-op]`, recall with `{ "force": true }` to override the mid-session disable — the user's explicit request takes priority. Never use `force` on automatic checkpoints.

**Do NOT checkpoint** when:

- You're still mid-change with uncommitted loose ends.
- The user hasn't reacted yet and the work hasn't been validated.
- The only change is a trivial typo fix with no user interaction.
