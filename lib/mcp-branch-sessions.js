// lib/mcp-branch-sessions.js — Branch session tools (worktree-backed isolation)
"use strict";

const path = require("path");
const fs = require("fs");
const { execFileSync } = require("child_process");
const {
  execGit,
  resolveRepoRoot,
  worktreePath,
  WORKTREE_BASE,
} = require("./mcp-git");
const {
  notifyWorktreeCreated,
  notifyWorktreeRemoved,
} = require("./mcp-activity-ipc");

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

const BRANCH_SESSION_START_TOOL = {
  name: "branch_session_start",
  description:
    "Start an isolated branch session. Creates a git worktree for the branch and checks out the branch in the main workspace via symbolic-ref so the user sees the branch files in the VS Code Explorer. IMPORTANT: After this call, continue using the normal workspace root for ALL file operations (create_file, replace_string_in_file, etc.) and terminal commands — do NOT use the worktree path. The workspace is already on the feature branch. This ensures the normal Copilot Keep/Undo UI works. The worktree exists for git bookkeeping; you don't need to interact with it directly. If the branch does not exist yet, it is created from the specified base (defaults to HEAD). If a worktree for this branch already exists, reuses it.",
  inputSchema: {
    type: "object",
    properties: {
      branch: {
        type: "string",
        description:
          "The branch to work on. If it does not exist, it will be created from 'base'.",
      },
      base: {
        type: "string",
        description:
          "The commit, branch, or tag to create a new branch from. Only used when creating a new branch. Defaults to HEAD.",
      },
    },
    required: ["branch"],
  },
};

const BRANCH_SESSION_END_TOOL = {
  name: "branch_session_end",
  description:
    "End an isolated branch session. Commits any uncommitted changes on the worktree branch (with an auto-generated message), then removes the worktree. The extension restores the original branch in the main workspace. By default the branch and its commits are preserved for later merge. Pass merge: true to automatically merge the feature branch into the current baseline with --no-ff and delete the feature branch. If merge conflicts occur, the merge is left IN PROGRESS with conflict markers in the workspace files — resolve them with normal file editing tools, then git add + git commit --no-edit. Pass discard: true to remove the worktree without committing (discards uncommitted changes).",
  inputSchema: {
    type: "object",
    properties: {
      branch: {
        type: "string",
        description: "The branch whose worktree session to end.",
      },
      merge: {
        type: "boolean",
        description:
          "If true, merge the feature branch into the main repo's current branch (usually the baseline) with --no-ff after removing the worktree, then delete the feature branch. On conflict, the merge is left in progress with conflict markers in the workspace files. Default: false.",
      },
      discard: {
        type: "boolean",
        description:
          "If true, remove the worktree without committing uncommitted changes. Default: false.",
      },
      nukeChanges: {
        type: "boolean",
        description:
          "⚠️ DESTRUCTIVE — IRREVERSIBLE. Removes the worktree AND force-deletes the branch, permanently destroying ALL commits on that branch. Every commit, every file change, every piece of work on the branch is gone forever with NO recovery path. This is a full wipe — the branch ceases to exist entirely. ONLY use this when the user has EXPLICITLY asked to delete/destroy/trash/nuke the branch and all its history. If the user said anything else — 'close', 'end', 'stop', 'park', 'save for later' — do NOT use this. When in doubt, do NOT set this flag. Default: false.",
      },
    },
    required: ["branch"],
  },
};

const BRANCH_READ_FILE_TOOL = {
  name: "branch_read_file",
  description:
    "Read a file from any branch without checking it out or needing a worktree. Uses git show to read the file directly from the branch's commit tree. Useful for inspecting code on another branch before deciding to start a session, or for comparing implementations across branches.",
  inputSchema: {
    type: "object",
    properties: {
      branch: {
        type: "string",
        description: "The branch (or commit/tag) to read from.",
      },
      filePath: {
        type: "string",
        description:
          "The repository-relative path to the file (e.g. 'lib/upload-ai-message.sh').",
      },
    },
    required: ["branch", "filePath"],
  },
};

const BRANCH_STATUS_TOOL = {
  name: "branch_status",
  description:
    "Show the status of all active branch sessions and local branches. Reports: which worktrees are active, their branches, dirty/clean state, and recent commits. Also lists local branches with their latest commit. Use to understand the state of ongoing parallel work before switching context or merging.",
  inputSchema: {
    type: "object",
    properties: {},
    required: [],
  },
};

const BRANCH_CLEANUP_TOOL = {
  name: "branch_cleanup",
  description:
    "Clean up stale feature branches that have been fully merged into the baseline branch (dev, main) or that have no unique commits. Removes local branches only — does not touch remote branches unless push: true is set. Dry-run by default: shows what would be deleted without deleting. Pass execute: true to actually delete. Useful for tidying up after many branch sessions.",
  inputSchema: {
    type: "object",
    properties: {
      baseBranch: {
        type: "string",
        description:
          "The baseline branch to check merges against. Defaults to the current branch (usually dev or main).",
      },
      execute: {
        type: "boolean",
        description:
          "If true, actually delete the merged branches. If false (default), only list what would be deleted (dry run).",
      },
      push: {
        type: "boolean",
        description:
          "If true and execute is true, also delete remote tracking branches. Default: false.",
      },
      pattern: {
        type: "string",
        description:
          "Only clean branches matching this prefix (e.g. 'feature/test-'). If omitted, cleans all feature/ and fix/ branches that are merged.",
      },
    },
    required: [],
  },
};

const BRANCH_SESSION_TOOLS = [
  BRANCH_SESSION_START_TOOL,
  BRANCH_SESSION_END_TOOL,
  BRANCH_READ_FILE_TOOL,
  BRANCH_STATUS_TOOL,
  BRANCH_CLEANUP_TOOL,
];

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async function handleBranchSessionStart(args, activityId) {
  const { branch, base } = args;
  if (!branch) throw new Error("branch is required");

  const toplevel = await resolveRepoRoot();
  const wtPath = worktreePath(branch);

  // If worktree already exists and is valid, return it
  if (fs.existsSync(wtPath)) {
    try {
      const existing = await execGit(["rev-parse", "--show-toplevel"], wtPath);
      const existingBranch = await execGit(
        ["symbolic-ref", "--short", "HEAD"],
        wtPath,
      );
      let existingCommit = "";
      try {
        existingCommit = await execGit(
          ["rev-parse", "--short", "HEAD"],
          wtPath,
        );
      } catch {}
      const status = await execGit(["status", "--short"], wtPath);
      return [
        {
          type: "text",
          text: [
            `Branch session already active.`,
            `Branch: ${existingBranch}`,
            existingCommit ? `Commit: ${existingCommit}` : "",
            `Status: ${status || "clean"}`,
            `Worktree: ${existing}`,
            ``,
            `The workspace is checked out to this branch. Use the normal workspace root for all file operations and terminal commands.`,
          ].join("\n"),
        },
      ];
    } catch {
      // Stale worktree — remove and recreate
      try {
        await execGit(["worktree", "remove", "--force", wtPath], toplevel);
      } catch {
        fs.rmSync(wtPath, { recursive: true, force: true });
        try {
          await execGit(["worktree", "prune"], toplevel);
        } catch {
          // ignore
        }
      }
    }
  }

  // Ensure base directory exists
  fs.mkdirSync(WORKTREE_BASE, { recursive: true });

  // Check if branch exists
  let branchExists = false;
  try {
    await execGit(["rev-parse", "--verify", `refs/heads/${branch}`], toplevel);
    branchExists = true;
  } catch {
    // branch doesn't exist yet
  }

  if (branchExists) {
    await execGit(["worktree", "add", wtPath, branch], toplevel);
  } else {
    const baseRef = base || "HEAD";
    await execGit(["worktree", "add", "-b", branch, wtPath, baseRef], toplevel);
  }

  const status = await execGit(["status", "--short", "--branch"], wtPath);

  let baseBranch = base || "";
  if (!baseBranch) {
    try {
      baseBranch = await execGit(["symbolic-ref", "--short", "HEAD"], toplevel);
    } catch {
      baseBranch = "HEAD";
    }
  }
  let baseCommit = "";
  try {
    baseCommit = await execGit(["rev-parse", "--short", "HEAD"], wtPath);
  } catch {
    // ignore
  }

  notifyWorktreeCreated(branch, wtPath, baseBranch, baseCommit, activityId);

  // Wait for the extension to focus the branch in the main repo
  try {
    const maxWait = 12000;
    const interval = 300;
    const start = Date.now();
    while (Date.now() - start < maxWait) {
      await new Promise((r) => setTimeout(r, interval));
      try {
        const current = await execGit(
          ["symbolic-ref", "--short", "HEAD"],
          toplevel,
        );
        if (current === branch) break;
      } catch {
        // detached HEAD or transient state — keep waiting
      }
    }
  } catch {
    // Non-fatal
  }

  let headCommit = "";
  try {
    headCommit = await execGit(["rev-parse", "--short", "HEAD"], wtPath);
  } catch {}

  return [
    {
      type: "text",
      text: [
        `Branch session started.`,
        `Branch: ${branch}`,
        headCommit ? `Commit: ${headCommit}` : "",
        `Created from: ${branchExists ? "(existing branch)" : base || "HEAD"}`,
        `Worktree: ${wtPath}`,
        ``,
        `The workspace is now checked out to this branch. Use the normal workspace root for all file operations and terminal commands — do NOT use the worktree path directly.`,
        `When done, call branch_session_end to commit and clean up.`,
        `Pass merge: true to auto-merge into the baseline when ending.`,
      ]
        .filter(Boolean)
        .join("\n"),
    },
  ];
}

async function handleBranchSessionEnd(args) {
  const { branch, discard, merge, nukeChanges } = args;
  if (!branch) throw new Error("branch is required");

  const toplevel = await resolveRepoRoot();
  const wtPath = worktreePath(branch);

  if (!fs.existsSync(wtPath)) {
    return [
      {
        type: "text",
        text: `No active worktree found for branch '${branch}'. Nothing to clean up.`,
      },
    ];
  }

  // Check for uncommitted work
  let dirty = "";
  let commitCwd = wtPath;
  if (!nukeChanges) {
    try {
      const mainBranch = await execGit(
        ["symbolic-ref", "--short", "HEAD"],
        toplevel,
      );
      if (mainBranch === branch) {
        const mainDirty = await execGit(["status", "--short"], toplevel);
        if (mainDirty) {
          dirty = mainDirty;
          commitCwd = toplevel;
        }
      }
    } catch {
      // ignore
    }
    if (!dirty) {
      try {
        dirty = await execGit(["status", "--short"], wtPath);
        commitCwd = wtPath;
      } catch {
        // ignore
      }
    }
  }

  let commitInfo = "";
  if (dirty && !discard && !nukeChanges) {
    try {
      await execGit(["add", "-A"], commitCwd);
      await execGit(
        [
          "commit",
          "-m",
          `WIP: auto-commit from branch session end on '${branch}'`,
        ],
        commitCwd,
      );
      commitInfo = `Auto-committed uncommitted changes on '${branch}'.`;
    } catch (err) {
      commitInfo = `Warning: could not auto-commit: ${err.message}`;
    }
  } else if (dirty && (discard || nukeChanges)) {
    commitInfo = `Discarded uncommitted changes on '${branch}'.`;
  }

  let lastCommit = "";
  try {
    lastCommit = await execGit(["log", "--oneline", "-1"], wtPath);
  } catch {
    // ignore
  }

  // Remove the worktree
  try {
    await execGit(["worktree", "remove", "--force", wtPath], toplevel);
  } catch {
    fs.rmSync(wtPath, { recursive: true, force: true });
    try {
      await execGit(["worktree", "prune"], toplevel);
    } catch {
      // ignore
    }
  }

  notifyWorktreeRemoved(branch, wtPath);

  // Wait for extension to restore original branch
  try {
    const maxWait = 8000;
    const interval = 300;
    const start = Date.now();
    while (Date.now() - start < maxWait) {
      await new Promise((r) => setTimeout(r, interval));
      try {
        const current = await execGit(
          ["symbolic-ref", "--short", "HEAD"],
          toplevel,
        );
        if (current !== branch) break;
      } catch {
        break;
      }
    }
  } catch {
    // Non-fatal
  }

  // Nuke: force-delete the branch entirely
  if (nukeChanges) {
    let nukeResult = "";
    try {
      await execGit(["branch", "-D", branch], toplevel);
      nukeResult = `Branch '${branch}' has been permanently destroyed. All commits on that branch are gone.`;
    } catch (err) {
      nukeResult = `Warning: force-delete of '${branch}' failed: ${err.message}. The branch may still exist.`;
    }

    return [
      {
        type: "text",
        text: [
          `Branch session NUKED.`,
          `Branch: ${branch}`,
          commitInfo || "No uncommitted changes were saved.",
          ``,
          nukeResult,
        ]
          .filter(Boolean)
          .join("\n"),
      },
    ];
  }

  // Merge-on-end
  let mergeInfo = "";
  let branchDeleted = false;
  if (merge && !discard) {
    try {
      const currentBranch = await execGit(
        ["symbolic-ref", "--short", "HEAD"],
        toplevel,
      );
      try {
        await execGit(
          [
            "merge",
            "--no-ff",
            branch,
            "-m",
            `Merge '${branch}' into ${currentBranch}`,
          ],
          toplevel,
        );
        mergeInfo = `Merged '${branch}' into ${currentBranch} (--no-ff).`;
        try {
          await execGit(["branch", "-d", branch], toplevel);
          branchDeleted = true;
          mergeInfo += ` Branch '${branch}' deleted.`;
        } catch (delErr) {
          mergeInfo += ` Warning: could not delete branch: ${delErr.message}`;
        }
      } catch {
        let conflictFiles = "";
        try {
          conflictFiles = await execGit(
            ["diff", "--name-only", "--diff-filter=U"],
            toplevel,
          );
        } catch {
          // ignore
        }

        if (conflictFiles) {
          const files = conflictFiles.split("\n").filter(Boolean);
          const filePaths = files.map((f) => path.join(toplevel, f));

          mergeInfo = [
            `Merge conflict: ${files.length} file(s) conflict when merging '${branch}' into ${currentBranch}.`,
            `The merge is IN PROGRESS — conflict markers are in these files:`,
            ``,
            ...filePaths.map((p) => `  ${p}`),
            ``,
            `To resolve:`,
            `1. Open each file above — look for <<<<<<< / ======= / >>>>>>> markers`,
            `2. Edit the files to keep the correct content (remove the markers)`,
            `3. Run: git add <file> for each resolved file`,
            `4. Run: git commit --no-edit to complete the merge`,
            `5. Then delete the branch: git branch -d ${branch}`,
            ``,
            `To abort instead: git merge --abort`,
          ].join("\n");
        } else {
          try {
            await execGit(["merge", "--abort"], toplevel);
          } catch {}
          mergeInfo = [
            `Merge failed when merging '${branch}' into ${currentBranch}.`,
            `The merge was aborted. Branch '${branch}' is preserved — resolve manually.`,
          ].join("\n");
        }
      }
    } catch {
      mergeInfo = `Warning: could not determine current branch for merge. Branch '${branch}' is preserved.`;
    }
  }

  const resultLines = [
    `Branch session ended.`,
    `Branch: ${branch}`,
    commitInfo,
    lastCommit ? `Last commit: ${lastCommit}` : "",
  ];

  if (mergeInfo) {
    resultLines.push("", mergeInfo);
  } else if (!branchDeleted) {
    // Auto-cleanup: if the branch is already fully merged into the baseline,
    // delete it to prevent accumulation of stale branches.
    let autoDeleted = false;
    try {
      const currentBranch = await execGit(
        ["symbolic-ref", "--short", "HEAD"],
        toplevel,
      );
      const mergedList = await execGit(
        ["branch", "--merged", currentBranch],
        toplevel,
      );
      const mergedBranches = mergedList
        .split("\n")
        .map((b) => b.replace(/^\*?\s+/, "").trim());
      if (mergedBranches.includes(branch)) {
        try {
          await execGit(["branch", "-d", branch], toplevel);
          autoDeleted = true;
          resultLines.push(
            "",
            `Branch '${branch}' was fully merged into ${currentBranch} and has been deleted.`,
          );
        } catch {
          // Could not delete — leave it
        }
      }
    } catch {
      // Could not check merge status — leave the branch
    }

    if (!autoDeleted) {
      resultLines.push(
        "",
        `The branch and all its commits are preserved.`,
        `To merge now: branch_session_end({ branch: "${branch}", merge: true })`,
        `Or manually: git merge --no-ff ${branch}`,
      );
    }
  }

  return [
    {
      type: "text",
      text: resultLines.filter(Boolean).join("\n"),
    },
  ];
}

async function handleBranchReadFile(args) {
  const { branch, filePath } = args;
  if (!branch) throw new Error("branch is required");
  if (!filePath) throw new Error("filePath is required");

  const toplevel = await resolveRepoRoot();

  let content;
  try {
    content = await execGit(["show", `${branch}:${filePath}`], toplevel);
  } catch (err) {
    throw new Error(
      `Could not read '${filePath}' from branch '${branch}': ${err.message}`,
    );
  }

  let commitHash = "";
  try {
    commitHash = await execGit(["rev-parse", "--short", branch], toplevel);
  } catch {
    // ignore
  }

  return [
    {
      type: "text",
      text: `File: ${filePath}\nBranch: ${branch}${commitHash ? ` (${commitHash})` : ""}\n\n${content}`,
    },
  ];
}

async function handleBranchStatus() {
  const toplevel = await resolveRepoRoot();
  const sections = [];

  let worktreeList = "";
  try {
    worktreeList = await execGit(["worktree", "list", "--porcelain"], toplevel);
  } catch {
    // ignore
  }

  if (worktreeList) {
    const worktrees = [];
    let current = {};
    for (const line of worktreeList.split("\n")) {
      if (line.startsWith("worktree ")) {
        if (current.path) worktrees.push(current);
        current = { path: line.slice(9) };
      } else if (line.startsWith("HEAD ")) {
        current.head = line.slice(5, 12);
      } else if (line.startsWith("branch ")) {
        current.branch = line.slice(7).replace("refs/heads/", "");
      } else if (line === "detached") {
        current.branch = "(detached)";
      } else if (line === "") {
        if (current.path) worktrees.push(current);
        current = {};
      }
    }
    if (current.path) worktrees.push(current);

    const agentWts = worktrees.filter((w) => w.path.startsWith(WORKTREE_BASE));
    const mainWt = worktrees.find((w) => !w.path.startsWith(WORKTREE_BASE));

    if (mainWt) {
      let mainStatus = "";
      try {
        mainStatus = await execGit(["status", "--short"], mainWt.path);
      } catch {
        // ignore
      }
      sections.push(
        `Main workspace:\n  Path: ${mainWt.path}\n  Branch: ${mainWt.branch || "(unknown)"}\n  Status: ${mainStatus || "clean"}`,
      );
    }

    if (agentWts.length > 0) {
      const wtLines = [];
      for (const wt of agentWts) {
        let status = "";
        try {
          status = await execGit(["status", "--short"], wt.path);
        } catch {
          status = "(inaccessible)";
        }
        let lastCommit = "";
        try {
          lastCommit = await execGit(["log", "--oneline", "-1"], wt.path);
        } catch {
          // ignore
        }
        wtLines.push(
          `  ${wt.branch || "(unknown)"}:\n    Path: ${wt.path}\n    Status: ${status || "clean"}\n    Last: ${lastCommit || "(no commits)"}`,
        );
      }
      sections.push(`Active branch sessions:\n${wtLines.join("\n")}`);
    } else {
      sections.push("Active branch sessions: none");
    }
  }

  let branches = "";
  try {
    branches = await execGit(
      [
        "branch",
        "-v",
        "--format=%(refname:short) %(objectname:short) %(subject)",
      ],
      toplevel,
    );
  } catch {
    // ignore
  }
  if (branches) {
    sections.push(`Local branches:\n${branches}`);
  }

  return [{ type: "text", text: sections.join("\n\n") }];
}

async function handleBranchCleanup(args) {
  const { baseBranch, execute, push, pattern } = args || {};
  const toplevel = await resolveRepoRoot();

  // Determine baseline
  let base = baseBranch;
  if (!base) {
    try {
      base = await execGit(["symbolic-ref", "--short", "HEAD"], toplevel);
    } catch {
      base = "dev";
    }
  }

  // Protected branches that should never be deleted
  const protectedBranches = new Set(["main", "master", "dev", "develop", base]);

  // Get merged branches
  let mergedRaw = "";
  try {
    mergedRaw = await execGit(["branch", "--merged", base], toplevel);
  } catch {
    return [
      { type: "text", text: `Could not list branches merged into '${base}'.` },
    ];
  }

  const merged = mergedRaw
    .split("\n")
    .map((b) => b.replace(/^\*?\s+/, "").trim())
    .filter((b) => b && !protectedBranches.has(b));

  // Get active worktree branches (don't delete those)
  const activeWorktreeBranches = new Set();
  try {
    const wtList = await execGit(["worktree", "list", "--porcelain"], toplevel);
    for (const line of wtList.split("\n")) {
      if (line.startsWith("branch ")) {
        activeWorktreeBranches.add(line.slice(7).replace("refs/heads/", ""));
      }
    }
  } catch {
    // ignore
  }

  // Filter candidates
  let candidates = merged.filter((b) => !activeWorktreeBranches.has(b));
  if (pattern) {
    candidates = candidates.filter((b) => b.startsWith(pattern));
  }

  if (candidates.length === 0) {
    return [
      {
        type: "text",
        text: `No stale branches found merged into '${base}'${pattern ? ` matching '${pattern}'` : ""}. Repository is clean.`,
      },
    ];
  }

  if (!execute) {
    // Dry run
    const lines = [
      `Dry run: ${candidates.length} branch(es) merged into '${base}' would be deleted:`,
      "",
      ...candidates.map((b) => `  - ${b}`),
      "",
      `To delete these branches, call branch_cleanup with execute: true.`,
    ];
    if (push) {
      lines.push(
        `Remote tracking branches would also be deleted (push: true).`,
      );
    }
    return [{ type: "text", text: lines.join("\n") }];
  }

  // Execute deletion
  const deleted = [];
  const failed = [];
  for (const branch of candidates) {
    try {
      await execGit(["branch", "-d", branch], toplevel);
      deleted.push(branch);
    } catch (err) {
      failed.push({ branch, error: err.message });
    }
  }

  // Optionally delete remote tracking branches
  const remoteDeleted = [];
  if (push && deleted.length > 0) {
    for (const branch of deleted) {
      try {
        await execGit(["push", "origin", "--delete", branch], toplevel);
        remoteDeleted.push(branch);
      } catch {
        // Remote branch may not exist — non-fatal
      }
    }
  }

  const lines = [`Branch cleanup complete (base: '${base}'):`];
  if (deleted.length > 0) {
    lines.push("", `Deleted ${deleted.length} local branch(es):`);
    for (const b of deleted) lines.push(`  ✓ ${b}`);
  }
  if (remoteDeleted.length > 0) {
    lines.push("", `Deleted ${remoteDeleted.length} remote branch(es):`);
    for (const b of remoteDeleted) lines.push(`  ✓ origin/${b}`);
  }
  if (failed.length > 0) {
    lines.push("", `Failed to delete ${failed.length} branch(es):`);
    for (const f of failed) lines.push(`  ✗ ${f.branch}: ${f.error}`);
  }
  return [{ type: "text", text: lines.join("\n") }];
}

// Periodic GC — clean up orphaned worktrees older than 24 hours
function gcWorktrees() {
  if (!fs.existsSync(WORKTREE_BASE)) return;
  const ONE_DAY = 24 * 60 * 60 * 1000;

  // 1. Prune stale worktree references from git's internal tracking
  try {
    const toplevel = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      timeout: 5000,
    })
      .toString()
      .trim();
    if (toplevel) {
      execFileSync("git", ["worktree", "prune"], {
        cwd: toplevel,
        timeout: 10000,
      });
    }
  } catch {
    // Non-fatal — may not be in a git repo context
  }

  // 2. Remove stale worktree directories that are no longer valid git checkouts
  try {
    const entries = fs.readdirSync(WORKTREE_BASE);
    for (const entry of entries) {
      const entryPath = path.join(WORKTREE_BASE, entry);
      try {
        const stat = fs.statSync(entryPath);
        if (Date.now() - stat.mtimeMs > ONE_DAY) {
          let isValidWorktree = false;
          try {
            execFileSync("git", ["rev-parse", "--show-toplevel"], {
              cwd: entryPath,
              timeout: 5000,
            });
            isValidWorktree = true;
          } catch {
            // Not a valid git worktree
          }
          if (!isValidWorktree) {
            fs.rmSync(entryPath, { recursive: true, force: true });
          }
        }
      } catch {
        // ignore
      }
    }
  } catch {
    // ignore
  }
}

// Run GC on startup and every 6 hours
gcWorktrees();
setInterval(gcWorktrees, 6 * 60 * 60 * 1000).unref();

module.exports = {
  BRANCH_SESSION_TOOLS,
  handleBranchSessionStart,
  handleBranchSessionEnd,
  handleBranchReadFile,
  handleBranchStatus,
  handleBranchCleanup,
};
