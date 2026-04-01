// lib/mcp-checkpoint.js — checkpoint tool: AI-generated local git commits
"use strict";

const fs = require("fs");
const path = require("path");
const { spawn } = require("child_process");
const { execGit, resolveRepoRoot, worktreePath } = require("./mcp-git");
const { notifyBranchCommit } = require("./mcp-activity-ipc");

const CHECKPOINT_TOOL = {
  name: "checkpoint",
  description:
    "Create a local git commit with an AI-generated message. Stages changes, generates a commit message from the diff, commits, and optionally pushes. Pass context for extra AI hints. Optionally override with a manual message.",
  inputSchema: {
    type: "object",
    properties: {
      context: {
        type: "string",
        description:
          "Optional extra context to include in the AI prompt (e.g. 'fixes the login bug introduced in last PR'). Ignored when message is provided.",
      },
      message: {
        type: "string",
        description:
          "Optional manual override for the commit message. If omitted, the message is AI-generated from the staged diff.",
      },
      all: {
        type: "boolean",
        description:
          "Stage all changes including untracked files (git add -A) before committing. Default: true.",
      },
      push: {
        type: "boolean",
        description: "Push to remote after committing. Default: false.",
      },
      force: {
        type: "boolean",
        description:
          "Override a mid-session disable. Only use this when the user explicitly asked for a checkpoint and the previous call returned [no-op]. Never set force on automatic checkpoints.",
      },
      cwd: {
        type: "string",
        description:
          "Absolute path to the git repository to commit in. Auto-detected from the workspace root when omitted. Pass explicitly when working in a multi-root workspace, a git worktree, or when the target repo differs from the server's working directory.",
      },
      branch: {
        type: "string",
        description:
          "Assert that HEAD is on this branch before committing. If the current branch does not match, the commit is aborted with an error. Use this to prevent accidentally committing to the wrong branch (e.g. committing to dev when you meant to commit to a feature branch).",
      },
    },
    required: [],
  },
};

async function generateAiCommitMessage(cwd, extraContext) {
  function gitOut(gitArgs) {
    return new Promise((resolve) => {
      require("child_process").execFile(
        "git",
        gitArgs,
        { cwd, timeout: 15000 },
        (err, stdout) => {
          resolve(err ? "" : (stdout || "").trim());
        },
      );
    });
  }

  const changedFiles = await gitOut(["diff", "--cached", "--name-only"]);
  const statSummary = await gitOut(["diff", "--cached", "--stat"]);
  let actualDiff = await gitOut(["diff", "--cached", "--unified=3"]);
  const diffLines = actualDiff.split("\n").length;
  if (diffLines > 2000) {
    actualDiff =
      actualDiff.split("\n").slice(0, 2000).join("\n") +
      `\n(truncated — showing first 2000 of ${diffLines} lines)`;
  }

  const recentHistory = await gitOut([
    "log",
    "--oneline",
    "-10",
    "--no-decorate",
  ]);
  const detailedRecent = await gitOut([
    "log",
    "-3",
    "--pretty=format:--- %h (%ar) ---%n%s%n%n%b",
  ]);

  let repoGuidance = "";
  for (const guidanceFile of [
    ".github/COMMIT_GUIDELINES.md",
    ".github/commit_guidelines.md",
    ".github/COMMIT_MESSAGE.md",
    ".github/commit_message.md",
    ".github/copilot-instructions.md",
    "AGENTS.md",
    "CLAUDE.md",
    "CONTRIBUTING.md",
  ]) {
    try {
      const raw = fs.readFileSync(path.join(cwd, guidanceFile), "utf8");
      const snippet = raw.split("\n").slice(0, 120).join("\n").trim();
      if (snippet) {
        repoGuidance += `\n--- ${guidanceFile} ---\n${snippet}\n`;
      }
    } catch {
      // File absent — ignore.
    }
  }

  const prompt = `You are a commit message generator. Your ONLY job is to describe
the staged diff below. You have no knowledge of any other project, conversation,
or task. Every word in your message must come from what you see in the diff and
the commit history of THIS repository. Do not infer, hallucinate, or borrow
context from outside this prompt.

This is a CHECKPOINT commit — the developer is marking a meaningful moment:
something works now that did not before, a logical unit of work is complete,
or they are about to switch context. Frame the message accordingly.

CONTEXT MATTERS MOST:
Read the recent commit history below. Each commit is part of an ongoing thread.
Frame yours as the next step in that story. If the diff and the history look
unrelated to each other, trust the diff — it is the ground truth.

SUBJECT LINE:
One line, <= 72 chars. Say what the commit DOES or FIXES, not what
files it touches.

BODY:
Describe the situation, what you did, and why. Someone reading git blame
should understand the reasoning without opening the diff.

Do NOT use section headers like 'What changed:', 'Why this matters:', etc.
For a tiny fix: one sentence or no body. For a real change: a short paragraph.

Never anthropomorphize code. Never restate the subject in different words.

OUTPUT FORMAT — output ONLY the commit between these markers:
COMMIT_BEGIN
<commit message>
COMMIT_END
`;

  let fullPrompt = `${prompt}
RECENT COMMIT HISTORY:
${recentHistory}

DETAILED RECENT COMMITS (last 3):
${detailedRecent}`;

  if (repoGuidance) {
    fullPrompt += `\n\nREPOSITORY GUIDANCE:${repoGuidance}`;
  }

  fullPrompt += `\n\n---\n\nChanged files:\n${changedFiles
    .split("\n")
    .map((file) => `  - ${file}`)
    .join("\n")}\n\nGit stat: ${statSummary
    .split("\n")
    .pop()}\n\nDIFF (${diffLines} lines):\n\`\`\`diff\n${actualDiff}\n\`\`\``;

  if (extraContext) {
    fullPrompt += `\n\nAdditional context from developer: ${extraContext}`;
  }

  const aiCmd =
    process.env.GIT_UPLOAD_AI_CMD ||
    'copilot -s --model gpt-5.1-codex --deny-tool write --deny-tool shell -p "$GIT_UPLOAD_AI_PROMPT"';

  const aiOutput = await new Promise((resolve, reject) => {
    let out = "";
    let err = "";
    const child = spawn("sh", ["-c", aiCmd], {
      env: { ...process.env, GIT_UPLOAD_AI_PROMPT: fullPrompt },
      cwd,
    });
    child.stdout.on("data", (data) => {
      out += data;
    });
    child.stderr.on("data", (data) => {
      err += data;
    });
    child.on("close", (code) => {
      if (code !== 0 || !out.trim()) {
        reject(
          new Error(
            `AI message generation failed (exit ${code}): ${err.trim()}`,
          ),
        );
      } else {
        resolve(out);
      }
    });
  });

  const lines = aiOutput.split("\n");
  let capturing = false;
  const messageLines = [];
  for (const line of lines) {
    if (line.trim() === "COMMIT_BEGIN") {
      capturing = true;
      continue;
    }
    if (line.trim() === "COMMIT_END") {
      break;
    }
    if (capturing) {
      messageLines.push(line);
    }
  }

  const commitMessage = messageLines.join("\n").trim();
  if (!commitMessage) {
    throw new Error(
      "Could not parse AI output. Use the message parameter to provide a manual message.",
    );
  }
  return commitMessage;
}

async function handleCheckpoint(args) {
  let cwd;
  if (args.cwd) {
    try {
      cwd = await execGit(["rev-parse", "--show-toplevel"], args.cwd);
    } catch {
      throw new Error(
        `Specified cwd is not inside a git repository: ${args.cwd}`,
      );
    }
  } else if (args.branch) {
    const wtPath = worktreePath(args.branch);
    if (fs.existsSync(wtPath)) {
      try {
        const wtBranch = await execGit(
          ["symbolic-ref", "--short", "HEAD"],
          wtPath,
        );
        if (wtBranch === args.branch) {
          cwd = await execGit(["rev-parse", "--show-toplevel"], wtPath);
        }
      } catch {
        // Invalid worktree — fall through.
      }
    }
    if (!cwd) {
      cwd = await resolveRepoRoot();
    }
  } else {
    cwd = await resolveRepoRoot();
  }

  let currentBranch = "";
  try {
    currentBranch = await execGit(["symbolic-ref", "--short", "HEAD"], cwd);
  } catch {
    // Detached HEAD — leave branch empty.
  }

  if (args.branch) {
    if (!currentBranch) {
      throw new Error(
        `Branch assertion failed: HEAD is detached, expected branch '${args.branch}'. Switch to the correct branch before checkpointing.`,
      );
    }
    if (currentBranch !== args.branch) {
      throw new Error(
        `Branch assertion failed: HEAD is on '${currentBranch}', expected '${args.branch}'. Switch to the correct branch before checkpointing.`,
      );
    }
  }

  let enabled = "true";
  try {
    enabled = await execGit(["config", "--get", "checkpoint.enabled"], cwd);
  } catch {
    // Not configured — default true.
  }
  if (enabled === "false") {
    return [
      {
        type: "text",
        text: "[no-op] Checkpoint is disabled for this repo. Use `git checkpoint --enable` to turn on.",
      },
    ];
  }

  if (args.all !== false) {
    await execGit(["add", "-A"], cwd);
  }

  try {
    await execGit(["diff", "--cached", "--quiet"], cwd);
    return [{ type: "text", text: "Nothing to commit — working tree clean." }];
  } catch {
    // There are staged changes.
  }

  let message = String(args.message || "").trim();
  if (!message) {
    message = await generateAiCommitMessage(
      cwd,
      String(args.context || "").trim(),
    );
  }

  const commitArgs = ["commit", "-m", message];
  let signCommits = "false";
  try {
    signCommits = await execGit(["config", "--get", "checkpoint.sign"], cwd);
  } catch {
    // Not configured.
  }
  if (signCommits === "true") {
    commitArgs.push("-S");
  }

  await execGit(commitArgs, cwd);

  const commitHash = await execGit(["rev-parse", "--short", "HEAD"], cwd);
  const oneline = await execGit(["log", "--oneline", "-1"], cwd);
  const stat = await execGit(["diff", "--stat", "HEAD~1..HEAD"], cwd);

  let pushResult = "";
  const doPush = args.push === true;
  let pushDefault = "false";
  try {
    pushDefault = await execGit(["config", "--get", "checkpoint.push"], cwd);
  } catch {
    // Not configured.
  }
  if (doPush || pushDefault === "true") {
    try {
      await execGit(["push"], cwd);
      pushResult = "\nPushed to remote.";
    } catch (err) {
      pushResult = `\nPush failed: ${err.message}`;
    }
  }

  const branchInfo = currentBranch
    ? ` on branch '${currentBranch}'`
    : " (detached HEAD)";

  if (currentBranch) {
    notifyBranchCommit(currentBranch, commitHash, cwd);
  }

  return [
    {
      type: "text",
      text: `Committed ${commitHash}${branchInfo}\n${oneline}\n\n${stat}${pushResult}`,
    },
  ];
}

module.exports = { CHECKPOINT_TOOL, handleCheckpoint };
