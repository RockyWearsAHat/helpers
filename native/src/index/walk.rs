//! Gitignore-aware repository walk — one fused pass (LINTER.md, "The live path").
//!
//! The walk IS the stat pass: each entry's `(mtime, len)` change witness comes from the
//! same directory scan that discovers it, so callers never re-stat what the walk already
//! touched. On macOS a single `getattrlistbulk` syscall returns name, type, mtime, and
//! size for a whole directory's entries at once — no per-file `stat` at all; other
//! platforms fall back to `read_dir` + `lstat`. Recursion fans out on the shared rayon
//! pool (no per-call thread spawning, no shared output mutex — results fold up the
//! recursion), and ignore rules match against a per-directory chain of compiled matchers
//! (`.gitignore` + `.ignore` per directory, ancestors up to the git root,
//! `.git/info/exclude`, then the global excludes file), deepest-first — the same decision
//! git makes, built once per directory instead of once per entry.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::Match;

/// A file found during the walk.
pub struct WalkedFile {
    /// Repo-relative path with forward slashes.
    pub rel: String,
    pub abs: PathBuf,
    /// Lowercase extension (no dot), or empty.
    pub ext: String,
    /// Modification time in UNIX nanoseconds (see [`ScanEntry::mtime`]).
    pub mtime: u128,
    /// `(mtime, len)` folded into one value — the verdict cache's change witness, computed
    /// here so the lint run never stats a file the walk already saw.
    pub state: u128,
}

/// One entry from a single batched directory scan — the walk's raw material, shared with
/// the whole-project replay's witness folds (LINTER.md, "An unchanged project replays the
/// whole report").
pub struct ScanEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    /// Byte length (0 for directories on the batched path — never used for them).
    pub len: u64,
    /// Modification time in UNIX nanoseconds — the replay's racy-window input: a witness
    /// is only trusted once every input is provably older than the moment it was stored.
    pub mtime: u128,
    /// `(mtime, len)` folded — bit-identical to the fold over `lstat` results that
    /// historical verdict caches hold, so they stay valid.
    pub state: u128,
}

/// Directories we never index even when not gitignored (build output, vcs, our own index, and —
/// crucially — dependency trees, which are NOT the project's own code and would swamp a review with
/// thousands of third-party findings). Shared so every walker (index + review) prunes identically.
pub const SKIP_DIRS: &[&str] = &[
    ".git",
    ".helpers",
    ".claude",
    // dependency trees (JS / Python / general)
    "node_modules",
    "vendor",
    ".venv",
    "venv",
    "env",
    ".env",
    "site-packages",
    "bower_components",
    "Pods",
    // build / generated output
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    // caches / tooling
    "__pycache__",
    ".cache",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".gradle",
    "coverage",
    ".idea",
];

/// Skip files larger than this — they are almost never source worth indexing.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Fold `(mtime nanos, len)` into the one-word change witness the verdict cache stores.
/// Must stay bit-identical to what historical caches hold, or every warm project re-lints.
fn fold_parts(mtime_nanos: u128, len: u64) -> u128 {
    mtime_nanos ^ ((len as u128) << 64)
}

/// Scan one directory in a single batched syscall where the platform has one, else
/// `read_dir` + per-entry `lstat`. Symlinks and special files report neither `is_dir` nor
/// `is_file` and are skipped by every caller — the same semantics the reference walker had.
pub fn scan_dir(dir: &Path) -> Vec<ScanEntry> {
    #[cfg(target_os = "macos")]
    if let Some(entries) = bulk::scan(dir) {
        return entries;
    }
    generic_scan(dir)
}

/// The portable scan: `read_dir` for names/types, `lstat` per entry for the state.
fn generic_scan(dir: &Path) -> Vec<ScanEntry> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let ft = entry.file_type().ok()?;
            let (mtime, len) = entry
                .metadata()
                .map(|m| {
                    let t = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
                    (t.map(|d| d.as_nanos()).unwrap_or(0), m.len())
                })
                .unwrap_or((0, 0));
            Some(ScanEntry {
                name,
                is_dir: ft.is_dir(),
                is_file: ft.is_file(),
                len,
                mtime,
                state: fold_parts(mtime, len),
            })
        })
        .collect()
}

/// The `getattrlistbulk` fast path: one open + a few syscalls per directory return every
/// entry's name, object type, modification time, and data length together.
#[cfg(target_os = "macos")]
mod bulk {
    use super::{fold_parts, ScanEntry};
    use std::ffi::c_void;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    /// `struct attrlist` (sys/attr.h).
    #[repr(C)]
    struct AttrList {
        bitmapcount: u16,
        reserved: u16,
        commonattr: u32,
        volattr: u32,
        dirattr: u32,
        fileattr: u32,
        forkattr: u32,
    }

    extern "C" {
        fn open(path: *const u8, oflag: i32) -> i32;
        fn close(fd: i32) -> i32;
        fn getattrlistbulk(
            dirfd: i32,
            alist: *mut AttrList,
            attrbuf: *mut c_void,
            bufsize: usize,
            options: u64,
        ) -> i32;
    }

    const O_RDONLY: i32 = 0x0000;
    const O_DIRECTORY: i32 = 0x0010_0000;
    const O_CLOEXEC: i32 = 0x0100_0000;

    const ATTR_BIT_MAP_COUNT: u16 = 5;
    const ATTR_CMN_NAME: u32 = 0x0000_0001;
    const ATTR_CMN_OBJTYPE: u32 = 0x0000_0008;
    const ATTR_CMN_MODTIME: u32 = 0x0000_0400;
    const ATTR_CMN_RETURNED_ATTRS: u32 = 0x8000_0000;
    const ATTR_FILE_DATALENGTH: u32 = 0x0000_0200;
    /// `fsobj_type_t` values (enum vtype) — the two kinds the walk keeps.
    const VREG: u32 = 1;
    const VDIR: u32 = 2;

    /// Read one little value at a possibly 4-byte-aligned offset (bulk records pack on
    /// 4-byte boundaries, so 8-byte fields may be unaligned).
    unsafe fn read_at<T: Copy>(base: *const u8, off: usize) -> T {
        (base.add(off) as *const T).read_unaligned()
    }

    /// All entries of `dir`, or `None` when the directory cannot be scanned this way (the
    /// caller falls back to the portable scan).
    pub fn scan(dir: &Path) -> Option<Vec<ScanEntry>> {
        let mut cpath = dir.as_os_str().as_bytes().to_vec();
        cpath.push(0);
        let fd = unsafe { open(cpath.as_ptr(), O_RDONLY | O_DIRECTORY | O_CLOEXEC) };
        if fd < 0 {
            return None;
        }
        let mut alist = AttrList {
            bitmapcount: ATTR_BIT_MAP_COUNT,
            reserved: 0,
            commonattr: ATTR_CMN_RETURNED_ATTRS | ATTR_CMN_NAME | ATTR_CMN_OBJTYPE | ATTR_CMN_MODTIME,
            volattr: 0,
            dirattr: 0,
            fileattr: ATTR_FILE_DATALENGTH,
            forkattr: 0,
        };
        // One reusable buffer per thread: zeroing a fresh 128KB per directory was a
        // measured multi-ms slice of the whole walk (the kernel overwrites what it uses;
        // record parsing never reads past what the kernel wrote for the batch).
        thread_local! {
            static BUF: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![0u8; 128 * 1024]);
        }
        let mut out = Vec::new();
        let complete = BUF.with(|cell| {
            let mut buf = cell.borrow_mut();
            let buf = &mut *buf;
            loop {
                let n = unsafe {
                    getattrlistbulk(fd, &mut alist, buf.as_mut_ptr() as *mut c_void, buf.len(), 0)
                };
                if n < 0 {
                    return false;
                }
                if n == 0 {
                    return true;
                }
                let mut rec: *const u8 = buf.as_ptr();
                for _ in 0..n {
                    let len: u32 = unsafe { read_at(rec, 0) };
                    if len < 4 {
                        // A malformed record cannot advance the cursor — abandon the batch
                        // rather than spin; the caller falls back to the portable scan.
                        return false;
                    }
                    // Field cursor starts after the record length; the returned-attributes
                    // bitmap is always first, then requested attrs in bit order.
                    let mut off = 4usize;
                    let ret_common: u32 = unsafe { read_at(rec, off) };
                    let ret_file: u32 = unsafe { read_at(rec, off + 12) };
                    off += 20;
                    let mut name = String::new();
                    if ret_common & ATTR_CMN_NAME != 0 {
                        let data_off: i32 = unsafe { read_at(rec, off) };
                        let data_len: u32 = unsafe { read_at(rec, off + 4) };
                        let start = off.wrapping_add(data_off as usize);
                        let bytes = unsafe {
                            std::slice::from_raw_parts(
                                rec.add(start),
                                (data_len as usize).saturating_sub(1), // trailing NUL
                            )
                        };
                        name = String::from_utf8_lossy(bytes).into_owned();
                        off += 8;
                    }
                    let mut objtype = 0u32;
                    if ret_common & ATTR_CMN_OBJTYPE != 0 {
                        objtype = unsafe { read_at(rec, off) };
                        off += 4;
                    }
                    let mut mtime_nanos = 0u128;
                    if ret_common & ATTR_CMN_MODTIME != 0 {
                        let sec: i64 = unsafe { read_at(rec, off) };
                        let nsec: i64 = unsafe { read_at(rec, off + 8) };
                        mtime_nanos = (sec.max(0) as u128) * 1_000_000_000 + nsec.max(0) as u128;
                        off += 16;
                    }
                    let mut flen = 0u64;
                    if ret_file & ATTR_FILE_DATALENGTH != 0 {
                        let l: i64 = unsafe { read_at(rec, off) };
                        flen = l.max(0) as u64;
                    }
                    if !name.is_empty() {
                        out.push(ScanEntry {
                            name,
                            is_dir: objtype == VDIR,
                            is_file: objtype == VREG,
                            len: flen,
                            mtime: mtime_nanos,
                            state: fold_parts(mtime_nanos, flen),
                        });
                    }
                    rec = unsafe { rec.add(len as usize) };
                }
            }
        });
        unsafe { close(fd) };
        complete.then_some(out)
    }
}

/// One directory's compiled ignore rules, chained to its parent's. Matching walks the chain
/// deepest-first and the first verdict wins — a deeper `.gitignore` overrides a shallower
/// one in either polarity, which is git's own precedence. `fp` folds the chain's rule
/// files' `(mtime, len)` states — the validity witness for cached per-directory decisions.
struct IgnoreNode {
    matcher: Arc<Gitignore>,
    parent: Option<Arc<IgnoreNode>>,
    fp: u128,
}

/// The chain fingerprint carried by `chain` (1 for the empty chain).
fn chain_fp(chain: &Option<Arc<IgnoreNode>>) -> u128 {
    chain.as_ref().map(|n| n.fp).unwrap_or(1)
}

/// A rule file's `(mtime, len)` state for the chain fingerprint (0 when absent).
fn rule_file_state(p: &Path) -> u128 {
    std::fs::metadata(p)
        .map(|m| {
            let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
            fold_parts(t.map(|d| d.as_nanos()).unwrap_or(0), m.len())
        })
        .unwrap_or(0)
}

/// Compile a directory's `.gitignore` + `.ignore` (the `.ignore` file wins within the
/// directory, matching the `ignore` crate). Reads the files' contents.
fn compile_dir_rules(dir: &Path, has_gitignore: bool, has_dotignore: bool) -> Option<Gitignore> {
    let mut b = GitignoreBuilder::new(dir);
    if has_gitignore {
        b.add(dir.join(".gitignore"));
    }
    if has_dotignore {
        b.add(dir.join(".ignore"));
    }
    b.build().ok()
}

/// Extend `parent` with a directory's own rules, driven by the SCAN's knowledge (presence
/// and `(mtime, len)` states of `.gitignore`/`.ignore`) so no extra syscall runs, and with
/// the compiled matcher cached per directory keyed by those states — rule files recompile
/// only when they actually change.
fn extend_chain(
    dir: &Path,
    parent: &Option<Arc<IgnoreNode>>,
    gitignore_state: u128,
    dotignore_state: u128,
) -> Option<Arc<IgnoreNode>> {
    if gitignore_state == 0 && dotignore_state == 0 {
        return None;
    }
    type MatcherCache = std::collections::HashMap<PathBuf, (u128, Arc<Gitignore>)>;
    static CACHE: std::sync::OnceLock<std::sync::Mutex<MatcherCache>> = std::sync::OnceLock::new();
    let key = gitignore_state ^ dotignore_state.rotate_left(31);
    let cached = {
        let cache = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
        cache.get(dir).filter(|(k, _)| *k == key).map(|(_, m)| m.clone())
    };
    let matcher = match cached {
        Some(m) => m,
        None => {
            let m = Arc::new(compile_dir_rules(dir, gitignore_state != 0, dotignore_state != 0)?);
            CACHE
                .get_or_init(Default::default)
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(dir.to_path_buf(), (key, m.clone()));
            m
        }
    };
    let fp = chain_fp(parent).rotate_left(17) ^ key;
    Some(Arc::new(IgnoreNode { matcher, parent: parent.clone(), fp }))
}

/// Whether `path` is excluded by the chain — first verdict deepest-first, unmatched ⇒ kept.
fn is_ignored(chain: &Option<Arc<IgnoreNode>>, path: &Path, is_dir: bool) -> bool {
    let mut cur = chain;
    while let Some(node) = cur {
        match node.matcher.matched(path, is_dir) {
            Match::Ignore(_) => return true,
            Match::Whitelist(_) => return false,
            Match::None => {}
        }
        cur = &node.parent;
    }
    false
}

/// The rules in force ABOVE the root: the global excludes file, the enclosing repo's
/// `.git/info/exclude`, and every ancestor directory's `.gitignore`/`.ignore` from the git
/// root down to the root's parent — so walking a subdirectory honors the same law as
/// walking the repo.
fn root_chain(root: &Path) -> Option<Arc<IgnoreNode>> {
    let mut chain: Option<Arc<IgnoreNode>> = None;
    let (global, _err) = Gitignore::global();
    if global.num_ignores() + global.num_whitelists() > 0 {
        let fp = rule_file_state(global.path());
        chain = Some(Arc::new(IgnoreNode { matcher: Arc::new(global), parent: None, fp }));
    }
    // Ancestors, git root first (shallowest), so the fold below leaves the root's own
    // parent deepest in the chain.
    let mut ancestors: Vec<&Path> = Vec::new();
    let mut cur = root.parent();
    let mut git_root: Option<&Path> = if root.join(".git").exists() { Some(root) } else { None };
    while let Some(dir) = cur {
        ancestors.push(dir);
        if dir.join(".git").exists() {
            git_root = Some(dir);
            break;
        }
        cur = dir.parent();
    }
    // Outside a git repo, ancestor gitignores do not apply (matches `ignore`'s walker).
    if let Some(git_root) = git_root {
        let exclude = git_root.join(".git/info/exclude");
        if exclude.is_file() {
            let mut b = GitignoreBuilder::new(git_root);
            b.add(&exclude);
            if let Ok(matcher) = b.build() {
                let fp = chain_fp(&chain).rotate_left(17) ^ rule_file_state(&exclude);
                chain = Some(Arc::new(IgnoreNode { matcher: Arc::new(matcher), parent: chain, fp }));
            }
        }
        for dir in ancestors.iter().rev() {
            if dir.starts_with(git_root) {
                let gi = rule_file_state(&dir.join(".gitignore"));
                let di = rule_file_state(&dir.join(".ignore"));
                if let Some(extended) = extend_chain(dir, &chain, gi, di) {
                    chain = Some(extended);
                }
            }
        }
    }
    chain
}

/// One directory's remembered keep/skip decisions. Ignore verdicts are a pure function of
/// (entry names + kinds, the ignore chain's rule-file contents), so they are cached per
/// directory and revalidated every run by `names_fold` (names + kinds, order-sensitive — the
/// mask indexes the scan order) and `chain_fp` (the chain's rule files' states). Entry
/// STATES are never cached — they come fresh from every scan, which is the whole witness.
struct DirDecisions {
    names_fold: u128,
    chain_fp: u128,
    keep: Vec<bool>,
}

/// The process-wide decision cache, keyed by directory path.
fn decision_cache() -> &'static std::sync::Mutex<std::collections::HashMap<PathBuf, DirDecisions>> {
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<PathBuf, DirDecisions>>,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Names+kinds fold for decision revalidation — any rename, addition, removal, or
/// file↔dir kind flip changes it (and therefore recomputes the directory's decisions).
fn names_fold(entries: &[ScanEntry]) -> u128 {
    entries.iter().fold(1u128, |acc, e| {
        acc.rotate_left(9)
            ^ (crate::lint_ai::token_seed(&e.name) as u128)
            ^ ((e.is_dir as u128) << 127)
            ^ ((e.is_file as u128) << 126)
    })
}

/// Scan one directory: keep its files, recurse into its subdirectories on the rayon pool,
/// and fold the results upward — no shared collector, deterministic after the caller's
/// final sort.
fn walk_dir(dir: &Path, root: &Path, chain: Option<Arc<IgnoreNode>>) -> (Vec<WalkedFile>, Vec<PathBuf>) {
    let entries = scan_dir(dir);
    // The scan already knows whether this directory carries rule files (and their states) —
    // extending the chain costs no extra syscall.
    let (mut gi, mut di) = (0u128, 0u128);
    for e in &entries {
        if e.is_file {
            match e.name.as_str() {
                ".gitignore" => gi = e.state | 1,
                ".ignore" => di = e.state | 1,
                _ => {}
            }
        }
    }
    let chain = extend_chain(dir, &chain, gi, di).or(chain);
    let fold = names_fold(&entries);
    let fp = chain_fp(&chain);
    // Reuse remembered verdicts when both inputs are provably unchanged; otherwise match
    // for real and remember. Per-entry glob matching was the walk's largest slice.
    let keep: Vec<bool> = {
        let cache = decision_cache().lock().unwrap_or_else(|e| e.into_inner());
        match cache.get(dir) {
            Some(d) if d.names_fold == fold && d.chain_fp == fp => Some(d.keep.clone()),
            _ => None,
        }
    }
    .unwrap_or_else(|| {
        let keep: Vec<bool> = entries
            .iter()
            .map(|e| {
                if SKIP_DIRS.contains(&e.name.as_str()) {
                    return false;
                }
                if e.is_dir || e.is_file {
                    !is_ignored(&chain, &dir.join(&e.name), e.is_dir)
                } else {
                    false
                }
            })
            .collect();
        decision_cache()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(dir.to_path_buf(), DirDecisions { names_fold: fold, chain_fp: fp, keep: keep.clone() });
        keep
    });
    let mut files: Vec<WalkedFile> = Vec::new();
    let mut subdirs: Vec<PathBuf> = Vec::new();
    for (e, kept) in entries.iter().zip(&keep) {
        if !kept {
            continue;
        }
        let abs = dir.join(&e.name);
        if e.is_dir {
            subdirs.push(abs);
        } else if e.is_file {
            if e.len > MAX_FILE_BYTES {
                continue;
            }
            let rel = abs.strip_prefix(root).unwrap_or(&abs).to_string_lossy().replace('\\', "/");
            let ext = abs.extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
            files.push(WalkedFile { rel, abs, ext, mtime: e.mtime, state: e.state });
        }
    }
    use rayon::prelude::*;
    let nested: Vec<(Vec<WalkedFile>, Vec<PathBuf>)> =
        subdirs.par_iter().map(|d| walk_dir(d, root, chain.clone())).collect();
    let mut dirs: Vec<PathBuf> = subdirs;
    for (mut v, mut d) in nested {
        files.append(&mut v);
        dirs.append(&mut d);
    }
    (files, dirs)
}

/// Walk `root`, honoring `.gitignore`/`.ignore` law, returning indexable files sorted by
/// path — each already carrying its `(mtime, len)` state from the same scan.
pub fn walk_repo(root: &Path) -> Vec<WalkedFile> {
    walk_repo_full(root).0
}

/// [`walk_repo`] plus every kept DIRECTORY the walk visited (the root included) — the
/// kqueue tier's watch set needs the dirs so a file created in any of them posts an event
/// (LINTER.md, "The kqueue tier").
pub fn walk_repo_full(root: &Path) -> (Vec<WalkedFile>, Vec<PathBuf>) {
    let (mut files, mut dirs) = walk_dir(root, root, root_chain(root));
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    dirs.push(root.to_path_buf());
    dirs.sort();
    (files, dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The walk against the `ignore` crate's own walker on this repo: identical file sets —
    /// the fused walker changes the cost, never the law.
    #[test]
    fn fused_walk_matches_the_reference_walker() {
        let root = crate::git::workspace_root();
        if !root.join(".git").exists() {
            return; // contract only meaningful inside a repo
        }
        let ours: std::collections::BTreeSet<String> =
            walk_repo(&root).into_iter().map(|f| f.rel).collect();
        let reference: std::collections::BTreeSet<String> = ignore::WalkBuilder::new(&root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .parents(true)
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !SKIP_DIRS.contains(&name.as_ref())
            })
            .build()
            .flatten()
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .filter(|e| e.metadata().map(|m| m.len() <= MAX_FILE_BYTES).unwrap_or(false))
            .map(|e| {
                e.path()
                    .strip_prefix(&root)
                    .unwrap_or(e.path())
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        let missing: Vec<&String> = reference.difference(&ours).collect();
        let extra: Vec<&String> = ours.difference(&reference).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "walker diverged — missing {missing:?}, extra {extra:?}"
        );
    }

    /// The batched scan against the portable scan on a directory this test owns (a shared
    /// directory mutates under the parallel suite): same names, same types, same
    /// `(mtime, len)` states — the syscall changes, the witness cannot.
    #[test]
    fn batched_scan_matches_portable_scan() {
        let dir = std::env::temp_dir().join(format!("scan-parity-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.rs"), "fn a() {}").unwrap();
        std::fs::write(dir.join("b.txt"), "prose, longer than a").unwrap();
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(dir.join("a.rs"), dir.join("link.rs"));
        }
        let project = |mut v: Vec<ScanEntry>| -> Vec<(String, bool, bool, u64, u128, u128)> {
            v.sort_by(|a, b| a.name.cmp(&b.name));
            v.into_iter()
                .map(|e| (e.name, e.is_dir, e.is_file, e.len, e.mtime, e.state))
                .collect()
        };
        let ours = project(scan_dir(&dir));
        let portable = project(generic_scan(&dir));
        // Directory sizes differ between stat and bulk conventions; files must agree
        // exactly (their state is the verdict cache's replay witness), the symlink must be
        // neither file nor dir on both paths, and dir names must agree.
        let files = |v: &[(String, bool, bool, u64, u128, u128)]| -> Vec<_> {
            v.iter().filter(|e| e.2).cloned().collect::<Vec<_>>()
        };
        assert_eq!(files(&ours), files(&portable), "file entries diverged");
        assert_eq!(
            ours.iter().map(|e| (&e.0, e.1)).collect::<Vec<_>>(),
            portable.iter().map(|e| (&e.0, e.1)).collect::<Vec<_>>(),
            "names/kinds diverged"
        );
        assert!(
            ours.iter().any(|e| e.0 == "link.rs" && !e.1 && !e.2),
            "symlink must be neither file nor dir"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every walked file carries a live `(mtime, len)` state — the fused stat contract.
    #[test]
    fn walked_files_carry_their_state() {
        let root = crate::git::workspace_root();
        let files = walk_repo(&root);
        assert!(!files.is_empty());
        assert!(files.iter().all(|f| f.state != 0), "state must come fused from the walk");
    }
}
