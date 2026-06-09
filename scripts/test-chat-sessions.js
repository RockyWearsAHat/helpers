#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs");
const Module = require("module");
const os = require("os");
const path = require("path");

function writeChatSessionFile(filePath, title) {
  const lines = [
    JSON.stringify({
      kind: 0,
      v: {
        version: 3,
        sessionId: path.basename(filePath, ".jsonl"),
        creationDate: Date.now(),
        requests: [],
      },
    }),
    JSON.stringify({
      kind: 1,
      k: ["customTitle"],
      v: title,
    }),
  ];
  fs.writeFileSync(filePath, `${lines.join("\n")}\n`, "utf8");
}

async function main() {
  const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "gsh-chat-sessions-"));
  const workspaceRoot = path.join(tmpRoot, "repo");
  const workspaceStorage = path.join(tmpRoot, "workspaceStorage", "workspace-id");
  const chatSessionsDir = path.join(workspaceStorage, "chatSessions");
  const storageUriPath = path.join(workspaceStorage, "state.vscdb");
  const globalStorageUriPath = path.join(tmpRoot, "globalStorage");
  const activityUpdates = [];

  fs.mkdirSync(workspaceRoot, { recursive: true });
  fs.mkdirSync(chatSessionsDir, { recursive: true });
  fs.mkdirSync(globalStorageUriPath, { recursive: true });
  fs.writeFileSync(storageUriPath, "", "utf8");

  const recentSessionId = "recent-session";
  const recentFile = path.join(chatSessionsDir, `${recentSessionId}.jsonl`);
  writeChatSessionFile(recentFile, "Recent Session");
  const now = new Date();
  fs.utimesSync(recentFile, now, now);

  const fakeVscode = {
    workspace: {
      workspaceFolders: [{ uri: { fsPath: workspaceRoot } }],
    },
  };

  const originalLoad = Module._load;
  Module._load = function patchedLoad(request, parent, isMain) {
    if (request === "vscode") return fakeVscode;
    return originalLoad.call(this, request, parent, isMain);
  };

  try {
    const createChatSessions = require("../vscode-extension/src/chat-sessions");
    const chatSessions = createChatSessions({
      getWebviewProvider: () => ({
        pushUpdate: (payload) => activityUpdates.push(payload),
      }),
      getActivityItems: () => [],
    });

    chatSessions.startChatSessionWatcher({
      storageUri: { fsPath: storageUriPath },
      globalStorageUri: { fsPath: globalStorageUriPath },
    });

    await new Promise((resolve) => setTimeout(resolve, 80));
    const sessions = chatSessions.getChatSessions();
    chatSessions.dispose();

    assert.ok(
      sessions.has(recentSessionId),
      "watcher should discover recent chat session files",
    );
    assert.strictEqual(
      sessions.get(recentSessionId).title,
      "Recent Session",
      "watcher should parse the session custom title",
    );
    assert.ok(
      activityUpdates.length >= 1,
      "watcher should publish sidebar activity updates",
    );
  } finally {
    Module._load = originalLoad;
    fs.rmSync(tmpRoot, { recursive: true, force: true });
  }

  console.log("chat-sessions tests passed");
}

main().catch((error) => {
  console.error(error && error.stack ? error.stack : String(error));
  process.exit(1);
});
