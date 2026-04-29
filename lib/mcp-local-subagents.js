// lib/mcp-local-subagents.js — Local sub-agent MCP tools
//
// Provides MCP tools that let a paid orchestrator offload work to a free
// local agent running on the user's machine.
//
//   * `ollama_subagent` — runs a Copilot-style tool-use loop against a local
//     Ollama model. Curated workspace tools: read_file, list_dir, grep,
//     web_search, scrape_url, optional write_file/run_shell. The local
//     model thinks-and-acts until it returns a final answer.
//
//   * `system_execute` — full-power local agent. Same loop, but with the
//     gloves off: unrestricted shell, full filesystem, and a persistent
//     Playwright browser (open URL, click, type, screenshot, eval). After
//     every screenshot the image is fed back to the next model turn so a
//     vision-capable Ollama model literally sees the page. Designed for
//     concrete one-shot system tasks the orchestrator wants done
//     autonomously: log in, fetch creds, rotate keys, capture and OCR a
//     screen, drive a UI test, etc.
//
//   * `ollama_list_models` — utility to enumerate installed Ollama models.
//
// Environment variables (set by the VS Code extension from settings):
//
//   GSH_LOCAL_SUBAGENT_ALLOW_WRITE=1
//   GSH_LOCAL_SUBAGENT_ALLOW_SHELL=1
//   GSH_LOCAL_SUBAGENT_WORKSPACE=/abs/path  (cwd boundary; defaults to process.cwd())
//   GSH_LOCAL_SUBAGENT_OLLAMA_HOST=http://127.0.0.1:11434
//   GSH_LOCAL_SUBAGENT_OLLAMA_MODEL=llama3.1
//   GSH_LOCAL_SUBAGENT_OLLAMA_MAX_ITER=12
//   GSH_LOCAL_SUBAGENT_OLLAMA_TIMEOUT=300
//   GSH_LOCAL_SUBAGENT_FULL_SYSTEM=1                  (master switch for system_execute)
//   GSH_LOCAL_SUBAGENT_SYSTEM_MODEL=qwen2.5vl:7b      (default vision model)
//   GSH_LOCAL_SUBAGENT_SYSTEM_MAX_ITER=25
//   GSH_LOCAL_SUBAGENT_SYSTEM_TIMEOUT=900
//   GSH_LOCAL_SUBAGENT_BROWSER_HEADLESS=1
//   GSH_LOCAL_SUBAGENT_BROWSER_CHANNEL=chrome         (or msedge, chromium)

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

const SYSTEM_EXECUTE_TOOL = {
  name: "system_execute",
  description:
    "Delegate a concrete system task to a free local agent that runs autonomously on the user's machine. The local model executes a Copilot-style tool-use loop with FULL system access: unrestricted shell, full filesystem, and a persistent Playwright browser (open URL, click, type, screenshot, evaluate JS). Every browser screenshot is fed back as a real image to the next model turn — use a vision-capable Ollama model (qwen2.5vl, llava, llama3.2-vision) for anything visual. Use this for tasks that would otherwise burn paid context: logging into a site to fetch a credential or rotate a key, driving a UI test and capturing screenshots, processing a downloaded file, running a long shell pipeline. Provide a precise success criterion in `task` — the orchestrator is the brain, the local agent is the hands. Requires the Local Sub-Agents → Full System Access setting to be enabled.",
  inputSchema: {
    type: "object",
    properties: {
      task: {
        type: "string",
        description:
          "The exact task to perform on the user's system, including success criteria and the desired return shape. Example: 'Open https://console.aws.amazon.com, log in with the credentials in .env, navigate to IAM, rotate the access key for user ci-bot, and return the new access key id and secret as JSON.'",
      },
      model: {
        type: "string",
        description:
          "Ollama model tag. For tasks that involve browser screenshots use a vision-capable model (e.g. 'qwen2.5vl:7b', 'llava:13b', 'llama3.2-vision:11b'). Omit to use the workspace default.",
      },
      context_files: {
        type: "array",
        items: { type: "string" },
        description:
          "Optional list of workspace-relative file paths to pre-load into the local agent's context (e.g. ['.env', 'README.md']).",
      },
      context_images: {
        type: "array",
        items: { type: "string" },
        description:
          "Optional list of absolute image paths or base64 data URLs to attach to the initial user message (e.g. screenshots the orchestrator already has).",
      },
      return_format: {
        type: "string",
        enum: ["text", "json"],
        description:
          "Hint for how the local agent should shape its final answer. Defaults to 'text'.",
      },
      max_iterations: {
        type: "integer",
        description:
          "Hard cap on agent loop iterations (default 25, max 100).",
      },
      timeout_seconds: {
        type: "integer",
        description:
          "Wall-clock timeout for the entire task (default 900, max 7200).",
      },
      headless: {
        type: "boolean",
        description:
          "If false, the Playwright browser opens visibly so the user can watch the agent work. Defaults to the workspace setting.",
      },
    },
    required: ["task"],
  },
};

const LOCAL_SUBAGENT_TOOLS = [
  OLLAMA_SUBAGENT_TOOL,
  OLLAMA_LIST_MODELS_TOOL,
  SYSTEM_EXECUTE_TOOL,
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

function resolveSystemFullAccess() {
  return envFlag("GSH_LOCAL_SUBAGENT_FULL_SYSTEM");
}

function resolveSystemModel(override) {
  return (
    override ||
    process.env.GSH_LOCAL_SUBAGENT_SYSTEM_MODEL ||
    process.env.GSH_LOCAL_SUBAGENT_OLLAMA_MODEL ||
    ""
  );
}

function resolveSystemMaxIter(override) {
  if (Number.isFinite(override) && override > 0) return Math.min(override, 100);
  const fromEnv = parseInt(
    process.env.GSH_LOCAL_SUBAGENT_SYSTEM_MAX_ITER,
    10,
  );
  if (Number.isFinite(fromEnv) && fromEnv > 0) return Math.min(fromEnv, 100);
  return 25;
}

function resolveSystemTimeout(override) {
  if (Number.isFinite(override) && override > 0) return Math.min(override, 7200);
  const fromEnv = parseInt(
    process.env.GSH_LOCAL_SUBAGENT_SYSTEM_TIMEOUT,
    10,
  );
  if (Number.isFinite(fromEnv) && fromEnv > 0) return Math.min(fromEnv, 7200);
  return 900;
}

function resolveBrowserHeadless(override) {
  if (typeof override === "boolean") return override;
  const env = process.env.GSH_LOCAL_SUBAGENT_BROWSER_HEADLESS;
  if (env === "0" || env === "false") return false;
  return true;
}

function resolveBrowserChannel() {
  return (
    process.env.GSH_LOCAL_SUBAGENT_BROWSER_CHANNEL || "chrome"
  );
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

// ─── system_execute: full-power local agent ─────────────────────────────────

let _playwrightModule = null;
function loadPlaywright() {
  if (_playwrightModule) return _playwrightModule;
  try {
    _playwrightModule = require("playwright");
    return _playwrightModule;
  } catch {
    /* fall through */
  }
  try {
    _playwrightModule = require("playwright-core");
    return _playwrightModule;
  } catch (err) {
    const message =
      "Browser tools require Playwright. Install once with:\n" +
      "  npm install -g playwright && npx playwright install chromium\n" +
      "or, in this workspace:\n" +
      "  npm install playwright\n\n" +
      `(underlying error: ${err.message})`;
    throw new Error(message);
  }
}

async function loadFileAsBase64Image(absPath) {
  const data = await fs.promises.readFile(absPath);
  const ext = path.extname(absPath).slice(1).toLowerCase() || "png";
  return { mime: `image/${ext === "jpg" ? "jpeg" : ext}`, base64: data.toString("base64") };
}

function parseImageInput(input) {
  if (typeof input !== "string") return null;
  const dataMatch = input.match(/^data:(image\/[^;]+);base64,(.+)$/);
  if (dataMatch) return { mime: dataMatch[1], base64: dataMatch[2] };
  return null;
}

function buildSystemAgentTools({
  workspaceRoot,
  browserState,
  researchHandler,
  pendingImages,
}) {
  const tools = [];

  // Filesystem (unrestricted within filesystem perms; default-bounded to
  // workspace, but allow absolute paths under full-system mode).
  tools.push({
    schema: {
      type: "function",
      function: {
        name: "read_file",
        description:
          "Read a UTF-8 file. Accepts workspace-relative or absolute paths. Truncates to max_chars.",
        parameters: {
          type: "object",
          properties: {
            path: { type: "string" },
            max_chars: { type: "integer" },
          },
          required: ["path"],
        },
      },
    },
    run: async ({ path: filePath, max_chars }) => {
      const abs = path.isAbsolute(filePath)
        ? filePath
        : path.resolve(workspaceRoot, filePath);
      const content = await fs.promises.readFile(abs, "utf8");
      return truncate(content, max_chars || 16000);
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "write_file",
        description:
          "Write UTF-8 content to a file. Accepts workspace-relative or absolute paths. Creates parent directories.",
        parameters: {
          type: "object",
          properties: {
            path: { type: "string" },
            content: { type: "string" },
          },
          required: ["path", "content"],
        },
      },
    },
    run: async ({ path: filePath, content }) => {
      const abs = path.isAbsolute(filePath)
        ? filePath
        : path.resolve(workspaceRoot, filePath);
      await fs.promises.mkdir(path.dirname(abs), { recursive: true });
      await fs.promises.writeFile(abs, String(content), "utf8");
      return `wrote ${Buffer.byteLength(String(content), "utf8")} bytes to ${abs}`;
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "list_dir",
        description: "List directory entries. Accepts workspace-relative or absolute paths.",
        parameters: {
          type: "object",
          properties: { path: { type: "string" } },
          required: ["path"],
        },
      },
    },
    run: async ({ path: dirPath }) => {
      const abs = path.isAbsolute(dirPath || ".")
        ? dirPath
        : path.resolve(workspaceRoot, dirPath || ".");
      const entries = await fs.promises.readdir(abs, { withFileTypes: true });
      return entries
        .slice(0, 500)
        .map((e) => `${e.isDirectory() ? "d" : "f"} ${e.name}`)
        .join("\n");
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "grep",
        description:
          "Search files via ripgrep. Pattern is a regex. Optional path scopes the search.",
        parameters: {
          type: "object",
          properties: {
            pattern: { type: "string" },
            path: { type: "string" },
            max_results: { type: "integer" },
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
  });

  // Unrestricted shell — full system access.
  tools.push({
    schema: {
      type: "function",
      function: {
        name: "run_shell",
        description:
          "Run a shell command via /bin/sh -c with full system access. 5-minute timeout. Returns combined stdout/stderr.",
        parameters: {
          type: "object",
          properties: {
            command: { type: "string" },
            cwd: {
              type: "string",
              description:
                "Optional working directory (absolute or workspace-relative). Defaults to workspace root.",
            },
            timeout_seconds: { type: "integer" },
          },
          required: ["command"],
        },
      },
    },
    run: ({ command, cwd, timeout_seconds }) =>
      new Promise((resolve) => {
        const cwdAbs = cwd
          ? path.isAbsolute(cwd)
            ? cwd
            : path.resolve(workspaceRoot, cwd)
          : workspaceRoot;
        const timeoutMs = Math.min(
          Math.max((timeout_seconds || 300) * 1000, 1000),
          15 * 60 * 1000,
        );
        execFile(
          "/bin/sh",
          ["-c", String(command)],
          { cwd: cwdAbs, timeout: timeoutMs, maxBuffer: 8 * 1024 * 1024 },
          (err, stdout, stderr) => {
            const combined = `${stdout || ""}${stderr ? `\n[stderr]\n${stderr}` : ""}`;
            if (err && err.killed) {
              resolve(`[command timed out]\n${truncate(combined, 8000)}`);
              return;
            }
            if (err) {
              resolve(
                `[exit ${err.code ?? "?"}]\n${truncate(combined, 8000)}`,
              );
              return;
            }
            resolve(truncate(combined, 8000) || "[no output]");
          },
        );
      }),
  });

  // Browser tools — lazy-launch persistent context on first use.
  async function ensureBrowser() {
    if (browserState.page) return browserState.page;
    const playwright = loadPlaywright();
    const launchOpts = { headless: browserState.headless };
    if (browserState.channel) launchOpts.channel = browserState.channel;
    try {
      browserState.browser = await playwright.chromium.launch(launchOpts);
    } catch (err) {
      // Fall back to bundled chromium if a system channel is unavailable.
      browserState.browser = await playwright.chromium.launch({
        headless: browserState.headless,
      });
    }
    browserState.context = await browserState.browser.newContext({
      viewport: { width: 1280, height: 800 },
    });
    browserState.page = await browserState.context.newPage();
    return browserState.page;
  }

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_open",
        description:
          "Navigate the persistent browser to a URL. Launches the browser on first call.",
        parameters: {
          type: "object",
          properties: {
            url: { type: "string" },
            wait_for: {
              type: "string",
              enum: ["load", "domcontentloaded", "networkidle"],
            },
          },
          required: ["url"],
        },
      },
    },
    run: async ({ url, wait_for }) => {
      const page = await ensureBrowser();
      await page.goto(url, { waitUntil: wait_for || "domcontentloaded", timeout: 45000 });
      return `loaded ${page.url()} (title: ${truncate((await page.title()) || "", 200)})`;
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_click",
        description: "Click an element. Selector can be CSS or text= (Playwright syntax).",
        parameters: {
          type: "object",
          properties: { selector: { type: "string" } },
          required: ["selector"],
        },
      },
    },
    run: async ({ selector }) => {
      const page = await ensureBrowser();
      await page.click(selector, { timeout: 15000 });
      return `clicked ${selector}`;
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_type",
        description: "Fill or type into an input. Replaces existing text by default.",
        parameters: {
          type: "object",
          properties: {
            selector: { type: "string" },
            text: { type: "string" },
            press_enter: { type: "boolean" },
          },
          required: ["selector", "text"],
        },
      },
    },
    run: async ({ selector, text, press_enter }) => {
      const page = await ensureBrowser();
      await page.fill(selector, String(text), { timeout: 15000 });
      if (press_enter) await page.press(selector, "Enter");
      return `filled ${selector} (${String(text).length} chars)${press_enter ? " and pressed Enter" : ""}`;
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_press",
        description: "Press a key globally on the page (e.g. 'Enter', 'Escape', 'Tab').",
        parameters: {
          type: "object",
          properties: { key: { type: "string" } },
          required: ["key"],
        },
      },
    },
    run: async ({ key }) => {
      const page = await ensureBrowser();
      await page.keyboard.press(String(key));
      return `pressed ${key}`;
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_wait",
        description:
          "Wait for a selector to appear, OR wait for ms milliseconds. Provide either selector or ms.",
        parameters: {
          type: "object",
          properties: {
            selector: { type: "string" },
            ms: { type: "integer" },
          },
        },
      },
    },
    run: async ({ selector, ms }) => {
      const page = await ensureBrowser();
      if (selector) {
        await page.waitForSelector(selector, { timeout: 30000 });
        return `selector appeared: ${selector}`;
      }
      const wait = Math.min(Math.max(ms || 500, 1), 30000);
      await page.waitForTimeout(wait);
      return `waited ${wait}ms`;
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_screenshot",
        description:
          "Capture a screenshot of the current page. The image is automatically attached to the next model turn so a vision model can see it. Returns a brief text confirmation.",
        parameters: {
          type: "object",
          properties: {
            full_page: { type: "boolean" },
          },
        },
      },
    },
    run: async ({ full_page }) => {
      const page = await ensureBrowser();
      const buf = await page.screenshot({ fullPage: !!full_page, type: "png" });
      pendingImages.push({ mime: "image/png", base64: buf.toString("base64") });
      return `captured ${full_page ? "full-page" : "viewport"} screenshot of ${page.url()} (${buf.length} bytes; attached as image to next turn)`;
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_get_text",
        description:
          "Get visible text from the page or a specific selector. Useful when a vision model isn't available.",
        parameters: {
          type: "object",
          properties: {
            selector: { type: "string" },
            max_chars: { type: "integer" },
          },
        },
      },
    },
    run: async ({ selector, max_chars }) => {
      const page = await ensureBrowser();
      const text = selector
        ? await page.locator(selector).innerText({ timeout: 10000 })
        : await page.evaluate(() => document.body?.innerText || "");
      return truncate(String(text || ""), max_chars || 8000);
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_get_url",
        description: "Return the current URL and document title.",
        parameters: { type: "object", properties: {} },
      },
    },
    run: async () => {
      const page = await ensureBrowser();
      return JSON.stringify({ url: page.url(), title: await page.title() });
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_eval",
        description:
          "Run a JS expression in the page context and return the JSON-stringified result. Use for reading DOM state, extracting structured data, etc.",
        parameters: {
          type: "object",
          properties: { expression: { type: "string" } },
          required: ["expression"],
        },
      },
    },
    run: async ({ expression }) => {
      const page = await ensureBrowser();
      const value = await page.evaluate(`(async () => { return (${expression}); })()`);
      return truncate(JSON.stringify(value, null, 2) ?? "undefined", 8000);
    },
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "browser_close",
        description: "Close the persistent browser. Optional — it is closed automatically when the task ends.",
        parameters: { type: "object", properties: {} },
      },
    },
    run: async () => {
      await closeBrowserState(browserState);
      return "browser closed";
    },
  });

  // Web research tools (delegate to existing research handler if available).
  if (researchHandler) {
    tools.push({
      schema: {
        type: "function",
        function: {
          name: "web_search",
          description: "Search the web and return titles/URLs/snippets.",
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
          max_results: args.max_results || 8,
        });
        return extractText(content);
      },
    });
  }

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "think",
        description:
          "Scratchpad for chain-of-thought. Use to write down a plan or notes that don't need to be returned to the orchestrator. Returns nothing useful.",
        parameters: {
          type: "object",
          properties: { notes: { type: "string" } },
          required: ["notes"],
        },
      },
    },
    run: async () => "noted",
  });

  tools.push({
    schema: {
      type: "function",
      function: {
        name: "finish",
        description:
          "Call this exactly once when the task is complete. Provide the final answer in the format requested by the orchestrator.",
        parameters: {
          type: "object",
          properties: { answer: { type: "string" } },
          required: ["answer"],
        },
      },
    },
    run: async ({ answer }) => `[FINAL]${answer}`,
  });

  return tools;
}

async function closeBrowserState(state) {
  try {
    if (state.context) await state.context.close();
  } catch {}
  try {
    if (state.browser) await state.browser.close();
  } catch {}
  state.page = null;
  state.context = null;
  state.browser = null;
}

async function handleSystemExecute(args, deps) {
  if (!resolveSystemFullAccess()) {
    return [
      {
        type: "text",
        text:
          "system_execute is disabled. Enable 'Local Sub-Agents → Full System Access' " +
          "in the GitHub Shell Helpers settings panel to allow Copilot to run autonomous " +
          "shell + browser tasks on your machine.",
      },
    ];
  }

  const task = String(args?.task || "").trim();
  if (!task) {
    return [{ type: "text", text: "system_execute: 'task' is required." }];
  }
  const host = resolveOllamaHost();
  const model = resolveSystemModel(args?.model);
  if (!model) {
    return [
      {
        type: "text",
        text:
          "system_execute: no model specified and no default set. Pick a vision-capable model " +
          "(qwen2.5vl:7b, llava:13b, llama3.2-vision:11b) in the settings panel or pass `model`.",
      },
    ];
  }
  const maxIter = resolveSystemMaxIter(args?.max_iterations);
  const timeoutSec = resolveSystemTimeout(args?.timeout_seconds);
  const returnFormat = args?.return_format === "json" ? "json" : "text";
  const headless = resolveBrowserHeadless(args?.headless);
  const channel = resolveBrowserChannel();
  const workspaceRoot = resolveWorkspaceRoot();

  const browserState = {
    browser: null,
    context: null,
    page: null,
    headless,
    channel,
  };
  const pendingImages = [];

  // Pre-load context files / images into the initial user message.
  const initialImages = [];
  if (Array.isArray(args?.context_images)) {
    for (const img of args.context_images) {
      const parsed = parseImageInput(img);
      if (parsed) {
        initialImages.push(parsed);
        continue;
      }
      try {
        const abs = path.isAbsolute(img) ? img : path.resolve(workspaceRoot, img);
        if (fs.existsSync(abs)) {
          initialImages.push(await loadFileAsBase64Image(abs));
        }
      } catch {
        /* ignore bad image */
      }
    }
  }
  let contextFilesBlob = "";
  if (Array.isArray(args?.context_files) && args.context_files.length) {
    const parts = [];
    for (const fp of args.context_files) {
      try {
        const abs = path.isAbsolute(fp) ? fp : path.resolve(workspaceRoot, fp);
        const content = await fs.promises.readFile(abs, "utf8");
        parts.push(`── ${fp} ──\n${truncate(content, 8000)}`);
      } catch (err) {
        parts.push(`── ${fp} ──\n[read failed: ${err.message}]`);
      }
    }
    contextFilesBlob = `\n\nPre-loaded files:\n${parts.join("\n\n")}`;
  }

  const tools = buildSystemAgentTools({
    workspaceRoot,
    browserState,
    researchHandler: deps?.researchHandler,
    pendingImages,
  });
  const toolByName = new Map(tools.map((t) => [t.schema.function.name, t]));

  const systemPrompt = [
    "You are a local sub-agent running on the user's machine.",
    "You were dispatched by an orchestrator (Copilot) that wants you to autonomously achieve a concrete system task and report back.",
    `Workspace root: ${workspaceRoot}`,
    "You have unrestricted access to: filesystem (read/write), shell, and a persistent Playwright browser (open/click/type/screenshot/eval).",
    "Every browser_screenshot you take is fed to your next turn as a real image — look at it before acting next.",
    "",
    "Operating rules:",
    "1. Plan, then act. Use `think` to write a short plan if the task is non-trivial.",
    "2. After every UI action, take a screenshot and look at it before issuing the next action — element names and selectors change.",
    "3. Read .env files when the task asks you to use saved credentials.",
    "4. When the task is complete, call `finish` exactly once with the requested answer.",
    `5. Return format: ${returnFormat}. ${returnFormat === "json" ? "The `answer` argument to finish must be a valid JSON string." : "The `answer` argument to finish should be plain text."}`,
    "",
    "Do not ask for clarification — make reasonable assumptions and act.",
  ].join("\n");

  const initialUserMessage = {
    role: "user",
    content: `${task}${contextFilesBlob}`,
  };
  if (initialImages.length) {
    initialUserMessage.images = initialImages.map((img) => img.base64);
  }

  const messages = [
    { role: "system", content: systemPrompt },
    initialUserMessage,
  ];

  const transcript = [];
  const deadline = Date.now() + timeoutSec * 1000;
  let finalAnswer = null;
  let iterations = 0;
  let stopReason = "max_iterations";

  try {
    for (let i = 0; i < maxIter; i += 1) {
      iterations = i + 1;
      if (Date.now() > deadline) {
        stopReason = "timeout";
        break;
      }
      const remainingMs = Math.max(2000, deadline - Date.now());
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
            options: { temperature: 0.2, num_ctx: 16384 },
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
          result: truncate(String(resultText), 400),
        });
        if (typeof resultText === "string" && resultText.startsWith("[FINAL]")) {
          finalAnswer = resultText.slice("[FINAL]".length);
          stopReason = "finished";
          break;
        }
        messages.push({
          role: "tool",
          content:
            typeof resultText === "string" ? resultText : String(resultText),
          tool_name: name,
        });
        // Flush any screenshots captured by the tool as a follow-up user
        // message so the vision model sees them on the next turn.
        if (pendingImages.length) {
          messages.push({
            role: "user",
            content: `[${pendingImages.length} screenshot(s) attached above for your inspection]`,
            images: pendingImages.map((img) => img.base64),
          });
          pendingImages.length = 0;
        }
      }
      if (finalAnswer != null) break;
    }

    if (finalAnswer == null) {
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
                  "Stop using tools. Provide your final answer now in the requested format.",
              },
            ],
            options: { temperature: 0.2, num_ctx: 16384 },
          },
          Math.max(15000, deadline - Date.now()),
        );
        finalAnswer = String(wrap?.message?.content || "").trim();
        if (stopReason === "max_iterations") {
          stopReason = "max_iterations (forced summary)";
        }
      } catch (err) {
        finalAnswer = `[no final answer — ${err.message}]`;
      }
    }
  } finally {
    await closeBrowserState(browserState);
  }

  const lines = [
    `system_execute (${model}) — ${stopReason} after ${iterations} iteration${iterations === 1 ? "" : "s"}.`,
    "",
    "── Final answer ──",
    finalAnswer || "[empty]",
  ];
  if (transcript.length) {
    lines.push("", "── Tool transcript ──");
    for (const entry of transcript) {
      lines.push(
        `[${entry.iteration}] ${entry.tool}(${truncate(JSON.stringify(entry.args || {}), 160)}) → ${entry.result}`,
      );
    }
  }
  return [{ type: "text", text: lines.join("\n") }];
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
    if (toolName === "system_execute") {
      return handleSystemExecute(toolArguments || {}, deps || {});
    }
    return null;
  };
}

module.exports = {
  LOCAL_SUBAGENT_TOOLS,
  OLLAMA_SUBAGENT_TOOL,
  OLLAMA_LIST_MODELS_TOOL,
  SYSTEM_EXECUTE_TOOL,
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
