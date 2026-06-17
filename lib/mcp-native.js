// lib/mcp-native.js — bridge from the Node MCP daemon to the native Rust tools.
//
// The Node process stays the MCP host; the tool *implementations* live in the
// `gsh-native` Rust binary. This module fetches the binary's advertised schemas
// (built-in tools PLUS any project-local flows registered in .gsh/tools/) and
// dispatches calls to it. There is intentionally NO Node fallback: if the binary
// is missing the tools are required, so we throw a clear, actionable error
// (`gsh doctor` / `gsh build` catch it earlier).
//
// Schemas are dynamic: a project flow registered via register_workspace_tool
// appears in tools/list on the next request, so the set of native tool names is
// derived from a short-lived cache of `gsh-native schemas` rather than a fixed
// list.
"use strict";

const path = require("path");
const fs = require("fs");
const { spawnSync } = require("child_process");

// Installed binary lives at the repo root (built/copied by `gsh build`).
const BIN = process.env.GSH_NATIVE_BIN || path.join(__dirname, "..", "gsh-native");

// Resolve the active workspace root exactly as the MCP host does
// (getWorkspaceRoot in git-shell-helpers-mcp.js): the first $GSH_WORKSPACE_ROOTS
// entry, else the process cwd. We spawn the native binary pinned to this root —
// both via `cwd` and an explicit, normalized $GSH_WORKSPACE_ROOTS — so the
// binary never independently re-guesses the workspace (e.g. falling back to its
// own install dir) and never disagrees with the host about which project a
// write like register_workspace_tool targets.
function resolveWorkspaceRoot() {
  const raw = process.env.GSH_WORKSPACE_ROOTS || "";
  if (raw.trim().startsWith("[")) {
    try {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed) && parsed.length > 0 && parsed[0]) {
        return String(parsed[0]);
      }
    } catch {
      /* fall through to cwd */
    }
  }
  const first = raw.split(",").filter(Boolean)[0];
  return first || process.cwd();
}

// Spawn options that pin the native binary to the resolved workspace.
function nativeSpawnOptions(extra) {
  const root = resolveWorkspaceRoot();
  return {
    cwd: root,
    env: { ...process.env, GSH_WORKSPACE_ROOTS: JSON.stringify([root]) },
    ...extra,
  };
}

// Tools that can register/unregister project flows — calling one invalidates the
// schema cache so the new flow is visible immediately.
const MUTATING_TOOLS = new Set([
  "register_workspace_tool",
  "unregister_workspace_tool",
]);

const CACHE_TTL_MS = 1000;
let _cache = { at: 0, schemas: [], names: new Set() };

function missingBinaryError() {
  return new Error(
    `Native GSH tools are unavailable: gsh-native binary not found at ${BIN}. ` +
      "These tools are required and have no Node fallback — install Rust " +
      "(https://rustup.rs), then run `gsh build`.",
  );
}

function refresh() {
  if (!fs.existsSync(BIN)) throw missingBinaryError();
  const r = spawnSync(
    BIN,
    ["schemas"],
    nativeSpawnOptions({ encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }),
  );
  if (r.status !== 0) {
    throw new Error(
      `gsh-native schemas failed (exit ${r.status}): ${(r.stderr || "").trim() || "no output"}`,
    );
  }
  let parsed;
  try {
    parsed = JSON.parse(r.stdout);
  } catch (e) {
    throw new Error(`gsh-native schemas returned invalid JSON: ${e.message}`);
  }
  if (!Array.isArray(parsed)) {
    throw new Error("gsh-native schemas did not return a JSON array.");
  }
  _cache = {
    at: Date.now(),
    schemas: parsed,
    names: new Set(parsed.map((s) => s && s.name).filter(Boolean)),
  };
  return _cache;
}

function state() {
  if (Date.now() - _cache.at < CACHE_TTL_MS) return _cache;
  return refresh();
}

// Current native tool schemas (built-ins + project flows). Throws (fail-loud)
// if the required binary is missing or broken.
function loadNativeSchemas() {
  return state().schemas;
}

// The set of tool names the native binary currently owns.
function getNativeToolNames() {
  return state().names;
}

function isNativeTool(name) {
  return state().names.has(name);
}

// Run a native tool: spawn `gsh-native call <name>` with the JSON args on stdin.
// Returns the MCP content array. The binary prints a JSON {error} envelope even
// on non-zero exit, which we surface as a thrown Error.
function runNativeTool(name, args) {
  if (!fs.existsSync(BIN)) throw missingBinaryError();
  const r = spawnSync(
    BIN,
    ["call", name],
    nativeSpawnOptions({
      input: JSON.stringify(args || {}),
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
  let parsed;
  try {
    parsed = JSON.parse(r.stdout);
  } catch {
    const detail = (r.stderr || r.stdout || "no output").trim();
    throw new Error(`gsh-native ${name} failed (exit ${r.status}): ${detail}`);
  }
  // Registering/unregistering a flow changes the tool surface — refresh now.
  if (MUTATING_TOOLS.has(name)) _cache.at = 0;
  if (parsed && parsed.error) {
    throw new Error(parsed.error.message || `native tool ${name} failed`);
  }
  return (parsed && parsed.content) || [];
}

module.exports = {
  BIN,
  isNativeTool,
  getNativeToolNames,
  loadNativeSchemas,
  runNativeTool,
};
