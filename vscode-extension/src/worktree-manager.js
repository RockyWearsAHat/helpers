"use strict";
// src/worktree-manager.js — Worktree ↔ Chat binding, focus, head override, file browser
const vscode = require("vscode");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const WORKTREE_BINDINGS_KEY = "worktreeBindings.v2";
const TAB_WORKTREE_KEY = "tabToWorktree";
const SESSION_STATE_KEY = "gsh.sessionState.v1";
const UNKNOWN_SESSION_GRACE_MS =
  Number.parseInt(
    process.env.GSH_WORKTREE_UNKNOWN_SESSION_GRACE_MS || "1200",
    10,
  ) || 1200;

module.exports = function createWorktreeManager(deps) {
  const { _context, getDiagnosticsOutputChannel } = deps;

  let _worktreeBindings = new Map();
  let _tabToWorktree = new Map();
  let _sessionStateMap = new Map(); // worktreePath → { originalBranch, stashRef, focusedAt }
  let _pendingBranchSessionStarts = new Map();
  let _activeWorktreeFolder = null;
  let _pendingUnknownSessionUnfocus = null;
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

  function loadSessionState() {
    try {
      const raw = _context?.globalState?.get(SESSION_STATE_KEY);
      if (raw && typeof raw === "object" && !Array.isArray(raw)) {
        _sessionStateMap = new Map(Object.entries(raw));
      }
    } catch {}
  }

  function saveSessionState() {
    try {
      const obj = {};
      for (const [k, v] of _sessionStateMap) obj[k] = v;
      _context?.globalState?.update(SESSION_STATE_KEY, obj);
    } catch {}
  }

  function _recordSessionState(wtPath) {
    _sessionStateMap.set(wtPath, {
      originalBranch: _originalBranch,
      stashRef: _stashRef,
      stashMessage: "gsh-session-focus: auto-stash",
      focusedAt: Date.now(),
    });
    saveSessionState();
  }

  function _clearSessionState(wtPath) {
    _sessionStateMap.delete(wtPath);
    saveSessionState();
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

  function _clearPendingUnknownSessionUnfocus(reason) {
    if (!_pendingUnknownSessionUnfocus) return;
    clearTimeout(_pendingUnknownSessionUnfocus);
    _pendingUnknownSessionUnfocus = null;
    if (reason) {
      _writeWorktreeDebug(`unknown-session: cancelled delayed unfocus (${reason})`);
    }
  }

  function _scheduleUnknownSessionUnfocus(reason) {
    if (!_activeWorktreeFolder) return;
    if (_pendingUnknownSessionUnfocus) {
      _writeWorktreeDebug(
        `unknown-session: delayed unfocus already pending (${reason})`,
      );
      return;
    }

    const scheduledWorktree = _activeWorktreeFolder;
    _writeWorktreeDebug(
      `unknown-session: delaying unfocus for ${UNKNOWN_SESSION_GRACE_MS}ms (${reason})`,
    );

    _pendingUnknownSessionUnfocus = setTimeout(() => {
      _pendingUnknownSessionUnfocus = null;

      if (
        !_activeWorktreeFolder ||
        _activeWorktreeFolder !== scheduledWorktree
      ) {
        _writeWorktreeDebug(
          "unknown-session: delayed unfocus skipped because active worktree changed",
        );
        return;
      }

      const currentKey = _resolveCurrentChatKey();
      const reboundWorktree = currentKey ? _tabToWorktree.get(currentKey) : null;
      if (
        reboundWorktree &&
        _worktreeBindings.has(reboundWorktree) &&
        fs.existsSync(reboundWorktree)
      ) {
        _writeWorktreeDebug(
          `unknown-session: delayed unfocus resolved to ${path.basename(reboundWorktree)}`,
        );
        _focusWorktreeFolder(reboundWorktree);
        return;
      }

      _writeWorktreeDebug(
        `unknown-session: grace expired, unfocusing ${path.basename(scheduledWorktree)}`,
      );
      _unfocusWorktreeFolder();
    }, UNKNOWN_SESSION_GRACE_MS);
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
    // Fire immediately to clear any stale cached state (e.g. intermediate
    // "Added" status between symbolic-ref and reset --hard, or after a
    // branchCommit brings the repo forward)
    try {
      const gitExt = vscode.extensions.getExtension("vscode.git");
      if (gitExt?.isActive) {
        const api = gitExt.exports?.getAPI(1);
        const repo = api?.repositories?.[0];
        if (repo) repo.status();
      }
    } catch {}
    // Fire again after a short delay to catch async file-watcher updates
    _gitRefreshTimer = setTimeout(() => {
      _gitRefreshTimer = null;
      try {
        const gitExt = vscode.extensions.getExtension("vscode.git");
        if (!gitExt?.isActive) return;
        const api = gitExt.exports?.getAPI(1);
        const repo = api?.repositories?.[0];
        if (repo) repo.status();
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
    // Step 1: Update HEAD ref (fast, unlikely to fail)
    try {
      execFileSync("git", ["symbolic-ref", "HEAD", `refs/heads/${branch}`], {
        cwd: repoRoot,
        timeout: 5000,
        stdio: ["pipe", "pipe", "pipe"],
      });
    } catch (err) {
      _writeWorktreeDebug(
        `symref failed for ${branch}: ${err.stderr?.toString().trim() || err.message?.split("\n")[0] || err}`,
      );
      return false;
    }

    // Step 2: Update working tree to match HEAD.
    // May fail due to index.lock, permission issues, etc. — retry once.
    for (let attempt = 0; attempt < 2; attempt++) {
      try {
        execFileSync("git", ["reset", "--hard", "HEAD"], {
          cwd: repoRoot,
          timeout: 10000,
          stdio: ["pipe", "pipe", "pipe"],
        });
        return true;
      } catch (err) {
        const stderr =
          err.stderr?.toString().trim() ||
          err.message?.split("\n")[0] ||
          String(err);
        if (attempt === 0 && stderr.includes("index.lock")) {
          _writeWorktreeDebug(
            `reset --hard attempt ${attempt} failed (index.lock), retrying: ${stderr}`,
          );
          try {
            fs.unlinkSync(path.join(repoRoot, ".git", "index.lock"));
          } catch {}
          continue;
        }
        _writeWorktreeDebug(
          `reset --hard failed for ${branch} (attempt ${attempt}): ${stderr}`,
        );
      }
    }

    // symref succeeded but reset failed — HEAD points to branch but working
    // tree is stale.  Return 'partial' so the caller can decide what to do.
    return "partial";
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
        `stash failed: ${err.stderr?.toString().trim() || err.message?.split("\n")[0] || err}`,
      );
      return null;
    }
  }

  function _findStashByMessage(repoRoot, msgFragment) {
    try {
      const list = execFileSync("git", ["stash", "list", "--format=%H %gs"], {
        cwd: repoRoot,
        timeout: 5000,
        stdio: ["pipe", "pipe", "pipe"],
      })
        .toString()
        .trim();
      if (!list) return null;
      for (const line of list.split("\n")) {
        const spaceIdx = line.indexOf(" ");
        if (spaceIdx === -1) continue;
        const hash = line.slice(0, spaceIdx);
        const subject = line.slice(spaceIdx + 1);
        if (subject.includes(msgFragment)) return hash;
      }
    } catch {}
    return null;
  }

  function _popStash(repoRoot, expectedRef) {
    if (!expectedRef) return;
    try {
      const topRef = execFileSync(
        "git",
        ["stash", "list", "-1", "--format=%H"],
        {
          cwd: repoRoot,
          timeout: 5000,
          stdio: ["pipe", "pipe", "pipe"],
        },
      )
        .toString()
        .trim();
      if (topRef === expectedRef) {
        execFileSync("git", ["stash", "pop"], {
          cwd: repoRoot,
          timeout: 10000,
          stdio: ["pipe", "pipe", "pipe"],
        });
        _writeWorktreeDebug(`popped stash: ${expectedRef}`);
      } else {
        // Hash mismatch — try to find stash by message as fallback (handles
        // manual stash ops or a VS Code reload between focus and unfocus).
        _writeWorktreeDebug(
          `stash mismatch: top=${topRef} expected=${expectedRef}, searching by message`,
        );
        const fallbackRef = _findStashByMessage(
          repoRoot,
          "gsh-session-focus: auto-stash",
        );
        if (fallbackRef) {
          execFileSync("git", ["stash", "pop", fallbackRef], {
            cwd: repoRoot,
            timeout: 10000,
            stdio: ["pipe", "pipe", "pipe"],
          });
          _writeWorktreeDebug(
            `popped stash by message fallback: ${fallbackRef}`,
          );
        } else {
          _writeWorktreeDebug(`no matching stash found, skipping pop`);
        }
      }
    } catch (err) {
      _writeWorktreeDebug(
        `stash pop failed: ${err.message?.split("\n")[0] || err}`,
      );
    }
  }

  function _refreshMainRepo(repoRoot) {
    for (let attempt = 0; attempt < 2; attempt++) {
      try {
        execFileSync("git", ["reset", "--hard", "HEAD"], {
          cwd: repoRoot,
          timeout: 10000,
          stdio: ["pipe", "pipe", "pipe"],
        });
        break;
      } catch (err) {
        const stderr =
          err.stderr?.toString().trim() ||
          err.message?.split("\n")[0] ||
          String(err);
        if (attempt === 0 && stderr.includes("index.lock")) {
          try {
            fs.unlinkSync(path.join(repoRoot, ".git", "index.lock"));
          } catch {}
          continue;
        }
        _writeWorktreeDebug(`refreshMainRepo reset failed: ${stderr}`);
      }
    }
    _triggerGitRefresh();
  }

  // ------- Focus -------

  function _focusWorktreeFolder(worktreePath) {
    if (_activeWorktreeFolder === worktreePath) {
      _clearPendingUnknownSessionUnfocus("focus confirmed");
      return;
    }
    if (!fs.existsSync(worktreePath)) return;

    _clearPendingUnknownSessionUnfocus("switching focus");

    const mainRepo = _getMainRepoPath();

    // If resuming after a VS Code reload, recover session state from persistence
    // so we don't accidentally overwrite the real baseline branch/stash.
    if (!_originalBranch && _sessionStateMap.has(worktreePath)) {
      const saved = _sessionStateMap.get(worktreePath);
      _originalBranch = saved.originalBranch || null;
      _stashRef = saved.stashRef || null;
      _writeWorktreeDebug(
        `startup: recovered session state: branch=${_originalBranch} stash=${_stashRef || "none"}`,
      );
    }

    const binding = _worktreeBindings.get(worktreePath);
    const targetBranch = binding?.branch;

    if (mainRepo && targetBranch) {
      const currentBranch = _gitCurrentBranch(mainRepo);

      // If this is the first focus (from baseline), save the original branch
      // and stash.  Guard: if the repo is already on the target branch
      // (leftover from a previous session or restart), don't save the feature
      // branch as the baseline — use the binding's baseBranch instead.
      if (!_originalBranch) {
        if (currentBranch === targetBranch) {
          // Already on the target branch — use the base branch from the
          // binding as the real baseline (e.g. "dev"), not the feature branch.
          _originalBranch = binding.baseBranch || currentBranch;
          _stashRef = null;
          _writeWorktreeDebug(
            `saved baseline (already on target): branch=${_originalBranch} stash=none`,
          );
        } else {
          _originalBranch = currentBranch;
          _stashRef = _stashMainRepo(mainRepo);
          _writeWorktreeDebug(
            `saved baseline: branch=${_originalBranch} stash=${_stashRef || "none"}`,
          );
        }
        _recordSessionState(worktreePath);
      }

      // Write head override before checkout so the Git status bar shows the
      // target branch immediately, even if the reset step is slow.
      _writeHeadOverride(mainRepo, targetBranch);

      // If we're already on the target branch, just refresh the working tree
      // instead of doing a full symref → reset cycle.
      if (currentBranch === targetBranch) {
        _refreshMainRepo(mainRepo);
        _writeWorktreeDebug(
          `already on ${targetBranch}, refreshed working tree`,
        );
      } else {
        const ok = _checkoutBranchViaSymref(mainRepo, targetBranch);
        if (ok === true) {
          _triggerGitRefresh();
          _writeWorktreeDebug(`checked out ${targetBranch} via symbolic-ref`);
        } else if (ok === "partial") {
          // symref updated HEAD but reset --hard failed — working tree is
          // stale but HEAD points to the right branch.  Still mark as
          // partially focused so branchCommit refreshes can fix it.
          _triggerGitRefresh();
          _writeWorktreeDebug(
            `partial checkout for ${targetBranch}: HEAD updated but working tree stale`,
          );
        } else {
          // Total failure — remove head override to avoid confusion
          _removeHeadOverride(mainRepo);
          _writeWorktreeDebug(
            `FOCUS FAILED for ${worktreePath}: symref failed for ${targetBranch}`,
          );
          getDiagnosticsOutputChannel().appendLine(
            `[worktree] Focus FAILED: could not checkout ${targetBranch}`,
          );
          return;
        }
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
    _clearPendingUnknownSessionUnfocus("unfocus start");
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
            [
              "stash",
              "push",
              "--include-untracked",
              "-m",
              "gsh-session-unfocus: saving branch work",
            ],
            { cwd: mainRepo, timeout: 10000, stdio: ["pipe", "pipe", "pipe"] },
          );
          _writeWorktreeDebug(
            `stashed session branch work before unfocus (${dirty.split("\n").length} file(s), incl untracked)`,
          );
        }
      } catch (err) {
        _writeWorktreeDebug(
          `warning: could not stash session work: ${err.stderr?.toString().trim() || err.message?.split("\n")[0] || err}`,
        );
      }

      const ok = _checkoutBranchViaSymref(mainRepo, _originalBranch);
      if (ok === true || ok === "partial") {
        _removeHeadOverride(mainRepo);
        _popStash(mainRepo, _stashRef);
        _triggerGitRefresh();
        _writeWorktreeDebug(`restored baseline: ${_originalBranch}`);
      } else {
        _writeWorktreeDebug(
          `warning: could not restore baseline ${_originalBranch}, removing head override anyway`,
        );
        _removeHeadOverride(mainRepo);
      }
      _originalBranch = null;
      _stashRef = null;
      _displayedBranch = null;
      _clearSessionState(prev);
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

    _worktreeFileProvider?.refresh();

    getDiagnosticsOutputChannel().appendLine(
      `[worktree] Bound branch=${branch} path=${worktreePath} tab=${tabKey || "pending"}`,
    );

    // Only auto-focus if the binding tab is the currently active session.
    // Prevents a background agent's branch_session_start from hijacking the
    // repo checkout when the user has switched to a different chat.
    const currentKey = _resolveCurrentChatKey();
    if (tabKey && currentKey && tabKey !== currentKey) {
      _writeWorktreeDebug(
        `bindWorktree: deferred focus for ${branch} (active=${currentKey.slice(-12)} bind=${tabKey.slice(-12)})`,
      );
    } else {
      _focusWorktreeFolder(worktreePath);
    }
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
    _worktreeFileProvider?.refresh();

    if (_activeWorktreeFolder === worktreePath) {
      _unfocusWorktreeFolder();
    }
  }

  function _resolveCurrentChatKey() {
    // Try proposed API session URI first (more stable across tab navigation),
    // then fall back to the tab URI from the tab groups API.
    try {
      const sessionRes = vscode.window.activeChatPanelSessionResource;
      if (sessionRes) return sessionRes.toString();
    } catch {}
    return _getActiveChatTabKey();
  }

  function _bindWorktreeWithRetry(msg, tabKey, attempt) {
    // On each retry, also try the proposed session API in case the tab
    // navigator returned a non-chat URI (user clicked an editor mid-session)
    const resolvedKey = tabKey || _resolveCurrentChatKey();

    if (resolvedKey) {
      _writeWorktreeDebug(
        `bindWorktree: key=${resolvedKey.slice(-16)} branch=${msg.branch} attempt=${attempt}`,
      );
      _bindWorktree(
        msg.worktreePath,
        msg.branch,
        msg.baseBranch,
        msg.baseCommit,
        resolvedKey,
      );
      return;
    }

    if (attempt < 8) {
      setTimeout(
        () => {
          _bindWorktreeWithRetry(msg, null, attempt + 1);
        },
        300 * (attempt + 1),
      );
      return;
    }

    _writeWorktreeDebug(
      `bindWorktree: no key after ${attempt} retries, binding without tab`,
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
      const tabKey = captured?.tabKey || _resolveCurrentChatKey();
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

    // Process each entry asynchronously — yield to the event loop between
    // iterations so N×5 synchronous git calls don't block the extension host.
    function processEntry(index) {
      if (index >= entries.length) return;
      setImmediate(() => {
        const entry = entries[index];
        const wtPath = path.join(worktreeBase, entry);
        try {
          if (!fs.statSync(wtPath).isDirectory()) {
            processEntry(index + 1);
            return;
          }
        } catch {
          processEntry(index + 1);
          return;
        }

        if (boundPaths.has(wtPath)) {
          processEntry(index + 1);
          return;
        }

        _rescueOneOrphan(wtPath, repoRoot);
        processEntry(index + 1);
      });
    }
    processEntry(0);
  }

  function _rescueOneOrphan(wtPath, repoRoot) {
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
      branch = path.basename(wtPath);
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

    // No active session matches a binding. If the repo is stuck on a
    // feature branch from a previous session, restore the baseline.
    const currentBranch = _gitCurrentBranch(mainRepo);
    if (currentBranch) {
      for (const [, binding] of _worktreeBindings) {
        if (binding.branch === currentBranch && binding.baseBranch) {
          _writeWorktreeDebug(
            `startup-restore: repo on session branch ${currentBranch}, restoring to ${binding.baseBranch}`,
          );
          _checkoutBranchViaSymref(mainRepo, binding.baseBranch);
          _removeHeadOverride(mainRepo);
          _triggerGitRefresh();
          return;
        }
      }
    }

    _writeWorktreeDebug("startup-restore: no matching session, baseline OK");
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
        if (_worktreeBindings.size === 0) {
          const item = new vscode.TreeItem("No active branch session");
          item.description = "Start a branch session to browse files";
          item.tooltip =
            "Branch session files appear here when the current chat owns a session.";
          return [item];
        }

        const summary = new vscode.TreeItem(
          "No branch session focused in this chat",
        );
        summary.description = `${_worktreeBindings.size} parked`;
        summary.tooltip =
          "Branch sessions are parked, not lost. Switch back to the chat that owns one or run branch_status to find them.";
        summary.contextValue = "worktreeSessionHint";

        const parkedItems = Array.from(_worktreeBindings.entries())
          .sort(([, left], [, right]) =>
            (left.branch || "").localeCompare(right.branch || ""),
          )
          .map(([wtPath, binding]) => {
            const item = new vscode.TreeItem(
              binding.branch || path.basename(wtPath),
            );
            item.description = "parked";
            item.tooltip =
              `Session parked at ${wtPath}. Switch back to the owning chat to bring it into the workspace.`;
            item.contextValue = "worktreeSessionHint";
            return item;
          });

        return [summary, ...parkedItems];
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
      } else if (_worktreeBindings.size > 0) {
        treeView.title = `Branch Files (${_worktreeBindings.size} parked)`;
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

    // Only chat-session tabs affect worktree focus. Editor/terminal tab
    // switches are irrelevant — early-return to avoid log spam (VS Code
    // fires dozens of tab-change events per click).
    if (viewType !== "workbench.editor.chatSession") return;

    const tabUri = activeTab?.input?.uri?.toString() || null;

    _writeWorktreeDebug(
      `tab-change: tabUri=${tabUri ? tabUri.slice(-12) : "null"} active=${_activeWorktreeFolder || "null"} bindings=${_worktreeBindings.size}`,
    );

    if (activeTab.input.uri) {
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
        _writeWorktreeDebug(
          `tab-change: unbound chat tab \u2192 scheduling delayed unfocus`,
        );
        _scheduleUnknownSessionUnfocus("tab-change: unbound chat tab");
      }
      return;
    }
  }

  // ------- VS Code Patch Management -------

  function checkVscodePatches() {
    const branchSessionsEnabled = vscode.workspace
      .getConfiguration("gitShellHelpers.branchSessions")
      .get("enabled", false);
    if (!branchSessionsEnabled) return;

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
      const msg = `Optional VS Code branch-session patches are not installed: ${missing.join(", ")}. Branch sessions still work without them, but the UI is smoother when they are applied.`;

      vscode.window
        .showInformationMessage(msg, "Apply Patches", "Dismiss")
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
          _clearPendingUnknownSessionUnfocus("bound current worktree to session URI");
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
          `session-focus: unknown URI ${sessionUri.slice(-12)} \u2014 scheduling delayed unfocus`,
        );
        _scheduleUnknownSessionUnfocus(
          `session-focus: unknown URI ${sessionUri.slice(-12)}`,
        );
      }
      return;
    }

    if (_activeWorktreeFolder) {
      _writeWorktreeDebug(
        `session-focus: session=null active=${_activeWorktreeFolder} \u2014 scheduling delayed unfocus`,
      );
      _scheduleUnknownSessionUnfocus("session-focus: session=null");
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
    loadSessionState,
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
