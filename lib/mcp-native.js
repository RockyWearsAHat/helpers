// lib/mcp-native.js — bridge from the Node MCP daemon to the native Rust tools.
//
// The pragmatic-port architecture keeps the Node process as the MCP host but
// moves the hot, portable tool *implementations* into the `gsh-native` Rust
// binary. This module owns the list of tools that live in Rust, fetches their
// schemas from the binary, and dispatches calls to it. There is intentionally
// NO Node fallback: if the binary is missing the tools are required, so we throw
// a clear, actionable error (the daemon surfaces it; `gsh doctor`/`gsh build`
// catch it earlier).
"use strict";

const path = require("path");
const fs = require("fs");
const { spawnSync } = require("child_process");

// Installed binary lives at the repo root (built/copied by `gsh build`).
const BIN = process.env.GSH_NATIVE_BIN || path.join(__dirname, "..", "gsh-native");

// Tools owned by the native binary. This MUST match `registry::all_tools()` in
// the Rust crate. It grows as JS implementations are ported and deleted.
const NATIVE_TOOL_NAMES = new Set([
  "workspace_context",
  "checkpoint",
  "strict_lint",
  "index_project",
  "project_map",
  "lookup",
]);

function isNativeTool(name) {
  return NATIVE_TOOL_NAMES.has(name);
}

function missingBinaryError() {
  return new Error(
    `Native GSH tools are unavailable: gsh-native binary not found at ${BIN}. ` +
      "These tools are required and have no Node fallback — install Rust " +
      "(https://rustup.rs), then run `gsh build`.",
  );
}

// Load the native tool schemas once at daemon startup. Throws (fail-loud) if the
// binary is missing or misbehaving, so a broken native build can't silently
// drop the tools from the advertised surface.
function loadNativeSchemas() {
  if (!fs.existsSync(BIN)) throw missingBinaryError();
  const r = spawnSync(BIN, ["schemas"], {
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });
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
  return parsed;
}

// Run a native tool: spawn `gsh-native call <name>` with the JSON args on stdin.
// Returns the MCP content array. The binary prints a JSON {error} envelope even
// on non-zero exit, which we surface as a thrown Error.
function runNativeTool(name, args) {
  if (!fs.existsSync(BIN)) throw missingBinaryError();
  const r = spawnSync(BIN, ["call", name], {
    input: JSON.stringify(args || {}),
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  let parsed;
  try {
    parsed = JSON.parse(r.stdout);
  } catch {
    const detail = (r.stderr || r.stdout || "no output").trim();
    throw new Error(`gsh-native ${name} failed (exit ${r.status}): ${detail}`);
  }
  if (parsed && parsed.error) {
    throw new Error(parsed.error.message || `native tool ${name} failed`);
  }
  return (parsed && parsed.content) || [];
}

module.exports = {
  BIN,
  NATIVE_TOOL_NAMES,
  isNativeTool,
  loadNativeSchemas,
  runNativeTool,
};
