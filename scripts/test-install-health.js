#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs");
const Module = require("module");
const os = require("os");
const path = require("path");

async function main() {
  const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "gsh-install-health-"));
  const originalLoad = Module._load;

  let branchSessionsEnabled = false;
  let nextWarningChoice;
  const warningCalls = [];
  const infoMessages = [];
  const terminalCommands = [];
  const openedUrls = [];

  const fakeVscode = {
    workspace: {
      workspaceFolders: [],
      getConfiguration: () => ({
        get: (key, defaultValue) => {
          if (key === "enabled") return branchSessionsEnabled;
          return defaultValue;
        },
      }),
    },
    window: {
      showWarningMessage: async (message, options, ...actions) => {
        warningCalls.push({ message, options, actions });
        return nextWarningChoice;
      },
      showInformationMessage: async (message) => {
        infoMessages.push(message);
      },
      showErrorMessage: async (message) => {
        throw new Error(message);
      },
      createTerminal: () => ({
        show() {},
        sendText(text) {
          terminalCommands.push(text);
        },
      }),
    },
    env: {
      openExternal: async (uri) => {
        openedUrls.push(uri.toString());
      },
    },
    Uri: {
      parse: (value) => ({
        toString() {
          return value;
        },
      }),
    },
  };

  Module._load = function patchedLoad(request, parent, isMain) {
    if (request === "vscode") return fakeVscode;
    return originalLoad.call(this, request, parent, isMain);
  };

  try {
    const createInstallHealth = require("../vscode-extension/src/install-health");

    const installRootOne = path.join(tmpRoot, "install-one");
    const scriptsOne = path.join(installRootOne, "scripts");
    fs.mkdirSync(scriptsOne, { recursive: true });
    fs.writeFileSync(path.join(installRootOne, "git-shell-helpers-mcp"), "", "utf8");
    for (const helper of [
      "patch-vscode-apply-all.js",
      "patch-vscode-folder-switch.js",
      "patch-vscode-git-head-display.js",
      "patch-vscode-runsubagent-model.js",
    ]) {
      fs.writeFileSync(path.join(scriptsOne, helper), "", "utf8");
    }

    const healthOne = createInstallHealth({
      _context: {},
      findGitShellHelpersMcpPath: () =>
        path.join(installRootOne, "git-shell-helpers-mcp"),
      execFileSync: () => JSON.stringify({ patches: [], allPatched: true }),
    });

    const statusOne = healthOne.collectHealthStatus();
    assert.strictEqual(statusOne.hasLocalInstall, true);
    assert.strictEqual(statusOne.shouldShowPopup, true);
    assert.match(statusOne.detail, /git-research-mcp/);
    assert.strictEqual(statusOne.canApplyPatches, false);

    const installRootTwo = path.join(tmpRoot, "install-two");
    const scriptsTwo = path.join(installRootTwo, "scripts");
    fs.mkdirSync(scriptsTwo, { recursive: true });
    fs.writeFileSync(path.join(installRootTwo, "git-shell-helpers-mcp"), "", "utf8");
    fs.writeFileSync(path.join(installRootTwo, "git-research-mcp"), "", "utf8");
    fs.writeFileSync(path.join(installRootTwo, "install-git-shell-helpers"), "", "utf8");
    for (const helper of [
      "patch-vscode-apply-all.js",
      "patch-vscode-folder-switch.js",
      "patch-vscode-git-head-display.js",
      "patch-vscode-runsubagent-model.js",
    ]) {
      fs.writeFileSync(path.join(scriptsTwo, helper), "", "utf8");
    }

    const execCalls = [];
    const healthTwo = createInstallHealth({
      _context: {},
      findGitShellHelpersMcpPath: () =>
        path.join(installRootTwo, "git-shell-helpers-mcp"),
      execFileSync: (command, args) => {
        execCalls.push([command, args]);
        if (args[1] === "--json") {
          return JSON.stringify({
            allPatched: false,
            patches: [
              { name: "folder-switch", status: "unpatched" },
              { name: "git-head-display", status: "patched" },
            ],
          });
        }
        return "ok";
      },
    });

    branchSessionsEnabled = true;
    nextWarningChoice = "Apply Patches";
    await healthTwo.maybeShowStartupPopup();

    assert.strictEqual(warningCalls.length, 1);
    assert.strictEqual(warningCalls[0].options.modal, true);
    assert.ok(warningCalls[0].actions.includes("Apply Patches"));
    assert.ok(
      execCalls.some(([, args]) => args[0].endsWith("patch-vscode-apply-all.js") && args[1] === "--json"),
    );
    assert.ok(
      execCalls.some(([, args]) => args[0].endsWith("patch-vscode-apply-all.js") && args.length === 1),
    );
    assert.ok(
      execCalls.some(([, args]) => args[0].endsWith("patch-vscode-apply-all.js") && args[1] === "--check"),
    );
    assert.strictEqual(infoMessages.length, 1);
    assert.strictEqual(terminalCommands.length, 0);
    assert.strictEqual(openedUrls.length, 0);
  } finally {
    Module._load = originalLoad;
  }

  console.log("install-health tests passed");
}

main().catch((error) => {
  console.error(error && error.stack ? error.stack : String(error));
  process.exit(1);
});