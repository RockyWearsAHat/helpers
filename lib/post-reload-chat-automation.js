#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

function arg(name, fallback = "") {
  const idx = process.argv.indexOf(name);
  if (idx === -1 || idx + 1 >= process.argv.length) return fallback;
  return String(process.argv[idx + 1] || "").trim();
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function waitSync(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function toInt(value, fallback) {
  const n = Number.parseInt(String(value || ""), 10);
  return Number.isFinite(n) ? n : fallback;
}

function claimDebounceSlot(root, trigger) {
  try {
    const dir = path.join(root || process.cwd(), ".gsh", "tools");
    fs.mkdirSync(dir, { recursive: true });
    const signalPath = path.join(dir, ".post-reload-signal-" + trigger + ".json");
    const token = String(Date.now()) + "-" + String(process.pid) + "-" + Math.random().toString(36).slice(2);
    fs.writeFileSync(signalPath, JSON.stringify({ token, ts: Date.now() }), "utf8");
    return { signalPath, token };
  } catch {
    return null;
  }
}

function stillOwnDebounceSlot(slot) {
  if (!slot || !slot.signalPath || !slot.token) return true;
  try {
    const raw = fs.readFileSync(slot.signalPath, "utf8");
    const parsed = JSON.parse(raw);
    return parsed && parsed.token === slot.token;
  } catch {
    return false;
  }
}

function runAppleScript(lines) {
  const scriptLines = Array.isArray(lines) ? lines : [String(lines)];
  const args = [];
  for (const line of scriptLines) args.push("-e", line);
  return spawnSync("osascript", args, { encoding: "utf8" });
}

function hasVsCodeWindow() {
  const r = runAppleScript([
    'set foundWindow to false',
    'tell application "System Events"',
    '  repeat with appName in {"Code - Insiders", "Code"}',
    '    if exists process appName then',
    '      tell process appName',
    '        if (count of windows) > 0 then set foundWindow to true',
    '      end tell',
    '    end if',
    '  end repeat',
    'end tell',
    'if foundWindow then return "READY"',
    'return "WAIT"',
  ]);
  return (r.stdout || "").trim() === "READY";
}

async function waitForVsCodeWindow(timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (hasVsCodeWindow()) return true;
    await sleep(250);
  }
  return false;
}

function activateVsCode() {
  runAppleScript([
    'if application "Code - Insiders" is running then',
    '  tell application "Code - Insiders" to activate',
    'else if application "Code" is running then',
    '  tell application "Code" to activate',
    'end if',
  ]);
}

function commandPalette(commandText) {
  runAppleScript([
    'tell application "System Events"',
    '  set appName to ""',
    '  if exists process "Code - Insiders" then',
    '    set appName to "Code - Insiders"',
    '  else if exists process "Code" then',
    '    set appName to "Code"',
    '  end if',
    '  if appName is "" then return',
    '  tell process appName',
    '    keystroke "p" using {command down, shift down}',
    '    delay 0.2',
    `    keystroke ${JSON.stringify(commandText)}`,
    '    delay 0.15',
    '    key code 36',
    '  end tell',
    'end tell',
  ]);
}

function openChatShortcut() {
  runAppleScript([
    'tell application "System Events"',
    '  set appName to ""',
    '  if exists process "Code - Insiders" then',
    '    set appName to "Code - Insiders"',
    '  else if exists process "Code" then',
    '    set appName to "Code"',
    '  end if',
    '  if appName is "" then return',
    '  tell process appName',
    '    keystroke "i" using {command down, control down}',
    '  end tell',
    'end tell',
  ]);
}

async function forceAcceptLoop(timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    runAppleScript([
      'tell application "System Events"',
      '  set appName to ""',
      '  if exists process "Code - Insiders" then',
      '    set appName to "Code - Insiders"',
      '  else if exists process "Code" then',
      '    set appName to "Code"',
      '  end if',
      '  if appName is "" then return',
      '  tell process appName',
      '    set frontmost to true',
      '    key code 36',
      '  end tell',
      'end tell',
    ]);
    await sleep(150);
  }
}

function isChatVisible() {
  const r = runAppleScript([
    'tell application "System Events"',
    '  set appName to ""',
    '  if exists process "Code - Insiders" then',
    '    set appName to "Code - Insiders"',
    '  else if exists process "Code" then',
    '    set appName to "Code"',
    '  end if',
    '  if appName is "" then return "NO"',
    '  tell process appName',
    '    if (count of windows) = 0 then return "NO"',
    '    set elems to entire contents of front window',
    '    repeat with el in elems',
    '      try',
    '        set r to role of el as text',
    '        if r is "AXStaticText" or r is "AXButton" then',
    '          set n to (name of el as text)',
    '          set nn to do shell script "echo " & quoted form of n & " | tr \"[:upper:]\" \"[:lower:]\""',
    '          if nn is "chat" then return "YES"',
    '          if nn contains "chat" and nn contains "github shell helpers" then return "YES"',
    '        end if',
    '      end try',
    '    end repeat',
    '  end tell',
    'end tell',
    'return "NO"',
  ]);
  return (r.stdout || "").trim() === "YES";
}

function ensureChatOpen() {
  if (!isChatVisible()) {
    openChatShortcut();
  }
}

function confirmInProgressPromptIfShown(timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const r = runAppleScript([
      'tell application "System Events"',
      '  set appName to ""',
      '  if exists process "Code - Insiders" then',
      '    set appName to "Code - Insiders"',
      '  else if exists process "Code" then',
      '    set appName to "Code"',
      '  end if',
      '  if appName is "" then return "NO_APP"',
      '  tell process appName',
      '    if (count of windows) = 0 then return "NO_WINDOW"',
      '    set targets to {}',
      '    repeat with w in windows',
      '      set isTarget to false',
      '      try',
      '        set wt to value of attribute "AXTitle" of w as text',
      '        if wt contains "in progress" then set isTarget to true',
      '      end try',
      '      if isTarget is false then',
      '        try',
      '          set txt to (value of static texts of w) as text',
      '          if txt contains "in progress" then set isTarget to true',
      '        end try',
      '      end if',
      '      if isTarget then set end of targets to w',
      '      try',
      '        repeat with s in sheets of w',
      '          set isSheetTarget to false',
      '          try',
      '            set st to value of attribute "AXTitle" of s as text',
      '            if st contains "in progress" then set isSheetTarget to true',
      '          end try',
      '          if isSheetTarget is false then',
      '            try',
      '              set stxt to (value of static texts of s) as text',
      '              if stxt contains "in progress" then set isSheetTarget to true',
      '            end try',
      '          end if',
      '          if isSheetTarget then set end of targets to s',
      '        end repeat',
      '      end try',
      '    end repeat',
      '    if (count of targets) = 0 then return "NOT_SHOWN"',
      '    repeat with targetWin in targets',
      '      repeat with bn in {"Yes", "Reload", "Continue", "Open Chat", "Stop and Continue", "Stop"}',
      '        try',
      '          click button (bn as text) of targetWin',
      '          return "CLICKED"',
      '        end try',
      '      end repeat',
      '      try',
      '        set db to value of attribute "AXDefaultButton" of targetWin',
      '        perform action "AXPress" of db',
      '        return "CLICKED"',
      '      end try',
      '      try',
      '        repeat with b in buttons of targetWin',
      '          try',
      '            set bn to (name of b as text)',
      '            if bn is not "Cancel" then',
      '              click b',
      '              return "CLICKED"',
      '            end if',
      '          end try',
      '        end repeat',
      '      end try',
      '      try',
      '        click button 2 of targetWin',
      '        return "CLICKED"',
      '      end try',
      '    end repeat',
      '    try',
      '      set frontmost to true',
      '      key code 36',
      '      return "ENTER_SENT"',
      '    end try',
      '    return "BLOCKED"',
      '  end tell',
      'end tell',
    ]);

    const status = (r.stdout || "").trim();
    if (status === "CLICKED") return true;
    if (status === "ENTER_SENT") return true;
    if (status === "BLOCKED") return false;

    if (Date.now() < deadline) waitSync(250);
  }
  return false;
}

function clickRelativeToChatAndPlus() {
  runAppleScript([
    'tell application "System Events"',
    '  set appName to ""',
    '  if exists process "Code - Insiders" then',
    '    set appName to "Code - Insiders"',
    '  else if exists process "Code" then',
    '    set appName to "Code"',
    '  end if',
    '  if appName is "" then return',
    '  tell process appName',
    '    if (count of windows) = 0 then return',
    '    set plusPos to missing value',
    '    set chatPos to missing value',
    '    set elems to entire contents of front window',
    '    repeat with el in elems',
    '      try',
    '        set r to role of el',
    '        if plusPos is missing value and r is "AXButton" then',
    '          if (name of el as text) is "+" then set plusPos to position of el',
    '        end if',
    '        if chatPos is missing value and r is "AXStaticText" then',
    '          if (name of el as text) is "CHAT" then set chatPos to position of el',
    '        end if',
    '      end try',
    '      if plusPos is not missing value and chatPos is not missing value then exit repeat',
    '    end repeat',
    '    if plusPos is missing value or chatPos is missing value then return',
    '    set px to item 1 of plusPos',
    '    set py to item 2 of plusPos',
    '    set cy to item 2 of chatPos',
    '    set targetY to py - ((py - cy) * 0.05)',
    '    click at {px, targetY}',
    '  end tell',
    'end tell',
  ]);
}

function focusChatInput() {
  runAppleScript([
    'tell application "System Events"',
    '  set appName to ""',
    '  if exists process "Code - Insiders" then',
    '    set appName to "Code - Insiders"',
    '  else if exists process "Code" then',
    '    set appName to "Code"',
    '  end if',
    '  if appName is "" then return',
    '  tell process appName',
    '    if (count of windows) = 0 then return',
    '    set bestPos to missing value',
    '    set bestSize to missing value',
    '    set bestY to -1',
    '    set elems to entire contents of front window',
    '    repeat with el in elems',
    '      try',
    '        set r to role of el',
    '        if r is "AXTextArea" or r is "AXTextField" then',
    '          set p to position of el',
    '          set s to size of el',
    '          if (item 2 of p) > bestY and (item 1 of s) > 120 then',
    '            set bestY to item 2 of p',
    '            set bestPos to p',
    '            set bestSize to s',
    '          end if',
    '        end if',
    '      end try',
    '    end repeat',
    '    if bestPos is missing value then return',
    '    set x to (item 1 of bestPos) + 20',
    '    set y to (item 2 of bestPos) + ((item 2 of bestSize) / 2)',
    '    click at {x, y}',
    '  end tell',
    'end tell',
  ]);
}

function typeAndSend(message) {
  if (!message) return;
  runAppleScript([
    'tell application "System Events"',
    '  set appName to ""',
    '  if exists process "Code - Insiders" then',
    '    set appName to "Code - Insiders"',
    '  else if exists process "Code" then',
    '    set appName to "Code"',
    '  end if',
    '  if appName is "" then return',
    '  tell process appName',
    `    keystroke ${JSON.stringify(message)}`,
    '    delay 0.1',
    '    key code 36',
    '  end tell',
    'end tell',
  ]);
}

async function main() {
  if (process.platform !== "darwin") return;

  const root = arg("--root", process.env.GSH_AUTOMATION_ROOT || process.cwd());
  const trigger = arg("--event", process.env.GSH_AUTOMATION_TRIGGER || "register");
  const toolName = arg("--tool", process.env.GSH_AUTOMATION_TOOL || "");
  const shouldReload = (process.env.GSH_AUTOMATION_FORCE_RELOAD || "1") !== "0";
  const shouldSend = (process.env.GSH_AUTOMATION_SEND_CONTINUE || "0") === "1";
  const continueText = process.env.GSH_AUTOMATION_CONTINUE_TEXT || "reloaded, continue";
  const debounceMs = Math.max(0, toInt(process.env.GSH_AUTOMATION_DEBOUNCE_MS, 0));

  const slot = claimDebounceSlot(root, trigger);
  if (debounceMs > 0) {
    await sleep(debounceMs);
    if (!stillOwnDebounceSlot(slot)) return;
  }

  await sleep(500);
  activateVsCode();

  if (shouldReload) {
    commandPalette("Developer: Reload Window");
    await forceAcceptLoop(3000);
    confirmInProgressPromptIfShown(20000);
  }

  let ready = false;
  const waitDeadline = Date.now() + 30000;
  while (Date.now() < waitDeadline) {
    confirmInProgressPromptIfShown(250);
    if (await waitForVsCodeWindow(500)) {
      ready = true;
      break;
    }
  }
  if (!ready) return;

  await sleep(900);
  activateVsCode();
  ensureChatOpen();
  confirmInProgressPromptIfShown(5000);
  await sleep(500);
  clickRelativeToChatAndPlus();
  await sleep(200);
  focusChatInput();

  if (shouldSend) {
    await sleep(200);
    const suffix = toolName ? " (" + trigger + ": " + toolName + ")" : "";
    typeAndSend(continueText + suffix);
  }
}

main().catch(() => {
  process.exit(0);
});
