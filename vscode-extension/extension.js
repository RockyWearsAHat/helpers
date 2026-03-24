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

const SECTION = "gitShellHelpers.communityCache";
const SCHEMA_VERSION = 1;
const PREDEFINED = {
  baseBranch: "main",
  branchPrefix: "automation/community-cache-submission",
};

let cachedRepos = [];
let cachedUser = "";
let _context = null;
let _webviewProvider = null;

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
  return _context?.globalState.get("mode", "pull-only") ?? "pull-only";
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
  { value: "disabled", label: "Disabled" },
  { value: "pull-only", label: "Pull only — never submit" },
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
    const modeOptions = MODES.map(
      (m) =>
        `<option value="${m.value}"${m.value === mode ? " selected" : ""}>${m.label}</option>`,
    ).join("");

    const modeDescriptions = {
      disabled:
        "Community cache is completely off. Audits will not pull or submit any shared data.",
      "pull-only":
        "All audits pull shared best-practice data from the community cache. Conclusions are never submitted back.",
      "pull-and-auto-submit":
        "All audits pull shared data. Conclusions are submitted back from every repository.",
      "auto-submit-only-public":
        "All audits pull shared data. Conclusions are submitted back only from your public repositories.",
      "auto-submit-whitelist":
        "All audits pull shared data. Conclusions are submitted back only from the repositories you select below.",
    };
    const modeDesc = modeDescriptions[mode] || "";

    let scopeSection = "";
    if (mode === "auto-submit-whitelist") {
      const repoList =
        whitelist.length > 0
          ? whitelist
              .map(
                (r) =>
                  `<div class="repo"><span class="codicon codicon-repo"></span> ${escapeHtml(r)}</div>`,
              )
              .join("")
          : '<div class="muted">No repositories selected — submissions blocked</div>';
      scopeSection = `
        <h3>Whitelisted Repositories</h3>
        ${repoList}
        ${cachedUser ? '<button class="secondary" id="selectReposBtn">Select repositories\u2026</button>' : ""}`;
    } else if (mode === "auto-submit-only-public") {
      const publicCount = cachedRepos.filter(
        (r) => r.visibility === "PUBLIC",
      ).length;
      scopeSection = `
        <h3>Submission Scope</h3>
        <div class="scope-info">Conclusions submitted from your <strong>${publicCount}</strong> public repo${publicCount !== 1 ? "s" : ""}. All repos still pull cache data during audits.</div>`;
    } else if (mode === "pull-and-auto-submit") {
      scopeSection = `
        <h3>Submission Scope</h3>
        <div class="scope-info">Conclusions submitted from <strong>all</strong> repositories.</div>`;
    } else if (mode === "pull-only") {
      scopeSection = `
        <h3>Submission Scope</h3>
        <div class="scope-info">No conclusions submitted. All repos still pull cache data during audits.</div>`;
    }

    const authSection = cachedUser
      ? `<div class="account">
          <span class="codicon codicon-account"></span>
          <span>Signed in as <strong>${escapeHtml(cachedUser)}</strong></span>
        </div>
        <button class="secondary" id="logoutBtn">Sign out</button>`
      : `<div class="muted">Not signed in to GitHub</div>
        <button class="primary" id="loginBtn">Sign in to GitHub</button>`;

    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
  body {
    font-family: var(--vscode-font-family);
    font-size: var(--vscode-font-size);
    color: var(--vscode-foreground);
    padding: 12px 14px;
    margin: 0;
  }
  h3 {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--vscode-descriptionForeground);
    margin: 16px 0 6px;
  }
  h3:first-child { margin-top: 0; }
  select {
    width: 100%;
    padding: 4px 6px;
    border: 1px solid var(--vscode-dropdown-border);
    border-radius: 2px;
    background: var(--vscode-dropdown-background);
    color: var(--vscode-dropdown-foreground);
    font-size: var(--vscode-font-size);
    outline: none;
  }
  select:focus {
    border-color: var(--vscode-focusBorder);
  }
  button {
    display: block;
    width: 100%;
    padding: 6px 14px;
    margin-top: 8px;
    border: none;
    border-radius: 2px;
    font-size: var(--vscode-font-size);
    cursor: pointer;
  }
  button.primary {
    background: var(--vscode-button-background);
    color: var(--vscode-button-foreground);
  }
  button.primary:hover {
    background: var(--vscode-button-hoverBackground);
  }
  button.secondary {
    background: var(--vscode-button-secondaryBackground);
    color: var(--vscode-button-secondaryForeground);
  }
  button.secondary:hover {
    background: var(--vscode-button-secondaryHoverBackground);
  }
  .account {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0;
  }
  .repo {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    font-size: 12px;
  }
  .muted {
    color: var(--vscode-descriptionForeground);
    font-style: italic;
    padding: 4px 0;
  }
  .mode-desc {
    color: var(--vscode-descriptionForeground);
    font-size: 12px;
    padding: 4px 0 0;
    line-height: 1.4;
  }
  .scope-info {
    font-size: 12px;
    padding: 4px 0;
    line-height: 1.4;
  }
  .codicon {
    font-family: codicon;
    font-size: 14px;
  }
  .codicon-account::before { content: "\\eb99"; }
  .codicon-repo::before { content: "\\ea62"; }
</style>
</head>
<body>
  <h3>GitHub Account</h3>
  ${authSection}

  <h3>Mode</h3>
  <select id="modeSelect">${modeOptions}</select>
  <div class="mode-desc">${modeDesc}</div>

  ${scopeSection}

  <script>
    const vscode = acquireVsCodeApi();
    document.getElementById("loginBtn")?.addEventListener("click", () => vscode.postMessage({type:"login"}));
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
// Commands
// ---------------------------------------------------------------------------

async function loginGitHub() {
  if (cachedUser) {
    vscode.window.showInformationMessage(
      `Already signed in to GitHub as ${cachedUser}.`,
    );
    return;
  }

  const terminal = vscode.window.createTerminal("GitHub Sign In");
  terminal.show();
  terminal.sendText("gh auth login");

  const disposable = vscode.window.onDidCloseTerminal(async (t) => {
    if (t === terminal) {
      disposable.dispose();
      if (await isGhAuthed()) {
        cachedUser = await getGhUser();
        cachedRepos = await fetchRepos();
        _webviewProvider?.refresh();
        vscode.window.showInformationMessage(
          `Signed in as ${cachedUser}. ${cachedRepos.length} repos loaded.`,
        );
      }
    }
  });
}

async function logoutGitHub() {
  if (!cachedUser) {
    vscode.window.showInformationMessage("Not currently signed in to GitHub.");
    return;
  }

  const action = await vscode.window.showWarningMessage(
    `Sign out of GitHub account ${cachedUser}?`,
    "Sign out",
    "Cancel",
  );
  if (action !== "Sign out") return;

  const terminal = vscode.window.createTerminal("GitHub Sign Out");
  terminal.show();
  terminal.sendText("gh auth logout --hostname github.com");

  const disposable = vscode.window.onDidCloseTerminal(async (t) => {
    if (t === terminal) {
      disposable.dispose();
      cachedUser = "";
      cachedRepos = [];
      _webviewProvider?.refresh();
      vscode.window.showInformationMessage("Signed out of GitHub.");
    }
  });
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

function activate(context) {
  _context = context;

  importFromJson();

  // Webview panel
  _webviewProvider = new CommunityCacheViewProvider(context.extensionUri);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      CommunityCacheViewProvider.viewType,
      _webviewProvider,
    ),
  );

  // On first activation in this workspace, reveal the panel so users discover it
  const seenKey = "communityCache.introduced";
  if (!context.globalState.get(seenKey)) {
    context.globalState.update(seenKey, true);
    // Small delay lets VS Code finish rendering before we focus the view
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
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
