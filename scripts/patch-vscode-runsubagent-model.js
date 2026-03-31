#!/usr/bin/env node
// patch-vscode-runsubagent-model.js
//
// Patches VS Code's workbench bundle to allow the runSubagent tool to accept
// an optional `model` parameter, giving orchestrator models call-time control
// over which model a subagent invocation uses.
//
// Without this patch, a subagent's model is determined statically from:
//   1. The agent definition's `model:` frontmatter field (if present)
//   2. The parent session's model (fallback)
//
// With this patch, the calling model can pass `model: "claude-haiku-4-5"` to
// runSubagent and it will override the agent's default — enabling cost-
// proportional routing where lightweight steps use cheaper models and complex
// steps use more capable ones.
//
// Two injection points in the workbench bundle (RunSubagentTool class):
//
//   1. getToolData() — adds `model` to the JSON schema so the model sees it
//      as a valid parameter and the LLM prompt includes it in tool description
//
//   2. invoke() — applies the override after resolveSubagentModel() runs,
//      right before the subagent request object is constructed so both
//      `userSelectedModelId` and `modelConfiguration` pick up the value
//
// Upstream proposal: proposals/004-runsubagent-model-param.md
//
// Usage (standalone — normally called via patch-vscode-apply-all.js):
//   node patch-vscode-runsubagent-model.js          # apply patch
//   node patch-vscode-runsubagent-model.js --check  # check status
//   node patch-vscode-runsubagent-model.js --revert # revert to backup
//
// Requires: VS Code restart (Cmd+Q, reopen) — workbench bundle.

"use strict";

const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

function detectVscodePath() {
  const candidates =
    process.platform === "darwin"
      ? [
          "/Applications/Visual Studio Code.app/Contents/Resources/app",
          (process.env.HOME || "") +
            "/Applications/Visual Studio Code.app/Contents/Resources/app",
        ]
      : process.platform === "win32"
        ? [
            (process.env.LOCALAPPDATA || "") +
              "\\Programs\\Microsoft VS Code\\resources\\app",
            "C:\\Program Files\\Microsoft VS Code\\resources\\app",
            "C:\\Program Files (x86)\\Microsoft VS Code\\resources\\app",
          ]
        : [
            "/usr/share/code/resources/app",
            "/opt/visual-studio-code/resources/app",
            "/snap/code/current/usr/share/code/resources/app",
          ];
  for (const c of candidates) {
    if (c && fs.existsSync(c)) return c;
  }
  try {
    const probe = process.platform === "win32" ? "where code" : "which code";
    const codeExe = execSync(probe, {
      timeout: 3000,
      stdio: ["pipe", "pipe", "pipe"],
    })
      .toString()
      .trim()
      .split("\n")[0]
      .trim();
    if (codeExe) {
      let dir = path.dirname(fs.realpathSync(codeExe));
      for (let i = 0; i < 8; i++) {
        const candidate = path.join(dir, "resources", "app");
        if (fs.existsSync(candidate)) return candidate;
        dir = path.dirname(dir);
      }
    }
  } catch {}
  return null;
}

const VSCODE_PATH = detectVscodePath();
if (!VSCODE_PATH) {
  console.error(
    "[patch-vscode] Could not locate VS Code installation. Tried platform defaults and PATH.",
  );
  process.exit(1);
}

const BUNDLE = path.join(
  VSCODE_PATH,
  "out/vs/workbench/workbench.desktop.main.js",
);

// ---------------------------------------------------------------------------
// Patch 1: Add `model` property to the runSubagent tool JSON schema
// ---------------------------------------------------------------------------
// Before: schema exposes only `prompt` and `description`
// After:  schema also exposes optional `model` for call-time model selection

const OLD_SCHEMA =
  'properties:{prompt:{type:"string",description:"A detailed description of the task for the agent to perform"},description:{type:"string",description:"A short (3-5 word) description of the task"}},required:["prompt","description"]}';

const NEW_SCHEMA =
  'properties:{prompt:{type:"string",description:"A detailed description of the task for the agent to perform"},description:{type:"string",description:"A short (3-5 word) description of the task"},model:{type:"string",description:"Optional model identifier for this subagent invocation. Overrides the agent definition\'s default model. Examples: claude-sonnet-4-6, claude-haiku-4-5, gpt-4o, gpt-4o-mini."}},required:["prompt","description"]}';

// ---------------------------------------------------------------------------
// Patch 2: Apply call-time model override just before request construction
// ---------------------------------------------------------------------------
// The variable `p` holds the resolved userSelectedModelId and `v` holds the
// human-readable model name.  We override both if `r.model` is provided and
// the model is found in the registry.  This runs after resolveSubagentModel()
// so the agent definition's model is computed first and our override wins.
//
// `r` = e.parameters (the tool call inputs from the model)
// `p` = modeModelId / userSelectedModelId
// `v` = resolvedModelName (displayed in the UI)

const OLD_INVOKE =
  "let ve={sessionResource:e.context.sessionResource,requestId:e.callId";

const NEW_INVOKE =
  "if(r.model){let _smo=this.languageModelsService.lookupLanguageModel(r.model);_smo&&(p=r.model,v=_smo.name)}" +
  "let ve={sessionResource:e.context.sessionResource,requestId:e.callId";

// ---------------------------------------------------------------------------
// Patch registry
// ---------------------------------------------------------------------------

const PATCHES = [
  { old: OLD_SCHEMA, new: NEW_SCHEMA, name: "schema" },
  { old: OLD_INVOKE, new: NEW_INVOKE, name: "invoke" },
];

// Exported for use by the coordinator script
module.exports = { PATCHES, BUNDLE };

// ---------------------------------------------------------------------------
// Local helpers
// ---------------------------------------------------------------------------

function isPatchable() {
  if (!fs.existsSync(BUNDLE)) return "missing";
  const src = fs.readFileSync(BUNDLE, "utf8");
  const allPatched = PATCHES.every((p) => src.includes(p.new.slice(0, 60)));
  if (allPatched) return "patched";
  const allOriginal = PATCHES.every((p) => src.includes(p.old));
  if (allOriginal) return "unpatched";
  return "partial";
}

function apply(bundleSrc) {
  if (!bundleSrc) bundleSrc = fs.readFileSync(BUNDLE, "utf8");
  let changed = false;
  for (const p of PATCHES) {
    if (bundleSrc.includes(p.new.slice(0, 60))) continue;
    const idx = bundleSrc.indexOf(p.old);
    if (idx === -1) {
      return {
        src: bundleSrc,
        changed,
        error: `injection point not found for '${p.name}' — VS Code version may have changed.`,
      };
    }
    bundleSrc =
      bundleSrc.slice(0, idx) + p.new + bundleSrc.slice(idx + p.old.length);
    changed = true;
  }
  return { src: bundleSrc, changed };
}

function revert(bundleSrc) {
  if (!bundleSrc) bundleSrc = fs.readFileSync(BUNDLE, "utf8");
  let changed = false;
  for (const p of [...PATCHES].reverse()) {
    if (!bundleSrc.includes(p.new.slice(0, 60))) continue;
    const idx = bundleSrc.indexOf(p.new);
    if (idx === -1) continue;
    bundleSrc =
      bundleSrc.slice(0, idx) + p.old + bundleSrc.slice(idx + p.new.length);
    changed = true;
  }
  return { src: bundleSrc, changed };
}

// ---------------------------------------------------------------------------
// Standalone CLI
// ---------------------------------------------------------------------------

if (require.main === module) {
  const arg = process.argv[2];

  if (arg === "--check") {
    const status = isPatchable();
    if (status === "patched") {
      console.log("PATCHED — runSubagent model parameter enabled.");
      process.exit(0);
    } else if (status === "unpatched") {
      console.log("UNPATCHED");
      process.exit(1);
    } else {
      console.log(
        "UNKNOWN — injection point not found. VS Code version may have changed.",
      );
      process.exit(1);
    }
  }

  if (arg === "--revert") {
    if (!fs.existsSync(BUNDLE)) {
      console.error("Bundle not found at", BUNDLE);
      process.exit(1);
    }
    const result = revert();
    if (result.changed) {
      fs.writeFileSync(BUNDLE, result.src, "utf8");
      console.log("Reverted runSubagent model patch.");
      console.log("Quit and restart VS Code to deactivate.");
    } else {
      console.log("Nothing to revert — patch not applied.");
    }
    process.exit(0);
  }

  // Apply mode
  if (!fs.existsSync(BUNDLE)) {
    console.error("Bundle not found at", BUNDLE);
    process.exit(1);
  }

  const src = fs.readFileSync(BUNDLE, "utf8");
  if (src.includes(NEW_SCHEMA.slice(0, 60))) {
    console.log("Already patched. Nothing to apply.");
    process.exit(0);
  }

  const result = apply(src);
  if (result.error) {
    console.error("Patch failed:", result.error);
    process.exit(1);
  }

  fs.writeFileSync(BUNDLE, result.src, "utf8");
  console.log("Patched RunSubagentTool — `model` parameter enabled.");
  console.log("Quit and restart VS Code (Cmd+Q, reopen) to activate.");
}
