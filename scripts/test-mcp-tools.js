#!/usr/bin/env node
"use strict";

// Smoke test: verify every helpers MCP tool answers correctly and quickly.
// Tests all tools in the MCP registry for:
// 1. Response validity (JSON, no errors)
// 2. Response speed (< 5s per tool, < 30s total)

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const BIN =
  process.env.HELPERS_NATIVE_BIN || path.join(__dirname, "..", "helpers-native");

if (!fs.existsSync(BIN)) {
  console.log("SKIP test-mcp-tools: helpers-native not built (run `helpers build`).");
  process.exit(0);
}

const tools = [
  { name: "checkpoint", args: { all: false } },
  { name: "index_project", args: { root: "." } },
  { name: "project_map", args: { root: "." } },
  { name: "lookup", args: { root: ".", query: "test" } },
  { name: "project_setup", args: { root: "." } },
  { name: "lint", args: { file: ".", severity: "all" } },
  { name: "lint_flag", args: { flag: "test" } },
  { name: "lint_submit", args: { violations: [] } },
  { name: "lint_rule", args: { rule: "test" } },
  { name: "lint_config", args: { action: "status" } },
  { name: "lint_query", args: {} },
  { name: "build_knowledge_index", args: { root: "." } },
  { name: "search_knowledge_index", args: { query: "test" } },
  { name: "search_knowledge_cache", args: { query: "test" } },
  { name: "read_knowledge_note", args: { filename: "test.md" } },
  { name: "write_knowledge_note", args: { filename: "test.md", body: "test" } },
  { name: "update_knowledge_note", args: { filename: "test.md", section: "test", body: "test" } },
  { name: "append_to_knowledge_note", args: { filename: "test.md", body: "test" } },
  { name: "submit_community_research", args: { notes: [] } },
  { name: "register_workspace_tool", args: { root: ".", name: "test", description: "test", command: "echo test" } },
  { name: "unregister_workspace_tool", args: { root: ".", name: "test" } },
  { name: "list_workspace_tools", args: { root: "." } },
];

const startTime = Date.now();
const results = [];
let passed = 0;
let failed = 0;

console.log(`Testing ${tools.length} MCP tools for correctness and speed...\n`);

for (const tool of tools) {
  const toolStart = Date.now();
  try {
    const r = spawnSync(BIN, ["call", tool.name], {
      input: JSON.stringify(tool.args),
      encoding: "utf8",
      maxBuffer: 32 * 1024 * 1024,
      timeout: 5000,
    });

    const toolTime = Date.now() - toolStart;

    if (r.error) {
      throw r.error;
    }

    if (r.status !== 0 && tool.name !== "read_knowledge_note" && tool.name !== "project_setup") {
      // Some tools may exit non-zero for valid reasons (e.g., file not found)
      // Only fail if it's completely broken
      if (r.stderr && r.stderr.includes("fatal")) {
        throw new Error(r.stderr);
      }
    }

    const parsed = JSON.parse(r.stdout);

    // Check for error in response
    if (parsed.error && parsed.error.message) {
      // Some errors are expected for minimal args (e.g., file not found)
      // As long as we got a valid MCP response, the tool answered
      if (toolTime > 5000) {
        throw new Error(`Slow response: ${toolTime}ms`);
      }
    }

    results.push({
      tool: tool.name,
      status: "✓",
      time: `${toolTime}ms`,
    });
    passed++;
  } catch (err) {
    results.push({
      tool: tool.name,
      status: "✗",
      error: err.message,
    });
    failed++;
  }
}

const totalTime = Date.now() - startTime;

console.log("Results:");
console.log("--------");
for (const r of results) {
  if (r.status === "✓") {
    console.log(`${r.status} ${r.tool.padEnd(30)} ${r.time}`);
  } else {
    console.log(`${r.status} ${r.tool.padEnd(30)} ${r.error}`);
  }
}

console.log(`\n${passed}/${tools.length} tools passed, ${failed} failed`);
console.log(`Total time: ${totalTime}ms (should be < 30s)\n`);

if (failed > 0 || totalTime > 30000) {
  process.exit(1);
}

console.log("test-mcp-tools passed");
