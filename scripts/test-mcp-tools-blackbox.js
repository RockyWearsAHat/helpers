#!/usr/bin/env node
"use strict";

// Black-box MCP test: launch the real stdio server, speak JSON-RPC, and exercise
// the tool surface the way an agent would.
//
//   - Every advertised tool must have a valid schema (name + object inputSchema).
//   - Standalone, side-effect-free tools are actually invoked in a throwaway git
//     repo and their output asserted.
//   - Tools that need the network, a browser, the VS Code extension, or a local
//     model are schema-checked only (invoking them here would be slow/unsafe);
//     their logic is covered by their own unit tests.
//
// Exits non-zero on the first failure. Skips cleanly if the server can't load
// its required native binary (so a machine without `gsh build` stays green).

const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawn, spawnSync } = require("child_process");

const SERVER = path.join(__dirname, "..", "git-shell-helpers-mcp.js");
const NATIVE = path.join(__dirname, "..", "gsh-native");

if (!fs.existsSync(NATIVE)) {
  console.log("SKIP test-mcp-tools-blackbox: gsh-native not built (run `gsh build`).");
  process.exit(0);
}

// ── A throwaway git repo so root-scoped tools have real content ──────────────
const repo = fs.mkdtempSync(path.join(os.tmpdir(), "gsh-bb-"));
spawnSync("git", ["init", "-q"], { cwd: repo });
spawnSync("git", ["config", "user.email", "t@t.t"], { cwd: repo });
spawnSync("git", ["config", "user.name", "t"], { cwd: repo });
fs.mkdirSync(path.join(repo, "src"));
fs.writeFileSync(path.join(repo, "src", "core.rs"), "pub fn widget() {}\n");
fs.writeFileSync(path.join(repo, "src", "user.rs"), "fn run() { widget(); }\n");
fs.writeFileSync(path.join(repo, "tool.sh"), "#!/bin/bash\necho hi\n");
fs.mkdirSync(path.join(repo, "knowledge"));
fs.writeFileSync(
  path.join(repo, "knowledge", "note-alpha.md"),
  "# Widget Notes\nThe widget subsystem renders gadgets and caches sprockets.\n",
);

// ── Minimal JSON-RPC stdio client ───────────────────────────────────────────
class Client {
  constructor(env) {
    this.child = spawn(process.execPath, [SERVER], {
      env,
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.buf = "";
    this.id = 0;
    this.pending = new Map();
    this.child.stdout.on("data", (d) => {
      this.buf += d;
      let nl;
      while ((nl = this.buf.indexOf("\n")) >= 0) {
        const line = this.buf.slice(0, nl);
        this.buf = this.buf.slice(nl + 1);
        if (!line.trim()) continue;
        let msg;
        try {
          msg = JSON.parse(line);
        } catch {
          continue;
        }
        if (msg.id != null && this.pending.has(msg.id)) {
          this.pending.get(msg.id)(msg);
          this.pending.delete(msg.id);
        }
      }
    });
  }
  req(method, params, timeoutMs = 30000) {
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`timeout: ${method} ${(params && params.name) || ""}`));
      }, timeoutMs);
      this.pending.set(id, (m) => {
        clearTimeout(timer);
        resolve(m);
      });
      this.child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
    });
  }
  close() {
    try {
      this.child.kill();
    } catch {}
  }
}

// ── Expected functional behavior for the safe-to-invoke tools ────────────────
// value: regex the result text must match. The repo path is passed as `root`.
const SAFE = {
  index_project: { args: { root: repo }, expect: /files,/ },
  project_map: { args: { root: repo }, expect: /Project Map/ },
  lookup: { args: { root: repo, query: "widget" }, expect: /widget/ },
  cs_lint: { args: { root: repo }, expect: /principle review|No CS2420/ },
  checkpoint: { args: { cwd: repo }, expect: /Committed|Nothing to commit|no-op/ },
  strict_lint: { args: { folderPath: repo }, expect: /strict_lint \(standalone\)|providers run|✓ Clean/ },
  // Knowledge tools — local-only paths (no network) exercised end-to-end.
  build_knowledge_index: { args: {}, expect: /Files indexed/ },
  search_knowledge_cache: { args: { query: "widget gadget" }, expect: /Query:|Total results/ },
  write_knowledge_note: { args: { path: "bb-note.md", content: "# BB\nwidget content here" }, expect: /Action: created/ },
  read_knowledge_note: { args: { path: "note-alpha.md" }, expect: /Title: Widget Notes/ },
};

let failures = 0;
const fail = (msg) => {
  console.error("FAIL: " + msg);
  failures++;
};
const ok = (msg) => console.log("  ok: " + msg);

async function main() {
  const client = new Client({
    ...process.env,
    GSH_WORKSPACE_ROOTS: JSON.stringify([repo]),
    GSH_DISABLE_LOCAL_SUBAGENTS: "1", // don't spin up local model handlers
  });

  const init = await client.req("initialize", {});
  if (!init.result || !init.result.serverInfo) fail("initialize did not return serverInfo");
  else ok("initialize");

  const listed = await client.req("tools/list", {});
  const tools = (listed.result && listed.result.tools) || [];
  if (tools.length === 0) {
    fail("tools/list returned no tools");
    client.close();
    return finish();
  }
  ok(`tools/list returned ${tools.length} tools`);

  // White-box-ish: every advertised tool has a valid schema.
  for (const t of tools) {
    if (!t.name || typeof t.name !== "string") fail(`tool with no name: ${JSON.stringify(t)}`);
    else if (!t.inputSchema || t.inputSchema.type !== "object")
      fail(`tool ${t.name} has no object inputSchema`);
  }
  ok("all tools have a name + object inputSchema");

  // Black-box: invoke the safe tools and assert their output.
  const names = new Set(tools.map((t) => t.name));
  for (const [name, spec] of Object.entries(SAFE)) {
    if (!names.has(name)) {
      fail(`expected tool not advertised: ${name}`);
      continue;
    }
    try {
      const res = await client.req("tools/call", { name, arguments: spec.args });
      if (res.error) {
        fail(`${name} returned error: ${res.error.message}`);
        continue;
      }
      const text = (res.result.content || []).map((c) => c.text).join("\n");
      if (!spec.expect.test(text)) {
        fail(`${name} output did not match ${spec.expect}: ${text.slice(0, 120)}`);
      } else {
        ok(`${name} invoked, output matched`);
      }
    } catch (e) {
      fail(`${name}: ${e.message}`);
    }
  }

  // Black-box: register a project flow, then call it (must be live immediately).
  const reg = await client.req("tools/call", {
    name: "register_workspace_tool",
    arguments: { name: "bb-flow", description: "echo a marker", command: "echo flow-works-42" },
  });
  if (reg.error) fail("register_workspace_tool: " + reg.error.message);
  else ok("register_workspace_tool");
  const listed2 = await client.req("tools/list", {});
  if (!(listed2.result.tools || []).some((t) => t.name === "bb-flow"))
    fail("registered flow not live in tools/list");
  else ok("registered flow appears live in tools/list");
  const flow = await client.req("tools/call", { name: "bb-flow", arguments: {} });
  if (flow.error) fail("flow call: " + flow.error.message);
  else if (!/flow-works-42/.test((flow.result.content || []).map((c) => c.text).join("")))
    fail("flow output did not match");
  else ok("registered flow callable end-to-end");

  // Black-box: unknown tool and disabled-tool semantics.
  const unknown = await client.req("tools/call", { name: "definitely_not_a_tool", arguments: {} });
  if (!unknown.error) fail("unknown tool should return a JSON-RPC error");
  else ok("unknown tool returns a structured error");

  client.close();
  finish();
}

function finish() {
  fs.rmSync(repo, { recursive: true, force: true });
  if (failures > 0) {
    console.error(`\nBLACKBOX: FAIL (${failures} failure(s))`);
    process.exit(1);
  }
  console.log("\nBLACKBOX: pass");
  process.exit(0);
}

main().catch((e) => {
  fail("harness error: " + e.message);
  finish();
});
