"use strict";
// src/gpg-auth.js — GitHub authentication and GPG key management
const vscode = require("vscode");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFile } = require("child_process");

module.exports = function createGpgAuth(deps) {
  const {
    getCachedRepos,
    setCachedRepos,
    getCachedUser,
    setCachedUser,
    getCachedGpgNeedsUpload,
    setCachedGpgNeedsUpload,
    getCachedGpgUploadFailed,
    setCachedGpgUploadFailed,
    getWebviewProvider,
    runGh,
    isGhAuthed,
    getGhUser,
    fetchRepos,
    getWhitelist,
    setWhitelist,
    getMode,
    buildSettingsJson,
    syncAllSettings,
    readJsonFile,
    writeJsonFile,
    globalSettingsPath,
    workspaceSettingsPath,
    SCHEMA_VERSION,
    PREDEFINED,
  } = deps;

  // ---------------------------------------------------------------------------
  // Commands
  // ---------------------------------------------------------------------------

  async function loginGitHub() {
    if (getCachedUser()) {
      vscode.window.showInformationMessage(
        `Already signed in as ${getCachedUser()}.`,
      );
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

      setCachedUser(session.account.label);

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

      setCachedRepos(await fetchRepos());
      await checkGpgUploadStatus();
      getWebviewProvider()?.refresh();
      syncAllSettings();
    } catch {
      /* User cancelled or auth failed */
    }
  }

  async function checkGpgUploadStatus() {
    setCachedGpgNeedsUpload(false);
    setCachedGpgUploadFailed(false);
    if (!getCachedUser()) return;
    try {
      const keyId = (
        await execAsync("git", ["config", "--global", "user.signingkey"])
      ).trim();
      if (!keyId) return;
      const list = await runGh(["gpg-key", "list"]);
      // gh gpg-key list output contains key IDs — check if our key is already there
      if (!list.toLowerCase().includes(keyId.toLowerCase().slice(-16))) {
        setCachedGpgNeedsUpload(true);
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
        setCachedGpgNeedsUpload(false);
        setCachedGpgUploadFailed(false);
        getWebviewProvider()?.refresh();
        vscode.window.showInformationMessage(
          "GPG key uploaded — future commits will show as Verified.",
        );
      } else {
        setCachedGpgUploadFailed(true);
        getWebviewProvider()?.refresh();
      }
    } catch {
      setCachedGpgUploadFailed(true);
      getWebviewProvider()?.refresh();
    }
  }

  async function logoutGitHub() {
    if (!getCachedUser()) return;
    const action = await vscode.window.showWarningMessage(
      `Sign out of GitHub (${getCachedUser()})?`,
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

    setCachedUser("");
    setCachedRepos([]);
    getWebviewProvider()?.refresh();
  }

  async function selectRepos() {
    if (!getCachedUser()) {
      vscode.window.showWarningMessage("Sign in to GitHub first.");
      return;
    }

    if (getCachedRepos().length === 0) {
      await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: "Fetching GitHub repositories…",
        },
        async () => {
          setCachedRepos(await fetchRepos());
        },
      );
    }

    const repos = getCachedRepos();
    if (repos.length === 0) {
      vscode.window.showWarningMessage("No repositories found.");
      return;
    }

    const currentWhitelist = getWhitelist();
    const items = repos.map((r) => ({
      label: r.nameWithOwner,
      description: r.visibility === "PUBLIC" ? "public" : "private",
      picked: currentWhitelist.includes(r.nameWithOwner),
    }));

    const selected = await vscode.window.showQuickPick(items, {
      canPickMany: true,
      title: "Select repositories allowed to submit to community cache",
      placeHolder: `${repos.length} repos — check the ones to whitelist`,
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
      `GitHub user: ${getCachedUser() || "(not signed in)"}`,
      `Mode: ${mode}`,
      "",
      `Global JSON: ${globalExists ? globalFile : "not found"}`,
      globalData ? `  mode: ${globalData.mode}` : "",
      "",
      `Loaded repos: ${getCachedRepos().length}`,
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
  // GPG key provisioning for Verified Commits
  // ---------------------------------------------------------------------------

  function execAsync(cmd, args, opts = {}) {
    return new Promise((resolve, reject) => {
      execFile(
        cmd,
        args,
        { timeout: 30000, ...opts },
        (err, stdout, stderr) => {
          if (err) reject(new Error(stderr || err.message));
          else resolve(stdout);
        },
      );
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
      name = (
        await execAsync("git", ["config", "--global", "user.name"])
      ).trim();
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
    if (!getCachedUser()) return false;
    try {
      const pubKey = await execAsync(gpgCommand, [
        "--armor",
        "--export",
        keyId,
      ]);
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

  return {
    loginGitHub,
    checkGpgUploadStatus,
    uploadGpgKeyNow,
    logoutGitHub,
    selectRepos,
    showCommunityStatus,
    execAsync,
    execAsyncStdin,
    resolveGpgCommand,
    resolveBrewCommand,
    installGpgWithBrew,
    ensureGpgAvailable,
    ensureGpgKey,
    uploadGpgKeyToGitHub,
  };
};
