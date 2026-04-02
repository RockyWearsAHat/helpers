#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

const {
  escapeMarkdownTableCell,
  formatReadError,
  getAvailableModelsPath,
  handleListLanguageModels,
} = require("../lib/mcp-language-models");

async function main() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "gsh-models-"));
  const defaultModelsPath = path.join(
    tempDir,
    ".copilot",
    "available-models.json",
  );
  const missingPath = path.join(tempDir, "missing-models.json");
  const existingPath = path.join(tempDir, "available-models.json");
  const fallbackPath = path.join(tempDir, "fallback-models.json");
  const originalHomedir = os.homedir;

  os.homedir = () => tempDir;
  assert.strictEqual(getAvailableModelsPath(), defaultModelsPath);

  os.homedir = () => {
    throw new Error("homedir unavailable");
  };
  assert.strictEqual(getAvailableModelsPath(), "");
  assert.strictEqual(formatReadError(new Error("real error")), "real error");
  assert.strictEqual(
    formatReadError("plain string failure"),
    "plain string failure",
  );
  assert.strictEqual(
    escapeMarkdownTableCell("Claude | Sonnet\nAnthropic"),
    "Claude \\| Sonnet<br>Anthropic",
  );
  assert.strictEqual(escapeMarkdownTableCell(undefined), "");

  const noHomeDirResult = await handleListLanguageModels();
  assert.ok(
    noHomeDirResult[0].text.includes("Could not resolve a home directory"),
  );

  os.homedir = () => tempDir;
  const defaultMissingResult = await handleListLanguageModels();
  assert.ok(defaultMissingResult[0].text.includes(defaultModelsPath));

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
          {
            id: "provider|edge",
            name: "Provider | Edge",
            vendor: "OpenAI\nLabs",
            qualifiedName: "Provider | Edge\n(OpenAI Labs)",
            maxInputTokens: 12345,
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
  assert.ok(validResult[0].text.includes("provider\\|edge"));
  assert.ok(validResult[0].text.includes("Provider \\| Edge"));
  assert.ok(validResult[0].text.includes("OpenAI<br>Labs"));

  fs.writeFileSync(fallbackPath, JSON.stringify({}, null, 2), "utf8");

  const fallbackResult = await handleListLanguageModels({
    availableModelsPath: fallbackPath,
  });
  assert.ok(fallbackResult[0].text.includes("updated unknown"));

  const originalReadFileSync = fs.readFileSync;
  try {
    fs.readFileSync = (filePath, encoding) => {
      if (filePath === fallbackPath) {
        throw new Error("real error");
      }
      return originalReadFileSync(filePath, encoding);
    };

    const errorObjectResult = await handleListLanguageModels({
      availableModelsPath: fallbackPath,
    });
    assert.ok(errorObjectResult[0].text.includes(fallbackPath));
    assert.ok(errorObjectResult[0].text.includes("real error"));

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
    os.homedir = originalHomedir;
  }

  fs.rmSync(tempDir, { recursive: true, force: true });
  console.log("ok");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
