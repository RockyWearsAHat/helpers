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
  'properties:{prompt:{type:"string",description:"A detailed description of the task for the agent to perform"},description:{type:"string",description:"A short (3-5 word) description of the task"},model:{type:"string",description:"Optional model identifier for this subagent invocation. Overrides the agent definition\'s default model. Examples: claude-sonnet-4.6, claude-haiku-4.5, gpt-4o, gpt-4o-mini, gpt-4.1-mini. Use the list_language_models MCP tool to see all available model ids. Accepts either a model id (e.g. claude-haiku-4.5) or a display name (e.g. \'Claude Haiku 4.5\'). When omitted, the agent\'s own model: frontmatter or the parent session model is used."}},required:["prompt","description"]}';

// ---------------------------------------------------------------------------
// Patch 2: Apply call-time model override just before request construction
// ---------------------------------------------------------------------------
// Sentinel-based design: a unique variable name `_GSH_RSMM_` bookends the
// inject.  Revert strips everything from the sentinel up to (but not
// including) INVOKE_ANCHOR — immune to future edits of the inject body.
//
// `r` = e.parameters (the tool call inputs from the model)
// `p` = modeModelId / userSelectedModelId  (set by resolveSubagentModel)
// `v` = resolvedModelName (displayed in the UI)

const INVOKE_SENTINEL = "let _GSH_RSMM_=1;";
const INVOKE_ANCHOR =
  "let ve={sessionResource:e.context.sessionResource,requestId:e.callId";

// Strategy:
//   1. Try lookupLanguageModel(id) — works for internal opaque identifiers
//   2. Try lookupLanguageModelByQualifiedName(name) — works for display names
//      like "Claude Haiku 4.5" or "Claude Haiku 4.5 (copilot)"
//   3. Fallback: set p=r.model directly (id passthrough like the UI picker
//      does) and derive v from the id by capitalising words
// This means both "claude-haiku-4.5" (id) and "Claude Haiku 4.5" (name)
// are accepted and routed correctly.
const INVOKE_BODY =
  "if(r.model){" +
  "let _lm=this.languageModelsService.lookupLanguageModel(r.model);" +
  "if(_lm){p=r.model;v=_lm.name}" +
  "else{let _qr=this.languageModelsService.lookupLanguageModelByQualifiedName(r.model);if(_qr?.metadata){_lm=_qr.metadata;p=_qr.identifier;v=_lm.name}" +
  "else{p=r.model;v=r.model.replace(/-/g,' ').replace(/\\b\\w/g,c=>c.toUpperCase())}}" +
  "this.logService.info(`[gsh] runSubagent model override → ${p} (${v})`)" +
  "}";

const NEW_INVOKE = INVOKE_SENTINEL + INVOKE_BODY + INVOKE_ANCHOR;

// ---------------------------------------------------------------------------
// Patch 1 schema constants
// ---------------------------------------------------------------------------

const PATCHES = [
  {
    old: OLD_SCHEMA,
    new: NEW_SCHEMA,
    name: "schema",
    mark: "Optional model identifier for this subagent",
  },
];

// Exported for use by the coordinator script
module.exports = {
  PATCHES,
  BUNDLE,
  INVOKE_SENTINEL,
  INVOKE_ANCHOR,
  NEW_INVOKE,
};

// ---------------------------------------------------------------------------
// Local helpers
// ---------------------------------------------------------------------------

function isPatchable() {
  if (!fs.existsSync(BUNDLE)) return "missing";
  const src = fs.readFileSync(BUNDLE, "utf8");
  const schemaApplied = src.includes(PATCHES[0].mark);
  const invokeApplied = src.includes(INVOKE_SENTINEL);
  if (schemaApplied && invokeApplied) return "patched";
  if (!schemaApplied && !invokeApplied) return "unpatched";
  return "partial";
}

function apply(bundleSrc) {
  if (!bundleSrc) bundleSrc = fs.readFileSync(BUNDLE, "utf8");
  let changed = false;

  // Patch 1: schema
  const schemaP = PATCHES[0];
  if (!bundleSrc.includes(schemaP.mark)) {
    const idx = bundleSrc.indexOf(schemaP.old);
    if (idx === -1) {
      return {
        src: bundleSrc,
        changed,
        error:
          "schema injection point not found — VS Code version may have changed.",
      };
    }
    bundleSrc =
      bundleSrc.slice(0, idx) +
      schemaP.new +
      bundleSrc.slice(idx + schemaP.old.length);
    changed = true;
  }

  // Patch 2: invoke — sentinel-based
  if (!bundleSrc.includes(INVOKE_SENTINEL)) {
    const anchorIdx = bundleSrc.indexOf(INVOKE_ANCHOR);
    if (anchorIdx === -1) {
      return {
        src: bundleSrc,
        changed,
        error: "invoke anchor not found — VS Code version may have changed.",
      };
    }
    bundleSrc =
      bundleSrc.slice(0, anchorIdx) +
      NEW_INVOKE +
      bundleSrc.slice(anchorIdx + INVOKE_ANCHOR.length);
    changed = true;
  }

  return { src: bundleSrc, changed };
}

function revert(bundleSrc) {
  if (!bundleSrc) bundleSrc = fs.readFileSync(BUNDLE, "utf8");
  let changed = false;

  // Revert patch 2 (invoke) — sentinel-based: strip from sentinel to anchor
  const sentinelIdx = bundleSrc.indexOf(INVOKE_SENTINEL);
  if (sentinelIdx !== -1) {
    const anchorIdx = bundleSrc.indexOf(INVOKE_ANCHOR, sentinelIdx);
    if (anchorIdx !== -1) {
      // Remove sentinel + inject body; leave INVOKE_ANCHOR in place
      bundleSrc = bundleSrc.slice(0, sentinelIdx) + bundleSrc.slice(anchorIdx);
      changed = true;
    }
  }

  // Revert patch 1 (schema)
  const schemaP = PATCHES[0];
  if (bundleSrc.includes(schemaP.mark)) {
    const idx = bundleSrc.indexOf(schemaP.new);
    if (idx !== -1) {
      bundleSrc =
        bundleSrc.slice(0, idx) +
        schemaP.old +
        bundleSrc.slice(idx + schemaP.new.length);
      changed = true;
    }
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
    } else if (status === "partial") {
      const src = fs.readFileSync(BUNDLE, "utf8");
      const detail = [
        `schema:${src.includes(PATCHES[0].mark) ? "yes" : "no"}`,
        `invoke:${src.includes(INVOKE_SENTINEL) ? "yes" : "no"}`,
      ].join(" ");
      console.log(`PARTIAL — ${detail}`);
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
  if (isPatchable() === "patched") {
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
