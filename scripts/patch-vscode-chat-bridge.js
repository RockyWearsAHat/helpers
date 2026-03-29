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
// Why this patch exists:
//   VS Code's extension API has no event for "focused chat session changed."
//   Tab events fire for panel-level focus but not for conversation switches
//   within the Chat panel. This is a gap — extensions that need session-level
//   focus tracking (like branch-per-chat workflows) have no API surface.
//   Upstream PR candidate: expose onDidChangeFocusedSession or equivalent.
//
// Usage:
//   node patch-vscode-chat-bridge.js          # apply patch
//   node patch-vscode-chat-bridge.js --check  # check patch status (exit 0=patched, 1=not)
//
// This script is meant to be called by patch-vscode-apply-all.js which handles
// backup, revert, and coordination with other patches.

const fs = require("fs");
const path = require("path");

const BUNDLE = path.join(
  "/Applications/Visual Studio Code.app/Contents/Resources/app/out/vs/workbench",
  "workbench.desktop.main.js",
);

// File-write snippet (parameterised by the expression for the session URI):
function makeWriteSnippet(uriExpr) {
  return `import("fs").then(function(f){try{var d=(process.env.HOME||"/tmp")+"/.cache/gsh",a=d+"/active-chat-session.json";f.mkdirSync(d,{recursive:!0});f.writeFileSync(a,JSON.stringify({s:${uriExpr},t:Date.now()}))}catch(x){}}).catch(function(){})`;
}

// --- Patch 1: setLastFocusedWidget ---
const OLD_1 =
  "setLastFocusedWidget(e){e!==this._lastFocusedWidget&&(this._lastFocusedWidget=e,this._onDidChangeFocusedWidget.fire(e),this._onDidChangeFocusedSession.fire())}";
const NEW_1 = `setLastFocusedWidget(e){e!==this._lastFocusedWidget&&(this._lastFocusedWidget=e,this._onDidChangeFocusedWidget.fire(e),this._onDidChangeFocusedSession.fire(),${makeWriteSnippet("e&&e.viewModel&&e.viewModel.sessionResource?e.viewModel.sessionResource.toString():null")})}`;

// --- Patch 2: onDidChangeViewModel ---
const OLD_2 =
  "this._lastFocusedWidget===e&&!Ye(t,o)&&this._onDidChangeFocusedSession.fire()";
const NEW_2 = `this._lastFocusedWidget===e&&!Ye(t,o)&&(this._onDidChangeFocusedSession.fire(),${makeWriteSnippet("o?o.toString():null")})`;

// Exported for use by the coordinator script
module.exports = { OLD_1, NEW_1, OLD_2, NEW_2, BUNDLE };

if (require.main === module) {
  if (!fs.existsSync(BUNDLE)) {
    console.error("Bundle not found at", BUNDLE);
    process.exit(1);
  }
  const src = fs.readFileSync(BUNDLE, "utf8");

  if (process.argv.includes("--check")) {
    const has1 = src.includes(NEW_1);
    const has2 = src.includes(NEW_2);
    if (has1 && has2) {
      console.log("PATCHED — both code paths (widget focus + viewModel change).");
      process.exit(0);
    } else if (has1 || has2) {
      console.log("PARTIAL — only one code path patched.");
      process.exit(1);
    } else {
      console.log("UNPATCHED");
      process.exit(1);
    }
  }

  // Apply mode: read current bundle and apply patches in-place
  let patched = src;
  let applied = 0;

  if (patched.includes(NEW_1)) {
    console.log("Patch 1 (setLastFocusedWidget): already applied.");
  } else {
    const idx = patched.indexOf(OLD_1);
    if (idx === -1) {
      console.error("Injection point 1 not found — VS Code version may have changed.");
      process.exit(1);
    }
    patched = patched.slice(0, idx) + NEW_1 + patched.slice(idx + OLD_1.length);
    console.log("Patch 1 (setLastFocusedWidget): applied.");
    applied++;
  }

  if (patched.includes(NEW_2)) {
    console.log("Patch 2 (onDidChangeViewModel): already applied.");
  } else {
    const idx = patched.indexOf(OLD_2);
    if (idx === -1) {
      console.error("Injection point 2 not found — VS Code version may have changed.");
      process.exit(1);
    }
    patched = patched.slice(0, idx) + NEW_2 + patched.slice(idx + OLD_2.length);
    console.log("Patch 2 (onDidChangeViewModel): applied.");
    applied++;
  }

  if (applied > 0) {
    fs.writeFileSync(BUNDLE, patched, "utf8");
    console.log("Bundle updated. Quit and restart VS Code to activate.");
  } else {
    console.log("Nothing to apply.");
  }
}
