# Obsolescence Strategy: Graceful Degradation as Upstream Features Land

This document describes how our patch system is designed to **become unnecessary** as VS Code upstream features land, without breaking during the transition period.

## Design Principle: Detect-and-Defer

Every patch integration follows the same pattern:

1. **Check if the upstream feature exists** (API available, behavior already correct)
2. **If yes**: use the native API, skip the patch
3. **If no**: fall back to the patched behavior

This means our code works **today** with patches, and will work **tomorrow** without them — the patches simply stop being needed.

## Per-Feature Transition Plans

### 1. Folder Switch Dialog Suppression

**Current**: Bundle patch replacing `stopExtensionHosts()` with `_doStopExtensionHosts()`
**Upstream target**: `suppressConfirmation` option on `updateWorkspaceFolders()`

**Transition code** (to add to extension):

```javascript
function switchWorkspaceFolder(uri) {
  // Try the upstream API option first
  try {
    const success = vscode.workspace.updateWorkspaceFolders(
      0,
      1,
      { suppressConfirmation: true }, // upstream option
      { uri },
    );
    if (success) return true;
  } catch {
    // Option not supported in this VS Code version — fall through
  }

  // Fall back to patched behavior (no dialog because patch is applied)
  return vscode.workspace.updateWorkspaceFolders(0, 1, { uri });
}
```

**When upstream lands**: The `try` branch succeeds, patches become no-ops. `patch-vscode-apply-all.js --check` reports "UNPATCHED" (injection point changed), but that's fine — the feature works natively.

**Cleanup trigger**: When the minimum `engines.vscode` version in `package.json` includes the `suppressConfirmation` option, remove the fallback path and the patch script.

### 2. Git Extension headLabel Override

**Current**: Bundle patch injecting override check into `get headLabel()`
**Upstream target**: `repository.headLabelOverride` API or `.git/head-display-override` file convention

**Transition code**:

```javascript
async function setDisplayBranch(repoRoot, branchName) {
  // Try the Git extension API first
  const gitExt = vscode.extensions.getExtension("vscode.git")?.exports;
  const api = gitExt?.getAPI(1);
  const repo = api?.repositories?.find((r) => r.rootUri.fsPath === repoRoot);

  if (repo && "headLabelOverride" in repo) {
    // Upstream API available — use it directly
    repo.headLabelOverride = branchName || undefined;
    return;
  }

  // Fall back to file-based override (works with or without our patch)
  const overridePath = path.join(repoRoot, ".git", "gsh-head-override");
  if (branchName) {
    fs.writeFileSync(overridePath, branchName + "\n", "utf8");
  } else {
    try {
      fs.unlinkSync(overridePath);
    } catch {}
  }

  // Trigger Git extension refresh to pick up the change
  // (Only needed for file-based path — API path auto-fires events)
  await vscode.commands.executeCommand("git.refresh");
}
```

**When upstream lands**: The API branch executes, no file writes needed, no patch needed.

**If upstream uses a file convention**: If they adopt `.git/head-display-override` as a standard, we rename our file from `gsh-head-override` to match. If they pick a different mechanism, we adapt.

**Cleanup trigger**: When `vscode.git` API version exposes `headLabelOverride`, remove the file-based fallback and the patch script.

### 3. Chat Session Focus Events

**Current**: `chatParticipantPrivate` proposed API with `enable-proposed-api` in `argv.json`
**Upstream target**: Stable `vscode.window.onDidChangeActiveChatSession`

**Transition code** (already partially implemented):

```javascript
function subscribeToSessionFocus(context, handler) {
  // Try stable API first
  if (vscode.window.onDidChangeActiveChatSession) {
    context.subscriptions.push(
      vscode.window.onDidChangeActiveChatSession(handler),
    );
    return "stable";
  }

  // Fall back to proposed API
  if (vscode.window.onDidChangeActiveChatPanelSessionResource) {
    context.subscriptions.push(
      vscode.window.onDidChangeActiveChatPanelSessionResource(handler),
    );
    return "proposed";
  }

  return "unavailable";
}
```

**When upstream lands**: The stable branch executes. The `enabledApiProposals` entry and `argv.json` flag become no-ops (they enable a proposed API that's no longer proposed).

**Cleanup trigger**: When the minimum `engines.vscode` version includes the stable event, remove `chatParticipantPrivate` from `enabledApiProposals` and document that `argv.json` modification is no longer needed.

### 4. runSubagent Model Parameter

**Current**: Bundle patch (`patch-vscode-runsubagent-model.js`) adding `model` field to `RunSubagentTool`'s JSON schema and injecting an override into `invoke()` before the request object is built.
**Upstream target**: `model` (and eventually `tier`) as first-class parameters in the `runSubagent` tool schema.

**Transition code**:

```javascript
// No extension API change needed — runSubagent is a built-in tool.
// When upstream adds `model` to the schema natively, the patch's NEW string
// won't match (the schema text will be different), so patch-vscode-apply-all.js
// will report UNKNOWN for this patch. That's the signal to remove it.
//
// The behavior (model override via runSubagent call) will work natively
// without any extension code change — models will just pass `model: "..."`.
```

**When upstream lands**: `patch-vscode-runsubagent-model.js --check` returns UNKNOWN (injection point not found because the schema already contains `model`). The native implementation handles the feature. Remove the patch from `PATCH_DEFS` in `patch-vscode-apply-all.js`.

**Cleanup trigger**: When VS Code's `runSubagent` schema in the workbench bundle already contains a `model` property, the patch is no longer needed.

## Patch Management During Transition

### The `patch-vscode-apply-all.js` coordinator

The coordinator already handles version changes gracefully:

- **Injection point not found**: Reports "UNKNOWN" status — the patch can't be applied. This happens when VS Code changes the bundled code (either from an update or from landing the upstream feature).
- **Already patched**: No-op. Safe to re-run.
- **Revert available**: Pristine backups allow clean rollback.

### Extension behavior when patches are missing

The extension currently shows a warning when patches aren't applied. During the transition period, this should be updated:

```javascript
function _checkVscodePatches() {
  // Skip patch check if native APIs are available
  if (vscode.workspace.updateWorkspaceFolders.length > 3) return; // has options param
  if ("headLabelOverride" in someRepo) return;

  // ...existing patch check and prompt logic
}
```

### Version gating

Each feature transition should be gated on VS Code version, not feature detection alone, to avoid false positives from API stubs:

```javascript
const VSCODE_VERSION = vscode.version;
const HAS_NATIVE_FOLDER_SWITCH = semver.gte(VSCODE_VERSION, "1.XX.0"); // fill when known
```

## File Naming Convention

Our current override file is `.git/gsh-head-override` (prefixed with `gsh-` to indicate it's our convention). If upstream adopts a standard name, we'll migrate. The `gsh-` prefix ensures we don't conflict with any future upstream convention.

## Timeline

| Feature                 | Upstream Status                        | Our Action                 | Est. Transition                             |
| ----------------------- | -------------------------------------- | -------------------------- | ------------------------------------------- |
| Folder switch           | PR #292783 merged (agent-sessions)     | Propose general API option | When VS Code exposes option publicly        |
| headLabel override      | Issue #260706 open, Git API scaffolded | Propose API addition       | When Git API v2 lands                       |
| Chat session events     | Active infrastructure work             | Propose stable promotion   | When `chatParticipantPrivate` stabilizes    |
| runSubagent model param | Not filed — patched locally            | Propose schema + invoke    | When `runSubagent` schema gains `model` key |

## Principle: Never Self-Obsolete

Our patches are **additive** — they add behavior that doesn't exist yet. They never:

- Remove upstream functionality
- Depend on internal implementation details beyond the injection point
- Modify behavior that upstream is actively changing

When upstream adds the equivalent feature, our patches simply stop being needed. The extension detects the native capability and uses it. The patch scripts report "injection point not found" and do nothing. No code breaks.
