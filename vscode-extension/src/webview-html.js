"use strict";

const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

module.exports = function createRenderWebviewHtml(deps) {
  const {
    getMcpStatusViewModel,
    escapeHtml,
    isGroupEnabled,
    isStrictLintingEnabled,
    getProviderStatus,
    scanLocalAgents,
    getActivityItems,
    _activityCountLabel,
    TOOL_GROUPS,
    MODES,
    QUICK_ACTIONS,
    getCachedUser,
    getCachedModels,
    getCachedRepos,
    getCachedGpgNeedsUpload,
    getCachedGpgUploadFailed,
  } = deps;

  const templateCache = new Map();

  function readTemplate(extensionUri, name) {
    const cacheKey = `${extensionUri.fsPath}:${name}`;
    if (!templateCache.has(cacheKey)) {
      const filePath = path.join(extensionUri.fsPath, "media", name);
      templateCache.set(cacheKey, fs.readFileSync(filePath, "utf8"));
    }
    return templateCache.get(cacheKey);
  }

  function renderTemplate(template, replacements) {
    return template.replace(/\{\{([A-Z0-9_]+)\}\}/g, (match, key) => {
      return Object.prototype.hasOwnProperty.call(replacements, key)
        ? String(replacements[key])
        : "";
    });
  }

  function buildCommonReplacements(webview, extensionUri, bootstrapJson) {
    return {
      CSP_SOURCE: webview.cspSource,
      STYLES_URI: webview
        .asWebviewUri(
          vscode.Uri.joinPath(extensionUri, "media", "community-cache.css"),
        )
        .toString(),
      SCRIPT_URI: webview
        .asWebviewUri(
          vscode.Uri.joinPath(extensionUri, "media", "community-cache.js"),
        )
        .toString(),
      BOOTSTRAP_JSON: bootstrapJson,
    };
  }

  function buildBootstrapJson(activityItems, activityCountLabel) {
    return JSON.stringify({
      initialActivityItems: activityItems,
      initialActivityCount: activityCountLabel,
    }).replace(/</g, "\\u003c");
  }

  return async function renderWebviewHtml({
    extensionUri,
    webview,
    mode,
    whitelist,
  }) {
    const activityItems = getActivityItems();
    const activityCountLabel = _activityCountLabel(activityItems);
    const bootstrapJson = buildBootstrapJson(activityItems, activityCountLabel);
    const common = buildCommonReplacements(
      webview,
      extensionUri,
      bootstrapJson,
    );

    if (!getCachedUser()) {
      return renderTemplate(
        readTemplate(extensionUri, "community-cache-gate.html"),
        common,
      );
    }

    const gpgHint = getCachedGpgNeedsUpload()
      ? getCachedGpgUploadFailed()
        ? '<div class="gpg-hint">Upload failed. <button class="gpg-hint-link" id="reloginGpgBtn" type="button">Re-login</button></div>'
        : '<div class="gpg-hint">Key not on GitHub - commits show Unverified. <button class="gpg-hint-link" id="uploadGpgBtn" type="button">Upload now</button></div>'
      : "";

    const cpConfig = vscode.workspace.getConfiguration(
      "gitShellHelpers.checkpoint",
    );
    const cpEnabled = cpConfig.get("enabled", true);
    const cpAutoPush = cpConfig.get("autoPush", false);
    const cpSign = cpConfig.get("sign", false);
    const cpUseAI = cpConfig.get("useAI", true);
    const cpModel = String(cpConfig.get("model") || "").trim();
    const cpModelInfo = cpModel
      ? getCachedModels().find((model) => model.id === cpModel) || null
      : null;
    const cpModelLabel = cpModel
      ? cpModelInfo?.name || cpModel
      : "Automatic";
    const cpModelDesc = cpModel
      ? cpModelInfo?.vendor
        ? `AI commit messages use ${cpModelInfo.vendor}`
        : "AI commit messages use the selected chat model"
      : "AI commit messages use VS Code's automatic model choice";
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
        desc: "Sign commits with GPG so GitHub shows a Verified badge",
        value: cpSign,
      },
      {
        key: "useAI",
        label: "AI Messages",
        desc: "Generate commit messages with AI (disable to require -m)",
        value: cpUseAI,
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
      .join("") + `
        <div class="tool-item tool-item--interactive cp-model-row${cpUseAI ? "" : " cp-model-row--dim"}" data-cpmodel="picker" role="button" tabindex="${cpUseAI ? "0" : "-1"}" aria-disabled="${cpUseAI ? "false" : "true"}" title="Change checkpoint AI model">
          <div class="cp-model-spacer" aria-hidden="true"></div>
          <div class="tool-text">
            <span class="tl">Checkpoint model</span>
            <span class="td">${escapeHtml(cpModelDesc)}</span>
          </div>
          <div class="cp-model-meta">
            <span class="cp-model-chip${cpModel ? " active" : ""}">${escapeHtml(cpModelLabel)}</span>
            <svg class="cp-model-chevron" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 0 1 1.06 0l3.25 3.25a.75.75 0 0 1 0 1.06l-3.25 3.25a.75.75 0 0 1-1.06-1.06L8.94 8 6.22 5.28a.75.75 0 0 1 0-1.06z"/></svg>
          </div>
        </div>`;

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

    const enabledCount = TOOL_GROUPS.filter((group) =>
      isGroupEnabled(group.key),
    ).length;
    const strictLintingEnabled = isStrictLintingEnabled();
    const branchSessionsEnabled = vscode.workspace
      .getConfiguration("gitShellHelpers.branchSessions")
      .get("enabled", false);
    const sessionMemoryEnabled = vscode.workspace
      .getConfiguration("gitShellHelpers.sessionMemory")
      .get("enabled", true);
    const formatBypassEnabled = vscode.workspace
      .getConfiguration("gitShellHelpers.formatControl")
      .get("bypassOnAgentSave", false);

    const localSubagentsConfig = vscode.workspace.getConfiguration(
      "gitShellHelpers.localSubagents",
    );
    const ollamaDefaultModel = String(
      localSubagentsConfig.get("ollama.defaultModel", "") || "",
    ).trim();
    const ollamaMaxIter = localSubagentsConfig.get("ollama.maxIterations", 12);
    const ollamaAllowWrite = localSubagentsConfig.get(
      "ollama.allowWrite",
      false,
    );
    const ollamaAllowShell = localSubagentsConfig.get(
      "ollama.allowShell",
      false,
    );
    const fullSystemAccess = !!localSubagentsConfig.get(
      "fullSystemAccess",
      false,
    );
    const systemExecuteModel = String(
      localSubagentsConfig.get("systemExecute.defaultModel", "") || "",
    ).trim();
    const systemExecuteMaxIter = localSubagentsConfig.get(
      "systemExecute.maxIterations",
      25,
    );
    const systemExecuteHeadless = !!localSubagentsConfig.get(
      "systemExecute.browserHeadless",
      true,
    );
    const systemExecuteChannel = String(
      localSubagentsConfig.get("systemExecute.browserChannel", "chrome") ||
        "chrome",
    );

    const providerStatus = await getProviderStatus();
    const providerConfigured = [
      providerStatus.ollamaRunning,
      providerStatus.anthropicKey,
      providerStatus.openaiKey,
    ].filter(Boolean).length;

    const ollamaRows =
      providerStatus.ollamaRunning && providerStatus.ollamaModels.length > 0
        ? providerStatus.ollamaModels
            .filter((model) => deps._ollamaPinned.has(model))
            .map(
              (model) => `
        <div class="provider-model-row">
          <span class="provider-model-dot"></span>
          <span class="provider-model-name">${escapeHtml(model)}</span>
          <button class="provider-model-run" data-ollamarun="${escapeHtml(model)}" title="ollama run ${escapeHtml(model)}" type="button">run</button>
          <button class="provider-model-remove" data-ollamatoggle="${escapeHtml(model)}" title="Remove" type="button">x</button>
        </div>`,
            )
            .join("")
        : "";

    const ollamaAddBtn =
      providerStatus.ollamaRunning && providerStatus.ollamaModels.length > 0
        ? '<button class="provider-add-btn" id="ollamaAddModelsBtn" type="button">+ Add model</button>'
        : "";

    const ollamaAddPanel =
      providerStatus.ollamaRunning && providerStatus.ollamaModels.length > 0
        ? `<div class="provider-acc-panel" id="ollamaAccPanel"><div class="ollama-models">${providerStatus.ollamaModels
            .map((model) => {
              const pinned = deps._ollamaPinned.has(model);
              return `<div class="ollama-model-row${pinned ? " on" : ""}">
            <span class="ollama-model-check">&#10003;</span>
            <button class="ollama-tag${pinned ? " on" : ""}" data-ollamatoggle="${escapeHtml(model)}" type="button">${escapeHtml(model)}</button>
          </div>`;
            })
            .join("")}</div></div>`
        : "";

    const ollamaStatusRow = !providerStatus.ollamaRunning
      ? '<div class="provider-row provider-row-dim provider-row-clickable" id="ollamaRefreshChip" title="Click to recheck"><span class="provider-row-dot"></span><span class="provider-row-label">Ollama not running</span><span class="provider-row-action">recheck</span></div>'
      : "";

    const anthropicRow = `
      <div class="provider-row${providerStatus.anthropicKey ? " provider-row-set" : ""}">
        <span class="provider-row-dot${providerStatus.anthropicKey ? " set" : ""}"></span>
        <span class="provider-row-label">Anthropic</span>
        <button class="provider-row-action provider-chip-clickable" id="anthropicChipBtn" data-acc="anthropic" type="button">${providerStatus.anthropicKey ? "change key" : "add key"}</button>
      </div>
      <div class="provider-acc-panel" id="anthropicAccPanel">
        <div class="key-input-row">
          <input class="key-input" id="anthropicKeyInput" type="password"
            placeholder="${providerStatus.anthropicKey ? "&#9679;&#9679;&#9679;&#9679;&#9679;&#9679;&#9679;&#9679; (saved)" : "sk-ant-..."}"
            autocomplete="off" data-provider="anthropic" />
          <button class="key-save-btn" data-savekey="anthropic" type="button">Save</button>
          ${providerStatus.anthropicKey ? '<button class="key-clear-btn" data-clearkey="anthropic" type="button">Clear</button>' : ""}
        </div>
      </div>`;

    const openaiRow = `
      <div class="provider-row${providerStatus.openaiKey ? " provider-row-set" : ""}">
        <span class="provider-row-dot${providerStatus.openaiKey ? " set" : ""}"></span>
        <span class="provider-row-label">OpenAI</span>
        <button class="provider-row-action provider-chip-clickable" id="openaiChipBtn" data-acc="openai" type="button">${providerStatus.openaiKey ? "change key" : "add key"}</button>
      </div>
      <div class="provider-acc-panel" id="openaiAccPanel">
        <div class="key-input-row">
          <input class="key-input" id="openaiKeyInput" type="password"
            placeholder="${providerStatus.openaiKey ? "&#9679;&#9679;&#9679;&#9679;&#9679;&#9679;&#9679;&#9679; (saved)" : "sk-..."}"
            autocomplete="off" data-provider="openai" />
          <button class="key-save-btn" data-savekey="openai" type="button">Save</button>
          ${providerStatus.openaiKey ? '<button class="key-clear-btn" data-clearkey="openai" type="button">Clear</button>' : ""}
        </div>
      </div>`;

    const allAgents = scanLocalAgents().filter((agent) => agent.userInvocable);
    const agentRows =
      allAgents.length > 0
        ? allAgents
            .map(
              (agent, index) => `
        <div class="agent-item${index >= 3 ? " agent-overflow" : ""}" data-agent="${escapeHtml(agent.name)}">
          <div class="agent-dot"></div>
          <div class="agent-text">
            <span class="agent-name"><span class="agent-at">@</span>${escapeHtml(agent.name)}</span>
            ${agent.description ? `<span class="agent-desc">${escapeHtml(agent.description)}</span>` : ""}
          </div>
          <button class="agent-start-btn" data-agentname="${escapeHtml(agent.name)}" title="Open @${escapeHtml(agent.name)} in Copilot chat" type="button">
            <svg viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M3.5 2A1.5 1.5 0 0 0 2 3.5v9A1.5 1.5 0 0 0 3.5 14h9a1.5 1.5 0 0 0 1.5-1.5V8.75a.75.75 0 0 0-1.5 0v3.75h-9v-9H8a.75.75 0 0 0 0-1.5H3.5zm7.25.25a.75.75 0 0 0 0 1.5H12.2L7.47 8.47a.75.75 0 0 0 1.06 1.06L13 5.05v1.45a.75.75 0 0 0 1.5 0V2.75a.5.5 0 0 0-.5-.5h-3.25z"/></svg>
          </button>
        </div>`,
            )
            .join("") +
          (allAgents.length > 3
            ? `<button class="view-more-btn" id="viewMoreAgentsBtn" type="button">+ ${allAgents.length - 3} more</button>`
            : "")
        : '<div class="muted">No agents found in .github/agents/</div>';

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

    const formatBypassRow = `
      <div class="tool-item${formatBypassEnabled ? " active" : ""}" data-format-bypass="enabled">
        <div class="cb${formatBypassEnabled ? " on" : ""}"><div class="cb-tick"></div></div>
        <div class="tool-text">
          <span class="tl">Bypass Formatters on Agent Save</span>
          <span class="td">Suppress Prettier/ESLint on every save; format once at end of request</span>
        </div>
      </div>`;

    const modeOptions = MODES.map(
      (item) =>
        `<option value="${item.value}"${item.value === mode ? " selected" : ""}>${item.label}</option>`,
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
              .map((repo) => `<div class="repo-item">${escapeHtml(repo)}</div>`)
              .join("")
          : '<div class="muted">No repositories selected</div>';
      scopeSection = `
        <div class="sub-label">Whitelisted Repositories</div>
        ${repoList}
        <button class="btn-secondary" id="selectReposBtn">Select repositories...</button>`;
    } else if (mode === "auto-submit-only-public") {
      const publicCount = getCachedRepos().filter(
        (repo) => repo.visibility === "PUBLIC",
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

    const quickActionsHtml = QUICK_ACTIONS.map(
      (action) => `
      <div class="qa-item" data-qaaction="${escapeHtml(action.id)}">
        <div class="qa-icon">
          <svg viewBox="0 0 16 16" fill="currentColor"><path d="${escapeHtml(action.iconPath)}"/></svg>
        </div>
        <div class="qa-text">
          <span class="qa-label">${escapeHtml(action.label)}</span>
          <span class="qa-desc">${escapeHtml(action.desc)}</span>
        </div>
        <button class="qa-run-btn" data-qa="${escapeHtml(action.id)}" title="Run in chat" type="button">
          <svg viewBox="0 0 16 16" fill="currentColor"><path d="M3 2.5A.5.5 0 0 1 3.5 2l10 5.5a.5.5 0 0 1 0 .87l-10 5.5A.5.5 0 0 1 3 13.5v-11z"/></svg>
        </button>
      </div>`,
    ).join("");

    // ─── Local sub-agents UI ───────────────────────────────────────────
    const ollamaModelOptions = (() => {
      const allModels = providerStatus.ollamaModels || [];
      if (!allModels.length) {
        return `<option value="" selected>(no models installed)</option>`;
      }
      const opts = [
        `<option value=""${ollamaDefaultModel ? "" : " selected"}>(require explicit model)</option>`,
      ];
      for (const model of allModels) {
        const sel = model === ollamaDefaultModel ? " selected" : "";
        opts.push(`<option value="${escapeHtml(model)}"${sel}>${escapeHtml(model)}</option>`);
      }
      return opts.join("");
    })();

    const ollamaSubagentBlock = `
      <div class="provider-row${providerStatus.ollamaRunning ? " provider-row-set" : ""}">
        <span class="provider-row-dot${providerStatus.ollamaRunning ? " set" : ""}"></span>
        <span class="provider-row-label">Ollama sub-agent</span>
        <span class="provider-row-status">${providerStatus.ollamaRunning ? `${providerStatus.ollamaModels.length} model${providerStatus.ollamaModels.length === 1 ? "" : "s"}` : "not running"}</span>
      </div>
      <div class="local-sub-panel">
        <label class="local-sub-label">Default model
          <select class="local-sub-input" id="ollamaSubagentModel" data-localsub="ollama.defaultModel">${ollamaModelOptions}</select>
        </label>
        <label class="local-sub-label">Max iterations
          <input class="local-sub-input" type="number" min="1" max="50" id="ollamaSubagentMaxIter" data-localsub="ollama.maxIterations" value="${escapeHtml(String(ollamaMaxIter))}" />
        </label>
        <div class="tool-item${ollamaAllowWrite ? " active" : ""}" data-localsubtoggle="ollama.allowWrite">
          <div class="cb${ollamaAllowWrite ? " on" : ""}"><div class="cb-tick"></div></div>
          <div class="tool-text">
            <span class="tl">Allow file writes</span>
            <span class="td">Let the local sub-agent write files inside this workspace</span>
          </div>
        </div>
        <div class="tool-item${ollamaAllowShell ? " active" : ""}" data-localsubtoggle="ollama.allowShell">
          <div class="cb${ollamaAllowShell ? " on" : ""}"><div class="cb-tick"></div></div>
          <div class="tool-text">
            <span class="tl">Allow shell commands</span>
            <span class="td">Let the local sub-agent run shell commands (60s timeout, workspace cwd)</span>
          </div>
        </div>
      </div>`;

    const channelOptions = ["chrome", "msedge", "chromium"]
      .map(
        (c) =>
          `<option value="${c}"${c === systemExecuteChannel ? " selected" : ""}>${c}</option>`,
      )
      .join("");

    const visionModelHint = systemExecuteModel
      ? ""
      : `<div class="hint">Pick a vision-capable model so the agent can see browser screenshots: qwen2.5vl:7b, llava:13b, llama3.2-vision:11b. Pull one with <code>ollama pull qwen2.5vl:7b</code>.</div>`;
    const playwrightHint = `<div class="hint">Browser tools require Playwright. One-time install: <code>npm install -g playwright && npx playwright install chromium</code></div>`;

    const systemExecuteBlock = `
      <div class="provider-row${fullSystemAccess ? " provider-row-set" : ""}">
        <span class="provider-row-dot${fullSystemAccess ? " set" : ""}"></span>
        <span class="provider-row-label">system_execute</span>
        <span class="provider-row-status">${fullSystemAccess ? "armed · full system access" : "disarmed"}</span>
      </div>
      <div class="tool-item${fullSystemAccess ? " active" : ""}" data-localsubtoggle="fullSystemAccess">
        <div class="cb${fullSystemAccess ? " on" : ""}"><div class="cb-tick"></div></div>
        <div class="tool-text">
          <span class="tl">Full system access (master switch)</span>
          <span class="td">Lets Copilot dispatch a free local Ollama agent that can run any shell command, read/write any file, and drive a real browser autonomously. Use for: log into a site to fetch a credential, drive a UI, run a long pipeline.</span>
        </div>
      </div>
      <div class="local-sub-panel">
        <label class="local-sub-label">Default model
          <input class="local-sub-input" type="text" data-localsub="systemExecute.defaultModel" value="${escapeHtml(systemExecuteModel)}" placeholder="qwen2.5vl:7b" />
        </label>
        <label class="local-sub-label">Max iterations
          <input class="local-sub-input" type="number" min="1" max="100" data-localsub="systemExecute.maxIterations" value="${escapeHtml(String(systemExecuteMaxIter))}" />
        </label>
        <label class="local-sub-label">Browser channel
          <select class="local-sub-input" data-localsub="systemExecute.browserChannel">${channelOptions}</select>
        </label>
        <div class="tool-item${systemExecuteHeadless ? " active" : ""}" data-localsubtoggle="systemExecute.browserHeadless">
          <div class="cb${systemExecuteHeadless ? " on" : ""}"><div class="cb-tick"></div></div>
          <div class="tool-text">
            <span class="tl">Headless browser</span>
            <span class="td">Turn off to watch the agent click through pages in a visible window</span>
          </div>
        </div>
        ${visionModelHint}
        ${playwrightHint}
      </div>`;

    const localSubagentCount =
      (providerStatus.ollamaRunning ? 1 : 0) + (fullSystemAccess ? 1 : 0);

    const replacements = {
      ...common,
      QUICK_ACTIONS_HTML: quickActionsHtml,
      ACTIVITY_SECTION_CLASS: activityItems.length === 0 ? " sect--idle" : "",
      ACTIVITY_COUNT_LABEL: escapeHtml(activityCountLabel),
      ACTIVITY_LIST_CLASS:
        activityItems.length === 0 ? " activity-list-hidden" : "",
      AGENT_COUNT: String(allAgents.length),
      AGENT_ROWS: agentRows,
      PROVIDER_CONFIGURED: String(providerConfigured),
      OLLAMA_STATUS_ROW: ollamaStatusRow,
      OLLAMA_ROWS: ollamaRows,
      OLLAMA_ADD_BTN: ollamaAddBtn,
      OLLAMA_ADD_PANEL: ollamaAddPanel,
      ANTHROPIC_ROW: anthropicRow,
      OPENAI_ROW: openaiRow,
      MCP_STATUS_HTML: mcpStatusHtml,
      ENABLED_COUNT: String(enabledCount),
      TOOL_GROUP_TOTAL: String(TOOL_GROUPS.length),
      TOOL_ROWS: toolRows,
      CP_ROWS: cpRows,
      GPG_HINT: gpgHint,
      CHAT_TOOL_COUNT: String(
        [
          strictLintingEnabled,
          branchSessionsEnabled,
          sessionMemoryEnabled,
          formatBypassEnabled,
        ].filter(Boolean).length,
      ),
      CHAT_TOOL_TOTAL: "4",
      STRICT_LINTING_ROW: strictLintingRow,
      BRANCH_SESSIONS_ROW: branchSessionsRow,
      SESSION_MEMORY_ROW: sessionMemoryRow,
      FORMAT_BYPASS_ROW: formatBypassRow,
      LOCAL_SUBAGENT_COUNT: String(localSubagentCount),
      LOCAL_SUBAGENT_OLLAMA_BLOCK: ollamaSubagentBlock,
      LOCAL_SUBAGENT_SYSTEM_EXECUTE_BLOCK: systemExecuteBlock,
      MODE_OPTIONS: modeOptions,
      MODE_DESC: escapeHtml(modeDesc),
      SCOPE_SECTION: scopeSection,
      CACHED_USER: escapeHtml(getCachedUser()),
    };

    return renderTemplate(
      readTemplate(extensionUri, "community-cache-panel.html"),
      replacements,
    );
  };
};
