// patch-vscode-git-head-display.js
//
// Patches VS Code's built-in Git extension to support a "display override"
// for the branch name shown in the status bar.
//
// When the file .git/gsh-head-override exists in a repo root, its contents
// (trimmed) replace the branch name in the status bar — WITHOUT changing
// the real HEAD, index, or working tree.  This lets our extension show
// worktree branch names without git checkout/stash overhead.
//
// Two injection points:
//   1. `get headLabel()` — returns override branch name instead of real HEAD
//   2. SyncStatusBar `get command()` — hides the sync button when override
//      is active (prevents misleading "Sync to origin/dev" on a display-
//      overridden branch that has no upstream)
//
// Usage (standalone — normally called via patch-vscode-apply-all.js):
//   node patch-vscode-git-head-display.js          # apply patch
//   node patch-vscode-git-head-display.js --check  # check status
//   node patch-vscode-git-head-display.js --revert # revert to backup

const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

function detectVscodePath() {
  const candidates =
    process.platform === "darwin"
      ? [
          "/Applications/Visual Studio Code.app/Contents/Resources/app",
          (process.env.HOME || "") +
            "/Applications/Visual Studio Code.app/Contents/Resources/app",
        ]
      : process.platform === "win32"
        ? [
            (process.env.LOCALAPPDATA || "") +
              "\\Programs\\Microsoft VS Code\\resources\\app",
            "C:\\Program Files\\Microsoft VS Code\\resources\\app",
            "C:\\Program Files (x86)\\Microsoft VS Code\\resources\\app",
          ]
        : [
            "/usr/share/code/resources/app",
            "/opt/visual-studio-code/resources/app",
            "/snap/code/current/usr/share/code/resources/app",
          ];
  for (const c of candidates) {
    if (c && fs.existsSync(c)) return c;
  }
  try {
    const probe = process.platform === "win32" ? "where code" : "which code";
    const codeExe = execSync(probe, {
      timeout: 3000,
      stdio: ["pipe", "pipe", "pipe"],
    })
      .toString()
      .trim()
      .split("\n")[0]
      .trim();
    if (codeExe) {
      let dir = path.dirname(fs.realpathSync(codeExe));
      for (let i = 0; i < 8; i++) {
        const candidate = path.join(dir, "resources", "app");
        if (fs.existsSync(candidate)) return candidate;
        dir = path.dirname(dir);
      }
    }
  } catch {}
  return null;
}

const VSCODE_PATH = detectVscodePath();
if (!VSCODE_PATH) {
  console.error(
    "[patch-vscode] Could not locate VS Code installation. Tried platform defaults and PATH.",
  );
  process.exit(1);
}
const BUNDLE = path.join(VSCODE_PATH, "extensions/git/dist/main.js");

// ---------------------------------------------------------------------------
// Patch 1: headLabel override
// ---------------------------------------------------------------------------
const OLD_HEAD = "get headLabel(){let e=this.HEAD;return e?(e.name||";
const NEW_HEAD =
  "get headLabel(){let e=this.HEAD;" +
  'try{let g=require("fs").readFileSync(' +
  'require("path").join(this.repository.root,".git","gsh-head-override"),' +
  '"utf8").trim();if(g)return g}catch{}' +
  "return e?(e.name||";

// ---------------------------------------------------------------------------
// Patch 2: hide sync button when override is active
// ---------------------------------------------------------------------------
const OLD_SYNC =
  "get command(){if(!this.state.enabled)return;" +
  "if(!this.state.hasRemotes){if(this.state.remoteSourcePublishers.length===0)return;";
const NEW_SYNC =
  "get command(){" +
  'try{let g=require("fs").readFileSync(' +
  'require("path").join(this.repository.root,".git","gsh-head-override"),' +
  '"utf8").trim();if(g)return}catch{}' +
  "if(!this.state.enabled)return;" +
  "if(!this.state.hasRemotes){if(this.state.remoteSourcePublishers.length===0)return;";

// All patches in apply order
const PATCHES = [
  { old: OLD_HEAD, new: NEW_HEAD, name: "headLabel" },
  { old: OLD_SYNC, new: NEW_SYNC, name: "syncButton" },
];

function isPatchable() {
  if (!fs.existsSync(BUNDLE)) return "missing";
  const src = fs.readFileSync(BUNDLE, "utf8");
  const allPatched = PATCHES.every((p) => src.includes(p.new.slice(0, 60)));
  if (allPatched) return "patched";
  const allOriginal = PATCHES.every((p) => src.includes(p.old));
  if (allOriginal) return "unpatched";
  return "partial";
}

function apply(bundleSrc) {
  if (!bundleSrc) bundleSrc = fs.readFileSync(BUNDLE, "utf8");
  let changed = false;
  for (const p of PATCHES) {
    if (bundleSrc.includes(p.new.slice(0, 60))) continue;
    const idx = bundleSrc.indexOf(p.old);
    if (idx === -1) {
      return {
        src: bundleSrc,
        changed,
        error: `injection point not found for ${p.name}`,
      };
    }
    bundleSrc =
      bundleSrc.slice(0, idx) + p.new + bundleSrc.slice(idx + p.old.length);
    changed = true;
  }
  return { src: bundleSrc, changed };
}

function revert(bundleSrc) {
  if (!bundleSrc) bundleSrc = fs.readFileSync(BUNDLE, "utf8");
  let changed = false;
  for (const p of [...PATCHES].reverse()) {
    if (!bundleSrc.includes(p.new.slice(0, 60))) continue;
    const idx = bundleSrc.indexOf(p.new);
    if (idx === -1) continue;
    bundleSrc =
      bundleSrc.slice(0, idx) + p.old + bundleSrc.slice(idx + p.new.length);
    changed = true;
  }
  return { src: bundleSrc, changed };
}

module.exports = {
  OLD: OLD_HEAD,
  NEW: NEW_HEAD,
  OLD_SYNC,
  NEW_SYNC,
  PATCHES,
  BUNDLE,
  isPatchable,
  apply,
  revert,
};

// Standalone CLI
if (require.main === module) {
  const arg = process.argv[2];
  if (arg === "--check") {
    const status = isPatchable();
    if (status === "patched") {
      console.log("PATCHED");
      process.exit(0);
    } else if (status === "unpatched") {
      console.log("UNPATCHED");
      process.exit(1);
    } else {
      console.log(
        "UNKNOWN — injection point not found. VS Code version may have changed.",
      );
      process.exit(1);
    }
  }

  if (arg === "--revert") {
    const result = revert();
    if (result.changed) {
      fs.writeFileSync(BUNDLE, result.src, "utf8");
      console.log("Reverted git-head-display patch.");
    } else {
      console.log("Not patched — nothing to revert.");
    }
    process.exit(0);
  }

  // Default: apply
  const result = apply();
  if (result.error) {
    console.error(
      "Injection point not found — VS Code version may have changed.",
    );
    process.exit(1);
  }
  if (result.changed) {
    fs.writeFileSync(BUNDLE, result.src, "utf8");
    console.log("Applied git-head-display patch.");
  } else {
    console.log("Already patched.");
  }
}
