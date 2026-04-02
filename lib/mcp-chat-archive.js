"use strict";
// lib/mcp-chat-archive.js — MCP tools for searching the chat history archive
//
// The VS Code extension (chat-history-archive.js) continuously archives
// Copilot chat JSONL into Brotli-compressed chunks with Bloom filters and
// text projections. This module reads that archive and exposes it as MCP
// tools so agents can search past conversations, retrieve project direction
// (first user request), and inspect archive stats.
//
// Architecture:
//   Extension side: watches JSONL files → chunks → Brotli compress → manifest
//   MCP side (this file): reads manifest + compressed chunks → search + decompress
//
// Compaction levels:
//   L0: Raw JSONL (what VS Code writes, ephemeral — extension consumes it)
//   L1: Brotli-compressed chunks with text projections + Bloom filters (extension writes)
//   L2: Merged super-chunks with combined indexes (this module, on-demand)
//
// All compaction is lossless — raw JSONL .br files are always preserved.

const fs = require("fs");
const path = require("path");
const zlib = require("zlib");
const crypto = require("crypto");

// ─── Tool schemas ───────────────────────────────────────────────────────────

const CHAT_ARCHIVE_TOOLS = [
  {
    name: "search_chat_history",
    description:
      "Search the archived Copilot chat history across all sessions. The archive captures EVERYTHING said in chat — every user message, every assistant response, every tool call. Returns matching snippets with session context. Always searches BOTH the current project archive AND the global archive (sessions started without a workspace folder open — common for general 'chatting with an agent' conversations). Results are tagged with scope: 'project' or 'global'. Use this to recall past conversations, find previous solutions, understand why something was built a certain way, or clarify anything the user may have explained before. When confused about context that might have been discussed previously, ALWAYS search here first.",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description:
            "Search query — words or phrases to find in past chat sessions.",
        },
        session_id: {
          type: "string",
          description: "Optional. Restrict search to a specific session ID.",
        },
        date_from: {
          type: "string",
          description:
            "Optional. ISO 8601 date string (e.g. '2026-03-01') — only return results from on or after this date.",
        },
        date_to: {
          type: "string",
          description:
            "Optional. ISO 8601 date string (e.g. '2026-04-01') — only return results from on or before this date.",
        },
        recent_only: {
          type: "boolean",
          description:
            "Shorthand for the last 7 days. Overrides date_from/date_to when true. Default: false.",
        },
        max_results: {
          type: "integer",
          description: "Number of results to return (1-50). Default: 10.",
        },
      },
      required: ["query"],
    },
  },
  {
    name: "get_project_direction",
    description:
      "Retrieve the project direction — the first meaningful user request from the earliest chat session in this workspace. This captures the user's original intent: 'this is what I want to build'. Also returns any explicitly tagged direction entries from session memory.",
    inputSchema: {
      type: "object",
      properties: {
        include_recent: {
          type: "boolean",
          description:
            "Also include the most recent session's first request for comparison with originaldirection. Default: false.",
        },
      },
      required: [],
    },
  },
  {
    name: "get_chat_archive_stats",
    description:
      "Get statistics about the chat history archive: total sessions, chunks, raw vs compressed sizes, compaction ratios, and storage usage. Use to understand archive health and storage pressure.",
    inputSchema: {
      type: "object",
      properties: {},
      required: [],
    },
  },
  {
    name: "compact_chat_archive",
    description:
      "Run multi-level compaction on the chat archive. Merges small L1 chunks into larger L2 super-chunks with combined Bloom filters and merged text indexes. Reduces file count and improves search speed. Compaction is lossless — original compressed data is preserved and can always be decompressed. Safe to run at any time.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description:
            "Optional. Compact only a specific session. Default: compact all sessions with eligible chunks.",
        },
        min_chunks: {
          type: "integer",
          description:
            "Minimum number of L1 chunks before merging into an L2 super-chunk. Default: 4.",
        },
      },
      required: [],
    },
  },
];

// ─── Constants ──────────────────────────────────────────────────────────────

const BLOOM_FILTER_BITS = 8192; // Larger for merged super-chunks
const BLOOM_FILTER_HASHES = 4;
const L2_CHUNK_PREFIX = "L2-";
const DIRECTION_FILE = "_project-direction.json";

// ─── Handler factory ────────────────────────────────────────────────────────

function createHandler(deps) {
  const { archiveRoot, globalArchiveRoot = null, workspaceRoots = [] } = deps;

  function manifestPath() {
    return path.join(archiveRoot, "index.json");
  }

  function readManifest() {
    try {
      const raw = fs.readFileSync(manifestPath(), "utf8");
      const parsed = JSON.parse(raw);
      if (parsed?.version && parsed.sessions) return parsed;
    } catch {
      // Archive not initialized yet
    }
    return { version: 1, sessions: {} };
  }

  function writeManifest(manifest) {
    fs.mkdirSync(archiveRoot, { recursive: true });
    fs.writeFileSync(manifestPath(), JSON.stringify(manifest, null, 2));
  }

  function readCompressedUtf8(relativePath) {
    try {
      const fullPath = path.join(archiveRoot, relativePath);
      const raw = fs.readFileSync(fullPath);
      return zlib.brotliDecompressSync(raw).toString("utf8");
    } catch {
      return "";
    }
  }

  function tokenize(text) {
    const matches = String(text)
      .toLowerCase()
      .match(/[a-z0-9][a-z0-9._:/-]{1,63}/g);
    return matches ? [...new Set(matches)] : [];
  }

  function bloomMaybeContains(base64, tokens) {
    if (!base64) return true;
    const filter = Buffer.from(base64, "base64");
    for (const token of tokens) {
      const digest = crypto.createHash("sha1").update(token).digest();
      for (let i = 0; i < BLOOM_FILTER_HASHES; i++) {
        const bits = filter.length * 8;
        const offset = digest.readUInt32BE(i * 4) % bits;
        const present = filter[Math.floor(offset / 8)] & (1 << (offset % 8));
        if (!present) return false;
      }
    }
    return true;
  }

  function bloomAdd(filter, token) {
    const digest = crypto.createHash("sha1").update(token).digest();
    const bits = filter.length * 8;
    for (let i = 0; i < BLOOM_FILTER_HASHES; i++) {
      const offset = digest.readUInt32BE(i * 4) % bits;
      filter[Math.floor(offset / 8)] |= 1 << (offset % 8);
    }
  }

  function buildBloomFilter(text, bitCount) {
    const filter = Buffer.alloc(bitCount / 8);
    for (const token of tokenize(text)) {
      bloomAdd(filter, token);
    }
    return filter.toString("base64");
  }

  // ─── Search ─────────────────────────────────────────────────────────────

  // Scan one archive root and return matching results tagged with `scope`.
  function searchOneArchive(root, scope, queryTokens, exactNeedle, sessionFilter, dateFrom, dateTo) {
    const results = [];
    let sessionsSearched = 0;

    let manifest;
    try {
      const raw = fs.readFileSync(path.join(root, "index.json"), "utf8");
      const parsed = JSON.parse(raw);
      manifest = parsed?.version && parsed.sessions ? parsed : { version: 1, sessions: {} };
    } catch {
      manifest = { version: 1, sessions: {} };
    }

    for (const session of Object.values(manifest.sessions)) {
      if (sessionFilter && session.sessionId !== sessionFilter) continue;
      sessionsSearched += 1;
      for (const chunk of session.chunks || []) {
        const chunkTime = chunk.updatedAt || chunk.createdAt || 0;
        if (dateFrom && chunkTime < dateFrom) continue;
        if (dateTo && chunkTime > dateTo) continue;

        if (
          queryTokens.length > 0 &&
          chunk.bloom &&
          !bloomMaybeContains(chunk.bloom, queryTokens)
        ) {
          continue;
        }

        let text = "";
        try {
          const fullPath = path.join(root, chunk.textPath);
          text = zlib.brotliDecompressSync(fs.readFileSync(fullPath)).toString("utf8");
        } catch {
          continue;
        }
        if (!text) continue;

        const haystack = text.toLowerCase();
        const exactMatch = haystack.includes(exactNeedle);
        const tokenMatches = queryTokens.filter((t) => haystack.includes(t));
        if (!exactMatch && tokenMatches.length !== queryTokens.length) continue;

        const snippet = buildSnippet(text, exactNeedle, queryTokens);
        results.push({
          sessionId: session.sessionId,
          title: session.title || "Copilot Chat",
          chunkId: chunk.id,
          createdAt: chunk.createdAt,
          updatedAt: chunkTime,
          date: chunkTime ? new Date(chunkTime).toISOString().slice(0, 10) : null,
          rawBytes: chunk.rawBytes || 0,
          compressedLevel: chunk.id.startsWith(L2_CHUNK_PREFIX) ? "L2" : "L1",
          scope,
          snippet,
          score:
            (exactMatch ? 100 : 0) +
            tokenMatches.length * 10 +
            (chunk.preview ? 1 : 0),
        });
      }
    }

    return { results, sessionsSearched };
  }

  function searchChatHistory(args) {
    const query = String(args.query || "").trim();
    if (!query) return { results: [], message: "Query is required." };

    const maxResults = Math.min(
      50,
      Math.max(1, parseInt(args.max_results, 10) || 10),
    );
    const sessionFilter = args.session_id || null;
    const exactNeedle = query.toLowerCase();
    const queryTokens = tokenize(query);

    // Date range filtering
    let dateFrom = null;
    let dateTo = null;
    if (args.recent_only === true) {
      dateFrom = Date.now() - 7 * 24 * 60 * 60 * 1000;
    } else {
      if (args.date_from) {
        const d = Date.parse(args.date_from);
        if (!isNaN(d)) dateFrom = d;
      }
      if (args.date_to) {
        const d = Date.parse(args.date_to);
        if (!isNaN(d)) dateTo = d + 86400000; // inclusive end of day
      }
    }

    // Search project-scoped archive
    const projectSearch = searchOneArchive(
      archiveRoot,
      "project",
      queryTokens,
      exactNeedle,
      sessionFilter,
      dateFrom,
      dateTo,
    );
    const allResults = [...projectSearch.results];
    let totalSessionsSearched = projectSearch.sessionsSearched;

    // Also search global (no-folder) archive when it differs from the project archive.
    // This covers sessions started without any workspace folder open, which the user
    // often mentions as "I was chatting with an agent" without a specific project context.
    const effectiveGlobal = globalArchiveRoot;
    if (effectiveGlobal && effectiveGlobal !== archiveRoot) {
      const globalSearch = searchOneArchive(
        effectiveGlobal,
        "global",
        queryTokens,
        exactNeedle,
        sessionFilter,
        dateFrom,
        dateTo,
      );
      // De-duplicate: a session+chunk pair should never appear twice, but guard anyway
      const seen = new Set(allResults.map((r) => `${r.sessionId}::${r.chunkId}`));
      for (const r of globalSearch.results) {
        const key = `${r.sessionId}::${r.chunkId}`;
        if (!seen.has(key)) {
          seen.add(key);
          allResults.push(r);
        }
      }
      totalSessionsSearched += globalSearch.sessionsSearched;
    }

    allResults.sort((a, b) => {
      if (b.score !== a.score) return b.score - a.score;
      return (b.updatedAt || 0) - (a.updatedAt || 0);
    });

    return {
      results: allResults.slice(0, maxResults),
      totalMatches: allResults.length,
      sessionsSearched: totalSessionsSearched,
    };
  }

  function buildSnippet(text, query, tokens) {
    const lines = String(text)
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean);
    const lowerQuery = query.toLowerCase();
    for (const line of lines) {
      const lower = line.toLowerCase();
      if (
        lower.includes(lowerQuery) ||
        tokens.every((t) => lower.includes(t))
      ) {
        return line.length > 400 ? line.slice(0, 397) + "..." : line;
      }
    }
    const first = lines[0] || "";
    return first.length > 400 ? first.slice(0, 397) + "..." : first;
  }

  // ─── Project Direction ──────────────────────────────────────────────────

  function readReadmeDirection() {
    for (const root of workspaceRoots) {
      const readmePath = path.join(root, "README.md");
      try {
        const text = fs.readFileSync(readmePath, "utf8");
        // Extract up to 3 meaningful non-heading paragraphs as direction summary
        const lines = text.split("\n");
        const paragraphs = [];
        let current = [];
        for (const line of lines) {
          if (line.startsWith("#")) {
            if (current.length) {
              paragraphs.push(current.join(" ").trim());
              current = [];
            }
            continue;
          }
          if (line.trim() === "") {
            if (current.length) {
              paragraphs.push(current.join(" ").trim());
              current = [];
            }
          } else {
            current.push(line.trim());
          }
        }
        if (current.length) paragraphs.push(current.join(" ").trim());
        const meaningful = paragraphs.filter((p) => p.length > 30).slice(0, 3);
        if (meaningful.length) {
          const summary = meaningful.join(" ").slice(0, 600);
          return { direction: summary, source: "readme", readmePath };
        }
      } catch {
        // README not present or unreadable in this root
      }
    }
    return null;
  }

  function getProjectDirection(args) {
    const includeRecent = args.include_recent === true;

    // Primary: derive direction from README.md in workspace roots
    const readmeResult = readReadmeDirection();
    if (readmeResult && !args.force_chat_detection) {
      return readmeResult;
    }

    const manifest = readManifest();
    const sessions = Object.values(manifest.sessions);

    if (!sessions.length) {
      return (
        readmeResult || {
          direction: null,
          message: "No chat sessions archived yet.",
        }
      );
    }

    // Sort sessions by creation date to find the earliest
    sessions.sort((a, b) => (a.createdAt || 0) - (b.createdAt || 0));
    const earliest = sessions[0];
    const firstRequest = extractFirstUserRequest(earliest);

    // Check for stored direction override
    const directionPath = path.join(archiveRoot, DIRECTION_FILE);
    let storedDirection = null;
    try {
      storedDirection = JSON.parse(fs.readFileSync(directionPath, "utf8"));
    } catch {
      // No stored direction yet
    }

    const result = {
      direction: storedDirection?.direction || firstRequest,
      source: storedDirection ? "explicit" : "auto-detected",
      sessionId: earliest.sessionId,
      sessionTitle: earliest.title,
      detectedAt: storedDirection?.detectedAt || earliest.createdAt,
    };

    if (includeRecent && sessions.length > 1) {
      const latest = sessions[sessions.length - 1];
      result.recentFirstRequest = extractFirstUserRequest(latest);
      result.recentSessionId = latest.sessionId;
      result.recentSessionTitle = latest.title;
    }

    // Auto-save direction if not already stored
    if (!storedDirection && firstRequest) {
      const dirData = {
        direction: firstRequest,
        sessionId: earliest.sessionId,
        detectedAt: earliest.createdAt || Date.now(),
        autoDetected: true,
      };
      try {
        fs.mkdirSync(path.dirname(directionPath), { recursive: true });
        fs.writeFileSync(directionPath, JSON.stringify(dirData, null, 2));
      } catch {
        // Non-critical — we still return the detected direction
      }
    }

    return result;
  }

  function extractFirstUserRequest(session) {
    if (!session?.chunks?.length) return null;

    // Read the first chunk's text projection and look for the first user message
    const firstChunkText = readCompressedUtf8(session.chunks[0].textPath);
    if (!firstChunkText) return null;

    const lines = firstChunkText.split("\n");
    for (const line of lines) {
      const trimmed = line.trim();
      // Text projections use "label: content" format
      // User messages appear as "text: ...", "prompt: ...", "message: ...", "query: ..."
      const userPrefixes = [
        "text:",
        "prompt:",
        "message:",
        "query:",
        "request:",
      ];
      for (const prefix of userPrefixes) {
        if (trimmed.toLowerCase().startsWith(prefix)) {
          const content = trimmed.slice(prefix.length).trim();
          // Skip very short or obviously system/tool messages
          if (
            content.length >= 10 &&
            !content.startsWith("{") &&
            !content.startsWith("[")
          ) {
            return content.length > 500
              ? content.slice(0, 500) + "..."
              : content;
          }
        }
      }
    }

    // Fallback: try reading raw JSONL from the first chunk
    const rawChunk = session.chunks[0];
    if (rawChunk?.rawPath) {
      const raw = readCompressedUtf8(rawChunk.rawPath);
      if (raw) {
        for (const line of raw.split("\n")) {
          if (!line.trim()) continue;
          try {
            const rec = JSON.parse(line);
            // kind 0 is the initial state record which may contain requests
            if (rec.kind === 0 && rec.v?.requests) {
              for (const req of rec.v.requests) {
                const msg =
                  req?.message?.text ||
                  req?.message ||
                  req?.text ||
                  req?.prompt;
                if (typeof msg === "string" && msg.trim().length >= 10) {
                  const text = msg.trim();
                  return text.length > 500 ? text.slice(0, 500) + "..." : text;
                }
              }
            }
            // kind 2 splice records for requests array
            if (
              rec.kind === 2 &&
              Array.isArray(rec.k) &&
              rec.k[0] === "requests" &&
              rec.k.length === 1 &&
              Array.isArray(rec.v)
            ) {
              for (const req of rec.v) {
                const msg =
                  req?.message?.text ||
                  req?.message ||
                  req?.text ||
                  req?.prompt;
                if (typeof msg === "string" && msg.trim().length >= 10) {
                  const text = msg.trim();
                  return text.length > 500 ? text.slice(0, 500) + "..." : text;
                }
              }
            }
          } catch {
            // Malformed line — skip
          }
        }
      }
    }

    return null;
  }

  // ─── Archive Stats ──────────────────────────────────────────────────────

  function getChatArchiveStats() {
    const manifest = readManifest();
    const sessions = Object.values(manifest.sessions);

    let totalChunks = 0;
    let totalRawBytes = 0;
    let totalTextBytes = 0;
    let totalCompressedBytes = 0;
    let l1Chunks = 0;
    let l2Chunks = 0;

    for (const session of sessions) {
      for (const chunk of session.chunks || []) {
        totalChunks += 1;
        totalRawBytes += chunk.rawBytes || 0;
        totalTextBytes += chunk.textBytes || 0;

        if (chunk.id.startsWith(L2_CHUNK_PREFIX)) {
          l2Chunks += 1;
        } else {
          l1Chunks += 1;
        }

        // Measure actual compressed file sizes on disk
        if (chunk.rawPath) {
          try {
            const stat = fs.statSync(path.join(archiveRoot, chunk.rawPath));
            totalCompressedBytes += stat.size;
          } catch {
            // File may be missing
          }
        }
        if (chunk.textPath) {
          try {
            const stat = fs.statSync(path.join(archiveRoot, chunk.textPath));
            totalCompressedBytes += stat.size;
          } catch {
            // File may be missing
          }
        }
      }
    }

    const compressionRatio =
      totalRawBytes > 0
        ? Math.round((1 - totalCompressedBytes / totalRawBytes) * 100)
        : 0;

    return {
      archiveRoot,
      totalSessions: sessions.length,
      totalChunks,
      l1Chunks,
      l2Chunks,
      totalRawBytes,
      totalRawMB: Math.round((totalRawBytes / 1024 / 1024) * 100) / 100,
      totalTextBytes,
      totalCompressedBytes,
      totalCompressedMB:
        Math.round((totalCompressedBytes / 1024 / 1024) * 100) / 100,
      compressionRatio: `${compressionRatio}%`,
      sessions: sessions.map((s) => ({
        sessionId: s.sessionId,
        title: s.title,
        chunks: (s.chunks || []).length,
        createdAt: s.createdAt,
        updatedAt: s.updatedAt,
      })),
    };
  }

  // ─── L2 Compaction ──────────────────────────────────────────────────────

  function compactChatArchive(args) {
    const sessionFilter = args.session_id || null;
    const minChunks = Math.max(2, parseInt(args.min_chunks, 10) || 4);
    const manifest = readManifest();
    let totalMerged = 0;
    let totalCreated = 0;

    for (const session of Object.values(manifest.sessions)) {
      if (sessionFilter && session.sessionId !== sessionFilter) continue;

      // Collect L1-only chunks eligible for merging
      const l1Chunks = (session.chunks || []).filter(
        (c) => !c.id.startsWith(L2_CHUNK_PREFIX),
      );

      if (l1Chunks.length < minChunks) continue;

      // Group L1 chunks into batches for merging
      const batchSize = Math.max(minChunks, 8);
      for (let i = 0; i + minChunks <= l1Chunks.length; i += batchSize) {
        const batch = l1Chunks.slice(i, i + batchSize);
        if (batch.length < minChunks) break;

        // Read and concatenate raw data from all chunks in the batch
        const rawBuffers = [];
        const textBuffers = [];
        let totalRaw = 0;
        let totalText = 0;
        let totalLines = 0;
        let allValid = true;

        for (const chunk of batch) {
          const rawData = readCompressedUtf8(chunk.rawPath);
          const textData = readCompressedUtf8(chunk.textPath);
          if (!rawData && !textData) {
            allValid = false;
            break;
          }
          const rawBuf = Buffer.from(rawData, "utf8");
          const textBuf = Buffer.from(textData, "utf8");
          rawBuffers.push(rawBuf);
          textBuffers.push(textBuf);
          totalRaw += rawBuf.length;
          totalText += textBuf.length;
          totalLines += chunk.lineCount || 0;
        }

        if (!allValid || totalRaw === 0) continue;

        // Create L2 super-chunk
        const l2Id = `${L2_CHUNK_PREFIX}${String(++session.chunkSeq).padStart(6, "0")}`;
        const sessionDir = path.join(
          archiveRoot,
          "sessions",
          session.sessionId,
          "chunks",
        );
        fs.mkdirSync(sessionDir, { recursive: true });

        const mergedRaw = Buffer.concat(rawBuffers);
        const mergedText = Buffer.concat(textBuffers);

        // Compress at higher quality for L2 (quality 9 vs L1's quality 5)
        const l2BrotliParams = {
          params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 9 },
        };

        const rawPath = path.join(sessionDir, `${l2Id}.jsonl.br`);
        const textPath = path.join(sessionDir, `${l2Id}.txt.br`);

        fs.writeFileSync(
          rawPath,
          zlib.brotliCompressSync(mergedRaw, l2BrotliParams),
        );
        fs.writeFileSync(
          textPath,
          zlib.brotliCompressSync(mergedText, l2BrotliParams),
        );

        // Build merged Bloom filter (larger for super-chunks)
        const bloom = buildBloomFilter(
          mergedText.toString("utf8"),
          BLOOM_FILTER_BITS,
        );

        // Build preview from merged text
        const firstLine = mergedText
          .toString("utf8")
          .split("\n")
          .map((l) => l.trim())
          .find(Boolean);
        const preview = firstLine
          ? firstLine.length > 140
            ? firstLine.slice(0, 137) + "..."
            : firstLine
          : null;

        // Record the source chunk IDs for traceability
        const sourceIds = batch.map((c) => c.id);

        session.chunks.push({
          id: l2Id,
          revision: session.revision,
          createdAt: batch[0].createdAt,
          rawBytes: totalRaw,
          textBytes: totalText,
          lineCount: totalLines,
          rawPath: path.relative(archiveRoot, rawPath),
          textPath: path.relative(archiveRoot, textPath),
          rawHash: crypto.createHash("sha256").update(mergedRaw).digest("hex"),
          bloom,
          preview,
          compactionLevel: 2,
          sourceChunkIds: sourceIds,
        });

        // Remove the merged L1 chunks from the session's chunk list
        // (but keep the files — they are the lossless originals)
        const mergedIds = new Set(sourceIds);
        session.chunks = session.chunks.filter((c) => !mergedIds.has(c.id));

        totalMerged += batch.length;
        totalCreated += 1;
      }
    }

    if (totalMerged > 0 || totalCreated > 0) {
      writeManifest(manifest);
    }

    return {
      action: "compacted",
      chunksMerged: totalMerged,
      superChunksCreated: totalCreated,
      message:
        totalCreated > 0
          ? `Merged ${totalMerged} L1 chunks into ${totalCreated} L2 super-chunk(s).`
          : "No sessions had enough chunks to compact.",
    };
  }

  // ─── Format helpers ─────────────────────────────────────────────────────

  function formatSearchResults(result) {
    if (result.message && !result.results?.length) return result.message;
    const lines = [
      `Found ${result.totalMatches} match(es) across ${result.sessionsSearched} session(s):`,
      "",
    ];
    for (const r of result.results) {
      const dateStr = r.date || (r.createdAt ? new Date(r.createdAt).toISOString().slice(0, 10) : "unknown");
      lines.push(
        `--- [${r.compressedLevel}${r.scope === "global" ? "/global" : ""}] ${r.title} (${r.sessionId.slice(0, 8)}...) ${dateStr} ---`,
      );
      lines.push(`  Chunk: ${r.chunkId}  Score: ${r.score}`);
      if (r.createdAt) {
        lines.push(`  Date: ${new Date(r.createdAt).toISOString()}`);
      }
      lines.push(`  ${r.snippet}`);
      lines.push("");
    }
    return lines.join("\n");
  }

  function formatDirectionResult(result) {
    if (!result.direction) {
      return result.message || "No project direction detected yet.";
    }
    const lines = [
      `Project Direction (${result.source}):`,
      "",
      `  "${result.direction}"`,
      "",
      `  Session: ${result.sessionTitle || result.sessionId}`,
      `  Detected: ${result.detectedAt ? new Date(result.detectedAt).toISOString() : "unknown"}`,
    ];
    if (result.recentFirstRequest) {
      lines.push("");
      lines.push(`Most recent session's first request:`);
      lines.push(`  "${result.recentFirstRequest}"`);
      lines.push(
        `  Session: ${result.recentSessionTitle || result.recentSessionId}`,
      );
    }
    return lines.join("\n");
  }

  function formatStatsResult(result) {
    const lines = [
      `Chat Archive Statistics`,
      `  Root: ${result.archiveRoot}`,
      `  Sessions: ${result.totalSessions}`,
      `  Total chunks: ${result.totalChunks} (L1: ${result.l1Chunks}, L2: ${result.l2Chunks})`,
      `  Raw size: ${result.totalRawMB} MB`,
      `  Compressed size: ${result.totalCompressedMB} MB`,
      `  Compression ratio: ${result.compressionRatio}`,
      "",
      "Sessions:",
    ];
    for (const s of result.sessions) {
      lines.push(
        `  ${s.title || "Copilot Chat"} (${s.sessionId.slice(0, 8)}...) — ${s.chunks} chunk(s)`,
      );
    }
    return lines.join("\n");
  }

  function formatCompactResult(result) {
    return result.message;
  }

  // ─── Dispatch handler ───────────────────────────────────────────────────

  return async function handleChatArchiveToolCall(toolName, toolArguments) {
    if (toolName === "search_chat_history") {
      const result = searchChatHistory(toolArguments);
      return [{ type: "text", text: formatSearchResults(result) }];
    }
    if (toolName === "get_project_direction") {
      const result = getProjectDirection(toolArguments);
      return [{ type: "text", text: formatDirectionResult(result) }];
    }
    if (toolName === "get_chat_archive_stats") {
      const result = getChatArchiveStats();
      return [{ type: "text", text: formatStatsResult(result) }];
    }
    if (toolName === "compact_chat_archive") {
      const result = compactChatArchive(toolArguments);
      return [{ type: "text", text: formatCompactResult(result) }];
    }
    return null;
  };
}

module.exports = { CHAT_ARCHIVE_TOOLS, createHandler };
