---
applyTo: "**"
description: "Agent-autonomous branch isolation. Teaches agents to manage their own worktree sessions in parallel without user intervention."
---

# Branch Workspace Control — Agent Autonomy

Multiple agents can work on separate branches simultaneously. Each agent owns its worktree session end-to-end: creation, file operations, commits, and cleanup. No user intervention is needed — the system handles isolation automatically.

## How It Works

1. An agent calls `branch_session_start` → gets an isolated worktree.
2. The extension automatically binds the active chat session to that branch.
3. The agent works entirely within the worktree path (file edits, terminal commands, checkpoint).
4. When done, the agent calls `branch_session_end` → worktree is cleaned up, binding is removed.

Other agents in other chat sessions do the same thing concurrently. Each agent's worktree is a separate filesystem checkout — no interference.

## MCP Tools

| Tool | Purpose |
|---|---|
| `branch_session_start` | Create a git worktree for isolated branch work. Returns the absolute worktree path. |
| `branch_session_end` | Commit outstanding changes and remove the worktree. Branch and commits persist for later merge. |
| `branch_status` | List all active worktrees, their branches, dirty/clean state, and recent commits. |
| `branch_read_file` | Read a file from any branch without checkout (useful for cross-branch comparison). |
| `navigate_to_branch` | Switch the VS Code workspace view to a branch worktree or back to the start branch. |
| `workspace_context` | Get workspace roots, current branches, worktree status, remotes. |
| `checkpoint` | Commit changes. Pass `cwd` (worktree path) and `branch` to scope to the session. |

## VS Code Language Model Tool

| Tool | Purpose |
|---|---|
| `gsh-branch-state` | Query the full branch context: start branch, focused branch, all active sessions, chat-to-branch bindings. Use to detect collisions before starting work. |

## Workflow: Isolated Branch Work

```
1. branch_session_start({ branch: "fix/issue-42" })
   → returns { path: "/Users/.../worktrees/fix-issue-42" }
   → chat is automatically bound to this branch

2. All file operations use the returned worktree path:
   - read_file("/Users/.../worktrees/fix-issue-42/src/foo.ts", ...)
   - run_in_terminal("cd /Users/.../worktrees/fix-issue-42 && npm test")
   - checkpoint({ cwd: "/Users/.../worktrees/fix-issue-42", branch: "fix/issue-42", all: true })

3. branch_session_end({ branch: "fix/issue-42" })
   → commits any remaining changes
   → removes worktree
   → workspace returns to start branch
```

## Collision Avoidance

Before starting a new session, call `gsh-branch-state` or `branch_status` to see what other agents are doing. Rules:

- **Never start a session on a branch another agent already owns.** Pick a different branch name.
- **Never modify files outside your worktree path.** Your worktree is your sandbox.
- **Never call `navigate_to_branch` to switch to another agent's worktree.** Only navigate to your own session or back to the start branch.

## Key Concepts

- **Start branch**: The branch the user had open when VS Code launched. The "home" position all agents return to when their session ends.
- **Worktree**: An isolated checkout in `~/.cache/gsh/worktrees/`. Each branch session gets its own directory. Fully independent of other worktrees and the main checkout.
- **Chat binding**: Automatic association between a Copilot chat session and a branch. Created when `branch_session_start` fires, removed when `branch_session_end` fires. No manual step needed.
- **Focused path**: The branch worktree currently shown in VS Code's explorer. Changes when `navigate_to_branch` is called or when the active session ends.
