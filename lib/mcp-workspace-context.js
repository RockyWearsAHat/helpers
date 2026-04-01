// lib/mcp-workspace-context.js — workspace_context tool
"use strict";

const path = require("path");
const fs = require("fs");
const { execGit, WORKTREE_BASE } = require("./mcp-git");

const WORKSPACE_CONTEXT_TOOL = {
  name: "workspace_context",
  description:
    "Return the current workspace context: workspace root folders, the active git branch in each, whether each is a worktree, and the remote URL. Call this at the start of a session or before making cross-branch operations to understand which branch and repository you are working in.",
  inputSchema: {
    type: "object",
    properties: {},
    required: [],
  },
};

async function handleWorkspaceContext() {
  let roots = [];
  if (process.env.GSH_WORKSPACE_ROOTS) {
    try {
      roots = JSON.parse(process.env.GSH_WORKSPACE_ROOTS);
    } catch {
      // malformed — ignore
    }
  }
  if (roots.length === 0) {
    roots = [process.cwd(), __dirname].filter(Boolean);
  }

  const seen = new Set();
  const results = [];
  for (const root of roots) {
    let toplevel;
    try {
      toplevel = await execGit(["rev-parse", "--show-toplevel"], root);
    } catch {
      continue;
    }
    if (seen.has(toplevel)) continue;
    seen.add(toplevel);

    let branch = "";
    try {
      branch = await execGit(["symbolic-ref", "--short", "HEAD"], toplevel);
    } catch {
      branch = "(detached HEAD)";
    }

    let isWorktree = false;
    try {
      const gitDir = await execGit(["rev-parse", "--git-dir"], toplevel);
      isWorktree = gitDir.includes(path.join(".git", "worktrees"));
    } catch {
      // ignore
    }

    let remote = "";
    try {
      remote = await execGit(["remote", "get-url", "origin"], toplevel);
    } catch {
      // no remote
    }

    let status = "";
    try {
      status = await execGit(["status", "--short", "--branch"], toplevel);
    } catch {
      // ignore
    }

    results.push({ root: toplevel, branch, isWorktree, remote, status });
  }

  if (results.length === 0) {
    return [{ type: "text", text: "No git repositories found in workspace." }];
  }

  const lines = results.map((r) => {
    const parts = [`Root: ${r.root}`, `Branch: ${r.branch}`];
    if (r.isWorktree) parts.push("Worktree: yes");
    if (r.remote) parts.push(`Remote: ${r.remote}`);
    if (r.status) parts.push(`Status:\n${r.status}`);
    return parts.join("\n");
  });

  // Scan for active branch session worktrees
  const activeWorktrees = [];
  try {
    if (fs.existsSync(WORKTREE_BASE)) {
      const entries = fs.readdirSync(WORKTREE_BASE);
      for (const entry of entries) {
        const entryPath = path.join(WORKTREE_BASE, entry);
        try {
          const stat = fs.statSync(entryPath);
          if (!stat.isDirectory()) continue;
          const wtBranch = await execGit(
            ["symbolic-ref", "--short", "HEAD"],
            entryPath,
          );
          const wtStatus = await execGit(
            ["status", "--short", "--branch"],
            entryPath,
          );
          activeWorktrees.push({
            path: entryPath,
            branch: wtBranch,
            status: wtStatus,
          });
        } catch {
          // not a valid worktree — skip
        }
      }
    }
  } catch {
    // ignore
  }

  let output = lines.join("\n\n---\n\n");
  if (activeWorktrees.length > 0) {
    output += "\n\n=== Active Branch Sessions ===\n\n";
    output += activeWorktrees
      .map((wt) => {
        const parts = [`Path: ${wt.path}`, `Branch: ${wt.branch}`];
        if (wt.status) parts.push(`Status:\n${wt.status}`);
        return parts.join("\n");
      })
      .join("\n\n---\n\n");
  }

  return [{ type: "text", text: output }];
}

module.exports = { WORKSPACE_CONTEXT_TOOL, handleWorkspaceContext };
