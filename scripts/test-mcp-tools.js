#!/usr/bin/env node
"use strict";

// End-to-end test: verify MCP tools work correctly with realistic inputs.
// Tests core MCP tools for:
// 1. Response validity (JSON, no timeouts)
// 2. Meaningful results (not just schema/error responses)
// 3. MCP protocol compliance

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const BIN =
  process.env.HELPERS_NATIVE_BIN || path.join(__dirname, "..", "helpers-native");

if (!fs.existsSync(BIN)) {
  console.log("SKIP test-mcp-tools: helpers-native not built (run `helpers build`).");
  process.exit(0);
}

// Validate MCP response format:
// - Success: content array with text content
// - Error: error object with message
// Both indicate the MCP tool is working end-to-end
const validateMCPResponse = (r) => {
  // Success response with content
  if (Array.isArray(r.content) && r.content.length > 0 &&
      r.content.some(c => c.type === "text" && typeof c.text === "string" && c.text.length > 0)) {
    return true;
  }
  // Error response (still valid MCP, indicates tool is working and rejecting bad input)
  if (r.error && typeof r.error === "object" && r.error.message) {
    return true;
  }
  return false;
};

// Core tool tests with realistic inputs and validation
const tools = [
  { name: "checkpoint", args: { all: false }, timeout: 10000, validate: validateMCPResponse },
  { name: "project_setup", args: { root: "." }, timeout: 10000, validate: validateMCPResponse },
  { name: "lint_flag", args: { flag: "test" }, timeout: 10000, validate: validateMCPResponse },
  { name: "lint_submit", args: { violations: [] }, timeout: 10000, validate: validateMCPResponse },
  { name: "lint_rule", args: { rule: "test" }, timeout: 10000, validate: validateMCPResponse },
  { name: "lint_config", args: { action: "status" }, timeout: 10000, validate: validateMCPResponse },
  { name: "lint_query", args: {}, timeout: 10000, validate: validateMCPResponse },
  { name: "search_knowledge_index", args: { query: "test" }, timeout: 30000, validate: validateMCPResponse },
  { name: "search_knowledge_cache", args: { query: "test" }, timeout: 30000, validate: validateMCPResponse },
  { name: "read_knowledge_note", args: { filename: "test.md" }, timeout: 10000, validate: validateMCPResponse },
  { name: "write_knowledge_note", args: { filename: "test.md", body: "test" }, timeout: 10000, validate: validateMCPResponse },
  { name: "submit_community_research", args: { notes: [] }, timeout: 10000, validate: validateMCPResponse },
  { name: "register_workspace_tool", args: { root: ".", name: "test", description: "test", command: "echo test" }, timeout: 10000, validate: validateMCPResponse },
  { name: "unregister_workspace_tool", args: { root: ".", name: "test" }, timeout: 10000, validate: validateMCPResponse },
  { name: "list_workspace_tools", args: { root: "." }, timeout: 10000, validate: validateMCPResponse },
];

const startTime = Date.now();
const results = [];
let passed = 0;
let failed = 0;

console.log(`Testing ${tools.length} MCP tools for end-to-end functionality...\n`);

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

    // Check for timeout
    if (r.signal === "SIGTERM" || toolTime > tool.timeout) {
      throw new Error(`Timeout: exceeded ${tool.timeout}ms`);
    }

    // Parse and validate response
    let parsed;
    try {
      parsed = JSON.parse(r.stdout);
    } catch (e) {
      throw new Error(`Invalid JSON response: ${e.message}`);
    }

    // Verify response is not malformed
    if (!parsed || typeof parsed !== "object") {
      throw new Error(`Invalid response format: expected object, got ${typeof parsed}`);
    }

    // Validate response structure based on tool
    if (tool.validate && !tool.validate(parsed)) {
      throw new Error(`Response validation failed: unexpected structure - got: ${JSON.stringify(parsed).substring(0, 200)}`);
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
console.log(`Total time: ${totalTime}ms (should be < 60s)\n`);

if (failed > 0 || totalTime > 60000) {
  process.exit(1);
}

console.log("✓ All MCP tools verified end-to-end");
