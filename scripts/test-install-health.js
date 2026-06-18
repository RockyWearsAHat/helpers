#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs");
const Module = require("module");
const os = require("os");
const path = require("path");

async function main() {
  const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "helpers-install-health-"));
  const originalLoad = Module._load;

  let nextWarningChoice;
  const warningCalls = [];
  const terminalCommands = [];
  const openedUrls = [];

  const fakeVscode = {
    workspace: {
      workspaceFolders: [],
      getConfiguration: () => ({
        get: (key, defaultValue) => defaultValue,
      }),
    },
    window: {
      showWarningMessage: async (message, options, ...actions) => {
        warningCalls.push({ message, options, actions });
        return nextWarningChoice;
      },
      showInformationMessage: async () => {},
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

    // Install one: missing git-research-mcp and no installer present → the popup
    // should flag the incomplete feature bundle and offer the setup guide.
    const installRootOne = path.join(tmpRoot, "install-one");
    fs.mkdirSync(installRootOne, { recursive: true });
    fs.writeFileSync(path.join(installRootOne, "helpers-server"), "", "utf8");

    const healthOne = createInstallHealth({
      _context: {},
      findGitShellHelpersMcpPath: () =>
        path.join(installRootOne, "helpers-server"),
    });

    const statusOne = healthOne.collectHealthStatus();
    assert.strictEqual(statusOne.hasLocalInstall, true);
    assert.strictEqual(statusOne.shouldShowPopup, true);
    assert.match(statusOne.detail, /git-research-mcp/);
    assert.strictEqual(statusOne.canRunInstaller, false);

    nextWarningChoice = "Open Setup Guide";
    await healthOne.maybeShowStartupPopup();
    assert.strictEqual(warningCalls.length, 1);
    assert.strictEqual(warningCalls[0].options.modal, true);
    assert.ok(warningCalls[0].actions.includes("Open Setup Guide"));
    assert.strictEqual(openedUrls.length, 1);

    // Install two: complete feature bundle + installer present → no popup.
    const installRootTwo = path.join(tmpRoot, "install-two");
    fs.mkdirSync(installRootTwo, { recursive: true });
    fs.writeFileSync(path.join(installRootTwo, "helpers-server"), "", "utf8");
    fs.writeFileSync(path.join(installRootTwo, "git-research-mcp"), "", "utf8");
    fs.writeFileSync(
      path.join(installRootTwo, "install-helpers"),
      "",
      "utf8",
    );

    const healthTwo = createInstallHealth({
      _context: {},
      findGitShellHelpersMcpPath: () =>
        path.join(installRootTwo, "helpers-server"),
    });

    const statusTwo = healthTwo.collectHealthStatus();
    assert.strictEqual(statusTwo.shouldShowPopup, false);
    assert.strictEqual(statusTwo.canRunInstaller, true);
    const shown = await healthTwo.maybeShowStartupPopup();
    assert.strictEqual(shown, false);
    assert.strictEqual(terminalCommands.length, 0);
  } finally {
    Module._load = originalLoad;
  }

  console.log("install-health tests passed");
}

main().catch((error) => {
  console.error(error && error.stack ? error.stack : String(error));
  process.exit(1);
});
