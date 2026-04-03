"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const zlib = require("zlib");

const createChatHistoryArchive = require("../vscode-extension/src/chat-history-archive");

const sessionId = "session-under-test";
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "gsh-chat-history-archive-"));
const storageRoot = path.join(tmpRoot, "storage");
const sourceFile = path.join(tmpRoot, `${sessionId}.jsonl`);
const archive = createChatHistoryArchive();

function makeResponseLine(text) {
  return JSON.stringify({
    kind: 2,
    k: ["requests", 0, "response"],
    v: [
      {
        kind: "toolInvocationSerialized",
        invocationMessage: {
          value: text,
        },
      },
    ],
  });
}

try {
  const hugeToolMessage = `tool summary ${"alpha beta gamma delta ".repeat(9000)}`;
  const initialLines = [
    JSON.stringify({
      kind: 0,
      v: {
        version: 3,
        sessionId,
        creationDate: 1775085361803,
        requests: [],
      },
    }),
    JSON.stringify({
      kind: 1,
      k: ["inputState", "inputText"],
      v: "Need a lossless chat archive that can be searched safely.",
    }),
    makeResponseLine(hugeToolMessage),
  ];

  fs.mkdirSync(storageRoot, { recursive: true });
  fs.writeFileSync(sourceFile, `${initialLines.join("\n")}\n`, "utf8");

  archive.initialize(storageRoot);
  const firstRun = archive.archiveSessionFile(sessionId, sourceFile, {
    title: "Archive Test Session",
  });
  assert.ok(firstRun, "first archive pass should produce a result");
  assert.ok(firstRun.chunksWritten >= 1, "first archive pass should write at least one chunk");

  const searchResults = archive.searchArchive("alpha beta gamma", { limit: 5 });
  assert.strictEqual(searchResults.length, 1, "search should find the archived chunk");
  assert.ok(
    searchResults[0].snippet.toLowerCase().includes("alpha beta gamma"),
    "search snippet should include the query text",
  );

  const renderedText = archive.renderSessionText(sessionId);
  assert.ok(
    renderedText.includes("lossless chat archive"),
    "rendered session text should include user input",
  );
  assert.ok(
    renderedText.includes("alpha beta gamma"),
    "rendered session text should include projected response text",
  );

  const archiveRoot = archive.getArchiveRoot();
  const manifest = JSON.parse(
    fs.readFileSync(path.join(archiveRoot, "index.json"), "utf8"),
  );
  const rawChunkFiles = manifest.sessions[sessionId].chunks.map((chunk) =>
    path.join(archiveRoot, chunk.rawPath),
  );
  const reconstructedRaw = rawChunkFiles
    .map((filePath) => zlib.brotliDecompressSync(fs.readFileSync(filePath)).toString("utf8"))
    .join("");
  assert.strictEqual(
    reconstructedRaw,
    fs.readFileSync(sourceFile, "utf8"),
    "raw archive chunks should reconstruct the original JSONL exactly",
  );

  const appendedLine = JSON.stringify({
    kind: 1,
    k: ["customTitle"],
    v: "Archived Session Title",
  });
  fs.appendFileSync(sourceFile, `${appendedLine}\n`, "utf8");

  const secondRun = archive.archiveSessionFile(sessionId, sourceFile, {
    title: "Archived Session Title",
  });
  assert.ok(secondRun, "second archive pass should produce a result");
  assert.ok(secondRun.appendedBytes > 0, "second archive pass should read appended bytes");

  const thirdRun = archive.archiveSessionFile(sessionId, sourceFile, {
    title: "Archived Session Title",
  });
  assert.ok(thirdRun, "third archive pass should produce a result");
  assert.strictEqual(thirdRun.appendedBytes, 0, "unchanged archive pass should not reread data");
  assert.strictEqual(thirdRun.chunksWritten, 0, "unchanged archive pass should not add chunks");

  const partialArchive = createChatHistoryArchive();
  const partialStorageRoot = path.join(tmpRoot, "partial-storage");
  const partialSessionId = "session-partial";
  fs.mkdirSync(partialStorageRoot, { recursive: true });
  partialArchive.initialize(partialStorageRoot);

  let partialRun = partialArchive.archiveSessionFile(partialSessionId, sourceFile, {
    title: "Partial Archive Session",
    maxBytes: 4096,
  });
  assert.ok(partialRun, "partial archive pass should produce a result");
  assert.ok(
    partialRun.appendedBytes > 0,
    "partial archive pass should read an initial byte budget",
  );
  assert.ok(
    partialRun.remainingBytes > 0,
    "partial archive pass should leave remaining bytes for follow-up work",
  );
  assert.strictEqual(
    partialRun.complete,
    false,
    "partial archive pass should report incomplete state",
  );

  let safetyCounter = 0;
  while (partialRun.remainingBytes > 0) {
    partialRun = partialArchive.archiveSessionFile(partialSessionId, sourceFile, {
      title: "Partial Archive Session",
      maxBytes: 4096,
    });
    safetyCounter += 1;
    assert.ok(
      safetyCounter < 2000,
      "partial archive should complete in a bounded number of passes",
    );
  }

  const partialArchiveRoot = partialArchive.getArchiveRoot();
  const partialManifest = JSON.parse(
    fs.readFileSync(path.join(partialArchiveRoot, "index.json"), "utf8"),
  );
  const partialRawChunkFiles = partialManifest.sessions[partialSessionId].chunks.map(
    (chunk) => path.join(partialArchiveRoot, chunk.rawPath),
  );
  const reconstructedPartialRaw = partialRawChunkFiles
    .map((filePath) => zlib.brotliDecompressSync(fs.readFileSync(filePath)).toString("utf8"))
    .join("");
  assert.strictEqual(
    reconstructedPartialRaw,
    fs.readFileSync(sourceFile, "utf8"),
    "partial archive passes should still reconstruct the original JSONL exactly",
  );

  console.log("ok");
} finally {
  fs.rmSync(tmpRoot, { recursive: true, force: true });
}
