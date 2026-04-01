"use strict";
// src/chat-history-archive.js — Lossless chunked archive for Copilot chat JSONL
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

const ARCHIVE_VERSION = 1;
const CHUNK_TARGET_BYTES = 128 * 1024;
const READ_BLOCK_BYTES = 64 * 1024;
const BROTLI_PARAMS = {
  params: {
    [zlib.constants.BROTLI_PARAM_QUALITY]: 5,
  },
};
const BLOOM_FILTER_BITS = 4096;
const BLOOM_FILTER_HASHES = 4;
const NEWLINE_BUFFER = Buffer.from("\n", "utf8");

module.exports = function createChatHistoryArchive() {
  let _archiveRoot = null;
  let _manifestPath = null;
  let _manifest = null;

  function initialize(storageRoot) {
    if (!storageRoot) return null;
    _archiveRoot = path.join(storageRoot, "chat-history-archive");
    fs.mkdirSync(_archiveRoot, { recursive: true });
    _manifestPath = path.join(_archiveRoot, "index.json");
    _manifest = _loadManifest();
    return _archiveRoot;
  }

  function getArchiveRoot() {
    return _archiveRoot;
  }

  function archiveSessionFile(sessionId, filePath, metadata = {}) {
    if (!_manifestPath || !sessionId || !filePath) return null;
    const stat = _safeStat(filePath);
    if (!stat?.isFile()) return null;

    const session = _ensureSessionState(sessionId);
    _mergeSessionMetadata(session, {
      ...metadata,
      sourceFilePath: filePath,
    });

    if (stat.size < session.archivedOffset) {
      session.revision += 1;
      session.archivedOffset = 0;
      session.partialLineBase64 = "";
    }

    if (stat.size === session.archivedOffset) {
      session.lastSourceSize = stat.size;
      session.updatedAt = Date.now();
      _writeManifest();
      return {
        appendedBytes: 0,
        chunksWritten: 0,
        partialBytes: session.partialLineBase64
          ? Buffer.from(session.partialLineBase64, "base64").length
          : 0,
        sourceSize: stat.size,
      };
    }

    const fd = fs.openSync(filePath, "r");
    let position = session.archivedOffset;
    let carry = session.partialLineBase64
      ? Buffer.from(session.partialLineBase64, "base64")
      : Buffer.alloc(0);
    const pendingRawBuffers = [];
    const pendingTextLines = [];
    let pendingRawBytes = 0;
    let pendingLineCount = 0;
    let appendedBytes = 0;
    let chunksWritten = 0;

    const flushPendingChunk = () => {
      if (!pendingRawBytes) return;
      _writeChunk(
        session,
        sessionId,
        pendingRawBuffers,
        pendingTextLines,
        pendingLineCount,
      );
      pendingRawBuffers.length = 0;
      pendingTextLines.length = 0;
      pendingRawBytes = 0;
      pendingLineCount = 0;
      chunksWritten += 1;
    };

    try {
      while (position < stat.size) {
        const toRead = Math.min(READ_BLOCK_BYTES, stat.size - position);
        const readBuffer = Buffer.allocUnsafe(toRead);
        const bytesRead = fs.readSync(fd, readBuffer, 0, toRead, position);
        if (bytesRead <= 0) break;
        appendedBytes += bytesRead;
        position += bytesRead;
        carry = _drainCompleteLines(
          carry,
          readBuffer.subarray(0, bytesRead),
          (lineBuffer) => {
            pendingRawBuffers.push(lineBuffer, NEWLINE_BUFFER);
            pendingRawBytes += lineBuffer.length + 1;
            pendingLineCount += 1;
            pendingTextLines.push(
              ..._extractProjectionLines(lineBuffer.toString("utf8")),
            );
            if (pendingRawBytes >= CHUNK_TARGET_BYTES) {
              flushPendingChunk();
            }
          },
        );
      }
    } finally {
      fs.closeSync(fd);
    }

    flushPendingChunk();
    session.archivedOffset = position;
    session.partialLineBase64 = carry.length ? carry.toString("base64") : "";
    session.lastSourceSize = stat.size;
    session.updatedAt = Date.now();
    _writeManifest();
    return {
      appendedBytes,
      chunksWritten,
      partialBytes: carry.length,
      sourceSize: stat.size,
    };
  }

  function updateSessionMetadata(sessionId, metadata = {}) {
    if (!_manifestPath || !sessionId) return null;
    const session = _ensureSessionState(sessionId);
    _mergeSessionMetadata(session, metadata);
    session.updatedAt = Date.now();
    _writeManifest();
    return session;
  }

  function searchArchive(query, options = {}) {
    const trimmedQuery = typeof query === "string" ? query.trim() : "";
    if (!trimmedQuery) return [];

    const limit = Number.isInteger(options.limit) ? options.limit : 20;
    const exactNeedle = trimmedQuery.toLowerCase();
    const queryTokens = _tokenize(trimmedQuery);
    const manifest = _ensureManifest();
    const results = [];

    for (const session of Object.values(manifest.sessions)) {
      if (options.sessionId && session.sessionId !== options.sessionId) continue;
      for (const chunk of session.chunks) {
        if (
          queryTokens.length > 0 &&
          chunk.bloom &&
          !_bloomMaybeContains(chunk.bloom, queryTokens)
        ) {
          continue;
        }
        const text = _readCompressedUtf8(_resolveArchivePath(chunk.textPath));
        const haystack = text.toLowerCase();
        const exactMatch = haystack.includes(exactNeedle);
        const tokenMatches = queryTokens.filter((token) => haystack.includes(token));
        if (!exactMatch && tokenMatches.length !== queryTokens.length) continue;
        results.push({
          sessionId: session.sessionId,
          title: session.title || "Copilot Chat",
          chunkId: chunk.id,
          createdAt: chunk.createdAt,
          snippet: _buildSnippet(text, trimmedQuery, queryTokens),
          score:
            (exactMatch ? 100 : 0) +
            tokenMatches.length * 10 +
            (chunk.preview ? 1 : 0),
        });
      }
    }

    results.sort((left, right) => {
      if (right.score !== left.score) return right.score - left.score;
      return (right.createdAt || 0) - (left.createdAt || 0);
    });
    return results.slice(0, limit);
  }

  function renderSessionText(sessionId) {
    const session = _ensureManifest().sessions[sessionId];
    if (!session) return "";
    return session.chunks
      .map((chunk) => _readCompressedUtf8(_resolveArchivePath(chunk.textPath)))
      .filter(Boolean)
      .join("\n");
  }

  function renderSearchResultsMarkdown(query, results) {
    const lines = [
      `# Archived Chat Search`,
      "",
      `Query: ${query}`,
      `Matches: ${results.length}`,
      "",
    ];

    for (const result of results) {
      lines.push(`## ${result.title}`);
      lines.push("");
      lines.push(`- Session: ${result.sessionId}`);
      lines.push(`- Chunk: ${result.chunkId}`);
      lines.push(
        `- Recorded: ${result.createdAt ? new Date(result.createdAt).toISOString() : "unknown"}`,
      );
      lines.push("");
      lines.push("```text");
      lines.push(result.snippet || "(no snippet available)");
      lines.push("```");
      lines.push("");
    }

    return lines.join("\n");
  }

  function _loadManifest() {
    try {
      const raw = fs.readFileSync(_manifestPath, "utf8");
      const parsed = JSON.parse(raw);
      if (parsed?.version !== ARCHIVE_VERSION || !parsed.sessions) {
        throw new Error("unsupported archive version");
      }
      return parsed;
    } catch {
      return {
        version: ARCHIVE_VERSION,
        sessions: {},
      };
    }
  }

  function _ensureManifest() {
    if (!_manifest) {
      _manifest = _loadManifest();
    }
    return _manifest;
  }

  function _ensureSessionState(sessionId) {
    const manifest = _ensureManifest();
    if (!manifest.sessions[sessionId]) {
      manifest.sessions[sessionId] = {
        sessionId,
        title: null,
        sourceFilePath: null,
        archivedOffset: 0,
        partialLineBase64: "",
        chunkSeq: 0,
        revision: 1,
        lastSourceSize: 0,
        createdAt: Date.now(),
        updatedAt: Date.now(),
        chunks: [],
      };
    }
    return manifest.sessions[sessionId];
  }

  function _mergeSessionMetadata(session, metadata) {
    if (typeof metadata.title === "string" && metadata.title.trim()) {
      session.title = metadata.title.trim();
    }
    if (typeof metadata.sourceFilePath === "string" && metadata.sourceFilePath) {
      session.sourceFilePath = metadata.sourceFilePath;
    }
  }

  function _writeManifest() {
    if (!_manifestPath) return;
    fs.mkdirSync(path.dirname(_manifestPath), { recursive: true });
    fs.writeFileSync(_manifestPath, JSON.stringify(_manifest, null, 2));
  }

  function _safeStat(filePath) {
    try {
      return fs.statSync(filePath);
    } catch {
      return null;
    }
  }

  function _drainCompleteLines(carry, nextBuffer, onLine) {
    const combined = carry.length
      ? Buffer.concat([carry, nextBuffer])
      : Buffer.from(nextBuffer);
    let start = 0;
    while (start < combined.length) {
      const newlineIndex = combined.indexOf(0x0a, start);
      if (newlineIndex === -1) break;
      let lineBuffer = combined.subarray(start, newlineIndex);
      if (lineBuffer.length && lineBuffer[lineBuffer.length - 1] === 0x0d) {
        lineBuffer = lineBuffer.subarray(0, lineBuffer.length - 1);
      }
      onLine(Buffer.from(lineBuffer));
      start = newlineIndex + 1;
    }
    if (start >= combined.length) return Buffer.alloc(0);
    return Buffer.from(combined.subarray(start));
  }

  function _writeChunk(session, sessionId, rawBuffers, textLines, lineCount) {
    const chunkId = String(++session.chunkSeq).padStart(6, "0");
    const sessionDir = path.join(_archiveRoot, "sessions", sessionId, "chunks");
    fs.mkdirSync(sessionDir, { recursive: true });

    const rawPayload = Buffer.concat(rawBuffers);
    const textPayload = Buffer.from(textLines.join("\n"), "utf8");
    const rawFilePath = path.join(sessionDir, `${chunkId}.jsonl.br`);
    const textFilePath = path.join(sessionDir, `${chunkId}.txt.br`);

    fs.writeFileSync(
      rawFilePath,
      zlib.brotliCompressSync(rawPayload, BROTLI_PARAMS),
    );
    fs.writeFileSync(
      textFilePath,
      zlib.brotliCompressSync(textPayload, BROTLI_PARAMS),
    );

    const rawRelativePath = path.relative(_archiveRoot, rawFilePath);
    const textRelativePath = path.relative(_archiveRoot, textFilePath);
    const preview = _buildPreview(textPayload.toString("utf8"));
    session.chunks.push({
      id: chunkId,
      revision: session.revision,
      createdAt: Date.now(),
      rawBytes: rawPayload.length,
      textBytes: textPayload.length,
      lineCount,
      rawPath: rawRelativePath,
      textPath: textRelativePath,
      rawHash: crypto.createHash("sha256").update(rawPayload).digest("hex"),
      bloom: _buildBloomFilter(textPayload.toString("utf8")),
      preview,
    });
  }

  function _extractProjectionLines(line) {
    const trimmed = line.trim();
    if (!trimmed) return [];

    let record = null;
    try {
      record = JSON.parse(trimmed);
    } catch {
      return [trimmed];
    }

    const lines = [];
    const keyPath = Array.isArray(record.k) ? record.k.join(".") : "record";

    if (record.kind === 0 && record.v) {
      _appendProjectionLine(lines, "session", record.v.customTitle);
      if (Array.isArray(record.v.requests)) {
        for (const request of record.v.requests) {
          _collectInterestingText(request, lines, "request");
        }
      }
      _collectInterestingText(record.v, lines, "session");
      const dedupedRoot = _dedupeLines(lines);
      return dedupedRoot.length > 0 ? dedupedRoot : [trimmed];
    }

    if (
      Array.isArray(record.k) &&
      record.k[0] === "customTitle" &&
      typeof record.v === "string"
    ) {
      _appendProjectionLine(lines, "title", record.v);
    }

    if (
      Array.isArray(record.k) &&
      record.k.includes("inputText") &&
      typeof record.v === "string"
    ) {
      _appendProjectionLine(lines, "draft", record.v);
    }

    if (
      record.kind === 2 &&
      Array.isArray(record.k) &&
      record.k.length === 1 &&
      record.k[0] === "requests" &&
      Array.isArray(record.v)
    ) {
      for (const request of record.v) {
        _collectInterestingText(request, lines, "request");
      }
    }

    _collectInterestingText(record.v, lines, keyPath);
    const deduped = _dedupeLines(lines);
    return deduped.length > 0 ? deduped : [trimmed];
  }

  function _collectInterestingText(value, output, label, depth = 0) {
    if (value == null || depth > 7) return;
    if (typeof value === "string") {
      _appendProjectionLine(output, label, value);
      return;
    }
    if (Array.isArray(value)) {
      for (const item of value) {
        _collectInterestingText(item, output, label, depth + 1);
      }
      return;
    }
    if (typeof value !== "object") return;

    if (value.kind === "progressMessage" && typeof value.content === "string") {
      _appendProjectionLine(output, "progress", value.content);
    }
    if (value.kind === "toolInvocationSerialized") {
      _collectInterestingText(value.invocationMessage, output, "tool", depth + 1);
      _collectInterestingText(value.pastTenseMessage, output, "tool", depth + 1);
    }

    for (const [key, child] of Object.entries(value)) {
      if (key === "requests" || key === "response") {
        _collectInterestingText(child, output, key, depth + 1);
        continue;
      }
      if (typeof child === "string") {
        if (
          key === "text" ||
          key === "prompt" ||
          key === "message" ||
          key === "content" ||
          key === "title" ||
          key === "label" ||
          key === "query"
        ) {
          _appendProjectionLine(output, key, child);
        }
        continue;
      }
      if (key === "value") {
        _collectInterestingText(child, output, label, depth + 1);
        continue;
      }
      if (
        key === "invocationMessage" ||
        key === "pastTenseMessage" ||
        key === "message" ||
        key === "response" ||
        key === "part"
      ) {
        _collectInterestingText(child, output, key, depth + 1);
      }
    }
  }

  function _appendProjectionLine(lines, label, value) {
    if (typeof value !== "string") return;
    const normalized = value.replace(/\s+/g, " ").trim();
    if (!normalized) return;
    lines.push(`${label}: ${normalized}`);
  }

  function _dedupeLines(lines) {
    return [...new Set(lines)];
  }

  function _tokenize(text) {
    const matches = String(text).toLowerCase().match(/[a-z0-9][a-z0-9._:/-]{1,63}/g);
    return matches ? [...new Set(matches)] : [];
  }

  function _buildBloomFilter(text) {
    const filter = Buffer.alloc(BLOOM_FILTER_BITS / 8);
    for (const token of _tokenize(text)) {
      _bloomAdd(filter, token);
    }
    return filter.toString("base64");
  }

  function _bloomAdd(filter, token) {
    const digest = crypto.createHash("sha1").update(token).digest();
    for (let index = 0; index < BLOOM_FILTER_HASHES; index++) {
      const offset = digest.readUInt32BE(index * 4) % BLOOM_FILTER_BITS;
      filter[Math.floor(offset / 8)] |= 1 << (offset % 8);
    }
  }

  function _bloomMaybeContains(base64, tokens) {
    if (!base64) return true;
    const filter = Buffer.from(base64, "base64");
    for (const token of tokens) {
      const digest = crypto.createHash("sha1").update(token).digest();
      for (let index = 0; index < BLOOM_FILTER_HASHES; index++) {
        const offset = digest.readUInt32BE(index * 4) % BLOOM_FILTER_BITS;
        const present = filter[Math.floor(offset / 8)] & (1 << (offset % 8));
        if (!present) return false;
      }
    }
    return true;
  }

  function _buildPreview(text) {
    const firstLine = String(text)
      .split("\n")
      .map((line) => line.trim())
      .find(Boolean);
    if (!firstLine) return null;
    return firstLine.length > 140 ? `${firstLine.slice(0, 137)}...` : firstLine;
  }

  function _buildSnippet(text, query, tokens) {
    const lines = String(text)
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
    const lowerQuery = query.toLowerCase();
    for (const line of lines) {
      const lowerLine = line.toLowerCase();
      if (
        lowerLine.includes(lowerQuery) ||
        tokens.every((token) => lowerLine.includes(token))
      ) {
        return line.length > 320 ? `${line.slice(0, 317)}...` : line;
      }
    }
    const firstLine = lines[0] || "";
    return firstLine.length > 320 ? `${firstLine.slice(0, 317)}...` : firstLine;
  }

  function _readCompressedUtf8(filePath) {
    try {
      const raw = fs.readFileSync(filePath);
      return zlib.brotliDecompressSync(raw).toString("utf8");
    } catch {
      return "";
    }
  }

  function _resolveArchivePath(relativePath) {
    return path.join(_archiveRoot, relativePath);
  }

  return {
    initialize,
    getArchiveRoot,
    archiveSessionFile,
    updateSessionMetadata,
    searchArchive,
    renderSessionText,
    renderSearchResultsMarkdown,
  };
};