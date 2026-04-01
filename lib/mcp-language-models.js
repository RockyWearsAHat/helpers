"use strict";

const fs = require("fs");
const os = require("os");
const path = require("path");

const LIST_LANGUAGE_MODELS_TOOL = {
  name: "list_language_models",
  description:
    "List the language models available in VS Code's language model service. Returns each model's id, display name, vendor, and qualifiedName. Pass the id or qualifiedName as the `model` parameter when calling runSubagent to route that subagent to a specific model. The list is written by the gsh VS Code extension on startup and whenever the model set changes.",
  inputSchema: {
    type: "object",
    properties: {},
    required: [],
  },
};

function getAvailableModelsPath() {
  let homeDir = "";
  try {
    homeDir = os.homedir();
  } catch {
    homeDir = "";
  }
  if (!homeDir) return "";
  return path.join(homeDir, ".copilot", "available-models.json");
}

function formatReadError(err) {
  return err instanceof Error ? err.message : String(err);
}

async function handleListLanguageModels(options = {}) {
  const availableModelsPath =
    options.availableModelsPath || getAvailableModelsPath();

  if (!availableModelsPath) {
    return [
      {
        type: "text",
        text: "Could not resolve a home directory to locate available-models.json. Make sure the gsh VS Code extension is installed and the runtime has a valid home directory.",
      },
    ];
  }

  if (!fs.existsSync(availableModelsPath)) {
    return [
      {
        type: "text",
        text: `No available-models.json found at ${availableModelsPath}. Make sure the gsh VS Code extension is installed and VS Code has been restarted after the latest extension update.`,
      },
    ];
  }

  try {
    const data = JSON.parse(fs.readFileSync(availableModelsPath, "utf8"));
    const lines = [
      `Available language models (updated ${data.updatedAt || "unknown"}):`,
      "",
      "| id | name | vendor | qualifiedName | maxInputTokens |",
      "|----|------|--------|---------------|----------------|",
      ...(data.models || []).map(
        (model) =>
          `| ${model.id} | ${model.name} | ${model.vendor} | ${model.qualifiedName} | ${model.maxInputTokens} |`,
      ),
      "",
      "Pass `id` or `qualifiedName` as the `model` param in runSubagent.",
      'Copilot models also accept the bare display name, e.g. "Claude Haiku 4.5".',
    ];
    return [{ type: "text", text: lines.join("\n") }];
  } catch (err) {
    const message = formatReadError(err);
    return [
      {
        type: "text",
        text: `Failed to read ${availableModelsPath}: ${message}`,
      },
    ];
  }
}

module.exports = {
  LIST_LANGUAGE_MODELS_TOOL,
  getAvailableModelsPath,
  handleListLanguageModels,
  formatReadError,
};
