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
  const archiveCalls = [];
  const archiveCallsBySession = new Map();
  const activityUpdates = [];

  fs.mkdirSync(workspaceRoot, { recursive: true });
  fs.mkdirSync(chatSessionsDir, { recursive: true });
  fs.mkdirSync(globalStorageUriPath, { recursive: true });
  fs.writeFileSync(storageUriPath, "", "utf8");

  const recentSessionId = "recent-session";
  const oldSessionId = "old-session";
  const recentFile = path.join(chatSessionsDir, `${recentSessionId}.jsonl`);
  const oldFile = path.join(chatSessionsDir, `${oldSessionId}.jsonl`);
  writeChatSessionFile(recentFile, "Recent Session");
  writeChatSessionFile(oldFile, "Older Session");

  const now = new Date();
  const oldTime = new Date(now.getTime() - 3600 * 1000);
  fs.utimesSync(recentFile, now, now);
  fs.utimesSync(oldFile, oldTime, oldTime);

  const fakeArchive = {
    initialize() {},
    archiveSessionFile(sessionId, filePath, metadata = {}) {
      const callCount = (archiveCallsBySession.get(sessionId) || 0) + 1;
      archiveCallsBySession.set(sessionId, callCount);
      archiveCalls.push({ sessionId, filePath, metadata });
      return {
        appendedBytes: Math.min(Number(metadata.maxBytes) || 0, fs.statSync(filePath).size),
        chunksWritten: callCount === 1 ? 1 : 0,
        partialBytes: 0,
        remainingBytes: callCount === 1 ? 1 : 0,
        complete: callCount !== 1,
        sourceSize: fs.statSync(filePath).size,
      };
    },
    updateSessionMetadata() {},
    searchArchive() {
      return [];
    },
    getArchiveRoot() {
      return path.join(tmpRoot, "archive");
    },
    renderSearchResultsMarkdown() {
      return "";
    },
  };

  const fakeVscode = {
    workspace: {
      workspaceFolders: [{ uri: { fsPath: workspaceRoot } }],
      getConfiguration: () => ({
        get: (key, defaultValue) => {
          if (key === "enabled") return true;
          return defaultValue;
        },
      }),
      openTextDocument: async () => ({ uri: { fsPath: "" } }),
    },
    window: {
      showInputBox: async () => "",
      showInformationMessage: async () => undefined,
      showTextDocument: async () => undefined,
    },
  };

  const originalLoad = Module._load;
  Module._load = function patchedLoad(request, parent, isMain) {
    if (request === "vscode") return fakeVscode;
    if (
      request === "./chat-history-archive" &&
      parent &&
      parent.filename &&
      parent.filename.endsWith(path.join("vscode-extension", "src", "chat-sessions.js"))
    ) {
      return () => fakeArchive;
    }
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
    chatSessions.dispose();

    assert.ok(
      archiveCalls.length >= 4,
      "startup archive should process recent writes and continue background drain",
    );
    assert.ok(
      archiveCalls.every((call) => call.metadata.maxBytes === 1024 * 1024),
      "chat session archiving should always use a bounded startup byte budget",
    );
    assert.ok(
      archiveCalls.some((call) => call.sessionId === recentSessionId),
      "recent sessions should still be archived immediately",
    );
    assert.ok(
      archiveCalls.some((call) => call.sessionId === oldSessionId),
      "older sessions should be archived incrementally in the background",
    );
    assert.ok(
      activityUpdates.length >= 1,
      "watcher should still publish sidebar activity updates",
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
