#!/usr/bin/env node
"use strict";

/*
 * gsh — unified control surface for Git Shell Helpers.
 *
 * One command to install GSH into any AI agent (Claude Code, GitHub Copilot),
 * toggle the whole tool surface on/off, enable/disable individual tools, check
 * health, and grade a CS project toward an A+.
 *
 * GSH is agent-agnostic: the tools ship as a standard stdio MCP server, so any
 * MCP-capable agent can use them. This CLI handles the per-agent wiring.
 *
 * Tool state lives in ~/.config/git-shell-helpers-mcp/tools.json and is re-read
 * by the running MCP server on every request, so every toggle here takes effect
 * live — no agent restart required.
 */

const fs = require("fs");
const path = require("path");
const os = require("os");
const { spawnSync } = require("child_process");

const HOME = process.env.HOME || process.env.USERPROFILE || os.homedir();
const REPO_DIR = fs.realpathSync(path.dirname(fs.realpathSync(__filename)));
const SERVER_PATH = path.join(REPO_DIR, "git-shell-helpers-mcp");
const DAEMON_PATH = path.join(REPO_DIR, "git-shell-helpers-mcpd.js");
const SHIM_SRC = path.join(REPO_DIR, "gsh-mcp.c");
const SHIM_BIN = path.join(REPO_DIR, "gsh-mcp");
// Native Rust git-cs-grade: crate dir, its release artifact, the installed
// command, and the Node implementation used as a graceful fallback.
const CS_GRADE_CRATE = path.join(REPO_DIR, "cs-grade");
const CS_GRADE_ARTIFACT = path.join(CS_GRADE_CRATE, "target", "release", "git-cs-grade");
const CS_GRADE_CMD = path.join(REPO_DIR, "git-cs-grade");
const CS_GRADE_JS = path.join(REPO_DIR, "git-cs-grade.js");
// Native Rust MCP tools (gsh-native): the crate dir, its release artifact, and
// the installed binary the MCP daemon shells out to. Unlike git-cs-grade there
// is NO Node fallback — these tools are Rust-only, so a missing/broken build is
// a hard error the user must fix (install Rust, then `gsh build`).
const GSH_NATIVE_CRATE = path.join(REPO_DIR, "native");
const GSH_NATIVE_ARTIFACT = path.join(GSH_NATIVE_CRATE, "target", "release", "gsh-native");
const GSH_NATIVE_CMD = path.join(REPO_DIR, "gsh-native");
const TOOLS_CONFIG_DIR = path.join(HOME, ".config", "git-shell-helpers-mcp");
const TOOLS_CONFIG_PATH = path.join(TOOLS_CONFIG_DIR, "tools.json");
const CLAUDE_DIR = path.join(HOME, ".claude");
const COPILOT_DIR = path.join(HOME, ".copilot");
const CLAUDE_CONFIG_SRC = path.join(REPO_DIR, "claude-config");

// ---------------------------------------------------------------------------
// tiny ANSI helpers (auto-disabled when not a TTY or NO_COLOR is set)
// ---------------------------------------------------------------------------
const color = process.stdout.isTTY && !process.env.NO_COLOR;
const c = (n, s) => (color ? `\x1b[${n}m${s}\x1b[0m` : s);
const bold = (s) => c("1", s);
const green = (s) => c("32", s);
const red = (s) => c("31", s);
const yellow = (s) => c("33", s);
const dim = (s) => c("2", s);
const ok = green("✓");
const no = red("✗");

function die(msg, code = 1) {
  process.stderr.write(`${red("gsh:")} ${msg}\n`);
  process.exit(code);
}

// ---------------------------------------------------------------------------
// tools.json read/write
// ---------------------------------------------------------------------------
function readConfig() {
  try {
    return JSON.parse(fs.readFileSync(TOOLS_CONFIG_PATH, "utf8"));
  } catch {
    return {};
  }
}

function writeConfig(cfg) {
  fs.mkdirSync(TOOLS_CONFIG_DIR, { recursive: true });
  // Normalize shape so the file stays clean and predictable.
  const out = {
    disabled: cfg.disabled === true,
    disabledTools: Array.from(new Set(cfg.disabledTools || [])).sort(),
  };
  fs.writeFileSync(TOOLS_CONFIG_PATH, `${JSON.stringify(out, null, 2)}\n`);
  return out;
}

// ---------------------------------------------------------------------------
// query the MCP server for the full universe of tool names
// ---------------------------------------------------------------------------
function listAllTools() {
  const req =
    JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {},
    }) +
    "\n" +
    JSON.stringify({ jsonrpc: "2.0", id: 2, method: "tools/list" }) +
    "\n";
  const res = spawnSync(process.execPath, [SERVER_PATH], {
    input: req,
    encoding: "utf8",
    timeout: 20000,
    env: { ...process.env, GSH_FORCE_ENABLE: "1" },
  });
  const names = [];
  for (const line of (res.stdout || "").split("\n")) {
    if (!line.trim()) continue;
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      continue;
    }
    if (msg.id === 2 && msg.result && Array.isArray(msg.result.tools)) {
      for (const t of msg.result.tools) names.push(t.name);
    }
  }
  return names.sort();
}

// ---------------------------------------------------------------------------
// agent detection
// ---------------------------------------------------------------------------
function has(cmd) {
  const r = spawnSync(process.platform === "win32" ? "where" : "command",
    process.platform === "win32" ? [cmd] : ["-v", cmd],
    { encoding: "utf8", shell: process.platform !== "win32" });
  return r.status === 0;
}

function detectAgents() {
  const agents = [];
  if (has("claude") || fs.existsSync(CLAUDE_DIR)) agents.push("claude");
  if (fs.existsSync(COPILOT_DIR) || has("code")) agents.push("copilot");
  return agents;
}

// ---------------------------------------------------------------------------
// recursive copy (no deps)
// ---------------------------------------------------------------------------
function copyTree(src, dest) {
  const stat = fs.statSync(src);
  if (stat.isDirectory()) {
    fs.mkdirSync(dest, { recursive: true });
    for (const entry of fs.readdirSync(src)) {
      copyTree(path.join(src, entry), path.join(dest, entry));
    }
  } else {
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.copyFileSync(src, dest);
  }
}

// Write a managed block (delimited) into a file without clobbering user content.
const BLOCK_START = "<!-- GSH:BEGIN (managed by `gsh install`; do not edit) -->";
const BLOCK_END = "<!-- GSH:END -->";
function writeManagedBlock(file, body) {
  let existing = "";
  try {
    existing = fs.readFileSync(file, "utf8");
  } catch {}
  const block = `${BLOCK_START}\n${body}\n${BLOCK_END}`;
  const re = new RegExp(
    `${BLOCK_START.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[\\s\\S]*?${BLOCK_END.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`,
  );
  let next;
  if (re.test(existing)) {
    next = existing.replace(re, block);
  } else {
    next = existing.trim() ? `${existing.trimEnd()}\n\n${block}\n` : `${block}\n`;
  }
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, next);
}

function removeManagedBlock(file) {
  let existing;
  try {
    existing = fs.readFileSync(file, "utf8");
  } catch {
    return;
  }
  const re = new RegExp(
    `\\n*${BLOCK_START.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[\\s\\S]*?${BLOCK_END.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\n*`,
  );
  fs.writeFileSync(file, existing.replace(re, "\n"));
}

// ---------------------------------------------------------------------------
// fast C shim: compile gsh-mcp so cold node startup is paid once (warm daemon)
// ---------------------------------------------------------------------------
function findCC() {
  for (const cc of [process.env.CC, "cc", "clang", "gcc"].filter(Boolean)) {
    if (has(cc)) return cc;
  }
  return null;
}

// Returns the path to the runnable MCP command + args the agent should launch.
// Prefers the compiled C shim (warm daemon, ~0 startup); falls back to node.
function buildShim({ quiet } = {}) {
  if (!fs.existsSync(SHIM_SRC)) return null;
  const cc = findCC();
  if (!cc) {
    if (!quiet) console.log(`  ${yellow("!")} no C compiler found — using direct node (still works, just cold start each session)`);
    return null;
  }
  const node = process.execPath;
  const r = spawnSync(
    cc,
    [
      "-O2", "-Wall",
      `-DNODE_BIN="${node}"`,
      `-DDAEMON_JS="${DAEMON_PATH}"`,
      `-DSTDIO_JS="${SERVER_PATH}"`,
      "-o", SHIM_BIN, SHIM_SRC,
    ],
    { encoding: "utf8" },
  );
  if (r.status !== 0) {
    if (!quiet) console.log(`  ${no} shim compile failed: ${(r.stderr || "").trim().split("\n")[0]}`);
    return null;
  }
  try { fs.chmodSync(SHIM_BIN, 0o755); } catch {}
  if (!quiet) console.log(`  ${ok} compiled fast C launcher (gsh-mcp) — fast startup, auto-managed background server`);
  return SHIM_BIN;
}

// The command+args to register with an agent for the MCP server.
function mcpLaunch() {
  if (fs.existsSync(SHIM_BIN)) return { cmd: SHIM_BIN, args: [] };
  return { cmd: process.execPath, args: [SERVER_PATH] };
}

// ---------------------------------------------------------------------------
// native git-cs-grade: compile the Rust crate so grading starts in ~1ms and
// runs several times faster, falling back to the Node implementation when no
// Rust toolchain (cargo) is available.
// ---------------------------------------------------------------------------
function findCargo() {
  for (const c of [process.env.CARGO, "cargo"].filter(Boolean)) {
    if (has(c)) return c;
  }
  return null;
}

// Install the Node implementation as the `git-cs-grade` command (the fallback).
function installCsGradeFallback() {
  if (!fs.existsSync(CS_GRADE_JS)) return false;
  try {
    fs.copyFileSync(CS_GRADE_JS, CS_GRADE_CMD);
    fs.chmodSync(CS_GRADE_CMD, 0o755);
    return true;
  } catch {
    return false;
  }
}

// Compile cs-grade to a native binary and install it as `git-cs-grade`.
// Returns the command path on success (native or Node fallback), else null.
function buildCsGrade({ quiet } = {}) {
  if (!fs.existsSync(CS_GRADE_CRATE)) return null;
  const cargo = findCargo();
  if (!cargo) {
    const used = installCsGradeFallback();
    if (!quiet) {
      console.log(`  ${yellow("!")} no Rust toolchain (cargo) found — using the Node git-cs-grade${used ? "" : " (missing!)"}.`);
      console.log(dim("      Install Rust for the fast native build: https://rustup.rs  (or: brew install rust), then run `gsh build`."));
    }
    return used ? CS_GRADE_CMD : null;
  }
  const r = spawnSync(cargo, ["build", "--release"], { cwd: CS_GRADE_CRATE, encoding: "utf8" });
  if (r.status !== 0 || !fs.existsSync(CS_GRADE_ARTIFACT)) {
    const used = installCsGradeFallback();
    if (!quiet) {
      console.log(`  ${no} git-cs-grade build failed: ${(r.stderr || "").trim().split("\n").pop() || "unknown error"}`);
      console.log(dim(`      Falling back to the Node implementation${used ? "" : " (also missing!)"}. Fix the build, then run \`gsh build\`.`));
    }
    return used ? CS_GRADE_CMD : null;
  }
  try {
    fs.copyFileSync(CS_GRADE_ARTIFACT, CS_GRADE_CMD);
    fs.chmodSync(CS_GRADE_CMD, 0o755);
  } catch (e) {
    if (!quiet) console.log(`  ${no} could not install git-cs-grade: ${e.message}`);
    return null;
  }
  if (!quiet) console.log(`  ${ok} compiled native git-cs-grade (Rust) — ~1ms startup, faster scans`);
  return CS_GRADE_CMD;
}

// Compile the native MCP tool binary (gsh-native) and install it at the repo
// root. These tools are Rust-only — there is no Node fallback — so any failure
// is reported as a hard error and sets a non-zero exit code, making `gsh build`
// and `gsh install` fail loudly so the user can fix the toolchain.
function buildNative({ quiet } = {}) {
  const fail = (msg, hint) => {
    console.log(`  ${no} ${msg}`);
    if (hint) console.log(dim(`      ${hint}`));
    process.exitCode = 1;
    return false;
  };
  if (!fs.existsSync(GSH_NATIVE_CRATE)) {
    return fail(`native tool crate missing at ${GSH_NATIVE_CRATE}`, "Reinstall the GSH source — the `native/` crate is required.");
  }
  const cargo = findCargo();
  if (!cargo) {
    return fail(
      "no Rust toolchain (cargo) found — the native MCP tools cannot be built.",
      "Install Rust: https://rustup.rs  (or: brew install rust), then run `gsh build`. These tools are required; there is no Node fallback.",
    );
  }
  const r = spawnSync(cargo, ["build", "--release"], { cwd: GSH_NATIVE_CRATE, encoding: "utf8" });
  if (r.status !== 0 || !fs.existsSync(GSH_NATIVE_ARTIFACT)) {
    return fail(
      `gsh-native build failed: ${(r.stderr || "").trim().split("\n").pop() || "unknown error"}`,
      "Fix the Rust build error above, then run `gsh build`.",
    );
  }
  try {
    fs.copyFileSync(GSH_NATIVE_ARTIFACT, GSH_NATIVE_CMD);
    fs.chmodSync(GSH_NATIVE_CMD, 0o755);
  } catch (e) {
    return fail(`could not install gsh-native: ${e.message}`, `Check write permissions on ${REPO_DIR}.`);
  }
  // Sanity-check that the binary answers `schemas` with valid JSON, so a broken
  // binary is caught at build time rather than when the agent first calls a tool.
  const probe = spawnSync(GSH_NATIVE_CMD, ["schemas"], { encoding: "utf8" });
  let toolCount = 0;
  try { toolCount = JSON.parse(probe.stdout).length; } catch {}
  if (probe.status !== 0 || toolCount === 0) {
    return fail("gsh-native built but did not report any tools.", "Run `" + GSH_NATIVE_CMD + " schemas` to see the error.");
  }
  if (!quiet) console.log(`  ${ok} compiled native MCP tools (gsh-native, Rust) — ${toolCount} tool(s), ~1ms startup`);
  return true;
}

function listDaemons() {
  const dir = path.join(HOME, ".cache", "gsh");
  try {
    return fs.readdirSync(dir).filter((f) => /^mcpd-.*\.sock$/.test(f)).map((f) => path.join(dir, f));
  } catch {
    return [];
  }
}

function cmdDaemon(args) {
  const sub = args[0] || "status";
  if (sub === "status") {
    const socks = listDaemons();
    const r = spawnSync("pgrep", ["-f", "git-shell-helpers-mcpd"], { encoding: "utf8" });
    const pids = (r.stdout || "").trim().split("\n").filter(Boolean);
    console.log(bold("\nGSH background server"));
    console.log(`  running:  ${pids.length ? green(pids.join(", ")) : dim("idle (starts automatically on next use)")}`);
    console.log(`  sockets:  ${socks.length || dim("0")}`);
    socks.forEach((s) => console.log(dim(`    ${s}`)));
    console.log(dim("\n  Managed automatically: starts on demand, exits after ~15 min idle.\n"));
    return;
  }
  if (sub === "stop" || sub === "restart") {
    spawnSync("pkill", ["-f", "git-shell-helpers-mcpd"], { stdio: "ignore" });
    for (const s of listDaemons()) { try { fs.unlinkSync(s); } catch {} }
    console.log(`${ok} Stopped the background server${sub === "restart" ? " (restarts automatically on next use)" : ""}.`);
    console.log(dim("Use this after changing GSH code so the next launch picks it up."));
    return;
  }
  die(`unknown 'gsh daemon' subcommand: ${sub} (use status|stop|restart)`);
}

// ---------------------------------------------------------------------------
// install: Claude Code
// ---------------------------------------------------------------------------
function installClaude(force) {
  console.log(bold("\n→ Installing GSH for Claude Code"));

  // 0. Build the fast C shim (warm daemon) and native git-cs-grade — each
  //    falls back to Node when its toolchain is unavailable.
  buildShim();
  buildCsGrade();
  buildNative();
  const { cmd, args } = mcpLaunch();

  // 1. Register the MCP server (user scope) — idempotent.
  if (has("claude")) {
    spawnSync("claude", ["mcp", "remove", "-s", "user", "gsh"], {
      stdio: "ignore",
    });
    const add = spawnSync(
      "claude",
      ["mcp", "add", "-s", "user", "gsh", "--", cmd, ...args],
      { encoding: "utf8" },
    );
    if (add.status === 0) {
      console.log(`  ${ok} MCP server 'gsh' registered (user scope) via ${cmd === SHIM_BIN ? "fast C shim" : "node"}`);
    } else {
      console.log(`  ${no} MCP registration failed: ${(add.stderr || "").trim()}`);
    }
  } else {
    console.log(`  ${yellow("!")} 'claude' CLI not found — add MCP manually:`);
    console.log(dim(`      claude mcp add -s user gsh -- ${cmd} ${args.join(" ")}`));
  }

  // 2. CLAUDE.md managed block (always-on core behavior).
  const coreFile = path.join(CLAUDE_CONFIG_SRC, "CLAUDE.gsh.md");
  if (fs.existsSync(coreFile)) {
    writeManagedBlock(path.join(CLAUDE_DIR, "CLAUDE.md"), fs.readFileSync(coreFile, "utf8").trim());
    console.log(`  ${ok} GSH core written to ~/.claude/CLAUDE.md (managed block)`);
  }

  // 3. Skills, commands, agents.
  for (const kind of ["skills", "commands", "agents"]) {
    const src = path.join(CLAUDE_CONFIG_SRC, kind);
    if (!fs.existsSync(src)) continue;
    const dest = path.join(CLAUDE_DIR, kind);
    for (const entry of fs.readdirSync(src)) {
      copyTree(path.join(src, entry), path.join(dest, entry));
    }
    console.log(`  ${ok} ${kind} installed to ~/.claude/${kind}/`);
  }
  console.log(dim("  Restart Claude Code (or /mcp reconnect) to pick up the gsh server."));
}

// ---------------------------------------------------------------------------
// install: GitHub Copilot (delegates to existing audit installer)
// ---------------------------------------------------------------------------
function installCopilot(force) {
  console.log(bold("\n→ Installing GSH for GitHub Copilot"));
  const bin = path.join(REPO_DIR, "git-copilot-devops-audit");
  if (!fs.existsSync(bin)) {
    console.log(`  ${no} git-copilot-devops-audit not found in ${REPO_DIR}`);
    return;
  }
  const args = ["--update-agent"];
  if (force) args.push("--force");
  const r = spawnSync(bin, args, { stdio: "inherit" });
  if (r.status === 0) console.log(`  ${ok} Copilot agents/instructions/skills installed`);
  else console.log(`  ${no} Copilot install exited with code ${r.status}`);
}

function cmdInstall(args) {
  let agent = "auto";
  let force = false;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--agent") agent = args[++i];
    else if (args[i] === "--force") force = true;
  }
  let targets;
  if (agent === "all") targets = ["claude", "copilot"];
  else if (agent === "auto") {
    targets = detectAgents();
    if (targets.length === 0) {
      die("no AI agent detected. Use --agent claude|copilot|all to force.");
    }
    console.log(dim(`Detected: ${targets.join(", ")}`));
  } else targets = [agent];

  for (const t of targets) {
    if (t === "claude") installClaude(force);
    else if (t === "copilot") installCopilot(force);
    else die(`unknown agent '${t}'`);
  }
  // Ensure tools.json exists in a known-good state.
  if (!fs.existsSync(TOOLS_CONFIG_PATH)) writeConfig(readConfig());
  console.log(green(bold("\nGSH installed.")) + dim(" Run `gsh status` to verify."));
}

function cmdUninstall(args) {
  let agent = args[0] === "--agent" ? args[1] : "claude";
  if (agent === "claude" || agent === "all") {
    if (has("claude")) spawnSync("claude", ["mcp", "remove", "-s", "user", "gsh"], { stdio: "ignore" });
    removeManagedBlock(path.join(CLAUDE_DIR, "CLAUDE.md"));
    console.log(`${ok} Removed gsh MCP server and CLAUDE.md block (skills/commands/agents left in place).`);
  }
  if (agent === "copilot" || agent === "all") {
    console.log(dim("Copilot config left in place; remove ~/.copilot/{agents,instructions,skills} manually if desired."));
  }
}

// ---------------------------------------------------------------------------
// enable / disable / bypass (master switch)
// ---------------------------------------------------------------------------
function setMaster(disabled) {
  const cfg = readConfig();
  cfg.disabled = disabled;
  writeConfig(cfg);
  if (disabled) console.log(`${ok} GSH ${bold("disabled")} (bypassed). All GSH tools hidden. Re-enable: ${bold("gsh enable")}`);
  else console.log(`${ok} GSH ${bold("enabled")}. Tool surface active.`);
  console.log(dim("Takes effect live — no agent restart needed."));
}

function cmdBypass(args) {
  const arg = (args[0] || "").toLowerCase();
  if (arg === "on") return setMaster(true);
  if (arg === "off") return setMaster(false);
  // toggle
  return setMaster(!readConfig().disabled);
}

// ---------------------------------------------------------------------------
// tool subcommands
// ---------------------------------------------------------------------------
function cmdTool(args) {
  const sub = args[0];
  const cfg = readConfig();
  const disabledSet = new Set(cfg.disabledTools || []);

  if (!sub || sub === "list") {
    const all = listAllTools();
    if (all.length === 0) {
      console.log(yellow("No tools reported by the MCP server (load error?). Run `gsh doctor`."));
      return;
    }
    console.log(bold(`\nGSH tools (${all.length})  ${cfg.disabled ? red("[MASTER: DISABLED]") : green("[MASTER: ENABLED]")}\n`));
    for (const name of all) {
      const off = disabledSet.has(name) || cfg.disabled;
      console.log(`  ${off ? no : ok} ${off ? dim(name) : name}`);
    }
    console.log(dim(`\nToggle: gsh tool disable <name> | gsh tool enable <name> | gsh tool enable all`));
    return;
  }

  if (sub === "enable" || sub === "disable") {
    const target = args[1];
    if (!target) die(`usage: gsh tool ${sub} <name|all>`);
    const all = listAllTools();
    if (target === "all") {
      cfg.disabledTools = sub === "disable" ? all.slice() : [];
      writeConfig(cfg);
      console.log(`${ok} ${sub === "disable" ? "Disabled" : "Enabled"} all ${all.length} tools.`);
      return;
    }
    if (all.length && !all.includes(target)) {
      console.log(yellow(`Warning: '${target}' is not a known GSH tool. Known: ${all.join(", ")}`));
    }
    if (sub === "disable") disabledSet.add(target);
    else disabledSet.delete(target);
    cfg.disabledTools = Array.from(disabledSet);
    writeConfig(cfg);
    console.log(`${ok} Tool '${target}' ${sub === "disable" ? "disabled" : "enabled"}.`);
    console.log(dim("Takes effect live."));
    return;
  }

  if (sub === "reset") {
    cfg.disabledTools = [];
    writeConfig(cfg);
    console.log(`${ok} Cleared per-tool disables.`);
    return;
  }

  die(`unknown 'gsh tool' subcommand: ${sub}`);
}

// ---------------------------------------------------------------------------
// status / doctor
// ---------------------------------------------------------------------------
function claudeMcpRegistered() {
  if (!has("claude")) return null;
  const r = spawnSync("claude", ["mcp", "get", "gsh"], { encoding: "utf8" });
  return r.status === 0;
}

function cmdStatus() {
  const cfg = readConfig();
  const all = listAllTools();
  const disabledCount = (cfg.disabledTools || []).length;
  console.log(bold("\nGSH status"));
  console.log(`  source repo:   ${REPO_DIR}`);
  console.log(`  MCP server:    ${fs.existsSync(SERVER_PATH) ? ok + " " + SERVER_PATH : no + " missing"}`);
  console.log(`  master switch: ${cfg.disabled ? red("DISABLED (bypassed)") : green("ENABLED")}`);
  console.log(`  tools:         ${all.length} total, ${disabledCount} disabled`);
  console.log(bold("\n  Agents"));
  const agents = detectAgents();
  const reg = claudeMcpRegistered();
  console.log(`    claude:  ${agents.includes("claude") ? ok + " present" : dim("not detected")}` +
    (reg === null ? "" : reg ? `  (${green("mcp registered")})` : `  (${yellow("mcp NOT registered — run gsh install")})`));
  console.log(`    copilot: ${agents.includes("copilot") ? ok + " present" : dim("not detected")}`);
  if (disabledCount) console.log(dim(`\n  Disabled tools: ${(cfg.disabledTools || []).join(", ")}`));
  console.log("");
}

function cmdDoctor() {
  console.log(bold("\nGSH doctor\n"));
  let problems = 0;
  const check = (label, pass, hint) => {
    console.log(`  ${pass ? ok : no} ${label}`);
    if (!pass && hint) { console.log(dim(`      → ${hint}`)); problems++; }
  };
  check("node available", true);
  check("MCP server file present", fs.existsSync(SERVER_PATH), `expected at ${SERVER_PATH}`);
  const all = listAllTools();
  check(`MCP server starts and lists tools (${all.length})`, all.length > 0, "check `node " + SERVER_PATH + "` for load errors");
  check("tools.json present", fs.existsSync(TOOLS_CONFIG_PATH), "run `gsh enable` to create it");
  check("fast C launcher compiled", fs.existsSync(SHIM_BIN), "run `gsh build` (needs a C compiler); optional — falls back to node");
  {
    // git-cs-grade is "native" when the installed command is the compiled
    // binary (Mach-O/ELF) rather than the Node script (starts with a shebang).
    let native = false;
    try { native = !fs.readFileSync(CS_GRADE_CMD).slice(0, 2).equals(Buffer.from("#!")); } catch {}
    check("git-cs-grade native (Rust) build", native, "run `gsh build` (needs cargo: https://rustup.rs); optional — falls back to node");
  }
  {
    // gsh-native is REQUIRED — the MCP tools it hosts have no Node fallback.
    let nativeTools = 0;
    if (fs.existsSync(GSH_NATIVE_CMD)) {
      const r = spawnSync(GSH_NATIVE_CMD, ["schemas"], { encoding: "utf8" });
      try { nativeTools = JSON.parse(r.stdout).length; } catch {}
    }
    check(
      `gsh-native MCP tools built (${nativeTools})`,
      nativeTools > 0,
      "REQUIRED (no Node fallback) — install Rust (https://rustup.rs), then run `gsh build`.",
    );
  }
  if (fs.existsSync(SHIM_BIN)) {
    const r = spawnSync("pgrep", ["-f", "git-shell-helpers-mcpd"], { encoding: "utf8" });
    const up = (r.stdout || "").trim().length > 0;
    console.log(`  ${dim("·")} background server ${up ? "running" : "idle (starts automatically on use)"}`);
  }
  const reg = claudeMcpRegistered();
  if (reg !== null) check("gsh registered with Claude Code", reg, "run `gsh install --agent claude`");
  check("claude-config bundled", fs.existsSync(CLAUDE_CONFIG_SRC), "reinstall GSH source");
  console.log(problems === 0 ? green("\n  All checks passed.\n") : yellow(`\n  ${problems} issue(s) found.\n`));
}

// ---------------------------------------------------------------------------
// grade (delegates to git-cs-grade)
// ---------------------------------------------------------------------------
function cmdGrade(args) {
  const bin = path.join(REPO_DIR, "git-cs-grade");
  if (!fs.existsSync(bin)) die("git-cs-grade not found in " + REPO_DIR);
  const r = spawnSync(bin, args, { stdio: "inherit" });
  process.exit(r.status || 0);
}

// ---------------------------------------------------------------------------
// index: cheap project map (delegates to the native gsh-native binary)
// ---------------------------------------------------------------------------
function projectRoot() {
  const top = spawnSync("git", ["rev-parse", "--show-toplevel"], {
    encoding: "utf8",
  });
  return top.status === 0 ? top.stdout.trim() : process.cwd();
}

function cmdIndex(args) {
  if (!fs.existsSync(GSH_NATIVE_CMD)) {
    die("gsh-native is not built — run `gsh build` (needs Rust: https://rustup.rs).");
  }
  const root = projectRoot();
  const sub = args[0] || "build";

  // Run a native MCP tool and return its text content.
  const callTool = (tool, payload) => {
    const r = spawnSync(GSH_NATIVE_CMD, ["call", tool], {
      input: JSON.stringify({ root, ...payload }),
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    });
    let parsed;
    try {
      parsed = JSON.parse(r.stdout);
    } catch {
      die(`gsh-native ${tool} failed: ${(r.stderr || r.stdout || "").trim()}`);
    }
    if (parsed.error) die(parsed.error.message);
    return parsed.content.map((c) => c.text).join("\n");
  };
  // Run a native CLI subcommand, streaming its output.
  const runNative = (nativeArgs) => {
    const r = spawnSync(GSH_NATIVE_CMD, nativeArgs, { stdio: "inherit" });
    process.exit(r.status || 0);
  };

  switch (sub) {
    case "build":
      console.log(callTool("index_project", {}));
      return;
    case "map":
      console.log(callTool("project_map", {}));
      return;
    case "lookup":
    case "where": {
      const query = args.slice(1).join(" ").trim();
      if (!query) die("usage: gsh index lookup <symbol-or-file>");
      console.log(callTool("lookup", { query }));
      return;
    }
    case "export": {
      const out = args[1] || path.join(root, `${path.basename(root)}.dxbundle`);
      runNative(["bundle", root, out]);
      return;
    }
    case "install": {
      if (!args[1]) die("usage: gsh index install <bundle.dxbundle>");
      runNative(["install", root, args[1]]);
      return;
    }
    case "refs":
    case "list":
      runNative(["refs", root]);
      return;
    default:
      die(`unknown 'gsh index' subcommand: ${sub} (build|map|lookup|export|install|refs)`);
  }
}

// ---------------------------------------------------------------------------
// help
// ---------------------------------------------------------------------------
function help() {
  console.log(`${bold("gsh")} — Git Shell Helpers control surface

${bold("USAGE")}
  gsh <command> [options]

${bold("SETUP")}
  install [--agent auto|claude|copilot|all] [--force]
                         Install/wire GSH into AI agent(s). Default: auto-detect.
  uninstall [--agent claude|copilot|all]
                         Unregister MCP server + remove managed CLAUDE.md block.
  status                 Show install state, master switch, tool counts, agents.
  doctor                 Run health checks.
  build                  (Re)compile the fast C launcher (gsh-mcp), the native
                         Rust git-cs-grade, and the native MCP tools
                         (gsh-native). gsh-native is required (no Node fallback);
                         a failed build is reported as an error.

${bold("BACKGROUND SERVER (auto-managed; for fast startup)")}
  daemon status          Show whether the background server is running.
  daemon restart         Restart it (use after changing GSH code).

${bold("TOGGLE GSH (master switch — live, no restart)")}
  enable                 Turn the whole GSH tool surface on.
  disable                Turn it off (bypass). Agents see zero GSH tools.
  bypass [on|off]        Toggle (no arg) or set the master switch.

${bold("TOGGLE INDIVIDUAL TOOLS")}
  tool list              List every tool with on/off state.
  tool disable <name|all>
  tool enable  <name|all>
  tool reset             Re-enable every individually-disabled tool.

${bold("GRADING")}
  grade [path] [--course cs2420|cs3500] [--apply]
                         Grade a CS project and write GRADE.md toward an A+.

${bold("PROJECT INDEX (cheap repo map — orient without grepping)")}
  index [build]          Build/refresh the project index (.gsh/index/).
  index map              Print the ranked module map + Mermaid graph.
  index lookup <q>       Find where a symbol is defined / what references it.
  index export [file]    Export the index as a portable .dxbundle.
  index install <file>   Install another project's .dxbundle for reference.
  index refs             List installed reference indexes.

${bold("NOTES")}
  • Tool state lives in ${dim("~/.config/git-shell-helpers-mcp/tools.json")}
    and is re-read live by the MCP server — toggles need no agent restart.
  • Agents can override a disabled tool for one call with ${dim("{ force: true }")}.
`);
}

// ---------------------------------------------------------------------------
// dispatch
// ---------------------------------------------------------------------------
function main() {
  const [cmd, ...rest] = process.argv.slice(2);
  switch (cmd) {
    case undefined:
    case "status": return cmdStatus();
    case "help":
    case "-h":
    case "--help": return help();
    case "install": return cmdInstall(rest);
    case "uninstall": return cmdUninstall(rest);
    case "enable": return setMaster(false);
    case "disable": return setMaster(true);
    case "bypass": return cmdBypass(rest);
    case "tool":
    case "tools": return cmdTool(rest);
    case "doctor": return cmdDoctor();
    case "daemon": return cmdDaemon(rest);
    case "build": buildShim(); buildCsGrade(); buildNative(); return;
    case "grade": return cmdGrade(rest);
    case "index": return cmdIndex(rest);
    default:
      die(`unknown command '${cmd}'. Run \`gsh help\`.`);
  }
}

main();
