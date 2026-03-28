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
        case "openChatSession":
          vscode.commands.executeCommand("workbench.action.chat.open");
          break;
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
            .update("enabled", msg.enabled, vscode.ConfigurationTarget.Global);
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
          if (_ollamaPinned.has(m)) {
            _ollamaPinned.delete(m);
          } else {
            _ollamaPinned.add(m);
          }
          _context.globalState.update("gsh.ollama.pinned", [..._ollamaPinned]);
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
          cachedGpgUploadFailed = false;
          cachedUser = "";
          cachedRepos = [];
          _webviewProvider?.refresh();
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
    if (!cachedUser) {
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

    const gpgHint = cachedGpgNeedsUpload
      ? cachedGpgUploadFailed
        ? `<div style="font-size:10.5px;color:var(--vscode-descriptionForeground);margin-top:6px">Upload failed. <span role="button" id="reloginGpgBtn" style="color:var(--vscode-textLink-foreground);text-decoration:underline;cursor:pointer">Re-login</span></div>`
        : `<div style="font-size:10.5px;color:var(--vscode-descriptionForeground);margin-top:6px">Key not on GitHub — commits show Unverified. <span role="button" id="uploadGpgBtn" style="color:var(--vscode-textLink-foreground);text-decoration:underline;cursor:pointer">Upload now</span></div>`
      : "";

    const cpConfig = vscode.workspace.getConfiguration(
      "gitShellHelpers.checkpoint",
    );
    const cpEnabled = cpConfig.get("enabled", true);
    const cpAutoPush = cpConfig.get("autoPush", false);
    const cpSign = cpConfig.get("sign", false);
    const mcpStatus = getMcpStatusViewModel(_context);

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
            .filter((m) => _ollamaPinned.has(m))
            .map(
              (m) => `
        <div class="provider-model-row">
          <span class="provider-model-dot"></span>
          <span class="provider-model-name">${escapeHtml(m)}</span>
          <button class="provider-model-run" data-ollamarun="${escapeHtml(m)}" title="ollama run ${escapeHtml(m)}">run</button>
          <button class="provider-model-remove" data-ollamatoggle="${escapeHtml(m)}" title="Remove">×</button>
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
              const pinned = _ollamaPinned.has(m);
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
            placeholder="${providerStatus.anthropicKey ? "●●●●●●●● (saved)" : "sk-ant-…"}"
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
            placeholder="${providerStatus.openaiKey ? "●●●●●●●● (saved)" : "sk-…"}"
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
    const activityRows =
      activityItems.length > 0
        ? activityItems
            .map((item) => _renderActivityItem(item, escapeHtml))
            .join("")
        : `<div class="activity-idle"><span class="activity-idle-dot"></span>idle</div>`;
    const activityCountLabel = _activityCountLabel(activityItems);

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
      const publicCount = cachedRepos.filter(
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
  .activity-item {
    border-radius: 4px; margin: 2px -5px;
    background: var(--vscode-editorWidget-background, rgba(128,128,128,0.04));
    border: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.1));
  }
  .activity-item--done {
    background: none; border-color: transparent;
    opacity: 0.7;
  }
  .activity-item--done:hover { opacity: 1; }
  .activity-row {
    display: flex; align-items: center; gap: 7px;
    padding: 5px 9px; cursor: pointer; list-style: none;
    font-size: 12px; user-select: none;
  }
  .activity-row::-webkit-details-marker { display: none; }
  .activity-title {
    flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-weight: 500;
  }
  .activity-item--done .activity-title { font-weight: 400; }
  .activity-sub {
    padding: 0 9px 5px 28px;
    font-size: 11px; line-height: 1.3;
    color: var(--vscode-descriptionForeground);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    opacity: 0.8;
  }
  .activity-meta {
    flex-shrink: 0; font-size: 10.5px;
    color: var(--vscode-descriptionForeground); opacity: 0.6;
  }
  .activity-pulse {
    flex-shrink: 0; width: 7px; height: 7px; border-radius: 999px;
    background: var(--vscode-notificationsInfoIcon-foreground, #3794ff);
    animation: pulse 1.4s ease-in-out infinite;
  }
  .activity-spinner {
    flex-shrink: 0; width: 14px; height: 14px; border-radius: 999px;
    border: 2px solid transparent;
    border-top-color: var(--vscode-notificationsInfoIcon-foreground, #3794ff);
    border-right-color: var(--vscode-notificationsInfoIcon-foreground, #3794ff);
    animation: spin 0.8s linear infinite;
  }
  .activity-dot-done {
    flex-shrink: 0; width: 6px; height: 6px; border-radius: 999px;
    background: var(--vscode-descriptionForeground); opacity: 0.4;
    margin: 0 4px;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
  .activity-elapsed {
    flex-shrink: 0; font-size: 10.5px;
    color: var(--vscode-descriptionForeground); opacity: 0.7;
  }
  .activity-chevron {
    flex-shrink: 0; width: 10px; height: 10px;
    transition: transform 0.15s; opacity: 0.5;
  }
  details[open] .activity-chevron { transform: rotate(90deg); }
  .activity-detail {
    padding: 0 9px 7px;
    font-size: 10.5px; color: var(--vscode-descriptionForeground);
  }
  .activity-detail pre {
    margin: 0; white-space: pre-wrap; word-break: break-all;
    font-family: var(--vscode-editor-font-family, monospace);
    font-size: 10px;
    max-height: 120px; overflow-y: auto;
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
    padding: 4px 2px; opacity: 0.7;
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
  /* Linger (between-tool-call) indicator */
  .activity-item--linger {
    border-radius: 3px; margin: 2px -4px;
    background: var(--vscode-editorWidget-background, rgba(128,128,128,0.04));
    border: 1px solid var(--vscode-panel-border, rgba(128,128,128,0.1));
    opacity: 0.75;
  }
  .activity-pulse--linger {
    background: var(--vscode-charts-yellow, #cca700) !important;
    animation: pulse 2s ease-in-out infinite !important;
  }
  .activity-item--session {
    cursor: pointer;
    border-radius: 3px; margin: 2px -4px; padding: 2px 4px;
  }
  .activity-item--session:hover {
    background: var(--vscode-list-hoverBackground, rgba(128,128,128,0.1));
  }
  .activity-item--session:hover .activity-label {
    text-decoration: underline;
  }

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
        <div class="acct-name">${escapeHtml(cachedUser)}</div>
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
      <div id="activityList"${activityItems.length === 0 ? ' class="activity-list-hidden"' : ""}>${activityItems.length === 0 ? "" : activityRows}</div>
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
        <div class="sect-count">${strictLintingEnabled ? "1/1" : "0/1"}</div>
      </summary>
      ${strictLintingRow}
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
      <span>${escapeHtml(cachedUser)}</span>
    </div>

    <div class="footer-gear" id="gearBtn" title="Account">
      <svg viewBox="0 0 16 16"><path d="M9.1 4.4L8.6 2H7.4l-.5 2.4-.7.3-2-1.3-.9.8 1.3 2-.2.7-2.4.5v1.2l2.4.5.3.7-1.3 2 .8.8 2-1.3.7.3.5 2.4h1.2l.5-2.4.7-.3 2 1.3.8-.8-1.3-2 .3-.7 2.4-.5V6.8l-2.4-.5-.3-.7 1.3-2-.8-.8-2 1.3-.7-.2zM9.4 8c0 .8-.6 1.4-1.4 1.4S6.6 8.8 6.6 8 7.2 6.6 8 6.6s1.4.6 1.4 1.4z"/></svg>
    </div>
  </div>
  <script>
    const vscode = acquireVsCodeApi();
    document.querySelectorAll('.tool-item').forEach(el => {
      if (el.dataset.strictLinting || el.dataset.cpkey) return;
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
        const list = document.getElementById("activityList");
        const count = document.getElementById("activityCount");
        if (!list) return;
        const items = msg.items || [];
        const active = items.filter(i => i.type === "session-active" || i.type === "tool");
        if (count) count.textContent = active.length === 0 && items.length === 0 ? "idle" : active.length === 0 ? items.length + " recent" : active.length + " running";
        const sect = list.closest(".sect--activity");
        if (items.length === 0) {
          if (sect) sect.classList.add("sect--idle");
          list.classList.add("activity-list-hidden");
          list.innerHTML = "";
          return;
        }
        if (sect) sect.classList.remove("sect--idle");
        list.classList.remove("activity-list-hidden");
        const esc = s => s.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;");
        list.innerHTML = items.map(item => {
          if (item.type === "session-active") return \`
            <div class="activity-item activity-item--session" data-sessionid="\${item.sessionId}">
              <div class="activity-row">
                <span class="activity-spinner"></span>
                <span class="activity-title">\${esc(item.label)}</span>
                <span class="activity-elapsed" data-started="\${item.startedAt}">\${item.elapsed}s</span>
              </div>
              \${item.preview ? \`<div class="activity-sub">\${esc(item.preview)}</div>\` : ""}
            </div>\`;
          if (item.type === "session-done") return \`
            <div class="activity-item activity-item--done" data-sessionid="\${item.sessionId}">
              <div class="activity-row">
                <span class="activity-dot-done"></span>
                <span class="activity-title">\${esc(item.label)}</span>
                <span class="activity-meta">completed</span>
              </div>
              \${item.preview ? \`<div class="activity-sub">\${esc(item.preview)}</div>\` : ""}
            </div>\`;
          return \`
            <details class="activity-item">
              <summary class="activity-row">
                <span class="activity-pulse"></span>
                <span class="activity-title">\${esc(item.label)}</span>
                <span class="activity-elapsed" data-started="\${item.startedAt}">\${item.elapsed}s</span>
                <svg class="activity-chevron" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 0 1 1.06 0l3.25 3.25a.75.75 0 0 1 0 1.06l-3.25 3.25a.75.75 0 0 1-1.06-1.06L8.94 8 6.22 5.28a.75.75 0 0 1 0-1.06z"/></svg>
              </summary>
              <div class="activity-detail"><pre>\${esc(item.args)}</pre></div>
            </details>\`;
        }).join("");
        list.querySelectorAll('.activity-item--session, .activity-item--done').forEach(el => {
          el.addEventListener('click', () => {
            vscode.postMessage({ type: 'openChatSession', sessionId: el.dataset.sessionid });
          });
        });
      }
    });
    // Live elapsed-time ticker
    setInterval(() => {
      document.querySelectorAll('.activity-elapsed[data-started]').forEach(el => {
        const started = parseInt(el.dataset.started, 10);
        if (!isNaN(started)) el.textContent = Math.floor((Date.now() - started) / 1000) + 's';
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
      if (btn) btn.textContent = isOpen ? '− Close' : '+ Add model';
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
  // Cancel linger — a new tool call means the session is still active
  if (_sessionLingerTimer) {
    clearTimeout(_sessionLingerTimer);
    _sessionLingerTimer = null;
  }
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
          // Agent turn starting — begin or refresh the session linger
          if (_sessionLingerTimer) {
            clearTimeout(_sessionLingerTimer);
            _sessionLingerTimer = null;
          }
          if (!_sessionStartedAt) {
            _sessionStartedAt = Date.now();
          }
          _sessionLingerTimer = setTimeout(() => {
            _sessionLingerTimer = null;
            _sessionStartedAt = 0;
            _webviewProvider?.pushUpdate({
              type: "activityUpdate",
              items: getActivityItems(),
            });
          }, SESSION_LINGER_MS);
          _webviewProvider?.pushUpdate({
            type: "activityUpdate",
            items: getActivityItems(),
          });
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
