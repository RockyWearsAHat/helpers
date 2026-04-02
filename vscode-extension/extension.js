// Git Shell Helpers — VS Code extension
//
// Thin entry point: initializes extracted modules and wires dependencies.
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

// Module imports
const createWebviewProviderClass = require("./src/webview-provider");
const createCopilotInspector = require("./src/copilot-inspector");
const createGpgAuth = require("./src/gpg-auth");
const createMcpServer = require("./src/mcp-server");
const createCommunitySettings = require("./src/community-settings");
const createActivityTracker = require("./src/activity-tracker");
const createChatSessions = require("./src/chat-sessions");
const createModelProvider = require("./src/model-provider");
const createWorktreeManager = require("./src/worktree-manager");
const createIpcServers = require("./src/ipc-servers");
const toolsConfig = require("./src/tools-config");
const createFormatControl = require("./src/format-control");

// Constants (used by mcp-server and gpg-auth modules)
const SCHEMA_VERSION = 1;
const PREDEFINED = {
  baseBranch: "main",
  branchPrefix: "automation/community-cache-submission",
};
const MCP_PROVIDER_ID = "gitShellHelpers.mcpServers";
const GLOBAL_MCP_SERVER_PATH = "/usr/local/bin/git-shell-helpers-mcp";

// Shared mutable state — owned by entry point, accessed by modules via closures
let _context = null;
let _webviewProvider = null;
let _diagnosticsOutputChannel = null;
let _customizationInspectorToolDisposable = null;
let cachedUser = "";
let cachedRepos = [];
let cachedGpgNeedsUpload = false;
let cachedGpgUploadFailed = false;
let _ipc = null;
let _formatControl = null;

function escapeHtml(text) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function getWebviewProvider() {
  return _webviewProvider;
}

// ---------------------------------------------------------------------------
// Minimal gh CLI helpers (shared by community-settings and gpg-auth)
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
// activate
// ---------------------------------------------------------------------------

function activate(context) {
  _context = context;

  // --- Initialize modules in dependency order ---

  // 1. MCP server (no deps on other modules)
  const mcpServer = createMcpServer({
    GLOBAL_MCP_SERVER_PATH,
    MCP_PROVIDER_ID,
    uniquePaths: (paths) => [...new Set(paths.filter(Boolean))],
    getExtensionStorageRoot: () =>
      context.storageUri?.fsPath || context.globalStorageUri?.fsPath || null,
  });

  // 2. Activity tracker (needs getWebviewProvider, getChatSessions — late-bound)
  const activity = createActivityTracker({
    getWebviewProvider,
    getChatSessions: () => chatSessionsModule.getChatSessions(),
  });

  // 3. Chat sessions (needs getWebviewProvider, getActivityItems)
  const chatSessionsModule = createChatSessions({
    getWebviewProvider,
    getActivityItems: () => activity.getActivityItems(),
  });

  // 4. Copilot inspector (needs channel/disposable getters + activity)
  const inspector = createCopilotInspector({
    getDiagnosticsChannel: () => _diagnosticsOutputChannel,
    setDiagnosticsChannel: (ch) => {
      _diagnosticsOutputChannel = ch;
    },
    getInspectorDisposable: () => _customizationInspectorToolDisposable,
    setInspectorDisposable: (d) => {
      _customizationInspectorToolDisposable = d;
    },
    beginToolCall: activity.beginToolCall,
    endToolCall: activity.endToolCall,
  });

  // 5. Community settings (needs mcpServer funcs, gh helpers)
  //    checkGpgUploadStatus is destructured but unused — pass no-op for safety
  const community = createCommunitySettings({
    _context,
    getCachedUser: () => cachedUser,
    getCachedRepos: () => cachedRepos,
    setCachedRepos: (v) => {
      cachedRepos = v;
    },
    runGh,
    isGhAuthed,
    getGhUser,
    fetchRepos,
    checkGpgUploadStatus: (...a) => gpgAuth.checkGpgUploadStatus(...a),
    readJsonFile: mcpServer.readJsonFile,
    writeJsonFile: mcpServer.writeJsonFile,
    globalSettingsPath: mcpServer.globalSettingsPath,
    workspaceSettingsPath: mcpServer.workspaceSettingsPath,
    workspaceManifestPath: mcpServer.workspaceManifestPath,
    isGroupEnabled: toolsConfig.isGroupEnabled,
    getWebviewProvider,
  });

  // 6. GPG auth (needs community, mcpServer, gh helpers, state)
  //    buildSettingsJson is destructured but unused — pass no-op for safety
  const gpgAuth = createGpgAuth({
    getCachedRepos: () => cachedRepos,
    setCachedRepos: (v) => {
      cachedRepos = v;
    },
    getCachedUser: () => cachedUser,
    setCachedUser: (v) => {
      cachedUser = v;
    },
    getCachedGpgNeedsUpload: () => cachedGpgNeedsUpload,
    setCachedGpgNeedsUpload: (v) => {
      cachedGpgNeedsUpload = v;
    },
    getCachedGpgUploadFailed: () => cachedGpgUploadFailed,
    setCachedGpgUploadFailed: (v) => {
      cachedGpgUploadFailed = v;
    },
    getWebviewProvider,
    runGh,
    isGhAuthed,
    getGhUser,
    fetchRepos,
    getWhitelist: community.getWhitelist,
    setWhitelist: community.setWhitelist,
    getMode: community.getMode,
    buildSettingsJson: () => ({}),
    syncAllSettings: community.syncAllSettings,
    readJsonFile: mcpServer.readJsonFile,
    writeJsonFile: mcpServer.writeJsonFile,
    globalSettingsPath: mcpServer.globalSettingsPath,
    workspaceSettingsPath: mcpServer.workspaceSettingsPath,
    SCHEMA_VERSION,
    PREDEFINED,
  });

  // 7. Model provider
  const models = createModelProvider({
    _context,
    getWebviewProvider,
  });

  // 8. Worktree manager
  const worktree = createWorktreeManager({
    _context,
    getDiagnosticsOutputChannel: inspector.getDiagnosticsOutputChannel,
  });

  // 9. IPC servers
  const ipc = createIpcServers({
    beginToolCall: activity.beginToolCall,
    endToolCall: activity.endToolCall,
    runStrictLinting: inspector.runStrictLinting,
    getActivityItems: activity.getActivityItems,
    getWebviewProvider,
    handleWorktreeIpcMessage: worktree.handleWorktreeIpcMessage,
    getActiveChatTabKey: worktree.getActiveChatTabKey,
    getPendingBranchSessionStarts: worktree.getPendingBranchSessionStarts,
    setSuppressTabDrivenUnfocusUntil: worktree.setSuppressTabDrivenUnfocusUntil,
    ensureSessionStarted: activity.ensureSessionStarted,
    writeWorktreeDebug: worktree.writeWorktreeDebug,
  });
  _ipc = ipc;

  // --- Startup sequence ---

  // Check VS Code workbench patches (non-blocking, deferred)
  setTimeout(() => worktree.checkVscodePatches(), 3000);

  // Restore persisted Ollama pinned models
  models.initPinnedModels(context);

  // Import settings, migrate MCP, register providers
  community.importFromJson();
  mcpServer.migrateLegacyMcpRegistrations();
  mcpServer.registerMcpServerProvider(context);
  inspector.registerCustomizationInspectorTool(context);

  // --- Webview provider ---
  const CommunityCacheViewProvider = createWebviewProviderClass({
    loginGitHub: gpgAuth.loginGitHub,
    logoutGitHub: gpgAuth.logoutGitHub,
    selectRepos: gpgAuth.selectRepos,
    setMode: community.setMode,
    setGroupEnabled: toolsConfig.setGroupEnabled,
    ensureGpgKey: gpgAuth.ensureGpgKey,
    openMcpServerControls: mcpServer.openMcpServerControls,
    openModelPicker: models.openModelPicker,
    refreshModels: models.refreshModels,
    openAgentInChat: models.openAgentInChat,
    runQuickAction: models.runQuickAction,
    openQuickActionWithoutSend: models.openQuickActionWithoutSend,
    setApiKey: models.setApiKey,
    detectOllama: models.detectOllama,
    uploadGpgKeyNow: gpgAuth.uploadGpgKeyNow,
    getMode: community.getMode,
    getWhitelist: community.getWhitelist,
    getMcpStatusViewModel: mcpServer.getMcpStatusViewModel,
    escapeHtml,
    isGroupEnabled: toolsConfig.isGroupEnabled,
    isStrictLintingEnabled: inspector.isCustomizationInspectorEnabled,
    getProviderStatus: models.getProviderStatus,
    scanLocalAgents: models.scanLocalAgents,
    getActivityItems: activity.getActivityItems,
    _activityCountLabel: activity._activityCountLabel,
    API_KEY_ANTHROPIC: models.API_KEY_ANTHROPIC,
    API_KEY_OPENAI: models.API_KEY_OPENAI,
    TOOL_GROUPS: toolsConfig.TOOL_GROUPS,
    MODES: community.MODES,
    QUICK_ACTIONS: models.QUICK_ACTIONS,
    getCachedUser: () => cachedUser,
    setCachedUser: (v) => {
      cachedUser = v;
    },
    getCachedRepos: () => cachedRepos,
    setCachedRepos: (v) => {
      cachedRepos = v;
    },
    getCachedGpgNeedsUpload: () => cachedGpgNeedsUpload,
    getCachedGpgUploadFailed: () => cachedGpgUploadFailed,
    setCachedGpgUploadFailed: (v) => {
      cachedGpgUploadFailed = v;
    },
    _ollamaPinned: models.getOllamaPinned(),
    _context,
  });
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
      await gpgAuth.checkGpgUploadStatus();
      _webviewProvider.refresh();
    }
  });

  // Detect Ollama on startup
  models.detectOllama();

  // Load available Copilot models on startup and whenever the model list changes
  models.refreshModels();
  if (vscode.lm?.onDidChangeChatModels) {
    context.subscriptions.push(
      vscode.lm.onDidChangeChatModels(() => models.refreshModels()),
    );
  }

  // Start IPC servers
  ipc.startStrictLintIpcServer();
  ipc.startActivityIpcServer();

  // Worktree management
  worktree.loadWorktreeBindings();
  worktree.loadTabWorktreeMap();
  worktree.loadSessionState();
  worktree.reconcileWorktreeBindings();
  worktree.registerWorktreeFileView(context);

  // Track chat editor tabs — switch explorer focus to the active worktree
  context.subscriptions.push(
    vscode.window.tabGroups.onDidChangeTabs(() =>
      worktree.onActiveTabChanged(),
    ),
    vscode.window.tabGroups.onDidChangeTabGroups(() =>
      worktree.onActiveTabChanged(),
    ),
  );

  // Track active chat session via proposed API (chatParticipantPrivate)
  try {
    if (vscode.window.onDidChangeActiveChatPanelSessionResource) {
      context.subscriptions.push(
        vscode.window.onDidChangeActiveChatPanelSessionResource(
          worktree.onChatSessionFocusChanged,
        ),
      );
    }
  } catch {}

  // Restore worktree focus if VS Code reopened with an active branch session
  worktree.waitForGitExtensionThenRestore();

  // Watch Copilot Chat's JSONL session files for live activity
  chatSessionsModule.startChatSessionWatcher(context);
  context.subscriptions.push({
    dispose: () => chatSessionsModule.dispose(),
  });

  // Format control — suppress formatters during agent saves
  _formatControl = createFormatControl();
  _formatControl.activate(context);

  // Write default tools config if none exists
  if (!fs.existsSync(toolsConfig.MCP_TOOLS_CONFIG_PATH)) {
    toolsConfig.writeToolsConfig({ disabledTools: [] });
  }

  // --- Command registrations ---
  context.subscriptions.push(
    vscode.commands.registerCommand(
      "gitShellHelpers.showCommunityStatus",
      community.showCommunityStatus,
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.inspectCopilotCustomizationWarnings",
      async (filePath) => {
        const result = await inspector.inspectCopilotCustomizationWarnings({
          filePath,
          notify: true,
          revealOutput: true,
        });
        return inspector.formatCustomizationInspectionReport(result);
      },
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.searchArchivedChatHistory",
      () => chatSessionsModule.searchArchivedChatHistory(),
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.loginGitHub",
      gpgAuth.loginGitHub,
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.logoutGitHub",
      gpgAuth.logoutGitHub,
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.selectRepos",
      gpgAuth.selectRepos,
    ),
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
      mcpServer.openMcpServerControls,
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.refreshModels",
      async () => {
        await models.refreshModels();
        vscode.window.showInformationMessage(
          `Git Shell Helpers: ${models.getCachedModels().length} Copilot model(s) found.`,
        );
      },
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.openModelPicker",
      models.openModelPicker,
    ),
  );

  // Sync checkpoint settings to git config when changed
  models.syncCheckpointSettings();
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("gitShellHelpers.checkpoint")) {
        models.syncCheckpointSettings();
      }
      if (e.affectsConfiguration("gitShellHelpers.customizationInspector")) {
        inspector.registerCustomizationInspectorTool(context);
      }
    }),
  );
}

async function deactivate() {
  if (_formatControl) {
    await _formatControl.deactivate();
  }
  if (_ipc) {
    _ipc.stopStrictLintIpcServer();
    _ipc.stopActivityIpcServer();
  }
}

module.exports = { activate, deactivate };
