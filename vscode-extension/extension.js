// Git Shell Helpers — VS Code extension
//
// Provides a "Community Cache" webview panel in the Explorer sidebar with
// styled buttons for GitHub sign-in/out, mode selection, and repo whitelist.
//
// Settings sync:
//   User settings   → ~/.copilot/devops-audit-community-settings.json
//   Workspace settings → .github/devops-audit-community-settings.json

const vscode = require("vscode");
const fs = require("fs");
const net = require("net");
const os = require("os");
const path = require("path");
const { execFile } = require("child_process");

const SCHEMA_VERSION = 1;
const PREDEFINED = {
  baseBranch: "main",
  branchPrefix: "automation/community-cache-submission",
};

let cachedRepos = [];
let cachedUser = "";
let cachedGpgNeedsUpload = false;
let cachedGpgUploadFailed = false;
let cachedModels = [];
let cachedOllamaModels = []; // [] | string[] of model names
let cachedOllamaRunning = false;
let _ollamaPinned = new Set(); // model names the user has enabled/pinned
let activeToolCalls = new Map(); // id → { id, tool, label, startedAt, args }
let _activitySeq = 0;
// Chat sessions are tracked from VS Code's chatSessions JSONL files.
// Detection is content-based: parse JSONL records to check if the last request
// has a modelState.value===1 completion record (no mtime heuristics).
let _chatSessions = new Map(); // sessionId → { title, active, startedAt, completedAt, filePath, lastSize, preview, requestCount }
let _chatSessionWatcher = null;
let _chatSessionPoller = null;
let _context = null;
let _webviewProvider = null;
let _diagnosticsOutputChannel = null;
let _customizationInspectorToolDisposable = null;
let _strictLintIpcServer = null;
let _activityIpcServer = null;
const _externalToInternal = new Map(); // externalId → internalId
const MCP_PROVIDER_ID = "gitShellHelpers.mcpServers";

// ── Module loading ──────────────────────────────────────────────────────────
const createCopilotInspector = require("./src/copilot-inspector");
const createMcpServer = require("./src/mcp-server");
const createGpgAuth = require("./src/gpg-auth");
const createWebviewProviderClass = require("./src/webview-provider");

// ── Community settings and gh CLI helpers ───────────────────────────────
function defaultCommunityRepoFromWorkspace(workspaceFolder) {
  const manifest = readJsonFile(workspaceManifestPath(workspaceFolder));
  return manifest?.defaultCommunityRepo || "";
}

function findLocalCommunityCloneFolder() {
  const folders = vscode.workspace.workspaceFolders || [];
  return (
    folders.find((folder) => fs.existsSync(workspaceManifestPath(folder))) ||
    null
  );
}

// ---------------------------------------------------------------------------
// gh CLI helpers
// ---------------------------------------------------------------------------

function runGh(args, timeout = 30000) {
  return new Promise((resolve, reject) => {
    execFile("gh", args, { timeout }, (err, stdout, stderr) => {
      if (err) reject(new Error(stderr || err.message));
      else resolve(stdout.trim());
    });
  });
}

async function isGhAuthed() {
  try {
    await runGh(["auth", "status"]);
    return true;
  } catch {
    return false;
  }
}

async function getGhUser() {
  try {
    return (await runGh(["api", "user", "--jq", ".login"])) || "";
  } catch {
    return "";
  }
}

async function fetchRepos() {
  try {
    const out = await runGh([
      "repo",
      "list",
      "--limit",
      "200",
      "--json",
      "nameWithOwner,visibility",
      "--jq",
      '.[] | "\\(.nameWithOwner)|\\(.visibility)"',
    ]);
    if (!out) return [];
    return out
      .split("\n")
      .filter(Boolean)
      .map((line) => {
        const [name, vis] = line.split("|");
        return { nameWithOwner: name, visibility: vis };
      });
  } catch {
    return [];
  }
}

// ---------------------------------------------------------------------------
// Whitelist + settings sync
// ---------------------------------------------------------------------------

function getWhitelist() {
  return _context?.globalState.get("whitelistedRepos", []) ?? [];
}

function getMode() {
  return _context?.globalState.get("mode", "disabled") ?? "disabled";
}

async function setMode(mode) {
  await _context?.globalState.update("mode", mode);
  syncAllSettings();
  _webviewProvider?.refresh();
}

async function setWhitelist(repos) {
  await _context?.globalState.update("whitelistedRepos", repos);
  syncAllSettings();
  _webviewProvider?.refresh();
}

function buildSettingsJson() {
  const globalData = readJsonFile(globalSettingsPath()) || {};
  const localCloneFolder = findLocalCommunityCloneFolder();
  const derivedCommunityRepo =
    globalData.communityRepo ||
    (localCloneFolder
      ? defaultCommunityRepoFromWorkspace(localCloneFolder)
      : "") ||
    "RockyWearsAHat/github-shell-helpers";

  return {
    schemaVersion: SCHEMA_VERSION,
    communityRepo: derivedCommunityRepo,
    ...PREDEFINED,
    mode: getMode(),
    whitelistedRepos: getWhitelist(),
    shareResearch: isGroupEnabled("communityResearch"),
    ...(globalData.localClone
      ? { localClone: globalData.localClone }
      : localCloneFolder
        ? { localClone: localCloneFolder.uri.fsPath }
        : {}),
  };
}

function buildWorkspaceSettingsJson(workspaceFolder) {
  const globalSettings = buildSettingsJson();
  const workspaceCommunityRepo =
    defaultCommunityRepoFromWorkspace(workspaceFolder);

  return {
    ...globalSettings,
    ...(workspaceCommunityRepo
      ? { communityRepo: workspaceCommunityRepo }
      : {}),
    ...(fs.existsSync(workspaceManifestPath(workspaceFolder))
      ? { localClone: "." }
      : {}),
  };
}

function syncAllSettings() {
  writeJsonFile(globalSettingsPath(), buildSettingsJson());
  const folders = vscode.workspace.workspaceFolders;
  if (folders) {
    for (const folder of folders) {
      writeJsonFile(
        workspaceSettingsPath(folder),
        buildWorkspaceSettingsJson(folder),
      );
    }
  }
}

function importFromJson() {
  const currentMode = _context?.globalState.get("mode");
  // Migrate legacy "pull-only" → "disabled"
  if (currentMode === "pull-only") {
    _context?.globalState.update("mode", "disabled");
    return;
  }
  if (!currentMode) {
    const globalData = readJsonFile(globalSettingsPath());
    if (globalData?.mode) {
      _context?.globalState.update("mode", globalData.mode);
      if (Array.isArray(globalData.whitelistedRepos)) {
        _context?.globalState.update(
          "whitelistedRepos",
          globalData.whitelistedRepos,
        );
      }
      return;
    }
    const folders = vscode.workspace.workspaceFolders;
    if (folders) {
      for (const folder of folders) {
        const wsData = readJsonFile(workspaceSettingsPath(folder));
        if (wsData?.mode) {
          _context?.globalState.update("mode", wsData.mode);
          if (Array.isArray(wsData.whitelistedRepos)) {
            _context?.globalState.update(
              "whitelistedRepos",
              wsData.whitelistedRepos,
            );
          }
          return;
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Webview panel
// ---------------------------------------------------------------------------

const QUICK_ACTIONS = [
  {
    id: "runAudit",
    label: "Run Audit",
    desc: "Copilot customization audit",
    query: "/copilot-devops-audit",
    // SVG path for a magnifying-glass / audit icon
    iconPath:
      "M10.5 0a5.5 5.5 0 1 1 0 11 5.5 5.5 0 0 1 0-11zm0 1.5a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM.22 14.78a.75.75 0 0 1 0-1.06l4.5-4.5a.75.75 0 1 1 1.06 1.06l-4.5 4.5a.75.75 0 0 1-1.06 0z",
  },
];

const MODES = [
  { value: "disabled", label: "Submissions disabled" },
  { value: "pull-and-auto-submit", label: "Submit from all repos" },
  { value: "auto-submit-only-public", label: "Submit from public repos only" },
  {
    value: "auto-submit-whitelist",
    label: "Submit from whitelisted repos only",
  },
];

// ── MCP tools configuration ──────────────────────────────────────────────
function escapeHtml(text) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

// ---------------------------------------------------------------------------
// MCP Tools config
// ---------------------------------------------------------------------------

const MCP_TOOLS_CONFIG_DIR = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".config",
  "git-shell-helpers-mcp",
);
const MCP_TOOLS_CONFIG_PATH = path.join(MCP_TOOLS_CONFIG_DIR, "tools.json");

// Tool groups: key → { label, description, tools[], alwaysOn? }
const TOOL_GROUPS = [
  {
    key: "knowledgeWrite",
    label: "Write Reusable Knowledge Locally",
    description: "Write, update & append knowledge notes",
    tools: [
      "write_knowledge_note",
      "update_knowledge_note",
      "append_to_knowledge_note",
    ],
  },
  {
    key: "communityResearch",
    label: "Share Knowledge Research",
    description: "Submit knowledge notes to community repo via PR",
    tools: ["submit_community_research"],
  },
  {
    key: "webSearch",
    label: "Web Search",
    description: "Search the web via SearXNG",
    tools: ["search_web"],
  },
  {
    key: "scrapeWebpage",
    label: "Scrape Webpage",
    description: "Fetch pages, strip HTML chrome, return clean text",
    tools: ["scrape_webpage"],
  },
  {
    key: "vision",
    label: "Vision",
    description:
      "Process images in-chat, allowing live analysis of visual output",
    tools: ["analyze_images"],
  },
  {
    key: "screenshot",
    label: "Screenshot",
    description:
      "Capture screenshots of the screen, an app window, or a region",
    tools: ["take_screenshot"],
  },
  {
    key: "checkpoint",
    label: "Git Checkpoint",
    description: "Commit working state via MCP tool — no terminal, no stalling",
    tools: ["checkpoint"],
  },
];

function readToolsConfig() {
  try {
    return JSON.parse(fs.readFileSync(MCP_TOOLS_CONFIG_PATH, "utf8"));
  } catch {
    return { disabledTools: [] };
  }
}

function writeToolsConfig(config) {
  fs.mkdirSync(MCP_TOOLS_CONFIG_DIR, { recursive: true });
  fs.writeFileSync(
    MCP_TOOLS_CONFIG_PATH,
    JSON.stringify(config, null, 2) + "\n",
    "utf8",
  );
}

function isGroupEnabled(groupKey) {
  const group = TOOL_GROUPS.find((g) => g.key === groupKey);
  if (!group || group.alwaysOn) return true;
  const config = readToolsConfig();
  const disabled = config.disabledTools || [];
  return !group.tools.some((t) => disabled.includes(t));
}

function setGroupEnabled(groupKey, enabled) {
  const group = TOOL_GROUPS.find((g) => g.key === groupKey);
  if (!group || group.alwaysOn) return;
  const config = readToolsConfig();
  const disabled = new Set(config.disabledTools || []);
  for (const tool of group.tools) {
    if (enabled) disabled.delete(tool);
    else disabled.add(tool);
  }
  config.disabledTools = [...disabled];
  writeToolsConfig(config);
}

function isStrictLintingEnabled() {
  return isCustomizationInspectorEnabled();
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------


// ── Tool call tracking, chat sessions, models, agents, IPC ────────────
function beginToolCall(tool, label, args) {
  // Cancel linger — a new tool call means the session is still active
  if (_sessionLingerTimer) {
    clearTimeout(_sessionLingerTimer);
    _sessionLingerTimer = null;
  }
  if (!_sessionStartedAt) {
    _sessionStartedAt = Date.now();
  }
  const id = `tc-${++_activitySeq}`;
  activeToolCalls.set(id, {
    id,
    tool,
    label,
    startedAt: Date.now(),
    args: args || {},
  });
  _webviewProvider?.pushUpdate({
    type: "activityUpdate",
    items: getActivityItems(),
  });
  return id;
}

function endToolCall(id) {
  activeToolCalls.delete(id);
  _webviewProvider?.pushUpdate({
    type: "activityUpdate",
    items: getActivityItems(),
  });
}

function getActivityItems() {
  const now = Date.now();
  const items = [];
  // Active tool calls from MCP IPC
  for (const c of activeToolCalls.values()) {
    items.push({
      id: c.id,
      type: "tool",
      label: c.label,
      elapsed: Math.floor((now - c.startedAt) / 1000),
      startedAt: c.startedAt,
      args: JSON.stringify(c.args, null, 2),
    });
  }
  // Collect all sessions, sort by most recent activity, take top 3
  const allSessions = [];
  for (const [sessionId, sess] of _chatSessions) {
    const recency = sess.active
      ? sess.startedAt
      : sess.completedAt || sess.startedAt;
    allSessions.push({ sessionId, recency, ...sess });
  }
  allSessions.sort((a, b) => b.recency - a.recency);
  const top3 = allSessions.slice(0, 3);
  for (const sess of top3) {
    if (sess.active) {
      items.push({
        id: `chat-${sess.sessionId}`,
        type: "session-active",
        label: sess.title,
        elapsed: Math.floor((now - (sess.activeAt || sess.startedAt)) / 1000),
        startedAt: sess.activeAt || sess.startedAt,
        preview: sess.preview || "Working\u2026",
        sessionId: sess.sessionId,
      });
    } else {
      items.push({
        id: `chat-${sess.sessionId}`,
        type: "session-done",
        label: sess.title,
        preview: sess.preview || "",
        sessionId: sess.sessionId,
      });
    }
  }
  return items;
}

function _formatDuration(ms) {
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return rem > 0 ? `${m} min ${rem}s` : `${m} min`;
}

function _formatAgo(ms) {
  const s = Math.round(ms / 1000);
  if (s < 60) return "just now";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.floor(m / 60);
  return `${h} hr ago`;
}

function _activityCountLabel(items) {
  const active = items.filter(
    (i) => i.type === "session-active" || i.type === "tool",
  );
  if (items.length === 0) return "idle";
  if (active.length === 0) return `${items.length} recent`;
  return `${active.length} running`;
}

function _renderActivityItem(item, esc) {
  if (item.type === "session-active") {
    return `
      <div class="activity-item activity-item--session" data-sessionid="${item.sessionId}">
        <div class="activity-row">
          <span class="activity-spinner"></span>
          <span class="activity-title">${esc(item.label)}</span>
          <span class="activity-elapsed" data-started="${item.startedAt}">${item.elapsed}s</span>
        </div>
        ${item.preview ? `<div class="activity-sub">${esc(item.preview)}</div>` : ""}
      </div>`;
  }
  if (item.type === "session-done") {
    return `
      <div class="activity-item activity-item--done" data-sessionid="${item.sessionId}">
        <div class="activity-row">
          <span class="activity-dot-done"></span>
          <span class="activity-title">${esc(item.label)}</span>
          <span class="activity-meta">completed</span>
        </div>
        ${item.preview ? `<div class="activity-sub">${esc(item.preview)}</div>` : ""}
      </div>`;
  }
  // tool call
  return `
    <details class="activity-item">
      <summary class="activity-row">
        <span class="activity-pulse"></span>
        <span class="activity-title">${esc(item.label)}</span>
        <span class="activity-elapsed" data-started="${item.startedAt}">${item.elapsed}s</span>
        <svg class="activity-chevron" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 0 1 1.06 0l3.25 3.25a.75.75 0 0 1 0 1.06l-3.25 3.25a.75.75 0 0 1-1.06-1.06L8.94 8 6.22 5.28a.75.75 0 0 1 0-1.06z"/></svg>
      </summary>
      <div class="activity-detail"><pre>${esc(item.args)}</pre></div>
    </details>`;
}

// ---------------------------------------------------------------------------
// Chat session watcher — reads Copilot's own JSONL files to track activity
// Completion is detected by parsing JSONL content: a session is active when
// the last request index N has no modelState record with value===1 (complete).
// This correctly handles the "thinking" phase where the file is static but
// the LLM is running — mtime/size heuristics fail here.
// ---------------------------------------------------------------------------

function _chatSessionsDir(ctx) {
  // storageUri points to workspaceStorage/<hash>/<extId>/ — the directory may not
  // exist (lazy-created), but its PARENT is the workspace hash dir, and chatSessions
  // is always a sibling there. Check the chatSessions path directly, not storageUri.
  if (ctx?.storageUri?.fsPath) {
    const candidate = path.join(
      path.dirname(ctx.storageUri.fsPath),
      "chatSessions",
    );
    if (fs.existsSync(candidate)) return candidate;
  }
  // Fallback: scan workspaceStorage for the workspace matching the open folder
  const wsStorage = path.join(
    os.homedir(),
    "Library",
    "Application Support",
    "Code",
    "User",
    "workspaceStorage",
  );
  if (!fs.existsSync(wsStorage)) return null;
  // Try to match workspace.json to the currently open folder
  const openFolder = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
  if (openFolder) {
    try {
      for (const d of fs.readdirSync(wsStorage)) {
        const wsjson = path.join(wsStorage, d, "workspace.json");
        const csDir = path.join(wsStorage, d, "chatSessions");
        try {
          const raw = fs.readFileSync(wsjson, "utf8");
          const data = JSON.parse(raw);
          const folder =
            data?.folder ||
            (Array.isArray(data?.folders) && data.folders[0]?.path) ||
            "";
          // folder is a URI like file:///Users/... — compare
          const folderPath = folder.startsWith("file://")
            ? decodeURIComponent(folder.replace(/^file:\/\//, ""))
            : folder;
          if (folderPath === openFolder && fs.existsSync(csDir)) return csDir;
        } catch {}
      }
    } catch {}
  }
  // Last resort: most-recently-modified chatSessions dir
  try {
    const dirs = fs
      .readdirSync(wsStorage)
      .map((d) => path.join(wsStorage, d, "chatSessions"))
      .filter((d) => {
        try {
          return fs.statSync(d).isDirectory();
        } catch {
          return false;
        }
      })
      .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
    if (dirs.length) return dirs[0];
  } catch {}
  return null;
}

function startChatSessionWatcher(ctx) {
  _chatSessionWatcher?.close();
  _chatSessionWatcher = null;
  if (_chatSessionPoller) {
    clearInterval(_chatSessionPoller);
    _chatSessionPoller = null;
  }

  const chatSessionsDir = _chatSessionsDir(ctx);
  if (!chatSessionsDir) return;

  // Scan .jsonl session files and process likely-active candidates.
  // Used as fallback when fs.watch gives null filename (macOS) and for polling.
  let _lastScanMs = 0;
  let _didBootstrapScan = false;
  const _scanRecentFiles = () => {
    const now = Date.now();
    if (now - _lastScanMs < 800) return; // debounce directory scans only
    _lastScanMs = now;
    try {
      const files = fs
        .readdirSync(chatSessionsDir)
        .filter((f) => f.endsWith(".jsonl"))
        .map((f) => {
          const fp = path.join(chatSessionsDir, f);
          let mtimeMs = 0;
          try {
            mtimeMs = fs.statSync(fp).mtimeMs;
          } catch {}
          return { f, fp, sid: f.slice(0, -6), mtimeMs };
        })
        .sort((a, b) => b.mtimeMs - a.mtimeMs);

      // Re-check known active sessions so they can transition to done.
      const candidate = new Map();
      for (const [sid, sess] of _chatSessions) {
        if (sess?.active && sess.filePath) {
          candidate.set(sid, { sid, fp: sess.filePath });
        }
      }

      // Only consider files modified in the last 5 minutes as potentially active.
      // Older files are definitely completed — no model thinks for 5+ minutes
      // without writing anything to the JSONL.
      const recentFiles = files.filter((f) => now - f.mtimeMs < 300000);
      for (const file of recentFiles) {
        candidate.set(file.sid, { sid: file.sid, fp: file.fp });
      }

      for (const c of candidate.values()) {
        _onChatSessionWrite(c.sid, c.fp);
      }
      _didBootstrapScan = true;
    } catch {}
    // Always push update (keeps elapsed time fresh)
    _pushActivityUpdate();
  };

  // Seed activity state immediately
  _scanRecentFiles();

  _chatSessionWatcher = fs.watch(
    chatSessionsDir,
    { persistent: false },
    (_evt, filename) => {
      if (!filename) {
        // macOS: filename is null — scan for recent changes
        _scanRecentFiles();
        return;
      }
      if (!filename.endsWith(".jsonl")) return;
      // Process this specific file IMMEDIATELY — no debounce
      const sessionId = filename.slice(0, -6);
      _onChatSessionWrite(sessionId, path.join(chatSessionsDir, filename));
      _pushActivityUpdate();
    },
  );

  // Poll every 2s — keeps elapsed time fresh and catches events fs.watch misses
  _chatSessionPoller = setInterval(_scanRecentFiles, 2000);
}

function _pushActivityUpdate() {
  _webviewProvider?.pushUpdate({
    type: "activityUpdate",
    items: getActivityItems(),
  });
}

function _chatSessionReadTail(filePath, bytes) {
  // Read last N bytes from the JSONL file
  const readLen = bytes || 65536; // 64 KB default — large sessions need more tail
  try {
    const fd = fs.openSync(filePath, "r");
    try {
      const { size } = fs.fstatSync(fd);
      const actual = Math.min(readLen, size);
      const buf = Buffer.alloc(actual);
      fs.readSync(fd, buf, 0, actual, size - actual);
      return { tail: buf.toString("utf8"), size };
    } finally {
      fs.closeSync(fd);
    }
  } catch {
    return { tail: "", size: 0 };
  }
}

function _chatSessionReadTitle(filePath, existing) {
  if (existing && existing !== "Copilot Chat") return existing;
  try {
    const fd = fs.openSync(filePath, "r");
    try {
      const stat = fs.fstatSync(fd);
      const fileSize = stat.size;
      let customTitle = null;
      let firstPrompt = null;

      // Helper: scan JSONL lines in a buffer for title data
      const scanLines = (buf, len) => {
        for (const line of buf.slice(0, len).toString("utf8").split("\n")) {
          try {
            const rec = JSON.parse(line);
            if (
              rec.kind === 1 &&
              rec.k?.[0] === "customTitle" &&
              typeof rec.v === "string"
            ) {
              customTitle = rec.v;
            }
            if (
              !firstPrompt &&
              rec.kind === 2 &&
              rec.k?.[0] === "requests" &&
              rec.k.length === 1 &&
              Array.isArray(rec.v)
            ) {
              for (const req of rec.v) {
                const msg =
                  req?.message?.text ||
                  req?.message ||
                  req?.text ||
                  req?.prompt;
                if (typeof msg === "string" && msg.trim()) {
                  firstPrompt = msg.trim().slice(0, 80);
                  break;
                }
              }
            }
          } catch {}
        }
      };

      // Pass 1: read first 8KB — covers small sessions where kind=0 snapshot fits
      const headBuf = Buffer.alloc(8192);
      const headN = fs.readSync(fd, headBuf, 0, 8192, 0);
      scanLines(headBuf, headN);
      if (customTitle) return customTitle;

      // Pass 2: if first line is huge (snapshot > 8KB), find first newline and read lines 2-10
      if (headN >= 8192 && !headBuf.slice(0, headN).includes(0x0a)) {
        // Scan forward in 64KB chunks to find the first newline
        const chunkSize = 65536;
        const scanBuf = Buffer.alloc(chunkSize);
        let offset = 8192;
        let nlOffset = -1;
        while (offset < fileSize && offset < 100 * 1024 * 1024) {
          const toRead = Math.min(chunkSize, fileSize - offset);
          const got = fs.readSync(fd, scanBuf, 0, toRead, offset);
          if (got === 0) break;
          const idx = scanBuf.indexOf(0x0a, 0);
          if (idx !== -1 && idx < got) {
            nlOffset = offset + idx;
            break;
          }
          offset += got;
        }
        if (nlOffset !== -1 && nlOffset + 1 < fileSize) {
          // Read 16KB after the first newline (lines 2-N)
          const afterBuf = Buffer.alloc(16384);
          const afterN = fs.readSync(fd, afterBuf, 0, 16384, nlOffset + 1);
          scanLines(afterBuf, afterN);
          if (customTitle) return customTitle;
        }
      }

      // Pass 3: read tail 32KB — catches customTitle written later in session
      if (!customTitle && fileSize > 8192) {
        const tailSize = Math.min(32768, fileSize);
        const tailBuf = Buffer.alloc(tailSize);
        const tailN = fs.readSync(
          fd,
          tailBuf,
          0,
          tailSize,
          fileSize - tailSize,
        );
        scanLines(tailBuf, tailN);
      }

      return customTitle || firstPrompt || "Copilot Chat";
    } finally {
      fs.closeSync(fd);
    }
  } catch {}
  return "Copilot Chat";
}

/** Extract a preview description from the tail of the JSONL file. */
function _chatSessionExtractPreview(tail) {
  const lines = tail.split("\n");
  let lastToolCall = null;
  let lastProgress = null;
  // Walk backwards through parsed lines for the most recent tool invocation or progress
  for (let i = lines.length - 1; i >= 0; i--) {
    try {
      const rec = JSON.parse(lines[i]);
      // Tool call invocation messages appear as response part mutations
      if (rec.kind === 2 || rec.kind === 1) {
        const val = rec.v;
        // Check if this is a response part with tool info
        if (val && typeof val === "object") {
          // invocationMessage from tool calls (e.g. "Reading file.ts")
          if (!lastToolCall && typeof val.invocationMessage === "string") {
            lastToolCall = val.invocationMessage;
          }
          // Progress messages
          if (
            !lastProgress &&
            typeof val.content === "string" &&
            val.kind === "progressMessage"
          ) {
            lastProgress = val.content;
          }
          // Array of response parts
          if (Array.isArray(val)) {
            for (let j = val.length - 1; j >= 0; j--) {
              const part = val[j];
              if (
                !lastToolCall &&
                typeof part?.invocationMessage === "string"
              ) {
                lastToolCall = part.invocationMessage;
              }
              if (
                !lastProgress &&
                typeof part?.content === "string" &&
                part?.kind === "progressMessage"
              ) {
                lastProgress = part.content;
              }
            }
          }
        }
      }
      if (lastToolCall) break; // found what we need
    } catch {}
  }
  return lastToolCall || lastProgress || null;
}

/**
 * Parse the JSONL tail to determine if the last request is still in progress.
 * A session is active when the last request index N has no modelState record
 * with value===1 (completed). This works correctly during the LLM "thinking"
 * phase when the file is static (mtime does not change).
 */
function _chatSessionParseState(tail) {
  const lines = tail.split("\n");
  let lastRequestIdx = -1;
  const doneRequests = new Set(); // requests with a terminal modelState

  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const rec = JSON.parse(line);
      const k = rec.k;
      if (!Array.isArray(k)) continue;

      // Track the highest request index seen from any per-request record
      if (k[0] === "requests" && typeof k[1] === "number") {
        if (k[1] > lastRequestIdx) lastRequestIdx = k[1];
      }
      // kind=2 splice of the TOP-LEVEL requests array tells us a new request was added.
      if (
        rec.kind === 2 &&
        k.length === 1 &&
        k[0] === "requests" &&
        Array.isArray(rec.v)
      ) {
        const spliceEnd = (rec.offset || 0) + rec.v.length - 1;
        if (spliceEnd > lastRequestIdx) lastRequestIdx = spliceEnd;
      }

      // modelState values: 0=Failed, 1=Completed, 2=InProgress, 3=NeedsInput
      // Only value 2 means truly active. Everything else is a terminal state.
      if (
        k[0] === "requests" &&
        typeof k[1] === "number" &&
        k[2] === "modelState" &&
        typeof rec.v?.value === "number"
      ) {
        if (rec.v.value !== 2) {
          doneRequests.add(k[1]);
        }
      }
    } catch {}
  }

  if (lastRequestIdx < 0) return { active: false, lastRequestIdx: -1 };
  return {
    active: !doneRequests.has(lastRequestIdx),
    lastRequestIdx,
  };
}

function _chatSessionReadCreationDate(filePath) {
  // Extract creationDate from the kind=0 snapshot (first line) or early mutation.
  // The snapshot can be huge (multi-MB), so scan bytewise for the key.
  try {
    const fd = fs.openSync(filePath, "r");
    try {
      // Read first 4KB — creationDate is near the start of the snapshot JSON
      const buf = Buffer.alloc(4096);
      const n = fs.readSync(fd, buf, 0, 4096, 0);
      const str = buf.slice(0, n).toString("utf8");
      const m = str.match(/"creationDate"\s*:\s*(\d+)/);
      if (m) return parseInt(m[1], 10);
    } finally {
      fs.closeSync(fd);
    }
  } catch {}
  return null;
}

function _onChatSessionWrite(sessionId, filePath) {
  const existing = _chatSessions.get(sessionId);
  const now = Date.now();

  // Always read the tail — content-based detection, not mtime
  const { tail, size: fileSize } = _chatSessionReadTail(filePath);
  if (!tail) return;

  // Skip if file hasn't changed since last check
  if (existing && existing.lastSize === fileSize && !existing.active) return;

  // Hard mtime guard: if the file hasn't been modified in 5+ minutes, it cannot
  // be an active session. This catches cases where the 64KB tail doesn't contain
  // the completion record for large/old sessions.
  let fileMtimeMs = 0;
  try {
    fileMtimeMs = fs.statSync(filePath).mtimeMs;
  } catch {}
  const fileStaleMs = Date.now() - fileMtimeMs;
  const forceCompleted = fileStaleMs > 300000; // 5 minutes

  const { active: rawActive, lastRequestIdx } = _chatSessionParseState(tail);
  const isActive = rawActive && !forceCompleted;
  const title = _chatSessionReadTitle(filePath, existing?.title);

  // Always extract preview from tail so completed sessions retain their last summary
  const newPreview = _chatSessionExtractPreview(tail);
  let preview = newPreview || existing?.preview || null;

  // Determine startedAt: prefer existing, then creationDate from JSONL, then now
  let startedAt = existing?.startedAt;
  if (!startedAt || (existing && !existing.active && isActive)) {
    startedAt = _chatSessionReadCreationDate(filePath) || now;
  }

  if (isActive) {
    // Staleness guard: if the file hasn't changed in 2+ minutes and the session
    // was already known, treat it as done (the model may have disconnected).
    if (existing && existing.lastSize === fileSize && existing.active) {
      const staleMs = now - (existing._lastChangedAt || existing.startedAt);
      if (staleMs > 120000) {
        _chatSessions.set(sessionId, {
          title,
          active: false,
          startedAt,
          completedAt: existing._lastChangedAt || now,
          filePath,
          sessionId,
          lastSize: fileSize,
          preview: preview || existing?.preview || null,
          requestCount: lastRequestIdx + 1,
          _lastChangedAt: existing._lastChangedAt || now,
        });
        return;
      }
    }
    _chatSessions.set(sessionId, {
      title,
      active: true,
      startedAt,
      completedAt: null,
      filePath,
      sessionId,
      lastSize: fileSize,
      preview: preview || "Working…",
      requestCount: lastRequestIdx + 1,
      _lastChangedAt:
        existing?.lastSize !== fileSize ? now : existing?._lastChangedAt || now,
    });
  } else {
    const completedAt = existing?.active ? now : existing?.completedAt || now;
    _chatSessions.set(sessionId, {
      title,
      active: false,
      startedAt,
      activeAt: null,
      completedAt,
      filePath,
      sessionId,
      lastSize: fileSize,
      preview: preview || existing?.preview || null,
      requestCount: lastRequestIdx + 1,
      _lastChangedAt: now,
    });
  }
}

// ---------------------------------------------------------------------------
// Models — enumerate available Copilot language models
// ---------------------------------------------------------------------------

async function refreshModels() {
  try {
    const models = await vscode.lm.selectChatModels({});
    cachedModels = (models || []).map((m) => ({
      id: m.id,
      name: m.name || m.id,
      vendor: m.vendor || "",
      family: m.family || "",
      version: m.version || "",
      maxInputTokens: m.maxInputTokens || 0,
    }));
    // deduplicate by id
    const seen = new Set();
    cachedModels = cachedModels.filter((m) => {
      if (seen.has(m.id)) return false;
      seen.add(m.id);
      return true;
    });
  } catch {
    cachedModels = [];
  }
  _webviewProvider?.refresh();
}

async function openModelPicker() {
  const commands = await vscode.commands.getCommands(true);
  const exactCandidates = [
    "chat.openLanguageModelPicker",
    "github.copilot.chat.openLanguageModelPicker",
    "workbench.action.chat.openLanguageModelPicker",
    "workbench.action.chat.changeDefaultModel",
    "github.copilot.chat.changeModel",
  ];
  const commandId =
    exactCandidates.find((c) => commands.includes(c)) ||
    commands.find(
      (c) =>
        c.toLowerCase().includes("chat") &&
        (c.toLowerCase().includes("model") ||
          c.toLowerCase().includes("language")) &&
        (c.toLowerCase().includes("pick") ||
          c.toLowerCase().includes("select") ||
          c.toLowerCase().includes("change")),
    );
  if (commandId) {
    await vscode.commands.executeCommand(commandId);
    return;
  }
  // Fallback: open quick-open with a model-related search
  await vscode.commands.executeCommand(
    "workbench.action.quickOpen",
    ">chat model",
  );
}

// ---------------------------------------------------------------------------
// Ollama detection
// ---------------------------------------------------------------------------

const OLLAMA_BASE = "http://127.0.0.1:11434";

async function detectOllama() {
  return new Promise((resolve) => {
    const http = require("http");
    const req = http.request(
      {
        hostname: "127.0.0.1",
        port: 11434,
        path: "/api/tags",
        method: "GET",
        timeout: 2000,
      },
      (res) => {
        let raw = "";
        res.on("data", (c) => {
          raw += c;
        });
        res.on("end", () => {
          try {
            const body = JSON.parse(raw);
            const names = (body.models || [])
              .map((m) => m.name || m.model || "")
              .filter(Boolean);
            cachedOllamaRunning = true;
            cachedOllamaModels = names;
          } catch {
            cachedOllamaRunning = true;
            cachedOllamaModels = [];
          }
          resolve();
        });
      },
    );
    req.on("error", () => {
      cachedOllamaRunning = false;
      cachedOllamaModels = [];
      resolve();
    });
    req.on("timeout", () => {
      req.destroy();
      cachedOllamaRunning = false;
      cachedOllamaModels = [];
      resolve();
    });
    req.end();
  });
}

// ---------------------------------------------------------------------------
// API key helpers (stored in VS Code SecretStorage — never on disk)
// ---------------------------------------------------------------------------

const API_KEY_ANTHROPIC = "gsh.apiKey.anthropic";
const API_KEY_OPENAI = "gsh.apiKey.openai";

async function getApiKey(key) {
  try {
    return (await _context?.secrets.get(key)) || "";
  } catch {
    return "";
  }
}

async function setApiKey(key, value) {
  try {
    if (value) await _context?.secrets.store(key, value);
    else await _context?.secrets.delete(key);
  } catch {
    /* ignore */
  }
}

async function getProviderStatus() {
  const [anthropicKey, openaiKey] = await Promise.all([
    getApiKey(API_KEY_ANTHROPIC),
    getApiKey(API_KEY_OPENAI),
  ]);
  return {
    anthropicKey: anthropicKey ? "set" : "",
    openaiKey: openaiKey ? "set" : "",
    ollamaRunning: cachedOllamaRunning,
    ollamaModels: cachedOllamaModels,
  };
}

// ---------------------------------------------------------------------------
// Local agents — scan and launch .github/agents/*.agent.md files
// ---------------------------------------------------------------------------

function parseAgentFrontmatter(content, fileName) {
  if (!content.startsWith("---")) return null;
  const eod = content.indexOf("\n---", 3);
  if (eod === -1) return null;
  const fm = content.slice(3, eod);
  const nameMatch = fm.match(/^name:\s*(.+)$/m);
  const descMatch = fm.match(/^description:\s*(.+)$/m);
  const invocableMatch = fm.match(/^user-invocable:\s*(true|false)\s*/m);
  const name = nameMatch
    ? nameMatch[1].trim().replace(/^["']|["']$/g, "")
    : fileName.replace(".agent.md", "");
  const description = descMatch
    ? descMatch[1].trim().replace(/^["']|["']$/g, "")
    : "";
  const userInvocable = invocableMatch
    ? invocableMatch[1].trim() !== "false"
    : true;
  return { name, description, userInvocable, fileName };
}

function scanLocalAgents() {
  const agents = [];
  const folders = vscode.workspace.workspaceFolders || [];
  for (const folder of folders) {
    const agentsDir = path.join(folder.uri.fsPath, ".github", "agents");
    if (!fs.existsSync(agentsDir)) continue;
    let files;
    try {
      files = fs.readdirSync(agentsDir).filter((f) => f.endsWith(".agent.md"));
    } catch {
      continue;
    }
    for (const file of files) {
      try {
        const content = fs.readFileSync(path.join(agentsDir, file), "utf8");
        const agent = parseAgentFrontmatter(content, file);
        if (agent) agents.push(agent);
      } catch {
        /* skip */
      }
    }
  }
  return agents.sort((a, b) => a.name.localeCompare(b.name));
}

async function openAgentInChat(agentName) {
  if (!agentName) return;
  try {
    const commands = await vscode.commands.getCommands(true);
    const candidates = [
      "workbench.action.chat.open",
      "workbench.panel.chat.view.copilot.focus",
    ];
    const cmd = candidates.find((c) => commands.includes(c));
    if (cmd) {
      await vscode.commands.executeCommand(cmd, { query: `@${agentName} ` });
      return;
    }
  } catch {
    /* fall through */
  }
  await vscode.commands.executeCommand(
    "workbench.action.quickOpen",
    `@${agentName}`,
  );
}

async function runQuickAction(actionId) {
  const qa = QUICK_ACTIONS.find((a) => a.id === actionId);
  if (!qa) return;
  try {
    const commands = await vscode.commands.getCommands(true);
    if (commands.includes("workbench.action.chat.open")) {
      // Pass the query without isPartialQuery so VS Code submits it immediately
      await vscode.commands.executeCommand("workbench.action.chat.open", {
        query: qa.query,
      });
      return;
    }
    if (commands.includes("workbench.panel.chat.view.copilot.focus")) {
      await vscode.commands.executeCommand(
        "workbench.panel.chat.view.copilot.focus",
        { query: qa.query },
      );
      return;
    }
  } catch {
    /* fall through */
  }
  await vscode.commands.executeCommand("workbench.action.quickOpen", qa.query);
}

async function openQuickActionWithoutSend(actionId) {
  const qa = QUICK_ACTIONS.find((a) => a.id === actionId);
  if (!qa) return;
  try {
    const commands = await vscode.commands.getCommands(true);
    if (commands.includes("workbench.action.chat.open")) {
      await vscode.commands.executeCommand("workbench.action.chat.open", {
        query: qa.query,
        isPartialQuery: true,
      });
      return;
    }
    if (commands.includes("workbench.panel.chat.view.copilot.focus")) {
      await vscode.commands.executeCommand(
        "workbench.panel.chat.view.copilot.focus",
        { query: qa.query, isPartialQuery: true },
      );
      return;
    }
  } catch {
    /* fall through */
  }
  // Last resort: copy to clipboard and notify
  await vscode.env.clipboard.writeText(qa.query);
  vscode.window.showInformationMessage(
    `Copied "${qa.query}" to clipboard — paste it into a new chat.`,
  );
}

// ---------------------------------------------------------------------------
// Checkpoint settings → git config sync
// ---------------------------------------------------------------------------

function syncCheckpointSettings() {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) return;

  const config = vscode.workspace.getConfiguration(
    "gitShellHelpers.checkpoint",
  );
  const keys = [
    { setting: "enabled", gitKey: "checkpoint.enabled" },
    { setting: "autoPush", gitKey: "checkpoint.push" },
    { setting: "sign", gitKey: "checkpoint.sign" },
  ];

  for (const folder of folders) {
    const cwd = folder.uri.fsPath;
    for (const { setting, gitKey } of keys) {
      const value = config.get(setting);
      if (value !== undefined) {
        execFile("git", ["config", gitKey, String(value)], { cwd }, (err) => {
          if (err) {
            // Not a git repo or git not available — ignore silently
          }
        });
      }
    }
  }
}


// ── Module wiring ───────────────────────────────────────────────────────────
const inspector = createCopilotInspector({
  getDiagnosticsChannel: () => _diagnosticsOutputChannel,
  setDiagnosticsChannel: (ch) => { _diagnosticsOutputChannel = ch; },
  getInspectorDisposable: () => _customizationInspectorToolDisposable,
  setInspectorDisposable: (d) => { _customizationInspectorToolDisposable = d; },
  beginToolCall,
  endToolCall,
});
const {
  uniquePaths, getFrontmatterRange, getFrontmatterListEntries,
  formatHoverContents, makeToolResult, formatDiagnosticSeverity,
  isCustomizationInspectorEnabled, formatCustomizationInspectionReport,
  resolveCustomizationDocument, inspectCopilotCustomizationWarnings,
  runStrictLinting, registerCustomizationInspectorTool,
  getDiagnosticsOutputChannel,
} = inspector;

const mcpServer = createMcpServer({
  GLOBAL_MCP_SERVER_PATH,
  MCP_PROVIDER_ID,
  uniquePaths,
});
const {
  findGitShellHelpersMcpPath, buildGitShellHelpersMcpEnv,
  registerMcpServerProvider, globalSettingsPath, workspaceSettingsPath,
  workspaceManifestPath, readJsonFile, writeJsonFile,
  userMcpConfigPath, workspaceMcpConfigPaths,
  removeStaticGitShellHelpersServers, migrateLegacyMcpRegistrations,
  getConfiguredGitShellHelpersMcpServer, getMcpStatusViewModel,
  openMcpServerControls,
} = mcpServer;

const gpgAuth = createGpgAuth({
  getCachedRepos: () => cachedRepos,
  setCachedRepos: (v) => { cachedRepos = v; },
  getCachedUser: () => cachedUser,
  setCachedUser: (v) => { cachedUser = v; },
  getCachedGpgNeedsUpload: () => cachedGpgNeedsUpload,
  setCachedGpgNeedsUpload: (v) => { cachedGpgNeedsUpload = v; },
  getCachedGpgUploadFailed: () => cachedGpgUploadFailed,
  setCachedGpgUploadFailed: (v) => { cachedGpgUploadFailed = v; },
  getWebviewProvider: () => _webviewProvider,
  runGh, isGhAuthed, getGhUser, fetchRepos,
  getWhitelist, getMode, setWhitelist, buildSettingsJson, syncAllSettings,
  readJsonFile, writeJsonFile, globalSettingsPath, workspaceSettingsPath,
  SCHEMA_VERSION, PREDEFINED,
});
const {
  loginGitHub, logoutGitHub, selectRepos, showCommunityStatus,
  checkGpgUploadStatus, uploadGpgKeyNow, ensureGpgAvailable, ensureGpgKey,
} = gpgAuth;

const CommunityCacheViewProvider = createWebviewProviderClass({
  loginGitHub, logoutGitHub, selectRepos, setMode, setGroupEnabled,
  ensureGpgKey, openMcpServerControls, openModelPicker, refreshModels,
  openAgentInChat, runQuickAction, openQuickActionWithoutSend,
  setApiKey, detectOllama, uploadGpgKeyNow,
  getMode, getWhitelist, getMcpStatusViewModel,
  escapeHtml, isGroupEnabled, isStrictLintingEnabled,
  getProviderStatus, scanLocalAgents, getActivityItems,
  API_KEY_ANTHROPIC, API_KEY_OPENAI, TOOL_GROUPS, MODES, QUICK_ACTIONS,
  get _ollamaPinned() { return _ollamaPinned; },
  get _context() { return _context; },
  getCachedUser: () => cachedUser,
  setCachedUser: (v) => { cachedUser = v; },
  getCachedRepos: () => cachedRepos,
  setCachedRepos: (v) => { cachedRepos = v; },
  getCachedGpgNeedsUpload: () => cachedGpgNeedsUpload,
  getCachedGpgUploadFailed: () => cachedGpgUploadFailed,
  setCachedGpgUploadFailed: (v) => { cachedGpgUploadFailed = v; },
});

// ── Activation ──────────────────────────────────────────────────────────
function activate(context) {
  _context = context;

  // Restore persisted Ollama pinned models
  const savedPinned = context.globalState.get("gsh.ollama.pinned", []);
  _ollamaPinned = new Set(Array.isArray(savedPinned) ? savedPinned : []);

  importFromJson();
  migrateLegacyMcpRegistrations();
  registerMcpServerProvider(context);
  registerCustomizationInspectorTool(context);

  // Git Helpers webview (MCP Tools + Community Cache)
  _webviewProvider = new CommunityCacheViewProvider(context.extensionUri);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      CommunityCacheViewProvider.viewType,
      _webviewProvider,
    ),
  );

  // On first activation, focus the Git Helpers panel so users discover it
  const seenKey = "gitHelpers.introduced.v3";
  if (!context.globalState.get(seenKey)) {
    context.globalState.update(seenKey, true);
    setTimeout(() => {
      vscode.commands.executeCommand("gitShellHelpers.communityCache.focus");
    }, 800);
  }

  // Auto-detect gh auth on startup
  isGhAuthed().then(async (authed) => {
    if (authed) {
      cachedUser = await getGhUser();
      cachedRepos = await fetchRepos();
      await checkGpgUploadStatus();
      _webviewProvider.refresh();
    }
  });

  // Detect Ollama on startup
  detectOllama();

  // Load available Copilot models on startup and whenever the model list changes
  refreshModels();
  if (vscode.lm?.onDidChangeChatModels) {
    context.subscriptions.push(
      vscode.lm.onDidChangeChatModels(() => refreshModels()),
    );
  }

  // Start strict lint IPC server so the gsh MCP tool can query VS Code diagnostics
  startStrictLintIpcServer();
  // Start activity IPC server so the gsh MCP server can report active tool calls
  startActivityIpcServer();
  // Watch Copilot Chat's JSONL session files for live activity.
  // The end-of-response marker is pendingRequests:null written to the JSONL.
  startChatSessionWatcher(context);
  context.subscriptions.push({
    dispose: () => {
      _chatSessionWatcher?.close();
      _chatSessionWatcher = null;
      if (_chatSessionPoller) {
        clearInterval(_chatSessionPoller);
        _chatSessionPoller = null;
      }
    },
  });

  // Write default tools config if none exists
  if (!fs.existsSync(MCP_TOOLS_CONFIG_PATH)) {
    writeToolsConfig({ disabledTools: [] });
  }

  context.subscriptions.push(
    vscode.commands.registerCommand(
      "gitShellHelpers.showCommunityStatus",
      showCommunityStatus,
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.inspectCopilotCustomizationWarnings",
      async (filePath) => {
        const result = await inspectCopilotCustomizationWarnings({
          filePath,
          notify: true,
          revealOutput: true,
        });
        return formatCustomizationInspectionReport(result);
      },
    ),
    vscode.commands.registerCommand("gitShellHelpers.loginGitHub", loginGitHub),
    vscode.commands.registerCommand(
      "gitShellHelpers.logoutGitHub",
      logoutGitHub,
    ),
    vscode.commands.registerCommand("gitShellHelpers.selectRepos", selectRepos),
    vscode.commands.registerCommand(
      "gitShellHelpers.restartMcpServer",
      async () => {
        const choice = await vscode.window.showInformationMessage(
          "Reload the window now to restart MCP servers and refresh chat tools?",
          "Reload Window",
          "Cancel",
        );
        if (choice === "Reload Window") {
          await vscode.commands.executeCommand("workbench.action.reloadWindow");
        }
      },
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.openMcpServerControls",
      openMcpServerControls,
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.refreshModels",
      async () => {
        await refreshModels();
        vscode.window.showInformationMessage(
          `Git Shell Helpers: ${cachedModels.length} Copilot model(s) found.`,
        );
      },
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.openModelPicker",
      openModelPicker,
    ),
  );

  // Sync checkpoint settings to git config when changed
  syncCheckpointSettings();
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("gitShellHelpers.checkpoint")) {
        syncCheckpointSettings();
      }
      if (e.affectsConfiguration("gitShellHelpers.customizationInspector")) {
        registerCustomizationInspectorTool(context);
      }
    }),
  );
}

// ---------------------------------------------------------------------------
// Strict Lint IPC server — allows the gsh MCP server to request diagnostics
// from VS Code's live language servers via a Unix socket.
// ---------------------------------------------------------------------------

const STRICT_LINT_SOCKET_PATH = path.join(os.tmpdir(), "gsh-strict-lint.sock");
const STRICT_LINT_IPC_INFO_PATH = path.join(
  os.homedir(),
  ".cache",
  "gsh",
  "strict-lint-ipc.json",
);

function startStrictLintIpcServer() {
  if (_strictLintIpcServer) return;

  try {
    if (fs.existsSync(STRICT_LINT_SOCKET_PATH)) {
      fs.unlinkSync(STRICT_LINT_SOCKET_PATH);
    }
  } catch {
    // ignore
  }

  try {
    fs.mkdirSync(path.dirname(STRICT_LINT_IPC_INFO_PATH), { recursive: true });
    fs.writeFileSync(
      STRICT_LINT_IPC_INFO_PATH,
      JSON.stringify(
        {
          socketPath: STRICT_LINT_SOCKET_PATH,
          updatedAt: new Date().toISOString(),
        },
        null,
        2,
      ),
      "utf8",
    );
  } catch {
    // ignore — non-fatal
  }

  _strictLintIpcServer = net.createServer((socket) => {
    let buffer = "";
    socket.setEncoding("utf8");

    socket.on("data", async (chunk) => {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      for (const line of lines) {
        if (!line.trim()) continue;

        let request;
        try {
          request = JSON.parse(line);
        } catch {
          socket.write(
            JSON.stringify({ ok: false, error: "Invalid JSON" }) + "\n",
          );
          continue;
        }

        try {
          const callId = beginToolCall(
            "strict-lint-mcp",
            `MCP Strict Lint: ${request.arguments?.filePath ? path.basename(request.arguments.filePath) : "workspace"}`,
            request.arguments || {},
          );
          try {
            const result = await runStrictLinting(request.arguments || {});
            socket.write(JSON.stringify({ ok: true, result }) + "\n");
          } finally {
            endToolCall(callId);
          }
        } catch (err) {
          socket.write(
            JSON.stringify({
              ok: false,
              error: err instanceof Error ? err.message : String(err),
            }) + "\n",
          );
        }
      }
    });

    socket.on("error", () => {});
  });

  _strictLintIpcServer.listen(STRICT_LINT_SOCKET_PATH);
  _strictLintIpcServer.on("error", () => {
    _strictLintIpcServer = null;
  });
}

function stopStrictLintIpcServer() {
  if (_strictLintIpcServer) {
    _strictLintIpcServer.close();
    _strictLintIpcServer = null;
  }
  try {
    fs.unlinkSync(STRICT_LINT_SOCKET_PATH);
  } catch {
    // ignore
  }
  try {
    fs.unlinkSync(STRICT_LINT_IPC_INFO_PATH);
  } catch {
    // ignore
  }
}

const ACTIVITY_SOCKET_PATH = path.join(os.tmpdir(), "gsh-activity.sock");
const ACTIVITY_IPC_INFO_PATH = path.join(
  os.homedir(),
  ".cache",
  "gsh",
  "activity-ipc.json",
);

function startActivityIpcServer() {
  if (_activityIpcServer) return;

  try {
    if (fs.existsSync(ACTIVITY_SOCKET_PATH)) {
      fs.unlinkSync(ACTIVITY_SOCKET_PATH);
    }
  } catch {
    // ignore
  }

  try {
    fs.mkdirSync(path.dirname(ACTIVITY_IPC_INFO_PATH), { recursive: true });
    fs.writeFileSync(
      ACTIVITY_IPC_INFO_PATH,
      JSON.stringify(
        {
          socketPath: ACTIVITY_SOCKET_PATH,
          updatedAt: new Date().toISOString(),
        },
        null,
        2,
      ),
      "utf8",
    );
  } catch {
    // ignore — non-fatal
  }

  _activityIpcServer = net.createServer((socket) => {
    let buffer = "";
    socket.setEncoding("utf8");
    socket.on("data", (chunk) => {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";
      for (const line of lines) {
        if (!line.trim()) continue;
        let msg;
        try {
          msg = JSON.parse(line);
        } catch {
          continue;
        }
        if (msg.type === "activityBegin" && msg.id) {
          const internalId = beginToolCall(
            msg.tool || "mcp",
            msg.label || msg.tool || "MCP Tool",
            msg.args || {},
          );
          _externalToInternal.set(msg.id, internalId);
        } else if (msg.type === "activityEnd" && msg.id) {
          const internalId = _externalToInternal.get(msg.id);
          if (internalId) {
            _externalToInternal.delete(msg.id);
            endToolCall(internalId);
          }
        } else if (msg.type === "sessionPulse") {
          // Agent turn starting — begin or refresh the session linger
          if (_sessionLingerTimer) {
            clearTimeout(_sessionLingerTimer);
            _sessionLingerTimer = null;
          }
          if (!_sessionStartedAt) {
            _sessionStartedAt = Date.now();
          }
          _sessionLingerTimer = setTimeout(() => {
            _sessionLingerTimer = null;
            _sessionStartedAt = 0;
            _webviewProvider?.pushUpdate({
              type: "activityUpdate",
              items: getActivityItems(),
            });
          }, SESSION_LINGER_MS);
          _webviewProvider?.pushUpdate({
            type: "activityUpdate",
            items: getActivityItems(),
          });
        }
      }
    });
    socket.on("error", () => {});
  });

  _activityIpcServer.listen(ACTIVITY_SOCKET_PATH);
  _activityIpcServer.on("error", () => {
    _activityIpcServer = null;
  });
}

function stopActivityIpcServer() {
  if (_activityIpcServer) {
    _activityIpcServer.close();
    _activityIpcServer = null;
  }
  try {
    fs.unlinkSync(ACTIVITY_SOCKET_PATH);
  } catch {
    // ignore
  }
  try {
    fs.unlinkSync(ACTIVITY_IPC_INFO_PATH);
  } catch {
    // ignore
  }
  _externalToInternal.clear();
}

function deactivate() {
  stopStrictLintIpcServer();
  stopActivityIpcServer();
}

module.exports = { activate, deactivate };
