#!/usr/bin/env node
// patch-vscode-folder-switch.js
//
// Patches VS Code's workbench bundle to allow switching workspace folders
// without a confirmation dialog. This enables seamless git worktree checkout:
// calling updateWorkspaceFolders(0, 1, {uri: worktreeUri}) from an extension
// silently transitions to the worktree folder instead of prompting.
//
// How it works:
//   VS Code's enterWorkspace() normally calls stopExtensionHosts(reason)
//   which fires a veto event — any participant (unsaved files, debug session)
//   can show a blocking dialog. This patch replaces that with a direct
//   _doStopExtensionHosts() call that skips the veto/dialog entirely.
//   Extension hosts still restart cleanly; only the dialog is removed.
//
//   First switch: ~2s extension host restart (FOLDER → WORKSPACE state).
//   Subsequent switches: instant dynamic update (already in WORKSPACE state).
//
// What changes:
//   enterWorkspace(e) {
//  -  if (!await this.extensionService.stopExtensionHosts(reason)) return;
//  +  await this.extensionService._doStopExtensionHosts();
//     ...rest unchanged...
//   }
//
// Usage:
//   node patch-vscode-folder-switch.js          # apply patch
//   node patch-vscode-folder-switch.js --revert # restore original
//   node patch-vscode-folder-switch.js --check  # check patch status

const fs = require("fs");
const path = require("path");

const BUNDLE = path.join(
  "/Applications/Visual Studio Code.app/Contents/Resources/app/out/vs/workbench",
  "workbench.desktop.main.js",
);
const BACKUP = BUNDLE + ".folder-switch.bak";

const OLD =
  "async enterWorkspace(e){if(!await this.extensionService.stopExtensionHosts(d(18199,null)))return;let o=xg(this.contextService.getWorkspace())";

const NEW =
  "async enterWorkspace(e){await this.extensionService._doStopExtensionHosts();let o=xg(this.contextService.getWorkspace())";

if (process.argv.includes("--check")) {
  if (!fs.existsSync(BUNDLE)) {
    console.error("Bundle not found at", BUNDLE);
    process.exit(1);
  }
  const src = fs.readFileSync(BUNDLE, "utf8");
  if (src.includes(NEW)) {
    console.log("PATCHED — folder switch dialog is disabled.");
  } else if (src.includes(OLD)) {
    console.log("UNPATCHED — original enterWorkspace with dialog.");
  } else {
    console.log(
      "UNKNOWN — neither patched nor original signature found.",
    );
    console.log(
      "VS Code version may have changed. Check enterWorkspace signature.",
    );
  }
  process.exit(0);
}

if (process.argv.includes("--revert")) {
  if (!fs.existsSync(BACKUP)) {
    console.error("No backup found at", BACKUP);
    process.exit(1);
  }
  fs.copyFileSync(BACKUP, BUNDLE);
  console.log("Reverted to original bundle.");
  process.exit(0);
}

const src = fs.readFileSync(BUNDLE, "utf8");

if (src.includes(NEW)) {
  console.log("Bundle already patched.");
  process.exit(0);
}

const idx = src.indexOf(OLD);
if (idx === -1) {
  console.error("Could not find injection point in bundle.");
  console.error(
    "VS Code version may have changed — check enterWorkspace signature.",
  );
  console.error("Run with --check for diagnostics.");
  process.exit(1);
}

if (!fs.existsSync(BACKUP)) {
  fs.copyFileSync(BUNDLE, BACKUP);
  console.log("Backed up original bundle.");
}

const patched = src.slice(0, idx) + NEW + src.slice(idx + OLD.length);
fs.writeFileSync(BUNDLE, patched, "utf8");

console.log("Patched enterWorkspace — folder switch dialog removed.");
console.log("Extension hosts will restart silently on workspace folder changes.");
console.log("");
console.log("First folder switch: ~2s (FOLDER→WORKSPACE state transition).");
console.log("Subsequent switches: instant (dynamic WORKSPACE update).");
console.log("");
console.log("Reload VS Code window to activate (Cmd+Shift+P → Reload Window).");
