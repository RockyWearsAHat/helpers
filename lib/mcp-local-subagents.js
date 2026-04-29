// lib/mcp-local-subagents.js — Local sub-agent tools (Ollama + OpenClaw)
//
// Provides MCP tools that let a paid model offload work to a free local
// agent. Two backends are supported:
//
//   * Ollama (`ollama_subagent`) — runs an autonomous tool-use loop against
//     a local Ollama model via /api/chat. The local model can call a curated
//     set of tools (read_file, list_dir, grep, write_file, run_shell,
//     web_search, scrape_url) until it produces a final answer.
//
//   * OpenClaw (`openclaw_task`) — dispatches a one-shot task to a locally
//     installed OpenClaw CLI/gateway and returns the result.
//
// Destructive tools (write_file, run_shell) are gated by environment
// variables that mirror the VS Code extension settings:
//
//   GSH_LOCAL_SUBAGENT_ALLOW_WRITE=1
//   GSH_LOCAL_SUBAGENT_ALLOW_SHELL=1
//   GSH_LOCAL_SUBAGENT_WORKSPACE=/abs/path  (cwd boundary; defaults to process.cwd())
//   GSH_LOCAL_SUBAGENT_OLLAMA_HOST=http://127.0.0.1:11434
//   GSH_LOCAL_SUBAGENT_OLLAMA_MODEL=llama3.1
//   GSH_LOCAL_SUBAGENT_OLLAMA_MAX_ITER=12
//   GSH_LOCAL_SUBAGENT_OLLAMA_TIMEOUT=300
//   GSH_LOCAL_SUBAGENT_OPENCLAW_BIN=openclaw
//   GSH_LOCAL_SUBAGENT_OPENCLAW_GATEWAY=http://127.0.0.1:18789
//   GSH_LOCAL_SUBAGENT_OPENCLAW_TIMEOUT=600

"use strict";

const fs = require("fs");
const path = require("path");
const http = require("http");
const https = require("https");
const { execFile, spawn } = require("child_process");
const { URL } = require("url");

// ─── Tool schemas ───────────────────────────────────────────────────────────

const OLLAMA_SUBAGENT_TOOL = {
  name: "ollama_subagent",
  description:
    "Run an autonomous local sub-agent backed by Ollama (llama.cpp under the hood) to complete a task end-to-end. The local model executes a tool-use loop with read_file, list_dir, grep, web_search, scrape_url, and (when enabled) write_file and run_shell. Use this to offload work that would be expensive to send to a paid model — research, reading large files, generating boilerplate, summarizing many sources, repetitive edits, or first-pass implementations. Blocks until the local agent reports a final answer or the iteration cap is hit.",
  inputSchema: {
    type: "object",
    properties: {
      task: {
        type: "string",
        description:
          "The full task description for the local sub-agent. Be explicit about success criteria, files to inspect, and the desired output shape — the local model is smaller than a flagship paid model and benefits from concrete instructions.",
      },
      model: {
        type: "string",
        description:
          "Ollama model tag (e.g. 'llama3.1', 'qwen2.5-coder:14b'). Omit to use the workspace default. Use ollama_list_models to see what is installed locally.",
      },
      system_prompt: {
        type: "string",
        description:
          "Optional override for the agent's system prompt. The default instructs the model to use the available tools and finish with a clear final answer.",
      },
      max_iterations: {
        type: "integer",
        description:
          "Maximum tool-use rounds before forcing a final answer. Default 12.",
      },
      timeout_seconds: {
        type: "integer",
        description:
          "Total wall-clock budget for the whole loop. Default 300.",
      },
      temperature: {
        type: "number",
        description: "Sampling temperature for the local model. Default 0.2.",
      },
      num_ctx: {
        type: "integer",
        description:
          "Context window size to request from Ollama. Default 8192. Increase for tasks that require reading large files.",
      },
    },
    required: ["task"],
  },
};

const OLLAMA_LIST_MODELS_TOOL = {
  name: "ollama_list_models",
  description:
    "List Ollama models installed on the local machine. Returns the model tag, parameter size, quantization, and modified time for each entry. Use this before calling ollama_subagent to pick a model that is actually present.",
  inputSchema: {
    type: "object",
    properties: {
      host: {
        type: "string",
        description:
          "Ollama host URL. Defaults to the configured workspace value or http://127.0.0.1:11434.",
      },
    },
    required: [],
  },
};

const OPENCLAW_TASK_TOOL = {
  name: "openclaw_task",
  description:
    "Dispatch a single-shot task to the locally installed OpenClaw personal AI assistant and wait for it to finish. OpenClaw runs the task on the user's machine using their configured local or remote model and returns the final response. Use this when the user has OpenClaw set up and wants the work done on their hardware. Requires the openclaw CLI to be installed and the OpenClaw gateway daemon to be running.",
  inputSchema: {
    type: "object",
    properties: {
      message: {
        type: "string",
        description: "The task or question to send to OpenClaw.",
      },
      thinking: {
        type: "string",
        enum: ["off", "low", "medium", "high"],
        description:
          "Reasoning depth hint forwarded as --thinking. Default uses the workspace setting (medium).",
      },
      target: {
        type: "string",
        description:
          "Optional channel/peer to deliver the response to (e.g. '+15551234567' for SMS, 'discord:123' for a Discord DM). Omit to keep the result inline.",
      },
      timeout_seconds: {
        type: "integer",
        description:
          "Wall-clock timeout for the openclaw invocation. Default 600.",
      },
    },
    required: ["message"],
  },
};

const OPENCLAW_STATUS_TOOL = {
  name: "openclaw_status",
  description:
    "Check whether the OpenClaw CLI is installed and whether its gateway is reachable. Returns the binary path, version, and gateway health. Call this before openclaw_task to give the user a useful error message if OpenClaw isn't set up.",
  inputSchema: {
    type: "object",
    properties: {},
    required: [],
  },
};

const LOCAL_SUBAGENT_TOOLS = [
  OLLAMA_SUBAGENT_TOOL,
  OLLAMA_LIST_MODELS_TOOL,
  OPENCLAW_TASK_TOOL,
  OPENCLAW_STATUS_TOOL,
];

// ─── Settings resolution ────────────────────────────────────────────────────

function envFlag(name) {
  const value = process.env[name];
  return value === "1" || value === "true";
}

function resolveOllamaHost(override) {
  return (
    override ||
    process.env.GSH_LOCAL_SUBAGENT_OLLAMA_HOST ||
    "http://127.0.0.1:11434"
  );
}

function resolveOllamaModel(override) {
  return (
    override ||
    process.env.GSH_LOCAL_SUBAGENT_OLLAMA_MODEL ||
    ""
  );
}

function resolveOllamaMaxIter(override) {
  if (Number.isFinite(override) && override > 0) return Math.min(override, 50);
  const fromEnv = parseInt(process.env.GSH_LOCAL_SUBAGENT_OLLAMA_MAX_ITER, 10);
  if (Number.isFinite(fromEnv) && fromEnv > 0) return Math.min(fromEnv, 50);
  return 12;
}

function resolveOllamaTimeout(override) {
  if (Number.isFinite(override) && override > 0) return Math.min(override, 3600);
  const fromEnv = parseInt(process.env.GSH_LOCAL_SUBAGENT_OLLAMA_TIMEOUT, 10);
  if (Number.isFinite(fromEnv) && fromEnv > 0) return Math.min(fromEnv, 3600);
  return 300;
}

function resolveWorkspaceRoot() {
  const envRoot = process.env.GSH_LOCAL_SUBAGENT_WORKSPACE;
  if (envRoot && fs.existsSync(envRoot)) return path.resolve(envRoot);
  // GSH_WORKSPACE_ROOTS may be either a JSON array (set by the VS Code
  // extension) or a comma-separated list (legacy), so handle both.
  const raw = process.env.GSH_WORKSPACE_ROOTS || "";
  let roots = [];
  if (raw.trim().startsWith("[")) {
    try {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) roots = parsed.filter((s) => typeof s === "string");
    } catch {
      /* fall through */
    }
  }
  if (!roots.length) {
    roots = raw.split(",").map((s) => s.trim()).filter(Boolean);
  }
  if (roots[0] && fs.existsSync(roots[0])) return path.resolve(roots[0]);
  return process.cwd();
}

function resolveOpenclawBin() {
  return process.env.GSH_LOCAL_SUBAGENT_OPENCLAW_BIN || "openclaw";
}

function resolveOpenclawGateway() {
  return (
    process.env.GSH_LOCAL_SUBAGENT_OPENCLAW_GATEWAY ||
    "http://127.0.0.1:18789"
  );
}

function resolveOpenclawTimeout(override) {
  if (Number.isFinite(override) && override > 0) return Math.min(override, 7200);
  const fromEnv = parseInt(
    process.env.GSH_LOCAL_SUBAGENT_OPENCLAW_TIMEOUT,
    10,
  );
  if (Number.isFinite(fromEnv) && fromEnv > 0) return Math.min(fromEnv, 7200);
  return 600;
}

// ─── HTTP helpers ───────────────────────────────────────────────────────────

function httpJson(method, urlString, body, timeoutMs) {
  return new Promise((resolve, reject) => {
    let parsed;
    try {
      parsed = new URL(urlString);
    } catch (err) {
      reject(new Error(`Invalid URL: ${urlString} (${err.message})`));
      return;
    }
    const transport = parsed.protocol === "https:" ? https : http;
    const payload = body == null ? null : Buffer.from(JSON.stringify(body));
    const headers = { Accept: "application/json" };
    if (payload) {
      headers["Content-Type"] = "application/json";
      headers["Content-Length"] = String(payload.length);
    }
    const req = transport.request(
      {
        method,
        hostname: parsed.hostname,
        port: parsed.port || (parsed.protocol === "https:" ? 443 : 80),
        path: `${parsed.pathname || "/"}${parsed.search || ""}`,
        headers,
        timeout: timeoutMs,
      },
      (res) => {
        let raw = "";
        res.setEncoding("utf8");
        res.on("data", (chunk) => {
          raw += chunk;
        });
        res.on("end", () => {
          if (res.statusCode < 200 || res.statusCode >= 300) {
            reject(
              new Error(
                `HTTP ${res.statusCode} from ${urlString}: ${raw.slice(0, 400)}`,
              ),
            );
            return;
          }
          if (!raw) {
            resolve(null);
            return;
          }
          try {
            resolve(JSON.parse(raw));
          } catch (err) {
            reject(new Error(`Invalid JSON from ${urlString}: ${err.message}`));
          }
        });
      },
    );
    req.on("error", reject);
    req.on("timeout", () => {
      req.destroy(new Error(`Request to ${urlString} timed out`));
    });
    if (payload) req.write(payload);
    req.end();
  });
}

// ─── Workspace-confined tool helpers ────────────────────────────────────────

function resolveWithinWorkspace(workspaceRoot, relPath) {
  if (typeof relPath !== "string" || !relPath.length) {
    throw new Error("path must be a non-empty string");
  }
  const candidate = path.resolve(workspaceRoot, relPath);
  const rootWithSep = workspaceRoot.endsWith(path.sep)
    ? workspaceRoot
    : workspaceRoot + path.sep;
  if (candidate !== workspaceRoot && !candidate.startsWith(rootWithSep)) {
    throw new Error(
      `path '${relPath}' escapes workspace root ${workspaceRoot}`,
    );
  }
  return candidate;
}

function truncate(str, max) {
  if (!str) return "";
  if (str.length <= max) return str;
  return `${str.slice(0, max)}\n…[truncated ${str.length - max} chars]`;
}

// Simple ripgrep wrapper. Falls back to a JS scan for portability.
function grepWorkspace({ workspaceRoot, pattern, path: subPath, max }) {
  return new Promise((resolve) => {
    const target = subPath
      ? resolveWithinWorkspace(workspaceRoot, subPath)
      : workspaceRoot;
    const limit = Math.max(1, Math.min(max || 100, 500));
    const args = [
      "--no-heading",
      "--with-filename",
      "--line-number",
      "--color=never",
      "--max-count=20",
      pattern,
      target,
    ];
    execFile(
      "rg",
      args,
      { cwd: workspaceRoot, timeout: 30000, maxBuffer: 4 * 1024 * 1024 },
      (err, stdout) => {
        if (err && err.code !== 1) {
          // 1 = no matches, anything else = real error
          resolve({ ok: false, error: err.message });
          return;
        }
        const lines = (stdout || "")
          .split("\n")
          .filter(Boolean)
          .slice(0, limit);
        resolve({ ok: true, lines });
      },
    );
  });
}

// ─── Local agent tool registry (exposed to the Ollama model) ────────────────

function buildLocalToolRegistry(researchHandler) {
  const workspaceRoot = resolveWorkspaceRoot();
  const allowWrite = envFlag("GSH_LOCAL_SUBAGENT_ALLOW_WRITE");
  const allowShell = envFlag("GSH_LOCAL_SUBAGENT_ALLOW_SHELL");

  const tools = [
    {
      schema: {
        type: "function",
        function: {
          name: "read_file",
          description:
            "Read a UTF-8 file from the workspace. Paths are resolved relative to the workspace root.",
          parameters: {
            type: "object",
            properties: {
              path: { type: "string", description: "Workspace-relative path." },
              max_chars: {
                type: "integer",
                description: "Truncate to this many chars (default 12000).",
              },
            },
            required: ["path"],
          },
        },
      },
      run: async ({ path: relPath, max_chars }) => {
        const abs = resolveWithinWorkspace(workspaceRoot, relPath);
        const stat = fs.statSync(abs);
        if (!stat.isFile()) throw new Error(`Not a file: ${relPath}`);
        const content = fs.readFileSync(abs, "utf8");
        return truncate(content, max_chars || 12000);
      },
    },
    {
      schema: {
        type: "function",
        function: {
          name: "list_dir",
          description:
            "List entries in a workspace directory. Returns up to 200 entries.",
          parameters: {
            type: "object",
            properties: {
              path: {
                type: "string",
                description:
                  "Workspace-relative path. Use '.' for the workspace root.",
              },
            },
            required: ["path"],
          },
        },
      },
      run: async ({ path: relPath }) => {
        const abs = resolveWithinWorkspace(workspaceRoot, relPath || ".");
        const entries = fs.readdirSync(abs, { withFileTypes: true });
        return entries
          .slice(0, 200)
          .map((entry) => `${entry.isDirectory() ? "d" : "f"} ${entry.name}`)
          .join("\n");
      },
    },
    {
      schema: {
        type: "function",
        function: {
          name: "grep",
          description:
            "Search the workspace for a regex pattern using ripgrep. Returns matching file:line:text lines.",
          parameters: {
            type: "object",
            properties: {
              pattern: {
                type: "string",
                description: "Regular expression to search for.",
              },
              path: {
                type: "string",
                description:
                  "Optional workspace-relative directory or file to scope the search.",
              },
              max_results: {
                type: "integer",
                description: "Maximum match lines to return (default 100).",
              },
            },
            required: ["pattern"],
          },
        },
      },
      run: async ({ pattern, path: subPath, max_results }) => {
        const result = await grepWorkspace({
          workspaceRoot,
          pattern,
          path: subPath,
          max: max_results,
        });
        if (!result.ok) return `[grep failed: ${result.error}]`;
        if (!result.lines.length) return "[no matches]";
        return result.lines.join("\n");
      },
    },
  ];

  if (allowWrite) {
    tools.push({
      schema: {
        type: "function",
        function: {
          name: "write_file",
          description:
            "Write UTF-8 content to a workspace file. Creates parent directories as needed. Overwrites existing files.",
          parameters: {
            type: "object",
            properties: {
              path: { type: "string", description: "Workspace-relative path." },
              content: { type: "string", description: "File contents." },
            },
            required: ["path", "content"],
          },
        },
      },
      run: async ({ path: relPath, content }) => {
        const abs = resolveWithinWorkspace(workspaceRoot, relPath);
        fs.mkdirSync(path.dirname(abs), { recursive: true });
        fs.writeFileSync(abs, String(content), "utf8");
        return `wrote ${Buffer.byteLength(String(content), "utf8")} bytes to ${relPath}`;
      },
    });
  }

  if (allowShell) {
    tools.push({
      schema: {
        type: "function",
        function: {
          name: "run_shell",
          description:
            "Run a shell command in the workspace root and return stdout/stderr. Has a 60-second timeout.",
          parameters: {
            type: "object",
            properties: {
              command: {
                type: "string",
                description: "Shell command to execute (passed to /bin/sh -c).",
              },
            },
            required: ["command"],
          },
        },
      },
      run: ({ command }) =>
        new Promise((resolve) => {
          execFile(
            "/bin/sh",
            ["-c", String(command)],
            {
              cwd: workspaceRoot,
              timeout: 60000,
              maxBuffer: 2 * 1024 * 1024,
            },
            (err, stdout, stderr) => {
              const out = (stdout || "").toString();
              const errOut = (stderr || "").toString();
              const combined = `${out}${errOut ? `\n[stderr]\n${errOut}` : ""}`;
              if (err && err.killed) {
                resolve(`[command timed out]\n${truncate(combined, 4000)}`);
                return;
              }
              if (err) {
                resolve(
                  `[exit ${err.code ?? "?"}]\n${truncate(combined, 4000)}`,
                );
                return;
              }
              resolve(truncate(combined, 4000) || "[no output]");
            },
          );
        }),
    });
  }

  if (researchHandler) {
    tools.push({
      schema: {
        type: "function",
        function: {
          name: "web_search",
          description:
            "Search the public web via Google and return up to max_results titles, URLs, and snippets.",
          parameters: {
            type: "object",
            properties: {
              query: { type: "string" },
              max_results: { type: "integer" },
            },
            required: ["query"],
          },
        },
      },
      run: async (args) => {
        const content = await researchHandler("search_web", {
          query: args.query,
          max_results: args.max_results || 10,
        });
        return extractText(content);
      },
    });
    tools.push({
      schema: {
        type: "function",
        function: {
          name: "scrape_url",
          description:
            "Fetch one or more URLs and return cleaned article text. Pass an array of absolute URLs.",
          parameters: {
            type: "object",
            properties: {
              urls: { type: "array", items: { type: "string" } },
            },
            required: ["urls"],
          },
        },
      },
      run: async (args) => {
        const content = await researchHandler("scrape_webpage", {
          urls: args.urls,
        });
        return truncate(extractText(content), 20000);
      },
    });
  }

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "finish",
        description:
          "Call this exactly once when the task is complete. Provide the final answer for the calling agent.",
        parameters: {
          type: "object",
          properties: {
            answer: {
              type: "string",
              description: "The final answer to return to the calling agent.",
            },
          },
          required: ["answer"],
        },
      },
    },
    run: async ({ answer }) => `[FINAL]${answer}`,
  });

  return { tools, workspaceRoot, allowWrite, allowShell };
}

function extractText(content) {
  if (!Array.isArray(content)) return "";
  const text = content
    .filter((item) => item && item.type === "text" && item.text)
    .map((item) => item.text)
    .join("\n");
  return text;
}

// ─── ollama_list_models handler ─────────────────────────────────────────────

async function handleOllamaListModels(args) {
  const host = resolveOllamaHost(args?.host);
  let body;
  try {
    body = await httpJson("GET", `${host}/api/tags`, null, 5000);
  } catch (err) {
    return [
      {
        type: "text",
        text:
          `Could not reach Ollama at ${host}: ${err.message}\n\n` +
          `Install: https://ollama.ai  |  Start: 'ollama serve' (or run any 'ollama run <model>')`,
      },
    ];
  }
  const models = Array.isArray(body?.models) ? body.models : [];
  if (!models.length) {
    return [
      {
        type: "text",
        text:
          `Ollama is reachable at ${host} but no models are installed.\n` +
          `Pull one with: ollama pull llama3.1`,
      },
    ];
  }
  const lines = [
    `Ollama models at ${host} (${models.length}):`,
    "",
    ...models.map((model) => {
      const name = model.name || model.model || "?";
      const size = model.size
        ? `${(model.size / 1e9).toFixed(1)} GB`
        : "";
      const params = model.details?.parameter_size || "";
      const quant = model.details?.quantization_level || "";
      const meta = [size, params, quant].filter(Boolean).join(" · ");
      return meta ? `  ${name}  (${meta})` : `  ${name}`;
    }),
  ];
  return [{ type: "text", text: lines.join("\n") }];
}

// ─── ollama_subagent handler (the agent loop) ───────────────────────────────

async function handleOllamaSubagent(args, deps) {
  const task = String(args?.task || "").trim();
  if (!task) {
    return [{ type: "text", text: "ollama_subagent: 'task' is required." }];
  }
  const host = resolveOllamaHost();
  const model = resolveOllamaModel(args?.model);
  if (!model) {
    return [
      {
        type: "text",
        text:
          "ollama_subagent: no model specified and no workspace default set.\n" +
          "Run ollama_list_models to see installed models, then pass `model` " +
          "or set the default in the GitHub Shell Helpers settings panel.",
      },
    ];
  }
  const maxIter = resolveOllamaMaxIter(args?.max_iterations);
  const timeoutSec = resolveOllamaTimeout(args?.timeout_seconds);
  const temperature =
    typeof args?.temperature === "number" ? args.temperature : 0.2;
  const numCtx = Number.isFinite(args?.num_ctx) ? args.num_ctx : 8192;

  const { tools, workspaceRoot, allowWrite, allowShell } =
    buildLocalToolRegistry(deps?.researchHandler);
  const toolByName = new Map(tools.map((t) => [t.schema.function.name, t]));

  const systemPrompt =
    args?.system_prompt ||
    [
      "You are a local sub-agent running on the user's machine via Ollama.",
      "You were dispatched by a more capable orchestrator that wants to offload work to save cost and latency.",
      `Workspace root: ${workspaceRoot}`,
      `Capabilities: read_file, list_dir, grep, web_search, scrape_url${allowWrite ? ", write_file" : ""}${allowShell ? ", run_shell" : ""}, finish.`,
      "",
      "Loop:",
      "1. Decide what information you need to complete the task.",
      "2. Call tools to gather it. Tool calls must use the function-call API.",
      "3. When you have enough information, call `finish` with the final answer for the caller.",
      "",
      "Be concise, accurate, and do not invent file paths. Always finish by calling `finish`.",
    ].join("\n");

  const messages = [
    { role: "system", content: systemPrompt },
    { role: "user", content: task },
  ];

  const transcript = [];
  const deadline = Date.now() + timeoutSec * 1000;
  let finalAnswer = null;
  let iterations = 0;
  let stopReason = "max_iterations";

  for (let i = 0; i < maxIter; i += 1) {
    iterations = i + 1;
    if (Date.now() > deadline) {
      stopReason = "timeout";
      break;
    }
    const remainingMs = Math.max(1000, deadline - Date.now());
    let response;
    try {
      response = await httpJson(
        "POST",
        `${host}/api/chat`,
        {
          model,
          stream: false,
          messages,
          tools: tools.map((t) => t.schema),
          options: { temperature, num_ctx: numCtx },
        },
        remainingMs,
      );
    } catch (err) {
      stopReason = `ollama_error: ${err.message}`;
      break;
    }
    const message = response?.message || {};
    messages.push(message);

    const toolCalls = Array.isArray(message.tool_calls)
      ? message.tool_calls
      : [];
    if (!toolCalls.length) {
      // Some models forget to call `finish` — accept plain content as final.
      const text = String(message.content || "").trim();
      if (text) {
        finalAnswer = text;
        stopReason = "completed_without_finish";
      } else {
        stopReason = "empty_response";
      }
      break;
    }

    for (const call of toolCalls) {
      const name = call?.function?.name;
      const rawArgs = call?.function?.arguments;
      let parsedArgs = rawArgs;
      if (typeof rawArgs === "string") {
        try {
          parsedArgs = rawArgs ? JSON.parse(rawArgs) : {};
        } catch {
          parsedArgs = {};
        }
      }
      const tool = toolByName.get(name);
      let resultText;
      if (!tool) {
        resultText = `[error: unknown tool "${name}"]`;
      } else {
        try {
          resultText = await tool.run(parsedArgs || {});
        } catch (err) {
          resultText = `[error: ${err.message}]`;
        }
      }
      transcript.push({
        iteration: iterations,
        tool: name,
        args: parsedArgs,
        result: truncate(String(resultText), 600),
      });
      if (typeof resultText === "string" && resultText.startsWith("[FINAL]")) {
        finalAnswer = resultText.slice("[FINAL]".length);
        stopReason = "finished";
        break;
      }
      messages.push({
        role: "tool",
        content: typeof resultText === "string" ? resultText : String(resultText),
        tool_name: name,
      });
    }
    if (finalAnswer != null) break;
  }

  if (finalAnswer == null) {
    // Last-resort: ask the model for a final answer with no tools.
    try {
      const wrap = await httpJson(
        "POST",
        `${host}/api/chat`,
        {
          model,
          stream: false,
          messages: [
            ...messages,
            {
              role: "user",
              content:
                "Stop using tools. Summarize what you found and provide your best final answer now.",
            },
          ],
          options: { temperature, num_ctx: numCtx },
        },
        Math.max(15000, deadline - Date.now()),
      );
      finalAnswer = String(wrap?.message?.content || "").trim();
      if (!stopReason || stopReason === "max_iterations") {
        stopReason = `${stopReason} (forced summary)`;
      }
    } catch (err) {
      finalAnswer = `[no final answer — ${err.message}]`;
    }
  }

  const lines = [
    `Local sub-agent (Ollama: ${model}) — ${stopReason} after ${iterations} iteration${iterations === 1 ? "" : "s"}.`,
    "",
    "── Final answer ──",
    finalAnswer || "[empty]",
  ];
  if (transcript.length) {
    lines.push("", "── Tool transcript ──");
    for (const entry of transcript) {
      lines.push(
        `[${entry.iteration}] ${entry.tool}(${truncate(JSON.stringify(entry.args || {}), 200)}) → ${entry.result}`,
      );
    }
  }
  return [{ type: "text", text: lines.join("\n") }];
}

// ─── OpenClaw handlers ──────────────────────────────────────────────────────

function runOpenclawCli(binary, cliArgs, timeoutMs, input) {
  return new Promise((resolve) => {
    const child = spawn(binary, cliArgs, {
      stdio: ["pipe", "pipe", "pipe"],
      env: process.env,
    });
    let stdout = "";
    let stderr = "";
    let done = false;
    const finish = (result) => {
      if (done) return;
      done = true;
      resolve(result);
    };
    const timer = setTimeout(() => {
      try {
        child.kill("SIGTERM");
      } catch {}
      finish({
        ok: false,
        code: null,
        stdout,
        stderr,
        error: `openclaw timed out after ${Math.round(timeoutMs / 1000)}s`,
      });
    }, timeoutMs);
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString("utf8");
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", (err) => {
      clearTimeout(timer);
      finish({
        ok: false,
        code: null,
        stdout,
        stderr,
        error: err.message,
      });
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      finish({
        ok: code === 0,
        code,
        stdout,
        stderr,
        error: code === 0 ? null : `openclaw exited with code ${code}`,
      });
    });
    if (input) {
      try {
        child.stdin.write(input);
      } catch {}
    }
    try {
      child.stdin.end();
    } catch {}
  });
}

async function handleOpenclawStatus() {
  const binary = resolveOpenclawBin();
  const gateway = resolveOpenclawGateway();
  const cli = await runOpenclawCli(binary, ["--version"], 5000);
  let cliState;
  if (cli.ok) {
    const version = (cli.stdout || cli.stderr || "").trim().split("\n")[0];
    cliState = `installed: ${binary}  (${version || "version unknown"})`;
  } else if (/ENOENT|not found/i.test(cli.error || "")) {
    cliState = `not installed: '${binary}' is not on PATH. Install with: npm install -g openclaw@latest`;
  } else {
    cliState = `error invoking '${binary}': ${cli.error}`;
  }

  let gatewayState;
  try {
    await httpJson("GET", `${gateway}/health`, null, 2000);
    gatewayState = `gateway reachable at ${gateway}`;
  } catch (err) {
    gatewayState = `gateway unreachable at ${gateway}: ${err.message}\n  Try: openclaw gateway --port 18789`;
  }

  return [
    {
      type: "text",
      text: ["OpenClaw status:", `  ${cliState}`, `  ${gatewayState}`].join(
        "\n",
      ),
    },
  ];
}

async function handleOpenclawTask(args) {
  const message = String(args?.message || "").trim();
  if (!message) {
    return [{ type: "text", text: "openclaw_task: 'message' is required." }];
  }
  const binary = resolveOpenclawBin();
  const timeoutMs = resolveOpenclawTimeout(args?.timeout_seconds) * 1000;
  const cliArgs = ["agent", "--message", message];
  const thinking = args?.thinking;
  if (thinking && thinking !== "off") {
    cliArgs.push("--thinking", thinking);
  }
  if (args?.target) {
    cliArgs.push("--target", String(args.target));
  }
  const result = await runOpenclawCli(binary, cliArgs, timeoutMs);
  if (!result.ok) {
    if (/ENOENT|not found/i.test(result.error || "")) {
      return [
        {
          type: "text",
          text:
            `openclaw_task: '${binary}' is not installed.\n` +
            `Install with: npm install -g openclaw@latest\n` +
            `Then run: openclaw onboard --install-daemon`,
        },
      ];
    }
    return [
      {
        type: "text",
        text:
          `openclaw_task failed: ${result.error}\n` +
          `stdout:\n${truncate(result.stdout, 2000)}\n` +
          `stderr:\n${truncate(result.stderr, 2000)}`,
      },
    ];
  }
  const body = (result.stdout || "").trim() || "[openclaw produced no output]";
  return [
    {
      type: "text",
      text: `OpenClaw task complete (exit ${result.code}):\n\n${truncate(body, 16000)}`,
    },
  ];
}

// ─── Dispatcher ─────────────────────────────────────────────────────────────

function createLocalSubagentHandler(deps) {
  return async function handleLocalSubagentTool(toolName, toolArguments) {
    if (toolName === "ollama_subagent") {
      return handleOllamaSubagent(toolArguments || {}, deps || {});
    }
    if (toolName === "ollama_list_models") {
      return handleOllamaListModels(toolArguments || {});
    }
    if (toolName === "openclaw_task") {
      return handleOpenclawTask(toolArguments || {});
    }
    if (toolName === "openclaw_status") {
      return handleOpenclawStatus();
    }
    return null;
  };
}

module.exports = {
  LOCAL_SUBAGENT_TOOLS,
  OLLAMA_SUBAGENT_TOOL,
  OLLAMA_LIST_MODELS_TOOL,
  OPENCLAW_TASK_TOOL,
  OPENCLAW_STATUS_TOOL,
  createLocalSubagentHandler,
  // exported for unit tests
  _internal: {
    resolveWithinWorkspace,
    resolveWorkspaceRoot,
    resolveOllamaHost,
    buildLocalToolRegistry,
    truncate,
  },
};
