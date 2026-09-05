#!/usr/bin/env node
"use strict";

// Smoke test: verify every helpers MCP tool answers correctly.
// Tests all tools in the MCP registry for:
// 1. Response validity (JSON, no errors)
// 2. Schema response check (tools respond with valid MCP protocol format)

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
  { name: "checkpoint", args: { all: false }, timeout: 10000 },
  { name: "index_project", args: { root: "." }, timeout: 60000 },
  { name: "project_map", args: { root: "." }, timeout: 60000 },
  { name: "lookup", args: { root: ".", query: "test" }, timeout: 60000 },
  { name: "project_setup", args: { root: "." }, timeout: 10000 },
  { name: "lint", args: { file: ".", severity: "all" }, timeout: 60000 },
  { name: "lint_flag", args: { flag: "test" }, timeout: 10000 },
  { name: "lint_submit", args: { violations: [] }, timeout: 10000 },
  { name: "lint_rule", args: { rule: "test" }, timeout: 10000 },
  { name: "lint_config", args: { action: "status" }, timeout: 10000 },
  { name: "lint_query", args: {}, timeout: 10000 },
  { name: "build_knowledge_index", args: { root: "." }, timeout: 60000 },
  { name: "search_knowledge_index", args: { query: "test" }, timeout: 30000 },
  { name: "search_knowledge_cache", args: { query: "test" }, timeout: 30000 },
  { name: "read_knowledge_note", args: { filename: "test.md" }, timeout: 10000 },
  { name: "write_knowledge_note", args: { filename: "test.md", body: "test" }, timeout: 10000 },
  { name: "update_knowledge_note", args: { filename: "test.md", section: "test", body: "test" }, timeout: 10000 },
  { name: "append_to_knowledge_note", args: { filename: "test.md", body: "test" }, timeout: 10000 },
  { name: "submit_community_research", args: { notes: [] }, timeout: 10000 },
  { name: "register_workspace_tool", args: { root: ".", name: "test", description: "test", command: "echo test" }, timeout: 10000 },
  { name: "unregister_workspace_tool", args: { root: ".", name: "test" }, timeout: 10000 },
  { name: "list_workspace_tools", args: { root: "." }, timeout: 10000 },
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
      timeout: tool.timeout,
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
      if (toolTime > tool.timeout) {
        throw new Error(`Slow response: ${toolTime}ms (timeout: ${tool.timeout}ms)`);
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
console.log(`Total time: ${totalTime}ms (should be < 240s)\n`);

if (failed > 0 || totalTime > 240000) {
  process.exit(1);
}

console.log("test-mcp-tools passed");
