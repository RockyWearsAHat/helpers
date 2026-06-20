//! Agent-config data embedded into the binary so `helpers install` is fully
//! self-contained — no source tree needed at install time. The trees are baked
//! in at compile time from the repo's `claude-config/` and `copilot-config/`.

use std::io;
use std::path::Path;

use include_dir::{include_dir, Dir};

/// The Claude agent config: `CLAUDE.helpers.md`, `skills/`, `commands/`, `agents/`.
pub static CLAUDE_CONFIG: Dir = include_dir!("$CARGO_MANIFEST_DIR/../claude-config");

/// The Copilot agent config: `instructions/`, `agents/`, `skills/`.
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
