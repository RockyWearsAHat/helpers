#!/usr/bin/env node
"use strict";
// Thin npm wrapper: exec the native helpers-native binary's CLI. The binary is
// downloaded by the postinstall into the package root. All real logic is native.

const path = require("path");
const { spawnSync } = require("child_process");

const exe = process.platform === "win32" ? ".exe" : "";
const bin = path.resolve(__dirname, "..", `helpers-native${exe}`);

const r = spawnSync(bin, ["cli", ...process.argv.slice(2)], { stdio: "inherit" });
if (r.error) {
  console.error(
    `helpers: native binary not found (${bin}). Reinstall, or run: helpers build --from-source`,
  );
  process.exit(1);
}
process.exit(r.status === null ? 1 : r.status);
