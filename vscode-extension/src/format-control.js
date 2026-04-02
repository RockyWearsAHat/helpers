"use strict";
// src/format-control.js — Bypass formatters on agent save
//
// When gitShellHelpers.formatControl.bypassOnAgentSave is enabled, this
// module suppresses editor.formatOnSave (and editor.codeActionsOnSave) so
// that agent-triggered file saves do not run Prettier, ESLint fix-on-save,
// or similar formatters mid-edit. Formatting is deferred to an explicit
// command that runs once at the very end of the request.

const vscode = require("vscode");

module.exports = function createFormatControl() {
  let _enabled = false;
  let _originalFormatOnSave = undefined;
  let _originalCodeActionsOnSave = undefined;
  let _suppressed = false;

  /**
   * Read the current setting value.
   */
  function _isSettingEnabled() {
    return vscode.workspace
      .getConfiguration("gitShellHelpers.formatControl")
      .get("bypassOnAgentSave", false);
  }

  /**
   * Suppress editor.formatOnSave and editor.codeActionsOnSave at the
   * workspace level so agent saves don't trigger formatters.
   */
  async function _suppress() {
    if (_suppressed) return;

    const editorConfig = vscode.workspace.getConfiguration("editor");

    // Capture originals from the effective config (user + workspace merged)
    const inspectFoS = editorConfig.inspect("formatOnSave");
    _originalFormatOnSave =
      inspectFoS?.workspaceValue ??
      inspectFoS?.globalValue ??
      inspectFoS?.defaultValue ??
      false;

    const inspectCA = editorConfig.inspect("codeActionsOnSave");
    _originalCodeActionsOnSave =
      inspectCA?.workspaceValue ??
      inspectCA?.globalValue ??
      inspectCA?.defaultValue ??
      {};

    // Disable formatting on save at workspace level
    await editorConfig.update(
      "formatOnSave",
      false,
      vscode.ConfigurationTarget.Workspace,
    );
    await editorConfig.update(
      "codeActionsOnSave",
      {},
      vscode.ConfigurationTarget.Workspace,
    );

    _suppressed = true;
  }

  /**
   * Restore the original editor.formatOnSave and editor.codeActionsOnSave
   * values. If the original was the default, remove the workspace override.
   */
  async function _restore() {
    if (!_suppressed) return;

    const editorConfig = vscode.workspace.getConfiguration("editor");

    // Restore formatOnSave — remove workspace override if original came from
    // user/default scope, otherwise set the workspace value back.
    const inspectFoS = editorConfig.inspect("formatOnSave");
    const hadWorkspaceFoS = inspectFoS?.workspaceValue !== undefined;
    if (
      _originalFormatOnSave === inspectFoS?.defaultValue &&
      !hadWorkspaceFoS
    ) {
      // Original was the default — just remove our workspace override
      await editorConfig.update(
        "formatOnSave",
        undefined,
        vscode.ConfigurationTarget.Workspace,
      );
    } else {
      await editorConfig.update(
        "formatOnSave",
        _originalFormatOnSave,
        vscode.ConfigurationTarget.Workspace,
      );
    }

    // Restore codeActionsOnSave
    const inspectCA = editorConfig.inspect("codeActionsOnSave");
    const hadWorkspaceCA = inspectCA?.workspaceValue !== undefined;
    const originalIsEmpty =
      _originalCodeActionsOnSave &&
      Object.keys(_originalCodeActionsOnSave).length === 0;
    if (originalIsEmpty && !hadWorkspaceCA) {
      await editorConfig.update(
        "codeActionsOnSave",
        undefined,
        vscode.ConfigurationTarget.Workspace,
      );
    } else {
      await editorConfig.update(
        "codeActionsOnSave",
        _originalCodeActionsOnSave,
        vscode.ConfigurationTarget.Workspace,
      );
    }

    _suppressed = false;
  }

  /**
   * Format all currently open text editors. Agents call this command once
   * at the very end of a request to apply deferred formatting.
   */
  async function formatOpenFiles() {
    // Temporarily restore formatOnSave so formatDocument uses the real
    // formatter config, then re-suppress after.
    const wasSuppressed = _suppressed;
    if (wasSuppressed) {
      await _restore();
    }

    const seen = new Set();
    for (const tabGroup of vscode.window.tabGroups.all) {
      for (const tab of tabGroup.tabs) {
        if (tab.input instanceof vscode.TabInputText) {
          const uri = tab.input.uri;
          const key = uri.toString();
          if (seen.has(key)) continue;
          seen.add(key);

          try {
            // Open the document and format it
            const doc = await vscode.workspace.openTextDocument(uri);
            if (doc.isClosed || doc.uri.scheme !== "file") continue;
            await vscode.window.showTextDocument(doc, {
              preview: false,
              preserveFocus: true,
            });
            await vscode.commands.executeCommand(
              "editor.action.formatDocument",
            );
            await doc.save();
          } catch {
            // Skip files that can't be formatted (binary, etc.)
          }
        }
      }
    }

    // Re-suppress if the setting is still enabled
    if (wasSuppressed && _isSettingEnabled()) {
      await _suppress();
    }
  }

  /**
   * Initialize: read the setting and suppress if enabled.
   * Returns disposables to add to context.subscriptions.
   */
  function activate(context) {
    _enabled = _isSettingEnabled();
    if (_enabled) {
      _suppress();
    }

    // React to setting changes
    const configDisposable = vscode.workspace.onDidChangeConfiguration((e) => {
      if (
        e.affectsConfiguration(
          "gitShellHelpers.formatControl.bypassOnAgentSave",
        )
      ) {
        const nowEnabled = _isSettingEnabled();
        if (nowEnabled && !_enabled) {
          _suppress();
        } else if (!nowEnabled && _enabled) {
          _restore();
        }
        _enabled = nowEnabled;
      }
    });

    // Command: format all open files (agents call this at the end)
    const formatCmd = vscode.commands.registerCommand(
      "gitShellHelpers.formatOpenFiles",
      formatOpenFiles,
    );

    context.subscriptions.push(configDisposable, formatCmd);
  }

  /**
   * Cleanup: restore original settings on extension deactivation.
   */
  async function deactivate() {
    await _restore();
  }

  return { activate, deactivate, formatOpenFiles };
};
