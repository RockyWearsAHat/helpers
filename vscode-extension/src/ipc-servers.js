"use strict";
// src/ipc-servers.js — Strict Lint IPC and Activity IPC Unix socket servers
const vscode = require("vscode");
const fs = require("fs");
const net = require("net");
const os = require("os");
const path = require("path");

const STRICT_LINT_SOCKET_PATH = path.join(os.tmpdir(), "gsh-strict-lint.sock");
const STRICT_LINT_IPC_INFO_PATH = path.join(
  os.homedir(),
  ".cache",
  "gsh",
  "strict-lint-ipc.json",
);

const ACTIVITY_SOCKET_PATH = path.join(os.tmpdir(), "gsh-activity.sock");
const ACTIVITY_IPC_INFO_PATH = path.join(
  os.homedir(),
  ".cache",
  "gsh",
  "activity-ipc.json",
);

module.exports = function createIpcServers(deps) {
  const {
    beginToolCall,
    endToolCall,
    runStrictLinting,
    getActivityItems,
    getWebviewProvider,
    handleWorktreeIpcMessage,
    getActiveChatTabKey,
    getPendingBranchSessionStarts,
    setSuppressTabDrivenUnfocusUntil,
    ensureSessionStarted,
  } = deps;

  let _strictLintIpcServer = null;
  let _activityIpcServer = null;
  const _externalToInternal = new Map();

  function startStrictLintIpcServer() {
    if (_strictLintIpcServer) return;

    try {
      if (fs.existsSync(STRICT_LINT_SOCKET_PATH)) {
        fs.unlinkSync(STRICT_LINT_SOCKET_PATH);
      }
    } catch {}

    try {
      fs.mkdirSync(path.dirname(STRICT_LINT_IPC_INFO_PATH), {
        recursive: true,
      });
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
    } catch {}

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
    } catch {}
    try {
      fs.unlinkSync(STRICT_LINT_IPC_INFO_PATH);
    } catch {}
  }

  function startActivityIpcServer() {
    if (_activityIpcServer) return;

    try {
      if (fs.existsSync(ACTIVITY_SOCKET_PATH)) {
        fs.unlinkSync(ACTIVITY_SOCKET_PATH);
      }
    } catch {}

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
    } catch {}

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
            if (msg.tool === "branch_session_start") {
              const tabKey = getActiveChatTabKey();
              getPendingBranchSessionStarts().set(msg.id, {
                tabKey,
                capturedAt: Date.now(),
              });
              setSuppressTabDrivenUnfocusUntil(Date.now() + 1500);
              deps.writeWorktreeDebug?.(
                `activityBegin branch_session_start activity=${msg.id} tab=${tabKey || "null"}`,
              );
            }
          } else if (msg.type === "activityEnd" && msg.id) {
            const internalId = _externalToInternal.get(msg.id);
            if (internalId) {
              _externalToInternal.delete(msg.id);
              endToolCall(internalId);
            }
          } else if (msg.type === "sessionPulse") {
            ensureSessionStarted();
            getWebviewProvider()?.pushUpdate({
              type: "activityUpdate",
              items: getActivityItems(),
            });
          } else if (
            msg.type === "worktreeCreated" ||
            msg.type === "worktreeRemoved"
          ) {
            handleWorktreeIpcMessage(msg);
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
    } catch {}
    try {
      fs.unlinkSync(ACTIVITY_IPC_INFO_PATH);
    } catch {}
    _externalToInternal.clear();
  }

  return {
    startStrictLintIpcServer,
    stopStrictLintIpcServer,
    startActivityIpcServer,
    stopActivityIpcServer,
  };
};
