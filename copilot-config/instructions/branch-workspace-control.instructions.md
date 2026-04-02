---
applyTo: "**"
description: "Branch isolation via git worktrees. Opt-in via the gitShellHelpers.branchSessions.enabled setting. Worktrees appear as normal branches — the extension manages focus so git status and Explorer reflect the active chat's branch."
---

# Branch Workspace Control

Branch session tools are **off by default**. Enable them via:

```
Settings → Git Shell Helpers → Branch Sessions → Enabled
```

When **disabled** (default): agents use direct branching (`git checkout -b`) per the branch-lifecycle instructions. No worktree tools are exposed.

When **enabled**: agents get MCP tools to create isolated git worktrees for parallel branch work. Worktrees are **automatically bound to the active chat** — each chat gets its own isolated branch.

## Dead-Simple Decision Rule

Use this rule before starting any branch workflow work:

- Use branch sessions when work spans multiple files, needs parallel chats, or has meaningful regression risk.
- Avoid branch sessions for one-file trivial fixes, typo-only changes, or quick config edits that can safely land on baseline.

If you are unsure, do this in order:

1. Call `workspace_context`.
2. If `gitShellHelpers.branchSessions.enabled` is true and the task is non-trivial, use `branch_session_start`.
3. Otherwise use direct branching (`git checkout -b`) per lifecycle rules.

## Strengths and Limits

Branch sessions are strongest when:

- You need isolation across multiple chats.
- You need safe context switching without losing in-progress work.
- You want the workspace to feel like normal branch usage while preserving worktree isolation.

Branch sessions should be avoided when:

- The user asked for a direct quick fix on baseline.
- The change is small enough that branching overhead adds no value.
- You cannot verify baseline branch intent and the user has not clarified it.

## Incorrect-Behavior Prevention Checklist

Run this checklist every time:

1. `workspace_context` before branch start, branch end, or checkpoint.
2. `branch_status` before creating a new session to avoid collisions.
3. Use `checkpoint` with `branch` guard when committing feature-branch work.
4. End sessions with `branch_session_end` so workspace focus is restored to baseline.

## How It Looks to the User

Branch sessions are meant to feel like normal branches, but they are not magic. The workspace follows whichever chat currently owns focus:

- `git branch` in the repo root shows the feature branch as current
- VS Code's source control panel shows the feature branch
- The Explorer sidebar reflects the feature branch's files
- Switching between chat sessions switches the visible branch automatically

Under the hood, the extension:

1. Creates an isolated git worktree in `~/.cache/gsh/worktrees/<branch>`
2. Checks out the feature branch in the **main repo** when the chat is focused
3. Stashes any uncommitted work before switching, restores it after
4. Switches the main repo back to the baseline branch when the chat loses focus or the session ends

This means a branch session can become parked when you leave its chat. The work is still on that branch and still in its worktree; it is not deleted or lost.

## MCP Tools (available when enabled)

| Tool                   | Purpose                                                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `branch_session_start` | Create a git worktree and check out the branch in the workspace. The workspace root is your working directory.                 |
| `branch_session_end`   | Commit outstanding changes and remove the worktree. Branch and commits persist for later merge. Unbinds from the chat session. |
| `branch_status`        | List all active worktrees, their branches, dirty/clean state, and recent commits.                                              |
| `branch_read_file`     | Read a file from any branch without checkout (useful for cross-branch comparison).                                             |
| `workspace_context`    | Get workspace roots, current branches, worktree status, remotes, AND active branch sessions.                                   |
| `checkpoint`           | Commit changes. No special args needed — the workspace is already on the feature branch.                                       |

## Workflow: Isolated Branch Work

```
1. workspace_context()
   → see workspace roots and any active branch sessions

2. branch_session_start({ branch: "fix/issue-42" })
   → workspace is now on branch fix/issue-42
   → bound to this chat session

3. All file operations use the normal workspace root:
   - read_file("src/foo.ts", ...)
   - run_in_terminal("npm test")
   - checkpoint({ all: true })

4. branch_session_end({ branch: "fix/issue-42" })
   → commits any remaining changes
   → removes worktree, restores baseline branch
```

If the workspace switches back to baseline after you leave a chat, that is expected. The session is parked. Switch back to the owning chat or call `branch_status` to find it.

## Recovery Playbook

If branch behavior looks wrong, do this exact sequence:

1. Call `workspace_context` to see the currently focused branch.
2. Call `branch_status` to locate active and parked sessions.
3. If your branch is parked, return to the owning chat or start a new session on a new branch.
4. If no session exists and you expected one, create it again with `branch_session_start`.

This should be the default recovery path. Do not guess, and do not edit files in cached worktree paths directly.

## Follow-Up Messages

When returning to a chat that has an active branch session:

1. Call `workspace_context` — it lists active branch sessions
2. If a session exists for your branch, the workspace is already on that branch — resume working normally
3. No special paths needed — use the workspace root

## Collision Avoidance

Before starting a new session, call `branch_status` to see what other agents are doing. Rules:

- **Never start a session on a branch another agent already owns.** Pick a different branch name.
- **Each chat session gets at most one branch.** Starting a new branch replaces the binding.

## Key Concepts

- **Worktree**: An isolated checkout in `~/.cache/gsh/worktrees/`. Each branch session gets its own directory. The user never interacts with this directory directly — the extension focuses the main repo checkout onto the worktree's branch.
- **Focus**: When a chat with an active branch session is focused, the extension checks out that branch in the main repo and stashes/restores any prior work. This means `git branch`, `git status`, and the VS Code Explorer all reflect the session's branch.
- **Parked session**: A branch session whose chat is not currently focused. The workspace may be back on baseline, but the branch session still exists and is recoverable by returning to that chat or using `branch_status`.
- **Session binding**: The VS Code extension tracks which chat owns which worktree. Bindings persist across extension reloads. Stale bindings (where the worktree was removed externally) are cleaned up automatically.
- **Toggle**: The `gitShellHelpers.branchSessions.enabled` setting controls whether branch tools appear. Restart the MCP server after changing it (reload window).
