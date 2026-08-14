//! Agent-config data embedded into the binary so `helpers install` is fully
//! self-contained — no source tree needed at install time. The trees are baked
//! in at compile time from the repo's `claude-config/` and `copilot-config/`.

use std::io;
use std::path::Path;

use include_dir::{include_dir, Dir};

/// The single, agent-agnostic always-on core (`CORE.md`) — one doc installed verbatim
/// for every agent, so the guidance is never duplicated or allowed to drift per-agent.
pub static AGENT_CONFIG: Dir = include_dir!("$CARGO_MANIFEST_DIR/../agent-config");

/// Claude-only assets layered on top of the shared core: `skills/`, `commands/`, `agents/`.
pub static CLAUDE_CONFIG: Dir = include_dir!("$CARGO_MANIFEST_DIR/../claude-config");

/// Copilot-only assets layered on top of the shared core: scoped `instructions/`, `agents/`, `skills/`.
pub static COPILOT_CONFIG: Dir = include_dir!("$CARGO_MANIFEST_DIR/../copilot-config");

/// Recursively write an embedded directory's contents into `dest` (created if
/// missing), overwriting existing files so reinstalls pick up guidance updates.
pub fn extract_dir(dir: &Dir, dest: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for file in dir.files() {
        let name = file
            .path()
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "embedded file has no name"))?;
        std::fs::write(dest.join(name), file.contents())?;
    }
    for sub in dir.dirs() {
        let name = sub
            .path()
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "embedded dir has no name"))?;
        extract_dir(sub, &dest.join(name))?;
    }
    Ok(())
}

/// Read a single embedded file's text by path relative to the tree root.
pub fn file_text(dir: &Dir, rel: &str) -> Option<String> {
    dir.get_file(rel)
        .and_then(|f| f.contents_utf8())
        .map(str::to_string)
}

/// The one agent-agnostic core body (`agent-config/CORE.md`), single source of truth for
/// Helpers' always-on guidance. Claude Code gets this live via the MCP `initialize`
/// response's `instructions` field ([`crate::mcp::run`]); Codex/Copilot, which have no
/// equivalent live channel, still get it written into their managed config files
/// ([`crate::cli::install_codex`], [`crate::cli::install_copilot`]).
pub fn agent_core_body() -> Option<String> {
    file_text(&AGENT_CONFIG, "CORE.md").map(|b| b.trim().to_string())
}
