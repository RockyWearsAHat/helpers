#!/usr/bin/env node
"use strict";

const assert = require("assert");

const {
  LOCAL_SUBAGENT_TOOLS,
  createLocalSubagentHandler,
  _internal,
} = require("../lib/mcp-local-subagents");

async function main() {
  const tool = LOCAL_SUBAGENT_TOOLS.find(
    (entry) => entry.name === "build_workspace_tool",
  );
  assert.ok(tool, "build_workspace_tool should be registered");
  assert.ok(
    tool.description.includes("register_workspace_tool"),
    "tool description should mention register_workspace_tool",
  );

  // build_workspace_tool now returns instructions for Copilot to write the
  // script itself and call register_workspace_tool — no Ollama, no loop.
  const handler = createLocalSubagentHandler({});

  const result = await handler("build_workspace_tool", {
    tool_request: "A tool that returns the current git branch as plain text.",
    tool_name_hint: "get_git_branch",
  });
  assert.ok(Array.isArray(result) && result.length > 0, "should return content");
  assert.ok(result[0].text.includes("register_workspace_tool"), "should instruct Copilot to call register_workspace_tool");
  assert.ok(result[0].text.includes("get_git_branch"), "should include tool name hint");

  const errorResult = await handler("build_workspace_tool", {});
  assert.strictEqual(
    errorResult[0].text,
    "build_workspace_tool: 'tool_request' is required.",
  );

  console.log("ok");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});