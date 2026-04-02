"use strict";
// src/webview-provider.js — Community Cache webview panel for Explorer sidebar
const vscode = require("vscode");

module.exports = function createWebviewProviderClass(deps) {
  const {
    // Functions
    loginGitHub,
    logoutGitHub,
    selectRepos,
    setMode,
    setGroupEnabled,
    ensureGpgKey,
    openMcpServerControls,
    openModelPicker,
    refreshModels,
    openAgentInChat,
    runQuickAction,
    openQuickActionWithoutSend,
    setApiKey,
    detectOllama,
    uploadGpgKeyNow,
    getMode,
    getWhitelist,
    getMcpStatusViewModel,
    escapeHtml,
    isGroupEnabled,
    isStrictLintingEnabled,
    getProviderStatus,
    scanLocalAgents,
    getActivityItems,
    _activityCountLabel,
    // Constants
    API_KEY_ANTHROPIC,
    API_KEY_OPENAI,
    TOOL_GROUPS,
    MODES,
    QUICK_ACTIONS,
    // Mutable state accessors
    getCachedUser,
    setCachedUser,
    getCachedRepos,
    setCachedRepos,
    getCachedGpgNeedsUpload,
    getCachedGpgUploadFailed,
    setCachedGpgUploadFailed,
  } = deps;

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
          case "openChatSession": {
            let opened = false;
            const sessionId = msg.sessionId;
            if (sessionId) {
              // Try to open the specific session via its URI
              try {
                const sessionUri = vscode.Uri.parse(
                  `vscode-chat-session://local/${sessionId}`,
                );
                await vscode.window.showTextDocument(sessionUri, {
                  preview: false,
                  preserveFocus: false,
                });
                opened = true;
              } catch {
                // Fallback: try the chat.open command with sessionId
                try {
                  await vscode.commands.executeCommand(
                    "workbench.action.chat.open",
                    { sessionId },
                  );
                  opened = true;
                } catch {
                  // ignore
                }
              }
            }
            if (!opened) {
              vscode.commands.executeCommand("workbench.action.chat.open");
            }
            break;
          }
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
          case "toggleStrictLinting":
            await vscode.workspace
              .getConfiguration("gitShellHelpers.customizationInspector")
              .update(
                "enabled",
                msg.enabled,
                vscode.ConfigurationTarget.Global,
              );
            this._update();
            break;
          case "toggleBranchSessions":
            await vscode.workspace
              .getConfiguration("gitShellHelpers.branchSessions")
              .update(
                "enabled",
                msg.enabled,
                vscode.ConfigurationTarget.Global,
              );
            this._update();
            break;
          case "toggleSessionMemory":
            await vscode.workspace
              .getConfiguration("gitShellHelpers.sessionMemory")
              .update(
                "enabled",
                msg.enabled,
                vscode.ConfigurationTarget.Global,
              );
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
          case "openModelPicker":
            await openModelPicker();
            break;
          case "refreshModels":
            await refreshModels();
            break;
          case "openAgent":
            await openAgentInChat(msg.name || "");
            break;
          case "runQuickAction":
            await runQuickAction(msg.action || "");
            break;
          case "openQuickActionWithoutSend":
            await openQuickActionWithoutSend(msg.action || "");
            break;
          case "saveApiKey": {
            const keyId =
              msg.provider === "anthropic" ? API_KEY_ANTHROPIC : API_KEY_OPENAI;
            const val = String(msg.value || "").trim();
            await setApiKey(keyId, val);
            vscode.window.showInformationMessage(
              val
                ? `${msg.provider === "anthropic" ? "Anthropic" : "OpenAI"} API key saved.`
                : `${msg.provider === "anthropic" ? "Anthropic" : "OpenAI"} API key cleared.`,
            );
            this._update();
            break;
          }
          case "refreshOllama":
            await detectOllama();
            this._update();
            break;
          case "ollamaToggle": {
            const m = String(msg.model || "").trim();
            if (!m) break;
            if (deps._ollamaPinned.has(m)) {
              deps._ollamaPinned.delete(m);
            } else {
              deps._ollamaPinned.add(m);
            }
            deps._context.globalState.update("gsh.ollama.pinned", [
              ...deps._ollamaPinned,
            ]);
            this._update();
            break;
          }
          case "ollamaRun": {
            const model = String(msg.model || "").trim();
            if (!model) break;
            const term = vscode.window.createTerminal({
              name: `ollama: ${model}`,
            });
            term.show();
            term.sendText(`ollama run ${model}`);
            break;
          }
          case "mcpChipAction": {
            if (msg.tone === "bad") {
              const action = await vscode.window.showErrorMessage(
                "git-shell-helpers-mcp binary not found. Reinstall the extension or run the installer script.",
                "Run Installer",
                "Open Terminal",
              );
              if (action === "Run Installer") {
                const terminal = vscode.window.createTerminal("gsh installer");
                terminal.show();
                terminal.sendText("install-git-shell-helpers");
              } else if (action === "Open Terminal") {
                await vscode.commands.executeCommand(
                  "workbench.action.terminal.new",
                );
              }
            } else if (msg.tone === "warn") {
              const action = await vscode.window.showWarningMessage(
                "MCP provider API unavailable. Open the MCP panel and start or trust the gsh server.",
                "Open MCP Panel",
              );
              if (action === "Open MCP Panel") await openMcpServerControls();
            } else {
              await openMcpServerControls();
            }
            break;
          }
          case "uploadGpgKey":
            await uploadGpgKeyNow();
            break;
          case "reloginGpg":
            setCachedGpgUploadFailed(false);
            setCachedUser("");
            setCachedRepos([]);
            this.refresh();
            await loginGitHub();
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

    pushUpdate(data) {
      if (!this._view?.visible) return;
      this._view.webview.postMessage(data);
    }

    async _update() {
      if (!this._view) return;
      const mode = getMode();
      const whitelist = getWhitelist();
      this._view.webview.html = await this._getHtml(mode, whitelist);
    }

    async _getHtml(mode, whitelist) {
      // Gate: require GitHub sign-in
      if (!getCachedUser()) {
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

      const gpgHint = getCachedGpgNeedsUpload()
        ? getCachedGpgUploadFailed()
          ? `<div style="font-size:10.5px;color:var(--vscode-descriptionForeground);margin-top:6px">Upload failed. <span role="button" id="reloginGpgBtn" style="color:var(--vscode-textLink-foreground);text-decoration:underline;cursor:pointer">Re-login</span></div>`
          : `<div style="font-size:10.5px;color:var(--vscode-descriptionForeground);margin-top:6px">Key not on GitHub — commits show Unverified. <span role="button" id="uploadGpgBtn" style="color:var(--vscode-textLink-foreground);text-decoration:underline;cursor:pointer">Upload now</span></div>`
        : "";

      const cpConfig = vscode.workspace.getConfiguration(
        "gitShellHelpers.checkpoint",
      );
      const cpEnabled = cpConfig.get("enabled", true);
      const cpAutoPush = cpConfig.get("autoPush", false);
      const cpSign = cpConfig.get("sign", false);
      const mcpStatus = getMcpStatusViewModel(deps._context);

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
      const strictLintingEnabled = isStrictLintingEnabled();
      const branchSessionsEnabled = vscode.workspace
        .getConfiguration("gitShellHelpers.branchSessions")
        .get("enabled", false);
      const sessionMemoryEnabled = vscode.workspace
        .getConfiguration("gitShellHelpers.sessionMemory")
        .get("enabled", true);

      // --- Provider status ---
      const providerStatus = await getProviderStatus();
      const providerConfigured = [
        providerStatus.ollamaRunning,
        providerStatus.anthropicKey,
        providerStatus.openaiKey,
      ].filter(Boolean).length;

      // Ollama: pinned models shown as agent-style rows; not-running shows refresh nudge
      const ollamaRows =
        providerStatus.ollamaRunning && providerStatus.ollamaModels.length > 0
          ? providerStatus.ollamaModels
              .filter((m) => deps._ollamaPinned.has(m))
              .map(
                (m) => `
        <div class="provider-model-row">
          <span class="provider-model-dot"></span>
          <span class="provider-model-name">${escapeHtml(m)}</span>
          <button class="provider-model-run" data-ollamarun="${escapeHtml(m)}" title="ollama run ${escapeHtml(m)}">run</button>
          <button class="provider-model-remove" data-ollamatoggle="${escapeHtml(m)}" title="Remove">\u00d7</button>
        </div>`,
              )
              .join("")
          : "";

      const ollamaAddBtn =
        providerStatus.ollamaRunning && providerStatus.ollamaModels.length > 0
          ? `<button class="provider-add-btn" id="ollamaAddModelsBtn">+ Add model</button>`
          : "";

      // Ollama add-model panel (all available, click to pin)
      const ollamaAddPanel =
        providerStatus.ollamaRunning && providerStatus.ollamaModels.length > 0
          ? `<div class="provider-acc-panel" id="ollamaAccPanel"><div class="ollama-models">${providerStatus.ollamaModels
              .map((m) => {
                const pinned = deps._ollamaPinned.has(m);
                return `<div class="ollama-model-row${pinned ? " on" : ""}">
            <span class="ollama-model-check">\u2713</span>
            <button class="ollama-tag${pinned ? " on" : ""}" data-ollamatoggle="${escapeHtml(m)}">${escapeHtml(m)}</button>
          </div>`;
              })
              .join("")}</div></div>`
          : "";

      const ollamaStatusRow = !providerStatus.ollamaRunning
        ? `<div class="provider-row provider-row-dim" id="ollamaRefreshChip" style="cursor:pointer" title="Click to recheck"><span class="provider-row-dot"></span><span class="provider-row-label">Ollama not running</span><span class="provider-row-action">recheck</span></div>`
        : "";

      // Anthropic / OpenAI: clean row with inline expand for key entry
      const anthropicRow = `
      <div class="provider-row${providerStatus.anthropicKey ? " provider-row-set" : ""}">
        <span class="provider-row-dot${providerStatus.anthropicKey ? " set" : ""}"></span>
        <span class="provider-row-label">Anthropic</span>
        <button class="provider-row-action provider-chip-clickable" id="anthropicChipBtn" data-acc="anthropic">${providerStatus.anthropicKey ? "change key" : "add key"}</button>
      </div>
      <div class="provider-acc-panel" id="anthropicAccPanel">
        <div class="key-input-row">
          <input class="key-input" id="anthropicKeyInput" type="password"
            placeholder="${providerStatus.anthropicKey ? "\u25cf\u25cf\u25cf\u25cf\u25cf\u25cf\u25cf\u25cf (saved)" : "sk-ant-\u2026"}"
            autocomplete="off" data-provider="anthropic" />
          <button class="key-save-btn" data-savekey="anthropic">Save</button>
          ${providerStatus.anthropicKey ? `<button class="key-clear-btn" data-clearkey="anthropic">Clear</button>` : ""}
        </div>
      </div>`;
      const openaiRow = `
      <div class="provider-row${providerStatus.openaiKey ? " provider-row-set" : ""}">
        <span class="provider-row-dot${providerStatus.openaiKey ? " set" : ""}"></span>
        <span class="provider-row-label">OpenAI</span>
        <button class="provider-row-action provider-chip-clickable" id="openaiChipBtn" data-acc="openai">${providerStatus.openaiKey ? "change key" : "add key"}</button>
      </div>
      <div class="provider-acc-panel" id="openaiAccPanel">
        <div class="key-input-row">
          <input class="key-input" id="openaiKeyInput" type="password"
            placeholder="${providerStatus.openaiKey ? "\u25cf\u25cf\u25cf\u25cf\u25cf\u25cf\u25cf\u25cf (saved)" : "sk-\u2026"}"
            autocomplete="off" data-provider="openai" />
          <button class="key-save-btn" data-savekey="openai">Save</button>
          ${providerStatus.openaiKey ? `<button class="key-clear-btn" data-clearkey="openai">Clear</button>` : ""}
        </div>
      </div>`;

      // --- Local agents section ---
      const allAgents = scanLocalAgents().filter((a) => a.userInvocable);
      const agentRows =
        allAgents.length > 0
          ? allAgents
              .map(
                (a, i) => `
        <div class="agent-item${i >= 3 ? " agent-overflow" : ""}" data-agent="${escapeHtml(a.name)}">
          <div class="agent-dot"></div>
          <div class="agent-text">
            <span class="agent-name"><span class="agent-at">@</span>${escapeHtml(a.name)}</span>
            ${a.description ? `<span class="agent-desc">${escapeHtml(a.description)}</span>` : ""}
          </div>
          <button class="agent-start-btn" data-agentname="${escapeHtml(a.name)}" title="Open @${escapeHtml(a.name)} in Copilot chat">
            <svg viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M3.5 2A1.5 1.5 0 0 0 2 3.5v9A1.5 1.5 0 0 0 3.5 14h9a1.5 1.5 0 0 0 1.5-1.5V8.75a.75.75 0 0 0-1.5 0v3.75h-9v-9H8a.75.75 0 0 0 0-1.5H3.5zm7.25.25a.75.75 0 0 0 0 1.5H12.2L7.47 8.47a.75.75 0 0 0 1.06 1.06L13 5.05v1.45a.75.75 0 0 0 1.5 0V2.75a.5.5 0 0 0-.5-.5h-3.25z"/></svg>
          </button>
        </div>`,
              )
              .join("") +
            (allAgents.length > 3
              ? `<button class="view-more-btn" id="viewMoreAgentsBtn">+ ${allAgents.length - 3} more</button>`
              : "")
          : `<div class="muted">No agents found in .github/agents/</div>`;

      // --- Activity section ---
      const activityItems = getActivityItems();
      const activityCountLabel = _activityCountLabel(activityItems);
      const activityItemsJson = JSON.stringify(activityItems).replace(
        /</g,
        "\\u003c",
      );
      const activityCountJson = JSON.stringify(activityCountLabel).replace(
        /</g,
        "\\u003c",
      );

      const mcpStatusHtml = `
      <div class="mcp-chip ${mcpStatus.tone}" id="manageMcpBtn" data-tone="${mcpStatus.tone}" title="${escapeHtml(mcpStatus.detail)}">
        <span class="mcp-dot"></span>
        <span class="mcp-chip-status">${escapeHtml(mcpStatus.label)}</span>
      </div>`;

      const strictLintingRow = `
      <div class="tool-item${strictLintingEnabled ? " active" : ""}" data-strict-linting="enabled">
        <div class="cb${strictLintingEnabled ? " on" : ""}"><div class="cb-tick"></div></div>
        <div class="tool-text">
          <span class="tl">Strict Linting</span>
          <span class="td">Reads live VS Code errors, warnings, hover details, and quick fixes in chat</span>
        </div>
      </div>`;

      const branchSessionsRow = `
      <div class="tool-item${branchSessionsEnabled ? " active" : ""}" data-branch-sessions="enabled">
        <div class="cb${branchSessionsEnabled ? " on" : ""}"><div class="cb-tick"></div></div>
        <div class="tool-text">
          <span class="tl">Branch Sessions</span>
          <span class="td">The workspace follows the active chat's branch; parked sessions stay available via branch_status</span>
        </div>
      </div>`;

      const sessionMemoryRow = `
      <div class="tool-item${sessionMemoryEnabled ? " active" : ""}" data-session-memory="enabled">
        <div class="cb${sessionMemoryEnabled ? " on" : ""}"><div class="cb-tick"></div></div>
        <div class="tool-text">
          <span class="tl">Session Memory</span>
          <span class="td">Agents log actions and outcomes for Engram-style surprise-weighted learning</span>
        </div>
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
        const publicCount = getCachedRepos().filter(
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

      // --- Quick Actions ---
      const quickActionsHtml = QUICK_ACTIONS.map(
        (qa) => `
      <div class="qa-item" data-qaaction="${escapeHtml(qa.id)}">
        <div class="qa-icon">
          <svg viewBox="0 0 16 16" fill="currentColor"><path d="${escapeHtml(qa.iconPath)}"/></svg>
        </div>
        <div class="qa-text">
          <span class="qa-label">${escapeHtml(qa.label)}</span>
          <span class="qa-desc">${escapeHtml(qa.desc)}</span>
        </div>
        <button class="qa-run-btn" data-qa="${escapeHtml(qa.id)}" title="Run in chat">
          <svg viewBox="0 0 16 16" fill="currentColor"><path d="M3 2.5A.5.5 0 0 1 3.5 2l10 5.5a.5.5 0 0 1 0 .87l-10 5.5A.5.5 0 0 1 3 13.5v-11z"/></svg>
        </button>
      </div>`,
      ).join("");

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
  .sect { padding: 10px 14px 13px; }
  .sect + .sect { border-top: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.12)); }
  .sect-head {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 9px;
  }
  .sect-title {
    font-size: 11px; font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.5px; color: var(--vscode-foreground); opacity: 0.65;
  }
  .sect-count {
    font-size: 10px; line-height: 1.6; font-weight: 500;
    color: var(--vscode-badge-foreground, var(--vscode-descriptionForeground));
    background: var(--vscode-badge-background, rgba(128,128,128,0.14));
    padding: 0 6px; border-radius: 10px;
  }

  /* Tool items — checkbox style */
  .tool-item {
    display: flex; align-items: flex-start; gap: 9px;
    padding: 5px 6px; margin: 1px -6px;
    border-radius: 4px; cursor: pointer; user-select: none;
    transition: background 0.1s;
  }
  .tool-item:hover { background: var(--vscode-list-hoverBackground, rgba(128,128,128,0.08)); }
  .cb {
    flex-shrink: 0; width: 14px; height: 14px; margin-top: 2px;
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
  .tl { display: block; font-size: 12.5px; font-weight: 500; line-height: 1.3; }
  .td { display: block; font-size: 11px; color: var(--vscode-descriptionForeground); line-height: 1.3; margin-top: 2px; }

  .hint {
    font-size: 11px; color: var(--vscode-descriptionForeground);
    margin-top: 8px; padding: 0; opacity: 0.6;
    background: none; border-radius: 0;
  }

  .sect-head-left {
    display: flex; align-items: center; gap: 8px;
  }
  .mcp-chip {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 8px 2px 6px; border-radius: 999px;
    cursor: pointer; font-size: 11px; line-height: 1.5;
    border: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.16));
    background: var(--vscode-editorWidget-background, rgba(128,128,128,0.04));
    user-select: none; transition: opacity 0.12s;
  }
  .mcp-chip:hover { opacity: 0.8; }
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
    width: 6px; height: 6px; border-radius: 999px; flex-shrink: 0;
    background: var(--vscode-descriptionForeground);
  }
  .mcp-chip.good .mcp-dot { background: var(--vscode-testing-iconPassed, #2ea043); }
  .mcp-chip.warn .mcp-dot { background: var(--vscode-inputValidation-warningBorder, #cca700); }
  .mcp-chip.bad .mcp-dot { background: var(--vscode-inputValidation-errorBorder, #be1100); }
  .mcp-chip-status { color: var(--vscode-descriptionForeground); }

  /* Community cache */
  select {
    width: 100%; padding: 5px 8px;
    border: 1px solid var(--vscode-dropdown-border); border-radius: 4px;
    background: var(--vscode-dropdown-background);
    color: var(--vscode-dropdown-foreground);
    font-size: var(--vscode-font-size); outline: none;
  }
  select:focus { border-color: var(--vscode-focusBorder); }
  .mode-desc {
    font-size: 11px; color: var(--vscode-descriptionForeground);
    line-height: 1.5; margin-top: 6px;
  }
  .sub-label {
    font-size: 10.5px; font-weight: 600; color: var(--vscode-descriptionForeground);
    margin: 10px 0 4px; text-transform: uppercase; letter-spacing: 0.5px; opacity: 0.7;
  }
  .repo-item {
    font-size: 11.5px; padding: 2px 0; line-height: 1.4;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .scope-text { font-size: 11.5px; line-height: 1.5; }
  .muted { color: var(--vscode-descriptionForeground); font-style: italic; font-size: 11.5px; }
  .btn-secondary {
    display: block; width: 100%; padding: 6px 12px; margin-top: 8px;
    border: 1px solid transparent; border-radius: 4px; font-size: 12px; cursor: pointer;
    background: var(--vscode-button-secondaryBackground);
    color: var(--vscode-button-secondaryForeground);
    transition: background 0.12s;
  }
  .btn-secondary:hover { background: var(--vscode-button-secondaryHoverBackground); }

  /* Footer */
  .footer {
    position: sticky; bottom: 0; left: 0; right: 0;
    padding: 7px 14px;
    border-top: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.12));
    background: var(--vscode-sideBar-background, var(--vscode-editor-background));
    display: flex; align-items: center; justify-content: space-between; gap: 8px;
    font-size: 11px; color: var(--vscode-descriptionForeground);
  }
  .content { flex: 1; overflow-y: auto; padding-bottom: 36px; }

  /* Local agents */
  .agent-item {
    display: flex; align-items: center; gap: 9px;
    padding: 5px 6px; margin: 2px -6px;
    border-radius: 4px; cursor: pointer; user-select: none;
    transition: background 0.1s;
  }
  .agent-item:hover { background: var(--vscode-list-hoverBackground, rgba(128,128,128,0.08)); }
  .agent-overflow { display: none; }
  .agent-dot {
    flex-shrink: 0; width: 7px; height: 7px; border-radius: 999px;
    background: var(--vscode-testing-iconPassed, #2ea043);
    transition: box-shadow 0.15s;
  }
  .agent-item:hover .agent-dot {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--vscode-testing-iconPassed, #2ea043) 22%, transparent);
  }
  .agent-text { flex: 1; min-width: 0; }
  .agent-name { display: block; font-size: 12.5px; font-weight: 500; line-height: 1.3; }
  .agent-desc {
    display: block; font-size: 10.5px;
    color: var(--vscode-descriptionForeground); line-height: 1.3; margin-top: 2px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .agent-start-btn {
    flex-shrink: 0; padding: 4px; margin-left: auto;
    border: none; background: none; cursor: pointer;
    color: var(--vscode-descriptionForeground);
    border-radius: 4px; display: flex; align-items: center;
    opacity: 0; transition: opacity 0.12s, background 0.1s;
  }
  .agent-item:hover .agent-start-btn {
    opacity: 1;
    background: var(--vscode-toolbar-hoverBackground, rgba(128,128,128,0.12));
    color: var(--vscode-foreground);
  }
  .agent-start-btn svg { width: 13px; height: 13px; }
  .view-more-btn {
    display: inline-block; padding: 3px 6px; margin-top: 6px;
    border: none; background: none; cursor: pointer;
    font-size: 11.5px; color: var(--vscode-textLink-foreground);
    font-family: inherit; border-radius: 3px;
  }
  .view-more-btn:hover { text-decoration: underline; }

  /* Providers */
  .provider-row {
    display: flex; align-items: center; gap: 7px;
    padding: 5px 2px; font-size: 12px;
    border-bottom: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.1));
  }
  .provider-row:last-of-type { border-bottom: none; }
  .provider-row-dim { opacity: 0.5; }
  .provider-row-dot {
    width: 7px; height: 7px; border-radius: 999px; flex-shrink: 0;
    background: var(--vscode-descriptionForeground); opacity: 0.3;
  }
  .provider-row-dot.set { background: var(--vscode-testing-iconPassed, #2ea043); opacity: 1; }
  .provider-row-label { flex: 1; font-weight: 500; }
  .provider-row-action {
    flex-shrink: 0; padding: 2px 8px; font-size: 11px;
    border: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.25));
    border-radius: 4px; background: none;
    color: var(--vscode-foreground); font-family: inherit;
    cursor: pointer; opacity: 0.7; transition: opacity 0.12s;
  }
  .provider-row-action:hover { opacity: 1; background: var(--vscode-toolbar-hoverBackground, rgba(128,128,128,0.1)); }
  /* Pinned Ollama models */
  .provider-model-row {
    display: flex; align-items: center; gap: 6px;
    padding: 4px 2px; font-size: 12px;
    border-radius: 4px; margin: 0 -4px; padding-left: 4px; padding-right: 4px;
    transition: background 0.1s;
  }
  .provider-model-row:hover { background: var(--vscode-list-hoverBackground, rgba(128,128,128,0.08)); }
  .provider-model-dot {
    width: 6px; height: 6px; border-radius: 999px; flex-shrink: 0;
    background: var(--vscode-testing-iconPassed, #2ea043);
  }
  .provider-model-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .provider-model-run, .provider-model-remove {
    flex-shrink: 0; padding: 2px 7px; border-radius: 3px; font-size: 10.5px;
    border: none; cursor: pointer; font-family: inherit;
    opacity: 0; transition: opacity 0.12s; line-height: 1.5;
  }
  .provider-model-row:hover .provider-model-run,
  .provider-model-row:hover .provider-model-remove { opacity: 1; }
  .provider-model-run {
    background: var(--vscode-button-background); color: var(--vscode-button-foreground);
  }
  .provider-model-run:hover { background: var(--vscode-button-hoverBackground); }
  .provider-model-remove {
    background: none; color: var(--vscode-descriptionForeground);
    font-size: 14px; padding: 0 5px;
  }
  .provider-model-remove:hover { color: var(--vscode-errorForeground); }
  .provider-add-btn {
    display: block; width: 100%; margin: 6px 0 2px;
    padding: 5px 0; text-align: center;
    background: none; border: 1px dashed var(--vscode-panel-border, rgba(128,128,128,0.3));
    border-radius: 4px; font-size: 11.5px; font-family: inherit;
    color: var(--vscode-foreground); opacity: 0.55; cursor: pointer;
    transition: opacity 0.12s, border-color 0.12s;
  }
  .provider-add-btn:hover { opacity: 0.9; border-color: var(--vscode-focusBorder, #007fd4); }
  .key-row { margin-bottom: 8px; }
  .key-row:last-child { margin-bottom: 0; }
  .key-label { font-size: 11px; font-weight: 600; color: var(--vscode-descriptionForeground); margin-bottom: 3px; }
  .key-input-row { display: flex; gap: 5px; align-items: center; }
  .key-input {
    flex: 1; min-width: 0;
    padding: 5px 7px; font-size: 12px;
    border: 1px solid var(--vscode-input-border, rgba(128,128,128,0.3));
    border-radius: 4px;
    background: var(--vscode-input-background); color: var(--vscode-input-foreground);
    font-family: inherit; outline: none;
  }
  .key-input:focus { border-color: var(--vscode-focusBorder); }
  .key-save-btn, .key-clear-btn {
    flex-shrink: 0; padding: 5px 10px; border: none; border-radius: 4px;
    font-size: 11px; cursor: pointer; font-family: inherit; transition: background 0.12s;
  }
  .key-save-btn {
    background: var(--vscode-button-background); color: var(--vscode-button-foreground);
  }
  .key-save-btn:hover { background: var(--vscode-button-hoverBackground); }
  .key-clear-btn {
    background: var(--vscode-button-secondaryBackground); color: var(--vscode-button-secondaryForeground);
  }
  .key-clear-btn:hover { background: var(--vscode-button-secondaryHoverBackground); }

  /* Ollama "add model" accordion list */
  .ollama-models { display: flex; flex-direction: column; gap: 1px; padding: 2px 0; }
  .ollama-model-row {
    display: flex; align-items: center; gap: 0;
    padding: 3px 6px; margin: 0 -6px; border-radius: 4px;
    transition: background 0.1s;
  }
  .ollama-model-row:hover { background: var(--vscode-list-hoverBackground, rgba(128,128,128,0.08)); }
  .ollama-tag {
    flex: 1; padding: 0; text-align: left;
    border: none; background: none;
    font-size: 12px; font-family: inherit; font-weight: 400;
    color: var(--vscode-foreground);
    cursor: pointer; user-select: none; transition: color 0.12s; line-height: 1.4;
  }
  .ollama-tag.on { font-weight: 500; }
  .ollama-model-check {
    width: 14px; flex-shrink: 0; font-size: 11px; margin-right: 5px;
    color: var(--vscode-testing-iconPassed, #2ea043); font-weight: 700; opacity: 0;
  }
  .ollama-model-row.on .ollama-model-check { opacity: 1; }

  /* Activity */
  .activity-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .activity-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .activity-group-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 1px 2px 0;
  }
  .activity-group-title {
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.55px;
    text-transform: uppercase;
    color: var(--vscode-descriptionForeground);
    opacity: 0.85;
  }
  .activity-group-count {
    flex-shrink: 0;
    min-width: 22px;
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 10px;
    font-weight: 700;
    text-align: center;
    color: var(--vscode-badge-foreground, var(--vscode-foreground));
    background: var(--vscode-badge-background, rgba(128,128,128,0.16));
  }
  .activity-card,
  .activity-tool {
    width: 100%;
    border: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.12));
    border-radius: 8px;
    overflow: hidden;
    background: color-mix(
      in srgb,
      var(--vscode-editorWidget-background, rgba(128,128,128,0.05)) 94%,
      transparent
    );
  }
  .activity-card {
    padding: 0;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s, transform 0.12s;
  }
  .activity-card:hover {
    background: color-mix(
      in srgb,
      var(--vscode-list-hoverBackground, rgba(128,128,128,0.1)) 82%,
      transparent
    );
    border-color: var(--vscode-focusBorder, rgba(128,128,128,0.3));
    transform: translateY(-1px);
  }
  .activity-card--live,
  .activity-tool {
    box-shadow: inset 3px 0 0 var(--vscode-textLink-foreground, #3794ff);
  }
  .activity-card--recent {
    box-shadow: inset 3px 0 0 var(--vscode-testing-iconPassed, #2ea043);
    opacity: 0.9;
  }
  .activity-tool {
    box-shadow: inset 3px 0 0 var(--vscode-charts-yellow, #cca700);
  }
  .activity-card-main,
  .activity-tool-summary {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 9px 10px 8px;
  }
  .activity-tool-summary {
    list-style: none;
    cursor: pointer;
    user-select: none;
  }
  .activity-tool-summary::-webkit-details-marker { display: none; }
  .activity-headline {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }
  .activity-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }
  .activity-state-dot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    border-radius: 999px;
  }
  .activity-state-dot--live {
    background: var(--vscode-textLink-foreground, #3794ff);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--vscode-textLink-foreground, #3794ff) 18%, transparent);
    animation: pulse 1.6s ease-in-out infinite;
  }
  .activity-state-dot--recent {
    background: var(--vscode-testing-iconPassed, #2ea043);
  }
  .activity-state-dot--tool {
    background: var(--vscode-charts-yellow, #cca700);
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.42; }
  }
  .activity-card-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
    font-weight: 600;
    line-height: 1.3;
  }
  .activity-pill-row {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    flex-wrap: wrap;
  }
  .activity-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 7px;
    border-radius: 999px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.2px;
    color: var(--vscode-descriptionForeground);
    background: color-mix(
      in srgb,
      var(--vscode-badge-background, rgba(128,128,128,0.18)) 86%,
      transparent
    );
  }
  .activity-pill--time {
    color: var(--vscode-foreground);
    background: color-mix(
      in srgb,
      var(--vscode-button-secondaryBackground, rgba(128,128,128,0.16)) 88%,
      transparent
    );
  }
  .activity-preview {
    font-size: 11px;
    line-height: 1.45;
    color: var(--vscode-descriptionForeground);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .activity-meta-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 10.5px;
    color: var(--vscode-descriptionForeground);
  }
  .activity-meta-row span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .activity-link-hint {
    color: var(--vscode-textLink-foreground);
  }
  .activity-tool-chevron {
    flex-shrink: 0;
    width: 10px;
    height: 10px;
    opacity: 0.5;
    transition: transform 0.15s;
  }
  details[open] .activity-tool-chevron { transform: rotate(90deg); }
  .activity-tool-detail {
    padding: 0 10px 10px;
  }
  .activity-tool-detail pre {
    margin: 0;
    padding: 8px 9px;
    border-radius: 6px;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--vscode-editor-font-family, monospace);
    font-size: 10px;
    color: var(--vscode-descriptionForeground);
    background: color-mix(
      in srgb,
      var(--vscode-editor-background, rgba(128,128,128,0.06)) 92%,
      transparent
    );
    max-height: 132px;
    overflow-y: auto;
  }
  .footer-user {
    display: flex; align-items: center; gap: 5px; overflow: hidden;
  }
  .footer-user svg { width: 12px; height: 12px; flex-shrink: 0; opacity: 0.55; fill: currentColor; }
  .footer-user span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .footer-gear {
    flex-shrink: 0; cursor: pointer; opacity: 0.45;
    padding: 3px; border-radius: 3px;
    transition: opacity 0.15s, background 0.1s; display: flex; align-items: center;
  }
  .footer-gear:hover { opacity: 1; background: var(--vscode-toolbar-hoverBackground, rgba(128,128,128,0.1)); }
  .footer-gear svg { width: 14px; height: 14px; fill: currentColor; }
  .footer-gear.active { opacity: 1; }

  /* Activity idle indicator */
  .activity-idle {
    display: flex; align-items: center; gap: 6px;
    font-size: 12px; color: var(--vscode-descriptionForeground);
    padding: 6px 2px 3px; opacity: 0.72;
  }
  .activity-idle-dot {
    width: 6px; height: 6px; border-radius: 999px; flex-shrink: 0;
    background: var(--vscode-descriptionForeground); opacity: 0.45;
  }
  /* When activity is idle, collapse the section to just the header row */
  .sect--idle { padding-bottom: 4px; }
  .activity-list-hidden { display: none; }
  /* The sect-count shows "idle" inline — style it softer */
  .sect--idle .sect-count {
    background: none; padding: 0;
    font-weight: 400; font-size: 11px;
    color: var(--vscode-descriptionForeground); opacity: 0.6;
  }
  /* Collapsible sections */
  details.sect > summary.sect-head {
    list-style: none; cursor: pointer; user-select: none;
  }
  details.sect > summary.sect-head::-webkit-details-marker { display: none; }
  details.sect > summary .sect-title::before {
    content: '\u25B8'; display: inline-block; margin-right: 4px; font-size: 9px; opacity: 0.6;
  }
  details[open] > summary .sect-title::before {
    content: '\u25BE';
  }
  details.sect:not([open]) { padding-bottom: 6px; }
  /* Provider key accordion */
  .provider-acc-panel {
    max-height: 0; overflow: hidden;
    transition: max-height 0.22s ease, opacity 0.15s ease;
    opacity: 0;
  }
  .provider-acc-panel.open { max-height: 400px; opacity: 1; padding-top: 6px; }
  .provider-chip-clickable { cursor: pointer; transition: opacity 0.12s; }
  .provider-chip-clickable:hover { opacity: 0.85; }
  .provider-chip-clickable.active {
    border-color: var(--vscode-focusBorder, #007fd4) !important; opacity: 1;
  }

  /* Quick Actions */
  .qa-list { display: flex; flex-direction: column; gap: 1px; }
  .qa-item {
    display: flex; align-items: center; gap: 9px;
    padding: 5px 6px; margin: 1px -6px;
    border-radius: 4px; cursor: pointer; user-select: none;
    transition: background 0.1s;
  }
  .qa-item:hover { background: var(--vscode-list-hoverBackground, rgba(128,128,128,0.08)); }
  .qa-icon {
    flex-shrink: 0; width: 22px; height: 22px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 4px;
    background: rgba(128,128,128,0.12);
    opacity: 0.85;
  }
  .qa-icon svg { width: 12px; height: 12px; fill: currentColor; }
  .qa-text { flex: 1; min-width: 0; }
  .qa-label { display: block; font-size: 12.5px; font-weight: 500; line-height: 1.3; }
  .qa-desc { display: block; font-size: 11px; color: var(--vscode-descriptionForeground); line-height: 1.3; margin-top: 2px; }
  .qa-run-btn {
    flex-shrink: 0; display: flex; align-items: center; justify-content: center;
    width: 22px; height: 22px; padding: 0;
    border: none; border-radius: 4px; cursor: pointer;
    background: transparent; color: var(--vscode-foreground); opacity: 0.55;
    transition: opacity 0.12s, background 0.12s;
  }
  .qa-run-btn:hover { opacity: 1; background: var(--vscode-toolbar-hoverBackground, rgba(128,128,128,0.14)); }
  .qa-run-btn svg { width: 11px; height: 11px; fill: currentColor; }
  /* Context menu */
  .qa-ctx-menu {
    display: none; position: fixed;
    background: var(--vscode-menu-background, var(--vscode-editorWidget-background));
    border: 1px solid var(--vscode-menu-border, var(--vscode-panel-border, rgba(128,128,128,0.3)));
    border-radius: 5px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.28);
    z-index: 9999; overflow: hidden; min-width: 200px;
    padding: 2px 0;
  }
  .qa-ctx-item {
    padding: 5px 12px;
    font-size: 12px; line-height: 1.5; cursor: pointer;
    color: var(--vscode-menu-foreground, var(--vscode-foreground));
    display: flex; align-items: center; gap: 7px;
    transition: background 0.08s;
  }
  .qa-ctx-item:hover { background: var(--vscode-menu-selectionBackground, var(--vscode-list-hoverBackground)); color: var(--vscode-menu-selectionForeground, var(--vscode-foreground)); }
  .qa-ctx-sep { height: 1px; background: var(--vscode-menu-separatorBackground, var(--vscode-panel-border, rgba(128,128,128,0.18))); margin: 2px 0; }

  /* Agent @ prefix */
  .agent-at {
    font-size: 11px; font-weight: 700;
    color: var(--vscode-textLink-foreground);
    letter-spacing: -0.2px; opacity: 0.9; margin-right: 0.5px;
  }
  /* Account panel overlay */
  body { position: relative; }
  .acct-panel {
    display: none;
    position: absolute;
    bottom: 40px;
    left: 8px; right: 8px;
    background: var(--vscode-editorWidget-background, var(--vscode-input-background, var(--vscode-editor-background)));
    border: 1px solid var(--vscode-widget-border, var(--vscode-panel-border, rgba(128,128,128,0.3)));
    border-radius: 6px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.28);
    z-index: 200;
    overflow: hidden;
  }
  .acct-panel.open { display: block; }
  .acct-header {
    display: flex; align-items: center; gap: 10px;
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.12));
  }
  .acct-avatar {
    width: 32px; height: 32px; flex-shrink: 0;
    border-radius: 999px;
    background: var(--vscode-button-background);
    display: flex; align-items: center; justify-content: center;
  }
  .acct-avatar svg { width: 18px; height: 18px; fill: var(--vscode-button-foreground, #fff); }
  .acct-info { flex: 1; min-width: 0; }
  .acct-name { font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acct-host { font-size: 11px; color: var(--vscode-descriptionForeground); margin-top: 1px; }
  .acct-actions { padding: 6px; }
  .acct-btn {
    display: flex; align-items: center; gap: 8px;
    width: 100%; padding: 6px 8px;
    border: none; background: none; border-radius: 4px;
    font-size: 12px; color: var(--vscode-foreground);
    cursor: pointer; text-align: left; font-family: inherit;
  }
  .acct-btn:hover { background: var(--vscode-list-hoverBackground, rgba(128,128,128,0.08)); }
  .acct-btn svg { width: 13px; height: 13px; fill: currentColor; flex-shrink: 0; opacity: 0.7; }
</style>
</head>
<body>
  <div class="acct-panel" id="acctPanel">
    <div class="acct-header">
      <div class="acct-avatar">
        <svg viewBox="0 0 16 16"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
      </div>
      <div class="acct-info">
        <div class="acct-name">${escapeHtml(getCachedUser())}</div>
        <div class="acct-host">github.com</div>
      </div>
    </div>
    <div class="acct-actions">
      <button class="acct-btn" id="signOutBtn">
        <svg viewBox="0 0 16 16"><path d="M10 12.5a.5.5 0 0 1-.5.5h-8a.5.5 0 0 1-.5-.5v-9a.5.5 0 0 1 .5-.5h8a.5.5 0 0 1 .5.5v2a.5.5 0 0 0 1 0v-2A1.5 1.5 0 0 0 9.5 2h-8A1.5 1.5 0 0 0 0 3.5v9A1.5 1.5 0 0 0 1.5 14h8a1.5 1.5 0 0 0 1.5-1.5v-2a.5.5 0 0 0-1 0v2z"/><path fill-rule="evenodd" d="M15.854 8.354a.5.5 0 0 0 0-.708l-3-3a.5.5 0 0 0-.708.708L14.293 7.5H5.5a.5.5 0 0 0 0 1h8.793l-2.147 2.146a.5.5 0 0 0 .708.708l3-3z"/></svg>
        Sign out
      </button>
    </div>
  </div>
  <div class="content">
    <details class="sect" open>
      <summary class="sect-head">
        <div class="sect-title">Quick Actions</div>
      </summary>
      <div class="qa-list">
        ${quickActionsHtml}
      </div>
    </details>
    <details class="sect sect--activity${activityItems.length === 0 ? " sect--idle" : ""}" open>
      <summary class="sect-head">
        <div class="sect-title">Activity</div>
        <div class="sect-count" id="activityCount">${activityCountLabel}</div>
      </summary>
      <div id="activityList" class="activity-list${activityItems.length === 0 ? " activity-list-hidden" : ""}"></div>
    </details>
    <details class="sect" open>
      <summary class="sect-head">
        <div class="sect-title">Local Agents</div>
        <div class="sect-count">${allAgents.length} available</div>
      </summary>
      <div id="agentsList">${agentRows}</div>
    </details>
    <details class="sect">
      <summary class="sect-head">
        <div class="sect-title">Providers</div>
        <div class="sect-count">${providerConfigured}/3</div>
      </summary>
      ${ollamaStatusRow}
      ${ollamaRows}
      ${ollamaAddBtn}
      ${ollamaAddPanel}
      ${anthropicRow}
      ${openaiRow}
    </details>
    <details class="sect">
      <summary class="sect-head">
        <div class="sect-head-left">
          <div class="sect-title">MCP Tools</div>
          ${mcpStatusHtml}
        </div>
        <div class="sect-count">${enabledCount}/${TOOL_GROUPS.length}</div>
      </summary>
      ${toolRows}
      <div class="hint">Read &amp; Search Knowledge are always on.</div>
    </details>
    <details class="sect">
      <summary class="sect-head">
        <div class="sect-title">Git Checkpoint</div>
      </summary>
      ${cpRows}
      ${gpgHint}
    </details>
    <details class="sect">
      <summary class="sect-head">
        <div class="sect-title">Chat Tools</div>
        <div class="sect-count">${(strictLintingEnabled ? 1 : 0) + (branchSessionsEnabled ? 1 : 0) + (sessionMemoryEnabled ? 1 : 0)}/3</div>
      </summary>
      ${strictLintingRow}
      ${branchSessionsRow}
      ${sessionMemoryRow}
    </details>
    <details class="sect">
      <summary class="sect-head">
        <div class="sect-title">Community Submissions</div>
      </summary>
      <select id="modeSelect">${modeOptions}</select>
      <div class="mode-desc">${modeDesc}</div>
      ${scopeSection}
    </details>
  </div>
  <div class="footer">
    <div class="footer-user">
      <svg viewBox="0 0 16 16"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
      <span>${escapeHtml(getCachedUser())}</span>
    </div>

    <div class="footer-gear" id="gearBtn" title="Account">
      <svg viewBox="0 0 16 16"><path d="M9.1 4.4L8.6 2H7.4l-.5 2.4-.7.3-2-1.3-.9.8 1.3 2-.2.7-2.4.5v1.2l2.4.5.3.7-1.3 2 .8.8 2-1.3.7.3.5 2.4h1.2l.5-2.4.7-.3 2 1.3.8-.8-1.3-2 .3-.7 2.4-.5V6.8l-2.4-.5-.3-.7 1.3-2-.8-.8-2 1.3-.7-.2zM9.4 8c0 .8-.6 1.4-1.4 1.4S6.6 8.8 6.6 8 7.2 6.6 8 6.6s1.4.6 1.4 1.4z"/></svg>
    </div>
  </div>
  <script>
    const vscode = acquireVsCodeApi();
    const initialActivityItems = ${activityItemsJson};
    const initialActivityCount = ${activityCountJson};

    function activityEscape(value) {
      return String(value || "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
    }

    function humanizeToolName(name) {
      return String(name || "tool")
        .replace(/[_-]+/g, " ")
        .replace(/\b\w/g, (letter) => letter.toUpperCase());
    }

    function formatActivityDuration(seconds) {
      const totalSeconds = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
      if (totalSeconds < 60) return totalSeconds + "s";
      const minutes = Math.floor(totalSeconds / 60);
      const remainder = totalSeconds % 60;
      if (minutes < 60) {
        return remainder > 0 ? minutes + "m " + remainder + "s" : minutes + "m";
      }
      const hours = Math.floor(minutes / 60);
      const minuteRemainder = minutes % 60;
      return minuteRemainder > 0 ? hours + "h " + minuteRemainder + "m" : hours + "h";
    }

    function formatActivityAgo(timestamp) {
      if (!timestamp) return "recent";
      const elapsedSeconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
      if (elapsedSeconds < 60) return "just now";
      if (elapsedSeconds < 3600) return Math.floor(elapsedSeconds / 60) + "m ago";
      return Math.floor(elapsedSeconds / 3600) + "h ago";
    }

    function pluralizeActivity(count, singular) {
      return count + " " + singular + (count === 1 ? "" : "s");
    }

    function groupActivityItems(items) {
      const liveSessions = items
        .filter((item) => item.type === "session-active")
        .sort((left, right) => (right.lastChangedAt || right.startedAt || 0) - (left.lastChangedAt || left.startedAt || 0));
      const tools = items
        .filter((item) => item.type === "tool")
        .sort((left, right) => (right.startedAt || 0) - (left.startedAt || 0));
      const recentSessions = items
        .filter((item) => item.type === "session-done")
        .sort((left, right) => (right.completedAt || right.startedAt || 0) - (left.completedAt || left.startedAt || 0));
      return { liveSessions, tools, recentSessions };
    }

    function renderActivityGroup(title, items, renderer) {
      if (!items.length) return "";
      return `
        <section class="activity-group">
          <div class="activity-group-head">
            <div class="activity-group-title">${activityEscape(title)}</div>
            <div class="activity-group-count">${items.length}</div>
          </div>
          ${items.map(renderer).join("")}
        </section>`;
    }

    function renderSessionCard(item, state) {
      const isLive = state === "live";
      const requestLabel = item.requestCount > 0
        ? pluralizeActivity(item.requestCount, "request")
        : isLive
          ? "active now"
          : "chat saved";
      const timingMarkup = isLive
        ? `<span class="activity-pill activity-pill--time activity-elapsed" data-started="${item.startedAt}">${formatActivityDuration(item.elapsed || 0)}</span>`
        : `<span class="activity-pill">${activityEscape(formatActivityAgo(item.completedAt))}</span>`;
      const stateClass = isLive ? "activity-card--live" : "activity-card--recent";
      const dotClass = isLive ? "activity-state-dot--live" : "activity-state-dot--recent";
      const preview = item.preview
        ? `<div class="activity-preview">${activityEscape(item.preview)}</div>`
        : "";
      return `
        <button class="activity-card ${stateClass}" data-sessionid="${activityEscape(item.sessionId)}" type="button">
          <div class="activity-card-main">
            <div class="activity-headline">
              <div class="activity-title-row">
                <span class="activity-state-dot ${dotClass}"></span>
                <span class="activity-card-title">${activityEscape(item.label)}</span>
              </div>
              <div class="activity-pill-row">
                <span class="activity-pill">${activityEscape(requestLabel)}</span>
                ${timingMarkup}
              </div>
            </div>
            ${preview}
            <div class="activity-meta-row">
              <span>${isLive ? "Open the chat panel to follow the current run." : "Reopen the chat panel to pick this session back up."}</span>
              <span class="activity-link-hint">open chat</span>
            </div>
          </div>
        </button>`;
    }

    function renderToolCard(item) {
      const argsText = String(item.args || "").trim();
      const hasArgs = argsText && argsText !== "{}" && argsText !== "[]";
      const header = `
        <div class="activity-headline">
          <div class="activity-title-row">
            <span class="activity-state-dot activity-state-dot--tool"></span>
            <span class="activity-card-title">${activityEscape(item.label)}</span>
          </div>
          <div class="activity-pill-row">
            <span class="activity-pill">${activityEscape(humanizeToolName(item.tool))}</span>
            <span class="activity-pill activity-pill--time activity-elapsed" data-started="${item.startedAt}">${formatActivityDuration(item.elapsed || 0)}</span>
            ${hasArgs ? '<svg class="activity-tool-chevron" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 0 1 1.06 0l3.25 3.25a.75.75 0 0 1 0 1.06l-3.25 3.25a.75.75 0 0 1-1.06-1.06L8.94 8 6.22 5.28a.75.75 0 0 1 0-1.06z"/></svg>' : ""}
          </div>
        </div>
        <div class="activity-meta-row">
          <span>Tool call running in the current chat.</span>
          <span>${activityEscape(item.tool || "tool")}</span>
        </div>`;
      if (!hasArgs) {
        return `
          <div class="activity-tool">
            <div class="activity-card-main">
              ${header}
            </div>
          </div>`;
      }
      return `
        <details class="activity-tool">
          <summary class="activity-tool-summary">
            ${header}
          </summary>
          <div class="activity-tool-detail"><pre>${activityEscape(argsText)}</pre></div>
        </details>`;
    }

    function renderActivityList(items, countLabel) {
      const list = document.getElementById("activityList");
      const count = document.getElementById("activityCount");
      const sect = list?.closest(".sect--activity");
      if (!list) return;
      if (count) count.textContent = countLabel || "idle";
      if (!items.length) {
        if (sect) sect.classList.add("sect--idle");
        list.classList.add("activity-list-hidden");
        list.innerHTML = "";
        return;
      }

      const groups = groupActivityItems(items);
      const markup = [
        renderActivityGroup("Live Chats", groups.liveSessions, (item) => renderSessionCard(item, "live")),
        renderActivityGroup("Running Tools", groups.tools, renderToolCard),
        renderActivityGroup("Recent Sessions", groups.recentSessions, (item) => renderSessionCard(item, "recent")),
      ].filter(Boolean).join("");

      if (sect) sect.classList.remove("sect--idle");
      list.classList.remove("activity-list-hidden");
      list.innerHTML = markup;
      list.querySelectorAll(".activity-card[data-sessionid]").forEach((card) => {
        card.addEventListener("click", () => {
          vscode.postMessage({ type: "openChatSession", sessionId: card.dataset.sessionid });
        });
      });
    }

    renderActivityList(initialActivityItems, initialActivityCount);
    document.querySelectorAll('.tool-item').forEach(el => {
      if (el.dataset.strictLinting || el.dataset.branchSessions || el.dataset.sessionMemory || el.dataset.cpkey) return;
      el.addEventListener('click', () => {
        const key = el.dataset.key;
        const active = el.classList.contains('active');
        vscode.postMessage({ type: 'toggleGroup', key, enabled: !active });
      });
    });
    document.querySelectorAll('[data-strict-linting]').forEach(el => {
      el.addEventListener('click', () => {
        const active = el.classList.contains('active');
        vscode.postMessage({ type: 'toggleStrictLinting', enabled: !active });
      });
    });
    document.querySelectorAll('[data-branch-sessions]').forEach(el => {
      el.addEventListener('click', () => {
        const active = el.classList.contains('active');
        vscode.postMessage({ type: 'toggleBranchSessions', enabled: !active });
      });
    });
    document.querySelectorAll('[data-session-memory]').forEach(el => {
      el.addEventListener('click', () => {
        const active = el.classList.contains('active');
        vscode.postMessage({ type: 'toggleSessionMemory', enabled: !active });
      });
    });
    document.querySelectorAll('[data-cpkey]').forEach(el => {
      el.addEventListener('click', () => {
        vscode.postMessage({ type: 'setCheckpoint', key: el.dataset.cpkey });
      });
    });
    document.getElementById("uploadGpgBtn")?.addEventListener("click", (e) => { e.preventDefault(); vscode.postMessage({type:"uploadGpgKey"}); });
    document.getElementById("reloginGpgBtn")?.addEventListener("click", (e) => { e.preventDefault(); vscode.postMessage({type:"reloginGpg"}); });
    document.getElementById("manageMcpBtn")?.addEventListener("click", () => {
      const tone = document.getElementById("manageMcpBtn").dataset.tone;
      if (tone === "bad") vscode.postMessage({type:"mcpChipAction",tone:"bad"});
      else if (tone === "warn") vscode.postMessage({type:"mcpChipAction",tone:"warn"});
      else vscode.postMessage({type:"mcpChipAction",tone:"good"});
    });

    document.querySelectorAll(".agent-item").forEach(item => {
      item.addEventListener("click", () => {
        const name = item.dataset.agent;
        if (name) vscode.postMessage({ type: "openAgent", name });
      });
    });
    document.querySelectorAll(".agent-start-btn").forEach(btn => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        vscode.postMessage({ type: "openAgent", name: btn.dataset.agentname });
      });
    });
    document.getElementById("viewMoreAgentsBtn")?.addEventListener("click", () => {
      document.querySelectorAll(".agent-overflow").forEach(el => { el.style.display = "flex"; });
      document.getElementById("viewMoreAgentsBtn").style.display = "none";
    });
    document.getElementById("ollamaRefreshChip")?.addEventListener("click", () => vscode.postMessage({type:"refreshOllama"}));
    document.querySelectorAll(".ollama-tag[data-ollamatoggle]").forEach(btn => {
      btn.addEventListener("click", () => vscode.postMessage({ type: "ollamaToggle", model: btn.dataset.ollamatoggle }));
    });
    document.querySelectorAll(".provider-model-run[data-ollamarun]").forEach(btn => {
      btn.addEventListener("click", (e) => { e.stopPropagation(); vscode.postMessage({ type: "ollamaRun", model: btn.dataset.ollamarun }); });
    });
    document.querySelectorAll(".provider-model-remove[data-ollamatoggle]").forEach(btn => {
      btn.addEventListener("click", (e) => { e.stopPropagation(); vscode.postMessage({ type: "ollamaToggle", model: btn.dataset.ollamatoggle }); });
    });
    document.querySelectorAll(".key-save-btn").forEach(btn => {
      btn.addEventListener("click", () => {
        const provider = btn.dataset.savekey;
        const input = document.getElementById(provider + "KeyInput");
        const value = input ? input.value.trim() : "";
        if (!value) return;
        vscode.postMessage({ type: "saveApiKey", provider, value });
        input.value = "";
      });
    });
    document.querySelectorAll(".key-clear-btn").forEach(btn => {
      btn.addEventListener("click", () => {
        vscode.postMessage({ type: "saveApiKey", provider: btn.dataset.clearkey, value: "" });
      });
    });
    document.querySelectorAll(".key-input").forEach(inp => {
      inp.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          const provider = inp.dataset.provider;
          const value = inp.value.trim();
          if (!value) return;
          vscode.postMessage({ type: "saveApiKey", provider, value });
          inp.value = "";
        }
      });
    });
    // Live activity updates from extension
    window.addEventListener("message", (event) => {
      const msg = event.data;
      if (msg?.type === "activityUpdate") {
        renderActivityList(msg.items || [], msg.countLabel || "idle");
      }
    });
    // Live elapsed-time ticker
    setInterval(() => {
      document.querySelectorAll('.activity-elapsed[data-started]').forEach(el => {
        const started = parseInt(el.dataset.started, 10);
        if (!isNaN(started)) {
          el.textContent = formatActivityDuration(Math.floor((Date.now() - started) / 1000));
        }
      });
    }, 1000);
    // Provider clickable: antropic/openai key buttons, ollama "Add model" button
    document.querySelectorAll('.provider-chip-clickable').forEach(btn => {
      btn.addEventListener('click', () => {
        const acc = btn.dataset.acc;
        if (!acc) return;
        const panel = document.getElementById(acc + 'AccPanel');
        if (!panel) return;
        const isOpen = panel.classList.toggle('open');
        btn.classList.toggle('active', isOpen);
        if (isOpen && acc !== 'ollama') {
          const input = document.getElementById(acc + 'KeyInput');
          setTimeout(() => input?.focus(), 60);
        }
      });
    });
    document.getElementById('ollamaAddModelsBtn')?.addEventListener('click', () => {
      const panel = document.getElementById('ollamaAccPanel');
      const btn = document.getElementById('ollamaAddModelsBtn');
      if (!panel) return;
      const isOpen = panel.classList.toggle('open');
      if (btn) btn.textContent = isOpen ? '\u2212 Close' : '+ Add model';
    });
    const gearBtn = document.getElementById("gearBtn");
    const acctPanel = document.getElementById("acctPanel");
    gearBtn?.addEventListener("click", (e) => {
      e.stopPropagation();
      const open = acctPanel.classList.toggle("open");
      gearBtn.classList.toggle("active", open);
    });
    document.addEventListener("click", () => {
      acctPanel?.classList.remove("open");
      gearBtn?.classList.remove("active");
    });
    acctPanel?.addEventListener("click", (e) => e.stopPropagation());
    document.getElementById("signOutBtn")?.addEventListener("click", () => vscode.postMessage({type:"logout"}));
    document.getElementById("selectReposBtn")?.addEventListener("click", () => vscode.postMessage({type:"selectRepos"}));
    document.getElementById("modeSelect")?.addEventListener("change", (e) => vscode.postMessage({type:"setMode", value: e.target.value}));
    // Quick Actions
    let _qaContextTarget = null;
    const qaCtxMenu = document.getElementById('qaContextMenu');
    document.querySelectorAll('.qa-run-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        vscode.postMessage({ type: 'runQuickAction', action: btn.dataset.qa });
      });
    });
    document.querySelectorAll('.qa-item').forEach(item => {
      item.addEventListener('click', () => {
        vscode.postMessage({ type: 'runQuickAction', action: item.dataset.qaaction });
      });
      item.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        _qaContextTarget = item.dataset.qaaction;
        if (!qaCtxMenu) return;
        qaCtxMenu.style.display = 'block';
        const menuW = 210, menuH = 60;
        qaCtxMenu.style.left = Math.min(e.clientX, window.innerWidth - menuW) + 'px';
        qaCtxMenu.style.top = Math.min(e.clientY, window.innerHeight - menuH) + 'px';
      });
    });
    document.getElementById('ctxOpenWithoutSend')?.addEventListener('click', (e) => {
      e.stopPropagation();
      if (_qaContextTarget) {
        vscode.postMessage({ type: 'openQuickActionWithoutSend', action: _qaContextTarget });
        _qaContextTarget = null;
      }
      if (qaCtxMenu) qaCtxMenu.style.display = 'none';
    });
    document.addEventListener('click', () => {
      if (qaCtxMenu) qaCtxMenu.style.display = 'none';
      _qaContextTarget = null;
    });
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && qaCtxMenu) qaCtxMenu.style.display = 'none';
    });
  </script>
  <div id="qaContextMenu" class="qa-ctx-menu">
    <div class="qa-ctx-item" id="ctxOpenWithoutSend">
      <svg viewBox="0 0 16 16" fill="currentColor" width="12" height="12"><path d="M2 2.5A.5.5 0 0 1 2.5 2h11a.5.5 0 0 1 .5.5v1a.5.5 0 0 1-.5.5H2.5a.5.5 0 0 1-.5-.5v-1zM2 6.5A.5.5 0 0 1 2.5 6h7a.5.5 0 0 1 0 1h-7A.5.5 0 0 1 2 6.5zM2.5 10a.5.5 0 0 0 0 1h5a.5.5 0 0 0 0-1h-5z"/></svg>
      Open in new chat without sending
    </div>
  </div>
</body>
</html>`;
    }
  }

  return CommunityCacheViewProvider;
};
