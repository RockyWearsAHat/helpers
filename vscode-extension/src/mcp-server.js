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
      process.env.HELPERS_NODE_PATH,
      process.env.VSCODE_HELPERS_NODE_PATH,
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

  // Locate the native helpers-native binary; the MCP server is `helpers-native mcp`
  // (Node-free). Honors HELPERS_NATIVE_BIN, then ~/bin, then common locations.
  function findGitShellHelpersMcpPath(context) {
    const homeDir = process.env.HOME || process.env.USERPROFILE || "";
    const exe = process.platform === "win32" ? ".exe" : "";
    const name = `helpers-native${exe}`;
    const candidates = uniquePaths([
      process.env.HELPERS_NATIVE_BIN || "",
      ...(vscode.workspace.workspaceFolders || []).map((f) =>
        path.join(f.uri.fsPath, name),
      ),
      path.join(homeDir, "bin", name),
      path.join("/usr/local/bin", name),
      path.join("/usr/bin", name),
      context.asAbsolutePath(name),
    ].filter(Boolean));

    return candidates.find((candidate) => fs.existsSync(candidate)) || "";
  }

  // Prefer the compiled fast C launcher (helpers-mcp) that sits next to the node
  // server. It proxies to a warm daemon so VS Code pays Node's cold start once
  // instead of on every session start. Returns "" when it hasn't been compiled
  // (no C compiler at install time), in which case we fall back to direct node.
  function findFastLauncher(serverPath) {
    if (!serverPath) return "";
    const shim = path.join(path.dirname(serverPath), "helpers-mcp");
    try {
      if (fs.existsSync(shim) && (fs.statSync(shim).mode & 0o111) !== 0) {
        return shim;
      }
    } catch {
      /* not executable / not present */
    }
    return "";
  }

  function buildGitShellHelpersMcpEnv(serverPath) {
    const serverDir = path.dirname(serverPath);
    // Preserve parent environment (especially PATH) so MCP process launches
    // consistently across app-launch contexts (Dock, Spotlight, shell).
    const env = { ...process.env };

    if (!fs.existsSync(path.join(serverDir, "git-research-mcp"))) {
      env.HELPERS_MCP_DISABLE_RESEARCH = "1";
    }

    if (!fs.existsSync(path.join(serverDir, "vision-tool", "mcp-server.js"))) {
      env.HELPERS_MCP_DISABLE_VISION = "1";
    }

    // Pass current workspace folder paths so the MCP server resolves the
    // correct workspace instead of falling back to __dirname / cwd.
    const roots = (vscode.workspace.workspaceFolders || []).map(
      (f) => f.uri.fsPath,
    );
    if (roots.length > 0) {
      env.HELPERS_WORKSPACE_ROOTS = JSON.stringify(roots);
    }

    // Pass current active chat session URI so session-memory entries are
    // correctly scoped to the originating chat conversation.
    try {
      const sessionUri = vscode.window.activeChatPanelSessionResource;
      if (sessionUri) {
        env.HELPERS_CHAT_SESSION_URI = sessionUri.toString();
      }
    } catch {
      /* proposed API unavailable */
    }

    // Pass session memory enabled setting so the MCP server can gate tools.
    const sessionMemoryEnabled = vscode.workspace
      .getConfiguration("gitShellHelpers.sessionMemory")
      .get("enabled", true);
    if (!sessionMemoryEnabled) {
      env.HELPERS_SESSION_MEMORY_DISABLED = "1";
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
    if (ollamaHost) env.HELPERS_LOCAL_SUBAGENT_OLLAMA_HOST = ollamaHost;
    const ollamaModel = String(
      localSubagents.get("ollama.defaultModel", "") || "",
    ).trim();
    if (ollamaModel) env.HELPERS_LOCAL_SUBAGENT_OLLAMA_MODEL = ollamaModel;
    const ollamaMaxIter = localSubagents.get("ollama.maxIterations", 12);
    if (Number.isFinite(ollamaMaxIter)) {
      env.HELPERS_LOCAL_SUBAGENT_OLLAMA_MAX_ITER = String(ollamaMaxIter);
    }
    const ollamaTimeout = localSubagents.get("ollama.timeoutSeconds", 300);
    if (Number.isFinite(ollamaTimeout)) {
      env.HELPERS_LOCAL_SUBAGENT_OLLAMA_TIMEOUT = String(ollamaTimeout);
    }
    if (localSubagents.get("ollama.allowWrite", false)) {
      env.HELPERS_LOCAL_SUBAGENT_ALLOW_WRITE = "1";
    }
    if (localSubagents.get("ollama.allowShell", false)) {
      env.HELPERS_LOCAL_SUBAGENT_ALLOW_SHELL = "1";
    }
    if (localSubagents.get("fullSystemAccess", false)) {
      env.HELPERS_LOCAL_SUBAGENT_FULL_SYSTEM = "1";
    }
    const systemModel = String(
      localSubagents.get("systemExecute.defaultModel", "") || "",
    ).trim();
    if (systemModel) env.HELPERS_LOCAL_SUBAGENT_SYSTEM_MODEL = systemModel;
    const systemMaxIter = localSubagents.get("systemExecute.maxIterations", 25);
    if (Number.isFinite(systemMaxIter)) {
      env.HELPERS_LOCAL_SUBAGENT_SYSTEM_MAX_ITER = String(systemMaxIter);
    }
    const systemTimeout = localSubagents.get("systemExecute.timeoutSeconds", 900);
    if (Number.isFinite(systemTimeout)) {
      env.HELPERS_LOCAL_SUBAGENT_SYSTEM_TIMEOUT = String(systemTimeout);
    }
    const browserHeadless = localSubagents.get(
      "systemExecute.browserHeadless",
      true,
    );
    env.HELPERS_LOCAL_SUBAGENT_BROWSER_HEADLESS = browserHeadless ? "1" : "0";
    const browserChannel = String(
      localSubagents.get("systemExecute.browserChannel", "chrome") || "",
    ).trim();
    if (browserChannel) {
      env.HELPERS_LOCAL_SUBAGENT_BROWSER_CHANNEL = browserChannel;
    }
    const browserUserDataDir = String(
      localSubagents.get("systemExecute.browserUserDataDir", "") || "",
    ).trim();
    if (browserUserDataDir) {
      env.HELPERS_LOCAL_SUBAGENT_BROWSER_USER_DATA_DIR = browserUserDataDir;
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
    // picks up the updated HELPERS_WORKSPACE_ROOTS environment variable.
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
    // server picks up the updated HELPERS_CHAT_SESSION_URI environment variable.
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

          // The MCP server is the native binary, Node-free (~1ms start).
          const command = serverPath;
          const serverArgs = ["mcp"];

          return [
            new vscode.McpStdioServerDefinition(
              "helpers",
              command,
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
    // Pre-rebrand static registrations to purge so upgraded installs don't keep
    // server entries pointing at binaries that no longer exist (the current
    // "helpers" server is extension-managed and must NOT be removed here).
    const legacyServerNames = ["gsh", "git-shell-helpers", "git-shell-helpers-mcp"];
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
    const server = config?.servers?.["helpers"];
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
          : "Could not locate helpers-server. Reinstall may be needed.",
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
