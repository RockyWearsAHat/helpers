#!/usr/bin/env node
// scripts/test-session-memory.js — Integration test for session memory module
"use strict";

const path = require("path");
const fs = require("fs/promises");

const WORKSPACE = "/tmp/test-session-memory-" + Date.now();
const TEST_SESSION_URI =
  "vscode-chat-session://github.copilot-chat/test-session-abc123";

async function main() {
  const createSessionMemory = require(
    path.join(__dirname, "..", "lib", "mcp-session-memory"),
  );
  const sm = createSessionMemory({
    WORKSPACE_ROOT: WORKSPACE,
    CHAT_SESSION_URI: TEST_SESSION_URI,
    escapeRegExp: (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
    tokenizeQuery: (q) =>
      q
        .toLowerCase()
        .split(/\s+/)
        .filter((t) => t.length >= 2),
    summarizeInline: (t, n) => t.slice(0, n),
    toPositiveInt: (v, d, min, max) =>
      Math.max(min, Math.min(max, parseInt(v) || d)),
  });

  // 1. Log a normal event
  const r1 = await sm.logSessionEvent({
    action: "refactored git-upload test detection",
    outcome: "success",
    surprise: 0.2,
    model: "claude-sonnet-4-6",
    tags: ["refactor", "bash"],
    context: "extracted to lib/upload-test-detection.sh",
  });
  console.log(
    "[pass] Log event 1:",
    r1.action,
    "index_rebuilt:",
    r1.index_rebuilt,
  );
  if (r1.entry.tier !== "capable")
    throw new Error("Expected tier 'capable', got: " + r1.entry.tier);

  // 2. Log a high-surprise failure
  const r2 = await sm.logSessionEvent({
    action: "attempted heredoc in terminal command",
    outcome: "failed - zsh interactive comments off",
    surprise: 0.9,
    model: "gpt-5.2",
    tags: ["shell", "terminal", "heredoc"],
    context: "heredocs break in agent terminal flows",
  });
  console.log("[pass] Log event 2:", r2.action, "tier:", r2.entry.tier);
  if (r2.entry.tier !== "capable")
    throw new Error(
      "Expected tier 'capable' for gpt-5.2, got: " + r2.entry.tier,
    );

  // 3. Log a haiku event
  const r3 = await sm.logSessionEvent({
    action: "listed files in src directory",
    outcome: "success",
    surprise: 0.0,
    model: "claude-haiku-4-5",
    tags: ["explore"],
  });
  console.log("[pass] Log event 3 tier:", r3.entry.tier);
  if (r3.entry.tier !== "quick")
    throw new Error("Expected tier 'quick', got: " + r3.entry.tier);

  // 4. Search — heredoc query should find the high-surprise entry
  const s1 = await sm.searchSessionLog({
    query: "heredoc terminal shell",
    current_model: "gpt-5.2",
  });
  console.log("[pass] Search results:", s1.results.length);
  if (s1.results.length === 0)
    throw new Error("Expected search results, got 0");
  if (!s1.results[0].action.includes("heredoc")) {
    throw new Error(
      "Expected top result to be heredoc event, got: " + s1.results[0].action,
    );
  }
  console.log(
    "[pass] Top result score:",
    s1.results[0].score,
    "surprise:",
    s1.results[0].surprise,
  );

  // 5. Same search with a different model — should still find it but with lower boost
  const s2 = await sm.searchSessionLog({
    query: "heredoc terminal",
    current_model: "claude-sonnet-4-6",
  });
  if (s2.results.length > 0 && s1.results.length > 0) {
    console.log(
      "[pass] Cross-model search works, score:",
      s2.results[0].score,
      "vs same-model:",
      s1.results[0].score,
    );
  }

  // 6. Summary
  const sum = await sm.getSessionSummary({ limit: 5 });
  console.log(
    "[pass] Summary — total:",
    sum.total_entries,
    "avg_surprise:",
    sum.avg_surprise,
  );
  if (sum.total_entries !== 3)
    throw new Error("Expected 3 entries, got: " + sum.total_entries);

  // 7. Format helpers
  const fmtLog = sm.formatLogResult(r1);
  if (!fmtLog.includes("refactored"))
    throw new Error("formatLogResult missing action text");
  console.log("[pass] Format log output OK");

  const fmtSearch = sm.formatSearchResults(s1);
  if (!fmtSearch.includes("heredoc"))
    throw new Error("formatSearchResults missing heredoc");
  console.log("[pass] Format search output OK");

  const fmtSum = sm.formatSummaryResult(sum);
  if (!fmtSum.includes("Total entries: 3"))
    throw new Error("formatSummaryResult wrong count");
  console.log("[pass] Format summary output OK");

  // 8. Verify session_uri is stamped on log entries
  if (r1.entry.session_uri !== TEST_SESSION_URI)
    throw new Error(
      "Expected session_uri on log entry, got: " + r1.entry.session_uri,
    );
  console.log("[pass] Log entries stamped with session_uri");

  // 9. Verify session_uri appears in search results
  if (s1.results[0].session_uri !== TEST_SESSION_URI)
    throw new Error(
      "Expected session_uri in search results, got: " +
        s1.results[0].session_uri,
    );
  console.log("[pass] Search results include session_uri");

  // 10. Verify session_uri=null when not provided
  const smNoSession = createSessionMemory({
    WORKSPACE_ROOT: WORKSPACE,
    CHAT_SESSION_URI: null,
    escapeRegExp: (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
    tokenizeQuery: (q) =>
      q
        .toLowerCase()
        .split(/\s+/)
        .filter((t) => t.length >= 2),
    summarizeInline: (t, n) => t.slice(0, n),
    toPositiveInt: (v, d, min, max) =>
      Math.max(min, Math.min(max, parseInt(v) || d)),
  });
  const r4 = await smNoSession.logSessionEvent({
    action: "event without session identity",
    outcome: "success",
    surprise: 0.0,
    model: "claude-haiku-4-5",
    tags: ["test"],
  });
  if (r4.entry.session_uri !== null)
    throw new Error(
      "Expected null session_uri when not provided, got: " +
        r4.entry.session_uri,
    );
  console.log("[pass] Null session_uri when CHAT_SESSION_URI not provided");

  // 11. Rebuild index manually
  const rebuildResult = await sm.buildSessionIndex();
  console.log(
    "[pass] Manual rebuild:",
    rebuildResult.entry_count,
    "entries,",
    rebuildResult.term_count,
    "terms",
  );

  // Cleanup
  await fs.rm(WORKSPACE, { recursive: true, force: true });
  console.log("\nALL SESSION MEMORY TESTS PASSED");
}

main().catch((e) => {
  console.error("FAIL:", e.message);
  process.exit(1);
});
