"use strict";
// src/mcp-server.js — MCP server discovery, registration, and configuration
const vscode = require("vscode");
const crypto = require("crypto");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFile } = require("child_process");

// Must match the same logic in chat-sessions.js _resolveArchiveRoot
function _workspaceArchiveId(workspaceFolderPath) {
  return crypto.createHash("sha1").update(workspaceFolderPath).digest("hex").slice(0, 12);
}

module.exports = function createMcpServer(deps) {
  const { GLOBAL_MCP_SERVER_PATH, MCP_PROVIDER_ID, uniquePaths } = deps;

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

    // Pass current workspace folder paths so the MCP server resolves the
    // correct workspace instead of falling back to __dirname / cwd.
    const roots = (vscode.workspace.workspaceFolders || []).map(
      (f) => f.uri.fsPath,
    );
    if (roots.length > 0) {
      env.GSH_WORKSPACE_ROOTS = JSON.stringify(roots);
    }

    // Pass current active chat session URI so session-memory entries are
    // correctly scoped to the originating chat conversation.
    try {
      const sessionUri = vscode.window.activeChatPanelSessionResource;
      if (sessionUri) {
        env.GSH_CHAT_SESSION_URI = sessionUri.toString();
      }
    } catch {
      /* proposed API unavailable */
    }

    // Pass session memory enabled setting so the MCP server can gate tools.
    const sessionMemoryEnabled = vscode.workspace
      .getConfiguration("gitShellHelpers.sessionMemory")
      .get("enabled", true);
    if (!sessionMemoryEnabled) {
      env.GSH_SESSION_MEMORY_DISABLED = "1";
    }

    // Pass the chat history archive root so MCP tools can search archived
    // chat sessions directly. Uses the same workspace-scoped path as
    // chat-sessions.js so both the extension watcher and the MCP server
    // read/write the same archive tree, even when storageUri falls back
    // to globalStorageUri (which would otherwise mix all projects together).
    const storageRoot =
      deps.getExtensionStorageRoot && deps.getExtensionStorageRoot();
    if (storageRoot) {
      const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
      const wsSlug = workspaceFolder
        ? `ws-${_workspaceArchiveId(workspaceFolder)}`
        : "ws-global";
      // storageRoot + ws-archives/ws-{hash} = per-project root;
      // chat-history-archive.js appends "/chat-history-archive" when initialize() is called,
      // so we pass the parent to match exactly.
      env.GSH_CHAT_ARCHIVE_ROOT = path.join(
        storageRoot,
        "ws-archives",
        wsSlug,
        "chat-history-archive",
      );
      if (workspaceFolder) {
        env.GSH_CHAT_WORKSPACE_PATH = workspaceFolder;
      }
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

    // Restart the MCP server when workspace folders change so the server
    // picks up the updated GSH_WORKSPACE_ROOTS environment variable.
    context.subscriptions.push(
      vscode.workspace.onDidChangeWorkspaceFolders(() => {
        changeEmitter.fire();
      }),
    );

    // Restart the MCP server when the active chat session changes so the
    // server picks up the updated GSH_CHAT_SESSION_URI environment variable.
    // This ensures session-memory entries are scoped to the correct chat.
    try {
      if (vscode.window.onDidChangeActiveChatPanelSessionResource) {
        context.subscriptions.push(
          vscode.window.onDidChangeActiveChatPanelSessionResource(() => {
            changeEmitter.fire();
          }),
        );
      }
    } catch {
      /* proposed API unavailable */
    }

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

  function workspaceMcpConfigPaths() {
    return (vscode.workspace.workspaceFolders || []).map((folder) =>
      path.join(folder.uri.fsPath, ".vscode", "mcp.json"),
    );
  }

  function removeStaticGitShellHelpersServers(configPath) {
    const legacyServerNames = ["gsh", "git-shell-helpers"];
    const config = readJsonFile(configPath);
    if (!config?.servers || typeof config.servers !== "object") {
      return false;
    }

    let changed = false;
    for (const serverName of legacyServerNames) {
      if (config.servers[serverName]) {
        delete config.servers[serverName];
        changed = true;
      }
    }

    if (!changed) {
      return false;
    }

    if (Object.keys(config.servers).length === 0) {
      delete config.servers;
    }

    writeJsonFile(configPath, config);
    return true;
  }

  function migrateLegacyMcpRegistrations() {
    const configPaths = [userMcpConfigPath(), ...workspaceMcpConfigPaths()];
    for (const configPath of configPaths) {
      removeStaticGitShellHelpersServers(configPath);
    }
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
    const resolvedPath = findGitShellHelpersMcpPath(context);
    const binaryExists = resolvedPath ? fs.existsSync(resolvedPath) : false;
    const providerSupported =
      !!vscode.lm?.registerMcpServerDefinitionProvider &&
      typeof vscode.McpStdioServerDefinition === "function";

    if (!binaryExists) {
      return {
        tone: "bad",
        label: "Not found",
        detail: resolvedPath
          ? `Server binary is missing: ${resolvedPath}`
          : "Could not locate git-shell-helpers-mcp. Reinstall may be needed.",
      };
    }

    if (!providerSupported) {
      return {
        tone: "warn",
        label: "Needs trust",
        detail:
          "VS Code MCP provider API unavailable. Start or trust the server from the MCP panel.",
      };
    }

    return {
      tone: "good",
      label: "Ready",
      detail: `Auto-starts when tools are used.\n${resolvedPath}`,
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

  return {
    findGitShellHelpersMcpPath,
    buildGitShellHelpersMcpEnv,
    registerMcpServerProvider,
    globalSettingsPath,
    workspaceSettingsPath,
    workspaceManifestPath,
    readJsonFile,
    writeJsonFile,
    userMcpConfigPath,
    workspaceMcpConfigPaths,
    removeStaticGitShellHelpersServers,
    migrateLegacyMcpRegistrations,
    getConfiguredGitShellHelpersMcpServer,
    getMcpStatusViewModel,
    openMcpServerControls,
  };
};
