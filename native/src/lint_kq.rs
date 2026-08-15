//! `lint_kq` — the kqueue replay tier (native/architecture.dx, "The kqueue tier").
//!
//! The stat sweep proves currency in milliseconds; this tier proves it in MICROSECONDS by
//! holding an `EVFILT_VNODE` watch (an `O_EVTONLY` fd) on every input and asking the kernel
//! "did anything fire?" with one zero-timeout `kevent` drain. The knote is enqueued inside
//! the mutating syscall itself — a `write` that has returned is visible to the very next
//! poll — so, unlike fseventsd (measured ~10ms ingestion lag, stale CLEAN served), this
//! witness has no daemon in the path and no delivery window.
//!
//! Soundness protocol (tools/lint.rs drives it): ARM the watch set first, then sweep the
//! tree again and require the walk fold unchanged (an edit racing the arming lands either
//! in the fold difference or as a pending event), then require a quiet drain, then commit
//! the body. Every failure mode — an event, an incomplete watch set, fd exhaustion, a
//! fresh process, another platform — falls back to the stat tier: slower, never stale.
//! Derived caches (`lint-verdicts/`, `lint-replay/`) are never watched, exactly as they
//! are excluded from the stat witness, so a run cannot invalidate its own memo.

use std::path::{Path, PathBuf};

/// Return the memoized body for `(root, key)` when the kernel reports the watch set quiet
/// since it was committed. One kevent drain + a map lookup — microseconds.
#[cfg(target_os = "macos")]
pub fn replay(root: &Path, key: &str) -> Option<String> {
    macos::replay(root, key)
}

#[cfg(target_os = "linux")]
pub fn replay(root: &Path, key: &str) -> Option<String> {
    linux::replay(root, key)
}

/// (Re)arm the watch set for `root` over exactly `paths`. Returns whether EVERY path is
/// watched (an incomplete set disables the tier until a later arm succeeds).
#[cfg(target_os = "macos")]
pub fn arm(root: &Path, paths: &[PathBuf]) -> bool {
    macos::arm(root, paths)
}

#[cfg(target_os = "linux")]
pub fn arm(root: &Path, paths: &[PathBuf]) -> bool {
    linux::arm(root, paths)
}

/// Whether the watch set is armed, complete, and has seen no event since the last drain —
/// the final gate before a commit.
#[cfg(target_os = "macos")]
pub fn confirm_quiet(root: &Path) -> bool {
    macos::confirm_quiet(root)
}

#[cfg(target_os = "linux")]
pub fn confirm_quiet(root: &Path) -> bool {
    linux::confirm_quiet(root)
}

/// Memoize `body` for `(root, key)` against the current quiet generation.
#[cfg(target_os = "macos")]
pub fn commit(root: &Path, key: &str, body: &str) {
    macos::commit(root, key, body)
}

#[cfg(target_os = "linux")]
pub fn commit(root: &Path, key: &str, body: &str) {
    linux::commit(root, key, body)
}

/// Reopen ONLY the fired vnodes against their unchanged paths (the incremental tier's
/// re-arm: membership is unchanged by construction — adds/removes fall back to the full
/// arm). Returns quiet-and-complete after the reopen, i.e. safe to commit.
#[cfg(target_os = "macos")]
pub fn rearm_fired(root: &Path) -> bool {
    macos::rearm_fired(root)
}

#[cfg(target_os = "linux")]
pub fn rearm_fired(root: &Path) -> bool {
    linux::rearm_fired(root)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn rearm_fired(_root: &Path) -> bool {
    false
}

/// The paths whose vnodes fired since the last arm (drained now, kernel-synchronous), for
/// the INCREMENTAL tier (native/architecture.dx): `None` when there is no complete watch set to vouch —
/// only an armed daemon may treat "not fired" as "provably unchanged".
#[cfg(target_os = "macos")]
pub fn fired_paths(root: &Path) -> Option<Vec<PathBuf>> {
    macos::fired_paths(root)
}

/// Block until the project's watch set fires (or `timeout_ms` passes) — the eager pump's
/// wait. Returns whether an event arrived. The kernel queue is a shared consumer: events
/// this wait absorbs are recorded under the same lock every drain uses.
#[cfg(target_os = "macos")]
pub fn wait_event(root: &Path, timeout_ms: u64) -> bool {
    macos::wait_event(root, timeout_ms)
}

#[cfg(target_os = "linux")]
pub fn wait_event(root: &Path, timeout_ms: u64) -> bool {
    linux::wait_event(root, timeout_ms)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn wait_event(_root: &Path, timeout_ms: u64) -> bool {
    std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
    false
}

#[cfg(target_os = "linux")]
pub fn fired_paths(root: &Path) -> Option<Vec<PathBuf>> {
    linux::fired_paths(root)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn fired_paths(_root: &Path) -> Option<Vec<PathBuf>> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn replay(_root: &Path, _key: &str) -> Option<String> {
    None
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn arm(_root: &Path, _paths: &[PathBuf]) -> bool {
    false
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn confirm_quiet(_root: &Path) -> bool {
    false
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn commit(_root: &Path, _key: &str, _body: &str) {}

#[cfg(target_os = "macos")]
mod macos {
    use std::collections::{HashMap, HashSet};
    use std::ffi::c_void;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    // ── kqueue / rlimit FFI (no crates; libSystem symbols) ──────────────────────────────

    #[repr(C)]
    struct Kevent {
        ident: usize,
        filter: i16,
        flags: u16,
        fflags: u32,
        data: isize,
        udata: *mut c_void,
    }

    #[repr(C)]
    struct Timespec {
        tv_sec: isize,
        tv_nsec: isize,
    }

    #[repr(C)]
    struct Rlimit {
        rlim_cur: u64,
        rlim_max: u64,
    }

    extern "C" {
        fn kqueue() -> i32;
        fn kevent(
            kq: i32,
            changelist: *const Kevent,
            nchanges: i32,
            eventlist: *mut Kevent,
            nevents: i32,
            timeout: *const Timespec,
        ) -> i32;
        fn open(path: *const u8, oflag: i32) -> i32;
        fn close(fd: i32) -> i32;
        fn getrlimit(resource: i32, rlp: *mut Rlimit) -> i32;
        fn setrlimit(resource: i32, rlp: *const Rlimit) -> i32;
    }

    const EVFILT_VNODE: i16 = -4;
    const EV_ADD: u16 = 0x0001;
    const EV_CLEAR: u16 = 0x0020;
    /// Content-true triggers: writes, growth, deletion, rename, link-count, revoke. A bare
    /// `touch` (NOTE_ATTRIB) changes no byte and no finding — deliberately not watched, so
    /// the memo survives mtime-only churn the stat tier would have re-swept for.
    const NOTES: u32 = 0x0002 | 0x0004 | 0x0001 | 0x0020 | 0x0010 | 0x0040;
    /// `O_EVTONLY`: a watch-only descriptor that does not prevent unmount — the designed
    /// mode for exactly this use.
    const O_EVTONLY: i32 = 0x8000;
    const O_CLOEXEC: i32 = 0x0100_0000;
    const RLIMIT_NOFILE: i32 = 8;

    /// One watched project: its queue, its fd-per-path slab, and the committed bodies.
    struct Project {
        kq: i32,
        /// Path → (fd, slab index). The index rides each event's `udata`, so a drain can
        /// name exactly which paths fired and the next arm reopens only those.
        watched: HashMap<PathBuf, (i32, usize)>,
        /// Slab index → path (for udata resolution).
        slab: Vec<PathBuf>,
        /// Slab indices whose vnode fired since the last arm — reopened next arm (a
        /// rename-over leaves the old fd pointing at the dead vnode).
        fired: HashSet<usize>,
        /// Every requested path is watched; false disables the tier (fail toward stat).
        complete: bool,
        /// An event arrived since the last commit generation — memos are stale.
        dirty: bool,
        memos: HashMap<String, String>,
    }

    fn registry() -> &'static Mutex<HashMap<PathBuf, Project>> {
        static REGISTRY: std::sync::OnceLock<Mutex<HashMap<PathBuf, Project>>> =
            std::sync::OnceLock::new();
        REGISTRY.get_or_init(Default::default)
    }

    /// Raise the fd ceiling once — a watch set is an fd per input, and the default soft
    /// limit (256) is far below a real repository.
    fn raise_fd_limit() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| unsafe {
            let mut lim = Rlimit { rlim_cur: 0, rlim_max: 0 };
            if getrlimit(RLIMIT_NOFILE, &mut lim) == 0 {
                lim.rlim_cur = lim.rlim_max.min(65536);
                let _ = setrlimit(RLIMIT_NOFILE, &lim);
            }
        });
    }

    /// Drain every pending event (zero timeout). Marks fired paths and returns how many
    /// events arrived.
    fn drain(p: &mut Project) -> usize {
        let mut total = 0usize;
        let zero = Timespec { tv_sec: 0, tv_nsec: 0 };
        let mut buf: [Kevent; 64] = unsafe { std::mem::zeroed() };
        loop {
            let n = unsafe { kevent(p.kq, std::ptr::null(), 0, buf.as_mut_ptr(), 64, &zero) };
            if n <= 0 {
                break;
            }
            for e in buf.iter().take(n as usize) {
                p.fired.insert(e.udata as usize);
                if std::env::var_os("HELPERS_LINT_TRACE").is_some() {
                    let path = p.slab.get(e.udata as usize).map(|p| p.display().to_string());
                    eprintln!("[lint-kq] event fflags={:#x} on {:?}", e.fflags, path);
                }
            }
            total += n as usize;
            if n < 64 {
                break;
            }
        }
        if total > 0 {
            p.dirty = true;
            p.memos.clear();
        }
        total
    }

    /// Open `path` watch-only and register its vnode on `kq` with `udata = slot`.
    fn watch_one(kq: i32, path: &Path, slot: usize) -> Option<i32> {
        let mut cpath = path.as_os_str().as_bytes().to_vec();
        cpath.push(0);
        let fd = unsafe { open(cpath.as_ptr(), O_EVTONLY | O_CLOEXEC) };
        if fd < 0 {
            return None;
        }
        let change = Kevent {
            ident: fd as usize,
            filter: EVFILT_VNODE,
            flags: EV_ADD | EV_CLEAR,
            fflags: NOTES,
            data: 0,
            udata: slot as *mut c_void,
        };
        let zero = Timespec { tv_sec: 0, tv_nsec: 0 };
        if unsafe { kevent(kq, &change, 1, std::ptr::null_mut(), 0, &zero) } < 0 {
            unsafe { close(fd) };
            return None;
        }
        Some(fd)
    }

    pub fn arm(root: &Path, paths: &[PathBuf]) -> bool {
        raise_fd_limit();
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let p = reg.entry(root.to_path_buf()).or_insert_with(|| Project {
            kq: unsafe { kqueue() },
            watched: HashMap::new(),
            slab: Vec::new(),
            fired: HashSet::new(),
            complete: false,
            dirty: false,
            memos: HashMap::new(),
        });
        if p.kq < 0 {
            return false;
        }
        // Absorb pending events first so `fired` names every vnode needing a reopen.
        drain(p);
        let desired: HashSet<&PathBuf> = paths.iter().collect();
        // Close watches on paths that left the input set.
        let stale: Vec<PathBuf> =
            p.watched.keys().filter(|k| !desired.contains(*k)).cloned().collect();
        for path in stale {
            if let Some((fd, slot)) = p.watched.remove(&path) {
                unsafe { close(fd) };
                p.fired.remove(&slot);
            }
        }
        // Reopen fired paths — their fd may reference a renamed-over or deleted vnode.
        // (The open-new-before-close-old overlap below, in the shared open loop, is what
        // keeps a racing edit from vanishing with the old fd's pending knote.)
        let fired: Vec<usize> = p.fired.drain().collect();
        for slot in fired {
            let Some(path) = p.slab.get(slot).cloned() else { continue };
            let fresh = watch_one(p.kq, &path, slot);
            if let Some((fd, _)) = p.watched.remove(&path) {
                unsafe { close(fd) };
            }
            if let Some(fd) = fresh {
                p.watched.insert(path, (fd, slot));
            }
        }
        // Open everything not currently watched.
        let mut complete = true;
        for path in paths {
            if p.watched.contains_key(path) {
                continue;
            }
            let slot = p.slab.len();
            match watch_one(p.kq, path, slot) {
                Some(fd) => {
                    p.slab.push(path.clone());
                    p.watched.insert(path.clone(), (fd, slot));
                }
                None => complete = false, // vanished mid-arm or fd pressure — stat tier rules
            }
        }
        p.complete = complete;
        p.dirty = false;
        complete
    }

    pub fn confirm_quiet(root: &Path) -> bool {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let Some(p) = reg.get_mut(root) else { return false };
        p.complete && drain(p) == 0 && !p.dirty
    }

    pub fn commit(root: &Path, key: &str, body: &str) {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = reg.get_mut(root) {
            if p.complete && !p.dirty {
                p.memos.insert(key.to_string(), body.to_string());
            }
        }
    }

    pub fn rearm_fired(root: &Path) -> bool {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let Some(p) = reg.get_mut(root) else { return false };
        if !p.complete {
            return false;
        }
        drain(p);
        let fired: Vec<usize> = p.fired.drain().collect();
        let mut ok = true;
        for slot in fired {
            let Some(path) = p.slab.get(slot).cloned() else { continue };
            // Open the NEW watch BEFORE closing the old: the same vnode posts to both
            // registrations during the overlap, so an edit racing the reopen is never
            // lost — closing first deleted the old fd's pending knote (a measured-class
            // soundness hole, fixed by ordering alone).
            let fresh = watch_one(p.kq, &path, slot);
            if let Some((fd, _)) = p.watched.remove(&path) {
                unsafe { close(fd) };
            }
            match fresh {
                Some(fd) => {
                    p.watched.insert(path, (fd, slot));
                }
                None => ok = false, // vanished — membership changed, full arm owns it
            }
        }
        p.complete = ok;
        if ok && drain(p) == 0 {
            p.dirty = false;
            true
        } else {
            false
        }
    }

    pub fn fired_paths(root: &Path) -> Option<Vec<PathBuf>> {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let p = reg.get_mut(root)?;
        if !p.complete {
            return None;
        }
        drain(p);
        Some(p.fired.iter().filter_map(|slot| p.slab.get(*slot).cloned()).collect())
    }

    pub fn wait_event(root: &Path, timeout_ms: u64) -> bool {
        // Snapshot the queue fd WITHOUT holding the lock across the blocking wait.
        let kq = {
            let reg = registry().lock().unwrap_or_else(|e| e.into_inner());
            match reg.get(root) {
                Some(p) if p.kq >= 0 => p.kq,
                _ => {
                    // Not armed yet (the pump can start before the first arm) — retry
                    // soon, never sleep the whole timeout blind.
                    drop(reg);
                    std::thread::sleep(std::time::Duration::from_millis(timeout_ms.min(100)));
                    return false;
                }
            }
        };
        let ts = Timespec {
            tv_sec: (timeout_ms / 1000) as isize,
            tv_nsec: ((timeout_ms % 1000) * 1_000_000) as isize,
        };
        let mut buf: [Kevent; 8] = unsafe { std::mem::zeroed() };
        let n = unsafe { kevent(kq, std::ptr::null(), 0, buf.as_mut_ptr(), 8, &ts) };
        if n <= 0 {
            return false;
        }
        // Record what THIS consumer absorbed under the shared lock, then let the caller
        // drain the rest.
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = reg.get_mut(root) {
            for e in buf.iter().take(n as usize) {
                p.fired.insert(e.udata as usize);
            }
            p.dirty = true;
            p.memos.clear();
        }
        true
    }

    pub fn replay(root: &Path, key: &str) -> Option<String> {
        let trace = std::env::var_os("HELPERS_LINT_TRACE").is_some();
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let Some(p) = reg.get_mut(root) else {
            if trace {
                eprintln!("[lint-kq] miss: no watcher for {}", root.display());
            }
            return None;
        };
        if !p.complete || drain(p) > 0 || p.dirty {
            if trace {
                eprintln!(
                    "[lint-kq] miss: complete={} dirty={} ({} memos)",
                    p.complete,
                    p.dirty,
                    p.memos.len()
                );
            }
            return None;
        }
        let hit = p.memos.get(key).cloned();
        if trace && hit.is_none() {
            eprintln!(
                "[lint-kq] miss: no memo for key {key:?} (have: {:?})",
                p.memos.keys().collect::<Vec<_>>()
            );
        }
        hit
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The kernel-synchronous contract this tier exists for: a `write` that returned is
        /// visible to the very next drain — no sleeps, no flush, no daemon. The fseventsd
        /// path failed exactly this test.
        #[test]
        fn a_completed_write_is_visible_to_the_immediate_next_poll() {
            let dir = std::env::temp_dir().join(format!("lint-kq-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join("watched.rs");
            std::fs::write(&file, "fn a() {}").unwrap();

            let paths = vec![dir.clone(), file.clone()];
            assert!(arm(&dir, &paths), "watch set arms completely");
            assert!(confirm_quiet(&dir), "freshly armed set is quiet");
            commit(&dir, "k", "VERDICT");
            assert_eq!(replay(&dir, "k").as_deref(), Some("VERDICT"), "quiet set replays");

            for round in 0..5u32 {
                std::fs::write(&file, format!("fn a() {{ /* {round} */ }}")).unwrap();
                assert_eq!(
                    replay(&dir, "k"),
                    None,
                    "round {round}: an edit made before the poll must kill the memo"
                );
                assert!(arm(&dir, &paths), "re-arm after the edit");
                assert!(confirm_quiet(&dir), "quiet again after re-arm");
                commit(&dir, "k", "VERDICT2");
                assert_eq!(replay(&dir, "k").as_deref(), Some("VERDICT2"));
            }

            // A new file in a watched DIRECTORY (not itself watched) also kills the memo.
            std::fs::write(dir.join("new.rs"), "fn b() {}").unwrap();
            assert_eq!(replay(&dir, "k"), None, "a create in a watched dir is an event");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

/// The Linux event tier: inotify. Directory watches cover their direct children (event
/// records carry the child name), so the whole tree costs one watch descriptor per
/// directory — no fd-per-file. inotify events are generated inline in the VFS ops, the
/// same kernel-synchronous property the kqueue tier rests on; a queue overflow
/// (`IN_Q_OVERFLOW`) or a watched directory vanishing marks the set incomplete and the
/// stat tier rules until the next full arm. Same protocol, same memos, same fallbacks.
#[cfg(target_os = "linux")]
mod linux {
    use std::collections::{HashMap, HashSet};
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    extern "C" {
        fn inotify_init1(flags: i32) -> i32;
        fn inotify_add_watch(fd: i32, path: *const u8, mask: u32) -> i32;
        fn inotify_rm_watch(fd: i32, wd: i32) -> i32;
        fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    }

    const IN_NONBLOCK: i32 = 0x800;
    const IN_CLOEXEC: i32 = 0x8_0000;
    const IN_MODIFY: u32 = 0x2;
    const IN_CLOSE_WRITE: u32 = 0x8;
    const IN_MOVED_FROM: u32 = 0x40;
    const IN_MOVED_TO: u32 = 0x80;
    const IN_CREATE: u32 = 0x100;
    const IN_DELETE: u32 = 0x200;
    const IN_DELETE_SELF: u32 = 0x400;
    const IN_MOVE_SELF: u32 = 0x800;
    const IN_Q_OVERFLOW: u32 = 0x4000;
    const IN_IGNORED: u32 = 0x8000;
    const IN_ONLYDIR: u32 = 0x0100_0000;
    /// Content-true triggers, matching the kqueue tier's NOTE set (no ATTRIB: a bare
    /// touch changes no finding).
    const MASK: u32 = IN_MODIFY
        | IN_CLOSE_WRITE
        | IN_MOVED_FROM
        | IN_MOVED_TO
        | IN_CREATE
        | IN_DELETE
        | IN_DELETE_SELF
        | IN_MOVE_SELF;

    struct Project {
        fd: i32,
        /// Watch descriptor → the directory it names.
        dirs: HashMap<i32, PathBuf>,
        /// Directory → its watch descriptor (the arm diff's view).
        by_path: HashMap<PathBuf, i32>,
        fired: HashSet<PathBuf>,
        complete: bool,
        dirty: bool,
        memos: HashMap<String, String>,
    }

    fn registry() -> &'static Mutex<HashMap<PathBuf, Project>> {
        static REGISTRY: std::sync::OnceLock<Mutex<HashMap<PathBuf, Project>>> =
            std::sync::OnceLock::new();
        REGISTRY.get_or_init(Default::default)
    }

    /// Drain the queue (non-blocking reads). Records fired paths; overflow or a watched
    /// directory vanishing flips `complete` — the stat tier rules from then on.
    fn drain(p: &mut Project) -> usize {
        let mut total = 0usize;
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { read(p.fd, buf.as_mut_ptr(), buf.len()) };
            if n <= 0 {
                break;
            }
            let mut off = 0usize;
            while off + 16 <= n as usize {
                let wd = i32::from_ne_bytes(buf[off..off + 4].try_into().unwrap());
                let mask = u32::from_ne_bytes(buf[off + 4..off + 8].try_into().unwrap());
                let len = u32::from_ne_bytes(buf[off + 12..off + 16].try_into().unwrap()) as usize;
                let name_bytes = &buf[off + 16..(off + 16 + len).min(n as usize)];
                let name = name_bytes.split(|b| *b == 0).next().unwrap_or(&[]);
                if mask & IN_Q_OVERFLOW != 0 {
                    p.complete = false;
                } else if mask & (IN_DELETE_SELF | IN_MOVE_SELF | IN_IGNORED) != 0 {
                    p.complete = false; // a watched dir itself changed — membership moved
                } else if let Some(dir) = p.dirs.get(&wd) {
                    let path = if name.is_empty() {
                        dir.clone()
                    } else {
                        dir.join(std::ffi::OsStr::from_bytes(name))
                    };
                    p.fired.insert(path);
                }
                total += 1;
                off += 16 + len;
            }
        }
        if total > 0 {
            p.dirty = true;
            p.memos.clear();
        }
        total
    }

    pub fn arm(root: &Path, paths: &[PathBuf]) -> bool {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let p = reg.entry(root.to_path_buf()).or_insert_with(|| Project {
            fd: unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) },
            dirs: HashMap::new(),
            by_path: HashMap::new(),
            fired: HashSet::new(),
            complete: false,
            dirty: false,
            memos: HashMap::new(),
        });
        if p.fd < 0 {
            return false;
        }
        drain(p);
        // Watch the DIRECTORIES of the input set (IN_ONLYDIR rejects files — their parent
        // dirs are always in the set by construction).
        let desired: HashSet<&PathBuf> = paths.iter().collect();
        let stale: Vec<PathBuf> =
            p.by_path.keys().filter(|k| !desired.contains(*k)).cloned().collect();
        for path in stale {
            if let Some(wd) = p.by_path.remove(&path) {
                unsafe { inotify_rm_watch(p.fd, wd) };
                p.dirs.remove(&wd);
            }
        }
        let mut complete = true;
        for path in paths {
            if p.by_path.contains_key(path) {
                continue;
            }
            let mut cpath = path.as_os_str().as_bytes().to_vec();
            cpath.push(0);
            let wd = unsafe { inotify_add_watch(p.fd, cpath.as_ptr(), MASK | IN_ONLYDIR) };
            if wd >= 0 {
                p.dirs.insert(wd, path.clone());
                p.by_path.insert(path.clone(), wd);
            } else if path.is_dir() {
                complete = false; // a real directory refused a watch (limits) — stat tier
            }
        }
        p.complete = complete;
        p.dirty = false;
        p.fired.clear();
        complete
    }

    pub fn confirm_quiet(root: &Path) -> bool {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let Some(p) = reg.get_mut(root) else { return false };
        p.complete && drain(p) == 0 && !p.dirty
    }

    pub fn commit(root: &Path, key: &str, body: &str) {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = reg.get_mut(root) {
            if p.complete && !p.dirty {
                p.memos.insert(key.to_string(), body.to_string());
            }
        }
    }

    pub fn replay(root: &Path, key: &str) -> Option<String> {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let p = reg.get_mut(root)?;
        if !p.complete || drain(p) > 0 || p.dirty {
            return None;
        }
        p.memos.get(key).cloned()
    }

    pub fn fired_paths(root: &Path) -> Option<Vec<PathBuf>> {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let p = reg.get_mut(root)?;
        if !p.complete {
            return None;
        }
        drain(p);
        Some(p.fired.iter().cloned().collect())
    }

    extern "C" {
        fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
    }

    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }

    pub fn wait_event(root: &Path, timeout_ms: u64) -> bool {
        let fd = {
            let reg = registry().lock().unwrap_or_else(|e| e.into_inner());
            match reg.get(root) {
                Some(p) if p.fd >= 0 => p.fd,
                _ => {
                    drop(reg);
                    std::thread::sleep(std::time::Duration::from_millis(timeout_ms.min(100)));
                    return false;
                }
            }
        };
        let mut pfd = PollFd { fd, events: 0x1 /* POLLIN */, revents: 0 };
        let n = unsafe { poll(&mut pfd, 1, timeout_ms.min(i32::MAX as u64) as i32) };
        if n <= 0 {
            return false;
        }
        // Data is ready; the caller's next drain (under the lock) consumes and records it.
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = reg.get_mut(root) {
            drain(p);
        }
        true
    }

    /// inotify watch descriptors persist across events — nothing to reopen. Quiet check
    /// plus clearing the fired set is the whole re-arm.
    pub fn rearm_fired(root: &Path) -> bool {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let Some(p) = reg.get_mut(root) else { return false };
        if !p.complete {
            return false;
        }
        drain(p);
        p.fired.clear();
        if drain(p) == 0 {
            p.dirty = false;
            true
        } else {
            false
        }
    }
}
