"use strict";
// src/webview-provider-split.js — Community Cache webview panel for Explorer sidebar
const vscode = require("vscode");
const createRenderWebviewHtml = require("./webview-html");

module.exports = function createWebviewProviderClass(deps) {
  const {
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
    API_KEY_ANTHROPIC,
    API_KEY_OPENAI,
    setCachedUser,
    setCachedRepos,
    setCachedGpgUploadFailed,
  } = deps;

  const renderWebviewHtml = createRenderWebviewHtml(deps);

  class CommunityCacheViewProvider {
    static viewType = "gitShellHelpers.communityCache";

    constructor(extensionUri) {
      this._extensionUri = extensionUri;
      this._view = null;
    }

    resolveWebviewView(webviewView) {
      this._view = webviewView;
      webviewView.webview.options = {
        enableScripts: true,
        localResourceRoots: [vscode.Uri.joinPath(this._extensionUri, "media")],
      };
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
            break;
          case "toggleStrictLinting":
            await vscode.workspace
              .getConfiguration("gitShellHelpers.customizationInspector")
              .update(
                "enabled",
                msg.enabled,
                vscode.ConfigurationTarget.Global,
              );
            break;
          case "toggleBranchSessions":
            await vscode.workspace
              .getConfiguration("gitShellHelpers.branchSessions")
              .update(
                "enabled",
                msg.enabled,
                vscode.ConfigurationTarget.Global,
              );
            break;
          case "toggleSessionMemory":
            await vscode.workspace
              .getConfiguration("gitShellHelpers.sessionMemory")
              .update(
                "enabled",
                msg.enabled,
                vscode.ConfigurationTarget.Global,
              );
            break;
          case "toggleFormatBypass":
            await vscode.workspace
              .getConfiguration("gitShellHelpers.formatControl")
              .update(
                "bypassOnAgentSave",
                msg.enabled,
                vscode.ConfigurationTarget.Global,
              );
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
            const model = String(msg.model || "").trim();
            if (!model) break;
            if (deps._ollamaPinned.has(model)) {
              deps._ollamaPinned.delete(model);
            } else {
              deps._ollamaPinned.add(model);
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
            const terminal = vscode.window.createTerminal({
              name: `ollama: ${model}`,
            });
            terminal.show();
            terminal.sendText(`ollama run ${model}`);
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
              if (action === "Open MCP Panel") {
                await openMcpServerControls();
              }
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
        if (webviewView.visible) {
          this._update();
        }
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
      return renderWebviewHtml({
        extensionUri: this._extensionUri,
        webview: this._view.webview,
        mode,
        whitelist,
      });
    }
  }

  return CommunityCacheViewProvider;
};
