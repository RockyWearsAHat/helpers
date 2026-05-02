// lib/mcp-user-tools.js — Workspace-local user-defined MCP tools
//
// One registration tool. Copilot writes the script and calls
// register_workspace_tool(name, description, script). The tool is
// immediately callable via tools/call on the gsh MCP server — no restart.
//
// Layout inside the workspace root:
//
//   .gsh/tools/
//     manifest.json     ← [{name, description, language, entry}]
//     <name>.js         ← JS: module.exports = async (args) => "result"
//     <name>.sh         ← shell: reads $GSH_TOOL_ARGS (JSON), writes stdout
//
// First call with a given name writes the file. Subsequent calls with the
// same name and no new script skip the write (already built).

"use strict";

const fs = require("fs");
const path = require("path");
const { spawn, spawnSync } = require("child_process");

const TOOLS_DIR = ".gsh/tools";
const MANIFEST = "manifest.json";

function spawnPostReloadAutomation(eventName, toolName, root, options = {}) {
  if (process.env.GSH_AUTOMATION_DISABLED === "1") return;
  if (process.platform !== "darwin") return false;

  const scriptPath = path.join(__dirname, "post-reload-chat-automation.js");
  if (!fs.existsSync(scriptPath)) return false;

  const forceReload = options.forceReload === false ? "0" : "1";
  const sendContinue = options.sendContinue === true ? "1" : "0";
  const continueText = String(options.continueText || "window reloaded");
  const debounceMs = Number.isFinite(options.debounceMs)
    ? String(Math.max(0, Math.floor(options.debounceMs)))
    : "2500";

  try {
    const child = spawn(
      process.execPath,
      [scriptPath, "--event", eventName, "--tool", toolName, "--root", root],
      {
        cwd: root,
        detached: true,
        stdio: "ignore",
        env: {
          ...process.env,
          GSH_AUTOMATION_TRIGGER: eventName,
          GSH_AUTOMATION_TOOL: toolName,
          GSH_AUTOMATION_ROOT: root,
          GSH_AUTOMATION_FORCE_RELOAD: forceReload,
          GSH_AUTOMATION_SEND_CONTINUE: sendContinue,
          GSH_AUTOMATION_CONTINUE_TEXT: continueText,
          GSH_AUTOMATION_DEBOUNCE_MS: debounceMs,
        },
      },
    );
    child.unref();
    return true;
  } catch {
    // Best effort only. Registration/unregistration must still succeed.
    return false;
  }
}

// ─── Internal helpers ────────────────────────────────────────────────────────

function toolsDir(root) {
  return path.join(root, TOOLS_DIR);
}

function manifestPath(root) {
  return path.join(root, TOOLS_DIR, MANIFEST);
}

function loadManifest(root) {
  try {
    const raw = fs.readFileSync(manifestPath(root), "utf8");
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed.tools) ? parsed.tools : [];
  } catch {
    return [];
  }
}

function saveManifest(root, tools) {
  const dir = toolsDir(root);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(
    manifestPath(root),
    JSON.stringify({ tools }, null, 2) + "\n",
    "utf8",
  );
}

// ─── Dynamic tool list (called on every tools/list) ──────────────────────────

function getUserToolSchemas(root) {
  return loadManifest(root).map((t) => ({
    name: t.name,
    description: t.description || ("Workspace tool: " + t.name),
    inputSchema: t.parameters_schema || { type: "object", properties: {} },
  }));
}

function isUserTool(toolName, root) {
  return loadManifest(root).some((t) => t.name === toolName);
}

// ─── Execution ───────────────────────────────────────────────────────────────

async function executeUserTool(toolName, args, root) {
  const tool = loadManifest(root).find((t) => t.name === toolName);
  if (!tool) return null;

  const entry = path.join(toolsDir(root), tool.entry);
  if (!fs.existsSync(entry)) {
    return [{ type: "text", text: "Tool \"" + toolName + "\": entry file missing at " + entry + ". Re-register with the script to recreate it." }];
  }

  if (tool.language === "shell") {
    const r = spawnSync("bash", [entry], {
      encoding: "utf8",
      cwd: root,
      env: { ...process.env, GSH_TOOL_ARGS: JSON.stringify(args || {}) },
      timeout: 30000,
    });
    const out = (r.stdout || "").trim() || (r.stderr || "").trim() || "(no output)";
    return [{ type: "text", text: r.status !== 0 ? ("Tool \"" + toolName + "\" exited " + r.status + ":\n" + out) : out }];
  }

  // JavaScript — hot-reload on every call
  try {
    delete require.cache[require.resolve(entry)];
    const mod = require(entry);
    const fn = typeof mod === "function" ? mod
      : typeof mod.run === "function" ? mod.run
      : typeof mod.default === "function" ? mod.default
      : null;
    if (!fn) {
      return [{ type: "text", text: "Tool \"" + toolName + "\": export a function via module.exports = async (args) => { ... }" }];
    }
    const result = await fn(args || {});
    if (typeof result === "string") return [{ type: "text", text: result }];
    if (Array.isArray(result)) return result;
    if (result == null) return [{ type: "text", text: "(no output)" }];
    return [{ type: "text", text: JSON.stringify(result, null, 2) }];
  } catch (err) {
    return [{ type: "text", text: "Tool \"" + toolName + "\" threw: " + err.message }];
  }
}

// ─── register_workspace_tool handler ────────────────────────────────────────

async function handleRegisterWorkspaceTool(args, root) {
  const name = String(args && args.name || "").trim();
  const description = String(args && args.description || "").trim();
  const script = String(args && args.script || "").trim();

  if (!name) return [{ type: "text", text: "register_workspace_tool: 'name' is required." }];
  if (!description) return [{ type: "text", text: "register_workspace_tool: 'description' is required." }];
  if (!/^[a-z][a-z0-9_-]{0,63}$/.test(name)) {
    return [{ type: "text", text: "register_workspace_tool: invalid name \"" + name + "\". Use lowercase letters, digits, hyphens, underscores. Must start with a letter." }];
  }

  const lang = (args && args.language === "shell") ? "shell" : "javascript";
  const ext = lang === "shell" ? ".sh" : ".js";
  const entryFilename = name + ext;
  const entryPath = path.join(toolsDir(root), entryFilename);
  const alreadyBuilt = fs.existsSync(entryPath);

  fs.mkdirSync(toolsDir(root), { recursive: true });

  if (script) {
    fs.writeFileSync(entryPath, script, "utf8");
    if (lang === "shell") fs.chmodSync(entryPath, 0o755);
  } else if (!alreadyBuilt) {
    return [{ type: "text", text: "register_workspace_tool: \"" + name + "\" is not yet built. Pass 'script' with the implementation." }];
  }

  const tools = loadManifest(root);
  const record = { name, description, language: lang, entry: entryFilename };
  const idx = tools.findIndex((t) => t.name === name);
  if (idx >= 0) tools[idx] = record; else tools.push(record);
  saveManifest(root, tools);

  const action = (script && alreadyBuilt) ? "updated" : script ? "created and registered" : "already built — re-registered";
  return [{
    type: "text",
    text: [
      "Tool \"" + name + "\" " + action + ".",
      "Path: " + entryPath,
      "Language: " + lang,
      "",
      "It is now live in tools/list on this MCP server. Call it directly:",
      "  tools/call { \"name\": \"" + name + "\", \"arguments\": { ... } }",
    ].join("\n"),
  }];
}

// ─── unregister_workspace_tool handler ──────────────────────────────────────

async function handleUnregisterWorkspaceTool(args, root) {
  const name = String(args && args.name || "").trim();
  if (!name) return [{ type: "text", text: "unregister_workspace_tool: 'name' is required." }];

  const tools = loadManifest(root);
  const idx = tools.findIndex((t) => t.name === name);
  if (idx === -1) {
    return [{ type: "text", text: "unregister_workspace_tool: tool \"" + name + "\" not found in manifest." }];
  }

  const record = tools[idx];
  tools.splice(idx, 1);
  saveManifest(root, tools);

  // Optionally delete the script file
  if (args && args.delete_file !== false) {
    const entryPath = path.join(toolsDir(root), record.entry);
    try { fs.unlinkSync(entryPath); } catch { /* already gone */ }
  }

  return [{
    type: "text",
    text: [
      "Tool \"" + name + "\" removed from manifest.",
      args && args.delete_file === false
        ? "Script file preserved (delete_file=false)."
        : "Script file deleted.",
      "",
      "It is no longer listed in tools/list.",
    ].join("\n"),
  }];
}

async function handleReloadWindowReady(args, root) {
  const keyword = String((args && args.keyword) || "window reloaded").trim() || "window reloaded";
  const forceReload = !(args && args.force_reload === false);
  const eventName = String((args && args.event_name) || "tool-registration-batch").trim() || "tool-registration-batch";
  const debounceMsRaw = args && args.debounce_ms;
  const debounceMs = Number.isFinite(Number(debounceMsRaw))
    ? Math.max(0, Math.floor(Number(debounceMsRaw)))
    : 2500;

  const ok = spawnPostReloadAutomation(eventName, "", root, {
    forceReload,
    sendContinue: true,
    continueText: keyword,
    debounceMs,
  });

  if (!ok) {
    return [{
      type: "text",
      text:
        "reload_window_ready could not launch automation in this environment. " +
        "Run the script manually: node lib/post-reload-chat-automation.js",
    }];
  }

  return [{
    type: "text",
    text: [
      "Launched detached reload automation.",
      "After reload, chat will receive keyword: \"" + keyword + "\".",
      "Wait for that keyword before continuing the original request.",
    ].join("\n"),
  }];
}

// ─── MCP schema ───────────────────────────────────────────────────────────────

const REGISTER_WORKSPACE_TOOL = {
  name: "register_workspace_tool",
  description:
    "Register a script as a callable MCP tool in this workspace. Pass the full script source and it is written to .gsh/tools/ and immediately available via tools/call — no restart needed. On subsequent calls with the same name and no script, the existing file is re-registered (idempotent).",
  inputSchema: {
    type: "object",
    properties: {
      name: {
        type: "string",
        description: "Tool name in snake_case or kebab-case. Becomes the tools/call name.",
      },
      description: {
        type: "string",
        description: "What the tool does. Be specific — Copilot uses this to decide when to call it.",
      },
      script: {
        type: "string",
        description:
          "Full source code. JS: module.exports = async (args) => { return 'result'; }. Shell: read args from $GSH_TOOL_ARGS (JSON), write output to stdout. Omit to re-register an already-built tool without overwriting the file.",
      },
      language: {
        type: "string",
        enum: ["javascript", "shell"],
        description: "Implementation language. Default: javascript.",
      },
    },
    required: ["name", "description"],
  },
};

const RELOAD_WINDOW_READY_TOOL = {
  name: "reload_window_ready",
  description:
    "Launch a detached VS Code reload automation and post a resume keyword in chat after the window is ready. Use once after finishing a batch of tool registrations/unregistrations.",
  inputSchema: {
    type: "object",
    properties: {
      keyword: {
        type: "string",
        description: "Keyword message to post after reload. Default: window reloaded.",
      },
      force_reload: {
        type: "boolean",
        description: "Whether to trigger Developer: Reload Window before posting the keyword. Default: true.",
      },
      debounce_ms: {
        type: "number",
        description: "Debounce wait before acting, used to coalesce bursts. Default: 2500.",
      },
      event_name: {
        type: "string",
        description: "Event tag used for debounce slot isolation.",
      },
    },
    required: [],
  },
};

module.exports = {
  REGISTER_WORKSPACE_TOOL,
  RELOAD_WINDOW_READY_TOOL,
  UNREGISTER_WORKSPACE_TOOL: {
    name: "unregister_workspace_tool",
    description:
      "Remove a workspace tool from the MCP tool list. Deletes its entry from the manifest and optionally removes the script file. The tool immediately disappears from tools/list — no restart needed.",
    inputSchema: {
      type: "object",
      properties: {
        name: {
          type: "string",
          description: "Exact tool name to remove.",
        },
        delete_file: {
          type: "boolean",
          description: "Whether to delete the script file from .gsh/tools/. Default: true.",
        },
      },
      required: ["name"],
    },
  },
  getUserToolSchemas,
  isUserTool,
  executeUserTool,
  handleRegisterWorkspaceTool,
  handleReloadWindowReady,
  handleUnregisterWorkspaceTool,
};
