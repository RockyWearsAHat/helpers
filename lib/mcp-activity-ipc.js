// lib/mcp-activity-ipc.js — IPC client for notifying the VS Code extension
// of active tool calls, branch session events, and session pulses.
"use strict";

const path = require("path");
const fs = require("fs");
const net = require("net");

const ACTIVITY_IPC_INFO_PATH = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".cache",
  "gsh",
  "activity-ipc.json",
);

let _activitySocket = null;
let _activityConnecting = false;
let _activityNotifySeq = 0;
let _activityQueue = [];

function connectActivityIpc() {
  if (_activitySocket || _activityConnecting) return;
  let socketPath;
  try {
    const info = JSON.parse(fs.readFileSync(ACTIVITY_IPC_INFO_PATH, "utf8"));
    socketPath = info.socketPath;
  } catch {
    return;
  }
  if (!socketPath) return;
  _activityConnecting = true;
  const sock = net.createConnection(socketPath);
  sock.on("connect", () => {
    _activitySocket = sock;
    _activityConnecting = false;
    const queued = _activityQueue.splice(0);
    for (const line of queued) {
      try {
        sock.write(line);
      } catch {
        _activitySocket = null;
        break;
      }
    }
  });
  sock.on("error", () => {
    _activitySocket = null;
    _activityConnecting = false;
    _activityQueue = [];
  });
  sock.on("close", () => {
    _activitySocket = null;
    _activityConnecting = false;
  });
}

function _sendIpc(line) {
  if (_activitySocket) {
    try {
      _activitySocket.write(line);
    } catch {
      _activitySocket = null;
      return false;
    }
    return true;
  }
  if (_activityConnecting) {
    _activityQueue.push(line);
    return true;
  }
  return false;
}

function formatActivityLabel(toolName, args) {
  if (toolName === "checkpoint") return "Checkpoint";
  if (toolName === "search_web") {
    return args?.query
      ? `Search: ${String(args.query).substring(0, 40)}`
      : "Search Web";
  }
  if (toolName === "scrape_webpage") {
    try {
      return `Scrape: ${new URL(args.url).hostname}`;
    } catch {
      return "Scrape Webpage";
    }
  }
  if (
    toolName === "search_knowledge_index" ||
    toolName === "search_knowledge_cache"
  ) {
    return args?.query
      ? `Find: ${String(args.query).substring(0, 35)}`
      : "Search Knowledge";
  }
  if (toolName === "read_knowledge_note")
    return `Read: ${args?.filename || "note"}`;
  if (toolName === "write_knowledge_note")
    return `Write: ${args?.filename || "note"}`;
  if (toolName === "update_knowledge_note")
    return `Update: ${args?.filename || "note"}`;
  if (toolName === "append_to_knowledge_note")
    return `Append: ${args?.filename || "note"}`;
  if (toolName === "build_knowledge_index") return "Rebuild Index";
  if (toolName === "take_screenshot") return "Screenshot";
  if (toolName === "analyze_images") return "Analyze Images";
  if (toolName === "analyze_video") return "Analyze Video";
  if (toolName === "submit_community_research") return "Submit Research";
  return toolName.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function notifyActivityBegin(toolName, args) {
  connectActivityIpc();
  const id = `ext-${++_activityNotifySeq}`;
  const line =
    JSON.stringify({
      type: "activityBegin",
      id,
      tool: toolName,
      label: formatActivityLabel(toolName, args),
      args,
    }) + "\n";
  if (_sendIpc(line)) return id;
  return null;
}

function notifyActivityEnd(id) {
  if (!id || !_activitySocket) return;
  try {
    _activitySocket.write(JSON.stringify({ type: "activityEnd", id }) + "\n");
  } catch {
    _activitySocket = null;
  }
}

function notifySessionPulse() {
  connectActivityIpc();
  _sendIpc(JSON.stringify({ type: "sessionPulse" }) + "\n");
}

function notifyWorktreeCreated(
  branch,
  worktreePath,
  baseBranch,
  baseCommit,
  activityId,
) {
  connectActivityIpc();
  _sendIpc(
    JSON.stringify({
      type: "worktreeCreated",
      branch,
      worktreePath,
      baseBranch,
      baseCommit,
      activityId: activityId || null,
    }) + "\n",
  );
}

function notifyWorktreeRemoved(branch, worktreePath) {
  connectActivityIpc();
  _sendIpc(
    JSON.stringify({ type: "worktreeRemoved", branch, worktreePath }) + "\n",
  );
}

function notifyBranchCommit(branch, commitHash, cwd) {
  connectActivityIpc();
  _sendIpc(
    JSON.stringify({ type: "branchCommit", branch, commitHash, cwd }) + "\n",
  );
}

// Attempt initial connection on startup
connectActivityIpc();

module.exports = {
  connectActivityIpc,
  notifyActivityBegin,
  notifyActivityEnd,
  notifySessionPulse,
  notifyWorktreeCreated,
  notifyWorktreeRemoved,
  notifyBranchCommit,
};
