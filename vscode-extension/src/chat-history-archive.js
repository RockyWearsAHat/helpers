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

    // Auto-compact: once a session accumulates 8+ L1 chunks, merge them into
    // a single L2 super-chunk so the archive stays lean without manual calls.
    const l1Count = session.chunks.filter(
      (c) => !c.id.startsWith("L2-"),
    ).length;
    if (l1Count >= 8) {
      _compactSessionL2(session);
    }

    _writeManifest();
    return {
      appendedBytes,
      chunksWritten,
      partialBytes: carry.length,
      sourceSize: stat.size,
    };
  }

  // Compact a session's L1 chunks into an L2 super-chunk.
  // Higher brotli quality (9) gives ~15-25% better compression than L1 (5).
  // Original .br files are kept on disk for lossless recovery.
  function _compactSessionL2(session) {
    const l1Chunks = session.chunks.filter((c) => !c.id.startsWith("L2-"));
    if (l1Chunks.length < 4) return;

    const rawBuffers = [];
    const textBuffers = [];
    let totalRaw = 0;
    let totalLines = 0;

    for (const chunk of l1Chunks) {
      try {
        const rawFull = path.join(_archiveRoot, chunk.rawPath);
        const textFull = path.join(_archiveRoot, chunk.textPath);
        const rawDecomp = zlib.brotliDecompressSync(fs.readFileSync(rawFull));
        const textDecomp = zlib.brotliDecompressSync(fs.readFileSync(textFull));
        rawBuffers.push(rawDecomp, NEWLINE_BUFFER);
        textBuffers.push(textDecomp, NEWLINE_BUFFER);
        totalRaw += rawDecomp.length;
        totalLines += chunk.lineCount || 0;
      } catch {
        // If any chunk file is missing, abort compaction for this session
        return;
      }
    }

    if (totalRaw === 0) return;

    const l2Quality = { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 9 } };
    const l2Id = `L2-${String(++session.chunkSeq).padStart(6, "0")}`;
    const sessionDir = path.join(
      _archiveRoot,
      "sessions",
      session.sessionId,
      "chunks",
    );
    fs.mkdirSync(sessionDir, { recursive: true });

    const mergedRaw = Buffer.concat(rawBuffers);
    const mergedText = Buffer.concat(textBuffers);

    fs.writeFileSync(
      path.join(sessionDir, `${l2Id}.jsonl.br`),
      zlib.brotliCompressSync(mergedRaw, l2Quality),
    );
    fs.writeFileSync(
      path.join(sessionDir, `${l2Id}.txt.br`),
      zlib.brotliCompressSync(mergedText, l2Quality),
    );

    const bloom = _buildBloomFilter(mergedText.toString("utf8"));
    const preview = _buildPreview(mergedText.toString("utf8"));

    session.chunks = session.chunks.filter((c) => c.id.startsWith("L2-"));
    session.chunks.push({
      id: l2Id,
      revision: session.revision,
      createdAt: l1Chunks[0].createdAt,
      updatedAt: Date.now(),
      rawBytes: totalRaw,
      textBytes: mergedText.length,
      lineCount: totalLines,
      rawPath: path.relative(
        _archiveRoot,
        path.join(sessionDir, `${l2Id}.jsonl.br`),
      ),
      textPath: path.relative(
        _archiveRoot,
        path.join(sessionDir, `${l2Id}.txt.br`),
      ),
      rawHash: crypto.createHash("sha256").update(mergedRaw).digest("hex"),
      bloom,
      preview,
      compactionLevel: 2,
      sourceChunkCount: l1Chunks.length,
    });
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
      // Not JSON — keep as plain text only if it's reasonably sized
      return trimmed.length < 2000 ? [trimmed] : [];
    }

    const lines = [];
    const keyPath = Array.isArray(record.k) ? record.k.join(".") : "record";

    if (record.kind === 0 && record.v) {
      // Initial full-state snapshot — extract only safe metadata fields.
      // Do NOT recurse into record.v directly: it may contain binary attachment
      // data encoded as integer-keyed objects (e.g. image PNG bytes).
      const v = record.v;
      if (v.customTitle) _appendProjectionLine(lines, "session", v.customTitle);
      if (typeof v.creationDate === "number") {
        lines.push(`created: ${new Date(v.creationDate).toISOString()}`);
      }
      if (typeof v.initialLocation === "string") {
        lines.push(`location: ${v.initialLocation}`);
      }
      if (typeof v.responderUsername === "string") {
        lines.push(`agent: ${v.responderUsername}`);
      }
      // Extract from requests only if they exist (kind:0 usually has none)
      if (Array.isArray(v.requests) && v.requests.length > 0) {
        for (const request of v.requests) {
          _collectInterestingText(request, lines, "request");
        }
      }
      // Never fall back to raw JSON — return whatever metadata we found
      return _dedupeLines(lines);
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
    } else if (record.kind === 2 && Array.isArray(record.k) && record.k.length >= 2) {
      // Delta update to a specific path — extract if the path is content-bearing
      const k = record.k;
      if (
        k[0] === "requests" &&
        (k.includes("content") || k.includes("text") || k.includes("message") ||
         k.includes("response") || k.includes("message"))
      ) {
        _collectInterestingText(record.v, lines, k[k.length - 1]);
      }
    } else if (record.kind === 1 && Array.isArray(record.k)) {
      // Individual property update — only extract known content-bearing paths
      const k = record.k;
      const contentKeys = new Set(["text", "content", "message", "response", "label"]);
      if (k[k.length - 1] && contentKeys.has(k[k.length - 1]) && typeof record.v === "string") {
        _appendProjectionLine(lines, k[k.length - 1], record.v);
      }
    }

    // Never fall back to raw JSON — return extracted lines only
    return _dedupeLines(lines);
  }

  // Keys that always contain binary/positional data — never worth extracting text from
  const _SKIP_KEYS = new Set([
    "attachments", "selections", "decorations", "cellExecutions",
    "diagnostics", "codeCoverageData", "implicitContext",
  ]);

  function _isBinaryObject(value) {
    // Detect integer-keyed objects used to encode binary buffers
    // e.g. {"0":137,"1":80,...} is a PNG/Buffer stored as a plain object
    if (typeof value !== "object" || Array.isArray(value) || value === null) return false;
    const keys = Object.keys(value);
    if (keys.length < 8) return false;
    return keys.slice(0, 8).every((k) => /^\d+$/.test(k));
  }

  function _collectInterestingText(value, output, label, depth = 0) {
    if (value == null || depth > 6) return;
    if (typeof value === "string") {
      // Truncate very long strings — keep enough for full-text searchability
      // while preventing megabyte model responses from bloating the projection.
      const text = value.length > 6000 ? `${value.slice(0, 6000)} [...]` : value;
      _appendProjectionLine(output, label, text);
      return;
    }
    if (Array.isArray(value)) {
      for (const item of value) {
        _collectInterestingText(item, output, label, depth + 1);
      }
      return;
    }
    if (typeof value !== "object") return;
    // Skip binary buffer objects
    if (_isBinaryObject(value)) return;

    if (value.kind === "progressMessage" && typeof value.content === "string") {
      _appendProjectionLine(output, "progress", value.content);
    }
    if (value.kind === "toolInvocationSerialized") {
      _collectInterestingText(value.invocationMessage, output, "tool", depth + 1);
      _collectInterestingText(value.pastTenseMessage, output, "tool", depth + 1);
    }

    for (const [key, child] of Object.entries(value)) {
      // Skip known binary/positional fields entirely
      if (_SKIP_KEYS.has(key)) continue;
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
        } else if (key === "value") {
          // "value" may be large model-generated content — route through the
          // truncating path instead of emitting directly to avoid huge lines.
          _collectInterestingText(child, output, label, depth + 1);
        }
        continue;
      }
      if (key === "value") {
        // Non-string "value" — recurse unless it's a binary buffer
        if (!_isBinaryObject(child)) {
          _collectInterestingText(child, output, label, depth + 1);
        }
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