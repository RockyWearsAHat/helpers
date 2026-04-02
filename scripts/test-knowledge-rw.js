#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs/promises");
const os = require("os");
const path = require("path");

const createKnowledgeRW = require(path.join(
  __dirname,
  "..",
  "lib",
  "mcp-knowledge-rw",
));

function createDeps({
  workspaceRoot,
  knowledgeRoot,
  repoKnowledgeRoot,
  localIndexPath,
  homeDir,
  execCalls,
  allowPublish,
}) {
  return {
    WORKSPACE_ROOT: workspaceRoot,
    KNOWLEDGE_ROOT: knowledgeRoot,
    REPO_KNOWLEDGE_ROOT: repoKnowledgeRoot,
    LOCAL_INDEX_PATH: localIndexPath,
    GITHUB_RAW_BASE: "https://example.test/raw",
    GITHUB_API_BASE: "https://example.test/api",
    GITHUB_CACHE_DIR: path.join(workspaceRoot, ".cache"),
    CACHE_META_PATH: path.join(workspaceRoot, ".cache", "meta.json"),
    INDEX_MAX_AGE_MS: 600000,
    DEFAULT_USER_AGENT: "test-agent",
    escapeRegExp: (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
    tokenizeQuery: (query) =>
      query
        .toLowerCase()
        .split(/\s+/)
        .filter((token) => token.length >= 2),
    getMarkdownTitle: (text, fallbackTitle) => {
      const match = text.match(/^#\s+(.+)$/m);
      return match ? match[1].trim() : fallbackTitle;
    },
    summarizeInline: (text, maxChars) => text.slice(0, maxChars),
    summarizeText: (text, maxChars) => text.slice(0, maxChars),
    sleep: async () => {},
    fetchJson: async () => {
      throw new Error("not used in test");
    },
    fetchText: async () => {
      throw new Error("not used in test");
    },
    toPositiveInt: (value, fallback, min, max) => {
      const parsed = Number.parseInt(value, 10);
      if (Number.isNaN(parsed)) {
        return fallback;
      }
      return Math.max(min, Math.min(max, parsed));
    },
    buildKnowledgeIndex: async () => {
      const fileCount = (await fs.readdir(knowledgeRoot)).filter((entry) =>
        entry.endsWith(".md"),
      ).length;
      await fs.writeFile(
        localIndexPath,
        JSON.stringify({ file_count: fileCount }, null, 2),
        "utf8",
      );
      return {
        action: "built",
        path: path.relative(workspaceRoot, localIndexPath).replace(/\\/g, "/"),
        file_count: fileCount,
        term_count: 7,
      };
    },
    buildKnowledgeSnippet: () => "",
    scoreKnowledgeMatch: () => 0,
    collectMarkdownFiles: async () => [],
    execFileImpl: (_command, args, _options, callback) => {
      execCalls.push(args);
      if (!allowPublish) {
        callback(new Error("publish should be blocked"), "", "blocked");
        return;
      }
      callback(null, "", "submitted");
    },
    homeDir,
  };
}

async function writeSettings(homeDir, data) {
  const settingsPath = path.join(
    homeDir,
    ".copilot",
    "devops-audit-community-settings.json",
  );
  await fs.mkdir(path.dirname(settingsPath), { recursive: true });
  await fs.writeFile(settingsPath, JSON.stringify(data, null, 2), "utf8");
}

async function main() {
  const tempRoot = await fs.mkdtemp(
    path.join(os.tmpdir(), "test-knowledge-rw-"),
  );
  const workspaceRoot = path.join(tempRoot, "workspace");
  const knowledgeRoot = path.join(workspaceRoot, ".github", "knowledge");
  const repoKnowledgeRoot = path.join(tempRoot, "repo", "knowledge");
  const localIndexPath = path.join(knowledgeRoot, "_index.json");
  const homeEnabled = path.join(tempRoot, "home-enabled");
  const homeBlocked = path.join(tempRoot, "home-blocked");

  await fs.mkdir(knowledgeRoot, { recursive: true });
  await fs.mkdir(repoKnowledgeRoot, { recursive: true });
  await writeSettings(homeEnabled, { shareKnowledge: true });
  await writeSettings(homeBlocked, { shareKnowledge: false });

  const execCallsEnabled = [];
  const knowledgeRW = createKnowledgeRW(
    createDeps({
      workspaceRoot,
      knowledgeRoot,
      repoKnowledgeRoot,
      localIndexPath,
      homeDir: homeEnabled,
      execCalls: execCallsEnabled,
      allowPublish: true,
    }),
  );

  const localOnly = await knowledgeRW.writeKnowledgeNote({
    path: "local.md",
    content: "# Local\n\nBody",
  });
  assert.equal(localOnly.index.status, "rebuilt");
  assert.equal(localOnly.publish.status, "local-only");
  assert.equal(execCallsEnabled.length, 0);
  console.log("[pass] local knowledge write stays local by default");

  const published = await knowledgeRW.writeKnowledgeNote({
    path: "published.md",
    content: "# Published\n\nBody",
    publish: true,
  });
  assert.equal(published.index.status, "rebuilt");
  assert.equal(published.publish.status, "submitted");
  assert.equal(execCallsEnabled.length, 1);
  assert.equal(execCallsEnabled[0][1], path.join(knowledgeRoot, "published.md"));
  console.log("[pass] publish=true submits after index rebuild");

  await fs.writeFile(
    path.join(knowledgeRoot, "update.md"),
    "# Update\n\n## Facts\nold\n",
    "utf8",
  );
  const updated = await knowledgeRW.updateKnowledgeNote({
    path: "update.md",
    heading: "Facts",
    content: "new",
    publish: false,
  });
  const updatedText = await fs.readFile(path.join(knowledgeRoot, "update.md"), "utf8");
  assert.equal(updated.index.status, "rebuilt");
  assert.equal(updated.publish.status, "local-only");
  assert.ok(updatedText.includes("new"));
  console.log("[pass] update rebuilds index and honors publish=false");

  const execCallsBlocked = [];
  const blockedRW = createKnowledgeRW(
    createDeps({
      workspaceRoot,
      knowledgeRoot,
      repoKnowledgeRoot,
      localIndexPath,
      homeDir: homeBlocked,
      execCalls: execCallsBlocked,
      allowPublish: false,
    }),
  );
  const blocked = await blockedRW.writeKnowledgeNote({
    path: "blocked.md",
    content: "# Blocked\n\nBody",
    publish: true,
  });
  assert.equal(blocked.publish.status, "blocked");
  assert.equal(execCallsBlocked.length, 0);
  console.log("[pass] publish=true is blocked when shareKnowledge is disabled");

  const formatted = knowledgeRW.formatKnowledgeWriteResult(published);
  assert.ok(formatted.includes("Index: rebuilt"));
  assert.ok(formatted.includes("Publish: submitted"));
  console.log("[pass] formatter includes index and publish status");

  await fs.rm(tempRoot, { recursive: true, force: true });
  console.log("ALL KNOWLEDGE RW TESTS PASSED");
}

main().catch((error) => {
  console.error("FAIL:", error.message);
  process.exit(1);
});