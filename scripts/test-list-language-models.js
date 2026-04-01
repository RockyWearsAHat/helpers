#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

const {
  handleListLanguageModels,
} = require("../lib/mcp-language-models");

async function main() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "gsh-models-"));
  const missingPath = path.join(tempDir, "missing-models.json");
  const existingPath = path.join(tempDir, "available-models.json");

  const missingResult = await handleListLanguageModels({
    availableModelsPath: missingPath,
  });
  assert.ok(missingResult[0].text.includes(missingPath));

  fs.writeFileSync(
    existingPath,
    JSON.stringify(
      {
        updatedAt: "2026-03-31T00:00:00.000Z",
        models: [
          {
            id: "claude-haiku-4.5",
            name: "Claude Haiku 4.5",
            vendor: "Anthropic",
            qualifiedName: "Claude Haiku 4.5 (Anthropic)",
            maxInputTokens: 200000,
          },
        ],
      },
      null,
      2,
    ),
    "utf8",
  );

  const validResult = await handleListLanguageModels({
    availableModelsPath: existingPath,
  });
  assert.ok(validResult[0].text.includes("Available language models"));
  assert.ok(validResult[0].text.includes("claude-haiku-4.5"));
  assert.ok(validResult[0].text.includes("Claude Haiku 4.5 (Anthropic)"));

  const originalReadFileSync = fs.readFileSync;
  try {
    fs.readFileSync = (filePath, encoding) => {
      if (filePath === existingPath) throw "plain string failure";
      return originalReadFileSync(filePath, encoding);
    };

    const errorResult = await handleListLanguageModels({
      availableModelsPath: existingPath,
    });
    assert.ok(errorResult[0].text.includes(existingPath));
    assert.ok(errorResult[0].text.includes("plain string failure"));
  } finally {
    fs.readFileSync = originalReadFileSync;
  }

  fs.rmSync(tempDir, { recursive: true, force: true });
  console.log("ok");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});