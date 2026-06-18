"use strict";

const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

const SETUP_GUIDE_URL =
  "https://github.com/RockyWearsAHat/github-shell-helpers#script-installer-cross-platform";

/**
 * Startup install-health checks: detect an incomplete local Helpers bundle and offer
 * to run the installer (or open the setup guide).
 * @param {{ _context: object, findGitShellHelpersMcpPath: Function }} deps
 */
module.exports = function createInstallHealth(deps) {
  const { _context, findGitShellHelpersMcpPath } = deps;

  let _shownThisActivation = false;

  /** Deduplicate a list of paths, dropping empty entries. */
  function uniquePaths(paths) {
    return [...new Set((paths || []).filter(Boolean))];
  }

  /** First path in `paths` that exists on disk, or "" if none. */
  function firstExistingPath(paths) {
    return uniquePaths(paths).find((candidate) => fs.existsSync(candidate)) || "";
  }

  /** Single-quote a value for safe use in a shell command. */
  function shellQuote(value) {
    return `'${String(value).replace(/'/g, `'\\''`)}'`;
  }

  /** Absolute paths of the open workspace folders. */
  function getWorkspaceRoots() {
    return (vscode.workspace.workspaceFolders || []).map(
      (folder) => folder.uri.fsPath,
    );
  }

  /** Locate the cross-platform installer beside the server or in a workspace. */
  function resolveInstallerPath(serverPath) {
    const serverDir = path.dirname(serverPath);
    return firstExistingPath([
      path.join(serverDir, "install-helpers"),
      ...getWorkspaceRoots().map((root) =>
        path.join(root, "install-helpers"),
      ),
    ]);
  }

  /** Local feature modules that should ship with the server but are absent. */
  function resolveMissingLocalFeatures(serverPath) {
    const serverDir = path.dirname(serverPath);
    const missing = [];

    if (!fs.existsSync(path.join(serverDir, "git-research-mcp"))) {
      missing.push({
        key: "research-runtime",
        label: "git-research-mcp",
        detail: "web search and research tools will stay disabled",
      });
    }

    return missing;
  }

  /** Assess the local install and build the popup view-model (issues + actions). */
  function collectHealthStatus() {
    const serverPath = findGitShellHelpersMcpPath(_context);
    if (!serverPath) {
      return {
        hasLocalInstall: false,
        shouldShowPopup: false,
        issues: [],
      };
    }

    const installerPath = resolveInstallerPath(serverPath);
    const missingLocalFeatures = resolveMissingLocalFeatures(serverPath);
    const issues = [];

    if (missingLocalFeatures.length > 0) {
      issues.push({
        kind: "feature-bundle",
        title: "Local feature bundle is incomplete",
        items: missingLocalFeatures,
      });
    }

    const shouldShowPopup = issues.length > 0;
    const canRunInstaller = Boolean(installerPath);

    const message =
      "Helpers is installed, but parts of the local bundle are missing.";

    const detail = issues
      .map((issue) => {
        const lines = [issue.title + ":"];
        for (const item of issue.items) {
          lines.push(`• ${item.label} — ${item.detail}`);
        }
        return lines.join("\n");
      })
      .concat(
        shouldShowPopup
          ? [
              canRunInstaller
                ? "Run the installer to restore the missing local files."
                : "Open the setup guide to reinstall the missing local files.",
            ]
          : [],
      )
      .join("\n\n");

    return {
      hasLocalInstall: true,
      shouldShowPopup,
      serverPath,
      installerPath,
      canRunInstaller,
      message,
      detail,
      issues,
    };
  }

  /** Buttons to show on the startup popup for the given health status. */
  function getPopupActions(status) {
    const actions = [];
    const hasBundleIssues = status.issues.some(
      (issue) => issue.kind === "feature-bundle",
    );

    if (hasBundleIssues) {
      actions.push(status.canRunInstaller ? "Run Installer" : "Open Setup Guide");
    }

    actions.push("Dismiss");
    return actions;
  }

  /** Run the installer in a terminal, or open the setup guide when it's absent. */
  async function runInstaller(status) {
    if (!status.canRunInstaller || !status.installerPath) {
      await vscode.env.openExternal(vscode.Uri.parse(SETUP_GUIDE_URL));
      return;
    }

    const terminal = vscode.window.createTerminal("helpers installer");
    terminal.show();
    terminal.sendText(`bash ${shellQuote(status.installerPath)}`);
  }

  /** Once per activation, surface a modal popup when the install is incomplete. */
  async function maybeShowStartupPopup() {
    if (_shownThisActivation) {
      return false;
    }

    const status = collectHealthStatus();
    if (!status.shouldShowPopup) {
      return false;
    }

    _shownThisActivation = true;
    const actions = getPopupActions(status);
    const choice = await vscode.window.showWarningMessage(
      status.message,
      {
        modal: true,
        detail: status.detail,
      },
      ...actions,
    );

    if (choice === "Run Installer") {
      await runInstaller(status);
    } else if (choice === "Open Setup Guide") {
      await vscode.env.openExternal(vscode.Uri.parse(SETUP_GUIDE_URL));
    }

    return true;
  }

  return {
    collectHealthStatus,
    maybeShowStartupPopup,
  };
};
