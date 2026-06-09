// lib/mcp-strict-lint.js — strict_lint tool: VS Code diagnostics via IPC
"use strict";

const path = require("path");
const fs = require("fs");
const net = require("net");

const STRICT_LINT_IPC_INFO_PATH = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".cache",
  "gsh",
  "strict-lint-ipc.json",
);

const STRICT_LINT_TOOL = {
  name: "strict_lint",
  description:
    "Run VS Code's live diagnostics (errors and warnings) on a file, folder, or the entire workspace. Returns the same output you see in the Problems panel. Call this after every file edit before declaring implementation complete. If errors or warnings are reported, fix them or explicitly document why they are acceptable before returning.",
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
        description: "Which severity levels to include. Defaults to 'all'.",
      },
    },
    required: [],
  },
};

async function handleStrictLint(args) {
  let socketPath;
  try {
    const info = JSON.parse(fs.readFileSync(STRICT_LINT_IPC_INFO_PATH, "utf8"));
    socketPath = info.socketPath;
  } catch {
    return [
      {
        type: "text",
        text: "gsh strict-lint IPC unavailable. Make sure the gsh VS Code extension is running.",
      },
    ];
  }
  if (!socketPath) {
    return [
      {
        type: "text",
        text: "gsh strict-lint IPC socket path missing from info file.",
      },
    ];
  }
  return new Promise((resolve) => {
    const sock = net.createConnection(socketPath);
    let buffer = "";
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
          sock.destroy();
          if (resp.ok) {
            resolve([{ type: "text", text: resp.result }]);
          } else {
            resolve([
              { type: "text", text: `strict_lint error: ${resp.error}` },
            ]);
          }
        } catch {
          // ignore parse error, keep reading
        }
      }
    });
    sock.on("error", (err) => {
      resolve([
        {
          type: "text",
          text: `strict_lint IPC error: ${err.message}. Make sure the gsh VS Code extension is running.`,
        },
      ]);
    });
    sock.on("timeout", () => {
      sock.destroy();
      resolve([{ type: "text", text: "strict_lint IPC timed out after 15s." }]);
    });
  });
}

module.exports = { STRICT_LINT_TOOL, handleStrictLint };
