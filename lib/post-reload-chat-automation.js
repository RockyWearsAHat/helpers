#!/usr/bin/env node
"use strict";

/**
 * post-reload-chat-automation.js
 *
 * Detached script. Reloads the VS Code window and/or types a continuation
 * keyword into the chat input. Designed for speed — NO element enumeration
 * (no "entire contents of front window"). All interactions are keystroke-based.
 *
 * Env vars:
 *   GSH_AUTOMATION_FORCE_RELOAD     1=reload window (default 1)
 *   GSH_AUTOMATION_SEND_CONTINUE    1=type+send continue text (default 0)
 *   GSH_AUTOMATION_CONTINUE_TEXT    text to type (default "reloaded, continue")
 *   GSH_AUTOMATION_DEBOUNCE_MS      ms to wait before starting (default 0)
 *   GSH_AUTOMATION_ROOT             workspace root path
 *   GSH_AUTOMATION_TRIGGER          debounce slot key (default "register")
 *   GSH_AUTOMATION_DISABLED         1=exit immediately
 */

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

function arg(name, fallback) {
  if (fallback === undefined) fallback = "";
  var idx = process.argv.indexOf(name);
  if (idx === -1 || idx + 1 >= process.argv.length) return fallback;
  return String(process.argv[idx + 1] || "").trim();
}

function sleep(ms) {
  return new Promise(function(resolve) { setTimeout(resolve, ms); });
}

function toInt(value, fallback) {
  var n = Number.parseInt(String(value || ""), 10);
  return Number.isFinite(n) ? n : fallback;
}

function claimDebounceSlot(root, trigger) {
  try {
    var dir = path.join(root || process.cwd(), ".gsh", "tools");
    fs.mkdirSync(dir, { recursive: true });
    var signalPath = path.join(dir, ".post-reload-signal-" + trigger + ".json");
    var token = String(Date.now()) + "-" + String(process.pid) + "-" + Math.random().toString(36).slice(2);
    fs.writeFileSync(signalPath, JSON.stringify({ token: token, ts: Date.now() }), "utf8");
    return { signalPath: signalPath, token: token };
  } catch (e) {
    return null;
  }
}

function stillOwnDebounceSlot(slot) {
  if (!slot || !slot.signalPath || !slot.token) return true;
  try {
    var raw = fs.readFileSync(slot.signalPath, "utf8");
    var parsed = JSON.parse(raw);
    return parsed && parsed.token === slot.token;
  } catch (e) {
    return false;
  }
}

function runAppleScript(lines) {
  var scriptLines = Array.isArray(lines) ? lines : [String(lines)];
  var args = [];
  for (var i = 0; i < scriptLines.length; i++) {
    args.push("-e", scriptLines[i]);
  }
  return spawnSync("osascript", args, { encoding: "utf8" });
}

function getVsCodeProcessName() {
  var r = runAppleScript([
    'if application "Code - Insiders" is running then return "Code - Insiders"',
    'if application "Code" is running then return "Code"',
    'return ""'
  ]);
  return (r.stdout || "").trim();
}

function hasVsCodeWindow(appName) {
  var r = runAppleScript([
    'tell application "System Events"',
    '  tell process ' + JSON.stringify(appName),
    '    return (count of windows) > 0',
    '  end tell',
    'end tell'
  ]);
  return (r.stdout || "").trim() === "true";
}

function waitForWindow(appName, timeoutMs) {
  var deadline = Date.now() + timeoutMs;
  function poll() {
    if (hasVsCodeWindow(appName)) return Promise.resolve(true);
    if (Date.now() >= deadline) return Promise.resolve(false);
    return sleep(300).then(function() { return poll(); });
  }
  return poll();
}

function sendEnter(appName) {
  runAppleScript([
    'tell application "System Events"',
    '  tell process ' + JSON.stringify(appName),
    '    key code 36',
    '  end tell',
    'end tell'
  ]);
}

function sendKeystroke(appName, text) {
  if (!text) return;
  runAppleScript([
    'tell application "System Events"',
    '  tell process ' + JSON.stringify(appName),
    '    keystroke ' + JSON.stringify(text),
    '  end tell',
    'end tell'
  ]);
}

function sendKeystrokeWithMod(appName, key, mods) {
  runAppleScript([
    'tell application "System Events"',
    '  tell process ' + JSON.stringify(appName),
    '    keystroke ' + JSON.stringify(key) + ' using {' + mods + '}',
    '  end tell',
    'end tell'
  ]);
}

async function main() {
  if (process.platform !== "darwin") return;
  if (process.env.GSH_AUTOMATION_DISABLED === "1") return;

  var root = arg("--root", process.env.GSH_AUTOMATION_ROOT || process.cwd());
  var trigger = arg("--event", process.env.GSH_AUTOMATION_TRIGGER || "register");
  var toolName = arg("--tool", process.env.GSH_AUTOMATION_TOOL || "");
  var shouldReload = (process.env.GSH_AUTOMATION_FORCE_RELOAD || "1") !== "0";
  var shouldSend = (process.env.GSH_AUTOMATION_SEND_CONTINUE || "0") === "1";
  var continueText = process.env.GSH_AUTOMATION_CONTINUE_TEXT || "reloaded, continue";
  var debounceMs = Math.max(0, toInt(process.env.GSH_AUTOMATION_DEBOUNCE_MS, 0));

  var slot = claimDebounceSlot(root, trigger);
  if (debounceMs > 0) {
    await sleep(debounceMs);
    if (!stillOwnDebounceSlot(slot)) return;
  }

  var appName = getVsCodeProcessName();
  if (!appName) return;

  runAppleScript(['tell application ' + JSON.stringify(appName) + ' to activate']);
  await sleep(200);

  if (shouldReload) {
    runAppleScript([
      'tell application "System Events"',
      '  tell process ' + JSON.stringify(appName),
      '    keystroke "p" using {command down, shift down}',
      '    delay 0.3',
      '    keystroke "Developer: Reload Window"',
      '    delay 0.2',
      '    key code 36',
      '  end tell',
      'end tell'
    ]);

    // Fire Enter 4 times immediately to dismiss any "chat request in progress" dialog.
    // The dialog's default button is always focused — Enter accepts it instantly.
    await sleep(250);
    for (var i = 0; i < 4; i++) {
      sendEnter(appName);
      await sleep(80);
    }

    await sleep(800);
    var ready = await waitForWindow(appName, 25000);
    if (!ready) return;

    await sleep(1200);
  }

  if (!shouldSend) return;

  runAppleScript(['tell application ' + JSON.stringify(appName) + ' to activate']);
  await sleep(200);

  // Open chat via keyboard shortcut — no visibility check needed.
  sendKeystrokeWithMod(appName, "i", "command down, control down");
  await sleep(700);

  var suffix = toolName ? " (" + trigger + ": " + toolName + ")" : "";
  sendKeystroke(appName, continueText + suffix);
  await sleep(100);
  sendEnter(appName);
}

main().catch(function() { process.exit(0); });
