#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");

const { CHECKPOINT_TOOL } = require("../lib/mcp-checkpoint");
const { STRICT_LINT_TOOL } = require("../lib/mcp-strict-lint");
const { BRANCH_SESSION_TOOLS } = require("../lib/mcp-branch-sessions");
const { LIST_LANGUAGE_MODELS_TOOL } = require("../lib/mcp-language-models");
const { RESEARCH_TOOLS } = require("../lib/mcp-research-tools");
const { NATIVE_TOOL_NAMES } = require("../lib/mcp-native");
const {
  LOCAL_SUBAGENT_TOOLS,
} = require("../lib/mcp-local-subagents");
const {
  REGISTER_WORKSPACE_TOOL,
  RELOAD_WINDOW_READY_TOOL,
  UNREGISTER_WORKSPACE_TOOL,
} = require("../lib/mcp-user-tools");
const { tools: VISION_TOOLS } = require("../vision-tool/mcp-server");

function collectToolNames() {
  const schemas = [
    CHECKPOINT_TOOL,
    STRICT_LINT_TOOL,
    LIST_LANGUAGE_MODELS_TOOL,
    REGISTER_WORKSPACE_TOOL,
    RELOAD_WINDOW_READY_TOOL,
    UNREGISTER_WORKSPACE_TOOL,
    ...BRANCH_SESSION_TOOLS,
    ...RESEARCH_TOOLS,
    ...LOCAL_SUBAGENT_TOOLS,
    ...VISION_TOOLS,
    // Native (Rust) tools — workspace_context + session memory. Names come from
    // the bridge allowlist so the README doc check stays authoritative.
    ...[...NATIVE_TOOL_NAMES].map((name) => ({ name })),
  ];

  return [...new Set(schemas.map((tool) => tool && tool.name).filter(Boolean))]
    .sort();
}

function main() {
  const readmePath = path.join(__dirname, "..", "README.md");
  const readme = fs.readFileSync(readmePath, "utf8");
  const toolNames = collectToolNames();

  const missing = toolNames.filter((name) => !readme.includes("`" + name + "`"));

  if (missing.length > 0) {
    console.error("MCP_DOCS_SYNC: FAIL");
    for (const name of missing) {
      console.error("MISSING_TOOL_IN_README: " + name);
    }
    process.exit(1);
  }

  console.log("MCP_DOCS_SYNC: pass " + toolNames.length + " tools documented");
  process.exit(0);
}

main();
