"use strict";

const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const { execFileSync } = require("child_process");

const PATCH_HELPER_FILES = [
  "patch-vscode-apply-all.js",
  "patch-vscode-folder-switch.js",
  "patch-vscode-git-head-display.js",
  "patch-vscode-runsubagent-model.js",
];

const SETUP_GUIDE_URL =
  "https://github.com/RockyWearsAHat/github-shell-helpers#script-installer-cross-platform";

module.exports = function createInstallHealth(deps) {
  const { _context, findGitShellHelpersMcpPath, execFileSync: runFile = execFileSync } = deps;

  let _shownThisActivation = false;

  function uniquePaths(paths) {
    return [...new Set((paths || []).filter(Boolean))];
  }

  function firstExistingPath(paths) {
    return uniquePaths(paths).find((candidate) => fs.existsSync(candidate)) || "";
  }

  function shellQuote(value) {
    return `'${String(value).replace(/'/g, `'\\''`)}'`;
  }

  function getWorkspaceRoots() {
    return (vscode.workspace.workspaceFolders || []).map(
      (folder) => folder.uri.fsPath,
    );
  }

  function getShareRoot(serverPath) {
    const serverDir = path.dirname(serverPath);
    if (path.basename(serverDir) !== "bin") {
      return "";
    }
    return path.join(path.dirname(serverDir), "share", "github-shell-helpers");
  }

  function resolvePatchScriptPaths(serverPath) {
    const serverDir = path.dirname(serverPath);
    const shareRoot = getShareRoot(serverPath);
    const scriptDirs = uniquePaths([
      path.join(serverDir, "scripts"),
      shareRoot ? path.join(shareRoot, "scripts") : "",
      ...getWorkspaceRoots().map((root) => path.join(root, "scripts")),
    ]);

    const coordinatorPath = firstExistingPath(
      scriptDirs.map((scriptDir) =>
        path.join(scriptDir, "patch-vscode-apply-all.js"),
      ),
    );
    const helperRoot = coordinatorPath
      ? path.dirname(coordinatorPath)
      : scriptDirs.find((scriptDir) => fs.existsSync(scriptDir)) || scriptDirs[0] || "";
    const missingHelpers = helperRoot
      ? PATCH_HELPER_FILES.filter(
          (fileName) => !fs.existsSync(path.join(helperRoot, fileName)),
        )
      : PATCH_HELPER_FILES.slice();

    return { coordinatorPath, helperRoot, missingHelpers };
  }

  function resolveInstallerPath(serverPath) {
    const serverDir = path.dirname(serverPath);
    return firstExistingPath([
      path.join(serverDir, "install-git-shell-helpers"),
      ...getWorkspaceRoots().map((root) =>
        path.join(root, "install-git-shell-helpers"),
      ),
    ]);
  }

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

  function readPatchStatus(coordinatorPath) {
    if (!coordinatorPath) {
      return { missingPatches: [], checkError: "patch coordinator not found" };
    }

    try {
      const raw = runFile(process.execPath, [coordinatorPath, "--json"], {
        encoding: "utf8",
        timeout: 10000,
      });
      const status = JSON.parse(raw);
      return {
        missingPatches: (status.patches || [])
          .filter((patch) => patch.status !== "patched")
          .map((patch) => patch.name),
        checkError: "",
      };
    } catch (error) {
      return {
        missingPatches: [],
        checkError: error instanceof Error ? error.message : String(error),
      };
    }
  }

  function collectHealthStatus() {
    const serverPath = findGitShellHelpersMcpPath(_context);
    if (!serverPath) {
      return {
        hasLocalInstall: false,
        shouldShowPopup: false,
        issues: [],
      };
    }

    const branchSessionsEnabled = vscode.workspace
      .getConfiguration("gitShellHelpers.branchSessions")
      .get("enabled", false);
    const installerPath = resolveInstallerPath(serverPath);
    const patchScripts = resolvePatchScriptPaths(serverPath);
    const missingLocalFeatures = resolveMissingLocalFeatures(serverPath);
    const issues = [];

    if (missingLocalFeatures.length > 0) {
      issues.push({
        kind: "feature-bundle",
        title: "Local feature bundle is incomplete",
        items: missingLocalFeatures,
      });
    }

    if (patchScripts.missingHelpers.length > 0) {
      issues.push({
        kind: "patch-bundle",
        title: "Local patch helpers are missing",
        items: patchScripts.missingHelpers.map((fileName) => ({
          label: fileName,
          detail: "repatching and branch-session UX fixes are unavailable",
        })),
      });
    }

    let missingPatches = [];
    let patchCheckError = "";
    if (
      branchSessionsEnabled &&
      patchScripts.coordinatorPath &&
      patchScripts.missingHelpers.length === 0
    ) {
      const patchStatus = readPatchStatus(patchScripts.coordinatorPath);
      missingPatches = patchStatus.missingPatches;
      patchCheckError = patchStatus.checkError;

      if (patchCheckError) {
        issues.push({
          kind: "patch-check",
          title: "VS Code patch status could not be verified",
          items: [
            {
              label: "patch-vscode-apply-all.js",
              detail: patchCheckError,
            },
          ],
        });
      } else if (missingPatches.length > 0) {
        issues.push({
          kind: "patch-status",
          title: "VS Code branch-session patches are missing",
          items: missingPatches.map((name) => ({
            label: name,
            detail: "branch-session UX will stay degraded until patches are applied",
          })),
        });
      }
    }

    const shouldShowPopup = issues.length > 0;
    const hasPatchAction =
      branchSessionsEnabled &&
      patchScripts.coordinatorPath &&
      patchScripts.missingHelpers.length === 0 &&
      missingPatches.length > 0;
    const canRunInstaller = Boolean(installerPath);

    const message = issues.some((issue) => issue.kind === "patch-status")
      ? "Git Shell Helpers is installed, but your local desktop integration is incomplete."
      : "Git Shell Helpers is installed, but parts of the local bundle are missing.";

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
              hasPatchAction
                ? "Apply patches afterward to restore the full branch-session UX."
                : "",
            ].filter(Boolean)
          : [],
      )
      .join("\n\n");

    return {
      hasLocalInstall: true,
      shouldShowPopup,
      serverPath,
      branchSessionsEnabled,
      installerPath,
      patchCoordinatorPath: patchScripts.coordinatorPath,
      canRunInstaller,
      canApplyPatches: hasPatchAction,
      message,
      detail,
      issues,
    };
  }

  function getPopupActions(status) {
    const actions = [];
    const hasBundleIssues = status.issues.some(
      (issue) => issue.kind === "feature-bundle" || issue.kind === "patch-bundle",
    );

    if (hasBundleIssues) {
      actions.push(status.canRunInstaller ? "Run Installer" : "Open Setup Guide");
    }

    if (status.canApplyPatches) {
      actions.push("Apply Patches");
    }

    actions.push("Dismiss");
    return actions;
  }

  async function runInstaller(status) {
    if (!status.canRunInstaller || !status.installerPath) {
      await vscode.env.openExternal(vscode.Uri.parse(SETUP_GUIDE_URL));
      return;
    }

    const terminal = vscode.window.createTerminal("gsh installer");
    terminal.show();
    terminal.sendText(`bash ${shellQuote(status.installerPath)}`);
  }

  function applyPatches(status) {
    if (!status.patchCoordinatorPath) {
      return;
    }

    try {
      runFile(process.execPath, [status.patchCoordinatorPath], {
        encoding: "utf8",
        timeout: 30000,
      });
      runFile(process.execPath, [status.patchCoordinatorPath, "--check"], {
        encoding: "utf8",
        timeout: 10000,
      });
      vscode.window.showInformationMessage(
        "Patches applied. Quit and restart VS Code to activate workbench patches (Cmd+Q → reopen). Git extension patches activate on Reload Window.",
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      vscode.window.showErrorMessage(`Patch application failed: ${message}`);
    }
  }

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

    if (choice === "Apply Patches") {
      applyPatches(status);
    } else if (choice === "Run Installer") {
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