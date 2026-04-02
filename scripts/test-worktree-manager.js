#!/usr/bin/env node
"use strict";

const assert = require("assert");
const childProcess = require("child_process");
const fs = require("fs");
const Module = require("module");
const os = require("os");
const path = require("path");

async function main() {
  const tmpRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "gsh-worktree-manager-"),
  );
  const repoRoot = path.join(tmpRoot, "repo");
  const worktreePath = path.join(tmpRoot, "feature-test");

  fs.mkdirSync(path.join(repoRoot, ".git"), { recursive: true });
  fs.mkdirSync(worktreePath, { recursive: true });

  const branchByCwd = new Map([
    [repoRoot, "dev"],
    [worktreePath, "feature/test"],
  ]);
  const diagnostics = [];
  const globalState = new Map();
  let treeProvider = null;
  let treeView = null;
  let statusRefreshes = 0;

  class EventEmitter {
    constructor() {
      this._listeners = [];
      this.event = (listener) => {
        this._listeners.push(listener);
        return {
          dispose: () => {
            this._listeners = this._listeners.filter((cb) => cb !== listener);
          },
        };
      };
    }

    fire(value) {
      for (const listener of this._listeners) {
        listener(value);
      }
    }
  }

  class TreeItem {
    constructor(labelOrUri, collapsibleState) {
      if (typeof labelOrUri === "string") {
        this.label = labelOrUri;
      } else {
        this.resourceUri = labelOrUri;
        this.label = path.basename(labelOrUri.fsPath);
      }
      this.collapsibleState = collapsibleState;
    }
  }

  const knownSession = { toString: () => "chat://session-one" };
  const unknownSession = { toString: () => "chat://session-unknown" };
  const fakeVscode = {
    workspace: {
      workspaceFolders: [{ uri: { fsPath: repoRoot }, index: 0 }],
      getConfiguration: () => ({
        get: (key, defaultValue) => {
          if (key === "enabled") return true;
          if (key === "orphanSafetyCommit") return false;
          return defaultValue;
        },
      }),
      updateWorkspaceFolders: () => true,
    },
    window: {
      activeChatPanelSessionResource: knownSession,
      tabGroups: {
        activeTabGroup: {
          activeTab: {
            input: {
              viewType: "workbench.editor.chatSession",
              uri: knownSession,
            },
          },
        },
      },
      createTreeView: (_id, options) => {
        treeProvider = options.treeDataProvider;
        treeView = {
          title: "",
          dispose() {},
        };
        return treeView;
      },
      showInformationMessage: async () => undefined,
      showWarningMessage: async () => undefined,
    },
    extensions: {
      getExtension: (id) => {
        if (id !== "vscode.git") return null;
        return {
          isActive: true,
          exports: {
            getAPI() {
              return {
                repositories: [
                  {
                    status() {
                      statusRefreshes += 1;
                    },
                  },
                ],
              };
            },
          },
        };
      },
    },
    Uri: {
      file: (filePath) => ({
        fsPath: filePath,
        toString() {
          return filePath;
        },
      }),
    },
    TreeItem,
    TreeItemCollapsibleState: {
      None: 0,
      Collapsed: 1,
    },
    EventEmitter,
  };

  const originalLoad = Module._load;
  const originalExecFileSync = childProcess.execFileSync;
  const originalHomedir = os.homedir;
  const originalGrace = process.env.GSH_WORKTREE_UNKNOWN_SESSION_GRACE_MS;

  process.env.GSH_WORKTREE_UNKNOWN_SESSION_GRACE_MS = "20";
  os.homedir = () => tmpRoot;
  Module._load = function patchedLoad(request, parent, isMain) {
    if (request === "vscode") return fakeVscode;
    return originalLoad.call(this, request, parent, isMain);
  };

  childProcess.execFileSync = (command, args, options = {}) => {
    assert.strictEqual(command, "git");
    const cwd = options.cwd;
    const [verb, ...rest] = args;

    if (verb === "symbolic-ref" && rest[0] === "--short") {
      return Buffer.from(branchByCwd.get(cwd) || "", "utf8");
    }

    if (verb === "symbolic-ref" && rest[0] === "HEAD") {
      branchByCwd.set(cwd, rest[1].replace("refs/heads/", ""));
      return Buffer.from("", "utf8");
    }

    if (verb === "reset" && rest[0] === "--hard") {
      return Buffer.from("", "utf8");
    }

    if (verb === "status") {
      return Buffer.from("", "utf8");
    }

    if (verb === "stash") {
      return Buffer.from("", "utf8");
    }

    throw new Error(`Unexpected git command: ${args.join(" ")}`);
  };

  try {
    const createWorktreeManager = require("../vscode-extension/src/worktree-manager");
    const manager = createWorktreeManager({
      _context: {
        globalState: {
          get: (key) => globalState.get(key),
          update: (key, value) => {
            if (typeof value === "undefined") {
              globalState.delete(key);
            } else {
              globalState.set(key, value);
            }
            return Promise.resolve();
          },
        },
      },
      getDiagnosticsOutputChannel: () => ({
        appendLine: (line) => diagnostics.push(line),
      }),
    });

    manager.registerWorktreeFileView({ subscriptions: [] });
    manager.handleWorktreeIpcMessage({
      type: "worktreeCreated",
      branch: "feature/test",
      worktreePath,
      baseBranch: "dev",
      baseCommit: "abc123",
    });

    assert.strictEqual(
      branchByCwd.get(repoRoot),
      "feature/test",
      "worktree creation should focus the main repo on the feature branch",
    );
    assert.ok(statusRefreshes >= 1, "focus should refresh git status");
    assert.strictEqual(treeView.title, "\uD83C\uDF3F feature/test");

    fakeVscode.window.activeChatPanelSessionResource = unknownSession;
    fakeVscode.window.tabGroups.activeTabGroup.activeTab.input.uri =
      unknownSession;
    manager.onChatSessionFocusChanged(unknownSession);

    assert.strictEqual(
      branchByCwd.get(repoRoot),
      "feature/test",
      "unknown session should not immediately drop the workspace back to baseline",
    );

    await new Promise((resolve) => setTimeout(resolve, 5));
    fakeVscode.window.activeChatPanelSessionResource = knownSession;
    fakeVscode.window.tabGroups.activeTabGroup.activeTab.input.uri = knownSession;
    manager.onChatSessionFocusChanged(knownSession);

    await new Promise((resolve) => setTimeout(resolve, 35));
    assert.strictEqual(
      branchByCwd.get(repoRoot),
      "feature/test",
      "returning to the bound chat before the grace window ends should preserve focus",
    );

    fakeVscode.window.activeChatPanelSessionResource = unknownSession;
    fakeVscode.window.tabGroups.activeTabGroup.activeTab.input.uri =
      unknownSession;
    manager.onChatSessionFocusChanged(unknownSession);

    await new Promise((resolve) => setTimeout(resolve, 35));
    assert.strictEqual(
      branchByCwd.get(repoRoot),
      "dev",
      "persistent unknown sessions should eventually restore the baseline branch",
    );

    const parkedItems = treeProvider.getChildren();
    assert.ok(
      parkedItems.some(
        (item) => item.label === "No branch session focused in this chat",
      ),
      "parked sessions summary should be visible when no branch is focused",
    );
    assert.ok(
      parkedItems.some((item) => item.label === "feature/test"),
      "parked branch sessions should remain visible in the Branch Files view",
    );
    assert.strictEqual(treeView.title, "Branch Files (1 parked)");
    assert.ok(
      diagnostics.some((line) => line.includes("Focused:")),
      "focus diagnostics should be emitted",
    );
    assert.ok(
      diagnostics.some((line) => line.includes("Unfocused:")),
      "unfocus diagnostics should be emitted",
    );

    console.log("ok");
  } finally {
    childProcess.execFileSync = originalExecFileSync;
    Module._load = originalLoad;
    os.homedir = originalHomedir;
    if (typeof originalGrace === "undefined") {
      delete process.env.GSH_WORKTREE_UNKNOWN_SESSION_GRACE_MS;
    } else {
      process.env.GSH_WORKTREE_UNKNOWN_SESSION_GRACE_MS = originalGrace;
    }
    fs.rmSync(tmpRoot, { recursive: true, force: true });
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});