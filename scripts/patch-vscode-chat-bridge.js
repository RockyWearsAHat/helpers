#!/usr/bin/env node
// patch-vscode-chat-bridge.js
//
// Patches VS Code's workbench bundle to expose the active chat session resource
// via a JSON file at ~/.cache/gsh/active-chat-session.json.
//
// Two code paths are patched:
//   1. setLastFocusedWidget — fires when a different editor widget gets focus
//      (e.g. switching from a file tab to the chat panel).  Writes the focused
//      widget's sessionResource URI.
//   2. onDidChangeViewModel — fires when the user switches conversations WITHIN
//      the chat panel (e.g. clicking a different session in the sessions list).
//      The widget stays the same but the viewModel changes.  Writes the new
//      session's resource URI, or null when navigating to the sessions list.
//
// Usage:
//   node patch-vscode-chat-bridge.js          # apply patch
//   node patch-vscode-chat-bridge.js --revert # restore original
//   node patch-vscode-chat-bridge.js --check  # check patch status

const fs = require("fs");
const path = require("path");

const BUNDLE = path.join(
  "/Applications/Visual Studio Code.app/Contents/Resources/app/out/vs/workbench",
  "workbench.desktop.main.js",
);
const BACKUP = BUNDLE + ".bak";

// File-write snippet (parameterised by the expression for the session URI):
// import("fs").then(f => { try { write({s, t}) } catch {} }).catch(() => {})
function makeWriteSnippet(uriExpr) {
  return `import("fs").then(function(f){try{var d=(process.env.HOME||"/tmp")+"/.cache/gsh",a=d+"/active-chat-session.json";f.mkdirSync(d,{recursive:!0});f.writeFileSync(a,JSON.stringify({s:${uriExpr},t:Date.now()}))}catch(x){}}).catch(function(){})`;
}

// --- Patch 1: setLastFocusedWidget ---
const OLD_1 =
  "setLastFocusedWidget(e){e!==this._lastFocusedWidget&&(this._lastFocusedWidget=e,this._onDidChangeFocusedWidget.fire(e),this._onDidChangeFocusedSession.fire())}";
const NEW_1 =
  `setLastFocusedWidget(e){e!==this._lastFocusedWidget&&(this._lastFocusedWidget=e,this._onDidChangeFocusedWidget.fire(e),this._onDidChangeFocusedSession.fire(),${makeWriteSnippet("e&&e.viewModel&&e.viewModel.sessionResource?e.viewModel.sessionResource.toString():null")})}`;

// --- Patch 2: onDidChangeViewModel ---
// Inside the viewModel change handler, the destructured args are:
//   {previousSessionResource:t, currentSessionResource:o}
// We add the file write after _onDidChangeFocusedSession.fire() using a
// comma expression so the short-circuit chain still works.
const OLD_2 =
  "this._lastFocusedWidget===e&&!Ye(t,o)&&this._onDidChangeFocusedSession.fire()";
const NEW_2 =
  `this._lastFocusedWidget===e&&!Ye(t,o)&&(this._onDidChangeFocusedSession.fire(),${makeWriteSnippet("o?o.toString():null")})`;

if (process.argv.includes("--check")) {
  if (!fs.existsSync(BUNDLE)) {
    console.error("Bundle not found at", BUNDLE);
    process.exit(1);
  }
  const src = fs.readFileSync(BUNDLE, "utf8");
  const has1 = src.includes(NEW_1);
  const has2 = src.includes(NEW_2);
  if (has1 && has2) {
    console.log("PATCHED — both code paths (widget focus + viewModel change).");
  } else if (has1) {
    console.log("PARTIAL — widget focus patched, viewModel change missing.");
  } else if (src.includes("active-chat-session.json")) {
    console.log("PARTIAL — old single-path patch detected. Re-run to upgrade.");
  } else if (src.includes(OLD_1)) {
    console.log("UNPATCHED — original bundle.");
  } else {
    console.log("UNKNOWN — injection points not found. VS Code version may have changed.");
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

// Read the bundle
let src = fs.readFileSync(BUNDLE, "utf8");

// If the old single-path patch is applied, revert to backup first
if (src.includes("active-chat-session.json") && !src.includes(NEW_2)) {
  if (fs.existsSync(BACKUP)) {
    console.log("Reverting old single-path patch before applying dual patch...");
    src = fs.readFileSync(BACKUP, "utf8");
  }
}

// Check if already fully patched
if (src.includes(NEW_1) && src.includes(NEW_2)) {
  console.log("Bundle already patched (both paths).");
  process.exit(0);
}

// Create backup if none exists
if (!fs.existsSync(BACKUP)) {
  fs.copyFileSync(BUNDLE, BACKUP);
  console.log("Backed up original bundle.");
}

// Apply Patch 1
let patched = src;
const idx1 = patched.indexOf(OLD_1);
if (idx1 === -1) {
  console.error("Could not find injection point 1 (setLastFocusedWidget).");
  console.error("VS Code version may have changed.");
  process.exit(1);
}
patched = patched.slice(0, idx1) + NEW_1 + patched.slice(idx1 + OLD_1.length);
console.log("Applied patch 1: setLastFocusedWidget (widget focus).");

// Apply Patch 2
const idx2 = patched.indexOf(OLD_2);
if (idx2 === -1) {
  console.error("Could not find injection point 2 (onDidChangeViewModel).");
  console.error("VS Code version may have changed.");
  process.exit(1);
}
patched = patched.slice(0, idx2) + NEW_2 + patched.slice(idx2 + OLD_2.length);
console.log("Applied patch 2: onDidChangeViewModel (conversation switch).");

fs.writeFileSync(BUNDLE, patched, "utf8");
console.log("Patched workbench bundle successfully.");
console.log("Reload VS Code window to activate (Cmd+Shift+P → Reload Window).");
