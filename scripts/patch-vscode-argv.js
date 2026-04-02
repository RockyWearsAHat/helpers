#!/usr/bin/env node
"use strict";

const fs = require("fs");

function usage() {
  console.error("Usage: patch-vscode-argv.js <argv.json path> <extension id>");
}

function patchArgvFile(argvFile, extensionId) {
  const raw = fs.readFileSync(argvFile, "utf8");
  const stripped = raw.replace(/^\s*\/\/.*$/gm, "");
  const parsed = JSON.parse(stripped || "{}");
  const existing = Array.isArray(parsed["enable-proposed-api"])
    ? parsed["enable-proposed-api"].slice()
    : [];

  if (!existing.includes(extensionId)) {
    existing.push(extensionId);
  }

  const arrayText = JSON.stringify(existing);
  if (/"enable-proposed-api"\s*:/.test(raw)) {
    return raw.replace(
      /("enable-proposed-api"\s*:\s*)\[[\s\S]*?\]/,
      `$1${arrayText}`,
    );
  }

  const closingBrace = raw.lastIndexOf("}");
  if (closingBrace === -1) {
    throw new Error("argv.json is missing a closing brace");
  }

  const before = raw.slice(0, closingBrace).trimEnd();
  const needsComma = /[}\]"\d]$/.test(before);
  const insertion = `${needsComma ? "," : ""}\n\n\t// Enable proposed APIs for Git Shell Helpers extension.\n\t// Required for branch-per-chat session tracking.\n\t"enable-proposed-api": ${arrayText}\n`;
  return `${before}${insertion}}\n`;
}

function main() {
  const [, , argvFile, extensionId] = process.argv;
  if (!argvFile || !extensionId) {
    usage();
    process.exit(1);
  }

  const updated = patchArgvFile(argvFile, extensionId);
  fs.writeFileSync(argvFile, updated);
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(message);
    process.exit(1);
  }
}

module.exports = { patchArgvFile };
