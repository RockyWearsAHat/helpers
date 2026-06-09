"use strict";
// src/mcp-server.js — MCP server discovery, registration, and configuration
const vscode = require("vscode");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFile } = require("child_process");

module.exports = function createMcpServer(deps) {
  const { GLOBAL_MCP_SERVER_PATH, MCP_PROVIDER_ID, uniquePaths } = deps;

  function _runFile(command, args, options = {}) {
    return new Promise((resolve) => {
      execFile(command, args, options, (error, stdout) => {
        if (error) {
          resolve("");
          return;
        }
        resolve(String(stdout || "").trim());
      });
    });
  }

  function _pickNewestVersionDir(entries) {
    const versions = entries.filter((entry) => /^v\d+\.\d+\.\d+$/.test(entry));
    versions.sort((a, b) => {
      const aParts = a.slice(1).split(".").map(Number);
      const bParts = b.slice(1).split(".").map(Number);
      for (let i = 0; i < 3; i += 1) {
        const delta = (bParts[i] || 0) - (aParts[i] || 0);
        if (delta !== 0) return delta;
      }
      return 0;
    });
    return versions[0] || "";
  }

  async function resolveNodeCommand() {
    const homeDir = process.env.HOME || process.env.USERPROFILE || "";
    const nvmRoot = path.join(homeDir, ".nvm", "versions", "node");
    let newestNvmNode = "";
    try {
      const newest = _pickNewestVersionDir(fs.readdirSync(nvmRoot));
      if (newest) {
        newestNvmNode = path.join(nvmRoot, newest, "bin", "node");
      }
    } catch {
      // nvm not present or unreadable
    }

    const absoluteCandidates = uniquePaths([
      process.env.GSH_NODE_PATH,
      process.env.VSCODE_GSH_NODE_PATH,
      newestNvmNode,
      "/opt/homebrew/bin/node",
      "/usr/local/bin/node",
    ]).filter(Boolean);

    for (const candidate of absoluteCandidates) {
      if (!fs.existsSync(candidate)) {
        continue;
      }
      const version = await _runFile(candidate, ["-v"], { timeout: 1500 });
      if (version.startsWith("v")) {
        return candidate;
      }
    }

    const shell = process.env.SHELL || "/bin/zsh";
    const resolvedFromShell = await _runFile(
      shell,
      ["-lc", "command -v node"],
      { timeout: 2000 },
    );
    if (resolvedFromShell && fs.existsSync(resolvedFromShell)) {
      const version = await _runFile(resolvedFromShell, ["-v"], { timeout: 1500 });
      if (version.startsWith("v")) {
        return resolvedFromShell;
      }
    }

    // Last-resort fallback: use env so command lookup follows PATH.
    // This only runs when all absolute probes fail.
    return "/usr/bin/env";
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
    // Preserve parent environment (especially PATH) so MCP process launches
    // consistently across app-launch contexts (Dock, Spotlight, shell).
    const env = { ...process.env };

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

    // Pass local sub-agent settings (Ollama + system_execute) so the MCP
    // server can configure both the ollama_subagent loop and the
    // system_execute autonomous agent without re-reading VS Code config
    // from the spawned process.
    const localSubagents = vscode.workspace.getConfiguration(
      "gitShellHelpers.localSubagents",
    );
    const ollamaHost = String(
      localSubagents.get("ollama.host", "http://127.0.0.1:11434") || "",
    ).trim();
    if (ollamaHost) env.GSH_LOCAL_SUBAGENT_OLLAMA_HOST = ollamaHost;
    const ollamaModel = String(
      localSubagents.get("ollama.defaultModel", "") || "",
    ).trim();
    if (ollamaModel) env.GSH_LOCAL_SUBAGENT_OLLAMA_MODEL = ollamaModel;
    const ollamaMaxIter = localSubagents.get("ollama.maxIterations", 12);
    if (Number.isFinite(ollamaMaxIter)) {
      env.GSH_LOCAL_SUBAGENT_OLLAMA_MAX_ITER = String(ollamaMaxIter);
    }
    const ollamaTimeout = localSubagents.get("ollama.timeoutSeconds", 300);
    if (Number.isFinite(ollamaTimeout)) {
      env.GSH_LOCAL_SUBAGENT_OLLAMA_TIMEOUT = String(ollamaTimeout);
    }
    if (localSubagents.get("ollama.allowWrite", false)) {
      env.GSH_LOCAL_SUBAGENT_ALLOW_WRITE = "1";
    }
    if (localSubagents.get("ollama.allowShell", false)) {
      env.GSH_LOCAL_SUBAGENT_ALLOW_SHELL = "1";
    }
    if (localSubagents.get("fullSystemAccess", false)) {
      env.GSH_LOCAL_SUBAGENT_FULL_SYSTEM = "1";
    }
    const systemModel = String(
      localSubagents.get("systemExecute.defaultModel", "") || "",
    ).trim();
    if (systemModel) env.GSH_LOCAL_SUBAGENT_SYSTEM_MODEL = systemModel;
    const systemMaxIter = localSubagents.get("systemExecute.maxIterations", 25);
    if (Number.isFinite(systemMaxIter)) {
      env.GSH_LOCAL_SUBAGENT_SYSTEM_MAX_ITER = String(systemMaxIter);
    }
    const systemTimeout = localSubagents.get("systemExecute.timeoutSeconds", 900);
    if (Number.isFinite(systemTimeout)) {
      env.GSH_LOCAL_SUBAGENT_SYSTEM_TIMEOUT = String(systemTimeout);
    }
    const browserHeadless = localSubagents.get(
      "systemExecute.browserHeadless",
      true,
    );
    env.GSH_LOCAL_SUBAGENT_BROWSER_HEADLESS = browserHeadless ? "1" : "0";
    const browserChannel = String(
      localSubagents.get("systemExecute.browserChannel", "chrome") || "",
    ).trim();
    if (browserChannel) {
      env.GSH_LOCAL_SUBAGENT_BROWSER_CHANNEL = browserChannel;
    }
    const browserUserDataDir = String(
      localSubagents.get("systemExecute.browserUserDataDir", "") || "",
    ).trim();
    if (browserUserDataDir) {
      env.GSH_LOCAL_SUBAGENT_BROWSER_USER_DATA_DIR = browserUserDataDir;
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

    // Restart the MCP server when local sub-agent settings change so the
    // server re-reads the Ollama / system_execute configuration from env.
    context.subscriptions.push(
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("gitShellHelpers.localSubagents")) {
          changeEmitter.fire();
        }
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
          const nodeCommand = await resolveNodeCommand();

          const serverArgs =
            nodeCommand === "/usr/bin/env" ? ["node", serverPath] : [serverPath];

          return [
            new vscode.McpStdioServerDefinition(
              "gsh",
              nodeCommand,
              serverArgs,
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
