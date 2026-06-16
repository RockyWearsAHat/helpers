// lib/mcp-strict-lint-standalone.js
//
// Standalone diagnostics for strict_lint when the VS Code extension isn't
// running (e.g. Claude Code CLI, any non-VSCode agent). Instead of emulating a
// language server, it invokes each language's *own* tooling — the providers that
// best encode current best practice — and unifies their output:
//
//   JS/TS    eslint (best-practice rules) + tsc (--noEmit type errors)
//   Python   ruff (modern lint, defaults on) + mypy (when configured)
//   Rust     cargo clippy (idiomatic-Rust lints)
//   Go       go vet + staticcheck (when present)
//   Shell    shellcheck
//
// Each diagnostic keeps its rule id and message so the agent doesn't just fix the
// symptom — it learns the principle the provider is teaching.
"use strict";

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------
function onPath(cmd) {
  const r = spawnSync(process.platform === "win32" ? "where" : "command",
    process.platform === "win32" ? [cmd] : ["-v", cmd],
    { encoding: "utf8", shell: process.platform !== "win32" });
  return r.status === 0 && (r.stdout || "").trim().length > 0;
}

function findUp(startDir, names) {
  let dir = startDir;
  for (let i = 0; i < 40; i++) {
    for (const n of names) {
      if (fs.existsSync(path.join(dir, n))) return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

function localBin(root, bin) {
  let dir = root;
  for (let i = 0; i < 40; i++) {
    const p = path.join(dir, "node_modules", ".bin", bin);
    if (fs.existsSync(p)) return p;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

function run(cmd, args, opts = {}) {
  return spawnSync(cmd, args, {
    encoding: "utf8",
    timeout: opts.timeout || 60000,
    maxBuffer: 32 * 1024 * 1024,
    cwd: opts.cwd,
    killSignal: "SIGKILL",
  });
}

// Recursively list files under a dir (skip noise), returning by extension set.
const IGNORE_DIRS = new Set([
  ".git", "node_modules", "target", "build", "out", "bin", "dist",
  ".venv", "venv", "__pycache__", ".gradle", ".idea", ".cache",
]);
function listFiles(target, extPred) {
  const out = [];
  const stat = (() => { try { return fs.statSync(target); } catch { return null; } })();
  if (!stat) return out;
  if (stat.isFile()) { if (extPred(target)) out.push(target); return out; }
  const stack = [target];
  while (stack.length) {
    const dir = stack.pop();
    let entries;
    try { entries = fs.readdirSync(dir, { withFileTypes: true }); } catch { continue; }
    for (const e of entries) {
      if (e.isDirectory()) {
        if (IGNORE_DIRS.has(e.name) || e.name.startsWith(".")) continue;
        stack.push(path.join(dir, e.name));
      } else if (extPred(path.join(dir, e.name))) {
        out.push(path.join(dir, e.name));
      }
    }
  }
  return out;
}

const extIs = (...exts) => (f) => exts.some((e) => f.toLowerCase().endsWith(e));

// Linter results: { ran, tools: [names that executed], skipped, diagnostics }.
// A diagnostic: { file, line, col, severity: error|warning|hint, rule, message, tool }
function diag(file, line, col, severity, rule, message, tool) {
  return { file, line: line || 0, col: col || 0, severity, rule: rule || "", message: (message || "").trim(), tool };
}

// ---------------------------------------------------------------------------
// linters — each returns { ran, skipped, diagnostics }
// ---------------------------------------------------------------------------
function lintEslint(target, root) {
  const files = listFiles(target, extIs(".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs"));
  if (!files.length) return null;
  const configDir = findUp(root, [
    "eslint.config.js", "eslint.config.mjs", "eslint.config.cjs", "eslint.config.ts",
    ".eslintrc.js", ".eslintrc.cjs", ".eslintrc.json", ".eslintrc.yml", ".eslintrc.yaml", ".eslintrc",
  ]);
  const bin = localBin(root, "eslint") || (onPath("eslint") ? "eslint" : null);
  if (!bin) return { ran: false, skipped: "eslint (not installed)", diagnostics: [] };
  if (!configDir) return { ran: false, skipped: "eslint (no config found)", diagnostics: [] };
  // Pass a path relative to the config dir: flat config matches files against
  // cwd, and an absolute path can fall outside the project (e.g. macOS
  // /tmp -> /private/tmp), yielding a false "clean".
  const relTarget = path.relative(configDir, target) || ".";
  const r = run(bin, ["--format", "json", "--no-error-on-unmatched-pattern", relTarget], { cwd: configDir });
  const diags = [];
  try {
    for (const f of JSON.parse(r.stdout || "[]")) {
      for (const m of f.messages || []) {
        const sev = m.severity === 2 ? "error" : "warning";
        diags.push(diag(f.filePath, m.line, m.column, sev, m.ruleId, m.message, "eslint"));
      }
    }
  } catch {
    if ((r.stderr || "").trim()) return { ran: false, skipped: `eslint (error: ${r.stderr.trim().split("\n")[0]})`, diagnostics: [] };
  }
  return { ran: true, tools: ["eslint"], skipped: null, diagnostics: diags };
}

function lintTsc(target, root, scopeIsFile) {
  // tsc is project-wide; only run for folder/workspace scope to stay fast.
  if (scopeIsFile) return null;
  const tsRoot = findUp(root, ["tsconfig.json"]);
  if (!tsRoot) return null;
  if (!listFiles(target, extIs(".ts", ".tsx")).length) return null;
  const bin = localBin(root, "tsc") || (onPath("tsc") ? "tsc" : null);
  if (!bin) return { ran: false, skipped: "tsc (not installed)", diagnostics: [] };
  const r = run(bin, ["--noEmit", "--pretty", "false"], { cwd: tsRoot, timeout: 120000 });
  const diags = [];
  const re = /^(.+?)\((\d+),(\d+)\):\s+(error|warning)\s+(TS\d+):\s+(.*)$/;
  for (const line of (r.stdout || "").split("\n")) {
    const m = re.exec(line);
    if (m) diags.push(diag(path.resolve(tsRoot, m[1]), +m[2], +m[3], m[4], m[5], m[6], "tsc"));
  }
  return { ran: true, tools: ["tsc"], skipped: null, diagnostics: diags };
}

function lintRuff(target, root) {
  if (!listFiles(target, extIs(".py", ".pyi")).length) return null;
  if (!onPath("ruff")) return { ran: false, skipped: "ruff (not installed — `pip install ruff`)", diagnostics: [] };
  const r = run("ruff", ["check", "--output-format", "json", "--force-exclude", target], { cwd: root });
  const diags = [];
  try {
    for (const v of JSON.parse(r.stdout || "[]")) {
      diags.push(diag(v.filename, v.location?.row, v.location?.column, "warning", v.code, v.message, "ruff"));
    }
  } catch { /* ruff prints nothing on clean */ }
  return { ran: true, tools: ["ruff"], skipped: null, diagnostics: diags };
}

function lintMypy(target, root) {
  if (!listFiles(target, extIs(".py")).length) return null;
  const cfgDir = findUp(root, ["mypy.ini", ".mypy.ini"]) ||
    (findUp(root, ["pyproject.toml"]) && /\[tool\.mypy\]/.test(safeRead(path.join(findUp(root, ["pyproject.toml"]), "pyproject.toml"))) ? findUp(root, ["pyproject.toml"]) : null);
  if (!cfgDir) return null; // only run mypy when explicitly configured (avoids noise)
  if (!onPath("mypy")) return { ran: false, skipped: "mypy (not installed)", diagnostics: [] };
  const r = run("mypy", ["--no-error-summary", "--show-error-codes", "--no-color-output", target], { cwd: cfgDir, timeout: 120000 });
  const diags = [];
  const re = /^(.+?):(\d+):(?:(\d+):)?\s+(error|note|warning):\s+(.*?)(?:\s+\[([\w-]+)\])?$/;
  for (const line of (r.stdout || "").split("\n")) {
    const m = re.exec(line);
    if (m) {
      const sev = m[4] === "error" ? "error" : m[4] === "warning" ? "warning" : "hint";
      diags.push(diag(path.resolve(cfgDir, m[1]), +m[2], +(m[3] || 0), sev, m[6] || "", m[5], "mypy"));
    }
  }
  return { ran: true, tools: ["mypy"], skipped: null, diagnostics: diags };
}

function lintClippy(target, root) {
  const cargoDir = findUp(root, ["Cargo.toml"]);
  if (!cargoDir) return null;
  if (!listFiles(target, extIs(".rs")).length) return null;
  if (!onPath("cargo")) return { ran: false, skipped: "clippy (cargo not installed)", diagnostics: [] };
  const r = run("cargo", ["clippy", "--message-format=json", "-q"], { cwd: cargoDir, timeout: 180000 });
  const diags = [];
  for (const line of (r.stdout || "").split("\n")) {
    if (!line.trim()) continue;
    let m; try { m = JSON.parse(line); } catch { continue; }
    if (m.reason !== "compiler-message" || !m.message) continue;
    const msg = m.message;
    if (!["error", "warning"].includes(msg.level)) continue;
    const span = (msg.spans || []).find((s) => s.is_primary) || (msg.spans || [])[0];
    if (!span) continue;
    diags.push(diag(path.resolve(cargoDir, span.file_name), span.line_start, span.column_start,
      msg.level, (msg.code && msg.code.code) || "", msg.message, "clippy"));
  }
  return { ran: true, tools: ["clippy"], skipped: null, diagnostics: diags };
}

function lintGo(target, root) {
  const goRoot = findUp(root, ["go.mod"]);
  if (!goRoot) return null;
  if (!listFiles(target, extIs(".go")).length) return null;
  const results = { ran: false, tools: [], skipped: null, diagnostics: [] };
  if (onPath("go")) {
    const r = run("go", ["vet", "./..."], { cwd: goRoot, timeout: 120000 });
    results.ran = true;
    results.tools.push("go vet");
    const re = /^(.+?\.go):(\d+):(?:(\d+):)?\s+(.*)$/;
    for (const line of (r.stderr || "").split("\n")) {
      const m = re.exec(line.trim());
      if (m) results.diagnostics.push(diag(path.resolve(goRoot, m[1]), +m[2], +(m[3] || 0), "warning", "vet", m[4], "go vet"));
    }
  } else {
    results.skipped = "go vet (go not installed)";
  }
  if (onPath("staticcheck")) {
    const r = run("staticcheck", ["-f", "json", "./..."], { cwd: goRoot, timeout: 120000 });
    for (const line of (r.stdout || "").split("\n")) {
      if (!line.trim()) continue;
      let m; try { m = JSON.parse(line); } catch { continue; }
      if (!m.location) continue;
      const sev = m.severity === "error" ? "error" : "warning";
      results.diagnostics.push(diag(m.location.file, m.location.line, m.location.column, sev, m.code, m.message, "staticcheck"));
    }
    results.ran = true;
    results.tools.push("staticcheck");
  }
  return results;
}

function lintShellcheck(target) {
  const files = listFiles(target, extIs(".sh", ".bash"));
  if (!files.length) return null;
  if (!onPath("shellcheck")) return { ran: false, skipped: "shellcheck (not installed)", diagnostics: [] };
  const r = run("shellcheck", ["-f", "json", ...files.slice(0, 500)]);
  const diags = [];
  try {
    for (const v of JSON.parse(r.stdout || "[]")) {
      const sev = v.level === "error" ? "error" : v.level === "warning" ? "warning" : "hint";
      diags.push(diag(v.file, v.line, v.column, sev, `SC${v.code}`, v.message, "shellcheck"));
    }
  } catch { /* none */ }
  return { ran: true, tools: ["shellcheck"], skipped: null, diagnostics: diags };
}

function safeRead(p) { try { return fs.readFileSync(p, "utf8"); } catch { return ""; } }

const LINTERS = [
  (t, r, f) => lintEslint(t, r),
  (t, r, f) => lintTsc(t, r, f),
  (t, r) => lintRuff(t, r),
  (t, r) => lintMypy(t, r),
  (t, r) => lintClippy(t, r),
  (t, r) => lintGo(t, r),
  (t) => lintShellcheck(t),
];

// ---------------------------------------------------------------------------
// entry
// ---------------------------------------------------------------------------
function runStandaloneLint(args = {}) {
  const filePath = args.filePath;
  const folderPath = args.folderPath;
  const target = path.resolve(filePath || folderPath || args.workspaceRoot || process.cwd());
  if (!fs.existsSync(target)) {
    return `strict_lint (standalone): target not found: ${target}`;
  }
  const scopeIsFile = !!filePath && (() => { try { return fs.statSync(target).isFile(); } catch { return false; } })();
  const root = scopeIsFile ? path.dirname(target) : target;
  const filter = args.severityFilter || "all";

  const ran = [];
  const skipped = [];
  let diagnostics = [];
  for (const linter of LINTERS) {
    let res;
    try { res = linter(target, root, scopeIsFile); } catch (e) { continue; }
    if (!res) continue; // language not present in target
    if (res.ran) ran.push(...(res.tools || []));
    if (res.skipped) skipped.push(res.skipped);
    diagnostics.push(...res.diagnostics);
  }

  // severity filter
  if (filter === "errors-only") diagnostics = diagnostics.filter((d) => d.severity === "error");
  else if (filter === "warnings-and-above") diagnostics = diagnostics.filter((d) => d.severity !== "hint");

  return formatReport({ target, scopeIsFile, ran: [...new Set(ran)], skipped, diagnostics, filter });
}

function formatReport({ target, ran, skipped, diagnostics, filter }) {
  const counts = { error: 0, warning: 0, hint: 0 };
  for (const d of diagnostics) counts[d.severity]++;

  const lines = [];
  lines.push(`strict_lint (standalone) — ${target}`);
  lines.push(`providers run: ${ran.length ? ran.join(", ") : "none"}${skipped.length ? `  |  skipped: ${skipped.join(", ")}` : ""}`);

  if (ran.length === 0 && skipped.length === 0) {
    lines.push("");
    lines.push("No language tooling matched this target. Install a provider to lint here:");
    lines.push("  JS/TS → eslint + typescript | Python → ruff | Rust → clippy | Go → staticcheck | Shell → shellcheck");
    return lines.join("\n");
  }

  if (diagnostics.length === 0) {
    lines.push("");
    lines.push(`✓ Clean — 0 ${filter === "all" ? "errors/warnings/hints" : filter} from: ${ran.join(", ")}.`);
    if (skipped.length) lines.push(`(Some providers were skipped: ${skipped.join(", ")}.)`);
    return lines.join("\n");
  }

  const order = { error: 0, warning: 1, hint: 2 };
  diagnostics.sort((a, b) =>
    order[a.severity] - order[b.severity] ||
    a.file.localeCompare(b.file) || a.line - b.line);

  const label = { error: "ERRORS", warning: "WARNINGS", hint: "HINTS / RECOMMENDATIONS" };
  let current = null;
  for (const d of diagnostics) {
    if (d.severity !== current) {
      current = d.severity;
      lines.push("");
      lines.push(`${label[current]} (${counts[current]})`);
    }
    const rule = d.rule ? ` [${d.tool}:${d.rule}]` : ` [${d.tool}]`;
    lines.push(`  ${shorten(d.file)}:${d.line}:${d.col}${rule} ${d.message}`);
  }

  lines.push("");
  lines.push(`Summary: ${counts.error} error(s), ${counts.warning} warning(s), ${counts.hint} hint(s).`);
  lines.push("Each rule id is a best-practice principle from the language's own tooling — fix it and apply the principle going forward, don't just silence it.");
  return lines.join("\n");
}

function shorten(file) {
  const cwd = process.cwd();
  const r = path.relative(cwd, file);
  return r && !r.startsWith("..") ? r : file;
}

module.exports = { runStandaloneLint };
