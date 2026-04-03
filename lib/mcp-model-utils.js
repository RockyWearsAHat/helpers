// lib/mcp-model-utils.js — shared model resolution utilities
// Used by mcp-checkpoint and other tools that need to select a Copilot model.
"use strict";

const fs = require("fs");
const os = require("os");
const path = require("path");

// Cheapest models suitable for commit message generation, in preference order.
// These are checked against available-models.json at runtime.
const CHEAP_MODEL_PREFERENCE = [
  "gpt-5.4-mini",
  "gpt-5-mini",
  "claude-haiku-4.5",
  "gpt-4o-mini",
  "copilot-fast",
  "gpt-4.1",
  "qwen3:4b",
];

/**
 * Read ~/.copilot/available-models.json and return the models array.
 * Returns [] silently if the file is absent or malformed.
 */
function loadAvailableModels() {
  try {
    const p = path.join(os.homedir(), ".copilot", "available-models.json");
    const data = JSON.parse(fs.readFileSync(p, "utf8"));
    return Array.isArray(data.models) ? data.models : [];
  } catch {
    return [];
  }
}

// Normalize a string for fuzzy comparison: lowercase, strip spaces / dashes / dots / parens.
function normalizeForMatch(s) {
  return String(s || "")
    .toLowerCase()
    .replace(/[\s\-_.:()\[\]]/g, "");
}

/**
 * Resolve a user-supplied model name or id to the canonical model ID used
 * with `copilot --model`. Matching order:
 *   1. Exact id match
 *   2. Exact name match (case-insensitive)
 *   3. Exact qualifiedName match (case-insensitive)
 *   4. Fuzzy match (normalized id / name / qualifiedName)
 *   5. Partial prefix match on normalized id or name
 *
 * Returns the matched model's id, or null when no match is found.
 */
function resolveModelId(input, models) {
  if (!input || !models || models.length === 0) return null;
  const inp = String(input).trim();
  if (!inp) return null;
  const inpNorm = normalizeForMatch(inp);

  // 1. Exact ID
  const byId = models.find((m) => m.id === inp);
  if (byId) return byId.id;

  // 2. Exact name (case-insensitive)
  const byName = models.find(
    (m) => m.name && m.name.toLowerCase() === inp.toLowerCase(),
  );
  if (byName) return byName.id;

  // 3. Exact qualifiedName (case-insensitive)
  const byQn = models.find(
    (m) =>
      m.qualifiedName && m.qualifiedName.toLowerCase() === inp.toLowerCase(),
  );
  if (byQn) return byQn.id;

  // 4. Fuzzy: normalize all fields and compare
  const byFuzzy = models.find(
    (m) =>
      normalizeForMatch(m.id) === inpNorm ||
      normalizeForMatch(m.name) === inpNorm ||
      normalizeForMatch(m.qualifiedName) === inpNorm,
  );
  if (byFuzzy) return byFuzzy.id;

  // 5. Partial prefix: inpNorm is a substring of normalized id or name
  const byPartial = models.find(
    (m) =>
      normalizeForMatch(m.id).includes(inpNorm) ||
      normalizeForMatch(m.name).includes(inpNorm),
  );
  if (byPartial) return byPartial.id;

  return null;
}

/**
 * Detect the cheapest suitable model from the given models list.
 * Returns a model ID string; falls back to "gpt-4o-mini" if nothing
 * from the preference list is available.
 */
function detectCheapModel(models) {
  for (const preferred of CHEAP_MODEL_PREFERENCE) {
    if (models.find((m) => m.id === preferred)) return preferred;
  }
  return "gpt-4o-mini";
}

module.exports = {
  CHEAP_MODEL_PREFERENCE,
  loadAvailableModels,
  resolveModelId,
  detectCheapModel,
};
