---
applyTo: "**"
description: "Branch isolation via git worktrees. Opt-in via the gitShellHelpers.branchSessions.enabled setting. Worktrees are automatically bound to chat sessions for per-chat branch isolation."
---

# Branch Workspace Control

Branch session tools are **off by default**. Enable them via:

```
Settings → Git Shell Helpers → Branch Sessions → Enabled
```

When **disabled** (default): agents use direct branching (`git checkout -b`) per the branch-lifecycle instructions. No worktree tools are exposed.

When **enabled**: agents get MCP tools to create isolated git worktrees for parallel branch work. Worktrees are **automatically bound to the current chat session** — each chat gets its own isolated workspace.

## How Session Binding Works

When you call `branch_session_start`, the system:

1. Creates an isolated git worktree at `~/.cache/gsh/worktrees/<branch>`
2. Notifies the VS Code extension via IPC
3. Extension binds the worktree to the currently active chat session
4. Extension adds the worktree as a workspace folder (visible in the Explorer sidebar with a 🌿 prefix)

When you switch between chat sessions, the extension automatically ensures the correct worktree folder is present. When you call `branch_session_end`, the extension removes the workspace folder.

## MCP Tools (available when enabled)

| Tool                   | Purpose                                                                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `branch_session_start` | Create a git worktree for isolated branch work. Returns the absolute worktree path. Automatically binds to the current chat session. |
| `branch_session_end`   | Commit outstanding changes and remove the worktree. Branch and commits persist for later merge. Unbinds from the chat session.       |
| `branch_status`        | List all active worktrees, their branches, dirty/clean state, and recent commits.                                                    |
| `branch_read_file`     | Read a file from any branch without checkout (useful for cross-branch comparison).                                                   |
| `workspace_context`    | Get workspace roots, current branches, worktree status, remotes, AND active branch sessions.                                         |
| `checkpoint`           | Commit changes. Pass `cwd` (worktree path) and `branch` to scope to the session.                                                     |

## Workflow: Isolated Branch Work

```
1. workspace_context()
   → see workspace roots and any active branch sessions

2. branch_session_start({ branch: "fix/issue-42" })
   → returns { path: "/Users/.../worktrees/fix-issue-42" }
   → worktree folder appears in VS Code Explorer
   → bound to this chat session

3. All file operations use the returned worktree path:
   - read_file("/Users/.../worktrees/fix-issue-42/src/foo.ts", ...)
   - run_in_terminal("cd /Users/.../worktrees/fix-issue-42 && npm test")
   - checkpoint({ cwd: "/Users/.../worktrees/fix-issue-42", branch: "fix/issue-42", all: true })

4. branch_session_end({ branch: "fix/issue-42" })
   → commits any remaining changes
   → removes worktree and workspace folder
```

## Follow-Up Messages

When returning to a chat that has an active branch session:

1. Call `workspace_context` — it lists active branch sessions with paths
2. If a session exists for your branch, resume working in that worktree path
3. The worktree folder is already in the workspace — no need to recreate it

## Collision Avoidance

Before starting a new session, call `branch_status` to see what other agents are doing. Rules:

- **Never start a session on a branch another agent already owns.** Pick a different branch name.
- **Never modify files outside your worktree path.** Your worktree is your sandbox.
- **Each chat session gets at most one worktree.** Starting a new branch replaces the binding.

## Key Concepts

- **Worktree**: An isolated checkout in `~/.cache/gsh/worktrees/`. Each branch session gets its own directory. Fully independent of other worktrees and the main checkout.
- **Session binding**: The VS Code extension tracks which chat created which worktree. Bindings persist across extension reloads. Stale bindings (where the worktree was removed externally) are cleaned up automatically.
- **Workspace folders**: Active worktree paths appear as additional workspace folders in VS Code. The main repository folder is always present. Worktree folders are added/removed as sessions start/end.
- **Toggle**: The `gitShellHelpers.branchSessions.enabled` setting controls whether branch tools appear. Restart the MCP server after changing it (reload window).
