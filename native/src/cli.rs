//! The `helpers` control CLI — pure Rust, Node-free. Dispatched busybox-style
//! when the binary is invoked as `helpers` (a symlink to `helpers-native`), or
//! explicitly via `helpers-native cli <args…>`. Ports the former Node `helpers`
//! script: status, enable/disable/bypass, tool toggles, build, update, grade,
//! index, setup, doctor, install/uninstall — reusing the in-process native tool
//! registry (no subprocess for grade/index/setup) and the embedded agent config.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::{json, Value};

use crate::embed;
use crate::git::home;
use crate::gitcli;
use crate::registry;

/// The shipped version, baked in at compile time from the repo VERSION file.
const VERSION: &str = include_str!("../../VERSION");
const REPO_SLUG: &str = "RockyWearsAHat/helpers";
const EXE: &str = if cfg!(windows) { ".exe" } else { "" };

/// git-* CLIs (and `helpers`, `git-cs-grade`) symlinked busybox-style to the binary.
const SYMLINK_NAMES: &[&str] = &[
    "helpers",
    "git-resolve",
    "git-remerge",
    "git-fucked-the-push",
    "git-initialize",
    "git-get",
    "git-scan-for-leaked-envs",
    "git-upload",
    "git-checkpoint",
    "git-help-i-pushed-an-env",
    "git-cs-grade",
];

// ── colors ───────────────────────────────────────────────────────────────────
fn color_on() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}
fn paint(code: &str, s: &str) -> String {
    if color_on() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}
fn bold(s: &str) -> String { paint("1", s) }
fn green(s: &str) -> String { paint("32", s) }
fn red(s: &str) -> String { paint("31", s) }
fn yellow(s: &str) -> String { paint("33", s) }
fn dim(s: &str) -> String { paint("2", s) }
fn ok_mark() -> String { green("✓") }
fn no_mark() -> String { red("✗") }

// ── paths ────────────────────────────────────────────────────────────────────
/// The real binary path (resolves the `helpers` symlink to `helpers-native`).
fn native_bin() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(&p).ok().or(Some(p)))
        .unwrap_or_else(|| PathBuf::from(format!("helpers-native{EXE}")))
}
/// Directory the binary + its symlinks live in.
fn repo_dir() -> PathBuf {
    native_bin().parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."))
}
fn tools_config_path() -> PathBuf {
    home().join(".config").join("helpers-server").join("tools.json")
}
fn claude_dir() -> PathBuf { home().join(".claude") }
fn copilot_dir() -> PathBuf { home().join(".copilot") }

// ── tools.json config ────────────────────────────────────────────────────────
fn read_config() -> Value {
    std::fs::read_to_string(tools_config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}
/// Persist a normalized config (`{disabled, disabledTools[]}`), sorted+deduped.
fn write_config(disabled: bool, disabled_tools: &[String]) {
    let mut set: Vec<String> = disabled_tools.to_vec();
    set.sort();
    set.dedup();
    let out = json!({ "disabled": disabled, "disabledTools": set });
    let path = tools_config_path();
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(&out).unwrap()));
}
fn cfg_disabled(cfg: &Value) -> bool {
    cfg.get("disabled").and_then(Value::as_bool).unwrap_or(false)
}
fn cfg_disabled_tools(cfg: &Value) -> Vec<String> {
    cfg.get("disabledTools")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Every tool name the server serves: native registry + the web tools, sorted.
fn list_all_tools() -> Vec<String> {
    let mut names: Vec<String> = registry::schemas()
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_string))
        .collect();
    names.push("search_web".into());
    names.push("scrape_webpage".into());
    names.sort();
    names.dedup();
    names
}

// ── small process helpers ────────────────────────────────────────────────────
fn has_cmd(cmd: &str) -> bool {
    let (probe, args): (&str, &[&str]) =
        if cfg!(windows) { ("where", &[]) } else { ("command", &["-v"]) };
    let mut c = Command::new(probe);
    if cfg!(windows) {
        c.arg(cmd);
    } else {
        c.args(args).arg(cmd);
    }
    c.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn die(msg: &str) -> ! {
    eprintln!("{} {msg}", red("helpers:"));
    std::process::exit(1);
}

// ── dispatch ─────────────────────────────────────────────────────────────────
/// Entry point for the CLI. `args` are the arguments after the program name.
pub fn run(args: &[String]) -> ExitCode {
    let cmd = args.first().map(String::as_str).unwrap_or("status");
    let rest = if args.len() > 1 { &args[1..] } else { &[] };
    match cmd {
        "status" => cmd_status(),
        "help" | "-h" | "--help" => help(),
        "enable" => set_master(false),
        "disable" => set_master(true),
        "bypass" => cmd_bypass(rest),
        "tool" | "tools" => cmd_tool(rest),
        "doctor" => cmd_doctor(),
        "build" => return cmd_build(rest),
        "update" => cmd_update(rest),
        "grade" => return cmd_grade(rest),
        "index" => return cmd_index(rest),
        "setup" => return cmd_setup(rest),
        "install" => cmd_install(rest),
        "uninstall" => cmd_uninstall(rest),
        other => {
            eprintln!("{} unknown command '{other}'. Run `helpers help`.", red("helpers:"));
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

// ── status ───────────────────────────────────────────────────────────────────
fn cmd_status() {
    let cfg = read_config();
    let all = list_all_tools();
    let disabled_count = cfg_disabled_tools(&cfg).len();
    println!("{}", bold("\nHelpers status"));
    println!("  binary:        {}", native_bin().display());
    println!("  version:       v{}", VERSION.trim());
    println!(
        "  master switch: {}",
        if cfg_disabled(&cfg) { red("DISABLED (bypassed)") } else { green("ENABLED") }
    );
    println!("  tools:         {} total, {disabled_count} disabled", all.len());
    println!("{}", bold("\n  Agents"));
    let claude = has_cmd("claude") || claude_dir().exists();
    let copilot = copilot_dir().exists() || has_cmd("code");
    let reg = claude_mcp_registered();
    let reg_note = match reg {
        Some(true) => format!("  ({})", green("mcp registered")),
        Some(false) => format!("  ({})", yellow("mcp NOT registered — run helpers install")),
        None => String::new(),
    };
    println!(
        "    claude:  {}{reg_note}",
        if claude { format!("{} present", ok_mark()) } else { dim("not detected") }
    );
    println!(
        "    copilot: {}",
        if copilot { format!("{} present", ok_mark()) } else { dim("not detected") }
    );
    if disabled_count > 0 {
        println!("{}", dim(&format!("\n  Disabled tools: {}", cfg_disabled_tools(&cfg).join(", "))));
    }
    if let Some(latest) = cached_newer_version() {
        println!(
            "{}",
            yellow(&format!("\n  ↑ Update available: v{} → v{latest}", VERSION.trim()))
                + &dim(" — run `helpers update`")
        );
    }
    println!();
}

fn claude_mcp_registered() -> Option<bool> {
    if !has_cmd("claude") {
        return None;
    }
    Some(
        Command::new("claude")
            .args(["mcp", "get", "helpers"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
    )
}

// ── enable / disable / bypass ────────────────────────────────────────────────
fn set_master(disabled: bool) {
    let cfg = read_config();
    write_config(disabled, &cfg_disabled_tools(&cfg));
    if disabled {
        println!(
            "{} Helpers {} (bypassed). All Helpers tools hidden. Re-enable: {}",
            ok_mark(), bold("disabled"), bold("helpers enable")
        );
    } else {
        println!("{} Helpers {}. Tool surface active.", ok_mark(), bold("enabled"));
    }
    println!("{}", dim("Takes effect live — no agent restart needed."));
}

fn cmd_bypass(args: &[String]) {
    match args.first().map(|s| s.to_lowercase()).as_deref() {
        Some("on") => set_master(true),
        Some("off") => set_master(false),
        _ => set_master(!cfg_disabled(&read_config())),
    }
}

// ── tool toggles ─────────────────────────────────────────────────────────────
fn cmd_tool(args: &[String]) {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    let cfg = read_config();
    let mut disabled: Vec<String> = cfg_disabled_tools(&cfg);
    match sub {
        "list" => {
            let all = list_all_tools();
            let master = cfg_disabled(&cfg);
            println!(
                "{}",
                bold(&format!(
                    "\nHelpers tools ({})  {}\n",
                    all.len(),
                    if master { red("[MASTER: DISABLED]") } else { green("[MASTER: ENABLED]") }
                ))
            );
            for name in &all {
                let off = master || disabled.contains(name);
                let mark = if off { no_mark() } else { ok_mark() };
                let label = if off { dim(name) } else { name.clone() };
                println!("  {mark} {label}");
            }
            println!("{}", dim("\nToggle: helpers tool disable <name> | helpers tool enable <name> | helpers tool enable all"));
        }
        "enable" | "disable" => {
            let Some(target) = args.get(1) else {
                die(&format!("usage: helpers tool {sub} <name|all>"));
            };
            let all = list_all_tools();
            if target == "all" {
                disabled = if sub == "disable" { all.clone() } else { Vec::new() };
                write_config(cfg_disabled(&cfg), &disabled);
                println!(
                    "{} {} all {} tools.",
                    ok_mark(),
                    if sub == "disable" { "Disabled" } else { "Enabled" },
                    all.len()
                );
                return;
            }
            if !all.is_empty() && !all.contains(target) {
                println!("{}", yellow(&format!("Warning: '{target}' is not a known Helpers tool. Known: {}", all.join(", "))));
            }
            if sub == "disable" {
                if !disabled.contains(target) {
                    disabled.push(target.clone());
                }
            } else {
                disabled.retain(|t| t != target);
            }
            write_config(cfg_disabled(&cfg), &disabled);
            println!("{} Tool '{target}' {}.", ok_mark(), if sub == "disable" { "disabled" } else { "enabled" });
            println!("{}", dim("Takes effect live."));
        }
        "reset" => {
            write_config(cfg_disabled(&cfg), &[]);
            println!("{} Cleared per-tool disables.", ok_mark());
        }
        other => die(&format!("unknown 'helpers tool' subcommand: {other}")),
    }
}

// ── grade / index / setup (in-process, no subprocess) ────────────────────────
fn cmd_grade(args: &[String]) -> ExitCode {
    gitcli::cs_grade::run(args)
}

fn cmd_index(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("build");
    let (tool, tool_args): (&str, Value) = match sub {
        "build" => ("index_project", json!({})),
        "map" => ("project_map", json!({})),
        "lookup" | "where" => {
            let q = args.get(1..).map(|a| a.join(" ")).unwrap_or_default();
            if q.trim().is_empty() {
                die("usage: helpers index lookup <symbol-or-file>");
            }
            ("lookup", json!({ "query": q.trim() }))
        }
        other => die(&format!("unknown 'helpers index' subcommand: {other}")),
    };
    print_tool(tool, &tool_args)
}

fn cmd_setup(_args: &[String]) -> ExitCode {
    print_tool("project_setup", &json!({}))
}

/// Run a native tool in-process and print its text content.
fn print_tool(name: &str, args: &Value) -> ExitCode {
    match registry::dispatch(name, args) {
        Some(Ok(content)) => {
            for c in content {
                println!("{}", c.text);
            }
            ExitCode::SUCCESS
        }
        Some(Err(e)) => {
            eprintln!("{} {e}", red("helpers:"));
            ExitCode::from(1)
        }
        None => {
            eprintln!("{} unknown native tool: {name}", red("helpers:"));
            ExitCode::from(2)
        }
    }
}

// ── build / provision (symlinks; source build is dev-only) ────────────────────
/// `helpers build`: (re)create the busybox symlinks to this binary and verify.
/// The binary itself is already present (it's running / was downloaded), so there
/// is nothing to download here — installs fetch the prebuilt binary directly.
fn cmd_build(args: &[String]) -> ExitCode {
    let from_source = args.iter().any(|a| a == "--from-source");
    if from_source {
        return build_from_source();
    }
    match create_symlinks() {
        Ok(failures) => {
            if failures > 0 {
                println!(
                    "{}",
                    dim(&format!("      ({failures}/{} symlinks not created — optional; MCP tools unaffected)", SYMLINK_NAMES.len()))
                );
            }
            let count = registry::schemas().len();
            println!("{} native tools ready (helpers-native) — {count} tool(s) + web search, ~1ms startup", ok_mark());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{} could not finalize install: {e}", red("helpers:"));
            ExitCode::from(1)
        }
    }
}

/// Create `helpers`/`git-*` symlinks (relative) to the binary. Returns the count
/// of failures (non-fatal — e.g. Windows symlinks need elevation).
fn create_symlinks() -> std::io::Result<usize> {
    let dir = repo_dir();
    let target = format!("helpers-native{EXE}");
    let mut failures = 0;
    for name in SYMLINK_NAMES {
        let link = dir.join(format!("{name}{EXE}"));
        let _ = std::fs::remove_file(&link);
        if symlink(&target, &link).is_err() {
            failures += 1;
        }
    }
    Ok(failures)
}

#[cfg(unix)]
fn symlink(target: &str, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}
#[cfg(windows)]
fn symlink(target: &str, link: &Path) -> std::io::Result<()> {
    // Fall back to a copy when symlink creation needs elevation.
    let dir = link.parent().unwrap_or(Path::new("."));
    match std::os::windows::fs::symlink_file(target, link) {
        Ok(()) => Ok(()),
        Err(_) => std::fs::copy(dir.join(target), link).map(|_| ()),
    }
}

/// `helpers build --from-source`: compile the crate with cargo (dev fallback).
fn build_from_source() -> ExitCode {
    let crate_dir = repo_dir().join("native");
    if !crate_dir.exists() {
        eprintln!("{} native/ crate not found next to the binary — source build unavailable.", red("helpers:"));
        return ExitCode::from(1);
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .args(["build", "--release"])
        .current_dir(&crate_dir)
        .status();
    match status {
        Ok(s) if s.success() => {
            let _ = create_symlinks();
            println!("{} compiled helpers-native from source.", ok_mark());
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{} source build failed (need Rust: https://rustup.rs).", red("helpers:"));
            ExitCode::from(1)
        }
    }
}

// ── update ───────────────────────────────────────────────────────────────────
fn cmd_update(args: &[String]) {
    let check_only = args.iter().any(|a| a == "--check");
    let Some(latest) = latest_release_version() else {
        if !check_only {
            die("could not reach GitHub to check for updates (network?).");
        }
        return;
    };
    write_update_cache(&latest);
    if cmp_version(&latest, VERSION.trim()) <= 0 {
        if !check_only {
            println!("{} Helpers is up to date (v{}).", ok_mark(), VERSION.trim());
        }
        return;
    }
    if check_only {
        println!(
            "{} Update available: v{} → {}{}",
            yellow("↑"), VERSION.trim(), bold(&format!("v{latest}")), dim(" — run `helpers update`")
        );
        return;
    }
    apply_update(&latest);
}

/// Download the prebuilt binary for this platform from the latest release and
/// swap it in, then re-register. (Replaces the Node tarball/git-pull dance.)
fn apply_update(latest: &str) {
    println!("{} Updating Helpers → {}", yellow("↑"), bold(&format!("v{latest}")));
    let Some(tag) = host_target_tag() else {
        die("no prebuilt for this platform; rebuild from source (`helpers build --from-source`).");
    };
    let tmp = std::env::temp_dir().join(format!("helpers-update-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let tarball = tmp.join("bin.tar.gz");
    let urls = [
        format!("https://github.com/{REPO_SLUG}/releases/download/v{latest}/helpers-native-{tag}.tar.gz"),
        format!("https://github.com/{REPO_SLUG}/releases/latest/download/helpers-native-{tag}.tar.gz"),
    ];
    let mut got = false;
    for url in &urls {
        if curl_download(url, &tarball) {
            got = true;
            break;
        }
    }
    if !got {
        die("could not download the update for this platform.");
    }
    let extract = Command::new("tar")
        .args(["-xf", "bin.tar.gz"])
        .current_dir(&tmp)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !extract {
        die("could not extract the update.");
    }
    let src = tmp.join(format!("helpers-native{EXE}"));
    let dst = native_bin();
    if std::fs::copy(&src, &dst).is_err() {
        // The running binary may be busy; write alongside and swap.
        let staged = dst.with_extension("new");
        if std::fs::copy(&src, &staged).and_then(|_| std::fs::rename(&staged, &dst)).is_err() {
            die(&format!("could not replace {} — check permissions.", dst.display()));
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = create_symlinks();
    write_update_cache(latest);
    println!("{}", green(&bold(&format!("\nUpdated to v{latest}."))) + &dim(" Restart your agent (or /mcp reconnect) to load it."));
}

// ── doctor ───────────────────────────────────────────────────────────────────
fn cmd_doctor() {
    println!("{}", bold("\nHelpers doctor\n"));
    let mut problems = 0;
    let mut check = |label: &str, pass: bool, hint: &str| {
        println!("  {} {label}", if pass { ok_mark() } else { no_mark() });
        if !pass && !hint.is_empty() {
            println!("{}", dim(&format!("      → {hint}")));
            problems += 1;
        }
    };
    let bin = native_bin();
    check("native binary present", bin.exists(), "reinstall Helpers");
    let count = registry::schemas().len();
    check(&format!("native MCP tools available ({count})"), count > 0, "binary is broken — reinstall");
    check("tools.json present", tools_config_path().exists(), "run `helpers enable` to create it");
    let helpers_link = repo_dir().join("helpers");
    check("helpers symlink present", helpers_link.exists(), "run `helpers build`");
    if let Some(reg) = claude_mcp_registered() {
        check("helpers registered with Claude Code", reg, "run `helpers install --agent claude`");
    }
    // Toolchain is informational only (prebuilt is the default).
    let has_cargo = has_cmd("cargo");
    println!(
        "  {} Rust toolchain (cargo) — only for `helpers build --from-source`",
        if has_cargo { ok_mark() } else { dim("·") }
    );
    println!(
        "{}",
        if problems == 0 { green("\n  All checks passed.\n") } else { yellow(&format!("\n  {problems} issue(s) found.\n")) }
    );
}

// ── help ─────────────────────────────────────────────────────────────────────
fn help() {
    println!(
        "{}",
        bold("\nhelpers — control the Helpers MCP tools (Node-free, single binary)\n")
    );
    let lines = [
        "Usage: helpers <command> [args]",
        "",
        "  status                 Install state, master switch, tool counts, agents.",
        "  doctor                 Health checks.",
        "  install [--agent auto|claude|copilot|all]",
        "                         Register the MCP server + write agent guidance.",
        "  uninstall [--agent claude|copilot|all]",
        "  enable | disable | bypass [on|off]   Master switch (live).",
        "  tool list | tool {enable,disable} <name|all> | tool reset",
        "  build [--from-source]  (Re)create the helpers/git-* symlinks (or compile).",
        "  update [--check]       Download the latest prebuilt binary for this platform.",
        "  grade <path> [--json] Grade a CS project (course auto-detected).",
        "  index build|map|lookup <q>              Project index.",
        "  setup                  Deterministic project build-out plan.",
        "",
        "  The MCP server is `helpers <…>`-free: agents run `helpers-native mcp`.",
    ];
    for l in lines {
        println!("{l}");
    }
}

// ── version / update cache ───────────────────────────────────────────────────
fn cmp_version(a: &str, b: &str) -> i32 {
    let parse = |v: &str| -> Vec<i64> {
        v.trim_start_matches('v')
            .split('-')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|n| n.parse::<i64>().unwrap_or(0))
            .collect()
    };
    let (x, y) = (parse(a), parse(b));
    for i in 0..x.len().max(y.len()) {
        let d = x.get(i).copied().unwrap_or(0) - y.get(i).copied().unwrap_or(0);
        if d != 0 {
            return if d > 0 { 1 } else { -1 };
        }
    }
    0
}

/// Highest published release version via the GitHub API (token-authenticated when
/// available to dodge the unauthenticated rate limit). None when unreachable.
fn latest_release_version() -> Option<String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "-H", "Accept: application/vnd.github+json"]);
    if let Some(tok) = std::env::var("GITHUB_TOKEN").ok().or_else(|| std::env::var("GH_TOKEN").ok()) {
        if !tok.is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {tok}")]);
        }
    }
    cmd.arg(format!("https://api.github.com/repos/{REPO_SLUG}/releases?per_page=30"));
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let releases: Value = serde_json::from_slice(&out.stdout).ok()?;
    let mut best: Option<String> = None;
    for rel in releases.as_array()? {
        if rel.get("draft").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let Some(tag) = rel.get("tag_name").and_then(Value::as_str) else { continue };
        let v = tag.trim_start_matches('v').to_string();
        if best.as_deref().map(|b| cmp_version(&v, b) > 0).unwrap_or(true) {
            best = Some(v);
        }
    }
    best
}

fn update_cache_path() -> PathBuf {
    home().join(".config").join("helpers").join("update-check.json")
}
fn write_update_cache(latest: &str) {
    let path = update_cache_path();
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let _ = std::fs::write(path, json!({ "checkedAt": now, "latest": latest }).to_string());
}
/// A cached newer version (for the status hint), if the last check found one.
fn cached_newer_version() -> Option<String> {
    let raw = std::fs::read_to_string(update_cache_path()).ok()?;
    let cache: Value = serde_json::from_str(&raw).ok()?;
    let latest = cache.get("latest").and_then(Value::as_str)?;
    if cmp_version(latest, VERSION.trim()) > 0 {
        Some(latest.to_string())
    } else {
        None
    }
}

// ── shared helpers used by install too ───────────────────────────────────────
/// The prebuilt-binary target tag for this host (compile-time — the binary knows
/// its own target), matching the release asset names. None when unsupported.
pub fn host_target_tag() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        Some("macos-universal")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("windows-x86_64")
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        Some("windows-arm64")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64", target_env = "musl")) {
        Some("linux-x86_64-musl")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("linux-x86_64")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64", target_env = "musl")) {
        Some("linux-aarch64-musl")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("linux-aarch64")
    } else {
        None
    }
}

fn curl_download(url: &str, dest: &Path) -> bool {
    Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && dest.exists()
}

// ── install / uninstall ──────────────────────────────────────────────────────
const BLOCK_START: &str = "<!-- Helpers:BEGIN (managed by `helpers install`; do not edit) -->";
const BLOCK_END: &str = "<!-- Helpers:END -->";

/// Detect which agents to install for.
fn detect_agents() -> Vec<&'static str> {
    let mut a = Vec::new();
    if has_cmd("claude") || claude_dir().exists() {
        a.push("claude");
    }
    if copilot_dir().exists() || has_cmd("code") {
        a.push("copilot");
    }
    a
}

fn cmd_install(args: &[String]) {
    let mut agent = "auto".to_string();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--agent" {
            i += 1;
            agent = args.get(i).cloned().unwrap_or_else(|| "auto".into());
        }
        i += 1;
    }
    let targets: Vec<&str> = match agent.as_str() {
        "all" => vec!["claude", "copilot"],
        "auto" => {
            let d = detect_agents();
            if d.is_empty() {
                die("no AI agent detected. Use --agent claude|copilot|all to force.");
            }
            println!("{}", dim(&format!("Detected: {}", d.join(", "))));
            d
        }
        a => vec![Box::leak(a.to_string().into_boxed_str()) as &str],
    };
    prune_legacy_agent_config();
    for t in &targets {
        match *t {
            "claude" => install_claude(),
            "copilot" => install_copilot(),
            other => die(&format!("unknown agent '{other}'")),
        }
    }
    // Ensure tools.json exists in a known-good state.
    if !tools_config_path().exists() {
        let cfg = read_config();
        write_config(cfg_disabled(&cfg), &cfg_disabled_tools(&cfg));
    }
    println!(
        "{}{}",
        green(&bold("\nHelpers installed.")),
        dim(" Run `helpers status` to verify. Restart your agent (or /mcp reconnect).")
    );
}

fn install_claude() {
    println!("{}", bold("\n→ Installing Helpers for Claude Code"));
    let _ = create_symlinks();
    let bin = native_bin();

    // 1. Register the MCP server (Node-free: the binary speaks MCP directly).
    if has_cmd("claude") {
        let _ = Command::new("claude")
            .args(["mcp", "remove", "-s", "user", "helpers"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let added = Command::new("claude")
            .args(["mcp", "add", "-s", "user", "helpers", "--"])
            .arg(&bin)
            .arg("mcp")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if added {
            println!("  {} MCP server 'helpers' registered (user scope) — native, no Node", ok_mark());
        } else {
            println!("  {} MCP registration failed; add manually: claude mcp add -s user helpers -- {} mcp", no_mark(), bin.display());
        }
    } else {
        println!("  {} 'claude' CLI not found — add MCP manually:", yellow("!"));
        println!("{}", dim(&format!("      claude mcp add -s user helpers -- {} mcp", bin.display())));
    }

    // 2. CLAUDE.md managed block (from embedded config).
    if let Some(body) = embed::file_text(&embed::CLAUDE_CONFIG, "CLAUDE.helpers.md") {
        write_managed_block(&claude_dir().join("CLAUDE.md"), body.trim());
        println!("  {} Helpers core written to ~/.claude/CLAUDE.md (managed block)", ok_mark());
    }

    // 3. Skills, commands, agents (from embedded config).
    for kind in ["skills", "commands", "agents"] {
        if let Some(sub) = embed::CLAUDE_CONFIG.get_dir(kind) {
            if embed::extract_dir(sub, &claude_dir().join(kind)).is_ok() {
                println!("  {} {kind} installed to ~/.claude/{kind}/", ok_mark());
            }
        }
    }
}

fn install_copilot() {
    println!("{}", bold("\n→ Installing Helpers for GitHub Copilot"));
    for kind in ["instructions", "agents", "skills"] {
        if let Some(sub) = embed::COPILOT_CONFIG.get_dir(kind) {
            if embed::extract_dir(sub, &copilot_dir().join(kind)).is_ok() {
                println!("  {} {kind} installed to ~/.copilot/{kind}/", ok_mark());
            }
        }
    }
    println!("{}", dim("  Reload VS Code (or restart Copilot) to pick up the guidance."));
}

fn cmd_uninstall(args: &[String]) {
    let agent = if args.first().map(String::as_str) == Some("--agent") {
        args.get(1).map(String::as_str).unwrap_or("claude")
    } else {
        "claude"
    };
    if agent == "claude" || agent == "all" {
        if has_cmd("claude") {
            let _ = Command::new("claude")
                .args(["mcp", "remove", "-s", "user", "helpers"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        remove_managed_block(&claude_dir().join("CLAUDE.md"));
        println!("{} Removed helpers MCP server and CLAUDE.md block (skills/commands/agents left in place).", ok_mark());
    }
    if agent == "copilot" || agent == "all" {
        println!("{}", dim("Copilot config left in place; remove ~/.copilot/{agents,instructions,skills} manually if desired."));
    }
}

/// Remove pre-rebrand GSH config artifacts (incl. dangling symlinks).
fn prune_legacy_agent_config() {
    let legacy = [
        claude_dir().join("commands").join("gsh.md"),
        claude_dir().join("skills").join("gsh"),
        copilot_dir().join("instructions").join("gsh-routing-index.instructions.md"),
        copilot_dir().join("instructions").join("gsh-mcp-tools.instructions.md"),
        copilot_dir().join("skills").join("gsh"),
    ];
    let mut removed = 0;
    for p in &legacy {
        let Ok(meta) = std::fs::symlink_metadata(p) else { continue };
        let res = if meta.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        };
        if res.is_ok() {
            removed += 1;
        }
    }
    if removed > 0 {
        println!("{}", dim(&format!("  Removed {removed} legacy GSH config file(s).")));
    }
}

/// Write a delimited managed block into `file`, replacing an existing one without
/// clobbering the user's other content.
fn write_managed_block(file: &Path, body: &str) {
    let existing = std::fs::read_to_string(file).unwrap_or_default();
    let block = format!("{BLOCK_START}\n{body}\n{BLOCK_END}");
    let next = if let (Some(s), Some(e)) = (existing.find(BLOCK_START), existing.find(BLOCK_END)) {
        let end = e + BLOCK_END.len();
        if s < end {
            format!("{}{}{}", &existing[..s], block, &existing[end..])
        } else {
            format!("{}\n\n{block}\n", existing.trim_end())
        }
    } else if existing.trim().is_empty() {
        format!("{block}\n")
    } else {
        format!("{}\n\n{block}\n", existing.trim_end())
    };
    if let Some(p) = file.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::fs::write(file, next);
}

fn remove_managed_block(file: &Path) {
    let Ok(existing) = std::fs::read_to_string(file) else { return };
    if let (Some(s), Some(e)) = (existing.find(BLOCK_START), existing.find(BLOCK_END)) {
        let end = e + BLOCK_END.len();
        if s < end {
            let mut out = String::new();
            out.push_str(existing[..s].trim_end());
            out.push('\n');
            out.push_str(existing[end..].trim_start());
            let _ = std::fs::write(file, out);
        }
    }
}
