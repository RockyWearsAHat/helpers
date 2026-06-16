#!/usr/bin/env node
"use strict";

// Verify every advertised MCP tool is documented in the README. Tool names come
// from the Node web-research tools plus the native binary's advertised schemas
// (built-in Rust tools + project-local flows). If the native binary isn't built
// yet, only the Node tools are checked.

const fs = require("fs");
const path = require("path");

const { RESEARCH_TOOLS } = require("../lib/mcp-research-tools");
const { getNativeToolNames } = require("../lib/mcp-native");

function collectToolNames() {
  const names = new Set(RESEARCH_TOOLS.map((t) => t.name));
  try {
    for (const n of getNativeToolNames()) names.add(n);
  } catch {
    // Native binary not built — skip native names (covered by the black-box test).
  }
  return [...names].sort();
}

function main() {
  const readme = fs.readFileSync(path.join(__dirname, "..", "README.md"), "utf8");
  const toolNames = collectToolNames();
  const missing = toolNames.filter((name) => !readme.includes("`" + name + "`"));

  if (missing.length > 0) {
    console.error("MCP_DOCS_SYNC: FAIL");
    for (const name of missing) console.error("MISSING_TOOL_IN_README: " + name);
    process.exit(1);
  }

  console.log("MCP_DOCS_SYNC: pass " + toolNames.length + " tools documented");
  process.exit(0);
}

main();
