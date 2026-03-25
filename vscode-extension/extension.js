// Git Shell Helpers — VS Code extension
//
// Provides a "Community Cache" webview panel in the Explorer sidebar with
// styled buttons for GitHub sign-in/out, mode selection, and repo whitelist.
//
// Settings sync:
//   User settings   → ~/.copilot/devops-audit-community-settings.json
//   Workspace settings → .github/devops-audit-community-settings.json

const vscode = require("vscode");
const fs = require("fs");
const path = require("path");
const { execFile } = require("child_process");

const SCHEMA_VERSION = 1;
const PREDEFINED = {
  baseBranch: "main",
  branchPrefix: "automation/community-cache-submission",
};

let cachedRepos = [];
let cachedUser = "";
let _context = null;
let _webviewProvider = null;
const MCP_PROVIDER_ID = "gitShellHelpers.mcpServers";
const GLOBAL_MCP_SERVER_PATH = "/usr/local/bin/git-shell-helpers-mcp";

function uniquePaths(paths) {
  return [...new Set(paths.filter(Boolean))];
}

function findGitShellHelpersMcpPath(context) {
  const homeDir = process.env.HOME || process.env.USERPROFILE || "";
  const workspaceCandidates = (vscode.workspace.workspaceFolders || []).map(
    (folder) => path.join(folder.uri.fsPath, "git-shell-helpers-mcp"),
  );
  const candidates = uniquePaths([
    ...workspaceCandidates,
    path.join(homeDir, "bin", "git-shell-helpers-mcp"),
    GLOBAL_MCP_SERVER_PATH,
    context.asAbsolutePath("git-shell-helpers-mcp"),
  ]);

  return candidates.find((candidate) => fs.existsSync(candidate)) || "";
}

function buildGitShellHelpersMcpEnv(serverPath) {
  const serverDir = path.dirname(serverPath);
  const env = {};

  if (!fs.existsSync(path.join(serverDir, "git-research-mcp"))) {
    env.GIT_SHELL_HELPERS_MCP_DISABLE_RESEARCH = "1";
  }

  if (!fs.existsSync(path.join(serverDir, "vision-tool", "mcp-server.js"))) {
    env.GIT_SHELL_HELPERS_MCP_DISABLE_VISION = "1";
  }

  return env;
}

function registerMcpServerProvider(context) {
  if (
    !vscode.lm?.registerMcpServerDefinitionProvider ||
    typeof vscode.McpStdioServerDefinition !== "function"
  ) {
    return;
  }

  const changeEmitter = new vscode.EventEmitter();
  context.subscriptions.push(changeEmitter);
  context.subscriptions.push(
    vscode.lm.registerMcpServerDefinitionProvider(MCP_PROVIDER_ID, {
      onDidChangeMcpServerDefinitions: changeEmitter.event,
      provideMcpServerDefinitions: async () => {
        const serverPath = findGitShellHelpersMcpPath(context);
        if (!serverPath) {
          return [];
        }

        return [
          new vscode.McpStdioServerDefinition(
            "gsh",
            "node",
            [serverPath],
            buildGitShellHelpersMcpEnv(serverPath),
            "0.3.4",
          ),
        ];
      },
      resolveMcpServerDefinition: async (server) => server,
    }),
  );
}

// ---------------------------------------------------------------------------
// File helpers
// ---------------------------------------------------------------------------

function globalSettingsPath() {
  return path.join(
    process.env.HOME || process.env.USERPROFILE || "",
    ".copilot",
    "devops-audit-community-settings.json",
  );
}

function workspaceSettingsPath(workspaceFolder) {
  return path.join(
    workspaceFolder.uri.fsPath,
    ".github",
    "devops-audit-community-settings.json",
  );
}

function workspaceManifestPath(workspaceFolder) {
  return path.join(
    workspaceFolder.uri.fsPath,
    "community-cache",
    "manifest.json",
  );
}

function readJsonFile(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
}

function writeJsonFile(filePath, data) {
  const dir = path.dirname(filePath);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + "\n", "utf8");
}

function userMcpConfigPath() {
  const homeDir = process.env.HOME || process.env.USERPROFILE || "";
  if (process.platform === "darwin") {
    return path.join(
      homeDir,
      "Library",
      "Application Support",
      "Code",
      "User",
      "mcp.json",
    );
  }
  if (process.platform === "win32") {
    return path.join(
      process.env.APPDATA || path.join(homeDir, "AppData", "Roaming"),
      "Code",
      "User",
      "mcp.json",
    );
  }
  return path.join(homeDir, ".config", "Code", "User", "mcp.json");
}

function getConfiguredGitShellHelpersMcpServer() {
  const configPath = userMcpConfigPath();
  const config = readJsonFile(configPath);
  const server = config?.servers?.["gsh"];
  const serverPath =
    server?.command === "node" && Array.isArray(server?.args)
      ? server.args[0] || ""
      : "";
  return { configPath, server, serverPath };
}

function getMcpStatusViewModel(context) {
  const { configPath, server, serverPath } =
    getConfiguredGitShellHelpersMcpServer();
  const resolvedPath = serverPath || findGitShellHelpersMcpPath(context);
  const binaryExists = resolvedPath ? fs.existsSync(resolvedPath) : false;
  const providerSupported =
    !!vscode.lm?.registerMcpServerDefinitionProvider &&
    typeof vscode.McpStdioServerDefinition === "function";

  if (!server) {
    return {
      tone: "bad",
      label: "Not registered",
      detail: "Global MCP config does not contain a gsh server entry.",
    };
  }

  if (!binaryExists) {
    return {
      tone: "bad",
      label: "Broken path",
      detail: resolvedPath
        ? `Configured path is missing: ${resolvedPath}`
        : "The gsh server entry does not point to a valid runtime.",
    };
  }

  return {
    tone: providerSupported ? "good" : "warn",
    label: providerSupported ? "Starts on demand" : "Needs start or trust",
    detail: providerSupported
      ? "Server starts on demand via the extension provider."
      : "Server is installed but VS Code needs to start or trust it.",
  };
}

async function openMcpServerControls() {
  const commands = await vscode.commands.getCommands(true);
  const exactCandidates = [
    "mcp.listServers",
    "workbench.action.mcp.listServers",
    "chat.mcp.listServers",
  ];
  const commandId =
    exactCandidates.find((candidate) => commands.includes(candidate)) ||
    commands.find(
      (candidate) =>
        candidate.toLowerCase().includes("mcp") &&
        candidate.toLowerCase().includes("list") &&
        candidate.toLowerCase().includes("server"),
    );

  if (commandId) {
    await vscode.commands.executeCommand(commandId);
    return;
  }

  await vscode.commands.executeCommand(
    "workbench.action.quickOpen",
    ">MCP: List Servers",
  );
}

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

// ---------------------------------------------------------------------------
// gh CLI helpers
// ---------------------------------------------------------------------------

function runGh(args, timeout = 30000) {
  return new Promise((resolve, reject) => {
    execFile("gh", args, { timeout }, (err, stdout, stderr) => {
      if (err) reject(new Error(stderr || err.message));
      else resolve(stdout.trim());
    });
  });
}

async function isGhAuthed() {
  try {
    await runGh(["auth", "status"]);
    return true;
  } catch {
    return false;
  }
}

async function getGhUser() {
  try {
    return (await runGh(["api", "user", "--jq", ".login"])) || "";
  } catch {
    return "";
  }
}

async function fetchRepos() {
  try {
    const out = await runGh([
      "repo",
      "list",
      "--limit",
      "200",
      "--json",
      "nameWithOwner,visibility",
      "--jq",
      '.[] | "\\(.nameWithOwner)|\\(.visibility)"',
    ]);
    if (!out) return [];
    return out
      .split("\n")
      .filter(Boolean)
      .map((line) => {
        const [name, vis] = line.split("|");
        return { nameWithOwner: name, visibility: vis };
      });
  } catch {
    return [];
  }
}

// ---------------------------------------------------------------------------
// Whitelist + settings sync
// ---------------------------------------------------------------------------

function getWhitelist() {
  return _context?.globalState.get("whitelistedRepos", []) ?? [];
}

function getMode() {
  return _context?.globalState.get("mode", "disabled") ?? "disabled";
}

async function setMode(mode) {
  await _context?.globalState.update("mode", mode);
  syncAllSettings();
  _webviewProvider?.refresh();
}

async function setWhitelist(repos) {
  await _context?.globalState.update("whitelistedRepos", repos);
  syncAllSettings();
  _webviewProvider?.refresh();
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
  // Migrate legacy "pull-only" → "disabled"
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

// ---------------------------------------------------------------------------
// Webview panel
// ---------------------------------------------------------------------------

const MODES = [
  { value: "disabled", label: "Submissions disabled" },
  { value: "pull-and-auto-submit", label: "Submit from all repos" },
  { value: "auto-submit-only-public", label: "Submit from public repos only" },
  {
    value: "auto-submit-whitelist",
    label: "Submit from whitelisted repos only",
  },
];

class CommunityCacheViewProvider {
  static viewType = "gitShellHelpers.communityCache";

  constructor(extensionUri) {
    this._extensionUri = extensionUri;
    this._view = null;
  }

  resolveWebviewView(webviewView) {
    this._view = webviewView;
    webviewView.webview.options = { enableScripts: true };
    this._update();

    webviewView.webview.onDidReceiveMessage(async (msg) => {
      switch (msg.type) {
        case "login":
          await loginGitHub();
          break;
        case "logout":
          await logoutGitHub();
          break;
        case "selectRepos":
          await selectRepos();
          break;
        case "setMode":
          await setMode(msg.value);
          break;
        case "toggleGroup":
          setGroupEnabled(msg.key, msg.enabled);
          this._update();
          break;
        case "setCheckpoint": {
          const cpConfig = vscode.workspace.getConfiguration(
            "gitShellHelpers.checkpoint",
          );
          const current = cpConfig.get(msg.key);
          if (msg.key === "sign" && !current) {
            const ok = await ensureGpgKey();
            if (!ok) break;
          }
          await cpConfig.update(
            msg.key,
            !current,
            vscode.ConfigurationTarget.Global,
          );
          this._update();
          break;
        }
        case "openMcpControls":
          await openMcpServerControls();
          break;
      }
    });

    webviewView.onDidChangeVisibility(() => {
      if (webviewView.visible) this._update();
    });
  }

  refresh() {
    this._update();
  }

  _update() {
    if (!this._view) return;
    const mode = getMode();
    const whitelist = getWhitelist();
    this._view.webview.html = this._getHtml(mode, whitelist);
  }

  _getHtml(mode, whitelist) {
    // Gate: require GitHub sign-in
    if (!cachedUser) {
      return `<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: var(--vscode-font-family); color: var(--vscode-foreground);
    display: flex; align-items: center; justify-content: center;
    min-height: 100vh; padding: 32px 20px;
  }
  .gate { text-align: center; max-width: 220px; }
  .gate-icon { width: 40px; height: 40px; margin: 0 auto 16px; opacity: 0.4; }
  .gate-title { font-size: 14px; font-weight: 600; margin-bottom: 6px; }
  .gate-desc {
    font-size: 12px; color: var(--vscode-descriptionForeground);
    line-height: 1.5; margin-bottom: 20px;
  }
  .gate-btn {
    display: flex; align-items: center; justify-content: center; gap: 8px;
    width: 100%; padding: 9px 20px;
    background: var(--vscode-button-background); color: var(--vscode-button-foreground);
    border: none; border-radius: 4px; font-size: 13px; font-weight: 500; cursor: pointer;
  }
  .gate-btn:hover { background: var(--vscode-button-hoverBackground); }
  .gate-btn svg { width: 16px; height: 16px; fill: currentColor; }
</style></head><body>
  <div class="gate">
    <svg class="gate-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
    <div class="gate-title">Git Shell Helpers</div>
    <div class="gate-desc">Sign in to GitHub to configure MCP tools and community cache.</div>
    <button class="gate-btn" id="loginBtn">
      <svg viewBox="0 0 16 16"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
      Sign in with GitHub
    </button>
  </div>
  <script>
    const vscode = acquireVsCodeApi();
    document.getElementById("loginBtn").addEventListener("click", () => vscode.postMessage({type:"login"}));
  </script>
</body></html>`;
    }

    const cpConfig = vscode.workspace.getConfiguration(
      "gitShellHelpers.checkpoint",
    );
    const cpEnabled = cpConfig.get("enabled", true);
    const cpAutoPush = cpConfig.get("autoPush", false);
    const cpSign = cpConfig.get("sign", false);
    const mcpStatus = getMcpStatusViewModel(_context);

    const checkpointItems = [
      {
        key: "enabled",
        label: "Enabled",
        desc: "Enable git-checkpoint in this workspace",
        value: cpEnabled,
      },
      {
        key: "autoPush",
        label: "Auto-Push",
        desc: "Push to remote after every checkpoint commit",
        value: cpAutoPush,
      },
      {
        key: "sign",
        label: "Verified Commits",
        desc: "Sign commits with GPG so GitHub shows a \u2705 Verified badge",
        value: cpSign,
      },
    ];
    const cpRows = checkpointItems
      .map(
        (item) => `
        <div class="tool-item${item.value ? " active" : ""}" data-cpkey="${item.key}">
          <div class="cb${item.value ? " on" : ""}"><div class="cb-tick"></div></div>
          <div class="tool-text">
            <span class="tl">${escapeHtml(item.label)}</span>
            <span class="td">${escapeHtml(item.desc)}</span>
          </div>
        </div>`,
      )
      .join("");

    const toolRows = TOOL_GROUPS.map((group) => {
      const enabled = isGroupEnabled(group.key);
      return `
        <div class="tool-item${enabled ? " active" : ""}" data-key="${group.key}">
          <div class="cb${enabled ? " on" : ""}"><div class="cb-tick"></div></div>
          <div class="tool-text">
            <span class="tl">${escapeHtml(group.label)}</span>
            <span class="td">${escapeHtml(group.description)}</span>
          </div>
        </div>`;
    }).join("");

    const enabledCount = TOOL_GROUPS.filter((g) =>
      isGroupEnabled(g.key),
    ).length;

    const mcpStatusHtml = `
      <div class="mcp-chip ${mcpStatus.tone}" id="manageMcpBtn" title="${escapeHtml(mcpStatus.detail)}">
        <span class="mcp-dot"></span>
        <span class="mcp-chip-label">MCP</span>
        <span class="mcp-chip-status">${escapeHtml(mcpStatus.label)}</span>
      </div>`;

    // --- Community Cache ---
    const modeOptions = MODES.map(
      (m) =>
        `<option value="${m.value}"${m.value === mode ? " selected" : ""}>${m.label}</option>`,
    ).join("");

    const modeDescriptions = {
      disabled:
        "Audits pull shared data from the community cache. No conclusions are submitted back.",
      "pull-and-auto-submit":
        "Audits pull shared data. Conclusions are submitted back from every repository.",
      "auto-submit-only-public":
        "Audits pull shared data. Conclusions are submitted back only from your public repositories.",
      "auto-submit-whitelist":
        "Audits pull shared data. Conclusions are submitted back only from the repositories you select below.",
    };
    const modeDesc = modeDescriptions[mode] || "";

    let scopeSection = "";
    if (mode === "auto-submit-whitelist") {
      const repoList =
        whitelist.length > 0
          ? whitelist
              .map((r) => `<div class="repo-item">${escapeHtml(r)}</div>`)
              .join("")
          : '<div class="muted">No repositories selected</div>';
      scopeSection = `
        <div class="sub-label">Whitelisted Repositories</div>
        ${repoList}
        <button class="btn-secondary" id="selectReposBtn">Select repositories\u2026</button>`;
    } else if (mode === "auto-submit-only-public") {
      const publicCount = cachedRepos.filter(
        (r) => r.visibility === "PUBLIC",
      ).length;
      scopeSection = `
        <div class="sub-label">Scope</div>
        <div class="scope-text">Submitting from <strong>${publicCount}</strong> public repo${publicCount !== 1 ? "s" : ""}.</div>`;
    } else if (mode === "pull-and-auto-submit") {
      scopeSection = `
        <div class="sub-label">Scope</div>
        <div class="scope-text">Submitting from <strong>all</strong> repositories.</div>`;
    } else if (mode === "disabled") {
      scopeSection = `
        <div class="sub-label">Scope</div>
        <div class="scope-text">No submissions. Cache data is still pulled during audits.</div>`;
    }

    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: var(--vscode-font-family);
    font-size: var(--vscode-font-size);
    color: var(--vscode-foreground);
    display: flex; flex-direction: column; min-height: 100vh;
  }

  /* Sections */
  .sect { padding: 12px 14px; }
  .sect + .sect { border-top: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.12)); }
  .sect-head {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 8px;
  }
  .sect-title {
    font-size: 11px; font-weight: 600;
    text-transform: uppercase; letter-spacing: 0.5px;
    color: var(--vscode-descriptionForeground);
  }
  .sect-count {
    font-size: 10px; color: var(--vscode-descriptionForeground); opacity: 0.7;
  }

  /* Tool items — checkbox style */
  .tool-item {
    display: flex; align-items: flex-start; gap: 8px;
    padding: 5px 6px; margin: 0 -6px;
    border-radius: 3px; cursor: pointer; user-select: none;
  }
  .tool-item:hover { background: var(--vscode-list-hoverBackground, rgba(128,128,128,0.08)); }
  .cb {
    flex-shrink: 0; width: 14px; height: 14px; margin-top: 1px;
    border: 1.5px solid var(--vscode-checkbox-border, var(--vscode-input-border, rgba(128,128,128,0.5)));
    border-radius: 3px; position: relative;
    background: var(--vscode-checkbox-background, transparent);
    transition: all 0.15s;
  }
  .cb.on {
    background: var(--vscode-checkbox-selectBackground, var(--vscode-button-background));
    border-color: var(--vscode-checkbox-selectBorder, var(--vscode-button-background));
  }
  .cb-tick {
    position: absolute; left: 2.5px; top: 0.5px;
    width: 5px; height: 9px;
    border: solid var(--vscode-checkbox-foreground, var(--vscode-button-foreground, #fff));
    border-width: 0 1.5px 1.5px 0;
    transform: rotate(45deg);
    opacity: 0; transition: opacity 0.15s;
  }
  .cb.on .cb-tick { opacity: 1; }
  .tool-text { flex: 1; min-width: 0; }
  .tl { display: block; font-size: 12px; font-weight: 500; line-height: 1.3; }
  .td { display: block; font-size: 11px; color: var(--vscode-descriptionForeground); line-height: 1.2; margin-top: 1px; }

  .hint {
    font-size: 10.5px; color: var(--vscode-descriptionForeground);
    margin-top: 6px; opacity: 0.65;
  }

  .mcp-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px 3px 8px;
    border-radius: 999px;
    margin: 0 0 8px;
    cursor: pointer;
    font-size: 11px;
    line-height: 1;
    border: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.16));
    background: var(--vscode-editorWidget-background, rgba(128,128,128,0.04));
    user-select: none;
  }
  .mcp-chip:hover { opacity: 0.85; }
  .mcp-chip.good {
    border-color: color-mix(in srgb, var(--vscode-testing-iconPassed, #2ea043) 35%, var(--vscode-panel-border, rgba(128,128,128,0.16)));
  }
  .mcp-chip.warn {
    border-color: color-mix(in srgb, var(--vscode-inputValidation-warningBorder, #cca700) 40%, var(--vscode-panel-border, rgba(128,128,128,0.16)));
  }
  .mcp-chip.bad {
    border-color: color-mix(in srgb, var(--vscode-inputValidation-errorBorder, #be1100) 45%, var(--vscode-panel-border, rgba(128,128,128,0.16)));
  }
  .mcp-dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    flex-shrink: 0;
    background: var(--vscode-descriptionForeground);
  }
  .mcp-chip.good .mcp-dot { background: var(--vscode-testing-iconPassed, #2ea043); }
  .mcp-chip.warn .mcp-dot { background: var(--vscode-inputValidation-warningBorder, #cca700); }
  .mcp-chip.bad .mcp-dot { background: var(--vscode-inputValidation-errorBorder, #be1100); }
  .mcp-chip-label {
    font-weight: 600;
  }
  .mcp-chip-status {
    color: var(--vscode-descriptionForeground);
  }

  /* Community cache */
  select {
    width: 100%; padding: 4px 8px;
    border: 1px solid var(--vscode-dropdown-border); border-radius: 3px;
    background: var(--vscode-dropdown-background);
    color: var(--vscode-dropdown-foreground);
    font-size: var(--vscode-font-size); outline: none;
  }
  select:focus { border-color: var(--vscode-focusBorder); }
  .mode-desc {
    font-size: 11px; color: var(--vscode-descriptionForeground);
    line-height: 1.4; margin-top: 5px;
  }
  .sub-label {
    font-size: 11px; font-weight: 600; color: var(--vscode-descriptionForeground);
    margin: 10px 0 4px;
  }
  .repo-item {
    font-size: 11.5px; padding: 2px 0; line-height: 1.3;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .scope-text { font-size: 11.5px; line-height: 1.4; }
  .muted { color: var(--vscode-descriptionForeground); font-style: italic; font-size: 11.5px; }
  .btn-secondary {
    display: block; width: 100%; padding: 5px 12px; margin-top: 6px;
    border: none; border-radius: 3px; font-size: 12px; cursor: pointer;
    background: var(--vscode-button-secondaryBackground);
    color: var(--vscode-button-secondaryForeground);
  }
  .btn-secondary:hover { background: var(--vscode-button-secondaryHoverBackground); }

  /* Footer */
  .footer {
    position: sticky; bottom: 0; left: 0; right: 0;
    padding: 8px 14px;
    border-top: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.12));
    background: var(--vscode-sideBar-background, var(--vscode-editor-background));
    display: flex; align-items: center; justify-content: space-between;
    font-size: 11px; color: var(--vscode-descriptionForeground);
  }
  .content { flex: 1; overflow-y: auto; padding-bottom: 36px; }
  .footer-user {
    display: flex; align-items: center; gap: 4px; overflow: hidden;
  }
  .footer-user svg { width: 12px; height: 12px; flex-shrink: 0; opacity: 0.6; fill: currentColor; }
  .footer-user span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .footer-gear {
    flex-shrink: 0; cursor: pointer; opacity: 0.5;
    padding: 2px; transition: opacity 0.15s; display: flex; align-items: center;
  }
  .footer-gear:hover { opacity: 1; }
  .footer-gear svg { width: 14px; height: 14px; fill: currentColor; }
</style>
</head>
<body>
  <div class="content">
    <div class="sect">
      <div class="sect-head">
        <div class="sect-title">MCP Tools</div>
        <div class="sect-count">${enabledCount}/${TOOL_GROUPS.length}</div>
      </div>
      ${mcpStatusHtml}
      ${toolRows}
      <div class="hint">Read &amp; Search Knowledge are always on.</div>
    </div>
    <div class="sect">
      <div class="sect-head">
        <div class="sect-title">Git Checkpoint</div>
      </div>
      ${cpRows}
    </div>
    <div class="sect">
      <div class="sect-head">
        <div class="sect-title">Community Submissions</div>
      </div>
      <select id="modeSelect">${modeOptions}</select>
      <div class="mode-desc">${modeDesc}</div>
      ${scopeSection}
    </div>
  </div>
  <div class="footer">
    <div class="footer-user">
      <svg viewBox="0 0 16 16"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
      <span>${escapeHtml(cachedUser)}</span>
    </div>
    <div class="footer-gear" id="logoutBtn" title="Account settings">
      <svg viewBox="0 0 16 16"><path d="M9.1 4.4L8.6 2H7.4l-.5 2.4-.7.3-2-1.3-.9.8 1.3 2-.2.7-2.4.5v1.2l2.4.5.3.7-1.3 2 .8.8 2-1.3.7.3.5 2.4h1.2l.5-2.4.7-.3 2 1.3.8-.8-1.3-2 .3-.7 2.4-.5V6.8l-2.4-.5-.3-.7 1.3-2-.8-.8-2 1.3-.7-.2zM9.4 8c0 .8-.6 1.4-1.4 1.4S6.6 8.8 6.6 8 7.2 6.6 8 6.6s1.4.6 1.4 1.4z"/></svg>
    </div>
  </div>
  <script>
    const vscode = acquireVsCodeApi();
    document.querySelectorAll('.tool-item').forEach(el => {
      el.addEventListener('click', () => {
        const key = el.dataset.key;
        const active = el.classList.contains('active');
        vscode.postMessage({ type: 'toggleGroup', key, enabled: !active });
      });
    });
    document.querySelectorAll('[data-cpkey]').forEach(el => {
      el.addEventListener('click', () => {
        vscode.postMessage({ type: 'setCheckpoint', key: el.dataset.cpkey });
      });
    });
    document.getElementById("manageMcpBtn")?.addEventListener("click", () => vscode.postMessage({type:"openMcpControls"}));
    document.getElementById("logoutBtn")?.addEventListener("click", () => vscode.postMessage({type:"logout"}));
    document.getElementById("selectReposBtn")?.addEventListener("click", () => vscode.postMessage({type:"selectRepos"}));
    document.getElementById("modeSelect")?.addEventListener("change", (e) => vscode.postMessage({type:"setMode", value: e.target.value}));
  </script>
</body>
</html>`;
  }
}

function escapeHtml(text) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

// ---------------------------------------------------------------------------
// MCP Tools config
// ---------------------------------------------------------------------------

const MCP_TOOLS_CONFIG_DIR = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".config",
  "git-shell-helpers-mcp",
);
const MCP_TOOLS_CONFIG_PATH = path.join(MCP_TOOLS_CONFIG_DIR, "tools.json");

// Tool groups: key → { label, description, tools[], alwaysOn? }
const TOOL_GROUPS = [
  {
    key: "knowledgeWrite",
    label: "Write Reusable Knowledge Locally",
    description: "Write, update & append knowledge notes",
    tools: [
      "write_knowledge_note",
      "update_knowledge_note",
      "append_to_knowledge_note",
    ],
  },
  {
    key: "communityResearch",
    label: "Share Knowledge Research",
    description: "Submit knowledge notes to community repo via PR",
    tools: ["submit_community_research"],
  },
  {
    key: "webSearch",
    label: "Web Search",
    description: "Search the web via SearXNG",
    tools: ["search_web"],
  },
  {
    key: "scrapeWebpage",
    label: "Scrape Webpage",
    description: "Fetch pages, strip HTML chrome, return clean text",
    tools: ["scrape_webpage"],
  },
  {
    key: "vision",
    label: "Vision",
    description:
      "Process images in-chat, allowing live analysis of visual output",
    tools: ["analyze_images"],
  },
  {
    key: "screenshot",
    label: "Screenshot",
    description:
      "Capture screenshots of the screen, an app window, or a region",
    tools: ["take_screenshot"],
  },
  {
    key: "checkpoint",
    label: "Git Checkpoint",
    description: "Commit working state via MCP tool — no terminal, no stalling",
    tools: ["checkpoint"],
  },
];

function readToolsConfig() {
  try {
    return JSON.parse(fs.readFileSync(MCP_TOOLS_CONFIG_PATH, "utf8"));
  } catch {
    return { disabledTools: [] };
  }
}

function writeToolsConfig(config) {
  fs.mkdirSync(MCP_TOOLS_CONFIG_DIR, { recursive: true });
  fs.writeFileSync(
    MCP_TOOLS_CONFIG_PATH,
    JSON.stringify(config, null, 2) + "\n",
    "utf8",
  );
}

function isGroupEnabled(groupKey) {
  const group = TOOL_GROUPS.find((g) => g.key === groupKey);
  if (!group || group.alwaysOn) return true;
  const config = readToolsConfig();
  const disabled = config.disabledTools || [];
  return !group.tools.some((t) => disabled.includes(t));
}

function setGroupEnabled(groupKey, enabled) {
  const group = TOOL_GROUPS.find((g) => g.key === groupKey);
  if (!group || group.alwaysOn) return;
  const config = readToolsConfig();
  const disabled = new Set(config.disabledTools || []);
  for (const tool of group.tools) {
    if (enabled) disabled.delete(tool);
    else disabled.add(tool);
  }
  config.disabledTools = [...disabled];
  writeToolsConfig(config);
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

async function loginGitHub() {
  if (cachedUser) {
    vscode.window.showInformationMessage(`Already signed in as ${cachedUser}.`);
    return;
  }
  try {
    const session = await vscode.authentication.getSession("github", ["repo"], {
      createIfNone: true,
    });
    if (!session) return;

    cachedUser = session.account.label;

    // Forward token to gh CLI for shell script compatibility
    try {
      await new Promise((resolve, reject) => {
        const proc = execFile(
          "gh",
          ["auth", "login", "--with-token"],
          { timeout: 10000 },
          (err) => (err ? reject(err) : resolve()),
        );
        proc.stdin.write(session.accessToken);
        proc.stdin.end();
      });
    } catch {
      /* gh CLI not installed — OK */
    }

    cachedRepos = await fetchRepos();
    _webviewProvider?.refresh();
    syncAllSettings();
  } catch {
    /* User cancelled or auth failed */
  }
}

async function logoutGitHub() {
  if (!cachedUser) return;
  const action = await vscode.window.showWarningMessage(
    `Sign out of GitHub (${cachedUser})?`,
    "Sign out",
    "Cancel",
  );
  if (action !== "Sign out") return;

  try {
    await runGh(["auth", "logout", "--hostname", "github.com", "--yes"]);
  } catch {
    try {
      await new Promise((resolve) => {
        const proc = execFile(
          "gh",
          ["auth", "logout", "--hostname", "github.com"],
          { timeout: 5000 },
          () => resolve(),
        );
        proc.stdin?.write("Y\n");
        proc.stdin?.end();
      });
    } catch {
      /* ignore */
    }
  }

  cachedUser = "";
  cachedRepos = [];
  _webviewProvider?.refresh();
}

async function selectRepos() {
  if (!cachedUser) {
    vscode.window.showWarningMessage("Sign in to GitHub first.");
    return;
  }

  if (cachedRepos.length === 0) {
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "Fetching GitHub repositories…",
      },
      async () => {
        cachedRepos = await fetchRepos();
      },
    );
  }

  if (cachedRepos.length === 0) {
    vscode.window.showWarningMessage("No repositories found.");
    return;
  }

  const currentWhitelist = getWhitelist();
  const items = cachedRepos.map((r) => ({
    label: r.nameWithOwner,
    description: r.visibility === "PUBLIC" ? "public" : "private",
    picked: currentWhitelist.includes(r.nameWithOwner),
  }));

  const selected = await vscode.window.showQuickPick(items, {
    canPickMany: true,
    title: "Select repositories allowed to submit to community cache",
    placeHolder: `${cachedRepos.length} repos — check the ones to whitelist`,
  });

  if (selected) {
    await setWhitelist(selected.map((s) => s.label));
    vscode.window.showInformationMessage(
      `Whitelist updated: ${selected.length} repo(s) selected.`,
    );
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
    `GitHub user: ${cachedUser || "(not signed in)"}`,
    `Mode: ${mode}`,
    "",
    `Global JSON: ${globalExists ? globalFile : "not found"}`,
    globalData ? `  mode: ${globalData.mode}` : "",
    "",
    `Loaded repos: ${cachedRepos.length}`,
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

// ---------------------------------------------------------------------------
// Activation
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// GPG key provisioning for Verified Commits
// ---------------------------------------------------------------------------

function execAsync(cmd, args, opts = {}) {
  return new Promise((resolve, reject) => {
    execFile(cmd, args, { timeout: 30000, ...opts }, (err, stdout, stderr) => {
      if (err) reject(new Error(stderr || err.message));
      else resolve(stdout);
    });
  });
}

function execAsyncStdin(cmd, args, input) {
  return new Promise((resolve, reject) => {
    const proc = execFile(
      cmd,
      args,
      { timeout: 60000 },
      (err, stdout, stderr) => {
        if (err) reject(new Error(stderr || err.message));
        else resolve(stdout);
      },
    );
    proc.stdin.write(input);
    proc.stdin.end();
  });
}

const GPG_CANDIDATES = [
  "gpg",
  "gpg2",
  "/opt/homebrew/bin/gpg",
  "/opt/homebrew/bin/gpg2",
  "/usr/local/bin/gpg",
  "/usr/local/bin/gpg2",
  "/usr/local/MacGPG2/bin/gpg",
  "/usr/local/MacGPG2/bin/gpg2",
];

const BREW_CANDIDATES = [
  "brew",
  "/opt/homebrew/bin/brew",
  "/usr/local/bin/brew",
];

let cachedGpgCommand;
let cachedBrewCommand;

async function resolveGpgCommand() {
  if (cachedGpgCommand) return cachedGpgCommand;

  for (const candidate of GPG_CANDIDATES) {
    if (candidate.includes(path.sep) && !fs.existsSync(candidate)) {
      continue;
    }

    try {
      await execAsync(candidate, ["--version"]);
      cachedGpgCommand = candidate;
      return candidate;
    } catch {
      /* try next candidate */
    }
  }

  return "";
}

async function resolveBrewCommand() {
  if (cachedBrewCommand) return cachedBrewCommand;

  for (const candidate of BREW_CANDIDATES) {
    if (candidate.includes(path.sep) && !fs.existsSync(candidate)) {
      continue;
    }

    try {
      await execAsync(candidate, ["--version"]);
      cachedBrewCommand = candidate;
      return candidate;
    } catch {
      /* try next candidate */
    }
  }

  return "";
}

async function installGpgWithBrew() {
  if (process.platform !== "darwin") return false;

  const brewCommand = await resolveBrewCommand();
  if (!brewCommand) {
    vscode.window.showErrorMessage(
      "Verified Commits requires GPG, and Homebrew is not installed. Install Homebrew first, then try again.",
    );
    return false;
  }

  const choice = await vscode.window.showWarningMessage(
    "Verified Commits requires GPG. Install GnuPG with Homebrew now?",
    { modal: true },
    "Install",
    "Cancel",
  );
  if (choice !== "Install") return false;

  const installed = await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Installing GnuPG with Homebrew…",
      cancellable: false,
    },
    async () => {
      try {
        await execAsync(brewCommand, ["install", "gnupg"], {
          timeout: 10 * 60 * 1000,
          maxBuffer: 10 * 1024 * 1024,
        });
        return true;
      } catch (err) {
        vscode.window.showErrorMessage(
          `Failed to install GnuPG with Homebrew: ${err.message}`,
        );
        return false;
      }
    },
  );

  if (!installed) return false;

  cachedGpgCommand = undefined;
  const gpgCommand = await resolveGpgCommand();
  if (!gpgCommand) {
    vscode.window.showErrorMessage(
      "GnuPG installed, but VS Code still could not find 'gpg'. Reload the window and try again.",
    );
    return false;
  }

  vscode.window.showInformationMessage(
    "GnuPG installed. Continuing Verified Commits setup.",
  );
  return true;
}

async function ensureGpgAvailable() {
  let gpgCommand = await resolveGpgCommand();
  if (gpgCommand) return gpgCommand;

  if (process.platform === "darwin") {
    const installed = await installGpgWithBrew();
    if (!installed) return "";
    gpgCommand = await resolveGpgCommand();
    if (gpgCommand) return gpgCommand;
  }

  vscode.window.showErrorMessage(
    "Verified Commits requires GPG, but no gpg executable was found.",
  );
  return "";
}

async function ensureGpgKey() {
  const gpgCommand = await ensureGpgAvailable();
  if (!gpgCommand) {
    return false;
  }

  // 1. Check if a signing key is already configured
  try {
    const existing = (
      await execAsync("git", ["config", "--global", "user.signingkey"])
    ).trim();
    if (existing) return true; // already set up
  } catch {
    /* not configured yet */
  }

  // 2. Check for existing secret keys we can reuse
  let email = "";
  try {
    email = (
      await execAsync("git", ["config", "--global", "user.email"])
    ).trim();
  } catch {
    /* no email configured */
  }

  if (email) {
    try {
      const keys = await execAsync(gpgCommand, [
        "--list-secret-keys",
        "--keyid-format",
        "long",
        email,
      ]);
      const match = keys.match(/sec\s+\w+\/([A-F0-9]+)/i);
      if (match) {
        await execAsync("git", [
          "config",
          "--global",
          "user.signingkey",
          match[1],
        ]);
        vscode.window.showInformationMessage(
          `Using existing GPG key ${match[1].slice(-8)} for signing.`,
        );
        await uploadGpgKeyToGitHub(match[1], gpgCommand);
        return true;
      }
    } catch {
      /* no matching key */
    }
  }

  // 3. Need git user.name and user.email to generate a key
  let name = "";
  try {
    name = (await execAsync("git", ["config", "--global", "user.name"])).trim();
  } catch {
    /* no name configured */
  }

  if (!name || !email) {
    vscode.window.showErrorMessage(
      "Set git user.name and user.email before enabling Verified Commits.\n" +
        "Run: git config --global user.name 'Your Name' && git config --global user.email 'you@example.com'",
    );
    return false;
  }

  // 4. Generate a new GPG key
  const progress = await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Generating GPG key…",
    },
    async () => {
      try {
        const batch = [
          "Key-Type: RSA",
          "Key-Length: 4096",
          `Name-Real: ${name}`,
          `Name-Email: ${email}`,
          "Expire-Date: 0",
          "%no-protection",
          "%commit",
          "",
        ].join("\n");
        await execAsyncStdin(gpgCommand, ["--batch", "--gen-key"], batch);
        return true;
      } catch (err) {
        vscode.window.showErrorMessage(
          `Failed to generate GPG key: ${err.message}`,
        );
        return false;
      }
    },
  );

  if (!progress) return false;

  // 5. Read back the new key ID
  let keyId = "";
  try {
    const keys = await execAsync(gpgCommand, [
      "--list-secret-keys",
      "--keyid-format",
      "long",
      email,
    ]);
    const match = keys.match(/sec\s+\w+\/([A-F0-9]+)/i);
    if (match) keyId = match[1];
  } catch {
    /* failed to read key */
  }

  if (!keyId) {
    vscode.window.showErrorMessage(
      "GPG key was generated but could not read the key ID.",
    );
    return false;
  }

  // 6. Configure git to use this key
  await execAsync("git", ["config", "--global", "user.signingkey", keyId]);

  // 7. Upload to GitHub
  const uploaded = await uploadGpgKeyToGitHub(keyId, gpgCommand);
  const suffix = uploaded
    ? " and uploaded to GitHub ✅"
    : " (upload to GitHub manually for the Verified badge)";
  vscode.window.showInformationMessage(
    `GPG key ${keyId.slice(-8)} generated${suffix}`,
  );

  return true;
}

async function uploadGpgKeyToGitHub(keyId, gpgCommand) {
  if (!cachedUser) return false;
  try {
    const pubKey = await execAsync(gpgCommand, ["--armor", "--export", keyId]);
    if (!pubKey.trim()) return false;
    await runGh([
      "api",
      "/user/gpg_keys",
      "-f",
      `armored_public_key=${pubKey.trim()}`,
    ]);
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Checkpoint settings → git config sync
// ---------------------------------------------------------------------------

function syncCheckpointSettings() {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) return;

  const config = vscode.workspace.getConfiguration(
    "gitShellHelpers.checkpoint",
  );
  const keys = [
    { setting: "enabled", gitKey: "checkpoint.enabled" },
    { setting: "autoPush", gitKey: "checkpoint.push" },
    { setting: "sign", gitKey: "checkpoint.sign" },
  ];

  for (const folder of folders) {
    const cwd = folder.uri.fsPath;
    for (const { setting, gitKey } of keys) {
      const value = config.get(setting);
      if (value !== undefined) {
        execFile("git", ["config", gitKey, String(value)], { cwd }, (err) => {
          if (err) {
            // Not a git repo or git not available — ignore silently
          }
        });
      }
    }
  }
}

function activate(context) {
  _context = context;

  importFromJson();
  registerMcpServerProvider(context);

  // Git Helpers webview (MCP Tools + Community Cache)
  _webviewProvider = new CommunityCacheViewProvider(context.extensionUri);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      CommunityCacheViewProvider.viewType,
      _webviewProvider,
    ),
  );

  // On first activation, focus the Git Helpers panel so users discover it
  const seenKey = "gitHelpers.introduced.v3";
  if (!context.globalState.get(seenKey)) {
    context.globalState.update(seenKey, true);
    setTimeout(() => {
      vscode.commands.executeCommand("gitShellHelpers.communityCache.focus");
    }, 800);
  }

  // Auto-detect gh auth on startup
  isGhAuthed().then(async (authed) => {
    if (authed) {
      cachedUser = await getGhUser();
      cachedRepos = await fetchRepos();
      _webviewProvider.refresh();
    }
  });

  // Write default tools config if none exists
  if (!fs.existsSync(MCP_TOOLS_CONFIG_PATH)) {
    writeToolsConfig({ disabledTools: [] });
  }

  context.subscriptions.push(
    vscode.commands.registerCommand(
      "gitShellHelpers.showCommunityStatus",
      showCommunityStatus,
    ),
    vscode.commands.registerCommand("gitShellHelpers.loginGitHub", loginGitHub),
    vscode.commands.registerCommand(
      "gitShellHelpers.logoutGitHub",
      logoutGitHub,
    ),
    vscode.commands.registerCommand("gitShellHelpers.selectRepos", selectRepos),
    vscode.commands.registerCommand(
      "gitShellHelpers.restartMcpServer",
      async () => {
        const choice = await vscode.window.showInformationMessage(
          "Reload the window now to restart MCP servers and refresh chat tools?",
          "Reload Window",
          "Cancel",
        );
        if (choice === "Reload Window") {
          await vscode.commands.executeCommand("workbench.action.reloadWindow");
        }
      },
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.openMcpServerControls",
      openMcpServerControls,
    ),
  );

  // Sync checkpoint settings to git config when changed
  syncCheckpointSettings();
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("gitShellHelpers.checkpoint")) {
        syncCheckpointSettings();
      }
    }),
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
