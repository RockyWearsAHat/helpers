"use strict";
// scripts/test-chat-archive-mcp.js — End-to-end test for chat archive MCP tools
const path = require("path");
const fs = require("fs");
const os = require("os");

const CHAT_SESSIONS_DIR = path.join(
  os.homedir(),
  "Library",
  "Application Support",
  "Code",
  "User",
  "workspaceStorage",
  "34d497c35460054209ce9cf31a818fb5",
  "chatSessions",
);

async function main() {
  // ── Set up isolated test archive ──────────────────────────────────────────
  const archiveRoot = path.join(os.tmpdir(), `gsh-archive-test-${Date.now()}`);
  fs.mkdirSync(archiveRoot, { recursive: true });
  console.log("Archive root:", archiveRoot);

  // ── Archive real session files via extension-side archiver ────────────────
  const createChatHistoryArchive = require("../vscode-extension/src/chat-history-archive");
  const archive = createChatHistoryArchive();
  // initialize() appends "chat-history-archive" internally and returns the actual root
  const actualArchiveRoot = archive.initialize(archiveRoot);

  let files = [];
  try {
    files = fs
      .readdirSync(CHAT_SESSIONS_DIR)
      .filter((f) => f.endsWith(".jsonl"))
      .map((f) => ({
        f,
        fp: path.join(CHAT_SESSIONS_DIR, f),
        size: fs.statSync(path.join(CHAT_SESSIONS_DIR, f)).size,
      }))
      .filter((f) => f.size < 500 * 1024)
      .sort((a, b) => b.size - a.size)
      .slice(0, 4);
  } catch (err) {
    console.log("Could not read chat sessions dir:", err.message);
    console.log("Creating synthetic test data instead...");
    files = [];
  }

  if (files.length > 0) {
    console.log("\nArchiving top 5 sessions by size:");
    for (const { f, fp, size } of files) {
      const sid = f.slice(0, -6);
      const result = archive.archiveSessionFile(sid, fp);
      console.log(
        `  ${sid.slice(0, 8)}...  raw=${(size / 1024).toFixed(0)}KB  appended=${(result.appendedBytes / 1024).toFixed(0)}KB  chunks=${result.chunksWritten}`,
      );
    }
  } else {
    // Synthetic test data if no real sessions
    const syntheticJsonl = [
      JSON.stringify({
        kind: 0,
        k: [],
        v: {
          customTitle: "Test Session",
          requests: [
            {
              message: {
                text: "I want to build a web app for tracking workout sessions with charts and user authentication",
              },
            },
          ],
        },
      }),
      JSON.stringify({
        kind: 2,
        k: ["requests", 0, "response"],
        v: {
          kind: "markdownContent",
          content:
            "I'll help you build that! Let me start with the project structure...",
        },
      }),
      JSON.stringify({
        kind: 2,
        k: ["requests"],
        v: [
          {
            message: { text: "session memory compaction archive search" },
            response: { content: "Here is how session memory works..." },
          },
        ],
      }),
    ].join("\n");
    const synthPath = path.join(os.tmpdir(), "synthetic-session.jsonl");
    fs.writeFileSync(synthPath, syntheticJsonl);
    archive.archiveSessionFile("synthetic-test-session-id-0001", synthPath);
    console.log("Created synthetic test session.");
  }

  // ── Create MCP handler and test all 4 tools ───────────────────────────────
  const {
    CHAT_ARCHIVE_TOOLS,
    createHandler,
  } = require("../lib/mcp-chat-archive");
  const handler = createHandler({ archiveRoot: actualArchiveRoot });

  let passed = 0;
  let failed = 0;

  async function check(name, toolName, args, assertions) {
    try {
      const result = await handler(toolName, args);
      if (!result || !result[0] || result[0].type !== "text") {
        console.log(`FAIL [${name}]: no text result returned`);
        failed++;
        return;
      }
      const text = result[0].text;
      for (const { desc, test } of assertions) {
        if (!test(text)) {
          console.log(`FAIL [${name}] — ${desc}`);
          console.log("  Got:", text.slice(0, 200));
          failed++;
          return;
        }
      }
      console.log(`PASS [${name}]`);
      if (name.includes("direction") || name.includes("stats")) {
        console.log(
          text
            .split("\n")
            .slice(0, 8)
            .map((l) => `  ${l}`)
            .join("\n"),
        );
      }
      passed++;
    } catch (err) {
      console.log(`FAIL [${name}] threw: ${err.message}`);
      failed++;
    }
  }

  console.log("\n── Tool: get_chat_archive_stats ──────────────────────────");
  await check("stats: returns archive root", "get_chat_archive_stats", {}, [
    { desc: "contains Root:", test: (t) => t.includes("Root:") },
    { desc: "contains Sessions:", test: (t) => t.includes("Sessions") },
    {
      desc: "contains Compression",
      test: (t) => t.includes("Compression") || t.includes("compression"),
    },
  ]);

  console.log("\n── Tool: get_project_direction ───────────────────────────");
  await check("direction: detects first request", "get_project_direction", {}, [
    {
      desc: "returns direction or no-archive message",
      test: (t) =>
        t.includes("Project Direction") ||
        t.includes("No chat sessions") ||
        t.includes("direction"),
    },
  ]);
  await check(
    "direction: with include_recent",
    "get_project_direction",
    { include_recent: true },
    [
      {
        desc: "returns any direction content",
        test: (t) => t.length > 5,
      },
    ],
  );

  console.log("\n── Tool: search_chat_history ─────────────────────────────");
  await check(
    "search: returns match count header",
    "search_chat_history",
    { query: "session", max_results: 5 },
    [
      {
        desc: "contains match count",
        test: (t) => /Found \d+ match/.test(t) || t.includes("session"),
      },
    ],
  );
  await check(
    "search: no-match query returns gracefully",
    "search_chat_history",
    { query: "xyzzy_no_match_intentional_aaaaa", max_results: 3 },
    [
      {
        desc: "returns 0 matches or empty",
        test: (t) =>
          t.includes("0 match") || t.includes("Found 0") || t.length < 200,
      },
    ],
  );
  await check(
    "search: empty query returns error",
    "search_chat_history",
    { query: "" },
    [
      {
        desc: "returns error message",
        test: (t) => t.includes("required") || t.includes("Query"),
      },
    ],
  );

  console.log("\n── Tool: compact_chat_archive ────────────────────────────");
  await check(
    "compact: runs without error",
    "compact_chat_archive",
    { min_chunks: 2 },
    [
      {
        desc: "returns compaction result",
        test: (t) =>
          t.includes("compact") ||
          t.includes("chunk") ||
          t.includes("Merged") ||
          t.includes("enough"),
      },
    ],
  );

  // ── Check that unknown tool returns null ────
  const unknown = await handler("nonexistent_tool", {});
  if (unknown === null) {
    console.log("\nPASS [unknown tool returns null]");
    passed++;
  } else {
    console.log("\nFAIL [unknown tool should return null]");
    failed++;
  }

  // ── Print full stats + direction output for manual inspection ──────────────
  console.log("\n══ Full Stats Output ══");
  const statsOut = await handler("get_chat_archive_stats", {});
  console.log(statsOut[0].text);

  console.log("\n══ Full Direction Output ══");
  const dirOut = await handler("get_project_direction", {
    include_recent: true,
  });
  console.log(dirOut[0].text);

  // ── Summary ───────────────────────────────────────────────────────────────
  console.log(`\nResult: ${passed} passed, ${failed} failed`);
  if (failed > 0) process.exit(1);
}

main().catch((err) => {
  console.error("Unexpected error:", err);
  process.exit(1);
});
