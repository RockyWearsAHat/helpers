// lib/mcp-native.js — bridge from the Node MCP daemon to the native Rust tools.
//
// The Node process stays the MCP host; the tool *implementations* live in the
// `helpers-native` Rust binary. This module fetches the binary's advertised schemas
// (built-in tools PLUS any project-local flows registered in .helpers/tools/) and
// dispatches calls to it. There is intentionally NO Node fallback: if the binary
// is missing the tools are required, so we throw a clear, actionable error
// (`helpers doctor` / `helpers build` catch it earlier).
//
// Schemas are dynamic: a project flow registered via register_workspace_tool
// appears in tools/list on the next request, so the set of native tool names is
// derived from a short-lived cache of `helpers-native schemas` rather than a fixed
// list.
"use strict";

const path = require("path");
const fs = require("fs");
const { spawnSync } = require("child_process");

// Installed binary lives at the repo root (built/copied by `helpers build`).
const BIN = process.env.HELPERS_NATIVE_BIN || path.join(__dirname, "..", "helpers-native");

// Resolve the active workspace root. The caller (the MCP host) may pass the
// workspace it learned for *this session* — e.g. via the MCP `roots` capability,
// which is authoritative because it reflects the client's actual project rather
// than the daemon's process cwd. Falling back, we mirror the host's own
// getWorkspaceRoot: the first $HELPERS_WORKSPACE_ROOTS entry, else the process cwd.
// Whatever we resolve, we spawn the native binary pinned to it — both via `cwd`
// and an explicit, normalized $HELPERS_WORKSPACE_ROOTS — so the binary never
// independently re-guesses the workspace (e.g. falling back to its own install
// dir) and never disagrees with the host about which project a write targets.
function resolveWorkspaceRoot(explicit) {
  if (explicit && String(explicit).trim()) return String(explicit);
  const raw = process.env.HELPERS_WORKSPACE_ROOTS || "";
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
function nativeSpawnOptions(extra, workspaceRoot) {
  const root = resolveWorkspaceRoot(workspaceRoot);
  return {
    cwd: root,
    env: { ...process.env, HELPERS_WORKSPACE_ROOTS: JSON.stringify([root]) },
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
// Schema cache keyed by workspace root. Project flows differ per project, so a
// single global cache would surface one project's flows in another's session.
const _cache = new Map(); // root -> { at, schemas, names }

function missingBinaryError() {
  return new Error(
    `Native Helpers tools are unavailable: helpers-native binary not found at ${BIN}. ` +
      "These tools are required and have no Node fallback — install Rust " +
      "(https://rustup.rs), then run `helpers build`.",
  );
}

function refresh(workspaceRoot) {
  if (!fs.existsSync(BIN)) throw missingBinaryError();
  const r = spawnSync(
    BIN,
    ["schemas"],
    nativeSpawnOptions({ encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }, workspaceRoot),
  );
  if (r.status !== 0) {
    throw new Error(
      `helpers-native schemas failed (exit ${r.status}): ${(r.stderr || "").trim() || "no output"}`,
    );
  }
  let parsed;
  try {
    parsed = JSON.parse(r.stdout);
  } catch (e) {
    throw new Error(`helpers-native schemas returned invalid JSON: ${e.message}`);
  }
  if (!Array.isArray(parsed)) {
    throw new Error("helpers-native schemas did not return a JSON array.");
  }
  const entry = {
    at: Date.now(),
    schemas: parsed,
    names: new Set(parsed.map((s) => s && s.name).filter(Boolean)),
  };
  _cache.set(resolveWorkspaceRoot(workspaceRoot), entry);
  return entry;
}

function state(workspaceRoot) {
  const hit = _cache.get(resolveWorkspaceRoot(workspaceRoot));
  if (hit && Date.now() - hit.at < CACHE_TTL_MS) return hit;
  return refresh(workspaceRoot);
}

// Current native tool schemas (built-ins + project flows) for the given
// workspace. Throws (fail-loud) if the required binary is missing or broken.
function loadNativeSchemas(workspaceRoot) {
  return state(workspaceRoot).schemas;
}

// The set of tool names the native binary currently owns for the workspace.
function getNativeToolNames(workspaceRoot) {
  return state(workspaceRoot).names;
}

function isNativeTool(name, workspaceRoot) {
  return state(workspaceRoot).names.has(name);
}

// Run a native tool: spawn `helpers-native call <name>` with the JSON args on stdin,
// pinned to `opts.workspaceRoot` (the session's resolved project). Returns the
// MCP content array. The binary prints a JSON {error} envelope even on non-zero
// exit, which we surface as a thrown Error.
function runNativeTool(name, args, opts) {
  if (!fs.existsSync(BIN)) throw missingBinaryError();
  const workspaceRoot = opts && opts.workspaceRoot;
  const r = spawnSync(
    BIN,
    ["call", name],
    nativeSpawnOptions(
      {
        input: JSON.stringify(args || {}),
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
      },
      workspaceRoot,
    ),
  );
  let parsed;
  try {
    parsed = JSON.parse(r.stdout);
  } catch {
    const detail = (r.stderr || r.stdout || "no output").trim();
    throw new Error(`helpers-native ${name} failed (exit ${r.status}): ${detail}`);
  }
  // Registering/unregistering a flow changes this workspace's tool surface —
  // drop its cache entry so the change is visible on the next list.
  if (MUTATING_TOOLS.has(name)) _cache.delete(resolveWorkspaceRoot(workspaceRoot));
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
