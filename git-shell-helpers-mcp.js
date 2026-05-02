#!/usr/bin/env node
"use strict";

const path = require("path");
const readline = require("readline");
const { CHECKPOINT_TOOL, handleCheckpoint } = require("./lib/mcp-checkpoint");
const {
  WORKSPACE_CONTEXT_TOOL,
  handleWorkspaceContext,
} = require("./lib/mcp-workspace-context");
const { STRICT_LINT_TOOL, handleStrictLint } = require("./lib/mcp-strict-lint");
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
} = require("./lib/mcp-activity-ipc");

const MCP_VERSION = "2024-11-05";
const SERVER_VERSION = "1.1.0";
const TOOLS_CONFIG_PATH = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".config",
  "git-shell-helpers-mcp",
  "tools.json",
);

function send(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`);
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

function loadChatArchiveModule() {
  const archiveRoot = process.env.GSH_CHAT_ARCHIVE_ROOT;
  if (!archiveRoot) return null;
  try {
    const {
      CHAT_ARCHIVE_TOOLS,
      createHandler: createArchiveHandler,
    } = require("./lib/mcp-chat-archive");
    return {
      tools: CHAT_ARCHIVE_TOOLS,
      handler: createArchiveHandler({
        archiveRoot,
        globalArchiveRoot: process.env.GSH_CHAT_ARCHIVE_GLOBAL || null,
        workspaceRoots: process.env.GSH_WORKSPACE_ROOTS
          ? process.env.GSH_WORKSPACE_ROOTS.split(",").filter(Boolean)
          : [],
      }),
    };
  } catch (err) {
    process.stderr.write(
      `[git-shell-helpers-mcp] WARNING: could not load chat-archive: ${err.message}\n`,
    );
    return null;
  }
}

const researchModule = loadResearchModule();
const visionModule = loadVisionModule();
const chatArchiveModule = loadChatArchiveModule();
const localSubagentHandler = process.env.GSH_DISABLE_LOCAL_SUBAGENTS
  ? null
  : createLocalSubagentHandler({
      researchHandler: researchModule?.handler || null,
    });
const delegatedHandlers = [
  researchModule?.handler,
  visionModule?.handler,
  chatArchiveModule?.handler,
  localSubagentHandler,
].filter(Boolean);

const SESSION_MEMORY_TOOLS = new Set([
  "log_session_event",
  "search_session_log",
  "get_session_summary",
  "rebuild_session_index",
]);

function summarizeToolArguments(toolArguments) {
  const obj =
    toolArguments && typeof toolArguments === "object" ? toolArguments : {};
  const keys = Object.keys(obj);
  if (!keys.length) return "(none)";
  const summary = {};
  for (const key of keys.slice(0, 8)) {
    const value = obj[key];
    if (value === null || value === undefined) {
      summary[key] = value;
      continue;
    }
    if (typeof value === "string") {
      summary[key] = value.length > 160 ? value.slice(0, 157) + "..." : value;
      continue;
    }
    if (typeof value === "number" || typeof value === "boolean") {
      summary[key] = value;
      continue;
    }
    if (Array.isArray(value)) {
      summary[key] = `[array:${value.length}]`;
      continue;
    }
    summary[key] = "[object]";
  }
  return JSON.stringify(summary);
}

function summarizeToolResult(content) {
  if (!Array.isArray(content) || content.length === 0) return "no content";
  const textChunk = content.find(
    (item) => item && item.type === "text" && typeof item.text === "string",
  );
  if (!textChunk || !textChunk.text) return "non-text content";
  const oneLine = textChunk.text.replace(/\s+/g, " ").trim();
  if (!oneLine) return "empty text content";
  return oneLine.length > 180 ? oneLine.slice(0, 177) + "..." : oneLine;
}

function normalizeToolTag(name) {
  return String(name || "tool")
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
}

async function autoLogToolEvent(toolName, toolArguments, status, detail) {
  if (process.env.GSH_SESSION_MEMORY_DISABLED) return;
  if (process.env.GSH_AUTO_SESSION_LOG_DISABLED === "1") return;
  if (!researchModule?.handler) return;
  if (SESSION_MEMORY_TOOLS.has(toolName)) return;

  const safeArgs = summarizeToolArguments(toolArguments);
  const toolTag = normalizeToolTag(toolName);
  const outcome =
    status === "success"
      ? `success - ${detail || "completed"}`
      : `failed - ${detail || "tool call failed"}`;

  try {
    await researchModule.handler("log_session_event", {
      action: `auto tool call: ${toolName}`,
      outcome,
      tags: ["auto", "mcp-tool", toolTag, status],
      context: `args=${safeArgs}`,
    });
  } catch {
    // Never let telemetry-style auto logging affect user-visible tool behavior.
  }
}

const ALL_TOOLS = [
  ...(researchModule?.tools || []).filter(
    (t) =>
      !process.env.GSH_SESSION_MEMORY_DISABLED ||
      !SESSION_MEMORY_TOOLS.has(t.name),
  ),
  ...(visionModule?.tools || []),
  ...(chatArchiveModule?.tools || []),
  CHECKPOINT_TOOL,
  WORKSPACE_CONTEXT_TOOL,
  LIST_LANGUAGE_MODELS_TOOL,
  STRICT_LINT_TOOL,
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

function getEnabledTools() {
  const userTools = getUserToolSchemas(getWorkspaceRoot());
  const allWithUser = [...ALL_TOOLS, ...userTools];
  try {
    const config = JSON.parse(
      require("fs").readFileSync(TOOLS_CONFIG_PATH, "utf8"),
    );
    const disabled = new Set(config.disabledTools || []);
    if (disabled.size === 0) return allWithUser;
    return allWithUser.filter((tool) => !disabled.has(tool.name));
  } catch {
    return allWithUser;
  }
}

function isToolDisabled(toolName) {
  try {
    const config = JSON.parse(
      require("fs").readFileSync(TOOLS_CONFIG_PATH, "utf8"),
    );
    return (config.disabledTools || []).includes(toolName);
  } catch {
    return false;
  }
}

async function runBuiltInTool(toolName, toolArguments, activityId) {
  if (toolName === "checkpoint") {
    return handleCheckpoint(toolArguments);
  }
  if (toolName === "workspace_context") {
    return handleWorkspaceContext();
  }
  if (toolName === "list_language_models") {
    return handleListLanguageModels();
  }
  if (toolName === "strict_lint") {
    return handleStrictLint(toolArguments);
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
  const isForced = toolName === "checkpoint" && toolArguments.force === true;

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
      await autoLogToolEvent(
        toolName,
        toolArguments,
        "success",
        summarizeToolResult(builtInContent),
      );
      send({ jsonrpc: "2.0", id, result: { content: builtInContent } });
      return;
    }

    for (const handler of delegatedHandlers) {
      const content = await handler(toolName, toolArguments);
      if (content) {
        await autoLogToolEvent(
          toolName,
          toolArguments,
          "success",
          summarizeToolResult(content),
        );
        send({ jsonrpc: "2.0", id, result: { content } });
        return;
      }
    }

    sendError(id, -32601, `Unknown tool: ${toolName}`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    await autoLogToolEvent(toolName, toolArguments, "failed", message);
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

module.exports = { handleRequest, startServer };

if (require.main === module) {
  startServer();
}
