"use strict";
// src/tools-config.js — MCP tool group configuration
const fs = require("fs");
const path = require("path");

const MCP_TOOLS_CONFIG_DIR = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".config",
  "git-shell-helpers-mcp",
);
const MCP_TOOLS_CONFIG_PATH = path.join(MCP_TOOLS_CONFIG_DIR, "tools.json");

const TOOL_GROUPS = [
  {
    key: "knowledgeWrite",
    label: "Write Reusable Knowledge Locally",
    description: "Write, update & append knowledge notes",
    tools: [
      "write_knowledge_note",
      "update_knowledge_note",
      "append_to_knowledge_note",
    ],
  },
  {
    key: "communityResearch",
    label: "Share Knowledge Research",
    description: "Submit knowledge notes to community repo via PR",
    tools: ["submit_community_research"],
  },
  {
    key: "webSearch",
    label: "Web Search",
    description: "Search the web via SearXNG",
    tools: ["search_web"],
  },
  {
    key: "scrapeWebpage",
    label: "Scrape Webpage",
    description: "Fetch pages, strip HTML chrome, return clean text",
    tools: ["scrape_webpage"],
  },
  {
    key: "vision",
    label: "Vision",
    description:
      "Process images in-chat, allowing live analysis of visual output",
    tools: ["analyze_images"],
  },
  {
    key: "screenshot",
    label: "Screenshot",
    description:
      "Capture screenshots of the screen, an app window, or a region",
    tools: ["take_screenshot"],
  },
  {
    key: "checkpoint",
    label: "Git Checkpoint",
    description: "Commit working state via MCP tool — no terminal, no stalling",
    tools: ["checkpoint"],
  },
];

function readToolsConfig() {
  try {
    return JSON.parse(fs.readFileSync(MCP_TOOLS_CONFIG_PATH, "utf8"));
  } catch {
    return { disabledTools: [] };
  }
}

function writeToolsConfig(config) {
  fs.mkdirSync(MCP_TOOLS_CONFIG_DIR, { recursive: true });
  fs.writeFileSync(
    MCP_TOOLS_CONFIG_PATH,
    JSON.stringify(config, null, 2) + "\n",
    "utf8",
  );
}

function isGroupEnabled(groupKey) {
  const group = TOOL_GROUPS.find((g) => g.key === groupKey);
  if (!group || group.alwaysOn) return true;
  const config = readToolsConfig();
  const disabled = config.disabledTools || [];
  return !group.tools.some((t) => disabled.includes(t));
}

function setGroupEnabled(groupKey, enabled) {
  const group = TOOL_GROUPS.find((g) => g.key === groupKey);
  if (!group || group.alwaysOn) return;
  const config = readToolsConfig();
  const disabled = new Set(config.disabledTools || []);
  for (const tool of group.tools) {
    if (enabled) disabled.delete(tool);
    else disabled.add(tool);
  }
  config.disabledTools = [...disabled];
  writeToolsConfig(config);
}

module.exports = {
  MCP_TOOLS_CONFIG_PATH,
  TOOL_GROUPS,
  readToolsConfig,
  writeToolsConfig,
  isGroupEnabled,
  setGroupEnabled,
};
