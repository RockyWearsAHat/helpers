"use strict";
// src/model-provider.js — Model discovery, Ollama, API keys, agents, quick actions
const vscode = require("vscode");
const fs = require("fs");
const os = require("os");
const path = require("path");

const AVAILABLE_MODELS_PATH = path.join(
  os.homedir(),
  ".copilot",
  "available-models.json",
);
const { execFile } = require("child_process");

const API_KEY_ANTHROPIC = "gsh.apiKey.anthropic";
const API_KEY_OPENAI = "gsh.apiKey.openai";

const QUICK_ACTIONS = [
  {
    id: "runAudit",
    label: "Run Audit",
    desc: "Copilot customization audit",
    query: "/copilot-devops-audit",
    iconPath:
      "M10.5 0a5.5 5.5 0 1 1 0 11 5.5 5.5 0 0 1 0-11zm0 1.5a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM.22 14.78a.75.75 0 0 1 0-1.06l4.5-4.5a.75.75 0 1 1 1.06 1.06l-4.5 4.5a.75.75 0 0 1-1.06 0z",
  },
];

module.exports = function createModelProvider(deps) {
  const { _context, getWebviewProvider } = deps;

  let cachedModels = [];
  let cachedOllamaModels = [];
  let cachedOllamaRunning = false;
  let _ollamaPinned = new Set();
  let _lastAvailableModelsWriteWarningAt = 0;

  function formatErrorMessage(err) {
    return err instanceof Error ? err.message : String(err);
  }

  function warnAvailableModelsWriteFailure(err) {
    const now = Date.now();
    if (now - _lastAvailableModelsWriteWarningAt < 60000) return;
    _lastAvailableModelsWriteWarningAt = now;
    console.warn(
      `[gsh] Failed to write available models cache at ${AVAILABLE_MODELS_PATH}: ${formatErrorMessage(err)}`,
    );
  }

  function initPinnedModels(context) {
    const savedPinned = context.globalState.get("gsh.ollama.pinned", []);
    _ollamaPinned = new Set(Array.isArray(savedPinned) ? savedPinned : []);
  }

  async function refreshModels() {
    try {
      const models = await vscode.lm.selectChatModels({});
      cachedModels = (models || []).map((m) => ({
        id: m.id,
        name: m.name || m.id,
        vendor: m.vendor || "",
        family: m.family || "",
        version: m.version || "",
        maxInputTokens: m.maxInputTokens || 0,
      }));
      const seen = new Set();
      cachedModels = cachedModels.filter((m) => {
        if (seen.has(m.id)) return false;
        seen.add(m.id);
        return true;
      });
      // Write available models so the gsh MCP server can expose list_language_models
      try {
        const dir = path.dirname(AVAILABLE_MODELS_PATH);
        if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
        fs.writeFileSync(
          AVAILABLE_MODELS_PATH,
          JSON.stringify(
            {
              updatedAt: new Date().toISOString(),
              models: cachedModels.map((m) => ({
                id: m.id,
                name: m.name,
                vendor: m.vendor,
                qualifiedName: m.vendor ? `${m.name} (${m.vendor})` : m.name,
                family: m.family,
                maxInputTokens: m.maxInputTokens,
              })),
            },
            null,
            2,
          ),
          "utf8",
        );
      } catch (err) {
        // Non-fatal: MCP tool will fall back gracefully if file absent.
        // Log a throttled warning so failures remain debuggable.
        warnAvailableModelsWriteFailure(err);
      }
    } catch {
      cachedModels = [];
    }
    getWebviewProvider()?.refresh();
  }

  async function openModelPicker() {
    const commands = await vscode.commands.getCommands(true);
    const exactCandidates = [
      "chat.openLanguageModelPicker",
      "github.copilot.chat.openLanguageModelPicker",
      "workbench.action.chat.openLanguageModelPicker",
      "workbench.action.chat.changeDefaultModel",
      "github.copilot.chat.changeModel",
    ];
    const commandId =
      exactCandidates.find((c) => commands.includes(c)) ||
      commands.find(
        (c) =>
          c.toLowerCase().includes("chat") &&
          (c.toLowerCase().includes("model") ||
            c.toLowerCase().includes("language")) &&
          (c.toLowerCase().includes("pick") ||
            c.toLowerCase().includes("select") ||
            c.toLowerCase().includes("change")),
      );
    if (commandId) {
      await vscode.commands.executeCommand(commandId);
      return;
    }
    await vscode.commands.executeCommand(
      "workbench.action.quickOpen",
      ">chat model",
    );
  }

  async function detectOllama() {
    return new Promise((resolve) => {
      const http = require("http");
      const req = http.request(
        {
          hostname: "127.0.0.1",
          port: 11434,
          path: "/api/tags",
          method: "GET",
          timeout: 2000,
        },
        (res) => {
          let raw = "";
          res.on("data", (c) => {
            raw += c;
          });
          res.on("end", () => {
            try {
              const body = JSON.parse(raw);
              const names = (body.models || [])
                .map((m) => m.name || m.model || "")
                .filter(Boolean);
              cachedOllamaRunning = true;
              cachedOllamaModels = names;
            } catch {
              cachedOllamaRunning = true;
              cachedOllamaModels = [];
            }
            resolve();
          });
        },
      );
      req.on("error", () => {
        cachedOllamaRunning = false;
        cachedOllamaModels = [];
        resolve();
      });
      req.on("timeout", () => {
        req.destroy();
        cachedOllamaRunning = false;
        cachedOllamaModels = [];
        resolve();
      });
      req.end();
    });
  }

  async function getApiKey(key) {
    try {
      return (await _context?.secrets.get(key)) || "";
    } catch {
      return "";
    }
  }

  async function setApiKey(key, value) {
    try {
      if (value) await _context?.secrets.store(key, value);
      else await _context?.secrets.delete(key);
      invalidateProviderStatusCache();
    } catch {}
  }

  let _cachedProviderStatus = null;
  let _providerStatusAt = 0;
  const PROVIDER_CACHE_TTL = 5000;

  async function getProviderStatus() {
    const now = Date.now();
    if (_cachedProviderStatus && now - _providerStatusAt < PROVIDER_CACHE_TTL) {
      return _cachedProviderStatus;
    }
    const [anthropicKey, openaiKey] = await Promise.all([
      getApiKey(API_KEY_ANTHROPIC),
      getApiKey(API_KEY_OPENAI),
    ]);
    _cachedProviderStatus = {
      anthropicKey: anthropicKey ? "set" : "",
      openaiKey: openaiKey ? "set" : "",
      ollamaRunning: cachedOllamaRunning,
      ollamaModels: cachedOllamaModels,
    };
    _providerStatusAt = now;
    return _cachedProviderStatus;
  }

  function invalidateProviderStatusCache() {
    _cachedProviderStatus = null;
    _providerStatusAt = 0;
  }

  function parseAgentFrontmatter(content, fileName) {
    if (!content.startsWith("---")) return null;
    const eod = content.indexOf("\n---", 3);
    if (eod === -1) return null;
    const fm = content.slice(3, eod);
    const nameMatch = fm.match(/^name:\s*(.+)$/m);
    const descMatch = fm.match(/^description:\s*(.+)$/m);
    const invocableMatch = fm.match(/^user-invocable:\s*(true|false)\s*/m);
    const name = nameMatch
      ? nameMatch[1].trim().replace(/^["']|["']$/g, "")
      : fileName.replace(".agent.md", "");
    const description = descMatch
      ? descMatch[1].trim().replace(/^["']|["']$/g, "")
      : "";
    const userInvocable = invocableMatch
      ? invocableMatch[1].trim() !== "false"
      : true;
    return { name, description, userInvocable, fileName };
  }

  let _cachedAgents = null;
  let _agentsCacheAt = 0;
  const AGENTS_CACHE_TTL = 5000;

  function scanLocalAgents() {
    const now = Date.now();
    if (_cachedAgents && now - _agentsCacheAt < AGENTS_CACHE_TTL) {
      return _cachedAgents;
    }
    const agents = [];
    const folders = vscode.workspace.workspaceFolders || [];
    for (const folder of folders) {
      const agentsDir = path.join(folder.uri.fsPath, ".github", "agents");
      if (!fs.existsSync(agentsDir)) continue;
      let files;
      try {
        files = fs
          .readdirSync(agentsDir)
          .filter((f) => f.endsWith(".agent.md"));
      } catch {
        continue;
      }
      for (const file of files) {
        try {
          const content = fs.readFileSync(path.join(agentsDir, file), "utf8");
          const agent = parseAgentFrontmatter(content, file);
          if (agent) agents.push(agent);
        } catch {}
      }
    }
    _cachedAgents = agents.sort((a, b) => a.name.localeCompare(b.name));
    _agentsCacheAt = now;
    return _cachedAgents;
  }

  async function openAgentInChat(agentName) {
    if (!agentName) return;
    try {
      const commands = await vscode.commands.getCommands(true);
      const candidates = [
        "workbench.action.chat.open",
        "workbench.panel.chat.view.copilot.focus",
      ];
      const cmd = candidates.find((c) => commands.includes(c));
      if (cmd) {
        await vscode.commands.executeCommand(cmd, {
          query: `@${agentName} `,
        });
        return;
      }
    } catch {}
    await vscode.commands.executeCommand(
      "workbench.action.quickOpen",
      `@${agentName}`,
    );
  }

  async function runQuickAction(actionId) {
    const qa = QUICK_ACTIONS.find((a) => a.id === actionId);
    if (!qa) return;
    try {
      const commands = await vscode.commands.getCommands(true);
      if (commands.includes("workbench.action.chat.open")) {
        await vscode.commands.executeCommand("workbench.action.chat.open", {
          query: qa.query,
        });
        return;
      }
      if (commands.includes("workbench.panel.chat.view.copilot.focus")) {
        await vscode.commands.executeCommand(
          "workbench.panel.chat.view.copilot.focus",
          { query: qa.query },
        );
        return;
      }
    } catch {}
    await vscode.commands.executeCommand(
      "workbench.action.quickOpen",
      qa.query,
    );
  }

  async function openQuickActionWithoutSend(actionId) {
    const qa = QUICK_ACTIONS.find((a) => a.id === actionId);
    if (!qa) return;
    try {
      const commands = await vscode.commands.getCommands(true);
      if (commands.includes("workbench.action.chat.open")) {
        await vscode.commands.executeCommand("workbench.action.chat.open", {
          query: qa.query,
          isPartialQuery: true,
        });
        return;
      }
      if (commands.includes("workbench.panel.chat.view.copilot.focus")) {
        await vscode.commands.executeCommand(
          "workbench.panel.chat.view.copilot.focus",
          { query: qa.query, isPartialQuery: true },
        );
        return;
      }
    } catch {}
    await vscode.env.clipboard.writeText(qa.query);
    vscode.window.showInformationMessage(
      `Copied "${qa.query}" to clipboard — paste it into a new chat.`,
    );
  }

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
          execFile("git", ["config", gitKey, String(value)], { cwd }, () => {});
        }
      }
    }
  }

  function getCachedModels() {
    return cachedModels;
  }

  function getOllamaPinned() {
    return _ollamaPinned;
  }

  return {
    API_KEY_ANTHROPIC,
    API_KEY_OPENAI,
    QUICK_ACTIONS,
    initPinnedModels,
    refreshModels,
    openModelPicker,
    detectOllama,
    getApiKey,
    setApiKey,
    getProviderStatus,
    parseAgentFrontmatter,
    scanLocalAgents,
    openAgentInChat,
    runQuickAction,
    openQuickActionWithoutSend,
    syncCheckpointSettings,
    getCachedModels,
    getOllamaPinned,
  };
};
