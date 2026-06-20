//! helpers-native CLI — the binary the Node MCP daemon shells out to.
//!
//!   helpers-native schemas              print a JSON array of native tool schemas
//!   helpers-native call <tool>          read {args} JSON from stdin, run the tool,
//!                                   print {"content":[...]} or {"error":{...}}
//!   helpers-native bundle <root> <out>  export the project index as a .dxbundle
//!   helpers-native install <root> <b>   install a .dxbundle into <root>
//!   helpers-native refs <root>          list installed reference indexes
//!
//! Cold start is ~1ms (no V8), so the warm daemon pays only a trivial per-call
//! cost while getting native-speed execution for scan/index-heavy tools.

use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use helpers_native::cli;
use helpers_native::gitcli;
use helpers_native::index::bundle;
use helpers_native::mcp;
use helpers_native::proto::{emit_content, emit_error};
use helpers_native::registry;

fn main() -> ExitCode {
    // Busybox-style dispatch: when invoked through a `git-*` symlink, argv[0]'s
    // basename selects the ported CLI.
    if let Some(basename) = std::env::args().next().and_then(|a| {
        Path::new(&a)
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
    }) {
        if gitcli::is_cli(&basename) {
            let args: Vec<String> = std::env::args().skip(1).collect();
            return gitcli::dispatch(&basename, &args);
        }
        // Invoked as the `helpers` control CLI (a symlink to this binary).
        if basename == "helpers" || basename == "helpers.exe" {
            let args: Vec<String> = std::env::args().skip(1).collect();
            return cli::run(&args);
        }
    }

    let argv: Vec<String> = std::env::args().skip(1).collect();
    match argv.first().map(String::as_str) {
        // Explicit form: `helpers-native cli <args…>`.
        Some("cli") => cli::run(&argv[1..]),
        // Explicit form: `helpers-native gitcli <name> [args…]`.
        Some("gitcli") => {
            let name = match argv.get(1) {
                Some(n) => n.clone(),
                None => {
                    eprintln!("usage: helpers-native gitcli <name> [args…]");
                    return ExitCode::from(2);
                }
            };
            gitcli::dispatch(&name, &argv[2..])
        }
        Some("schemas") => {
            let arr = registry::schemas();
            println!(
                "{}",
                serde_json::to_string(&arr).expect("serialize schemas")
            );
            ExitCode::SUCCESS
        }
        Some("mcp") => mcp::run(),
        Some("call") => run_call(argv.get(1).map(String::as_str)),
        Some("bundle") => run_bundle(argv.get(1), argv.get(2)),
        Some("install") => run_install(argv.get(1), argv.get(2)),
        Some("refs") => run_refs(argv.get(1)),
        Some(other) => {
            eprintln!("helpers-native: unknown command: {other}");
            ExitCode::from(2)
        }
        None => {
            eprintln!("usage: helpers-native <mcp | schemas | call <tool> | bundle | install | refs>");
            ExitCode::from(2)
        }
    }
}

fn run_bundle(root: Option<&String>, out: Option<&String>) -> ExitCode {
    let (Some(root), Some(out)) = (root, out) else {
        eprintln!("usage: helpers-native bundle <root> <out.dxbundle>");
        return ExitCode::from(2);
    };
    match bundle::export_bundle(Path::new(root), Path::new(out)) {
        Ok(b) => {
            println!(
                "Exported {} ({} files, {} symbols, {} docs) -> {out}",
                b.name,
                b.file_count,
                b.symbol_count,
                b.docs.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("helpers-native bundle: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_install(root: Option<&String>, bundle_path: Option<&String>) -> ExitCode {
    let (Some(root), Some(bundle_path)) = (root, bundle_path) else {
        eprintln!("usage: helpers-native install <root> <bundle.dxbundle>");
        return ExitCode::from(2);
    };
    match bundle::install_bundle(Path::new(root), Path::new(bundle_path)) {
        Ok(name) => {
            println!("Installed reference index '{name}' under .helpers/index/refs/{name}/");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("helpers-native install: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_refs(root: Option<&String>) -> ExitCode {
    let root = root.map(String::as_str).unwrap_or(".");
    let refs = bundle::list_refs(Path::new(root));
    if refs.is_empty() {
        println!("No reference indexes installed. Use `helpers index install <bundle>`.");
    } else {
        for r in refs {
            println!("{r}");
        }
    }
    ExitCode::SUCCESS
}

fn run_call(tool: Option<&str>) -> ExitCode {
    let name = match tool {
        Some(n) => n,
        None => {
            emit_error("missing tool name (usage: helpers-native call <tool>)");
            return ExitCode::from(2);
        }
    };

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        emit_error("failed to read tool arguments from stdin");
        return ExitCode::from(2);
    }
    let args: serde_json::Value = if input.trim().is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str(&input) {
            Ok(v) => v,
            Err(e) => {
                emit_error(&format!("invalid JSON arguments: {e}"));
                return ExitCode::from(2);
            }
        }
    };

    match registry::dispatch(name, &args) {
        Some(Ok(content)) => {
            emit_content(&content);
            ExitCode::SUCCESS
        }
        Some(Err(message)) => {
            emit_error(&message);
            ExitCode::from(1)
        }
        None => {
            emit_error(&format!("unknown native tool: {name}"));
            ExitCode::from(3)
        }
    }
}
