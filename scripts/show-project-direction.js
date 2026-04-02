"use strict";
// scripts/show-project-direction.js — look up project direction from the live archive
const { createHandler } = require("../lib/mcp-chat-archive");
const os = require("os");
const path = require("path");
const fs = require("fs");

const wsBase = path.join(
  os.homedir(),
  "Library",
  "Application Support",
  "Code",
  "User",
  "workspaceStorage",
);

let archiveRoot = null;
try {
  for (const d of fs.readdirSync(wsBase)) {
    const candidate = path.join(wsBase, d, "chat-history-archive");
    if (fs.existsSync(path.join(candidate, "index.json"))) {
      archiveRoot = candidate;
      break;
    }
  }
} catch {}

if (!archiveRoot) {
  console.log(
    "No live archive found — extension needs to run at least once to populate it.",
  );
  process.exit(0);
}

console.log("Archive:", archiveRoot, "\n");
const handler = createHandler({ archiveRoot });
handler("get_project_direction", { include_recent: true })
  .then((r) => console.log(r[0].text))
  .catch((err) => console.error(err.message));
