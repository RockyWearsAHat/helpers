"use strict";
// src/community-settings.js — Community cache settings, whitelist, mode, and sync
const vscode = require("vscode");
const fs = require("fs");

const SCHEMA_VERSION = 1;
const PREDEFINED = {
  baseBranch: "main",
  branchPrefix: "automation/community-cache-submission",
};

const MODES = [
  { value: "disabled", label: "Submissions disabled" },
  { value: "pull-and-auto-submit", label: "Submit from all repos" },
  { value: "auto-submit-only-public", label: "Submit from public repos only" },
  {
    value: "auto-submit-whitelist",
    label: "Submit from whitelisted repos only",
  },
];

module.exports = function createCommunitySettings(deps) {
  const {
    _context,
    getCachedUser,
    getCachedRepos,
    setCachedRepos,
    runGh,
    isGhAuthed,
    getGhUser,
    fetchRepos,
    checkGpgUploadStatus,
    readJsonFile,
    writeJsonFile,
    globalSettingsPath,
    workspaceSettingsPath,
    workspaceManifestPath,
    isGroupEnabled,
    getWebviewProvider,
  } = deps;

  function defaultCommunityRepoFromWorkspace(workspaceFolder) {
    const manifest = readJsonFile(workspaceManifestPath(workspaceFolder));
    return manifest?.defaultCommunityRepo || "";
  }

  function findLocalCommunityCloneFolder() {
    const folders = vscode.workspace.workspaceFolders || [];
    return (
      folders.find((folder) => fs.existsSync(workspaceManifestPath(folder))) ||
      null
    );
  }

  function getWhitelist() {
    return _context?.globalState.get("whitelistedRepos", []) ?? [];
  }

  function getMode() {
    return _context?.globalState.get("mode", "disabled") ?? "disabled";
  }

  async function setMode(mode) {
    await _context?.globalState.update("mode", mode);
    syncAllSettings();
    getWebviewProvider()?.refresh();
  }

  async function setWhitelist(repos) {
    await _context?.globalState.update("whitelistedRepos", repos);
    syncAllSettings();
    getWebviewProvider()?.refresh();
  }

  function buildSettingsJson() {
    const globalData = readJsonFile(globalSettingsPath()) || {};
    const localCloneFolder = findLocalCommunityCloneFolder();
    const derivedCommunityRepo =
      globalData.communityRepo ||
      (localCloneFolder
        ? defaultCommunityRepoFromWorkspace(localCloneFolder)
        : "") ||
      "RockyWearsAHat/github-shell-helpers";

    return {
      schemaVersion: SCHEMA_VERSION,
      communityRepo: derivedCommunityRepo,
      ...PREDEFINED,
      mode: getMode(),
      whitelistedRepos: getWhitelist(),
      shareResearch: isGroupEnabled("communityResearch"),
      shareKnowledge: isGroupEnabled("communityResearch"),
      ...(globalData.localClone
        ? { localClone: globalData.localClone }
        : localCloneFolder
          ? { localClone: localCloneFolder.uri.fsPath }
          : {}),
    };
  }

  function buildWorkspaceSettingsJson(workspaceFolder) {
    const globalSettings = buildSettingsJson();
    const workspaceCommunityRepo =
      defaultCommunityRepoFromWorkspace(workspaceFolder);

    return {
      ...globalSettings,
      ...(workspaceCommunityRepo
        ? { communityRepo: workspaceCommunityRepo }
        : {}),
      ...(fs.existsSync(workspaceManifestPath(workspaceFolder))
        ? { localClone: "." }
        : {}),
    };
  }

  function syncAllSettings() {
    writeJsonFile(globalSettingsPath(), buildSettingsJson());
    const folders = vscode.workspace.workspaceFolders;
    if (folders) {
      for (const folder of folders) {
        writeJsonFile(
          workspaceSettingsPath(folder),
          buildWorkspaceSettingsJson(folder),
        );
      }
    }
  }

  function importFromJson() {
    const currentMode = _context?.globalState.get("mode");
    if (currentMode === "pull-only") {
      _context?.globalState.update("mode", "disabled");
      return;
    }
    if (!currentMode) {
      const globalData = readJsonFile(globalSettingsPath());
      if (globalData?.mode) {
        _context?.globalState.update("mode", globalData.mode);
        if (Array.isArray(globalData.whitelistedRepos)) {
          _context?.globalState.update(
            "whitelistedRepos",
            globalData.whitelistedRepos,
          );
        }
        return;
      }
      const folders = vscode.workspace.workspaceFolders;
      if (folders) {
        for (const folder of folders) {
          const wsData = readJsonFile(workspaceSettingsPath(folder));
          if (wsData?.mode) {
            _context?.globalState.update("mode", wsData.mode);
            if (Array.isArray(wsData.whitelistedRepos)) {
              _context?.globalState.update(
                "whitelistedRepos",
                wsData.whitelistedRepos,
              );
            }
            return;
          }
        }
      }
    }
  }

  function showCommunityStatus() {
    const mode = getMode();
    const whitelist = getWhitelist();

    const globalFile = globalSettingsPath();
    const globalExists = fs.existsSync(globalFile);
    const globalData = globalExists ? readJsonFile(globalFile) : null;

    const lines = [
      "Community Cache Status",
      "",
      `GitHub user: ${getCachedUser() || "(not signed in)"}`,
      `Mode: ${mode}`,
      "",
      `Global JSON: ${globalExists ? globalFile : "not found"}`,
      globalData ? `  mode: ${globalData.mode}` : "",
      "",
      `Loaded repos: ${getCachedRepos().length}`,
      `Whitelisted: ${whitelist.length > 0 ? whitelist.join(", ") : "(none)"}`,
    ];

    const folders = vscode.workspace.workspaceFolders;
    if (folders) {
      for (const folder of folders) {
        const wsFile = workspaceSettingsPath(folder);
        const wsExists = fs.existsSync(wsFile);
        const wsData = wsExists ? readJsonFile(wsFile) : null;
        lines.push(
          `Workspace JSON (${folder.name}): ${wsExists ? wsFile : "not found"}`,
        );
        if (wsData) lines.push(`  mode: ${wsData.mode}`);
      }
    }

    vscode.window.showInformationMessage(lines.filter(Boolean).join("\n"), {
      modal: true,
    });
  }

  return {
    MODES,
    getWhitelist,
    getMode,
    setMode,
    setWhitelist,
    syncAllSettings,
    importFromJson,
    showCommunityStatus,
    defaultCommunityRepoFromWorkspace,
    findLocalCommunityCloneFolder,
  };
};
