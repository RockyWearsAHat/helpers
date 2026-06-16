#!/usr/bin/env node
"use strict";

// Persist V8 bytecode across runs so module compilation isn't repeated on every
// cold start. No-op on Node < 22.8. Keeps the cache keyed by Node version, so a
// runtime upgrade transparently rebuilds it. This is the cheapest boot win.
try {
  require("module").enableCompileCache?.();
} catch {
  /* older Node — ignore */
}

const path = require("path");
const readline = require("readline");
const {
  isNativeTool,
  loadNativeSchemas,
  runNativeTool,
} = require("./lib/mcp-native");
const {
  BRANCH_SESSION_TOOLS,
  handleBranchSessionStart,
  handleBranchSessionEnd,
  handleBranchReadFile,
  handleBranchStatus,
  handleBranchCleanup,
} = require("./lib/mcp-branch-sessions");
const {
  LIST_LANGUAGE_MODELS_TOOL,
  handleListLanguageModels,
} = require("./lib/mcp-language-models");
const {
  LOCAL_SUBAGENT_TOOLS,
  createLocalSubagentHandler,
} = require("./lib/mcp-local-subagents");
const {
  REGISTER_WORKSPACE_TOOL,
  RELOAD_WINDOW_READY_TOOL,
  UNREGISTER_WORKSPACE_TOOL,
  getUserToolSchemas,
  isUserTool,
  executeUserTool,
  handleRegisterWorkspaceTool,
  handleReloadWindowReady,
  handleUnregisterWorkspaceTool,
} = require("./lib/mcp-user-tools");
const {
  notifyActivityBegin,
  notifyActivityEnd,
  notifySessionPulse,
  notifyBranchCommit,
} = require("./lib/mcp-activity-ipc");

const MCP_VERSION = "2024-11-05";
const SERVER_VERSION = "1.1.0";
const TOOLS_CONFIG_PATH = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".config",
  "git-shell-helpers-mcp",
  "tools.json",
);

// Per-connection output target. In stdio mode there is none and we write to
// stdout; in daemon mode each connection runs inside outputStore.run({write})
// so responses go to the right socket even across awaits.
const { AsyncLocalStorage } = require("async_hooks");
const outputStore = new AsyncLocalStorage();

function send(message) {
  const line = `${JSON.stringify(message)}\n`;
  const store = outputStore.getStore();
  if (store && store.write) store.write(line);
  else process.stdout.write(line);
}

function sendError(id, code, message) {
  send({
    jsonrpc: "2.0",
    id,
    error: { code, message },
  });
}

function loadResearchModule() {
  if (process.env.GIT_SHELL_HELPERS_MCP_DISABLE_RESEARCH) {
    return null;
  }
  try {
    const createResearch = require("./lib/mcp-research");
    const {
      RESEARCH_TOOLS,
      createHandler,
    } = require("./lib/mcp-research-tools");
    return {
      tools: RESEARCH_TOOLS,
      handler: createHandler(createResearch()),
    };
  } catch (err) {
    process.stderr.write(
      `[git-shell-helpers-mcp] WARNING: could not load research modules: ${err.message}\n`,
    );
    return null;
  }
}

function loadVisionModule() {
  if (process.env.GIT_SHELL_HELPERS_MCP_DISABLE_VISION) {
    return null;
  }
  try {
    const vision = require("./vision-tool/mcp-server.js");
    return {
      tools: vision.tools || [],
      handler: vision.handleToolCall,
    };
  } catch (err) {
    process.stderr.write(
      `[git-shell-helpers-mcp] WARNING: could not load vision-tool: ${err.message}\n`,
    );
    return null;
  }
}

const researchModule = loadResearchModule();
const visionModule = loadVisionModule();
const localSubagentHandler = process.env.GSH_DISABLE_LOCAL_SUBAGENTS
  ? null
  : createLocalSubagentHandler({
      researchHandler: researchModule?.handler || null,
    });
const delegatedHandlers = [
  researchModule?.handler,
  visionModule?.handler,
  localSubagentHandler,
].filter(Boolean);

// Schemas for the tools implemented in the native Rust binary. Loaded once at
// startup; throws (fail-loud) if the required binary is missing or broken.
const NATIVE_SCHEMAS = loadNativeSchemas();

const ALL_TOOLS = [
  ...(researchModule?.tools || []),
  ...(visionModule?.tools || []),
  ...NATIVE_SCHEMAS,
  LIST_LANGUAGE_MODELS_TOOL,
  REGISTER_WORKSPACE_TOOL,
  RELOAD_WINDOW_READY_TOOL,
  UNREGISTER_WORKSPACE_TOOL,
  ...(process.env.GSH_DISABLE_BRANCH_SESSIONS ? [] : BRANCH_SESSION_TOOLS),
  ...(process.env.GSH_DISABLE_LOCAL_SUBAGENTS ? [] : LOCAL_SUBAGENT_TOOLS),
];

function getWorkspaceRoot() {
  const raw = process.env.GSH_WORKSPACE_ROOTS || "";
  if (raw.trim().startsWith("[")) {
    try {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed[0];
    } catch {}
  }
  return raw.split(",").filter(Boolean)[0] || process.cwd();
}

function readToolsConfig() {
  try {
    return JSON.parse(require("fs").readFileSync(TOOLS_CONFIG_PATH, "utf8"));
  } catch {
    return {};
  }
}

// Master kill-switch. When `disabled` is true in tools.json, GSH is bypassed:
// the server advertises no tools and refuses every call (except `force`).
// This lets `gsh disable` / `gsh bypass` toggle the entire surface live —
// the config is re-read on every request, so no restart is needed.
function isGshDisabled() {
  // `gsh tool list` sets GSH_FORCE_ENABLE=1 to enumerate the full universe of
  // tools even while GSH is bypassed.
  if (process.env.GSH_FORCE_ENABLE === "1") return false;
  return readToolsConfig().disabled === true;
}

function getEnabledTools() {
  const userTools = getUserToolSchemas(getWorkspaceRoot());
  const allWithUser = [...ALL_TOOLS, ...userTools];
  if (process.env.GSH_FORCE_ENABLE === "1") return allWithUser;
  if (isGshDisabled()) return [];
  const disabled = new Set(readToolsConfig().disabledTools || []);
  if (disabled.size === 0) return allWithUser;
  return allWithUser.filter((tool) => !disabled.has(tool.name));
}

function isToolDisabled(toolName) {
  if (isGshDisabled()) return true;
  return (readToolsConfig().disabledTools || []).includes(toolName);
}

// After a native checkpoint commits, re-emit the VS Code branch-commit IPC the
// JS checkpoint used to send (the native binary stays decoupled from the editor
// socket). Parses the committed hash + branch from the tool's text output.
function maybeNotifyBranchCommit(content, args) {
  try {
    const text = (content && content[0] && content[0].text) || "";
    const m = text.match(/^Committed (\S+) on branch '([^']+)'/);
    if (!m) return;
    notifyBranchCommit(m[2], m[1], args.cwd || getWorkspaceRoot());
  } catch {
    /* notification is best-effort */
  }
}

async function runBuiltInTool(toolName, toolArguments, activityId) {
  // Native Rust tools take precedence — the daemon shells out to the binary.
  if (isNativeTool(toolName)) {
    const content = await runNativeTool(toolName, toolArguments);
    if (toolName === "checkpoint") maybeNotifyBranchCommit(content, toolArguments);
    return content;
  }
  if (toolName === "list_language_models") {
    return handleListLanguageModels();
  }
  if (!process.env.GSH_DISABLE_BRANCH_SESSIONS) {
    if (toolName === "branch_session_start") {
      return handleBranchSessionStart(toolArguments, activityId);
    }
    if (toolName === "branch_session_end") {
      return handleBranchSessionEnd(toolArguments);
    }
    if (toolName === "branch_read_file") {
      return handleBranchReadFile(toolArguments);
    }
    if (toolName === "branch_status") {
      return handleBranchStatus();
    }
    if (toolName === "branch_cleanup") {
      return handleBranchCleanup(toolArguments);
    }
  }
  if (toolName === "register_workspace_tool") {
    return handleRegisterWorkspaceTool(toolArguments, getWorkspaceRoot());
  }
  if (toolName === "reload_window_ready") {
    return handleReloadWindowReady(toolArguments, getWorkspaceRoot());
  }
  if (toolName === "unregister_workspace_tool") {
    return handleUnregisterWorkspaceTool(toolArguments, getWorkspaceRoot());
  }
  const root = getWorkspaceRoot();
  if (isUserTool(toolName, root)) {
    return executeUserTool(toolName, toolArguments, root);
  }
  return null;
}

async function handleRequest(request) {
  const { id, method } = request;

  if (method === "initialize") {
    send({
      jsonrpc: "2.0",
      id,
      result: {
        protocolVersion: MCP_VERSION,
        capabilities: { tools: {} },
        serverInfo: { name: "GitHub Shell Helpers", version: SERVER_VERSION },
      },
    });
    return;
  }

  if (method === "notifications/initialized") {
    return;
  }

  if (method === "tools/list") {
    notifySessionPulse();
    send({ jsonrpc: "2.0", id, result: { tools: getEnabledTools() } });
    return;
  }

  if (method !== "tools/call") {
    sendError(id, -32601, `Unknown method: ${method}`);
    return;
  }

  const toolName = request.params?.name;
  const toolArguments = request.params?.arguments || {};
  // `{ force: true }` is the documented escape hatch (see the no-op message
  // below): it overrides both a per-tool disable and the master kill-switch.
  const isForced = toolArguments.force === true;

  if (isToolDisabled(toolName) && !isForced) {
    send({
      jsonrpc: "2.0",
      id,
      result: {
        content: [
          {
            type: "text",
            text: `[no-op] The "${toolName}" tool was disabled by the user during this session. This is not an error — continue your current task normally. If the user explicitly asked for this action, recall with { "force": true } to override.`,
          },
        ],
      },
    });
    return;
  }

  const activityId = notifyActivityBegin(toolName, toolArguments);
  try {
    const builtInContent = await runBuiltInTool(
      toolName,
      toolArguments,
      activityId,
    );
    if (builtInContent) {
      send({ jsonrpc: "2.0", id, result: { content: builtInContent } });
      return;
    }

    for (const handler of delegatedHandlers) {
      const content = await handler(toolName, toolArguments);
      if (content) {
        send({ jsonrpc: "2.0", id, result: { content } });
        return;
      }
    }

    sendError(id, -32601, `Unknown tool: ${toolName}`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    sendError(id, -32603, message);
  } finally {
    notifyActivityEnd(activityId);
  }
}

function startServer() {
  const lineReader = readline.createInterface({
    input: process.stdin,
    crlfDelay: Infinity,
  });

  lineReader.on("line", async (line) => {
    if (!line.trim()) {
      return;
    }
    try {
      await handleRequest(JSON.parse(line));
    } catch {
      sendError(null, -32700, "Parse error");
    }
  });
}

// Serve one MCP session over a duplex stream (a unix socket from the daemon).
// Each line is processed inside an AsyncLocalStorage scope whose write() points
// back at this stream, so concurrent connections never cross responses.
function serveConnection(stream) {
  const write = (s) => {
    try {
      stream.write(s);
    } catch {
      /* peer gone */
    }
  };
  const lineReader = readline.createInterface({
    input: stream,
    crlfDelay: Infinity,
  });
  lineReader.on("line", (line) => {
    if (!line.trim()) return;
    outputStore.run({ write }, async () => {
      try {
        await handleRequest(JSON.parse(line));
      } catch {
        sendError(null, -32700, "Parse error");
      }
    });
  });
}

module.exports = { handleRequest, startServer, serveConnection };

if (require.main === module) {
  startServer();
}
