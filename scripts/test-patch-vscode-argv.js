#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

const { patchArgvFile } = require("./patch-vscode-argv");

function main() {
  const extensionId = "RockyWearsAHat.helpers";
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "helpers-argv-"));

  try {
    const existingPath = path.join(tempDir, "existing.json");
    fs.writeFileSync(
      existingPath,
      '{\n  // Keep my other proposals\n  "enable-proposed-api": ["sample.extension"]\n}\n',
      "utf8",
    );
    const patchedExisting = patchArgvFile(existingPath, extensionId);
    assert.ok(
      patchedExisting.includes(
        '"enable-proposed-api": ["sample.extension","RockyWearsAHat.helpers"]',
      ),
    );
    assert.ok(patchedExisting.includes("// Keep my other proposals"));

    const missingPath = path.join(tempDir, "missing.json");
    fs.writeFileSync(missingPath, '{\n  "window.zoomLevel": 1\n}\n', "utf8");
    const patchedMissing = patchArgvFile(missingPath, extensionId);
    assert.ok(
      patchedMissing.includes(
        '"enable-proposed-api": ["RockyWearsAHat.helpers"]',
      ),
    );
    assert.ok(patchedMissing.includes('"window.zoomLevel": 1,'));

    const duplicatePath = path.join(tempDir, "duplicate.json");
    fs.writeFileSync(
      duplicatePath,
      '{\n  "enable-proposed-api": ["RockyWearsAHat.helpers"]\n}\n',
      "utf8",
    );
    const patchedDuplicate = patchArgvFile(duplicatePath, extensionId);
    const idMatches = patchedDuplicate.match(
      /RockyWearsAHat\.helpers/g,
    );
    assert.strictEqual(idMatches ? idMatches.length : 0, 1);

    console.log("ok");
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

main();
