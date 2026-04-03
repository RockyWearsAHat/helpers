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
// Three injection points in the workbench bundle (RunSubagentTool class):
//
//   1. getToolData() — adds `model` to the JSON schema so the model sees it
//      as a valid parameter and the LLM prompt includes it in tool description
//
//   2. prepareToolInvocation() — resolves the call-time override early so the
//      UI badge / cached tool metadata show the actual subagent model rather
//      than the parent session model
//
//   3. invoke() — applies the override again right before the subagent request
//      object is constructed so both `userSelectedModelId` and
//      `modelConfiguration` pick up the value
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
const os = require("os");
const path = require("path");
const { execFileSync, execSync } = require("child_process");

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
  'properties:{prompt:{type:"string",description:"A detailed description of the task for the agent to perform"},description:{type:"string",description:"A short (3-5 word) description of the task"},model:{type:"string",description:"Optional model identifier for this subagent invocation. Overrides the agent definition\'s default model. Prefer the display name shown in VS Code, for example \'Claude Haiku 4.5\' or \'Claude Sonnet 4.6\'. Common shorthand like \'Haiku 4.5\' and known ids like \'claude-haiku-4.5\' are normalized when possible. When omitted, the agent\'s own model: frontmatter or the parent session model is used."}},required:["prompt","description"]}';

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

const PREPARE_SENTINEL = "let _GSH_RSMM_PREP_=1;";
const PREPARE_RESOLVE = "r=this.resolveSubagentModel(n,e.modelId);";
const PREPARE_ANCHOR =
  'return this._resolvedModels.set(e.toolCallId,r),{invocationMessage:o.description,toolSpecificData:{kind:"subagent",description:o.description,agentName:n?.name,prompt:o.prompt,modelName:r.resolvedModelName}}';
const PREPARE_OLD = PREPARE_RESOLVE + PREPARE_ANCHOR;

// Strategy:
//   1. Try lookupLanguageModelByQualifiedName(input) — works for display names
//      like "Claude Haiku 4.5 (copilot)" AND copilot-vendor models by bare name
//   2. Try lookupLanguageModel(input) — works for internal opaque identifiers
//      like "claude-haiku-4.5", then re-resolve to get the canonical identifier
//   3. Normalize shorthand ("Haiku 4.5" → "Claude Haiku 4.5", "claude-haiku-4.5"
//      → "Claude Haiku 4.5") and retry lookupLanguageModelByQualifiedName
//   4. Name-scan fallback — iterate ALL registered models and match by display
//      name case-insensitively.  This handles models from any vendor (Anthropic,
//      OpenAI, etc.) whose vendor field is not "copilot", which causes
//      lookupLanguageModelByQualifiedName to miss bare-name lookups.
//   5. Last resort: set p=input directly (id passthrough) and derive v from
//      the id by capitalising words
//
// This means "claude-haiku-4.5" (id), "Claude Haiku 4.5" (name), and
// "Claude Haiku 4.5 (Anthropic)" (qualified name) are all accepted.
const INVOKE_BODY =
  "if(r.model){" +
  "let _input=String(r.model).trim();let _lm;" +
  // Step 1: qualified name lookup (handles "Name (vendor)" and copilot bare names)
  "let _qr=this.languageModelsService.lookupLanguageModelByQualifiedName(_input);" +
  "if(_qr?.metadata){_lm=_qr.metadata;p=_qr.identifier;v=_lm.name}" +
  "else{" +
  // Step 2: direct ID lookup
  "let _idMeta=this.languageModelsService.lookupLanguageModel(_input);" +
  "if(_idMeta){" +
  "let _idResult=this.languageModelsService.lookupLanguageModelByQualifiedName(_idMeta.name);" +
  "if(_idResult?.metadata){_lm=_idResult.metadata;p=_idResult.identifier;v=_lm.name}" +
  "else{_lm=_idMeta;p=_input;v=_lm.name}" +
  "}else{" +
  // Step 3: normalize shorthand
  "let _normalized=_input;" +
  "if(/^(haiku|sonnet|opus)\\b/i.test(_normalized)){_normalized='Claude '+_normalized}" +
  "else if(/^claude-(haiku|sonnet|opus)-/i.test(_normalized)){" +
  "_normalized='Claude '+_normalized.replace(/^claude-/i,'').replace(/-fast$/i,' (fast mode)').replace(/-/g,' ').replace(/\\b\\w/g,c=>c.toUpperCase())}" +
  "let _normalizedResult=_normalized!==_input?this.languageModelsService.lookupLanguageModelByQualifiedName(_normalized):void 0;" +
  "if(_normalizedResult?.metadata){_lm=_normalizedResult.metadata;p=_normalizedResult.identifier;v=_lm.name}" +
  "else{" +
  // Step 4: name-scan fallback — iterate all models, match by name (case-insensitive)
  "let _lower=_input.toLowerCase();" +
  "let _found=null;" +
  "for(let[_id,_meta]of this.languageModelsService._modelCache||new Map()){" +
  "if(_meta.name&&_meta.name.toLowerCase()===_lower){_found={id:_id,meta:_meta};break}" +
  "}" +
  "if(!_found){for(let[_id,_meta]of this.languageModelsService._modelCache||new Map()){" +
  "if(_meta.id&&_meta.id.toLowerCase()===_lower){_found={id:_id,meta:_meta};break}" +
  "}}" +
  "if(_found){_lm=_found.meta;p=_found.id;v=_lm.name}" +
  // Step 5: raw passthrough
  "else{p=_input;v=_normalized.replace(/-/g,' ').replace(/\\b\\w/g,c=>c.toUpperCase())}" +
  "}" +
  "}}" +
  "this.logService.info(`[gsh] runSubagent model override → ${p} (${v})`)" +
  "}";

const NEW_INVOKE = INVOKE_SENTINEL + INVOKE_BODY + INVOKE_ANCHOR;

const PREPARE_BODY =
  "if(o.model){" +
  "let _input=String(o.model).trim();let _modeModelId=r.modeModelId;let _resolvedModelName=r.resolvedModelName;let _lm;" +
  "let _qr=this.languageModelsService.lookupLanguageModelByQualifiedName(_input);" +
  "if(_qr?.metadata){_lm=_qr.metadata;_modeModelId=_qr.identifier;_resolvedModelName=_lm.name}" +
  "else{" +
  "let _idMeta=this.languageModelsService.lookupLanguageModel(_input);" +
  "if(_idMeta){" +
  "let _idResult=this.languageModelsService.lookupLanguageModelByQualifiedName(_idMeta.name);" +
  "if(_idResult?.metadata){_lm=_idResult.metadata;_modeModelId=_idResult.identifier;_resolvedModelName=_lm.name}" +
  "else{_lm=_idMeta;_modeModelId=_input;_resolvedModelName=_lm.name}" +
  "}else{" +
  "let _normalized=_input;" +
  "if(/^(haiku|sonnet|opus)\\b/i.test(_normalized)){_normalized='Claude '+_normalized}" +
  "else if(/^claude-(haiku|sonnet|opus)-/i.test(_normalized)){" +
  "_normalized='Claude '+_normalized.replace(/^claude-/i,'').replace(/-fast$/i,' (fast mode)').replace(/-/g,' ').replace(/\\b\\w/g,c=>c.toUpperCase())}" +
  "let _normalizedResult=_normalized!==_input?this.languageModelsService.lookupLanguageModelByQualifiedName(_normalized):void 0;" +
  "if(_normalizedResult?.metadata){_lm=_normalizedResult.metadata;_modeModelId=_normalizedResult.identifier;_resolvedModelName=_lm.name}" +
  "else{" +
  "let _lower=_input.toLowerCase();" +
  "let _found=null;" +
  "for(let[_id,_meta]of this.languageModelsService._modelCache||new Map()){" +
  "if(_meta.name&&_meta.name.toLowerCase()===_lower){_found={id:_id,meta:_meta};break}" +
  "}" +
  "if(!_found){for(let[_id,_meta]of this.languageModelsService._modelCache||new Map()){" +
  "if(_meta.id&&_meta.id.toLowerCase()===_lower){_found={id:_id,meta:_meta};break}" +
  "}}" +
  "if(_found){_lm=_found.meta;_modeModelId=_found.id;_resolvedModelName=_lm.name}" +
  "else{_modeModelId=_input;_resolvedModelName=_normalized.replace(/-/g,' ').replace(/\\b\\w/g,c=>c.toUpperCase())}" +
  "}" +
  "}}" +
  "r={modeModelId:_modeModelId,resolvedModelName:_resolvedModelName}" +
  "}";

const NEW_PREPARE =
  PREPARE_RESOLVE + PREPARE_SENTINEL + PREPARE_BODY + PREPARE_ANCHOR;

// ---------------------------------------------------------------------------
// Patch 1 schema constants
// ---------------------------------------------------------------------------

const PATCHES = [
  {
    old: OLD_SCHEMA,
    new: NEW_SCHEMA,
    name: "schema",
    mark: "Prefer the display name shown in VS Code",
  },
];

// Exported for use by the coordinator script
module.exports = {
  PATCHES,
  BUNDLE,
  INVOKE_SENTINEL,
  INVOKE_ANCHOR,
  PREPARE_SENTINEL,
  PREPARE_RESOLVE,
  PREPARE_ANCHOR,
  PREPARE_OLD,
  NEW_PREPARE,
  NEW_INVOKE,
};

// ---------------------------------------------------------------------------
// Local helpers
// ---------------------------------------------------------------------------

function isPatchable() {
  if (!fs.existsSync(BUNDLE)) return "missing";
  const src = fs.readFileSync(BUNDLE, "utf8");
  const schemaApplied = src.includes(PATCHES[0].mark);
  const prepareApplied = src.includes(PREPARE_SENTINEL);
  const invokeApplied = src.includes(INVOKE_SENTINEL);
  if (schemaApplied && prepareApplied && invokeApplied) return "patched";
  if (!schemaApplied && !prepareApplied && !invokeApplied) return "unpatched";
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

  // Patch 3: prepareToolInvocation — sentinel-based
  if (!bundleSrc.includes(PREPARE_SENTINEL)) {
    const anchorIdx = bundleSrc.indexOf(PREPARE_OLD);
    if (anchorIdx === -1) {
      return {
        src: bundleSrc,
        changed,
        error:
          "prepareToolInvocation anchor not found — VS Code version may have changed.",
      };
    }
    bundleSrc =
      bundleSrc.slice(0, anchorIdx) +
      NEW_PREPARE +
      bundleSrc.slice(anchorIdx + PREPARE_OLD.length);
    changed = true;
  }

  return { src: bundleSrc, changed };
}

function validateJavaScriptSource(source, label) {
  const tempDir = fs.mkdtempSync(
    path.join(os.tmpdir(), "gsh-runsubagent-model-"),
  );
  const tempFile = path.join(
    tempDir,
    `${path.basename(label || "workbench.desktop.main", ".js")}.js`,
  );

  try {
    fs.writeFileSync(tempFile, source, "utf8");
    execFileSync(process.execPath, ["--check", tempFile], {
      stdio: "pipe",
      encoding: "utf8",
    });
    return null;
  } catch (error) {
    const stderr =
      error && typeof error === "object" && "stderr" in error && error.stderr
        ? String(error.stderr).trim()
        : "";
    return stderr || (error instanceof Error ? error.message : String(error));
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

function revert(bundleSrc) {
  if (!bundleSrc) bundleSrc = fs.readFileSync(BUNDLE, "utf8");
  let changed = false;

  // Revert patch 3 (prepareToolInvocation) — sentinel-based: strip from
  // sentinel to anchor and leave PREPARE_ANCHOR in place.
  const prepareSentinelIdx = bundleSrc.indexOf(PREPARE_SENTINEL);
  if (prepareSentinelIdx !== -1) {
    const prepareAnchorIdx = bundleSrc.indexOf(PREPARE_ANCHOR, prepareSentinelIdx);
    if (prepareAnchorIdx !== -1) {
      bundleSrc =
        bundleSrc.slice(0, prepareSentinelIdx) +
        bundleSrc.slice(prepareAnchorIdx);
      changed = true;
    }
  }

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
        `prepare:${src.includes(PREPARE_SENTINEL) ? "yes" : "no"}`,
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

  const validationError = validateJavaScriptSource(result.src, BUNDLE);
  if (validationError) {
    console.error("Patch failed: patched bundle did not pass syntax check.");
    console.error(validationError);
    process.exit(1);
  }

  fs.writeFileSync(BUNDLE, result.src, "utf8");
  console.log("Patched RunSubagentTool — `model` parameter enabled.");
  console.log("Quit and restart VS Code (Cmd+Q, reopen) to activate.");
}
