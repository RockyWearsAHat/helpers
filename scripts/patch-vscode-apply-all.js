#!/usr/bin/env node
// patch-vscode-apply-all.js
//
// Coordinator script that applies VS Code patches across multiple bundles.
// Manages pristine backups per bundle and applies patches in sequence.
//
// Workbench patches (require full Cmd+Q restart):
//   folder-switch      — removes workspace folder switch confirmation dialog
//
// Git extension patches (pick up on Reload Window):
//   git-head-display   — supports branch name display override via .git/gsh-head-override
//
// Usage:
//   node patch-vscode-apply-all.js           # apply all patches
//   node patch-vscode-apply-all.js --check   # check status of all patches
//   node patch-vscode-apply-all.js --revert  # restore pristine bundles
//   node patch-vscode-apply-all.js --json    # output status as JSON (for extension)

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

const BUNDLES = {
  workbench: {
    path: path.join(VSCODE_PATH, "out/vs/workbench/workbench.desktop.main.js"),
    label: "VS Code Workbench",
    requiresRestart: true,
  },
  git: {
    path: path.join(VSCODE_PATH, "extensions/git/dist/main.js"),
    label: "Git Extension",
    requiresRestart: false,
  },
};

const PATCHES_DIR = __dirname;

const PATCH_DEFS = [
  {
    name: "folder-switch",
    description: "Remove workspace folder switch confirmation dialog",
    script: path.join(PATCHES_DIR, "patch-vscode-folder-switch.js"),
    bundle: "workbench",
  },
  {
    name: "git-head-display",
    description: "Support branch name display override for worktrees",
    script: path.join(PATCHES_DIR, "patch-vscode-git-head-display.js"),
    bundle: "git",
  },
];

// Legacy detection patterns for workbench bundle
const LEGACY_WORKBENCH_PATTERNS = [
  "active-chat-session.json", // legacy chat-bridge patch
  "_doStopExtensionHosts();let o=xg", // folder-switch patch
];

function backupPath(bundlePath) {
  return bundlePath + ".pristine";
}

function checkPatch(patchScript) {
  try {
    execSync(`node "${patchScript}" --check`, { stdio: "pipe" });
    return "patched";
  } catch {
    return "unpatched";
  }
}

function getStatus() {
  const results = [];
  for (const def of PATCH_DEFS) {
    results.push({
      name: def.name,
      description: def.description,
      bundle: def.bundle,
      status: checkPatch(def.script),
    });
  }
  const bundleStatus = {};
  for (const [key, info] of Object.entries(BUNDLES)) {
    bundleStatus[key] = {
      exists: fs.existsSync(info.path),
      backupExists: fs.existsSync(backupPath(info.path)),
    };
  }
  return {
    bundles: bundleStatus,
    patches: results,
    allPatched: results.every((r) => r.status === "patched"),
  };
}

// --json mode
if (process.argv.includes("--json")) {
  console.log(JSON.stringify(getStatus()));
  process.exit(0);
}

// --check mode
if (process.argv.includes("--check")) {
  const status = getStatus();
  console.log("VS Code Patches");
  console.log("=".repeat(40));
  for (const p of status.patches) {
    const icon = p.status === "patched" ? "✓" : "✗";
    const bundleLabel = BUNDLES[p.bundle]?.label || p.bundle;
    console.log(`  ${icon} ${p.name}: ${p.status}`);
    console.log(`    ${p.description} [${bundleLabel}]`);
  }
  console.log("");
  if (status.allPatched) {
    console.log("All patches applied.");
  } else {
    console.log("Some patches missing. Run without --check to apply.");
  }
  for (const [key, info] of Object.entries(status.bundles)) {
    console.log(
      `${BUNDLES[key].label} backup: ${info.backupExists ? "exists" : "missing"}`,
    );
  }
  process.exit(status.allPatched ? 0 : 1);
}

// --revert mode
if (process.argv.includes("--revert")) {
  let reverted = false;
  for (const [key, info] of Object.entries(BUNDLES)) {
    const backup = backupPath(info.path);
    if (fs.existsSync(backup)) {
      fs.copyFileSync(backup, info.path);
      console.log(`Restored pristine ${info.label} bundle.`);
      reverted = true;
    }
  }
  if (!reverted) {
    console.error("No pristine backups found.");
    process.exit(1);
  }
  console.log("Quit and restart VS Code to activate.");
  process.exit(0);
}

// Apply mode: backup each bundle, then apply patches per bundle
// Group patches by bundle
const patchesByBundle = {};
for (const def of PATCH_DEFS) {
  if (!patchesByBundle[def.bundle]) patchesByBundle[def.bundle] = [];
  patchesByBundle[def.bundle].push(def);
}

let failed = false;
let needsRestart = false;

for (const [bundleKey, patches] of Object.entries(patchesByBundle)) {
  const info = BUNDLES[bundleKey];
  if (!info) {
    console.error(`Unknown bundle: ${bundleKey}`);
    failed = true;
    break;
  }
  if (!fs.existsSync(info.path)) {
    console.error(`Bundle not found: ${info.path}`);
    failed = true;
    break;
  }

  const backup = backupPath(info.path);

  // Create pristine backup if needed
  if (!fs.existsSync(backup)) {
    if (bundleKey === "workbench") {
      // Check for legacy patches in workbench
      const src = fs.readFileSync(info.path, "utf8");
      const hasLegacy = LEGACY_WORKBENCH_PATTERNS.some((p) => src.includes(p));
      if (hasLegacy) {
        const legacyBak = info.path + ".bak";
        const legacyFolderBak = info.path + ".folder-switch.bak";
        if (fs.existsSync(legacyBak)) {
          fs.copyFileSync(legacyBak, backup);
          console.log(
            `[${info.label}] Created pristine backup from legacy .bak file.`,
          );
        } else if (fs.existsSync(legacyFolderBak)) {
          fs.copyFileSync(legacyFolderBak, backup);
          console.log(
            `[${info.label}] Created pristine backup from legacy folder-switch.bak file.`,
          );
        } else {
          console.error(
            `[${info.label}] Has patches but no pristine backup found.`,
          );
          failed = true;
          break;
        }
      } else {
        fs.copyFileSync(info.path, backup);
        console.log(`[${info.label}] Created pristine backup.`);
      }
    } else {
      fs.copyFileSync(info.path, backup);
      console.log(`[${info.label}] Created pristine backup.`);
    }
  }

  // Start from pristine for this bundle
  fs.copyFileSync(backup, info.path);
  console.log(`[${info.label}] Restored pristine for clean patch application.`);

  // Apply each patch for this bundle
  for (const def of patches) {
    try {
      const output = execSync(`node "${def.script}"`, {
        stdio: "pipe",
        encoding: "utf8",
      });
      console.log(`[${def.name}] ${output.trim()}`);
      if (info.requiresRestart) needsRestart = true;
    } catch (err) {
      console.error(`[${def.name}] FAILED: ${err.stderr || err.message}`);
      failed = true;
      break;
    }
  }

  if (failed) {
    console.error(
      `\nPatch application failed for ${info.label}. Restoring pristine.`,
    );
    fs.copyFileSync(backup, info.path);
    break;
  }
}

// Clean up legacy backup files
for (const legacy of [
  BUNDLES.workbench.path + ".bak",
  BUNDLES.workbench.path + ".folder-switch.bak",
]) {
  if (fs.existsSync(legacy)) {
    fs.unlinkSync(legacy);
    console.log(`Cleaned up legacy backup: ${path.basename(legacy)}`);
  }
}

if (failed) {
  process.exit(1);
}

console.log("\nAll patches applied successfully.");
if (needsRestart) {
  console.log(
    "Quit and restart VS Code to activate workbench patches (Cmd+Q, then reopen).",
  );
  console.log("Note: Reload Window is NOT sufficient for workbench patches.");
} else {
  console.log("Reload Window to activate Git extension patches.");
}
