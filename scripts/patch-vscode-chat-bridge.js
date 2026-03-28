#!/usr/bin/env node
// patch-vscode-chat-bridge.js
//
// Patches VS Code's workbench bundle to expose the active chat session resource
// via a JSON file at ~/.cache/gsh/active-chat-session.json.
//
// This bridges the gap between VS Code's internal IChatWidgetService (which has
// onDidChangeFocusedSession) and extensions that need to know which chat session
// the user is currently viewing.
//
// Usage:
//   node patch-vscode-chat-bridge.js          # apply patch
//   node patch-vscode-chat-bridge.js --revert # restore original

const fs = require("fs");
const path = require("path");

const BUNDLE = path.join(
  "/Applications/Visual Studio Code.app/Contents/Resources/app/out/vs/workbench",
  "workbench.desktop.main.js",
);
const BACKUP = BUNDLE + ".bak";

const OLD =
  "setLastFocusedWidget(e){e!==this._lastFocusedWidget&&(this._lastFocusedWidget=e,this._onDidChangeFocusedWidget.fire(e),this._onDidChangeFocusedSession.fire())}";

// The patch adds an import("fs")-based side-effect after the event fires,
// writing {s: sessionResourceUri, t: timestamp} to a known JSON file.
const NEW =
  'setLastFocusedWidget(e){e!==this._lastFocusedWidget&&(this._lastFocusedWidget=e,this._onDidChangeFocusedWidget.fire(e),this._onDidChangeFocusedSession.fire(),import("fs").then(function(f){try{var d=(process.env.HOME||"/tmp")+"/.cache/gsh",a=d+"/active-chat-session.json";f.mkdirSync(d,{recursive:!0});f.writeFileSync(a,JSON.stringify({s:e&&e.viewModel&&e.viewModel.sessionResource?e.viewModel.sessionResource.toString():null,t:Date.now()}))}catch(x){}}).catch(function(){}))}';

if (process.argv.includes("--revert")) {
  if (!fs.existsSync(BACKUP)) {
    console.error("No backup found at", BACKUP);
    process.exit(1);
  }
  fs.copyFileSync(BACKUP, BUNDLE);
  console.log("Reverted to original bundle.");
  process.exit(0);
}

// Read the bundle
const src = fs.readFileSync(BUNDLE, "utf8");

// Check if already patched
if (src.includes("active-chat-session.json")) {
  console.log("Bundle already patched.");
  process.exit(0);
}

// Verify the injection point exists
const idx = src.indexOf(OLD);
if (idx === -1) {
  console.error("Could not find injection point in bundle.");
  console.error("VS Code version may have changed — check setLastFocusedWidget signature.");
  process.exit(1);
}

// Apply the patch
const patched = src.slice(0, idx) + NEW + src.slice(idx + OLD.length);
fs.writeFileSync(BUNDLE, patched, "utf8");

console.log("Patched workbench bundle successfully.");
console.log("Reload VS Code window to activate (Cmd+Shift+P → Reload Window).");
