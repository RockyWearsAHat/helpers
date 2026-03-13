// Git Shell Helpers — VS Code extension
//
// Provides a "Community Cache" tree view in the Explorer sidebar with actual
// buttons for GitHub sign-in/out and repo whitelist selection.  The only
// editable setting in VS Code Settings is the mode dropdown.
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
  communityRepo: "RockyWearsAHat/github-shell-helpers",
  baseBranch: "main",
  branchPrefix: "automation/community-cache-submission",
};

// In-memory cache of fetched repos (populated on activation)
let cachedRepos = [];
let cachedUser = "";
let treeProvider = null;

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
// Settings sync (mode + whitelist → JSON files for shell scripts)
// ---------------------------------------------------------------------------

function getWhitelist() {
  // Whitelist is stored in globalState, not in settings (since the setting is type null)
  return _context?.globalState.get("whitelistedRepos", []) ?? [];
}

async function setWhitelist(repos) {
  await _context?.globalState.update("whitelistedRepos", repos);
  syncAllSettings();
  treeProvider?.refresh();
  updateContextKeys();
}

function updateContextKeys() {
  vscode.commands.executeCommand(
    "setContext",
    "gitShellHelpers:signedIn",
    !!cachedUser,
  );
  vscode.commands.executeCommand(
    "setContext",
    "gitShellHelpers:hasWhitelistedRepos",
    getWhitelist().length > 0,
  );
}

let _context = null;

function buildSettingsJson(config, whitelistedRepos) {
  return {
    schemaVersion: SCHEMA_VERSION,
    ...PREDEFINED,
    mode: config.get("mode") || "pull-only",
    whitelistedRepos,
  };
}

function writeGlobalSettingsFile() {
  const config = vscode.workspace.getConfiguration(SECTION);
  const inspected = config.inspect("mode");
  if (!inspected?.globalValue) return;
  const obj = buildSettingsJson(config, getWhitelist());
  obj.mode = inspected.globalValue;
  writeJsonFile(globalSettingsPath(), obj);
}

function writeWorkspaceSettingsFile(workspaceFolder) {
  const config = vscode.workspace.getConfiguration(SECTION, workspaceFolder);
  const inspected = config.inspect("mode");
  const wsMode = inspected?.workspaceFolderValue ?? inspected?.workspaceValue;
  if (!wsMode) return;
  const obj = buildSettingsJson(config, getWhitelist());
  obj.mode = wsMode;
  writeJsonFile(workspaceSettingsPath(workspaceFolder), obj);
}

function syncAllSettings() {
  writeGlobalSettingsFile();
  const folders = vscode.workspace.workspaceFolders;
  if (folders) {
    for (const folder of folders) {
      writeWorkspaceSettingsFile(folder);
    }
  }
}

function importFromJson() {
  const config = vscode.workspace.getConfiguration(SECTION);
  const globalInspect = config.inspect("mode");
  if (!globalInspect?.globalValue) {
    const globalData = readJsonFile(globalSettingsPath());
    if (globalData?.mode) {
      config.update("mode", globalData.mode, vscode.ConfigurationTarget.Global);
      if (Array.isArray(globalData.whitelistedRepos)) {
        _context?.globalState.update(
          "whitelistedRepos",
          globalData.whitelistedRepos,
        );
      }
    }
  }
  const folders = vscode.workspace.workspaceFolders;
  if (folders) {
    for (const folder of folders) {
      const wsInspect = config.inspect("mode");
      if (!wsInspect?.workspaceFolderValue && !wsInspect?.workspaceValue) {
        const wsData = readJsonFile(workspaceSettingsPath(folder));
        if (wsData?.mode) {
          config.update(
            "mode",
            wsData.mode,
            vscode.ConfigurationTarget.WorkspaceFolder,
          );
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Tree view
// ---------------------------------------------------------------------------

class CommunityTreeProvider {
  constructor() {
    this._onDidChangeTreeData = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._onDidChangeTreeData.event;
  }

  refresh() {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element) {
    return element;
  }

  getChildren() {
    if (!cachedUser) return [];

    const items = [];

    const userItem = new vscode.TreeItem(
      cachedUser,
      vscode.TreeItemCollapsibleState.None,
    );
    userItem.iconPath = new vscode.ThemeIcon("account");
    userItem.description = "GitHub account";
    items.push(userItem);

    const whitelist = getWhitelist();
    if (whitelist.length > 0) {
      for (const repo of whitelist) {
        const item = new vscode.TreeItem(
          repo,
          vscode.TreeItemCollapsibleState.None,
        );
        item.iconPath = new vscode.ThemeIcon("repo");
        items.push(item);
      }
    } else {
      const hint = new vscode.TreeItem(
        "No repos whitelisted",
        vscode.TreeItemCollapsibleState.None,
      );
      hint.description = "use header icons to select";
      hint.iconPath = new vscode.ThemeIcon("info");
      items.push(hint);
    }

    return items;
  }
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
        updateContextKeys();
        treeProvider?.refresh();
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
      updateContextKeys();
      treeProvider?.refresh();
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
  const config = vscode.workspace.getConfiguration(SECTION);
  const inspected = config.inspect("mode");
  const whitelist = getWhitelist();

  const globalFile = globalSettingsPath();
  const globalExists = fs.existsSync(globalFile);
  const globalData = globalExists ? readJsonFile(globalFile) : null;

  const lines = [
    "Community Cache Status",
    "",
    `GitHub user: ${cachedUser || "(not signed in)"}`,
    `User (machine) mode: ${inspected?.globalValue ?? "(not set)"}`,
    `Workspace mode: ${inspected?.workspaceFolderValue ?? inspected?.workspaceValue ?? "(not set)"}`,
    `Effective mode: ${config.get("mode")}`,
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

  // Tree view
  treeProvider = new CommunityTreeProvider();
  const treeView = vscode.window.createTreeView(
    "gitShellHelpers.communityCache",
    { treeDataProvider: treeProvider },
  );
  context.subscriptions.push(treeView);

  // Set initial context keys (not signed in yet)
  updateContextKeys();

  // Auto-detect gh auth and load repos on startup
  isGhAuthed().then(async (authed) => {
    if (authed) {
      cachedUser = await getGhUser();
      cachedRepos = await fetchRepos();
      updateContextKeys();
      treeProvider.refresh();
    }
  });

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration(SECTION)) syncAllSettings();
    }),
  );

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
