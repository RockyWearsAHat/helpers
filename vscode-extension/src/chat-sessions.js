"use strict";
// src/chat-sessions.js — Copilot chat session JSONL watcher and parser
const vscode = require("vscode");
const fs = require("fs");
const os = require("os");
const path = require("path");
const createChatHistoryArchive = require("./chat-history-archive");

// Resolve the per-workspace archive root using VS Code's own stable workspace
// storage ID rather than hashing the folder path (which changes on rename).
//
// VS Code hands us context.storageUri whose *directory name* is already an
// opaque stable ID tied to the workspace, not the folder name.  We use that
// as the archive subdirectory key:
//   globalStorageBase/ws-archives/ws-{vscode-workspace-id}
//
// If storageUri is unavailable we fall back to reading workspaceStorage on
// disk to find the matching directory (the same logic _chatSessionsDir uses).
function _resolveArchiveRoot(ctx) {
  const storageBase = ctx?.globalStorageUri?.fsPath;
  if (!storageBase) return null;

  // context.storageUri is workspace-scoped and stable even on folder rename.
  if (ctx?.storageUri?.fsPath) {
    const wsId = path.basename(ctx.storageUri.fsPath);
    return path.join(storageBase, "ws-archives", `ws-${wsId}`);
  }

  // Fallback: scan workspaceStorage to find the directory whose workspace.json
  // matches the current open folder.  The directory name is VS Code's stable ID.
  const wsStorage = path.join(
    os.homedir(),
    "Library",
    "Application Support",
    "Code",
    "User",
    "workspaceStorage",
  );
  const openFolder = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
  if (openFolder && fs.existsSync(wsStorage)) {
    try {
      for (const d of fs.readdirSync(wsStorage)) {
        const wsjson = path.join(wsStorage, d, "workspace.json");
        try {
          const raw = fs.readFileSync(wsjson, "utf8");
          const data = JSON.parse(raw);
          const folder =
            data?.folder ||
            (Array.isArray(data?.folders) && data.folders[0]?.path) ||
            "";
          const folderPath = folder.startsWith("file://")
            ? decodeURIComponent(folder.replace(/^\/\//, "").replace(/^file:\/\//, ""))
            : folder;
          if (folderPath === openFolder) {
            return path.join(storageBase, "ws-archives", `ws-${d}`);
          }
        } catch {}
      }
    } catch {}
  }

  return path.join(storageBase, "ws-archives", "ws-global");
}

module.exports = function createChatSessions(deps) {
  const { getWebviewProvider, getActivityItems } = deps;

  const ARCHIVE_CHUNK_BYTES = 1024 * 1024;
  const ARCHIVE_QUEUE_DELAY_MS = 10;
  const _chatSessions = new Map(); // sessionId → session data
  const _chatHistoryArchive = createChatHistoryArchive();
  let _chatSessionWatcher = null;
  let _chatSessionPoller = null;
  let _archiveQueueTimer = null;
  let _archiveQueue = [];
  const _queuedArchiveSessions = new Set();

  function getChatSessions() {
    return _chatSessions;
  }

  function _chatSessionsDir(ctx) {
    if (ctx?.storageUri?.fsPath) {
      const candidate = path.join(
        path.dirname(ctx.storageUri.fsPath),
        "chatSessions",
      );
      if (fs.existsSync(candidate)) return candidate;
    }
    const wsStorage = path.join(
      os.homedir(),
      "Library",
      "Application Support",
      "Code",
      "User",
      "workspaceStorage",
    );
    if (!fs.existsSync(wsStorage)) return null;
    const openFolder = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    if (openFolder) {
      try {
        for (const d of fs.readdirSync(wsStorage)) {
          const wsjson = path.join(wsStorage, d, "workspace.json");
          const csDir = path.join(wsStorage, d, "chatSessions");
          try {
            const raw = fs.readFileSync(wsjson, "utf8");
            const data = JSON.parse(raw);
            const folder =
              data?.folder ||
              (Array.isArray(data?.folders) && data.folders[0]?.path) ||
              "";
            const folderPath = folder.startsWith("file://")
              ? decodeURIComponent(folder.replace(/^file:\/\//, ""))
              : folder;
            if (folderPath === openFolder && fs.existsSync(csDir)) return csDir;
          } catch {}
        }
      } catch {}
    }
    try {
      const dirs = fs
        .readdirSync(wsStorage)
        .map((d) => path.join(wsStorage, d, "chatSessions"))
        .filter((d) => {
          try {
            return fs.statSync(d).isDirectory();
          } catch {
            return false;
          }
        })
        .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
      if (dirs.length) return dirs[0];
    } catch {}
    return null;
  }

  function _pushActivityUpdate() {
    getWebviewProvider()?.pushUpdate({
      type: "activityUpdate",
      items: getActivityItems(),
    });
  }

  function _clearArchiveQueue() {
    if (_archiveQueueTimer) {
      clearTimeout(_archiveQueueTimer);
      _archiveQueueTimer = null;
    }
    _archiveQueue = [];
    _queuedArchiveSessions.clear();
  }

  function _scheduleArchiveQueueDrain() {
    if (_archiveQueueTimer || _archiveQueue.length === 0) return;
    _archiveQueueTimer = setTimeout(() => {
      _archiveQueueTimer = null;
      _drainArchiveQueue();
    }, ARCHIVE_QUEUE_DELAY_MS);
  }

  function _enqueueArchiveWork(sessionId, filePath, metadata = {}) {
    if (!sessionId || !filePath || _queuedArchiveSessions.has(sessionId)) return;
    _queuedArchiveSessions.add(sessionId);
    _archiveQueue.push({ sessionId, filePath, metadata });
    _scheduleArchiveQueueDrain();
  }

  function _drainArchiveQueue() {
    const next = _archiveQueue.shift();
    if (!next) return;

    let result = null;
    try {
      result = _chatHistoryArchive.archiveSessionFile(next.sessionId, next.filePath, {
        ...next.metadata,
        maxBytes: ARCHIVE_CHUNK_BYTES,
      });
    } catch {}

    if (result?.remainingBytes > 0) {
      _archiveQueue.push(next);
    } else {
      _queuedArchiveSessions.delete(next.sessionId);
    }

    _scheduleArchiveQueueDrain();
  }

  function _chatSessionReadTail(filePath, bytes) {
    const readLen = bytes || 65536;
    try {
      const fd = fs.openSync(filePath, "r");
      try {
        const { size } = fs.fstatSync(fd);
        const actual = Math.min(readLen, size);
        const buf = Buffer.alloc(actual);
        fs.readSync(fd, buf, 0, actual, size - actual);
        return { tail: buf.toString("utf8"), size };
      } finally {
        fs.closeSync(fd);
      }
    } catch {
      return { tail: "", size: 0 };
    }
  }

  function _chatSessionReadTitle(filePath, existing) {
    if (existing && existing !== "Copilot Chat") return existing;
    try {
      const fd = fs.openSync(filePath, "r");
      try {
        const stat = fs.fstatSync(fd);
        const fileSize = stat.size;
        let customTitle = null;
        let firstPrompt = null;

        const scanLines = (buf, len) => {
          for (const line of buf.slice(0, len).toString("utf8").split("\n")) {
            try {
              const rec = JSON.parse(line);
              if (
                rec.kind === 1 &&
                rec.k?.[0] === "customTitle" &&
                typeof rec.v === "string"
              ) {
                customTitle = rec.v;
              }
              if (
                !firstPrompt &&
                rec.kind === 2 &&
                rec.k?.[0] === "requests" &&
                rec.k.length === 1 &&
                Array.isArray(rec.v)
              ) {
                for (const req of rec.v) {
                  const msg =
                    req?.message?.text ||
                    req?.message ||
                    req?.text ||
                    req?.prompt;
                  if (typeof msg === "string" && msg.trim()) {
                    firstPrompt = msg.trim().slice(0, 80);
                    break;
                  }
                }
              }
            } catch {}
          }
        };

        const headBuf = Buffer.alloc(8192);
        const headN = fs.readSync(fd, headBuf, 0, 8192, 0);
        scanLines(headBuf, headN);
        if (customTitle) return customTitle;

        if (headN >= 8192 && !headBuf.slice(0, headN).includes(0x0a)) {
          const chunkSize = 65536;
          const scanBuf = Buffer.alloc(chunkSize);
          let offset = 8192;
          let nlOffset = -1;
          while (offset < fileSize && offset < 100 * 1024 * 1024) {
            const toRead = Math.min(chunkSize, fileSize - offset);
            const got = fs.readSync(fd, scanBuf, 0, toRead, offset);
            if (got === 0) break;
            const idx = scanBuf.indexOf(0x0a, 0);
            if (idx !== -1 && idx < got) {
              nlOffset = offset + idx;
              break;
            }
            offset += got;
          }
          if (nlOffset !== -1 && nlOffset + 1 < fileSize) {
            const afterBuf = Buffer.alloc(16384);
            const afterN = fs.readSync(fd, afterBuf, 0, 16384, nlOffset + 1);
            scanLines(afterBuf, afterN);
            if (customTitle) return customTitle;
          }
        }

        if (!customTitle && fileSize > 8192) {
          const tailSize = Math.min(32768, fileSize);
          const tailBuf = Buffer.alloc(tailSize);
          const tailN = fs.readSync(
            fd,
            tailBuf,
            0,
            tailSize,
            fileSize - tailSize,
          );
          scanLines(tailBuf, tailN);
        }

        return customTitle || firstPrompt || "Copilot Chat";
      } finally {
        fs.closeSync(fd);
      }
    } catch {}
    return "Copilot Chat";
  }

  function _chatSessionExtractPreview(tail) {
    const lines = tail.split("\n");
    let lastToolCall = null;
    let lastProgress = null;
    for (let i = lines.length - 1; i >= 0; i--) {
      try {
        const rec = JSON.parse(lines[i]);
        if (rec.kind === 2 || rec.kind === 1) {
          const val = rec.v;
          if (val && typeof val === "object") {
            if (!lastToolCall && typeof val.invocationMessage === "string") {
              lastToolCall = val.invocationMessage;
            }
            if (
              !lastProgress &&
              typeof val.content === "string" &&
              val.kind === "progressMessage"
            ) {
              lastProgress = val.content;
            }
            if (Array.isArray(val)) {
              for (let j = val.length - 1; j >= 0; j--) {
                const part = val[j];
                if (
                  !lastToolCall &&
                  typeof part?.invocationMessage === "string"
                ) {
                  lastToolCall = part.invocationMessage;
                }
                if (
                  !lastProgress &&
                  typeof part?.content === "string" &&
                  part?.kind === "progressMessage"
                ) {
                  lastProgress = part.content;
                }
              }
            }
          }
        }
        if (lastToolCall) break;
      } catch {}
    }
    return lastToolCall || lastProgress || null;
  }

  function _chatSessionParseState(tail) {
    const lines = tail.split("\n");
    let lastRequestIdx = -1;
    const doneRequests = new Set();

    for (const line of lines) {
      if (!line.trim()) continue;
      try {
        const rec = JSON.parse(line);
        const k = rec.k;
        if (!Array.isArray(k)) continue;

        if (k[0] === "requests" && typeof k[1] === "number") {
          if (k[1] > lastRequestIdx) lastRequestIdx = k[1];
        }
        if (
          rec.kind === 2 &&
          k.length === 1 &&
          k[0] === "requests" &&
          Array.isArray(rec.v)
        ) {
          const spliceEnd = (rec.offset || 0) + rec.v.length - 1;
          if (spliceEnd > lastRequestIdx) lastRequestIdx = spliceEnd;
        }

        if (
          k[0] === "requests" &&
          typeof k[1] === "number" &&
          k[2] === "modelState" &&
          typeof rec.v?.value === "number"
        ) {
          if (rec.v.value !== 2) {
            doneRequests.add(k[1]);
          }
        }
      } catch {}
    }

    if (lastRequestIdx < 0) return { active: false, lastRequestIdx: -1 };
    return {
      active: !doneRequests.has(lastRequestIdx),
      lastRequestIdx,
    };
  }

  function _chatSessionReadCreationDate(filePath) {
    try {
      const fd = fs.openSync(filePath, "r");
      try {
        const buf = Buffer.alloc(4096);
        const n = fs.readSync(fd, buf, 0, 4096, 0);
        const str = buf.slice(0, n).toString("utf8");
        const m = str.match(/"creationDate"\s*:\s*(\d+)/);
        if (m) return parseInt(m[1], 10);
      } finally {
        fs.closeSync(fd);
      }
    } catch {}
    return null;
  }

  async function searchArchivedChatHistory() {
    const query = await vscode.window.showInputBox({
      prompt: "Search archived Copilot chat history",
      placeHolder: "Enter words or a phrase to search for",
      ignoreFocusOut: true,
    });
    if (!query) return;

    const results = _chatHistoryArchive.searchArchive(query, { limit: 20 });
    if (results.length === 0) {
      const archiveRoot = _chatHistoryArchive.getArchiveRoot();
      vscode.window.showInformationMessage(
        archiveRoot
          ? `No archived chat matches found for "${query}".`
          : "Chat archive has not been initialized yet.",
      );
      return;
    }

    const doc = await vscode.workspace.openTextDocument({
      language: "markdown",
      content: _chatHistoryArchive.renderSearchResultsMarkdown(query, results),
    });
    await vscode.window.showTextDocument(doc, { preview: false });
  }

  function _onChatSessionWrite(sessionId, filePath) {
    const existing = _chatSessions.get(sessionId);
    const now = Date.now();

    const archiveEnabled = vscode.workspace
      .getConfiguration("gitShellHelpers.chatArchive")
      .get("enabled", true);
    if (archiveEnabled) {
      const archiveResult = _chatHistoryArchive.archiveSessionFile(sessionId, filePath, {
        title: existing?.title,
        maxBytes: ARCHIVE_CHUNK_BYTES,
      });
      if (archiveResult?.remainingBytes > 0) {
        _enqueueArchiveWork(sessionId, filePath, { title: existing?.title });
      }
    }

    const { tail, size: fileSize } = _chatSessionReadTail(filePath);
    if (!tail) return;

    if (existing && existing.lastSize === fileSize && !existing.active) return;

    let fileMtimeMs = 0;
    try {
      fileMtimeMs = fs.statSync(filePath).mtimeMs;
    } catch {}
    const fileStaleMs = Date.now() - fileMtimeMs;
    const forceCompleted = fileStaleMs > 300000;

    const { active: rawActive, lastRequestIdx } = _chatSessionParseState(tail);
    const isActive = rawActive && !forceCompleted;
    const title = _chatSessionReadTitle(filePath, existing?.title);
    _chatHistoryArchive.updateSessionMetadata(sessionId, {
      title,
      sourceFilePath: filePath,
    });

    const newPreview = _chatSessionExtractPreview(tail);
    let preview = newPreview || existing?.preview || null;

    let startedAt = existing?.startedAt;
    if (!startedAt || (existing && !existing.active && isActive)) {
      startedAt = _chatSessionReadCreationDate(filePath) || now;
    }

    if (isActive) {
      const activeAt = existing?.active
        ? existing.activeAt || existing.startedAt || startedAt
        : now;
      if (existing && existing.lastSize === fileSize && existing.active) {
        const staleMs = now - (existing._lastChangedAt || existing.startedAt);
        if (staleMs > 120000) {
          _chatSessions.set(sessionId, {
            title,
            active: false,
            startedAt,
            completedAt: existing._lastChangedAt || now,
            filePath,
            sessionId,
            lastSize: fileSize,
            preview: preview || existing?.preview || null,
            requestCount: lastRequestIdx + 1,
            _lastChangedAt: existing._lastChangedAt || now,
          });
          return;
        }
      }
      _chatSessions.set(sessionId, {
        title,
        active: true,
        startedAt,
        activeAt,
        completedAt: null,
        filePath,
        sessionId,
        lastSize: fileSize,
        preview: preview || "Working\u2026",
        requestCount: lastRequestIdx + 1,
        _lastChangedAt:
          existing?.lastSize !== fileSize
            ? now
            : existing?._lastChangedAt || now,
      });
    } else {
      const completedAt = existing?.active ? now : existing?.completedAt || now;
      _chatSessions.set(sessionId, {
        title,
        active: false,
        startedAt,
        activeAt: null,
        completedAt,
        filePath,
        sessionId,
        lastSize: fileSize,
        preview: preview || existing?.preview || null,
        requestCount: lastRequestIdx + 1,
        _lastChangedAt: now,
      });
    }
  }

  function startChatSessionWatcher(ctx) {
    _chatSessionWatcher?.close();
    _chatSessionWatcher = null;
    _clearArchiveQueue();
    if (_chatSessionPoller) {
      clearInterval(_chatSessionPoller);
      _chatSessionPoller = null;
    }

    const storageBase = ctx?.storageUri?.fsPath || ctx?.globalStorageUri?.fsPath;
    _chatHistoryArchive.initialize(_resolveArchiveRoot(ctx));

    const chatSessionsDir = _chatSessionsDir(ctx);
    if (!chatSessionsDir) return;

    let _lastScanMs = 0;
    let _didBootstrapArchive = false;
    const _scanRecentFiles = () => {
      const now = Date.now();
      if (now - _lastScanMs < 800) return;
      _lastScanMs = now;
      try {
        const files = fs
          .readdirSync(chatSessionsDir)
          .filter((f) => f.endsWith(".jsonl"))
          .map((f) => {
            const fp = path.join(chatSessionsDir, f);
            let mtimeMs = 0;
            try {
              mtimeMs = fs.statSync(fp).mtimeMs;
            } catch {}
            return { f, fp, sid: f.slice(0, -6), mtimeMs };
          })
          .sort((a, b) => b.mtimeMs - a.mtimeMs);

        if (!_didBootstrapArchive) {
          for (const file of files) {
            _enqueueArchiveWork(file.sid, file.fp);
          }
          _didBootstrapArchive = true;
        }

        const candidate = new Map();
        for (const [sid, sess] of _chatSessions) {
          if (sess?.active && sess.filePath) {
            candidate.set(sid, { sid, fp: sess.filePath });
          }
        }

        const recentFiles = files.filter((f) => now - f.mtimeMs < 300000);
        for (const file of recentFiles) {
          candidate.set(file.sid, { sid: file.sid, fp: file.fp });
        }

        for (const c of candidate.values()) {
          _onChatSessionWrite(c.sid, c.fp);
        }
      } catch {}
      _pushActivityUpdate();
    };

    _scanRecentFiles();

    _chatSessionWatcher = fs.watch(
      chatSessionsDir,
      { persistent: false },
      (_evt, filename) => {
        if (!filename) {
          _scanRecentFiles();
          return;
        }
        if (!filename.endsWith(".jsonl")) return;
        const sessionId = filename.slice(0, -6);
        _onChatSessionWrite(sessionId, path.join(chatSessionsDir, filename));
        _pushActivityUpdate();
      },
    );

    _chatSessionPoller = setInterval(_scanRecentFiles, 2000);
  }

  function dispose() {
    _chatSessionWatcher?.close();
    _chatSessionWatcher = null;
    _clearArchiveQueue();
    if (_chatSessionPoller) {
      clearInterval(_chatSessionPoller);
      _chatSessionPoller = null;
    }
  }

  return {
    getChatSessions,
    searchArchivedChatHistory,
    startChatSessionWatcher,
    dispose,
  };
};
