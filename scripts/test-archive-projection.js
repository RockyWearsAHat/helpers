#!/usr/bin/env node
"use strict";
// Smoke test: verify kind:0 records with binary attachments produce compact text projections
const crypto = require("crypto");
const fs = require("fs");
const os = require("os");
const path = require("path");
const zlib = require("zlib");

const factory = require("../vscode-extension/src/chat-history-archive");
const archive = factory();
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "archive-test-"));

archive.initialize(tmpDir);

// Synthetic kind:0 record with a binary PNG attachment (integer-keyed object)
const binaryRecord = JSON.stringify({
  kind: 0,
  v: {
    version: 3,
    creationDate: 1775158804584,
    initialLocation: "panel",
    responderUsername: "GitHub Copilot",
    sessionId: "test-session-01",
    requests: [],
    pendingRequests: [],
    inputState: {
      attachments: [
        {
          id: "file:///tmp/image.png",
          name: "image.png",
          value: Object.fromEntries(
            [...Array(500)].map((_, i) => [String(i), i % 256]),
          ),
        },
      ],
    },
    customTitle: "Test Session Title",
  },
});

const testFile = path.join(tmpDir, "test-session-01.jsonl");
fs.writeFileSync(testFile, binaryRecord + "\n");

const result = archive.archiveSessionFile("test-session-01", testFile, {});
if (!result) {
  console.error("FAIL: archiveSessionFile returned null");
  process.exit(1);
}

const manifestPath = path.join(
  tmpDir,
  "chat-history-archive",
  "index.json",
);
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const session = manifest.sessions["test-session-01"];

if (!session || !session.chunks || session.chunks.length === 0) {
  console.error("FAIL: no chunks written");
  process.exit(1);
}

const chunk = session.chunks[0];
const textFull = path.join(tmpDir, "chat-history-archive", chunk.textPath);
const textContent = zlib
  .brotliDecompressSync(fs.readFileSync(textFull))
  .toString("utf8");

console.log(`Raw JSONL bytes:     ${result.appendedBytes}`);
console.log(`Text projection:     ${textContent.length} bytes`);
console.log(`Projection content:\n${textContent}`);

const ratio = textContent.length / result.appendedBytes;
if (ratio > 0.1) {
  console.error(
    `FAIL: text projection is ${Math.round(ratio * 100)}% of raw — still too large`,
  );
  fs.rmSync(tmpDir, { recursive: true });
  process.exit(1);
}

// Also verify the projection contains the session title, not JSON garbage
if (!textContent.includes("Test Session Title")) {
  console.error("FAIL: session title not found in projection");
  fs.rmSync(tmpDir, { recursive: true });
  process.exit(1);
}

if (textContent.includes('"kind":0')) {
  console.error("FAIL: raw JSON leaked into projection");
  fs.rmSync(tmpDir, { recursive: true });
  process.exit(1);
}

console.log(`\nPASS: projection is compact (${Math.round(ratio * 100)}% of raw) and contains session metadata`);
fs.rmSync(tmpDir, { recursive: true });
process.exit(0);
