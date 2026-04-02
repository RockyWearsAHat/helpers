// lib/mcp-git.js — Shared git utilities for the gsh MCP server
"use strict";

const path = require("path");
const fs = require("fs");
const { execFile } = require("child_process");

const WORKTREE_BASE = path.join(
  process.env.HOME || process.env.USERPROFILE || "/tmp",
  ".cache",
  "gsh",
  "worktrees",
);

function execGit(args, cwd) {
  return new Promise((resolve, reject) => {
    execFile("git", args, { cwd, timeout: 30000 }, (err, stdout, stderr) => {
      if (err) {
        reject(new Error((stderr || err.message || "").trim()));
      } else {
        resolve((stdout || "").trim());
      }
    });
  });
}

function worktreePath(branch) {
  const safeName = branch.replace(/[^a-zA-Z0-9._-]/g, "-");
  return path.join(WORKTREE_BASE, safeName);
}

function findRepoRoot(startDir) {
  let current = path.resolve(String(startDir || process.cwd()));
  while (current && current !== path.dirname(current)) {
    if (
      fs.existsSync(path.join(current, ".git")) ||
      fs.existsSync(path.join(current, ".github"))
    ) {
      return current;
    }
    current = path.dirname(current);
  }
  if (
    current &&
    (fs.existsSync(path.join(current, ".git")) ||
      fs.existsSync(path.join(current, ".github")))
  ) {
    return current;
  }
  return "";
}

async function resolveRepoRoot() {
  const candidates = [];
  if (process.env.GSH_WORKSPACE_ROOTS) {
    try {
      const roots = JSON.parse(process.env.GSH_WORKSPACE_ROOTS);
      if (Array.isArray(roots)) candidates.push(...roots);
    } catch {
      // ignore
    }
  }
  candidates.push(process.cwd(), __dirname);
  for (const dir of candidates.filter(Boolean)) {
    try {
      return await execGit(["rev-parse", "--show-toplevel"], dir);
    } catch {
      // not a git repo — try next
    }
  }
  throw new Error("Not inside a git repository.");
}

module.exports = {
  execGit,
  findRepoRoot,
  resolveRepoRoot,
  worktreePath,
  WORKTREE_BASE,
};
