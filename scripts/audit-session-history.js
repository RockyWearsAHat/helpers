#!/usr/bin/env node
"use strict";
const fs = require("fs");
const path = require("path");

const base = path.join(
  process.env.HOME,
  "Library/Application Support/Code/User/workspaceStorage/34d497c35460054209ce9cf31a818fb5/chatSessions",
);

const files = fs.readdirSync(base).filter((f) => f.endsWith(".jsonl"));
const sessions = [];

for (const file of files) {
  const filePath = path.join(base, file);
  const sessionId = file.replace(".jsonl", "");
  try {
    const content = fs.readFileSync(filePath, "utf8");
    const lines = content.trim().split("\n");
    let creationDate = 0;
    let firstRequest = "";
    let title = "";

    for (const line of lines) {
      const obj = JSON.parse(line);
      if (obj.kind === 0) {
        creationDate = obj.v?.creationDate || 0;
        title = obj.v?.customTitle || "";
      }
      if (
        obj.kind === 1 &&
        Array.isArray(obj.k) &&
        obj.k.includes("customTitle")
      ) {
        title = obj.v || "";
      }
      if (obj.kind === 2 && !firstRequest) {
        const reqs = Array.isArray(obj.v) ? obj.v : obj.v?.requests || [];
        if (reqs.length > 0) {
          const first = reqs[0];
          let msg = first.message || first.text || "";
          if (typeof msg === "object") msg = msg.text || JSON.stringify(msg);
          firstRequest = String(msg).slice(0, 120);
        }
      }
    }

    if (creationDate > 0) {
      sessions.push({
        creationDate,
        sessionId,
        firstRequest,
        title,
        size: content.length,
      });
    }
  } catch (e) {
    // skip bad files
  }
}

sessions.sort((a, b) => a.creationDate - b.creationDate);

console.log(`Total sessions: ${sessions.length}`);
console.log("\nOLDEST 10 sessions:");
for (const s of sessions.slice(0, 10)) {
  const dt = new Date(s.creationDate).toISOString().slice(0, 10);
  const kb = (s.size / 1024).toFixed(0);
  console.log(`  ${dt} [${kb}KB] "${s.title || "(untitled)"}"`);
  if (s.firstRequest) console.log(`    First: ${s.firstRequest}`);
}

console.log("\nMOST RECENT 5 sessions:");
for (const s of sessions.slice(-5).reverse()) {
  const dt = new Date(s.creationDate).toISOString();
  const kb = (s.size / 1024).toFixed(0);
  console.log(`  ${dt} [${kb}KB] ${s.sessionId}`);
  if (s.title) console.log(`    Title: ${s.title}`);
  if (s.firstRequest) console.log(`    First: ${s.firstRequest}`);
}
