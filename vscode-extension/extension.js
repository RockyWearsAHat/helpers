// Git Shell Helpers — VS Code extension
//
// Provides a "Community Cache" webview panel in the Explorer sidebar with
// styled buttons for GitHub sign-in/out, mode selection, and repo whitelist.
//
// Settings sync:
//   User settings   → ~/.copilot/devops-audit-community-settings.json
//   Workspace settings → .github/devops-audit-community-settings.json

const vscode = require("vscode");
const fs = require("fs");
const net = require("net");
const os = require("os");
const path = require("path");
const { execFile } = require("child_process");
const createWebviewProviderClass = require("./src/webview-provider");

const SCHEMA_VERSION = 1;
const PREDEFINED = {
  baseBranch: "main",
  branchPrefix: "automation/community-cache-submission",
};

let cachedRepos = [];
let cachedUser = "";
let cachedGpgNeedsUpload = false;
let cachedGpgUploadFailed = false;
let cachedModels = [];
let cachedOllamaModels = []; // [] | string[] of model names
let cachedOllamaRunning = false;
let _ollamaPinned = new Set(); // model names the user has enabled/pinned
let activeToolCalls = new Map(); // id → { id, tool, label, startedAt, args }
let _activitySeq = 0;
// Chat sessions are tracked from VS Code's chatSessions JSONL files.
// Detection is content-based: parse JSONL records to check if the last request
// has a modelState.value===1 completion record (no mtime heuristics).
let _chatSessions = new Map(); // sessionId → { title, active, startedAt, completedAt, filePath, lastSize, preview, requestCount }
let _chatSessionWatcher = null;
let _chatSessionPoller = null;
let _context = null;
let _webviewProvider = null;
let _diagnosticsOutputChannel = null;
let _customizationInspectorToolDisposable = null;
let _strictLintIpcServer = null;
let _activityIpcServer = null;
const _externalToInternal = new Map(); // externalId → internalId
let _sessionStartedAt = 0;

// Worktree-to-chat-session bindings — maps chat session IDs to worktree info.
// Persisted in context.globalState under "worktreeBindings" so bindings survive
// extension reloads. The extension manages workspace folders based on these bindings.
let _worktreeBindings = new Map(); // sessionId → { branch, worktreePath, baseBranch, baseCommit, createdAt }
let _chatTabToSession = new Map(); // tabUri.toString() → sessionId (built by temporal correlation + title fallback)
let _activeWorktreeFolder = null; // currently focused worktree path, or null
let _originalWorkspaceUri = null; // URI of the workspace folder to return to when leaving a worktree view
let _focusedSessionId = null; // session ID whose worktree currently has Explorer focus
// Focus switching is driven entirely by JSONL file activity — see
// _maybeUpdateWorktreeFocus in the "Activity-Driven Worktree Focus" section.

const MCP_PROVIDER_ID = "gitShellHelpers.mcpServers";
const GLOBAL_MCP_SERVER_PATH = "/usr/local/bin/git-shell-helpers-mcp";

function uniquePaths(paths) {
  return [...new Set(paths.filter(Boolean))];
}

function getDiagnosticsOutputChannel() {
  if (!_diagnosticsOutputChannel) {
    _diagnosticsOutputChannel = vscode.window.createOutputChannel(
      "Git Shell Helpers Diagnostics",
    );
  }
  return _diagnosticsOutputChannel;
}

function getFrontmatterRange(text) {
  if (!text.startsWith("---\n") && !text.startsWith("---\r\n")) {
    return null;
  }

  const lines = text.split(/\r?\n/);
  if (!lines.length || lines[0].trim() !== "---") {
    return null;
  }

  for (let index = 1; index < lines.length; index += 1) {
    if (lines[index].trim() === "---") {
      return { startLine: 0, endLine: index };
    }
  }

  return null;
}

function getFrontmatterListEntries(document, key) {
  const frontmatter = getFrontmatterRange(document.getText());
  if (!frontmatter) {
    return [];
  }

  const entries = [];
  let insideTargetList = false;
  let baseIndent = 0;

  for (
    let lineIndex = frontmatter.startLine + 1;
    lineIndex < frontmatter.endLine;
    lineIndex += 1
  ) {
    const line = document.lineAt(lineIndex).text;
    const trimmed = line.trim();

    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }

    const indent = line.length - line.trimStart().length;

    if (!insideTargetList && trimmed === `${key}:`) {
      insideTargetList = true;
      baseIndent = indent;
      continue;
    }

    if (!insideTargetList) {
      continue;
    }

    if (indent <= baseIndent && !trimmed.startsWith("- ")) {
      break;
    }

    const match = line.match(/^(\s*)-\s+(.+?)\s*$/);
    if (!match) {
      if (indent <= baseIndent) {
        break;
      }
      continue;
    }

    const value = match[2].trim().replace(/^['"]|['"]$/g, "");
    const valueColumn = line.indexOf(value);
    if (valueColumn >= 0) {
      entries.push({
        key,
        value,
        line: lineIndex,
        column: valueColumn,
      });
    }
  }

  return entries;
}

function formatHoverContents(hovers) {
  const rendered = [];
  for (const hover of hovers || []) {
    for (const item of hover.contents || []) {
      if (typeof item === "string") {
        rendered.push(item);
      } else if (item?.value) {
        rendered.push(item.value);
      }
    }
  }
  return rendered.join("\n---\n").trim();
}

function makeToolResult(value) {
  return new vscode.LanguageModelToolResult([
    new vscode.LanguageModelTextPart(value),
  ]);
}

function formatDiagnosticSeverity(severity) {
  switch (severity) {
    case vscode.DiagnosticSeverity.Error:
      return "error";
    case vscode.DiagnosticSeverity.Warning:
      return "warning";
    case vscode.DiagnosticSeverity.Information:
      return "info";
    case vscode.DiagnosticSeverity.Hint:
      return "hint";
    default:
      return "diagnostic";
  }
}

function isCustomizationInspectorEnabled() {
  return vscode.workspace
    .getConfiguration("gitShellHelpers.customizationInspector")
    .get("enabled", true);
}

function formatCustomizationInspectionReport(result) {
  if (!result?.ok) {
    if (result?.reason === "no-active-editor") {
      return [
        "Strict Linting",
        "",
        "No active editor. Open a Copilot customization file first.",
      ].join("\n");
    }
    if (result?.reason === "no-tools-list") {
      return [
        "Strict Linting",
        "",
        `No frontmatter tools list found in ${result.file || "the active file"}.`,
      ].join("\n");
    }
    return "Strict Linting\n\nInspection did not return any result.";
  }

  const entries = result.results || [];
  const errorCount = entries.reduce(
    (count, entry) =>
      count +
      (entry.diagnostics || []).filter(
        (diagnostic) => diagnostic.severity === vscode.DiagnosticSeverity.Error,
      ).length,
    0,
  );
  const warningCount = entries.reduce(
    (count, entry) =>
      count +
      (entry.diagnostics || []).filter(
        (diagnostic) =>
          diagnostic.severity === vscode.DiagnosticSeverity.Warning,
      ).length,
    0,
  );
  const infoCount = entries.reduce(
    (count, entry) =>
      count +
      (entry.diagnostics || []).filter(
        (diagnostic) =>
          diagnostic.severity !== vscode.DiagnosticSeverity.Error &&
          diagnostic.severity !== vscode.DiagnosticSeverity.Warning,
      ).length,
    0,
  );
  const codeActionCount = entries.reduce(
    (count, entry) => count + (entry.codeActions?.length || 0),
    0,
  );
  const hoverCount = entries.reduce(
    (count, entry) => count + (entry.hoverText ? 1 : 0),
    0,
  );

  const lines = [
    "Strict Linting",
    "",
    `File: ${result.file}`,
    `Summary: ${errorCount} error(s), ${warningCount} warning(s), ${infoCount} other diagnostic(s), ${codeActionCount} quick fix(es), ${hoverCount} hover note(s).`,
    "",
  ];

  for (const entry of entries) {
    lines.push(`tools -> ${entry.value} (${entry.line}:${entry.column})`);

    if (entry.diagnostics?.length) {
      lines.push("Diagnostics:");
      for (const diagnostic of entry.diagnostics) {
        lines.push(
          `- [${formatDiagnosticSeverity(diagnostic.severity)}${diagnostic.source ? ` | ${diagnostic.source}` : ""}] ${diagnostic.message}`,
        );
      }
    }

    if (entry.hoverText) {
      lines.push("Hover:");
      lines.push(entry.hoverText);
    }

    if (entry.codeActions?.length) {
      lines.push("Code Actions:");
      for (const action of entry.codeActions) {
        lines.push(`- ${action}`);
      }
    }

    if (!entry.hasSignal) {
      lines.push("No diagnostics, hover text, or code actions returned.");
    }

    lines.push("");
  }

  return lines.join("\n").trim();
}

async function resolveCustomizationDocument(filePath) {
  const explicitPath = String(filePath || "").trim();
  if (explicitPath) {
    return vscode.workspace.openTextDocument(explicitPath);
  }

  const editor = vscode.window.activeTextEditor;
  if (editor) {
    return editor.document;
  }

  return null;
}

async function inspectCopilotCustomizationWarnings(options = {}) {
  const normalizedOptions =
    typeof options === "string" ? { filePath: options } : options;
  const filePath = normalizedOptions.filePath || "";
  const revealOutput = normalizedOptions.revealOutput === true;
  const notify = normalizedOptions.notify !== false;

  const document = await resolveCustomizationDocument(filePath);
  if (!document) {
    if (notify) {
      vscode.window.showWarningMessage("Open a customization file first.");
    }
    return { ok: false, reason: "no-active-editor" };
  }

  const entries = getFrontmatterListEntries(document, "tools");
  if (entries.length === 0) {
    if (notify) {
      vscode.window.showInformationMessage(
        "No frontmatter tools list found in the active file.",
      );
    }
    return { ok: false, reason: "no-tools-list", file: document.uri.fsPath };
  }

  const allDiagnostics = vscode.languages.getDiagnostics(document.uri);
  const output = getDiagnosticsOutputChannel();
  output.clear();
  output.appendLine(`File: ${document.uri.fsPath}`);
  output.appendLine("");

  let foundSignal = false;
  const results = [];
  for (const entry of entries) {
    const position = new vscode.Position(entry.line, entry.column);
    const range = new vscode.Range(position, position);
    const diagnostics = allDiagnostics.filter((diagnostic) =>
      diagnostic.range.contains(position),
    );
    const hovers = await vscode.commands.executeCommand(
      "vscode.executeHoverProvider",
      document.uri,
      position,
    );
    const actions = await vscode.commands.executeCommand(
      "vscode.executeCodeActionProvider",
      document.uri,
      range,
    );

    const hoverText = formatHoverContents(hovers);
    const relevantActions = (actions || []).map((action) => action.title);
    const hasEntrySignal =
      diagnostics.length > 0 || hoverText || relevantActions.length > 0;
    foundSignal ||= hasEntrySignal;
    results.push({
      value: entry.value,
      line: entry.line + 1,
      column: entry.column + 1,
      diagnostics: diagnostics.map((diagnostic) => ({
        source: diagnostic.source || "unknown",
        message: diagnostic.message,
        severity: diagnostic.severity,
      })),
      hoverText,
      codeActions: relevantActions,
      hasSignal: hasEntrySignal,
    });

    output.appendLine(
      `tools -> ${entry.value} (${entry.line + 1}:${entry.column + 1})`,
    );

    if (diagnostics.length > 0) {
      output.appendLine("  Diagnostics:");
      for (const diagnostic of diagnostics) {
        output.appendLine(
          `    - [${diagnostic.source || "unknown"}] ${diagnostic.message}`,
        );
      }
    }

    if (hoverText) {
      output.appendLine("  Hover:");
      for (const line of hoverText.split("\n")) {
        output.appendLine(`    ${line}`);
      }
    }

    if (relevantActions.length > 0) {
      output.appendLine("  Code Actions:");
      for (const title of relevantActions) {
        output.appendLine(`    - ${title}`);
      }
    }

    if (!hasEntrySignal) {
      output.appendLine(
        "  No diagnostics, hover text, or code actions returned.",
      );
    }

    output.appendLine("");
  }

  if (revealOutput) {
    output.show(true);
  }

  if (notify && foundSignal) {
    vscode.window.showInformationMessage(
      "Strict Linting finished. See Git Shell Helpers Diagnostics output.",
    );
  } else if (notify) {
    vscode.window.showInformationMessage(
      "Strict Linting found no editor errors, warnings, or quick fixes for the tools list.",
    );
  }

  return {
    ok: true,
    file: document.uri.fsPath,
    foundSignal,
    results,
  };
}

async function runStrictLinting(options = {}) {
  const filePath = String(options.filePath || "").trim();
  const folderPath = String(options.folderPath || "").trim();
  const severityFilter = options.severityFilter || "all";

  const severityThreshold =
    severityFilter === "errors-only"
      ? vscode.DiagnosticSeverity.Error
      : severityFilter === "warnings-and-above"
        ? vscode.DiagnosticSeverity.Warning
        : vscode.DiagnosticSeverity.Hint;

  let diagnosticPairs;
  if (filePath) {
    const uri = vscode.Uri.file(filePath);
    diagnosticPairs = [[uri, vscode.languages.getDiagnostics(uri)]];
  } else if (folderPath) {
    const normalizedFolder = folderPath.endsWith("/")
      ? folderPath
      : folderPath + "/";
    diagnosticPairs = vscode.languages
      .getDiagnostics()
      .filter(
        ([uri]) =>
          uri.fsPath.startsWith(normalizedFolder) || uri.fsPath === folderPath,
      );
  } else {
    const workspaceRoots = (vscode.workspace.workspaceFolders || []).map(
      (f) => f.uri.fsPath,
    );
    const all = vscode.languages.getDiagnostics();
    diagnosticPairs =
      workspaceRoots.length > 0
        ? all.filter(([uri]) =>
            workspaceRoots.some((root) => uri.fsPath.startsWith(root)),
          )
        : all;
  }

  const filtered = diagnosticPairs
    .map(([uri, diags]) => [
      uri,
      diags.filter((d) => d.severity <= severityThreshold),
    ])
    .filter(([, diags]) => diags.length > 0);

  const totalErrors = filtered.reduce(
    (n, [, diags]) =>
      n +
      diags.filter((d) => d.severity === vscode.DiagnosticSeverity.Error)
        .length,
    0,
  );
  const totalWarnings = filtered.reduce(
    (n, [, diags]) =>
      n +
      diags.filter((d) => d.severity === vscode.DiagnosticSeverity.Warning)
        .length,
    0,
  );
  const totalOther = filtered.reduce(
    (n, [, diags]) =>
      n +
      diags.filter((d) => d.severity > vscode.DiagnosticSeverity.Warning)
        .length,
    0,
  );

  const scope = filePath
    ? path.basename(filePath)
    : folderPath
      ? folderPath
      : "workspace";

  const lines = [
    `Strict Linting — ${scope}`,
    "",
    `Summary: ${totalErrors} error(s), ${totalWarnings} warning(s), ${totalOther} other(s) across ${filtered.length} file(s).`,
    "",
  ];

  if (filtered.length === 0) {
    lines.push("No diagnostics found.");
  } else {
    for (const [uri, diags] of filtered) {
      lines.push(`File: ${uri.fsPath}`);
      for (const diag of diags) {
        const sev = formatDiagnosticSeverity(diag.severity);
        const src = diag.source ? ` [${diag.source}]` : "";
        const loc = `${diag.range.start.line + 1}:${diag.range.start.character + 1}`;
        lines.push(`  ${sev}${src} (${loc}): ${diag.message}`);
      }
      lines.push("");
    }
  }

  return lines.join("\n").trim();
}

let _strictLintToolDisposable = null;

function registerCustomizationInspectorTool(context) {
  _customizationInspectorToolDisposable?.dispose();
  _customizationInspectorToolDisposable = null;
  _strictLintToolDisposable?.dispose();
  _strictLintToolDisposable = null;

  if (!isCustomizationInspectorEnabled()) {
    return;
  }

  _customizationInspectorToolDisposable = vscode.lm.registerTool(
    "gsh-inspect-copilot-customization-warnings",
    {
      async invoke(options, token) {
        const filePath = options?.input?.filePath || "";
        const callId = beginToolCall(
          "inspect-customization",
          `Strict Linting: ${filePath ? path.basename(filePath) : "active editor"}`,
          { filePath: filePath || "(active editor)" },
        );
        try {
          const result = await inspectCopilotCustomizationWarnings({
            filePath,
            notify: false,
            revealOutput: false,
          });
          return makeToolResult(formatCustomizationInspectionReport(result));
        } finally {
          endToolCall(callId);
        }
      },
      async prepareInvocation(options) {
        const explicitPath = String(options?.input?.filePath || "").trim();
        const targetName = explicitPath
          ? path.basename(explicitPath)
          : path.basename(
              vscode.window.activeTextEditor?.document?.uri?.fsPath ||
                "customization file",
            );
        return {
          invocationMessage: `Strict Linting is reading live VS Code errors and warnings for ${targetName}`,
        };
      },
    },
  );
  context.subscriptions.push(_customizationInspectorToolDisposable);

  _strictLintToolDisposable = vscode.lm.registerTool("gsh-strict-lint", {
    async invoke(options, token) {
      const filePath = String(options?.input?.filePath || "").trim();
      const folderPath = String(options?.input?.folderPath || "").trim();
      const severityFilter = options?.input?.severityFilter || "all";
      const scope = filePath
        ? path.basename(filePath)
        : folderPath
          ? folderPath
          : "workspace";
      const callId = beginToolCall("strict-lint", `Strict Lint: ${scope}`, {
        filePath,
        folderPath,
        severityFilter,
      });
      try {
        const report = await runStrictLinting({
          filePath,
          folderPath,
          severityFilter,
        });
        return makeToolResult(report);
      } finally {
        endToolCall(callId);
      }
    },
    async prepareInvocation(options) {
      const filePath = String(options?.input?.filePath || "").trim();
      const folderPath = String(options?.input?.folderPath || "").trim();
      const scope = filePath
        ? path.basename(filePath)
        : folderPath
          ? folderPath
          : "workspace";
      return {
        invocationMessage: `Strict Linting — scanning ${scope} for errors and warnings`,
      };
    },
  });
  context.subscriptions.push(_strictLintToolDisposable);
}

// ---------------------------------------------------------------------------
// Branch-aware Language Model tool — inject branch state into Copilot chats
// so agents can query workspace state and manage their own branch isolation.
// Binding is automatic: when an agent calls branch_session_start (MCP), the
// extension auto-binds the active chat to that branch. No manual steps.
// ---------------------------------------------------------------------------

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

  // Branch sessions toggle — when disabled, hide branch worktree tools
  const branchEnabled = vscode.workspace
    .getConfiguration("gitShellHelpers.branchSessions")
    .get("enabled", false);
  if (!branchEnabled) {
    env.GSH_DISABLE_BRANCH_SESSIONS = "1";
  }

  // Pass workspace folder paths so the MCP server can resolve workspace
  // context (branch, worktree status) without relying on process.cwd().
  const folders = (vscode.workspace.workspaceFolders || []).map(
    (f) => f.uri.fsPath,
  );
  if (folders.length > 0) {
    env.GSH_WORKSPACE_ROOTS = JSON.stringify(folders);
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

function defaultCommunityRepoFromWorkspace(workspaceFolder) {
  const manifest = readJsonFile(workspaceManifestPath(workspaceFolder));
  return manifest?.defaultCommunityRepo || "";
}

function findLocalCommunityCloneFolder() {
  const folders = vscode.workspace.workspaceFolders || [];
  return (
    folders.find((folder) => fs.existsSync(workspaceManifestPath(folder))) ||
    null
  );
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
// Whitelist + settings sync
// ---------------------------------------------------------------------------

function getWhitelist() {
  return _context?.globalState.get("whitelistedRepos", []) ?? [];
}

function getMode() {
  return _context?.globalState.get("mode", "disabled") ?? "disabled";
}

async function setMode(mode) {
  await _context?.globalState.update("mode", mode);
  syncAllSettings();
  _webviewProvider?.refresh();
}

async function setWhitelist(repos) {
  await _context?.globalState.update("whitelistedRepos", repos);
  syncAllSettings();
  _webviewProvider?.refresh();
}

function buildSettingsJson() {
  const globalData = readJsonFile(globalSettingsPath()) || {};
  const localCloneFolder = findLocalCommunityCloneFolder();
  const derivedCommunityRepo =
    globalData.communityRepo ||
    (localCloneFolder
      ? defaultCommunityRepoFromWorkspace(localCloneFolder)
      : "") ||
    "RockyWearsAHat/github-shell-helpers";

  return {
    schemaVersion: SCHEMA_VERSION,
    communityRepo: derivedCommunityRepo,
    ...PREDEFINED,
    mode: getMode(),
    whitelistedRepos: getWhitelist(),
    shareResearch: isGroupEnabled("communityResearch"),
    ...(globalData.localClone
      ? { localClone: globalData.localClone }
      : localCloneFolder
        ? { localClone: localCloneFolder.uri.fsPath }
        : {}),
  };
}

function buildWorkspaceSettingsJson(workspaceFolder) {
  const globalSettings = buildSettingsJson();
  const workspaceCommunityRepo =
    defaultCommunityRepoFromWorkspace(workspaceFolder);

  return {
    ...globalSettings,
    ...(workspaceCommunityRepo
      ? { communityRepo: workspaceCommunityRepo }
      : {}),
    ...(fs.existsSync(workspaceManifestPath(workspaceFolder))
      ? { localClone: "." }
      : {}),
  };
}

function syncAllSettings() {
  writeJsonFile(globalSettingsPath(), buildSettingsJson());
  const folders = vscode.workspace.workspaceFolders;
  if (folders) {
    for (const folder of folders) {
      writeJsonFile(
        workspaceSettingsPath(folder),
        buildWorkspaceSettingsJson(folder),
      );
    }
  }
}

function importFromJson() {
  const currentMode = _context?.globalState.get("mode");
  // Migrate legacy "pull-only" → "disabled"
  if (currentMode === "pull-only") {
    _context?.globalState.update("mode", "disabled");
    return;
  }
  if (!currentMode) {
    const globalData = readJsonFile(globalSettingsPath());
    if (globalData?.mode) {
      _context?.globalState.update("mode", globalData.mode);
      if (Array.isArray(globalData.whitelistedRepos)) {
        _context?.globalState.update(
          "whitelistedRepos",
          globalData.whitelistedRepos,
        );
      }
      return;
    }
    const folders = vscode.workspace.workspaceFolders;
    if (folders) {
      for (const folder of folders) {
        const wsData = readJsonFile(workspaceSettingsPath(folder));
        if (wsData?.mode) {
          _context?.globalState.update("mode", wsData.mode);
          if (Array.isArray(wsData.whitelistedRepos)) {
            _context?.globalState.update(
              "whitelistedRepos",
              wsData.whitelistedRepos,
            );
          }
          return;
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Webview panel
// ---------------------------------------------------------------------------

const QUICK_ACTIONS = [
  {
    id: "runAudit",
    label: "Run Audit",
    desc: "Copilot customization audit",
    query: "/copilot-devops-audit",
    // SVG path for a magnifying-glass / audit icon
    iconPath:
      "M10.5 0a5.5 5.5 0 1 1 0 11 5.5 5.5 0 0 1 0-11zm0 1.5a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM.22 14.78a.75.75 0 0 1 0-1.06l4.5-4.5a.75.75 0 1 1 1.06 1.06l-4.5 4.5a.75.75 0 0 1-1.06 0z",
  },
];

const MODES = [
  { value: "disabled", label: "Submissions disabled" },
  { value: "pull-and-auto-submit", label: "Submit from all repos" },
  { value: "auto-submit-only-public", label: "Submit from public repos only" },
  {
    value: "auto-submit-whitelist",
    label: "Submit from whitelisted repos only",
  },
];

function escapeHtml(text) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

// ---------------------------------------------------------------------------
// MCP Tools config
// ---------------------------------------------------------------------------

const MCP_TOOLS_CONFIG_DIR = path.join(
  process.env.HOME || process.env.USERPROFILE || "",
  ".config",
  "git-shell-helpers-mcp",
);
const MCP_TOOLS_CONFIG_PATH = path.join(MCP_TOOLS_CONFIG_DIR, "tools.json");

// Tool groups: key → { label, description, tools[], alwaysOn? }
const TOOL_GROUPS = [
  {
    key: "knowledgeWrite",
    label: "Write Reusable Knowledge Locally",
    description: "Write, update & append knowledge notes",
    tools: [
      "write_knowledge_note",
      "update_knowledge_note",
      "append_to_knowledge_note",
    ],
  },
  {
    key: "communityResearch",
    label: "Share Knowledge Research",
    description: "Submit knowledge notes to community repo via PR",
    tools: ["submit_community_research"],
  },
  {
    key: "webSearch",
    label: "Web Search",
    description: "Search the web via SearXNG",
    tools: ["search_web"],
  },
  {
    key: "scrapeWebpage",
    label: "Scrape Webpage",
    description: "Fetch pages, strip HTML chrome, return clean text",
    tools: ["scrape_webpage"],
  },
  {
    key: "vision",
    label: "Vision",
    description:
      "Process images in-chat, allowing live analysis of visual output",
    tools: ["analyze_images"],
  },
  {
    key: "screenshot",
    label: "Screenshot",
    description:
      "Capture screenshots of the screen, an app window, or a region",
    tools: ["take_screenshot"],
  },
  {
    key: "checkpoint",
    label: "Git Checkpoint",
    description: "Commit working state via MCP tool — no terminal, no stalling",
    tools: ["checkpoint"],
  },
];

function readToolsConfig() {
  try {
    return JSON.parse(fs.readFileSync(MCP_TOOLS_CONFIG_PATH, "utf8"));
  } catch {
    return { disabledTools: [] };
  }
}

function writeToolsConfig(config) {
  fs.mkdirSync(MCP_TOOLS_CONFIG_DIR, { recursive: true });
  fs.writeFileSync(
    MCP_TOOLS_CONFIG_PATH,
    JSON.stringify(config, null, 2) + "\n",
    "utf8",
  );
}

function isGroupEnabled(groupKey) {
  const group = TOOL_GROUPS.find((g) => g.key === groupKey);
  if (!group || group.alwaysOn) return true;
  const config = readToolsConfig();
  const disabled = config.disabledTools || [];
  return !group.tools.some((t) => disabled.includes(t));
}

function setGroupEnabled(groupKey, enabled) {
  const group = TOOL_GROUPS.find((g) => g.key === groupKey);
  if (!group || group.alwaysOn) return;
  const config = readToolsConfig();
  const disabled = new Set(config.disabledTools || []);
  for (const tool of group.tools) {
    if (enabled) disabled.delete(tool);
    else disabled.add(tool);
  }
  config.disabledTools = [...disabled];
  writeToolsConfig(config);
}

function isStrictLintingEnabled() {
  return isCustomizationInspectorEnabled();
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

async function loginGitHub() {
  if (cachedUser) {
    vscode.window.showInformationMessage(`Already signed in as ${cachedUser}.`);
    _webviewProvider?.refresh();
    return;
  }
  try {
    const GITHUB_SCOPES = [
      "repo",
      "gist",
      "read:org",
      "workflow",
      "write:gpg_key",
    ];
    const session = await vscode.authentication.getSession(
      "github",
      GITHUB_SCOPES,
      {
        createIfNone: true,
      },
    );
    if (!session) return;

    cachedUser = session.account.label;

    // Forward token to gh CLI for shell script compatibility
    try {
      await new Promise((resolve, reject) => {
        const proc = execFile(
          "gh",
          ["auth", "login", "--with-token"],
          { timeout: 10000 },
          (err) => (err ? reject(err) : resolve()),
        );
        proc.stdin.write(session.accessToken);
        proc.stdin.end();
      });
    } catch {
      /* gh CLI not installed — OK */
    }

    cachedRepos = await fetchRepos();
    await checkGpgUploadStatus();
    _webviewProvider?.refresh();
    syncAllSettings();
  } catch {
    /* User cancelled or auth failed */
  }
}

async function checkGpgUploadStatus() {
  cachedGpgNeedsUpload = false;
  cachedGpgUploadFailed = false;
  if (!cachedUser) return;
  try {
    const keyId = (
      await execAsync("git", ["config", "--global", "user.signingkey"])
    ).trim();
    if (!keyId) return;
    const list = await runGh(["gpg-key", "list"]);
    // gh gpg-key list output contains key IDs — check if our key is already there
    if (!list.toLowerCase().includes(keyId.toLowerCase().slice(-16))) {
      cachedGpgNeedsUpload = true;
    }
  } catch {
    // gh not available, no signingkey, or scope missing — skip silently
  }
}

async function uploadGpgKeyNow() {
  try {
    const gpgCommand = await resolveGpgCommand();
    if (!gpgCommand) return;
    const keyId = (
      await execAsync("git", ["config", "--global", "user.signingkey"])
    ).trim();
    if (!keyId) return;
    const uploaded = await uploadGpgKeyToGitHub(keyId, gpgCommand);
    if (uploaded) {
      cachedGpgNeedsUpload = false;
      cachedGpgUploadFailed = false;
      _webviewProvider?.refresh();
      vscode.window.showInformationMessage(
        "GPG key uploaded — future commits will show as Verified.",
      );
    } else {
      cachedGpgUploadFailed = true;
      _webviewProvider?.refresh();
    }
  } catch {
    cachedGpgUploadFailed = true;
    _webviewProvider?.refresh();
  }
}

async function logoutGitHub() {
  if (!cachedUser) return;
  const action = await vscode.window.showWarningMessage(
    `Sign out of GitHub (${cachedUser})?`,
    "Sign out",
    "Cancel",
  );
  if (action !== "Sign out") return;

  try {
    await runGh(["auth", "logout", "--hostname", "github.com", "--yes"]);
  } catch {
    try {
      await new Promise((resolve) => {
        const proc = execFile(
          "gh",
          ["auth", "logout", "--hostname", "github.com"],
          { timeout: 5000 },
          () => resolve(),
        );
        proc.stdin?.write("Y\n");
        proc.stdin?.end();
      });
    } catch {
      /* ignore */
    }
  }

  cachedUser = "";
  cachedRepos = [];
  _webviewProvider?.refresh();
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
  const mode = getMode();
  const whitelist = getWhitelist();

  const globalFile = globalSettingsPath();
  const globalExists = fs.existsSync(globalFile);
  const globalData = globalExists ? readJsonFile(globalFile) : null;

  const lines = [
    "Community Cache Status",
    "",
    `GitHub user: ${cachedUser || "(not signed in)"}`,
    `Mode: ${mode}`,
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

// ---------------------------------------------------------------------------
// GPG key provisioning for Verified Commits
// ---------------------------------------------------------------------------

function execAsync(cmd, args, opts = {}) {
  return new Promise((resolve, reject) => {
    execFile(cmd, args, { timeout: 30000, ...opts }, (err, stdout, stderr) => {
      if (err) reject(new Error(stderr || err.message));
      else resolve(stdout);
    });
  });
}

function execAsyncStdin(cmd, args, input) {
  return new Promise((resolve, reject) => {
    const proc = execFile(
      cmd,
      args,
      { timeout: 60000 },
      (err, stdout, stderr) => {
        if (err) reject(new Error(stderr || err.message));
        else resolve(stdout);
      },
    );
    proc.stdin.write(input);
    proc.stdin.end();
  });
}

const GPG_CANDIDATES = [
  "gpg",
  "gpg2",
  "/opt/homebrew/bin/gpg",
  "/opt/homebrew/bin/gpg2",
  "/usr/local/bin/gpg",
  "/usr/local/bin/gpg2",
  "/usr/local/MacGPG2/bin/gpg",
  "/usr/local/MacGPG2/bin/gpg2",
];

const BREW_CANDIDATES = [
  "brew",
  "/opt/homebrew/bin/brew",
  "/usr/local/bin/brew",
];

let cachedGpgCommand;
let cachedBrewCommand;

async function resolveGpgCommand() {
  if (cachedGpgCommand) return cachedGpgCommand;

  for (const candidate of GPG_CANDIDATES) {
    if (candidate.includes(path.sep) && !fs.existsSync(candidate)) {
      continue;
    }

    try {
      await execAsync(candidate, ["--version"]);
      cachedGpgCommand = candidate;
      return candidate;
    } catch {
      /* try next candidate */
    }
  }

  return "";
}

async function resolveBrewCommand() {
  if (cachedBrewCommand) return cachedBrewCommand;

  for (const candidate of BREW_CANDIDATES) {
    if (candidate.includes(path.sep) && !fs.existsSync(candidate)) {
      continue;
    }

    try {
      await execAsync(candidate, ["--version"]);
      cachedBrewCommand = candidate;
      return candidate;
    } catch {
      /* try next candidate */
    }
  }

  return "";
}

async function installGpgWithBrew() {
  if (process.platform !== "darwin") return false;

  const brewCommand = await resolveBrewCommand();
  if (!brewCommand) {
    vscode.window.showErrorMessage(
      "Verified Commits requires GPG, and Homebrew is not installed. Install Homebrew first, then try again.",
    );
    return false;
  }

  const choice = await vscode.window.showWarningMessage(
    "Verified Commits requires GPG. Install GnuPG with Homebrew now?",
    { modal: true },
    "Install",
    "Cancel",
  );
  if (choice !== "Install") return false;

  const installed = await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Installing GnuPG with Homebrew…",
      cancellable: false,
    },
    async () => {
      try {
        await execAsync(brewCommand, ["install", "gnupg"], {
          timeout: 10 * 60 * 1000,
          maxBuffer: 10 * 1024 * 1024,
        });
        return true;
      } catch (err) {
        vscode.window.showErrorMessage(
          `Failed to install GnuPG with Homebrew: ${err.message}`,
        );
        return false;
      }
    },
  );

  if (!installed) return false;

  cachedGpgCommand = undefined;
  const gpgCommand = await resolveGpgCommand();
  if (!gpgCommand) {
    vscode.window.showErrorMessage(
      "GnuPG installed, but VS Code still could not find 'gpg'. Reload the window and try again.",
    );
    return false;
  }

  vscode.window.showInformationMessage(
    "GnuPG installed. Continuing Verified Commits setup.",
  );
  return true;
}

async function ensureGpgAvailable() {
  let gpgCommand = await resolveGpgCommand();
  if (gpgCommand) return gpgCommand;

  if (process.platform === "darwin") {
    const installed = await installGpgWithBrew();
    if (!installed) return "";
    gpgCommand = await resolveGpgCommand();
    if (gpgCommand) return gpgCommand;
  }

  vscode.window.showErrorMessage(
    "Verified Commits requires GPG, but no gpg executable was found.",
  );
  return "";
}

async function ensureGpgKey() {
  const gpgCommand = await ensureGpgAvailable();
  if (!gpgCommand) {
    return false;
  }

  // 1. Check if a signing key is already configured
  try {
    const existing = (
      await execAsync("git", ["config", "--global", "user.signingkey"])
    ).trim();
    if (existing) return true; // already set up
  } catch {
    /* not configured yet */
  }

  // 2. Check for existing secret keys we can reuse
  let email = "";
  try {
    email = (
      await execAsync("git", ["config", "--global", "user.email"])
    ).trim();
  } catch {
    /* no email configured */
  }

  if (email) {
    try {
      const keys = await execAsync(gpgCommand, [
        "--list-secret-keys",
        "--keyid-format",
        "long",
        email,
      ]);
      const match = keys.match(/sec\s+\w+\/([A-F0-9]+)/i);
      if (match) {
        await execAsync("git", [
          "config",
          "--global",
          "user.signingkey",
          match[1],
        ]);
        vscode.window.showInformationMessage(
          `Using existing GPG key ${match[1].slice(-8)} for signing.`,
        );
        await uploadGpgKeyToGitHub(match[1], gpgCommand);
        return true;
      }
    } catch {
      /* no matching key */
    }
  }

  // 3. Need git user.name and user.email to generate a key
  let name = "";
  try {
    name = (await execAsync("git", ["config", "--global", "user.name"])).trim();
  } catch {
    /* no name configured */
  }

  if (!name || !email) {
    vscode.window.showErrorMessage(
      "Set git user.name and user.email before enabling Verified Commits.\n" +
        "Run: git config --global user.name 'Your Name' && git config --global user.email 'you@example.com'",
    );
    return false;
  }

  // 4. Generate a new GPG key
  const progress = await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Generating GPG key…",
    },
    async () => {
      try {
        const batch = [
          "Key-Type: RSA",
          "Key-Length: 4096",
          `Name-Real: ${name}`,
          `Name-Email: ${email}`,
          "Expire-Date: 0",
          "%no-protection",
          "%commit",
          "",
        ].join("\n");
        await execAsyncStdin(gpgCommand, ["--batch", "--gen-key"], batch);
        return true;
      } catch (err) {
        vscode.window.showErrorMessage(
          `Failed to generate GPG key: ${err.message}`,
        );
        return false;
      }
    },
  );

  if (!progress) return false;

  // 5. Read back the new key ID
  let keyId = "";
  try {
    const keys = await execAsync(gpgCommand, [
      "--list-secret-keys",
      "--keyid-format",
      "long",
      email,
    ]);
    const match = keys.match(/sec\s+\w+\/([A-F0-9]+)/i);
    if (match) keyId = match[1];
  } catch {
    /* failed to read key */
  }

  if (!keyId) {
    vscode.window.showErrorMessage(
      "GPG key was generated but could not read the key ID.",
    );
    return false;
  }

  // 6. Configure git to use this key
  await execAsync("git", ["config", "--global", "user.signingkey", keyId]);

  // 7. Upload to GitHub
  const uploaded = await uploadGpgKeyToGitHub(keyId, gpgCommand);
  const suffix = uploaded
    ? " and uploaded to GitHub ✅"
    : " (upload to GitHub manually for the Verified badge)";
  vscode.window.showInformationMessage(
    `GPG key ${keyId.slice(-8)} generated${suffix}`,
  );

  return true;
}

async function uploadGpgKeyToGitHub(keyId, gpgCommand) {
  if (!cachedUser) return false;
  try {
    const pubKey = await execAsync(gpgCommand, ["--armor", "--export", keyId]);
    if (!pubKey.trim()) return false;
    await runGh([
      "api",
      "/user/gpg_keys",
      "-f",
      `armored_public_key=${pubKey.trim()}`,
    ]);
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Activity tracking — tool invocations visible in the webview panel
// ---------------------------------------------------------------------------

function beginToolCall(tool, label, args) {
  if (!_sessionStartedAt) {
    _sessionStartedAt = Date.now();
  }
  const id = `tc-${++_activitySeq}`;
  activeToolCalls.set(id, {
    id,
    tool,
    label,
    startedAt: Date.now(),
    args: args || {},
  });
  _webviewProvider?.pushUpdate({
    type: "activityUpdate",
    items: getActivityItems(),
  });
  return id;
}

function endToolCall(id) {
  activeToolCalls.delete(id);
  _webviewProvider?.pushUpdate({
    type: "activityUpdate",
    items: getActivityItems(),
  });
}

function getActivityItems() {
  const now = Date.now();
  const items = [];
  // Active tool calls from MCP IPC
  for (const c of activeToolCalls.values()) {
    items.push({
      id: c.id,
      type: "tool",
      label: c.label,
      elapsed: Math.floor((now - c.startedAt) / 1000),
      startedAt: c.startedAt,
      args: JSON.stringify(c.args, null, 2),
    });
  }
  // Collect all sessions, sort by most recent activity, take top 3
  const allSessions = [];
  for (const [sessionId, sess] of _chatSessions) {
    const recency = sess.active
      ? sess.startedAt
      : sess.completedAt || sess.startedAt;
    allSessions.push({ sessionId, recency, ...sess });
  }
  allSessions.sort((a, b) => b.recency - a.recency);
  const top3 = allSessions.slice(0, 3);
  for (const sess of top3) {
    if (sess.active) {
      items.push({
        id: `chat-${sess.sessionId}`,
        type: "session-active",
        label: sess.title,
        elapsed: Math.floor((now - (sess.activeAt || sess.startedAt)) / 1000),
        startedAt: sess.activeAt || sess.startedAt,
        preview: sess.preview || "Working\u2026",
        sessionId: sess.sessionId,
      });
    } else {
      items.push({
        id: `chat-${sess.sessionId}`,
        type: "session-done",
        label: sess.title,
        preview: sess.preview || "",
        sessionId: sess.sessionId,
      });
    }
  }
  return items;
}

function _formatDuration(ms) {
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return rem > 0 ? `${m} min ${rem}s` : `${m} min`;
}

function _formatAgo(ms) {
  const s = Math.round(ms / 1000);
  if (s < 60) return "just now";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.floor(m / 60);
  return `${h} hr ago`;
}

function _activityCountLabel(items) {
  const active = items.filter(
    (i) => i.type === "session-active" || i.type === "tool",
  );
  if (items.length === 0) return "idle";
  if (active.length === 0) return `${items.length} recent`;
  return `${active.length} running`;
}

function _renderActivityItem(item, esc) {
  if (item.type === "session-active") {
    return `
      <div class="activity-item activity-item--session" data-sessionid="${item.sessionId}">
        <div class="activity-row">
          <span class="activity-spinner"></span>
          <span class="activity-title">${esc(item.label)}</span>
          <span class="activity-elapsed" data-started="${item.startedAt}">${item.elapsed}s</span>
        </div>
        ${item.preview ? `<div class="activity-sub">${esc(item.preview)}</div>` : ""}
      </div>`;
  }
  if (item.type === "session-done") {
    return `
      <div class="activity-item activity-item--done" data-sessionid="${item.sessionId}">
        <div class="activity-row">
          <span class="activity-dot-done"></span>
          <span class="activity-title">${esc(item.label)}</span>
          <span class="activity-meta">completed</span>
        </div>
        ${item.preview ? `<div class="activity-sub">${esc(item.preview)}</div>` : ""}
      </div>`;
  }
  // tool call
  return `
    <details class="activity-item">
      <summary class="activity-row">
        <span class="activity-pulse"></span>
        <span class="activity-title">${esc(item.label)}</span>
        <span class="activity-elapsed" data-started="${item.startedAt}">${item.elapsed}s</span>
        <svg class="activity-chevron" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 0 1 1.06 0l3.25 3.25a.75.75 0 0 1 0 1.06l-3.25 3.25a.75.75 0 0 1-1.06-1.06L8.94 8 6.22 5.28a.75.75 0 0 1 0-1.06z"/></svg>
      </summary>
      <div class="activity-detail"><pre>${esc(item.args)}</pre></div>
    </details>`;
}

// ---------------------------------------------------------------------------
// Chat session watcher — reads Copilot's own JSONL files to track activity
// Completion is detected by parsing JSONL content: a session is active when
// the last request index N has no modelState record with value===1 (complete).
// This correctly handles the "thinking" phase where the file is static but
// the LLM is running — mtime/size heuristics fail here.
// ---------------------------------------------------------------------------

function _chatSessionsDir(ctx) {
  // storageUri points to workspaceStorage/<hash>/<extId>/ — the directory may not
  // exist (lazy-created), but its PARENT is the workspace hash dir, and chatSessions
  // is always a sibling there. Check the chatSessions path directly, not storageUri.
  if (ctx?.storageUri?.fsPath) {
    const candidate = path.join(
      path.dirname(ctx.storageUri.fsPath),
      "chatSessions",
    );
    if (fs.existsSync(candidate)) return candidate;
  }
  // Fallback: scan workspaceStorage for the workspace matching the open folder
  const wsStorage = path.join(
    os.homedir(),
    "Library",
    "Application Support",
    "Code",
    "User",
    "workspaceStorage",
  );
  if (!fs.existsSync(wsStorage)) return null;
  // Try to match workspace.json to the currently open folder
  const openFolder = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
  if (openFolder) {
    try {
      for (const d of fs.readdirSync(wsStorage)) {
        const wsjson = path.join(wsStorage, d, "workspace.json");
        const csDir = path.join(wsStorage, d, "chatSessions");
        try {
          const raw = fs.readFileSync(wsjson, "utf8");
          const data = JSON.parse(raw);
          const folder =
            data?.folder ||
            (Array.isArray(data?.folders) && data.folders[0]?.path) ||
            "";
          // folder is a URI like file:///Users/... — compare
          const folderPath = folder.startsWith("file://")
            ? decodeURIComponent(folder.replace(/^file:\/\//, ""))
            : folder;
          if (folderPath === openFolder && fs.existsSync(csDir)) return csDir;
        } catch {}
      }
    } catch {}
  }
  // Last resort: most-recently-modified chatSessions dir
  try {
    const dirs = fs
      .readdirSync(wsStorage)
      .map((d) => path.join(wsStorage, d, "chatSessions"))
      .filter((d) => {
        try {
          return fs.statSync(d).isDirectory();
        } catch {
          return false;
        }
      })
      .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
    if (dirs.length) return dirs[0];
  } catch {}
  return null;
}

function startChatSessionWatcher(ctx) {
  _chatSessionWatcher?.close();
  _chatSessionWatcher = null;
  if (_chatSessionPoller) {
    clearInterval(_chatSessionPoller);
    _chatSessionPoller = null;
  }

  const chatSessionsDir = _chatSessionsDir(ctx);
  if (!chatSessionsDir) return;

  // Scan .jsonl session files and process likely-active candidates.
  // Used as fallback when fs.watch gives null filename (macOS) and for polling.
  let _lastScanMs = 0;
  let _didBootstrapScan = false;
  const _scanRecentFiles = () => {
    const now = Date.now();
    if (now - _lastScanMs < 800) return; // debounce directory scans only
    _lastScanMs = now;
    try {
      const files = fs
        .readdirSync(chatSessionsDir)
        .filter((f) => f.endsWith(".jsonl"))
        .map((f) => {
          const fp = path.join(chatSessionsDir, f);
          let mtimeMs = 0;
          try {
            mtimeMs = fs.statSync(fp).mtimeMs;
          } catch {}
          return { f, fp, sid: f.slice(0, -6), mtimeMs };
        })
        .sort((a, b) => b.mtimeMs - a.mtimeMs);

      // Re-check known active sessions so they can transition to done.
      const candidate = new Map();
      for (const [sid, sess] of _chatSessions) {
        if (sess?.active && sess.filePath) {
          candidate.set(sid, { sid, fp: sess.filePath });
        }
      }

      // Only consider files modified in the last 5 minutes as potentially active.
      // Older files are definitely completed — no model thinks for 5+ minutes
      // without writing anything to the JSONL.
      const recentFiles = files.filter((f) => now - f.mtimeMs < 300000);
      for (const file of recentFiles) {
        candidate.set(file.sid, { sid: file.sid, fp: file.fp });
      }

      for (const c of candidate.values()) {
        _onChatSessionWrite(c.sid, c.fp);
      }
      _didBootstrapScan = true;
    } catch {}
    // Always push update (keeps elapsed time fresh)
    _pushActivityUpdate();
  };

  // Seed activity state immediately
  _scanRecentFiles();

  _chatSessionWatcher = fs.watch(
    chatSessionsDir,
    { persistent: false },
    (_evt, filename) => {
      if (!filename) {
        // macOS: filename is null — scan for recent changes
        _scanRecentFiles();
        return;
      }
      if (!filename.endsWith(".jsonl")) return;
      // Process this specific file IMMEDIATELY — no debounce
      const sessionId = filename.slice(0, -6);
      _onChatSessionWrite(sessionId, path.join(chatSessionsDir, filename));
      _pushActivityUpdate();
    },
  );

  // Poll every 2s — keeps elapsed time fresh and catches events fs.watch misses
  _chatSessionPoller = setInterval(_scanRecentFiles, 2000);
}

function _pushActivityUpdate() {
  _webviewProvider?.pushUpdate({
    type: "activityUpdate",
    items: getActivityItems(),
  });
}

// ---------------------------------------------------------------------------
// Activity-Driven Worktree Focus
// ---------------------------------------------------------------------------
// When a chat session's JSONL file grows (new message, agent output), the
// fs.watch-based _chatSessionWatcher fires _onChatSessionWrite.  If the session
// has a bound worktree, we focus the Explorer on it.  If a different session
// without a binding receives activity while we have a focused worktree, we
// unfocus back to the main workspace.  This is fully event-driven — no polling.

function _maybeUpdateWorktreeFocus(sessionId, fileSizeGrew) {
  if (!fileSizeGrew) return;
  if (_worktreeBindings.size === 0 && !_activeWorktreeFolder) return;

  const binding = _worktreeBindings.get(sessionId);
  if (binding && fs.existsSync(binding.worktreePath)) {
    if (_focusedSessionId !== sessionId) {
      getDiagnosticsOutputChannel().appendLine(
        `[worktree] JSONL activity → focus ${sessionId} worktree=${binding.worktreePath}`,
      );
      _focusedSessionId = sessionId;
      _focusWorktreeFolder(binding.worktreePath);
    }
  } else if (_activeWorktreeFolder && _focusedSessionId) {
    // Activity in a session without a worktree binding — return to main workspace.
    getDiagnosticsOutputChannel().appendLine(
      `[worktree] JSONL activity in unbound session ${sessionId} → unfocus`,
    );
    _focusedSessionId = null;
    _unfocusWorktreeFolder();
  }
}

function _chatSessionReadTail(filePath, bytes) {
  // Read last N bytes from the JSONL file
  const readLen = bytes || 65536; // 64 KB default — large sessions need more tail
  try {
    const fd = fs.openSync(filePath, "r");
    try {
      const { size } = fs.fstatSync(fd);
      const actual = Math.min(readLen, size);
      const buf = Buffer.alloc(actual);
      fs.readSync(fd, buf, 0, actual, size - actual);
      return { tail: buf.toString("utf8"), size };
    } finally {
      fs.closeSync(fd);
    }
  } catch {
    return { tail: "", size: 0 };
  }
}

function _chatSessionReadTitle(filePath, existing) {
  if (existing && existing !== "Copilot Chat") return existing;
  try {
    const fd = fs.openSync(filePath, "r");
    try {
      const stat = fs.fstatSync(fd);
      const fileSize = stat.size;
      let customTitle = null;
      let firstPrompt = null;

      // Helper: scan JSONL lines in a buffer for title data
      const scanLines = (buf, len) => {
        for (const line of buf.slice(0, len).toString("utf8").split("\n")) {
          try {
            const rec = JSON.parse(line);
            if (
              rec.kind === 1 &&
              rec.k?.[0] === "customTitle" &&
              typeof rec.v === "string"
            ) {
              customTitle = rec.v;
            }
            if (
              !firstPrompt &&
              rec.kind === 2 &&
              rec.k?.[0] === "requests" &&
              rec.k.length === 1 &&
              Array.isArray(rec.v)
            ) {
              for (const req of rec.v) {
                const msg =
                  req?.message?.text ||
                  req?.message ||
                  req?.text ||
                  req?.prompt;
                if (typeof msg === "string" && msg.trim()) {
                  firstPrompt = msg.trim().slice(0, 80);
                  break;
                }
              }
            }
          } catch {}
        }
      };

      // Pass 1: read first 8KB — covers small sessions where kind=0 snapshot fits
      const headBuf = Buffer.alloc(8192);
      const headN = fs.readSync(fd, headBuf, 0, 8192, 0);
      scanLines(headBuf, headN);
      if (customTitle) return customTitle;

      // Pass 2: if first line is huge (snapshot > 8KB), find first newline and read lines 2-10
      if (headN >= 8192 && !headBuf.slice(0, headN).includes(0x0a)) {
        // Scan forward in 64KB chunks to find the first newline
        const chunkSize = 65536;
        const scanBuf = Buffer.alloc(chunkSize);
        let offset = 8192;
        let nlOffset = -1;
        while (offset < fileSize && offset < 100 * 1024 * 1024) {
          const toRead = Math.min(chunkSize, fileSize - offset);
          const got = fs.readSync(fd, scanBuf, 0, toRead, offset);
          if (got === 0) break;
          const idx = scanBuf.indexOf(0x0a, 0);
          if (idx !== -1 && idx < got) {
            nlOffset = offset + idx;
            break;
          }
          offset += got;
        }
        if (nlOffset !== -1 && nlOffset + 1 < fileSize) {
          // Read 16KB after the first newline (lines 2-N)
          const afterBuf = Buffer.alloc(16384);
          const afterN = fs.readSync(fd, afterBuf, 0, 16384, nlOffset + 1);
          scanLines(afterBuf, afterN);
          if (customTitle) return customTitle;
        }
      }

      // Pass 3: read tail 32KB — catches customTitle written later in session
      if (!customTitle && fileSize > 8192) {
        const tailSize = Math.min(32768, fileSize);
        const tailBuf = Buffer.alloc(tailSize);
        const tailN = fs.readSync(
          fd,
          tailBuf,
          0,
          tailSize,
          fileSize - tailSize,
        );
        scanLines(tailBuf, tailN);
      }

      return customTitle || firstPrompt || "Copilot Chat";
    } finally {
      fs.closeSync(fd);
    }
  } catch {}
  return "Copilot Chat";
}

/** Extract a preview description from the tail of the JSONL file. */
function _chatSessionExtractPreview(tail) {
  const lines = tail.split("\n");
  let lastToolCall = null;
  let lastProgress = null;
  // Walk backwards through parsed lines for the most recent tool invocation or progress
  for (let i = lines.length - 1; i >= 0; i--) {
    try {
      const rec = JSON.parse(lines[i]);
      // Tool call invocation messages appear as response part mutations
      if (rec.kind === 2 || rec.kind === 1) {
        const val = rec.v;
        // Check if this is a response part with tool info
        if (val && typeof val === "object") {
          // invocationMessage from tool calls (e.g. "Reading file.ts")
          if (!lastToolCall && typeof val.invocationMessage === "string") {
            lastToolCall = val.invocationMessage;
          }
          // Progress messages
          if (
            !lastProgress &&
            typeof val.content === "string" &&
            val.kind === "progressMessage"
          ) {
            lastProgress = val.content;
          }
          // Array of response parts
          if (Array.isArray(val)) {
            for (let j = val.length - 1; j >= 0; j--) {
              const part = val[j];
              if (
                !lastToolCall &&
                typeof part?.invocationMessage === "string"
              ) {
                lastToolCall = part.invocationMessage;
              }
              if (
                !lastProgress &&
                typeof part?.content === "string" &&
                part?.kind === "progressMessage"
              ) {
                lastProgress = part.content;
              }
            }
          }
        }
      }
      if (lastToolCall) break; // found what we need
    } catch {}
  }
  return lastToolCall || lastProgress || null;
}

/**
 * Parse the JSONL tail to determine if the last request is still in progress.
 * A session is active when the last request index N has no modelState record
 * with value===1 (completed). This works correctly during the LLM "thinking"
 * phase when the file is static (mtime does not change).
 */
function _chatSessionParseState(tail) {
  const lines = tail.split("\n");
  let lastRequestIdx = -1;
  const doneRequests = new Set(); // requests with a terminal modelState

  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const rec = JSON.parse(line);
      const k = rec.k;
      if (!Array.isArray(k)) continue;

      // Track the highest request index seen from any per-request record
      if (k[0] === "requests" && typeof k[1] === "number") {
        if (k[1] > lastRequestIdx) lastRequestIdx = k[1];
      }
      // kind=2 splice of the TOP-LEVEL requests array tells us a new request was added.
      if (
        rec.kind === 2 &&
        k.length === 1 &&
        k[0] === "requests" &&
        Array.isArray(rec.v)
      ) {
        const spliceEnd = (rec.offset || 0) + rec.v.length - 1;
        if (spliceEnd > lastRequestIdx) lastRequestIdx = spliceEnd;
      }

      // modelState values: 0=Failed, 1=Completed, 2=InProgress, 3=NeedsInput
      // Only value 2 means truly active. Everything else is a terminal state.
      if (
        k[0] === "requests" &&
        typeof k[1] === "number" &&
        k[2] === "modelState" &&
        typeof rec.v?.value === "number"
      ) {
        if (rec.v.value !== 2) {
          doneRequests.add(k[1]);
        }
      }
    } catch {}
  }

  if (lastRequestIdx < 0) return { active: false, lastRequestIdx: -1 };
  return {
    active: !doneRequests.has(lastRequestIdx),
    lastRequestIdx,
  };
}

function _chatSessionReadCreationDate(filePath) {
  // Extract creationDate from the kind=0 snapshot (first line) or early mutation.
  // The snapshot can be huge (multi-MB), so scan bytewise for the key.
  try {
    const fd = fs.openSync(filePath, "r");
    try {
      // Read first 4KB — creationDate is near the start of the snapshot JSON
      const buf = Buffer.alloc(4096);
      const n = fs.readSync(fd, buf, 0, 4096, 0);
      const str = buf.slice(0, n).toString("utf8");
      const m = str.match(/"creationDate"\s*:\s*(\d+)/);
      if (m) return parseInt(m[1], 10);
    } finally {
      fs.closeSync(fd);
    }
  } catch {}
  return null;
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------

function _onChatSessionWrite(sessionId, filePath) {
  const existing = _chatSessions.get(sessionId);
  const now = Date.now();

  // Always read the tail — content-based detection, not mtime
  const { tail, size: fileSize } = _chatSessionReadTail(filePath);
  if (!tail) return;

  // Skip if file hasn't changed since last check
  if (existing && existing.lastSize === fileSize && !existing.active) return;

  // Hard mtime guard: if the file hasn't been modified in 5+ minutes, it cannot
  // be an active session. This catches cases where the 64KB tail doesn't contain
  // the completion record for large/old sessions.
  let fileMtimeMs = 0;
  try {
    fileMtimeMs = fs.statSync(filePath).mtimeMs;
  } catch {}
  const fileStaleMs = Date.now() - fileMtimeMs;
  const forceCompleted = fileStaleMs > 300000; // 5 minutes

  const { active: rawActive, lastRequestIdx } = _chatSessionParseState(tail);
  const isActive = rawActive && !forceCompleted;
  const title = _chatSessionReadTitle(filePath, existing?.title);

  // Activity-driven worktree focus: if new content was written to an active
  // session, switch the Explorer to the corresponding worktree (or unfocus if
  // the session has no binding).  This is event-driven — triggered by fs.watch.
  const fileSizeGrew = fileSize > (existing?.lastSize || 0);
  if (isActive) {
    _maybeUpdateWorktreeFocus(sessionId, fileSizeGrew);
  }

  // Always extract preview from tail so completed sessions retain their last summary
  const newPreview = _chatSessionExtractPreview(tail);
  let preview = newPreview || existing?.preview || null;

  // Determine startedAt: prefer existing, then creationDate from JSONL, then now
  let startedAt = existing?.startedAt;
  if (!startedAt || (existing && !existing.active && isActive)) {
    startedAt = _chatSessionReadCreationDate(filePath) || now;
  }

  if (isActive) {
    // Staleness guard: if the file hasn't changed in 2+ minutes and the session
    // was already known, treat it as done (the model may have disconnected).
    if (existing && existing.lastSize === fileSize && existing.active) {
      const staleMs = now - (existing._lastChangedAt || existing.startedAt);
      if (staleMs > 120000) {
        _chatSessions.set(sessionId, {
          title,
          active: false,
          startedAt,
          completedAt: existing._lastChangedAt || now,
          filePath,
          sessionId,
          lastSize: fileSize,
          preview: preview || existing?.preview || null,
          requestCount: lastRequestIdx + 1,
          _lastChangedAt: existing._lastChangedAt || now,
        });
        return;
      }
    }
    _chatSessions.set(sessionId, {
      title,
      active: true,
      startedAt,
      completedAt: null,
      filePath,
      sessionId,
      lastSize: fileSize,
      preview: preview || "Working…",
      requestCount: lastRequestIdx + 1,
      _lastChangedAt:
        existing?.lastSize !== fileSize ? now : existing?._lastChangedAt || now,
    });
    // If this session has a bound worktree, ensure the workspace folder is present
    const binding = _worktreeBindings.get(sessionId);
    if (binding && fs.existsSync(binding.worktreePath)) {
      ensureWorktreeFolder(binding.worktreePath);
    }
    // Temporal correlation: if a chat editor tab is active right now,
    // it must be the tab for this session — record the mapping.
    try {
      const activeTab = vscode.window.tabGroups?.activeTabGroup?.activeTab;
      if (
        activeTab?.input?.viewType === "workbench.editor.chatSession" &&
        activeTab.input.uri
      ) {
        _chatTabToSession.set(activeTab.input.uri.toString(), sessionId);
      }
    } catch {}
  } else {
    const completedAt = existing?.active ? now : existing?.completedAt || now;
    _chatSessions.set(sessionId, {
      title,
      active: false,
      startedAt,
      activeAt: null,
      completedAt,
      filePath,
      sessionId,
      lastSize: fileSize,
      preview: preview || existing?.preview || null,
      requestCount: lastRequestIdx + 1,
      _lastChangedAt: now,
    });
    // When a session completes, do nothing — workspace stays on current branch.
    // Only other chats becoming active or user clicking back will change the view.
  }
}

// ---------------------------------------------------------------------------
// Models — enumerate available Copilot language models
// ---------------------------------------------------------------------------

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
    // deduplicate by id
    const seen = new Set();
    cachedModels = cachedModels.filter((m) => {
      if (seen.has(m.id)) return false;
      seen.add(m.id);
      return true;
    });
  } catch {
    cachedModels = [];
  }
  _webviewProvider?.refresh();
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
  // Fallback: open quick-open with a model-related search
  await vscode.commands.executeCommand(
    "workbench.action.quickOpen",
    ">chat model",
  );
}

// ---------------------------------------------------------------------------
// Ollama detection
// ---------------------------------------------------------------------------

const OLLAMA_BASE = "http://127.0.0.1:11434";

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

// ---------------------------------------------------------------------------
// API key helpers (stored in VS Code SecretStorage — never on disk)
// ---------------------------------------------------------------------------

const API_KEY_ANTHROPIC = "gsh.apiKey.anthropic";
const API_KEY_OPENAI = "gsh.apiKey.openai";

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
  } catch {
    /* ignore */
  }
}

async function getProviderStatus() {
  const [anthropicKey, openaiKey] = await Promise.all([
    getApiKey(API_KEY_ANTHROPIC),
    getApiKey(API_KEY_OPENAI),
  ]);
  return {
    anthropicKey: anthropicKey ? "set" : "",
    openaiKey: openaiKey ? "set" : "",
    ollamaRunning: cachedOllamaRunning,
    ollamaModels: cachedOllamaModels,
  };
}

// ---------------------------------------------------------------------------
// Local agents — scan and launch .github/agents/*.agent.md files
// ---------------------------------------------------------------------------

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

function scanLocalAgents() {
  const agents = [];
  const folders = vscode.workspace.workspaceFolders || [];
  for (const folder of folders) {
    const agentsDir = path.join(folder.uri.fsPath, ".github", "agents");
    if (!fs.existsSync(agentsDir)) continue;
    let files;
    try {
      files = fs.readdirSync(agentsDir).filter((f) => f.endsWith(".agent.md"));
    } catch {
      continue;
    }
    for (const file of files) {
      try {
        const content = fs.readFileSync(path.join(agentsDir, file), "utf8");
        const agent = parseAgentFrontmatter(content, file);
        if (agent) agents.push(agent);
      } catch {
        /* skip */
      }
    }
  }
  return agents.sort((a, b) => a.name.localeCompare(b.name));
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
      await vscode.commands.executeCommand(cmd, { query: `@${agentName} ` });
      return;
    }
  } catch {
    /* fall through */
  }
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
      // Pass the query without isPartialQuery so VS Code submits it immediately
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
  } catch {
    /* fall through */
  }
  await vscode.commands.executeCommand("workbench.action.quickOpen", qa.query);
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
  } catch {
    /* fall through */
  }
  // Last resort: copy to clipboard and notify
  await vscode.env.clipboard.writeText(qa.query);
  vscode.window.showInformationMessage(
    `Copied "${qa.query}" to clipboard — paste it into a new chat.`,
  );
}

// ---------------------------------------------------------------------------
// Checkpoint settings → git config sync
// ---------------------------------------------------------------------------

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
        execFile("git", ["config", gitKey, String(value)], { cwd }, (err) => {
          if (err) {
            // Not a git repo or git not available — ignore silently
          }
        });
      }
    }
  }
}

function activate(context) {
  _context = context;

  // Restore persisted Ollama pinned models
  const savedPinned = context.globalState.get("gsh.ollama.pinned", []);
  _ollamaPinned = new Set(Array.isArray(savedPinned) ? savedPinned : []);

  importFromJson();
  migrateLegacyMcpRegistrations();
  registerMcpServerProvider(context);
  registerCustomizationInspectorTool(context);

  // Git Helpers webview (MCP Tools + Community Cache)
  const CommunityCacheViewProvider = createWebviewProviderClass({
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
    _renderActivityItem,
    _activityCountLabel,
    API_KEY_ANTHROPIC,
    API_KEY_OPENAI,
    TOOL_GROUPS,
    MODES,
    QUICK_ACTIONS,
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
    _ollamaPinned,
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
      await checkGpgUploadStatus();
      _webviewProvider.refresh();
    }
  });

  // Detect Ollama on startup
  detectOllama();

  // Load available Copilot models on startup and whenever the model list changes
  refreshModels();
  if (vscode.lm?.onDidChangeChatModels) {
    context.subscriptions.push(
      vscode.lm.onDidChangeChatModels(() => refreshModels()),
    );
  }

  // Start strict lint IPC server so the gsh MCP tool can query VS Code diagnostics
  startStrictLintIpcServer();
  // Start activity IPC server so the gsh MCP server can report active tool calls
  startActivityIpcServer();
  // Load persisted worktree↔session bindings and reconcile workspace folders
  loadWorktreeBindings();
  reconcileWorktreeBindings();
  // Track chat editor tabs — switch explorer focus to the active worktree
  context.subscriptions.push(
    vscode.window.tabGroups.onDidChangeTabs(() => _onActiveTabChanged()),
    vscode.window.tabGroups.onDidChangeTabGroups(() => _onActiveTabChanged()),
  );
  // Watch Copilot Chat's JSONL session files for live activity.
  // The end-of-response marker is pendingRequests:null written to the JSONL.
  startChatSessionWatcher(context);
  context.subscriptions.push({
    dispose: () => {
      _chatSessionWatcher?.close();
      _chatSessionWatcher = null;
      if (_chatSessionPoller) {
        clearInterval(_chatSessionPoller);
        _chatSessionPoller = null;
      }
    },
  });

  // Write default tools config if none exists
  if (!fs.existsSync(MCP_TOOLS_CONFIG_PATH)) {
    writeToolsConfig({ disabledTools: [] });
  }

  context.subscriptions.push(
    vscode.commands.registerCommand(
      "gitShellHelpers.showCommunityStatus",
      showCommunityStatus,
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.inspectCopilotCustomizationWarnings",
      async (filePath) => {
        const result = await inspectCopilotCustomizationWarnings({
          filePath,
          notify: true,
          revealOutput: true,
        });
        return formatCustomizationInspectionReport(result);
      },
    ),
    vscode.commands.registerCommand("gitShellHelpers.loginGitHub", loginGitHub),
    vscode.commands.registerCommand(
      "gitShellHelpers.logoutGitHub",
      logoutGitHub,
    ),
    vscode.commands.registerCommand("gitShellHelpers.selectRepos", selectRepos),
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
      openMcpServerControls,
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.refreshModels",
      async () => {
        await refreshModels();
        vscode.window.showInformationMessage(
          `Git Shell Helpers: ${cachedModels.length} Copilot model(s) found.`,
        );
      },
    ),
    vscode.commands.registerCommand(
      "gitShellHelpers.openModelPicker",
      openModelPicker,
    ),
  );

  // Sync checkpoint settings to git config when changed
  syncCheckpointSettings();
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("gitShellHelpers.checkpoint")) {
        syncCheckpointSettings();
      }
      if (e.affectsConfiguration("gitShellHelpers.customizationInspector")) {
        registerCustomizationInspectorTool(context);
      }
    }),
  );
}

// ---------------------------------------------------------------------------
// Strict Lint IPC server — allows the gsh MCP server to request diagnostics
// from VS Code's live language servers via a Unix socket.
// ---------------------------------------------------------------------------

const STRICT_LINT_SOCKET_PATH = path.join(os.tmpdir(), "gsh-strict-lint.sock");
const STRICT_LINT_IPC_INFO_PATH = path.join(
  os.homedir(),
  ".cache",
  "gsh",
  "strict-lint-ipc.json",
);

function startStrictLintIpcServer() {
  if (_strictLintIpcServer) return;

  try {
    if (fs.existsSync(STRICT_LINT_SOCKET_PATH)) {
      fs.unlinkSync(STRICT_LINT_SOCKET_PATH);
    }
  } catch {
    // ignore
  }

  try {
    fs.mkdirSync(path.dirname(STRICT_LINT_IPC_INFO_PATH), { recursive: true });
    fs.writeFileSync(
      STRICT_LINT_IPC_INFO_PATH,
      JSON.stringify(
        {
          socketPath: STRICT_LINT_SOCKET_PATH,
          updatedAt: new Date().toISOString(),
        },
        null,
        2,
      ),
      "utf8",
    );
  } catch {
    // ignore — non-fatal
  }

  _strictLintIpcServer = net.createServer((socket) => {
    let buffer = "";
    socket.setEncoding("utf8");

    socket.on("data", async (chunk) => {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      for (const line of lines) {
        if (!line.trim()) continue;

        let request;
        try {
          request = JSON.parse(line);
        } catch {
          socket.write(
            JSON.stringify({ ok: false, error: "Invalid JSON" }) + "\n",
          );
          continue;
        }

        try {
          const callId = beginToolCall(
            "strict-lint-mcp",
            `MCP Strict Lint: ${request.arguments?.filePath ? path.basename(request.arguments.filePath) : "workspace"}`,
            request.arguments || {},
          );
          try {
            const result = await runStrictLinting(request.arguments || {});
            socket.write(JSON.stringify({ ok: true, result }) + "\n");
          } finally {
            endToolCall(callId);
          }
        } catch (err) {
          socket.write(
            JSON.stringify({
              ok: false,
              error: err instanceof Error ? err.message : String(err),
            }) + "\n",
          );
        }
      }
    });

    socket.on("error", () => {});
  });

  _strictLintIpcServer.listen(STRICT_LINT_SOCKET_PATH);
  _strictLintIpcServer.on("error", () => {
    _strictLintIpcServer = null;
  });
}

function stopStrictLintIpcServer() {
  if (_strictLintIpcServer) {
    _strictLintIpcServer.close();
    _strictLintIpcServer = null;
  }
  try {
    fs.unlinkSync(STRICT_LINT_SOCKET_PATH);
  } catch {
    // ignore
  }
  try {
    fs.unlinkSync(STRICT_LINT_IPC_INFO_PATH);
  } catch {
    // ignore
  }
}

// ---------------------------------------------------------------------------
// Worktree ↔ Chat Session Binding
// ---------------------------------------------------------------------------

const WORKTREE_BINDINGS_KEY = "worktreeBindings";

function loadWorktreeBindings() {
  try {
    const raw = _context?.globalState?.get(WORKTREE_BINDINGS_KEY);
    if (Array.isArray(raw)) {
      _worktreeBindings = new Map(raw);
    }
  } catch {
    // ignore
  }
}

function saveWorktreeBindings() {
  try {
    _context?.globalState?.update(
      WORKTREE_BINDINGS_KEY,
      Array.from(_worktreeBindings.entries()),
    );
  } catch {
    // ignore
  }
}

function findActiveSessionId() {
  // Return the ID of the currently active chat session (most recently active).
  let bestId = null;
  let bestAt = 0;
  for (const [sid, sess] of _chatSessions) {
    if (sess.active) {
      const at = sess._lastChangedAt || sess.startedAt || 0;
      if (at > bestAt) {
        bestAt = at;
        bestId = sid;
      }
    }
  }
  return bestId;
}

function bindWorktreeToSession(sessionId, binding) {
  _worktreeBindings.set(sessionId, binding);
  saveWorktreeBindings();
  ensureWorktreeFolder(binding.worktreePath);
}

function unbindWorktreeFromSession(worktreePath) {
  let removedSessionId = null;
  for (const [sid, binding] of _worktreeBindings) {
    if (binding.worktreePath === worktreePath) {
      removedSessionId = sid;
      _worktreeBindings.delete(sid);
      break;
    }
  }
  saveWorktreeBindings();
  removeWorktreeFolder(worktreePath);
  return removedSessionId;
}

function ensureWorktreeFolder(worktreePath) {
  if (!fs.existsSync(worktreePath)) return;
  const folders = vscode.workspace.workspaceFolders || [];
  const alreadyPresent = folders.some((f) => f.uri.fsPath === worktreePath);
  if (alreadyPresent) return;

  // Add at the end of the workspace folders list.
  // The name includes the branch for clarity in the explorer sidebar.
  const dirName = path.basename(worktreePath);
  vscode.workspace.updateWorkspaceFolders(folders.length, 0, {
    uri: vscode.Uri.file(worktreePath),
    name: `🌿 ${dirName}`,
  });
}

function removeWorktreeFolder(worktreePath) {
  const folders = vscode.workspace.workspaceFolders || [];
  const idx = folders.findIndex((f) => f.uri.fsPath === worktreePath);
  if (idx === -1) return;
  vscode.workspace.updateWorkspaceFolders(idx, 1);
}

function handleWorktreeIpcMessage(msg) {
  if (msg.type === "worktreeCreated") {
    getDiagnosticsOutputChannel().appendLine(
      `[worktree] IPC received: worktreeCreated branch=${msg.branch} path=${msg.worktreePath}`,
    );
    _bindWorktreeWithRetry(msg, 0);
  } else if (msg.type === "worktreeRemoved") {
    getDiagnosticsOutputChannel().appendLine(
      `[worktree] IPC received: worktreeRemoved path=${msg.worktreePath}`,
    );
    unbindWorktreeFromSession(msg.worktreePath);
  }
}

// The IPC message from the MCP server may arrive before the JSONL watcher
// detects this chat session as active. Retry a few times with back-off.
function _bindWorktreeWithRetry(msg, attempt) {
  const sessionId = findActiveSessionId();
  if (sessionId) {
    getDiagnosticsOutputChannel().appendLine(
      `[worktree] Bound to session ${sessionId} on attempt ${attempt}`,
    );
    const binding = {
      branch: msg.branch,
      worktreePath: msg.worktreePath,
      baseBranch: msg.baseBranch || "",
      baseCommit: msg.baseCommit || "",
      createdAt: Date.now(),
    };
    bindWorktreeToSession(sessionId, binding);

    // Immediately focus the new worktree folder so the user "moves with" the agent.
    // Small delay: updateWorkspaceFolders (inside bindWorktreeToSession) needs a
    // tick to register before revealInExplorer can find the folder.
    _focusedSessionId = sessionId;
    setTimeout(() => _focusWorktreeFolder(msg.worktreePath), 500);
    return;
  }
  // Retry up to 5 times (0.5s, 1s, 1.5s, 2s, 2.5s = ~7.5s total)
  if (attempt < 5) {
    setTimeout(
      () => _bindWorktreeWithRetry(msg, attempt + 1),
      500 * (attempt + 1),
    );
  }
}

// Reconcile workspace folders on activation: add folders for bindings that
// still have valid worktrees, remove stale bindings whose worktrees are gone.
function reconcileWorktreeBindings() {
  let changed = false;
  for (const [sid, binding] of _worktreeBindings) {
    if (!fs.existsSync(binding.worktreePath)) {
      _worktreeBindings.delete(sid);
      changed = true;
    } else {
      ensureWorktreeFolder(binding.worktreePath);
    }
  }
  if (changed) saveWorktreeBindings();
}

// ---------------------------------------------------------------------------
// Tab ↔ Worktree Focus Switching
// When the user switches to a chat editor tab that has a bound worktree,
// reveal that worktree folder in the Explorer. When switching away from
// all chat tabs (or to a chat with no binding), return to the main repo.
// ---------------------------------------------------------------------------

function _focusWorktreeFolder(worktreePath) {
  if (_activeWorktreeFolder === worktreePath) return;
  // Remember where we came from so we can return later.
  if (!_activeWorktreeFolder && !_originalWorkspaceUri) {
    const mainFolder = vscode.workspace.workspaceFolders?.[0];
    if (mainFolder) _originalWorkspaceUri = mainFolder.uri;
  }
  _activeWorktreeFolder = worktreePath;
  // revealInExplorer is more reliable when given a file URI rather than a bare
  // directory root.  Find the first file inside the worktree to use as the
  // reveal target; fall back to the root itself.
  let revealUri = vscode.Uri.file(worktreePath);
  try {
    const entries = fs.readdirSync(worktreePath);
    const file = entries.find((e) => {
      try {
        return fs.statSync(path.join(worktreePath, e)).isFile();
      } catch {
        return false;
      }
    });
    if (file) {
      revealUri = vscode.Uri.file(path.join(worktreePath, file));
    }
  } catch {}
  getDiagnosticsOutputChannel().appendLine(
    `[worktree] revealInExplorer → ${revealUri.fsPath}`,
  );
  vscode.commands.executeCommand("revealInExplorer", revealUri);
}

function _unfocusWorktreeFolder() {
  if (!_activeWorktreeFolder) return;
  _activeWorktreeFolder = null;
  // Return to the folder the user was on before any worktree focus.
  const returnUri =
    _originalWorkspaceUri || vscode.workspace.workspaceFolders?.[0]?.uri;
  _originalWorkspaceUri = null;
  if (returnUri) {
    vscode.commands.executeCommand("revealInExplorer", returnUri);
  }
}

function _onActiveTabChanged() {
  if (_worktreeBindings.size === 0) return;

  let activeTab;
  try {
    activeTab = vscode.window.tabGroups?.activeTabGroup?.activeTab;
  } catch {
    return;
  }

  // Check if the active tab is a chat editor tab
  if (
    activeTab?.input?.viewType === "workbench.editor.chatSession" &&
    activeTab.input.uri
  ) {
    const tabKey = activeTab.input.uri.toString();
    let sessionId = _chatTabToSession.get(tabKey);

    // Fallback: match tab label to session titles that have worktree bindings
    if (!sessionId && activeTab.label) {
      for (const [sid, sess] of _chatSessions) {
        if (sess.title === activeTab.label && _worktreeBindings.has(sid)) {
          sessionId = sid;
          _chatTabToSession.set(tabKey, sid);
          break;
        }
      }
    }

    if (sessionId) {
      const binding = _worktreeBindings.get(sessionId);
      if (binding && fs.existsSync(binding.worktreePath)) {
        _focusedSessionId = sessionId;
        _focusWorktreeFolder(binding.worktreePath);
        return;
      }
    }
  }

  // Non-chat tab or chat without a binding — return to main repo
  if (_activeWorktreeFolder) {
    _focusedSessionId = null;
    _unfocusWorktreeFolder();
  }
}

const ACTIVITY_SOCKET_PATH = path.join(os.tmpdir(), "gsh-activity.sock");
const ACTIVITY_IPC_INFO_PATH = path.join(
  os.homedir(),
  ".cache",
  "gsh",
  "activity-ipc.json",
);

function startActivityIpcServer() {
  if (_activityIpcServer) return;

  try {
    if (fs.existsSync(ACTIVITY_SOCKET_PATH)) {
      fs.unlinkSync(ACTIVITY_SOCKET_PATH);
    }
  } catch {
    // ignore
  }

  try {
    fs.mkdirSync(path.dirname(ACTIVITY_IPC_INFO_PATH), { recursive: true });
    fs.writeFileSync(
      ACTIVITY_IPC_INFO_PATH,
      JSON.stringify(
        {
          socketPath: ACTIVITY_SOCKET_PATH,
          updatedAt: new Date().toISOString(),
        },
        null,
        2,
      ),
      "utf8",
    );
  } catch {
    // ignore — non-fatal
  }

  _activityIpcServer = net.createServer((socket) => {
    let buffer = "";
    socket.setEncoding("utf8");
    socket.on("data", (chunk) => {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";
      for (const line of lines) {
        if (!line.trim()) continue;
        let msg;
        try {
          msg = JSON.parse(line);
        } catch {
          continue;
        }
        if (msg.type === "activityBegin" && msg.id) {
          const internalId = beginToolCall(
            msg.tool || "mcp",
            msg.label || msg.tool || "MCP Tool",
            msg.args || {},
          );
          _externalToInternal.set(msg.id, internalId);
        } else if (msg.type === "activityEnd" && msg.id) {
          const internalId = _externalToInternal.get(msg.id);
          if (internalId) {
            _externalToInternal.delete(msg.id);
            endToolCall(internalId);
          }
        } else if (msg.type === "sessionPulse") {
          if (!_sessionStartedAt) {
            _sessionStartedAt = Date.now();
          }
          _webviewProvider?.pushUpdate({
            type: "activityUpdate",
            items: getActivityItems(),
          });
        } else if (
          msg.type === "worktreeCreated" ||
          msg.type === "worktreeRemoved"
        ) {
          handleWorktreeIpcMessage(msg);
        }
      }
    });
    socket.on("error", () => {});
  });

  _activityIpcServer.listen(ACTIVITY_SOCKET_PATH);
  _activityIpcServer.on("error", () => {
    _activityIpcServer = null;
  });
}

function stopActivityIpcServer() {
  if (_activityIpcServer) {
    _activityIpcServer.close();
    _activityIpcServer = null;
  }
  try {
    fs.unlinkSync(ACTIVITY_SOCKET_PATH);
  } catch {
    // ignore
  }
  try {
    fs.unlinkSync(ACTIVITY_IPC_INFO_PATH);
  } catch {
    // ignore
  }
  _externalToInternal.clear();
}

function deactivate() {
  stopStrictLintIpcServer();
  stopActivityIpcServer();
}

module.exports = { activate, deactivate };
