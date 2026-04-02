#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

const createResearch = require("../lib/mcp-research");

async function main() {
  const originalCwd = process.cwd();
  const originalWorkspaceRoots = process.env.GSH_WORKSPACE_ROOTS;
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "gsh-research-"));
  const nestedDir = path.join(tempRoot, "nested", "child");
  const uniqueToken = "mcpresearchroottesttoken";

  try {
    fs.mkdirSync(path.join(tempRoot, ".git"), { recursive: true });
    fs.mkdirSync(path.join(tempRoot, "knowledge"), { recursive: true });
    fs.mkdirSync(nestedDir, { recursive: true });
    fs.writeFileSync(
      path.join(tempRoot, "knowledge", "workspace-root-test.md"),
      `# Workspace Root Test\n\n${uniqueToken} beta gamma\n`,
      "utf8",
    );

    delete process.env.GSH_WORKSPACE_ROOTS;
    process.chdir(nestedDir);

    const research = createResearch();
    const result = await research.searchKnowledgeCache({
      query: uniqueToken,
      max_results: 5,
    });

    assert.ok(
      Array.isArray(result.results) && result.results.length > 0,
      "Research factory can search workspace knowledge from a nested cwd",
    );
    assert.ok(
      result.results.some((entry) => entry.path === "knowledge/workspace-root-test.md"),
      "Workspace root detection resolves the nested cwd to the surrounding repo root",
    );
  } finally {
    if (typeof originalWorkspaceRoots === "string") {
      process.env.GSH_WORKSPACE_ROOTS = originalWorkspaceRoots;
    } else {
      delete process.env.GSH_WORKSPACE_ROOTS;
    }
    process.chdir(originalCwd);
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }

  console.log("ok");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});