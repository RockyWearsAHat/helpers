// lib/mcp-strict-lint.js — strict_lint tool.
//
// Primary path: VS Code's live diagnostics via IPC (aggregates every installed
// language server — the richest source). Fallback when VS Code isn't running
// (Claude Code CLI, any non-VSCode agent): run the project's own language
// linters/type-checkers directly. See mcp-strict-lint-standalone.js.
"use strict";

const path = require("path");
const fs = require("fs");
const net = require("net");
const { runStandaloneLint } = require("./mcp-strict-lint-standalone");

const STRICT_LINT_IPC_INFO_PATH = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".cache",
  "gsh",
  "strict-lint-ipc.json",
);

const STRICT_LINT_TOOL = {
  name: "strict_lint",
  description:
    "Run strict diagnostics on a file, folder, or the whole workspace and report errors, warnings, AND best-practice hints. Inside VS Code it returns the live Problems panel (every installed language server). Elsewhere it runs the project's own tooling — eslint + tsc, ruff + mypy, cargo clippy, go vet + staticcheck, shellcheck — so you get each language provider's current best-practice recommendations with their rule ids. Call after every edit before declaring work complete; fix reported issues (or document why a warning is acceptable), and treat each rule as a principle to apply going forward.",
  inputSchema: {
    type: "object",
    properties: {
      filePath: {
        type: "string",
        description:
          "Absolute path to a specific file to check. Omit to check the whole workspace.",
      },
      folderPath: {
        type: "string",
        description:
          "Absolute path to a folder to check. Omit to check the whole workspace.",
      },
      severityFilter: {
        type: "string",
        enum: ["all", "errors-only", "warnings-and-above"],
        description:
          "Which severity levels to include. 'all' includes hint/style recommendations. Defaults to 'all'.",
      },
    },
    required: [],
  },
};

// VS Code reports this when it has no linter/language server active for the
// target — in that case the CLI providers are strictly better than nothing.
const PROVIDER_INACTIVE_RE =
  /no diagnostics provider|requires an active|provider .*not active|no .*provider activity/i;

// Try the VS Code extension over IPC. Resolves to:
//   { ok: true, text }                  — real diagnostics from VS Code
//   { ok: false, providerInactive, text } — VS Code reachable but errored
//   null                                — extension not reachable
function tryIpc(args) {
  return new Promise((resolve) => {
    let socketPath;
    try {
      socketPath = JSON.parse(
        fs.readFileSync(STRICT_LINT_IPC_INFO_PATH, "utf8"),
      ).socketPath;
    } catch {
      return resolve(null);
    }
    if (!socketPath) return resolve(null);

    const sock = net.createConnection(socketPath);
    let buffer = "";
    let settled = false;
    const done = (v) => {
      if (settled) return;
      settled = true;
      sock.destroy();
      resolve(v);
    };
    sock.setTimeout(15000);
    sock.setEncoding("utf8");
    sock.on("connect", () => {
      sock.write(JSON.stringify({ arguments: args || {} }) + "\n");
    });
    sock.on("data", (chunk) => {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";
      for (const line of lines) {
        if (!line.trim()) continue;
        try {
          const resp = JSON.parse(line);
          if (resp.ok) done({ ok: true, text: resp.result });
          else
            done({
              ok: false,
              providerInactive: PROVIDER_INACTIVE_RE.test(resp.error || ""),
              text: `strict_lint error: ${resp.error}`,
            });
        } catch {
          // keep reading
        }
      }
    });
    // Connection refused / socket gone / timeout → fall back to standalone.
    sock.on("error", () => done(null));
    sock.on("timeout", () => done(null));
  });
}

function standalone(args) {
  try {
    return runStandaloneLint(args || {});
  } catch (err) {
    return `strict_lint (standalone) failed: ${err.message}`;
  }
}

async function handleStrictLint(args) {
  const ipc = await tryIpc(args);

  // VS Code produced real diagnostics — authoritative (aggregates every provider).
  if (ipc && ipc.ok) return [{ type: "text", text: ipc.text }];

  // Either VS Code is unreachable, or it has no active provider for this target.
  // Both cases are handled by the language's own CLI tooling.
  const cliText = standalone(args);
  const cliFoundProviders = !/providers run: none/.test(cliText);

  if (cliFoundProviders) {
    const note =
      ipc && ipc.providerInactive
        ? "[VS Code had no active diagnostics provider for this target — used the language's CLI tooling instead]\n\n"
        : "";
    return [{ type: "text", text: note + cliText }];
  }

  // No CLI provider either. Prefer VS Code's (more specific) error if present.
  if (ipc && !ipc.ok) return [{ type: "text", text: ipc.text }];
  return [{ type: "text", text: cliText }];
}

module.exports = { STRICT_LINT_TOOL, handleStrictLint };
