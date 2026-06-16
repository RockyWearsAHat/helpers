//! Lexical path helpers mirroring Node's `path.resolve` and `path.relative`
//! (purely syntactic: no symlink resolution, no filesystem access), so the
//! resolved root and the "Wrote …" line match the original output.

use std::path::{Component, Path, PathBuf};

/// `path.resolve(input)`: make `input` absolute against the current directory
/// and normalise `.`/`..` lexically.
pub fn resolve(input: &str) -> PathBuf {
    let raw = Path::new(input);
    let base = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(raw)
    };
    normalize(&base)
}

fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `path.relative(from, to)` for two absolute paths: the relative route from
/// `from` to `to`, using `/` separators. Returns "" when they are equal.
pub fn relative(from: &Path, to: &Path) -> String {
    let from_parts = normal_parts(from);
    let to_parts = normal_parts(to);

    let common = from_parts
        .iter()
        .zip(to_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let ups = from_parts.len() - common;
    let mut segments: Vec<String> = std::iter::repeat("..".to_string()).take(ups).collect();
    segments.extend(to_parts[common..].iter().cloned());
    segments.join("/")
}

/// The `Normal` path components as strings (drops root/prefix), giving a
/// comparable, separator-independent sequence.
fn normal_parts(p: &Path) -> Vec<String> {
    p.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn relative_basic() {
        assert_eq!(
            relative(Path::new("/a/b"), Path::new("/a/b/GRADE.md")),
            "GRADE.md"
        );
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/b")), "");
        assert_eq!(relative(Path::new("/a/b/c"), Path::new("/a/x")), "../../x");
    }
}
