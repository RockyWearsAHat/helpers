#!/usr/bin/env node
// patch-vscode-apply-all.js
//
// Coordinator script that applies VS Code workbench patches.
// Manages a single backup of the pristine bundle and applies patches in sequence.
//
// Patches:
//   folder-switch  — removes workspace folder switch confirmation dialog
//
// Note: Chat session tracking uses the proposed API (chatParticipantPrivate)
// enabled via ~/.vscode/argv.json — no workbench patch needed.
//
// Usage:
//   node patch-vscode-apply-all.js           # apply all patches
//   node patch-vscode-apply-all.js --check   # check status of all patches
//   node patch-vscode-apply-all.js --revert  # restore pristine bundle
//   node patch-vscode-apply-all.js --json    # output status as JSON (for extension)

const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const BUNDLE = path.join(
  "/Applications/Visual Studio Code.app/Contents/Resources/app/out/vs/workbench",
  "workbench.desktop.main.js",
);
const BACKUP = BUNDLE + ".pristine";

const PATCHES_DIR = __dirname;

const PATCH_DEFS = [
  {
    name: "folder-switch",
    description: "Remove workspace folder switch confirmation dialog",
    script: path.join(PATCHES_DIR, "patch-vscode-folder-switch.js"),
  },
];

function checkPatch(patchScript) {
  try {
    execSync(`node "${patchScript}" --check`, { stdio: "pipe" });
    return "patched";
  } catch {
    return "unpatched";
  }
}

function getStatus() {
  const results = [];
  for (const def of PATCH_DEFS) {
    results.push({
      name: def.name,
      description: def.description,
      status: checkPatch(def.script),
    });
  }
  return {
    bundleExists: fs.existsSync(BUNDLE),
    backupExists: fs.existsSync(BACKUP),
    patches: results,
    allPatched: results.every((r) => r.status === "patched"),
  };
}

// --json mode: output machine-readable status
if (process.argv.includes("--json")) {
  console.log(JSON.stringify(getStatus()));
  process.exit(0);
}

// --check mode: human-readable status
if (process.argv.includes("--check")) {
  if (!fs.existsSync(BUNDLE)) {
    console.error("Bundle not found at", BUNDLE);
    process.exit(1);
  }
  const status = getStatus();
  console.log("VS Code Workbench Patches");
  console.log("=".repeat(40));
  for (const p of status.patches) {
    const icon = p.status === "patched" ? "✓" : "✗";
    console.log(`  ${icon} ${p.name}: ${p.status}`);
    console.log(`    ${p.description}`);
  }
  console.log("");
  if (status.allPatched) {
    console.log("All patches applied.");
  } else {
    console.log("Some patches missing. Run without --check to apply.");
  }
  console.log(`Pristine backup: ${status.backupExists ? "exists" : "missing"}`);
  process.exit(status.allPatched ? 0 : 1);
}

// --revert mode: restore pristine bundle
if (process.argv.includes("--revert")) {
  if (!fs.existsSync(BACKUP)) {
    console.error("No pristine backup found at", BACKUP);
    console.error("Cannot revert without a backup.");
    process.exit(1);
  }
  fs.copyFileSync(BACKUP, BUNDLE);
  console.log("Restored pristine bundle from backup.");
  console.log("Quit and restart VS Code to activate.");
  process.exit(0);
}

// Apply mode: backup pristine bundle, then apply all patches
if (!fs.existsSync(BUNDLE)) {
  console.error("Bundle not found at", BUNDLE);
  process.exit(1);
}

// Create pristine backup from the current bundle if none exists
// If the bundle already has patches, we need to strip them first.
// Strategy: if no pristine backup exists, check if bundle looks unpatched.
if (!fs.existsSync(BACKUP)) {
  const src = fs.readFileSync(BUNDLE, "utf8");
  const hasAnyPatch =
    src.includes("active-chat-session.json") ||  // legacy chat-bridge patch
    src.includes("_doStopExtensionHosts();let o=xg");  // folder-switch patch
  if (hasAnyPatch) {
    // Bundle has patches but no pristine backup — check for legacy backups
    const legacyBak = BUNDLE + ".bak";
    const legacyFolderBak = BUNDLE + ".folder-switch.bak";
    if (fs.existsSync(legacyBak)) {
      fs.copyFileSync(legacyBak, BACKUP);
      console.log("Created pristine backup from legacy .bak file.");
    } else if (fs.existsSync(legacyFolderBak)) {
      fs.copyFileSync(legacyFolderBak, BACKUP);
      console.log(
        "Created pristine backup from legacy folder-switch.bak file.",
      );
    } else {
      console.error("Bundle already has patches but no pristine backup found.");
      console.error(
        "Cannot safely manage patches. Manual intervention needed.",
      );
      process.exit(1);
    }
  } else {
    fs.copyFileSync(BUNDLE, BACKUP);
    console.log("Created pristine backup.");
  }
}

// Start from pristine and apply all patches fresh
// This ensures patches never conflict with each other
fs.copyFileSync(BACKUP, BUNDLE);
console.log("Restored pristine bundle for clean patch application.");

let failed = false;
for (const def of PATCH_DEFS) {
  try {
    const output = execSync(`node "${def.script}"`, {
      stdio: "pipe",
      encoding: "utf8",
    });
    console.log(`[${def.name}] ${output.trim()}`);
  } catch (err) {
    console.error(`[${def.name}] FAILED: ${err.stderr || err.message}`);
    failed = true;
    break;
  }
}

if (failed) {
  console.error("\nPatch application failed. Restoring pristine bundle.");
  fs.copyFileSync(BACKUP, BUNDLE);
  process.exit(1);
}

// Clean up legacy backup files
for (const legacy of [BUNDLE + ".bak", BUNDLE + ".folder-switch.bak"]) {
  if (fs.existsSync(legacy)) {
    fs.unlinkSync(legacy);
    console.log(`Cleaned up legacy backup: ${path.basename(legacy)}`);
  }
}

console.log("\nAll patches applied successfully.");
console.log("Quit and restart VS Code to activate (Cmd+Q, then reopen).");
console.log(
  "Note: Reload Window is NOT sufficient — the bundle is cached by Electron.",
);
