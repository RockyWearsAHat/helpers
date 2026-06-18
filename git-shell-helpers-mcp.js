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

// ─── per-session workspace via the MCP `roots` capability ───────────────────
// A resident daemon serves many projects from one process, so the workspace
// cannot come from process cwd/env. Instead each session learns its project
// from the client: clients that advertise the `roots` capability answer a
// `roots/list` request with their open folder(s). That answer is authoritative
// — it is the client's real project — so we prefer it over the cwd/env fallback.

// A `file://` URI (or a bare path) as a filesystem path; null if unusable.
function uriToPath(uri) {
  if (!uri || typeof uri !== "string") return null;
  if (uri.startsWith("file://")) {
    try {
      return decodeURIComponent(new URL(uri).pathname) || null;
    } catch {
      return null;
    }
  }
  return uri.startsWith("/") ? uri : null;
}

// Per-connection state, reused across every line of the session so a workspace
// learned at initialize/roots persists for later tool calls.
function makeSession(write) {
  return {
    write,
    workspaceRoot: null,
    supportsRoots: false,
    pending: new Map(), // server-issued request id -> response callback
    nextId: 1,
  };
}

// Ask the client for its roots; record the first as the session workspace.
function requestRoots(session) {
  if (!session || !session.supportsRoots) return;
  const id = `gsh-roots-${session.nextId++}`;
  session.pending.set(id, (msg) => {
    const roots = msg && msg.result && msg.result.roots;
    if (Array.isArray(roots) && roots.length > 0) {
      const p = uriToPath(roots[0].uri);
      if (p) session.workspaceRoot = p;
    }
  });
  send({ jsonrpc: "2.0", id, method: "roots/list", params: {} });
}

// True for a JSON-RPC response (a reply to a request we issued), not a request.
function isResponse(msg) {
  return (
    msg &&
    msg.method === undefined &&
    msg.id !== undefined &&
    ("result" in msg || "error" in msg)
  );
}

// Route one parsed message: replies to our own requests resolve pending
// callbacks; everything else is a client request/notification we handle.
async function dispatchMessage(session, msg) {
  if (isResponse(msg)) {
    const cb = session && session.pending.get(msg.id);
    if (cb) {
      session.pending.delete(msg.id);
      try {
        cb(msg);
      } catch {
        /* a bad roots reply just leaves the cwd/env fallback in place */
      }
    }
    return;
  }
  await handleRequest(msg);
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

const researchModule = loadResearchModule();
const delegatedHandlers = [researchModule?.handler].filter(Boolean);

// Validate the required native binary at startup (fail-loud if missing/broken).
loadNativeSchemas();

// The full tool surface: the Node web-research tools plus everything the native
// binary advertises (built-in Rust tools + project-local flows). Re-read each
// time so newly registered flows appear live.
function getAllTools() {
  return [
    ...(researchModule?.tools || []),
    ...loadNativeSchemas(getWorkspaceRoot()),
  ];
}

function getWorkspaceRoot() {
  // The client's own project (via the `roots` capability) wins when known — it
  // is correct even when a shared daemon's cwd points elsewhere.
  const session = outputStore.getStore();
  if (session && session.workspaceRoot) return session.workspaceRoot;
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
  const all = getAllTools();
  if (process.env.GSH_FORCE_ENABLE === "1") return all;
  if (isGshDisabled()) return [];
  const disabled = new Set(readToolsConfig().disabledTools || []);
  if (disabled.size === 0) return all;
  return all.filter((tool) => !disabled.has(tool.name));
}

function isToolDisabled(toolName) {
  if (isGshDisabled()) return true;
  return (readToolsConfig().disabledTools || []).includes(toolName);
}

// Native Rust tools (built-ins + project-local flows) are dispatched to the
// binary; everything else falls through to the delegated Node handlers (web).
async function runBuiltInTool(toolName, toolArguments) {
  const workspaceRoot = getWorkspaceRoot();
  if (isNativeTool(toolName, workspaceRoot)) {
    return runNativeTool(toolName, toolArguments, { workspaceRoot });
  }
  return null;
}

async function handleRequest(request) {
  const { id, method } = request;

  if (method === "initialize") {
    const session = outputStore.getStore();
    if (session) {
      const params = request.params || {};
      session.supportsRoots = !!(params.capabilities && params.capabilities.roots);
      // Some clients also include the folder synchronously in initialize; take
      // it as an immediate hint, later refined by the authoritative roots/list.
      const folders = params.workspaceFolders;
      const hint =
        uriToPath(params.rootUri) ||
        (Array.isArray(folders) && folders[0] && uriToPath(folders[0].uri)) ||
        (typeof params.rootPath === "string" ? params.rootPath : null);
      if (hint) session.workspaceRoot = hint;
    }
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
    // Now that the session is live, fetch the client's authoritative roots.
    requestRoots(outputStore.getStore());
    return;
  }

  if (method === "notifications/roots/list_changed") {
    // The client opened/closed a folder — re-resolve the workspace.
    requestRoots(outputStore.getStore());
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
    const builtInContent = await runBuiltInTool(toolName, toolArguments);
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
  // One session for the whole stdio process, run inside the store so per-session
  // workspace (roots) and routing work identically to the daemon path.
  const session = makeSession((s) => process.stdout.write(s));
  const lineReader = readline.createInterface({
    input: process.stdin,
    crlfDelay: Infinity,
  });

  // A stdio MCP server's lifetime is its stdin: when the client closes the pipe
  // (disconnects, or a one-shot probe like `gsh status` finishes) we must exit
  // rather than linger — otherwise the activity-IPC socket and other keep-alive
  // handles outlive the client, leaving zombie node servers and stalling callers
  // that spawnSync-wait for the process to die. But we cannot exit the instant
  // stdin closes: in-flight requests are handled asynchronously, so a bare exit
  // would race ahead and cut off the response still being written. Track pending
  // requests and exit only once the last one has flushed.
  let pending = 0;
  let stdinClosed = false;
  const exitWhenDrained = () => {
    if (!stdinClosed || pending > 0) return;
    // Flush any buffered stdout before tearing down lingering handles.
    process.stdout.write("", () => process.exit(0));
  };

  lineReader.on("line", (line) => {
    if (!line.trim()) return;
    pending += 1;
    outputStore.run(session, async () => {
      try {
        await dispatchMessage(session, JSON.parse(line));
      } catch {
        sendError(null, -32700, "Parse error");
      } finally {
        pending -= 1;
        exitWhenDrained();
      }
    });
  });
  lineReader.on("close", () => {
    stdinClosed = true;
    exitWhenDrained();
  });
}

// Serve one MCP session over a duplex stream (a unix socket from the daemon).
// One session object is reused for every line so workspace state learned at
// initialize/roots survives across the connection; each line runs inside an
// AsyncLocalStorage scope whose write() points back at this stream, so
// concurrent connections never cross responses.
function serveConnection(stream) {
  const write = (s) => {
    try {
      stream.write(s);
    } catch {
      /* peer gone */
    }
  };
  const session = makeSession(write);
  const lineReader = readline.createInterface({
    input: stream,
    crlfDelay: Infinity,
  });
  lineReader.on("line", (line) => {
    if (!line.trim()) return;
    outputStore.run(session, async () => {
      try {
        await dispatchMessage(session, JSON.parse(line));
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
