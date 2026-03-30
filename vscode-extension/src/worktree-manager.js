"use strict";
// src/worktree-manager.js — Worktree ↔ Chat binding, focus, head override, file browser
const vscode = require("vscode");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const WORKTREE_BINDINGS_KEY = "worktreeBindings.v2";
const TAB_WORKTREE_KEY = "tabToWorktree";

module.exports = function createWorktreeManager(deps) {
  const { _context, getDiagnosticsOutputChannel } = deps;

  let _worktreeBindings = new Map();
  let _tabToWorktree = new Map();
  let _pendingBranchSessionStarts = new Map();
  let _activeWorktreeFolder = null;
  let _suppressTabDrivenUnfocusUntil = 0;
  let _displayedBranch = null;
  let _worktreeFileProvider = null;
  let _gitRefreshTimer = null;

  // ------- Persistence -------

  function loadWorktreeBindings() {
    try {
      const raw = _context?.globalState?.get(WORKTREE_BINDINGS_KEY);
      if (Array.isArray(raw)) {
        _worktreeBindings = new Map(raw);
      }
    } catch {}
  }

  function saveWorktreeBindings() {
    try {
      _context?.globalState?.update(
        WORKTREE_BINDINGS_KEY,
        Array.from(_worktreeBindings.entries()),
      );
    } catch {}
  }

  function loadTabWorktreeMap() {
    try {
      const raw = _context?.globalState?.get(TAB_WORKTREE_KEY);
      if (Array.isArray(raw)) {
        _tabToWorktree = new Map(raw);
      }
    } catch {}
  }

  function saveTabWorktreeMap() {
    try {
      const entries = Array.from(_tabToWorktree.entries());
      _context?.globalState?.update(TAB_WORKTREE_KEY, entries.slice(-200));
    } catch {}
  }

  // ------- Utility -------

  function _getActiveChatTabKey() {
    try {
      const activeTab = vscode.window.tabGroups?.activeTabGroup?.activeTab;
      if (
        activeTab?.input?.viewType === "workbench.editor.chatSession" &&
        activeTab.input.uri
      ) {
        return activeTab.input.uri.toString();
      }
    } catch {}
    return null;
  }

  function _findRecentUnboundWorktree() {
    const now = Date.now();
    const boundPaths = new Set(_tabToWorktree.values());
    for (const [wtPath, binding] of _worktreeBindings) {
      if (boundPaths.has(wtPath)) continue;
      if (now - (binding.createdAt || 0) < 120000 && fs.existsSync(wtPath)) {
        return wtPath;
      }
    }
    return null;
  }

  function isPathWithinRoot(candidatePath, rootPath) {
    if (!candidatePath || !rootPath) return false;
    const relativePath = path.relative(rootPath, candidatePath);
    return (
      relativePath === "" ||
      (!relativePath.startsWith("..") && !path.isAbsolute(relativePath))
    );
  }

  function _prunePendingStarts() {
    const cutoff = Date.now() - 10 * 60 * 1000;
    for (const [id, entry] of _pendingBranchSessionStarts) {
      if ((entry?.capturedAt || 0) < cutoff) {
        _pendingBranchSessionStarts.delete(id);
      }
    }
  }

  function _getMainRepoPath() {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || null;
  }

  function _gitCurrentBranch(cwd) {
    try {
      return execFileSync("git", ["symbolic-ref", "--short", "HEAD"], {
        cwd,
        timeout: 5000,
        stdio: ["pipe", "pipe", "pipe"],
      })
        .toString()
        .trim();
    } catch {
      return null;
    }
  }

  // ------- Head Override -------

  function _writeHeadOverride(repoRoot, branchName) {
    try {
      const filePath = path.join(repoRoot, ".git", "gsh-head-override");
      fs.writeFileSync(filePath, branchName + "\n", "utf8");
      return true;
    } catch (err) {
      _writeWorktreeDebug(
        `writeHeadOverride failed: ${err.message?.split("\n")[0] || err}`,
      );
      return false;
    }
  }

  function _removeHeadOverride(repoRoot) {
    try {
      const filePath = path.join(repoRoot, ".git", "gsh-head-override");
      if (fs.existsSync(filePath)) {
        fs.unlinkSync(filePath);
        return true;
      }
    } catch (err) {
      _writeWorktreeDebug(
        `removeHeadOverride failed: ${err.message?.split("\n")[0] || err}`,
      );
    }
    return false;
  }

  function _triggerGitRefresh() {
    if (_gitRefreshTimer) clearTimeout(_gitRefreshTimer);
    _gitRefreshTimer = setTimeout(() => {
      _gitRefreshTimer = null;
      try {
        const gitExt = vscode.extensions.getExtension("vscode.git");
        if (!gitExt?.isActive) return;
        const api = gitExt.exports?.getAPI(1);
        const repo = api?.repositories?.[0];
        if (repo) repo.status();
      } catch {}
      // Force the File Explorer to re-scan so new/deleted files from
      // git reset --hard are visible immediately, not just after the
      // file system watcher eventually catches up.
      try {
        vscode.commands.executeCommand(
          "workbench.files.action.refreshFilesExplorer",
        );
      } catch {}
    }, 500);
  }

  // ------- Debug Logging -------

  function _writeWorktreeDebug(msg) {
    try {
      const debugPath = path.join(
        os.homedir(),
        ".cache",
        "gsh",
        "worktree-debug.log",
      );
      fs.appendFileSync(debugPath, `${new Date().toISOString()} ${msg}\n`);
    } catch {}
  }

  let _originalBranch = null; // baseline branch before session focus
  let _stashRef = null; // stash entry created on focus

  // ------- Main Repo Checkout (symbolic-ref) -------
  //
  // git symbolic-ref bypasses the worktree checkout restriction — it updates
  // HEAD directly without the safety check that prevents checking out a branch
  // already used by a worktree. Combined with git reset --hard, this gives us
  // a real checkout of the worktree's branch in the main repo, so the user
  // sees the same files the agent is working on.
  //
  // The worktree continues to work independently on the same branch — commits
  // in the worktree advance the shared branch ref. A subsequent reset --hard
  // in the main repo pulls in those commits instantly.

  function _checkoutBranchViaSymref(repoRoot, branch) {
    try {
      execFileSync("git", ["symbolic-ref", "HEAD", `refs/heads/${branch}`], {
        cwd: repoRoot,
        timeout: 5000,
        stdio: ["pipe", "pipe", "pipe"],
      });
      execFileSync("git", ["reset", "--hard", "HEAD"], {
        cwd: repoRoot,
        timeout: 10000,
        stdio: ["pipe", "pipe", "pipe"],
      });
      return true;
    } catch (err) {
      _writeWorktreeDebug(
        `symref checkout failed for ${branch}: ${err.message?.split("\n")[0] || err}`,
      );
      return false;
    }
  }

  function _stashMainRepo(repoRoot) {
    try {
      const status = execFileSync("git", ["status", "--porcelain"], {
        cwd: repoRoot,
        timeout: 5000,
        stdio: ["pipe", "pipe", "pipe"],
      })
        .toString()
        .trim();
      if (!status) return null;
      execFileSync(
        "git",
        ["stash", "push", "-m", "gsh-session-focus: auto-stash"],
        { cwd: repoRoot, timeout: 10000, stdio: ["pipe", "pipe", "pipe"] },
      );
      const ref = execFileSync("git", ["stash", "list", "-1", "--format=%H"], {
        cwd: repoRoot,
        timeout: 5000,
        stdio: ["pipe", "pipe", "pipe"],
      })
        .toString()
        .trim();
      _writeWorktreeDebug(`stashed main repo: ${ref}`);
      return ref;
    } catch (err) {
      _writeWorktreeDebug(
        `stash failed: ${err.message?.split("\n")[0] || err}`,
      );
      return null;
    }
  }

  function _popStash(repoRoot, expectedRef) {
    if (!expectedRef) return;
    try {
      // Find the stash entry by its commit hash anywhere in the stash list,
      // not just the top.  An intervening session-safety stash may have pushed
      // the baseline entry down from stash@{0} to stash@{1} or deeper.
      const list = execFileSync(
        "git",
        ["stash", "list", "--format=%H"],
        {
          cwd: repoRoot,
          timeout: 5000,
          stdio: ["pipe", "pipe", "pipe"],
        },
      )
        .toString()
        .trim();
      if (!list) {
        _writeWorktreeDebug(
          `stash pop: stash list empty, expected ${expectedRef}`,
        );
        return;
      }
      const entries = list.split("\n");
      const idx = entries.indexOf(expectedRef);
      if (idx === -1) {
        _writeWorktreeDebug(
          `stash pop: ref ${expectedRef} not found in stash list (${entries.length} entries)`,
        );
        return;
      }
      execFileSync("git", ["stash", "pop", `stash@{${idx}}`], {
        cwd: repoRoot,
        timeout: 10000,
        stdio: ["pipe", "pipe", "pipe"],
      });
      _writeWorktreeDebug(
        `popped stash@{${idx}}: ${expectedRef}`,
      );
    } catch (err) {
      _writeWorktreeDebug(
        `stash pop failed: ${err.message?.split("\n")[0] || err}`,
      );
    }
  }

  function _refreshMainRepo(repoRoot) {
    try {
      execFileSync("git", ["reset", "--hard", "HEAD"], {
        cwd: repoRoot,
        timeout: 10000,
        stdio: ["pipe", "pipe", "pipe"],
      });
    } catch {}
    _triggerGitRefresh();
  }

  // ------- Focus -------

  function _focusWorktreeFolder(worktreePath) {
    if (_activeWorktreeFolder === worktreePath) return;
    if (!fs.existsSync(worktreePath)) return;

    const mainRepo = _getMainRepoPath();
    const binding = _worktreeBindings.get(worktreePath);
    const targetBranch = binding?.branch;

    if (mainRepo && targetBranch) {
      // If this is the first focus (from baseline), save the original branch and stash
      if (!_originalBranch) {
        _originalBranch = _gitCurrentBranch(mainRepo);
        _stashRef = _stashMainRepo(mainRepo);
        _writeWorktreeDebug(
          `saved baseline: branch=${_originalBranch} stash=${_stashRef || "none"}`,
        );
      }

      const ok = _checkoutBranchViaSymref(mainRepo, targetBranch);
      if (ok) {
        _triggerGitRefresh();
        _writeWorktreeDebug(`checked out ${targetBranch} via symbolic-ref`);
      }
      _displayedBranch = targetBranch;
    }

    _activeWorktreeFolder = worktreePath;
    _worktreeFileProvider?.refresh();
    _writeWorktreeDebug(
      `FOCUSED worktree: ${worktreePath} branch: ${targetBranch || "unknown"}`,
    );
    getDiagnosticsOutputChannel().appendLine(
      `[worktree] Focused: ${worktreePath} branch=${targetBranch || "?"}`,
    );
  }

  function _unfocusWorktreeFolder() {
    if (!_activeWorktreeFolder) return;
    const prev = _activeWorktreeFolder;
    const mainRepo = _getMainRepoPath();

    if (mainRepo && _originalBranch) {
      // Safety: stash any uncommitted changes on the session branch before
      // switching back.  The MCP session-end handler should have committed
      // already, but this protects against race conditions and manual unfocus.
      try {
        const dirty = execFileSync("git", ["status", "--porcelain"], {
          cwd: mainRepo,
          timeout: 5000,
          stdio: ["pipe", "pipe", "pipe"],
        })
          .toString()
          .trim();
        if (dirty) {
          execFileSync(
            "git",
            ["stash", "push", "-m", "gsh-session-unfocus: saving branch work"],
            { cwd: mainRepo, timeout: 10000, stdio: ["pipe", "pipe", "pipe"] },
          );
          _writeWorktreeDebug(
            `stashed session branch work before unfocus (${dirty.split("\n").length} file(s))`,
          );
        }
      } catch (err) {
        _writeWorktreeDebug(
          `warning: could not stash session work: ${err.message?.split("\n")[0] || err}`,
        );
      }

      const ok = _checkoutBranchViaSymref(mainRepo, _originalBranch);
      if (ok) {
        _popStash(mainRepo, _stashRef);
        _triggerGitRefresh();
        _writeWorktreeDebug(`restored baseline: ${_originalBranch}`);
      }
      _originalBranch = null;
      _stashRef = null;
      _displayedBranch = null;
    }

    _activeWorktreeFolder = null;
    _worktreeFileProvider?.refresh();
    _writeWorktreeDebug(`UNFOCUSED worktree: ${prev}`);
    getDiagnosticsOutputChannel().appendLine(`[worktree] Unfocused: ${prev}`);
  }

  // ------- Binding -------

  function _bindWorktree(worktreePath, branch, baseBranch, baseCommit, tabKey) {
    if (!fs.existsSync(worktreePath)) return;

    _worktreeBindings.set(worktreePath, {
      branch,
      baseBranch: baseBranch || "",
      baseCommit: baseCommit || "",
      createdAt: Date.now(),
    });
    saveWorktreeBindings();

    if (!tabKey) {
      try {
        const sessionRes = vscode.window.activeChatPanelSessionResource;
        if (sessionRes) {
          tabKey = sessionRes.toString();
          _writeWorktreeDebug(
            `bindWorktree: got session URI from proposed API: ${tabKey.slice(-12)}`,
          );
        }
      } catch {}
    }

    if (tabKey) {
      _tabToWorktree.set(tabKey, worktreePath);
      saveTabWorktreeMap();
    }

    getDiagnosticsOutputChannel().appendLine(
      `[worktree] Bound branch=${branch} path=${worktreePath} tab=${tabKey || "pending"}`,
    );
    _focusWorktreeFolder(worktreePath);
  }

  function _unbindWorktree(worktreePath) {
    _worktreeBindings.delete(worktreePath);
    saveWorktreeBindings();

    for (const [tabKey, path_] of _tabToWorktree) {
      if (path_ === worktreePath) {
        _tabToWorktree.delete(tabKey);
      }
    }
    saveTabWorktreeMap();

    if (_activeWorktreeFolder === worktreePath) {
      _unfocusWorktreeFolder();
    }
  }

  function _bindWorktreeWithRetry(msg, tabKey, attempt) {
    if (tabKey) {
      _writeWorktreeDebug(
        `bindWorktree: tab=${tabKey} branch=${msg.branch} attempt=${attempt}`,
      );
      _bindWorktree(
        msg.worktreePath,
        msg.branch,
        msg.baseBranch,
        msg.baseCommit,
        tabKey,
      );
      return;
    }

    if (attempt < 5) {
      setTimeout(
        () => {
          const newTabKey = _getActiveChatTabKey();
          _bindWorktreeWithRetry(msg, newTabKey, attempt + 1);
        },
        500 * (attempt + 1),
      );
      return;
    }

    _writeWorktreeDebug(
      `bindWorktree: no tab after ${attempt} retries, binding without tab`,
    );
    _bindWorktree(
      msg.worktreePath,
      msg.branch,
      msg.baseBranch,
      msg.baseCommit,
      null,
    );
  }

  // ------- IPC Message Handler -------

  function handleWorktreeIpcMessage(msg) {
    if (msg.type === "worktreeCreated") {
      getDiagnosticsOutputChannel().appendLine(
        `[worktree] IPC received: worktreeCreated branch=${msg.branch} path=${msg.worktreePath}`,
      );
      _writeWorktreeDebug(
        `IPC worktreeCreated branch=${msg.branch} activity=${msg.activityId || "null"}`,
      );
      const captured = msg.activityId
        ? _pendingBranchSessionStarts.get(msg.activityId)
        : null;
      const tabKey = captured?.tabKey || _getActiveChatTabKey();
      _bindWorktreeWithRetry(msg, tabKey, 0);
    } else if (msg.type === "worktreeRemoved") {
      getDiagnosticsOutputChannel().appendLine(
        `[worktree] IPC received: worktreeRemoved path=${msg.worktreePath}`,
      );
      _unbindWorktree(msg.worktreePath);
    } else if (msg.type === "branchCommit") {
      // Agent committed on a branch — refresh main repo if that branch is focused
      const mainRepo = _getMainRepoPath();
      if (mainRepo && _displayedBranch && _displayedBranch === msg.branch) {
        _writeWorktreeDebug(
          `IPC branchCommit: refreshing main repo for ${msg.branch} (${msg.commitHash})`,
        );
        _refreshMainRepo(mainRepo);
      }
    }
  }

  // ------- Reconciliation -------

  function reconcileWorktreeBindings() {
    let changed = false;
    for (const [wtPath] of _worktreeBindings) {
      if (!fs.existsSync(wtPath)) {
        _worktreeBindings.delete(wtPath);
        changed = true;
      }
    }
    if (changed) saveWorktreeBindings();

    let tabChanged = false;
    for (const [tabKey, wtPath] of _tabToWorktree) {
      if (!_worktreeBindings.has(wtPath)) {
        _tabToWorktree.delete(tabKey);
        tabChanged = true;
      }
    }
    if (tabChanged) saveTabWorktreeMap();

    _cleanupLegacyWorktreeFolders();
    _context?.globalState?.update("gsh.originalBranch", undefined);
    _rescueOrphanedWorktrees();
  }

  function _rescueOrphanedWorktrees() {
    const enabled = vscode.workspace
      .getConfiguration("gitShellHelpers.branchSessions")
      .get("orphanSafetyCommit", true);
    if (!enabled) return;

    const worktreeBase = path.join(os.homedir(), ".cache", "gsh", "worktrees");
    if (!fs.existsSync(worktreeBase)) return;

    let entries;
    try {
      entries = fs.readdirSync(worktreeBase);
    } catch {
      return;
    }

    const boundPaths = new Set(_worktreeBindings.keys());
    const repoRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;

    for (const entry of entries) {
      const wtPath = path.join(worktreeBase, entry);
      try {
        if (!fs.statSync(wtPath).isDirectory()) continue;
      } catch {
        continue;
      }

      if (boundPaths.has(wtPath)) continue;

      const diag = getDiagnosticsOutputChannel();
      diag.appendLine(`[worktree] Found orphaned worktree: ${wtPath}`);

      let branch = "";
      try {
        branch = execFileSync("git", ["symbolic-ref", "--short", "HEAD"], {
          cwd: wtPath,
          timeout: 5000,
          stdio: ["pipe", "pipe", "pipe"],
        })
          .toString()
          .trim();
      } catch {
        branch = entry;
      }

      let dirty = "";
      try {
        dirty = execFileSync("git", ["status", "--short"], {
          cwd: wtPath,
          timeout: 5000,
          stdio: ["pipe", "pipe", "pipe"],
        })
          .toString()
          .trim();
      } catch {}

      if (dirty) {
        try {
          execFileSync("git", ["add", "-A"], {
            cwd: wtPath,
            timeout: 10000,
            stdio: ["pipe", "pipe", "pipe"],
          });
          execFileSync(
            "git",
            [
              "commit",
              "-m",
              `WIP: rescued uncommitted work from orphaned session on '${branch}'`,
            ],
            { cwd: wtPath, timeout: 10000, stdio: ["pipe", "pipe", "pipe"] },
          );
          diag.appendLine(
            `[worktree] Rescued dirty work on orphan branch '${branch}'`,
          );
        } catch (err) {
          diag.appendLine(
            `[worktree] Could not auto-commit orphan '${branch}': ${err.message}`,
          );
        }
      }

      let lastCommit = "";
      try {
        lastCommit = execFileSync("git", ["log", "--oneline", "-1"], {
          cwd: wtPath,
          timeout: 5000,
          stdio: ["pipe", "pipe", "pipe"],
        })
          .toString()
          .trim();
      } catch {}

      if (repoRoot) {
        try {
          execFileSync("git", ["worktree", "remove", "--force", wtPath], {
            cwd: repoRoot,
            timeout: 10000,
            stdio: ["pipe", "pipe", "pipe"],
          });
        } catch {
          try {
            fs.rmSync(wtPath, { recursive: true, force: true });
          } catch {}
          try {
            execFileSync("git", ["worktree", "prune"], {
              cwd: repoRoot,
              timeout: 5000,
              stdio: ["pipe", "pipe", "pipe"],
            });
          } catch {}
        }
      } else {
        try {
          fs.rmSync(wtPath, { recursive: true, force: true });
        } catch {}
      }

      diag.appendLine(
        `[worktree] Removed orphan worktree: ${wtPath} branch=${branch} last=${lastCommit}`,
      );
    }
  }

  // ------- Startup Restore -------

  function _waitForGitExtensionThenRestore() {
    try {
      const gitExt = vscode.extensions.getExtension("vscode.git");
      if (gitExt?.isActive) {
        const api = gitExt.exports?.getAPI(1);
        if (api?.repositories?.length > 0) {
          setTimeout(_restoreSessionStateOnStartup, 500);
          return;
        }
        if (api?.onDidOpenRepository) {
          const disposable = api.onDidOpenRepository(() => {
            disposable.dispose();
            setTimeout(_restoreSessionStateOnStartup, 300);
          });
          setTimeout(() => {
            disposable.dispose();
            _restoreSessionStateOnStartup();
          }, 8000);
          return;
        }
      }
    } catch {}
    setTimeout(_restoreSessionStateOnStartup, 3000);
  }

  function _restoreSessionStateOnStartup() {
    const mainRepo = _getMainRepoPath();
    if (!mainRepo) return;

    // Clean up any leftover head-override from previous sessions
    _removeHeadOverride(mainRepo);

    if (_worktreeBindings.size === 0) {
      _writeWorktreeDebug("startup-restore: no bindings");
      return;
    }

    let currentUri = null;
    try {
      const res = vscode.window.activeChatPanelSessionResource;
      if (res) currentUri = res.toString();
    } catch {}

    if (currentUri) {
      const worktree = _tabToWorktree.get(currentUri);
      if (
        worktree &&
        _worktreeBindings.has(worktree) &&
        fs.existsSync(worktree)
      ) {
        _focusWorktreeFolder(worktree);
        _writeWorktreeDebug(
          `startup-restore: session ${currentUri.slice(-12)} bound to ${path.basename(worktree)} — state restored`,
        );
        return;
      }
    }

    const tabKey = _getActiveChatTabKey();
    if (tabKey) {
      const worktree = _tabToWorktree.get(tabKey);
      if (
        worktree &&
        _worktreeBindings.has(worktree) &&
        fs.existsSync(worktree)
      ) {
        _focusWorktreeFolder(worktree);
        _writeWorktreeDebug(
          `startup-restore: tab ${tabKey.slice(-12)} bound to ${path.basename(worktree)} — state restored`,
        );
        return;
      }
    }

    _writeWorktreeDebug(
      "startup-restore: no matching session, ensuring baseline",
    );
  }

  function _cleanupLegacyWorktreeFolders() {
    const folders = vscode.workspace.workspaceFolders || [];
    if (folders.length <= 1) return;

    const worktreeBase = path.join(os.homedir(), ".cache", "gsh", "worktrees");
    const keep = folders.filter(
      (f) => f.index === 0 || !f.uri.fsPath.startsWith(worktreeBase),
    );
    const removeCount = folders.length - keep.length;
    if (removeCount === 0) return;

    getDiagnosticsOutputChannel().appendLine(
      `[worktree] Cleaning up ${removeCount} legacy worktree folder(s)`,
    );
    const foldersToAdd = keep
      .slice(1)
      .map((f) => ({ uri: f.uri, name: f.name }));
    vscode.workspace.updateWorkspaceFolders(
      1,
      folders.length - 1,
      ...foldersToAdd,
    );
  }

  // ------- File Browser Tree View -------

  class WorktreeFileProvider {
    constructor() {
      this._onDidChangeTreeData = new vscode.EventEmitter();
      this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    }

    refresh() {
      this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element) {
      return element;
    }

    getChildren(element) {
      if (!_activeWorktreeFolder) {
        const item = new vscode.TreeItem("No active branch session");
        item.description = "Start a branch session to browse files";
        return [item];
      }

      const dirPath = element
        ? element.resourceUri.fsPath
        : _activeWorktreeFolder;
      if (!fs.existsSync(dirPath)) return [];

      try {
        const entries = fs.readdirSync(dirPath, { withFileTypes: true });
        const sorted = entries
          .filter((e) => e.name !== ".git")
          .sort((a, b) => {
            if (a.isDirectory() !== b.isDirectory()) {
              return a.isDirectory() ? -1 : 1;
            }
            return a.name.localeCompare(b.name);
          });

        return sorted.map((entry) => {
          const fullPath = path.join(dirPath, entry.name);
          const uri = vscode.Uri.file(fullPath);
          if (entry.isDirectory()) {
            const item = new vscode.TreeItem(
              uri,
              vscode.TreeItemCollapsibleState.Collapsed,
            );
            item.contextValue = "worktreeDir";
            return item;
          } else {
            const item = new vscode.TreeItem(
              uri,
              vscode.TreeItemCollapsibleState.None,
            );
            item.command = {
              command: "vscode.open",
              title: "Open File",
              arguments: [uri],
            };
            item.contextValue = "worktreeFile";
            return item;
          }
        });
      } catch {
        return [];
      }
    }
  }

  function registerWorktreeFileView(context) {
    _worktreeFileProvider = new WorktreeFileProvider();
    const treeView = vscode.window.createTreeView(
      "gitShellHelpers.worktreeFiles",
      {
        treeDataProvider: _worktreeFileProvider,
        showCollapseAll: true,
      },
    );

    const updateTitle = () => {
      if (_activeWorktreeFolder) {
        const binding = _worktreeBindings.get(_activeWorktreeFolder);
        const branch = binding?.branch || path.basename(_activeWorktreeFolder);
        treeView.title = `\u{1F33F} ${branch}`;
      } else {
        treeView.title = "Branch Files";
      }
    };
    _worktreeFileProvider.onDidChangeTreeData(updateTitle);
    updateTitle();

    context.subscriptions.push(treeView);
  }

  // ------- Tab Change Handler -------

  function onActiveTabChanged() {
    if (_worktreeBindings.size === 0 && !_activeWorktreeFolder) return;

    let activeTab;
    try {
      activeTab = vscode.window.tabGroups?.activeTabGroup?.activeTab;
    } catch {
      return;
    }

    const viewType = activeTab?.input?.viewType || null;
    const tabUri = activeTab?.input?.uri?.toString() || null;

    _writeWorktreeDebug(
      `tab-change: viewType=${viewType || "null"} tabUri=${tabUri ? tabUri.slice(-12) : "null"} active=${_activeWorktreeFolder || "null"} bindings=${_worktreeBindings.size}`,
    );

    if (viewType === "workbench.editor.chatSession" && activeTab.input.uri) {
      const tabKey = activeTab.input.uri.toString();
      let worktreePath = _tabToWorktree.get(tabKey);

      if (!worktreePath) {
        const currentSession =
          vscode.window.activeChatPanelSessionResource?.toString();
        if (currentSession) {
          worktreePath = _tabToWorktree.get(currentSession);
          if (worktreePath) {
            _tabToWorktree.set(tabKey, worktreePath);
            saveTabWorktreeMap();
            _writeWorktreeDebug(
              `tab-change: cross-ref from session ${currentSession.slice(-12)} \u2192 ${tabKey.slice(-12)} for ${path.basename(worktreePath)}`,
            );
          }
        }
      }

      if (!worktreePath) {
        worktreePath = _findRecentUnboundWorktree();
        if (worktreePath) {
          _tabToWorktree.set(tabKey, worktreePath);
          saveTabWorktreeMap();
          _writeWorktreeDebug(
            `tab-change: lazy-bound ${tabKey.slice(-12)} to ${path.basename(worktreePath)}`,
          );
        }
      }

      _writeWorktreeDebug(
        `tab-change: resolved worktree=${worktreePath ? path.basename(worktreePath) : "null"} hasBind=${worktreePath ? _worktreeBindings.has(worktreePath) : "N/A"}`,
      );

      if (
        worktreePath &&
        _worktreeBindings.has(worktreePath) &&
        fs.existsSync(worktreePath)
      ) {
        _focusWorktreeFolder(worktreePath);
        return;
      }

      if (
        _activeWorktreeFolder &&
        Date.now() >= _suppressTabDrivenUnfocusUntil
      ) {
        _writeWorktreeDebug(`tab-change: unbound chat tab \u2192 unfocusing`);
        _unfocusWorktreeFolder();
      }
      return;
    }
  }

  // ------- VS Code Patch Management -------

  function checkVscodePatches() {
    const PATCH_APPLY_SCRIPT = path.join(
      __dirname,
      "..",
      "..",
      "scripts",
      "patch-vscode-apply-all.js",
    );
    if (!fs.existsSync(PATCH_APPLY_SCRIPT)) return;
    try {
      const { execSync } = require("child_process");
      const raw = execSync(`node "${PATCH_APPLY_SCRIPT}" --json`, {
        encoding: "utf8",
        timeout: 10000,
      });
      const status = JSON.parse(raw);
      if (status.allPatched) return;

      const missing = status.patches
        .filter((p) => p.status !== "patched")
        .map((p) => p.name);
      const msg = `VS Code patches missing: ${missing.join(", ")}. Branch session navigation requires these patches.`;

      vscode.window
        .showWarningMessage(msg, "Apply Patches", "Dismiss")
        .then((choice) => {
          if (choice !== "Apply Patches") return;
          try {
            execSync(`node "${PATCH_APPLY_SCRIPT}"`, {
              encoding: "utf8",
              timeout: 30000,
            });
            vscode.window.showInformationMessage(
              "Patches applied. Quit and restart VS Code to activate workbench patches (Cmd+Q \u2192 reopen). Git extension patches activate on Reload Window.",
              "OK",
            );
          } catch (err) {
            vscode.window.showErrorMessage(
              `Patch application failed: ${err.message}`,
            );
          }
        });
    } catch {}
  }

  // ------- Session Focus (Proposed API) -------

  function onChatSessionFocusChanged(sessionResource) {
    const sessionUri = sessionResource?.toString() || null;
    const tabKey = _getActiveChatTabKey();

    _writeWorktreeDebug(
      `session-focus event: sessionUri=${sessionUri || "null"} tabUri=${tabKey || "null"} bindings=${_worktreeBindings.size} active=${_activeWorktreeFolder || "null"} tabMap=[${[..._tabToWorktree.entries()].map(([k, v]) => k.slice(-12) + "\u2192" + path.basename(v)).join(",")}]`,
    );

    if (_worktreeBindings.size === 0 && !_activeWorktreeFolder) {
      _writeWorktreeDebug("session-focus: skipped (no bindings, no active)");
      return;
    }
    if (Date.now() < _suppressTabDrivenUnfocusUntil) {
      _writeWorktreeDebug("session-focus: skipped (suppressed)");
      return;
    }

    if (sessionUri) {
      let worktreePath = _tabToWorktree.get(sessionUri);
      if (!worktreePath && tabKey) {
        worktreePath = _tabToWorktree.get(tabKey);
      }

      _writeWorktreeDebug(
        `session-focus lookup: sessionUri=${sessionUri.slice(-12)} tabUri=${tabKey ? tabKey.slice(-12) : "null"} found=${worktreePath || "null"} hasBind=${worktreePath ? _worktreeBindings.has(worktreePath) : "N/A"} exists=${worktreePath ? fs.existsSync(worktreePath) : "N/A"}`,
      );

      if (
        worktreePath &&
        _worktreeBindings.has(worktreePath) &&
        fs.existsSync(worktreePath)
      ) {
        _crossReferenceUris(sessionUri, tabKey, worktreePath);
        _writeWorktreeDebug(`session-focus: focusing ${worktreePath}`);
        _focusWorktreeFolder(worktreePath);
        return;
      }

      const unbound = _findRecentUnboundWorktree();
      if (unbound) {
        _tabToWorktree.set(sessionUri, unbound);
        if (tabKey) _tabToWorktree.set(tabKey, unbound);
        saveTabWorktreeMap();
        _writeWorktreeDebug(
          `session-focus lazy-bound sessionUri=${sessionUri.slice(-12)} tabUri=${tabKey ? tabKey.slice(-12) : "null"} to ${unbound}`,
        );
        _focusWorktreeFolder(unbound);
        return;
      }

      if (
        _activeWorktreeFolder &&
        _worktreeBindings.has(_activeWorktreeFolder)
      ) {
        const boundPaths = new Set(_tabToWorktree.values());
        if (!boundPaths.has(_activeWorktreeFolder)) {
          _tabToWorktree.set(sessionUri, _activeWorktreeFolder);
          if (tabKey) _tabToWorktree.set(tabKey, _activeWorktreeFolder);
          saveTabWorktreeMap();
          _writeWorktreeDebug(
            `session-focus: bound to currently-focused worktree ${path.basename(_activeWorktreeFolder)} (no prior URI)`,
          );
          return;
        }
      }

      _writeWorktreeDebug(
        `session-focus: no worktree match, no unbound candidate`,
      );

      if (_activeWorktreeFolder) {
        _writeWorktreeDebug(
          `session-focus: unknown URI ${sessionUri.slice(-12)} \u2014 unfocusing immediately`,
        );
        _unfocusWorktreeFolder();
      }
      return;
    }

    if (_activeWorktreeFolder) {
      _writeWorktreeDebug(
        `session-focus unfocus: session=null active=${_activeWorktreeFolder}`,
      );
      _unfocusWorktreeFolder();
    }
  }

  function _crossReferenceUris(sessionUri, tabKey, worktreePath) {
    let changed = false;
    if (sessionUri && !_tabToWorktree.has(sessionUri)) {
      _tabToWorktree.set(sessionUri, worktreePath);
      changed = true;
    }
    if (tabKey && !_tabToWorktree.has(tabKey)) {
      _tabToWorktree.set(tabKey, worktreePath);
      changed = true;
    }
    if (changed) {
      saveTabWorktreeMap();
      _writeWorktreeDebug(
        `cross-ref: stored both URIs for ${path.basename(worktreePath)}`,
      );
    }
  }

  // ------- Public API -------

  return {
    loadWorktreeBindings,
    loadTabWorktreeMap,
    reconcileWorktreeBindings,
    registerWorktreeFileView,
    onActiveTabChanged,
    onChatSessionFocusChanged,
    checkVscodePatches,
    handleWorktreeIpcMessage,
    waitForGitExtensionThenRestore: _waitForGitExtensionThenRestore,
    getActiveChatTabKey: _getActiveChatTabKey,
    getPendingBranchSessionStarts: () => _pendingBranchSessionStarts,
    setSuppressTabDrivenUnfocusUntil: (v) => {
      _suppressTabDrivenUnfocusUntil = v;
    },
    writeWorktreeDebug: _writeWorktreeDebug,
  };
};
