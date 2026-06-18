#!/usr/bin/env node
"use strict";

// Smoke test for the native project-index tools (index_project / lookup).
// Skips cleanly when the gsh-native binary has not been built, so CI without a
// Rust toolchain stays green; the authoritative coverage lives in the crate's
// `cargo test` integration tests.

const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync, execFileSync } = require("child_process");

const BIN =
  process.env.GSH_NATIVE_BIN || path.join(__dirname, "..", "gsh-native");

if (!fs.existsSync(BIN)) {
  console.log("SKIP test-project-index: gsh-native not built (run `gsh build`).");
  process.exit(0);
}

function call(tool, payload) {
  const r = spawnSync(BIN, ["call", tool], {
    input: JSON.stringify(payload),
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
  });
  const parsed = JSON.parse(r.stdout);
  if (parsed.error) throw new Error(`${tool}: ${parsed.error.message}`);
  return parsed.content.map((c) => c.text).join("\n");
}

function assert(cond, msg) {
  if (!cond) throw new Error("ASSERT: " + msg);
}

const root = fs.mkdtempSync(path.join(os.tmpdir(), "gsh-pi-"));
try {
  spawnSync("git", ["init", "-q"], { cwd: root });
  fs.mkdirSync(path.join(root, "src"));
  fs.writeFileSync(path.join(root, "src", "core.rs"), "pub fn widget() {}\n");
  fs.writeFileSync(
    path.join(root, "src", "user.rs"),
    "fn run() { widget(); }\n",
  );

  const indexed = call("index_project", { root });
  assert(/files,/.test(indexed), "index_project should report files");

  const lookup = call("lookup", { root, query: "widget" });
  assert(/widget/.test(lookup), "lookup should find widget");
  assert(/core\.rs/.test(lookup), "lookup should point at core.rs");

  console.log("test-project-index passed");
} finally {
  fs.rmSync(root, { recursive: true, force: true });
}
