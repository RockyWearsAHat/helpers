"use strict";
// src/copilot-inspector.js — Copilot customization inspection and strict linting
const vscode = require("vscode");
const path = require("path");

module.exports = function createCopilotInspector(deps) {
  const {
    getDiagnosticsChannel,
    setDiagnosticsChannel,
    getInspectorDisposable,
    setInspectorDisposable,
    beginToolCall,
    endToolCall,
  } = deps;

  let _strictLintToolDisposable = null;

  function uniquePaths(paths) {
    return [...new Set(paths.filter(Boolean))];
  }

  function getDiagnosticsOutputChannel() {
    if (!getDiagnosticsChannel()) {
      setDiagnosticsChannel(
        vscode.window.createOutputChannel("Git Shell Helpers Diagnostics"),
      );
    }
    return getDiagnosticsChannel();
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
          (diagnostic) =>
            diagnostic.severity === vscode.DiagnosticSeverity.Error,
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
            uri.fsPath.startsWith(normalizedFolder) ||
            uri.fsPath === folderPath,
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

  function registerCustomizationInspectorTool(context) {
    getInspectorDisposable()?.dispose();
    setInspectorDisposable(null);
    _strictLintToolDisposable?.dispose();
    _strictLintToolDisposable = null;

    if (!isCustomizationInspectorEnabled()) {
      return;
    }

    setInspectorDisposable(
      vscode.lm.registerTool("gsh-inspect-copilot-customization-warnings", {
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
      }),
    );
    context.subscriptions.push(getInspectorDisposable());

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

  return {
    uniquePaths,
    getDiagnosticsOutputChannel,
    getFrontmatterRange,
    getFrontmatterListEntries,
    formatHoverContents,
    makeToolResult,
    formatDiagnosticSeverity,
    isCustomizationInspectorEnabled,
    formatCustomizationInspectionReport,
    resolveCustomizationDocument,
    inspectCopilotCustomizationWarnings,
    runStrictLinting,
    registerCustomizationInspectorTool,
  };
};
