#!/usr/bin/env bash
set -euo pipefail
SRC="/Users/alexwaldmann/bin/vscode-extension/extension.js"
OUT="/Users/alexwaldmann/bin-refactor-modular/vscode-extension/extension.js.new"

{
  # Section 1: Header — imports, constants, module-level state (lines 1-46)
  awk 'NR>=1 && NR<=46' "$SRC"

  # Injected: module require statements
  printf '\n// ── Module loading ──────────────────────────────────────────────────────────\n'
  printf 'const createCopilotInspector = require("./src/copilot-inspector");\n'
  printf 'const createMcpServer = require("./src/mcp-server");\n'
  printf 'const createGpgAuth = require("./src/gpg-auth");\n'
  printf 'const createWebviewProviderClass = require("./src/webview-provider");\n'
  printf '\n'

  # Section 2: Community settings, gh CLI helpers, QUICK_ACTIONS, MODES (lines 852-1060)
  printf '// ── Community settings and gh CLI helpers ───────────────────────────────\n'
  awk 'NR>=852 && NR<=1060' "$SRC"
  printf '\n'

  # Section 3: escapeHtml, MCP tools config, TOOL_GROUPS, read/writeToolsConfig,
  #            isGroupEnabled, setGroupEnabled, isStrictLintingEnabled (lines 2417-2533)
  printf '// ── MCP tools configuration ──────────────────────────────────────────────\n'
  awk 'NR>=2417 && NR<=2533' "$SRC"
  printf '\n'

  # Section 4: Tool call tracking, chat sessions, models, agents,
  #            checkpoint, IPC helpers (lines 3070-4009)
  printf '// ── Tool call tracking, chat sessions, models, agents, IPC ────────────\n'
  awk 'NR>=3070 && NR<=4009' "$SRC"
  printf '\n'

  # Injected: module wiring — placed after all function declarations and consts
  printf '// ── Module wiring ───────────────────────────────────────────────────────────\n'
  printf 'const inspector = createCopilotInspector({\n'
  printf '  getDiagnosticsChannel: () => _diagnosticsOutputChannel,\n'
  printf '  setDiagnosticsChannel: (ch) => { _diagnosticsOutputChannel = ch; },\n'
  printf '  getInspectorDisposable: () => _customizationInspectorToolDisposable,\n'
  printf '  setInspectorDisposable: (d) => { _customizationInspectorToolDisposable = d; },\n'
  printf '  beginToolCall,\n'
  printf '  endToolCall,\n'
  printf '});\n'
  printf 'const {\n'
  printf '  uniquePaths, getFrontmatterRange, getFrontmatterListEntries,\n'
  printf '  formatHoverContents, makeToolResult, formatDiagnosticSeverity,\n'
  printf '  isCustomizationInspectorEnabled, formatCustomizationInspectionReport,\n'
  printf '  resolveCustomizationDocument, inspectCopilotCustomizationWarnings,\n'
  printf '  runStrictLinting, registerCustomizationInspectorTool,\n'
  printf '  getDiagnosticsOutputChannel,\n'
  printf '} = inspector;\n'
  printf '\n'
  printf 'const mcpServer = createMcpServer({\n'
  printf '  GLOBAL_MCP_SERVER_PATH,\n'
  printf '  MCP_PROVIDER_ID,\n'
  printf '  uniquePaths,\n'
  printf '});\n'
  printf 'const {\n'
  printf '  findGitShellHelpersMcpPath, buildGitShellHelpersMcpEnv,\n'
  printf '  registerMcpServerProvider, globalSettingsPath, workspaceSettingsPath,\n'
  printf '  workspaceManifestPath, readJsonFile, writeJsonFile,\n'
  printf '  userMcpConfigPath, workspaceMcpConfigPaths,\n'
  printf '  removeStaticGitShellHelpersServers, migrateLegacyMcpRegistrations,\n'
  printf '  getConfiguredGitShellHelpersMcpServer, getMcpStatusViewModel,\n'
  printf '  openMcpServerControls,\n'
  printf '} = mcpServer;\n'
  printf '\n'
  printf 'const gpgAuth = createGpgAuth({\n'
  printf '  getCachedRepos: () => cachedRepos,\n'
  printf '  setCachedRepos: (v) => { cachedRepos = v; },\n'
  printf '  getCachedUser: () => cachedUser,\n'
  printf '  setCachedUser: (v) => { cachedUser = v; },\n'
  printf '  getCachedGpgNeedsUpload: () => cachedGpgNeedsUpload,\n'
  printf '  setCachedGpgNeedsUpload: (v) => { cachedGpgNeedsUpload = v; },\n'
  printf '  getCachedGpgUploadFailed: () => cachedGpgUploadFailed,\n'
  printf '  setCachedGpgUploadFailed: (v) => { cachedGpgUploadFailed = v; },\n'
  printf '  getWebviewProvider: () => _webviewProvider,\n'
  printf '  runGh, isGhAuthed, getGhUser, fetchRepos,\n'
  printf '  getWhitelist, getMode, setWhitelist, buildSettingsJson, syncAllSettings,\n'
  printf '  readJsonFile, writeJsonFile, globalSettingsPath, workspaceSettingsPath,\n'
  printf '  SCHEMA_VERSION, PREDEFINED,\n'
  printf '});\n'
  printf 'const {\n'
  printf '  loginGitHub, logoutGitHub, selectRepos, showCommunityStatus,\n'
  printf '  checkGpgUploadStatus, uploadGpgKeyNow, ensureGpgAvailable, ensureGpgKey,\n'
  printf '} = gpgAuth;\n'
  printf '\n'
  printf 'const CommunityCacheViewProvider = createWebviewProviderClass({\n'
  printf '  loginGitHub, logoutGitHub, selectRepos, setMode, setGroupEnabled,\n'
  printf '  ensureGpgKey, openMcpServerControls, openModelPicker, refreshModels,\n'
  printf '  openAgentInChat, runQuickAction, openQuickActionWithoutSend,\n'
  printf '  setApiKey, detectOllama, uploadGpgKeyNow,\n'
  printf '  getMode, getWhitelist, getMcpStatusViewModel,\n'
  printf '  escapeHtml, isGroupEnabled, isStrictLintingEnabled,\n'
  printf '  getProviderStatus, scanLocalAgents, getActivityItems,\n'
  printf '  API_KEY_ANTHROPIC, API_KEY_OPENAI, TOOL_GROUPS, MODES, QUICK_ACTIONS,\n'
  printf '  get _ollamaPinned() { return _ollamaPinned; },\n'
  printf '  get _context() { return _context; },\n'
  printf '  getCachedUser: () => cachedUser,\n'
  printf '  setCachedUser: (v) => { cachedUser = v; },\n'
  printf '  getCachedRepos: () => cachedRepos,\n'
  printf '  setCachedRepos: (v) => { cachedRepos = v; },\n'
  printf '  getCachedGpgNeedsUpload: () => cachedGpgNeedsUpload,\n'
  printf '  getCachedGpgUploadFailed: () => cachedGpgUploadFailed,\n'
  printf '  setCachedGpgUploadFailed: (v) => { cachedGpgUploadFailed = v; },\n'
  printf '});\n'
  printf '\n'

  # Section 5: activate, IPC servers, deactivate, module.exports (lines 4010-4386)
  printf '// ── Activation ──────────────────────────────────────────────────────────\n'
  awk 'NR>=4010 && NR<=4386' "$SRC"

} > "$OUT"

echo "Done. New file: $OUT ($(wc -l < "$OUT") lines)"