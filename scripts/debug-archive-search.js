#!/usr/bin/env node
"use strict";
const createChatHistoryArchive = require("../vscode-extension/src/chat-history-archive");
const fs = require("fs");
const os = require("os");
const path = require("path");
const zlib = require("zlib");

const sessionId = "debug-session";
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "dbg-"));
const archive = createChatHistoryArchive();

const hugeText = "tool summary " + "alpha beta gamma delta ".repeat(9000);
const lines = [
  JSON.stringify({ kind: 0, v: { version: 3, sessionId, creationDate: 1775085361803, requests: [] } }),
  JSON.stringify({ kind: 1, k: ["inputState", "inputText"], v: "Need a lossless chat archive." }),
  JSON.stringify({ kind: 2, k: ["requests", 0, "response"], v: [{ kind: "toolInvocationSerialized", invocationMessage: { value: hugeText } }] }),
];

const src = path.join(tmpRoot, sessionId + ".jsonl");
fs.mkdirSync(path.join(tmpRoot, "storage"), { recursive: true });
fs.writeFileSync(src, lines.join("\n") + "\n");

archive.initialize(path.join(tmpRoot, "storage"));
const r = archive.archiveSessionFile(sessionId, src, { title: "Test" });
console.log("Archive result:", JSON.stringify(r));

const manifestPath = path.join(tmpRoot, "storage", "chat-history-archive", "index.json");
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const sess = manifest.sessions[sessionId];
console.log("Chunks:", sess?.chunks?.length);

if (sess?.chunks?.length > 0) {
  const c = sess.chunks[0];
  const txtPath = path.join(tmpRoot, "storage", "chat-history-archive", c.textPath);
  const txt = zlib.brotliDecompressSync(fs.readFileSync(txtPath)).toString("utf8");
  console.log("Text projection size:", txt.length, "bytes");
  console.log("Contains 'alpha':", txt.includes("alpha"));
  console.log("Contains 'lossless':", txt.includes("lossless"));
  console.log("First 600 chars:\n", txt.slice(0, 600));
}

const results = archive.searchArchive("alpha beta gamma", { limit: 5 });
console.log("\nSearch results:", results.length);

fs.rmSync(tmpRoot, { recursive: true });
