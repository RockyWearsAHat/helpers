#!/usr/bin/env node
"use strict";

// Per-session workspace via the MCP `roots` capability.
//
// Regression test for the bug where a resident daemon resolved the workspace
// from its own process cwd, so register_workspace_tool wrote project flows into
// the wrong repo. The fix: a roots-capable client's declared folder is
// authoritative. Here we launch the real stdio server in a DECOY cwd with no
// GSH_WORKSPACE_ROOTS, answer its roots/list with a DIFFERENT project dir, and
// assert the registration lands in the project — not the decoy cwd.
//
// Exits non-zero on failure; skips cleanly if gsh-native isn't built.

const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawn, spawnSync } = require("child_process");

const SERVER = path.join(__dirname, "..", "git-shell-helpers-mcp.js");
const NATIVE = path.join(__dirname, "..", "gsh-native");

if (!fs.existsSync(NATIVE)) {
  console.log("SKIP test-mcp-roots-workspace: gsh-native not built (run `gsh build`).");
  process.exit(0);
}

function makeRepo(prefix) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  spawnSync("git", ["init", "-q"], { cwd: dir });
  return fs.realpathSync(dir);
}

// Decoy = where the "daemon" is parked; project = the client's real folder.
const decoy = makeRepo("gsh-decoy-");
const project = makeRepo("gsh-proj-");

let failures = 0;
const fail = (m) => {
  console.error("FAIL: " + m);
  failures++;
};
const ok = (m) => console.log("  ok: " + m);

// Minimal JSON-RPC client that also answers the server's roots/list request.
class RootsClient {
  constructor() {
    this.child = spawn(process.execPath, [SERVER], {
      cwd: decoy,
      // Crucially: no GSH_WORKSPACE_ROOTS — the workspace must come from roots.
      env: { ...process.env, GSH_WORKSPACE_ROOTS: "" },
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.buf = "";
    this.id = 0;
    this.pending = new Map();
    this.rootsAsked = null; // resolves once we've answered roots/list
    this.rootsAskedPromise = new Promise((r) => (this.rootsAsked = r));
    this.child.stdout.on("data", (d) => this.onData(d));
  }
  onData(d) {
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
      // Server → client request: answer roots/list with our project folder.
      if (msg.method === "roots/list") {
        this.send({
          jsonrpc: "2.0",
          id: msg.id,
          result: { roots: [{ uri: "file://" + project, name: "project" }] },
        });
        this.rootsAsked();
        continue;
      }
      if (msg.id != null && this.pending.has(msg.id)) {
        this.pending.get(msg.id)(msg);
        this.pending.delete(msg.id);
      }
    }
  }
  send(obj) {
    this.child.stdin.write(JSON.stringify(obj) + "\n");
  }
  notify(method, params) {
    this.send({ jsonrpc: "2.0", method, params });
  }
  req(method, params, timeoutMs = 30000) {
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`timeout: ${method}`));
      }, timeoutMs);
      this.pending.set(id, (m) => {
        clearTimeout(timer);
        resolve(m);
      });
      this.send({ jsonrpc: "2.0", id, method, params });
    });
  }
  close() {
    try {
      this.child.kill();
    } catch {}
  }
}

async function main() {
  const client = new RootsClient();

  // Declare the roots capability so the server fetches our folder.
  await client.req("initialize", { capabilities: { roots: { listChanged: true } } });
  ok("initialize (roots capability advertised)");

  // initialized triggers the server's roots/list; wait until we've answered.
  client.notify("notifications/initialized", {});
  await client.rootsAskedPromise;
  ok("server requested roots/list and we answered with the project dir");

  // Register a flow. It must land in the project (from roots), not the decoy.
  const res = await client.req("tools/call", {
    name: "register_workspace_tool",
    arguments: { name: "from-roots", description: "d", command: "echo hi" },
  });
  if (res.error) fail(`register returned error: ${res.error.message}`);

  const projManifest = path.join(project, ".gsh", "tools", "manifest.json");
  const decoyManifest = path.join(decoy, ".gsh", "tools", "manifest.json");

  if (!fs.existsSync(projManifest)) {
    fail("project manifest was not written");
  } else {
    const tools = JSON.parse(fs.readFileSync(projManifest, "utf8")).tools || [];
    if (tools.some((t) => t.name === "from-roots")) {
      ok("flow registered into the roots-provided project dir");
    } else {
      fail("project manifest missing the registered flow");
    }
  }

  if (fs.existsSync(decoyManifest)) {
    fail(`flow leaked into the decoy cwd: ${decoyManifest}`);
  } else {
    ok("decoy cwd was not touched");
  }

  client.close();
  finish();
}

function finish() {
  for (const dir of [decoy, project]) {
    try {
      fs.rmSync(dir, { recursive: true, force: true });
    } catch {}
  }
  if (failures > 0) {
    console.error(`\ntest-mcp-roots-workspace: ${failures} failure(s)`);
    process.exit(1);
  }
  console.log("\ntest-mcp-roots-workspace: all checks passed");
  process.exit(0);
}

main().catch((e) => {
  fail(e.message);
  finish();
});
