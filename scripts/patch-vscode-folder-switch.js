#!/usr/bin/env node
// patch-vscode-folder-switch.js
//
// Patches VS Code's workbench bundle to allow switching workspace folders
// without a confirmation dialog. This enables seamless git worktree checkout:
// calling updateWorkspaceFolders(0, 1, {uri: worktreeUri}) from an extension
// silently transitions to the worktree folder instead of prompting.
//
// What changes:
//   enterWorkspace(e) {
//  -  if (!await this.extensionService.stopExtensionHosts(reason)) return;
//  +  await this.extensionService._doStopExtensionHosts();
//     ...rest unchanged...
//   }
//
// Why this patch exists:
//   Extensions cannot programmatically switch workspace folders without
//   triggering a user-facing confirmation dialog. For automated workflows
//   (like branch-per-chat worktree switching), the dialog is a blocker.
//   The extension host still restarts cleanly — only the dialog is skipped.
//   Upstream PR candidate: add a suppressDialogs option to updateWorkspaceFolders.
//
// Usage:
//   node patch-vscode-folder-switch.js          # apply patch
//   node patch-vscode-folder-switch.js --check  # check patch status (exit 0=patched, 1=not)
//
// This script is meant to be called by patch-vscode-apply-all.js which handles
// backup, revert, and coordination with other patches.

const fs = require("fs");
const path = require("path");

const BUNDLE = path.join(
  "/Applications/Visual Studio Code.app/Contents/Resources/app/out/vs/workbench",
  "workbench.desktop.main.js",
);

const OLD =
  "async enterWorkspace(e){if(!await this.extensionService.stopExtensionHosts(d(18199,null)))return;let o=xg(this.contextService.getWorkspace())";

const NEW =
  "async enterWorkspace(e){await this.extensionService._doStopExtensionHosts();let o=xg(this.contextService.getWorkspace())";

// Exported for use by the coordinator script
module.exports = { OLD, NEW, BUNDLE };

if (require.main === module) {
  if (!fs.existsSync(BUNDLE)) {
    console.error("Bundle not found at", BUNDLE);
    process.exit(1);
  }
  const src = fs.readFileSync(BUNDLE, "utf8");

  if (process.argv.includes("--check")) {
    if (src.includes(NEW)) {
      console.log("PATCHED — folder switch dialog disabled.");
      process.exit(0);
    } else if (src.includes(OLD)) {
      console.log("UNPATCHED");
      process.exit(1);
    } else {
      console.log("UNKNOWN — injection point not found. VS Code version may have changed.");
      process.exit(1);
    }
  }

  // Apply mode: read current bundle and apply patch in-place
  if (src.includes(NEW)) {
    console.log("Already patched. Nothing to apply.");
    process.exit(0);
  }

  const idx = src.indexOf(OLD);
  if (idx === -1) {
    console.error("Injection point not found — VS Code version may have changed.");
    process.exit(1);
  }

  const patched = src.slice(0, idx) + NEW + src.slice(idx + OLD.length);
  fs.writeFileSync(BUNDLE, patched, "utf8");
  console.log("Patched enterWorkspace — folder switch dialog removed.");
  console.log("Quit and restart VS Code to activate.");
}
console.log("");
console.log("Reload VS Code window to activate (Cmd+Shift+P → Reload Window).");
