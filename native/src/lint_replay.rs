//! `lint_replay` — the whole-project report replay (native/architecture.dx, "An unchanged project
//! replays the whole report").
//!
//! One WITNESS — a fold of every input's `(mtime, len)` state — decides between "return
//! the stored body" and "run the pipeline". The witness is verified by STATTING, never by
//! events: mtimes are updated by the kernel synchronously with the write, so a stat sweep
//! can never miss an edit (file-system events were tried and measured unsound — see the
//! spec). The walk fold comes in from the caller (the walk already carries every file's
//! state); this module folds the AUXILIARY inputs — law listing and documents, corpus,
//! `lint-index/` top level, the model dir, config, feedback — and stores body+witness
//! pairs in an `HLM1` container beside the verdicts, so a fresh process replays exactly
//! like the daemon: warm and cold differ only by `exec`.
//!
//! Derived caches (`lint-verdicts/`, `lint-replay/`) are excluded from the fold — they are
//! engine products of inputs already folded, and folding them would make every store
//! invalidate its own memo.

use std::path::{Path, PathBuf};

use crate::index::walk::{scan_dir, WalkedFile};
use crate::lint_codec::{kind, Dec, Enc};

/// A witness half: the state fold and the NEWEST input mtime seen while folding. The
/// newest mtime feeds the racy window below — `(mtime, len)` alone cannot distinguish a
/// same-length edit that lands in the same mtime tick as the state a report was stored
/// under (git's "racy index" problem), so a report is only replayable once every input is
/// provably OLDER than the moment it was stored.
#[derive(Clone, Copy)]
pub struct Witness {
    pub fold: u128,
    pub newest: u128,
}

/// A same-length edit inside this window of the store moment could share the stored mtime
/// tick and be invisible to the fold; conservative enough for coarse-timestamp filesystems
/// (HFS+ stores whole seconds). A racy store is still made — it self-heals into a
/// replayable one on the next run, when the store moment has moved past the window.
const RACY_WINDOW_NANOS: u128 = 2_000_000_000;

/// Fold one directory's entries (names + states, non-recursive). Renames, additions,
/// removals, and content edits of direct entries all change the fold.
fn dir_fold(dir: &Path, exclude: &[&str]) -> Witness {
    let mut entries = scan_dir(dir);
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries.iter().filter(|e| !exclude.contains(&e.name.as_str()) && !is_run_output(&e.name)).fold(
        Witness { fold: 1, newest: 0 },
        |acc, e| Witness {
            fold: acc.fold.rotate_left(9) ^ e.state ^ (crate::lint_ai::token_seed(&e.name) as u128),
            newest: acc.newest.max(e.mtime),
        },
    )
}

/// A plain file's witness (state 0 when absent — absence is a state too).
fn file_fold(p: &Path) -> Witness {
    std::fs::metadata(p)
        .map(|m| {
            let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
            let mtime = t.map(|d| d.as_nanos()).unwrap_or(0);
            Witness { fold: mtime ^ ((m.len() as u128) << 64), newest: mtime }
        })
        .unwrap_or(Witness { fold: 0, newest: 0 })
}

/// The auxiliary witness: every report input that lives OUTSIDE the walked tree. The walk
/// fold (files + states) is the caller's half; together they cover the report's whole
/// input domain — see the spec section for the enumeration and the exclusion argument.
pub fn aux_witness(root: &Path, data: &Path) -> Witness {
    let model_dir = crate::lint_train::model_dir_pub();
    [
        dir_fold(&root.join(".helpers/lint-rules"), &[]),
        file_fold(&root.join(".helpers/lint.json")),
        file_fold(&crate::lint_feedback::feedback_path(root)),
        dir_fold(&data.join("corpus"), &[]),
        dir_fold(&data.join("lint-index"), &[]),
        // The model dir participates in the FOLD (a retrain must invalidate) but not in
        // the racy NEWEST: the racy window guards USER-editable inputs whose mtime
        // granularity could hide a same-length edit, while these are machine-written
        // whole artifacts — counting them made every run that saved an overlay hold its
        // own memo hostage for the window (measured: a fresh `rust.overlay-*.bin` kept
        // benches on the slow path forever).
        Witness {
            fold: dir_fold(&model_dir, &["lint-verdicts", "lint-replay"]).fold,
            newest: 0,
        },
        Witness {
            fold: crate::lint_ai::token_seed(crate::lint_train::train_version()) as u128,
            newest: 0,
        },
    ]
    .into_iter()
    .fold(Witness { fold: 1, newest: 0 }, |acc, w| Witness {
        fold: acc.fold.rotate_left(13) ^ w.fold,
        newest: acc.newest.max(w.newest),
    })
}

/// Whether `name` is a lint run's OWN derived output inside the model dir — a per-project rule
/// overlay (`<lang>.overlay-<project fingerprint>.bin`). These can never be an INPUT to a lint
/// decision: an overlay is a pure function of the module, the corpus, the project's law and its
/// code — every one of which is folded on its own — keyed by the very fingerprint the run computed
/// before writing it. Folding it made a run invalidate ITSELF: each project change wrote ~18 new
/// overlay files, which changed the model dir's fold, which killed the memo the next run would have
/// replayed, which forced a full run, which wrote the overlays again. Measured 2026-08-17: that loop
/// pinned every edit on this repo to a ~5s full pass with the incremental tier never once engaging.
/// The racy-NEWEST half of this argument was already made (see [`aux_witness`]); this is the same
/// argument for the FOLD.
pub(crate) fn is_run_output(name: &str) -> bool {
    name.contains(".overlay-")
}

/// One file's contribution to the walk fold — ORDER-INDEPENDENT (XOR of per-file terms),
/// so the incremental tier patches a changed file in O(1): `fold ^= old_term ^ new_term`.
/// The term mixes the path seed multiplicatively into the state so two files swapping
/// states can never cancel.
pub fn file_term(rel: &str, state: u128) -> u128 {
    let k = (crate::lint_ai::token_seed(rel) as u128) | 1;
    state.wrapping_mul(k) ^ k.rotate_left(64)
}

/// The walk's half of the witness: the file set and every file's state, so an edit, an
/// addition, a removal, or a rename anywhere in the tree changes the fold.
pub fn walk_witness(files: &[WalkedFile]) -> Witness {
    files.iter().fold(Witness { fold: 1, newest: 0 }, |acc, f| Witness {
        fold: acc.fold ^ file_term(&f.rel, f.state),
        newest: acc.newest.max(f.mtime),
    })
}

/// Fold the walk's half and the auxiliary half into the one witness a report is stored
/// under. Distinct rotation keeps `(a, b)` and `(b, a)` from colliding.
pub fn combine(walk: Witness, aux: Witness) -> Witness {
    Witness { fold: walk.fold.rotate_left(29) ^ aux.fold, newest: walk.newest.max(aux.newest) }
}

/// Whether a witness is past its racy window RIGHT NOW — shared by this module's store and
/// the kqueue tier's commit gate (an edit inside the window can share the stored mtime
/// tick at the same length, invisible to any `(mtime, len)` fold).
pub fn replay_safe(witness: &Witness) -> bool {
    witness.newest.saturating_add(RACY_WINDOW_NANOS) < now_nanos()
}

/// UNIX now in nanoseconds — the store moment the racy window is measured against.
fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Where a project's replay container lives — beside the verdicts, keyed the same way.
/// The base is a parameter so tests pin their own (the suite's HOME mutates in parallel);
/// production always passes the model dir.
fn replay_path(base: &Path, root: &Path) -> PathBuf {
    base.join("lint-replay")
        .join(format!("{:016x}.bin", crate::lint_ai::token_seed(&root.display().to_string())))
}

/// One decoded container entry: args key, witness fold, replay-safe flag, body.
type Entry = (String, u128, bool, String);

/// Decode a container's entries; empty on any mismatch (kind, `TRAIN_VERSION`, truncation).
fn load_entries(base: &Path, root: &Path) -> Vec<Entry> {
    let Some(bytes) = std::fs::read(replay_path(base, root)).ok() else { return Vec::new() };
    let Some((stamp, mut d)) = Dec::open(&bytes, kind::REPLAY) else { return Vec::new() };
    if stamp != crate::lint_train::train_version() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let Some(n) = d.u() else { return out };
    for _ in 0..n {
        let Some(k) = d.str() else { break };
        let Some(hi) = d.fixed_u64() else { break };
        let Some(lo) = d.fixed_u64() else { break };
        let Some(safe) = d.boolean() else { break };
        let Some(b) = d.str() else { break };
        out.push((k, (u128::from(hi) << 64) | u128::from(lo), safe, b));
    }
    out
}

/// Return the stored report body for `(root, key)` when `witness.fold` matches what the
/// body was rendered under AND the store was not racy. One small read; no walk, no models,
/// no verdicts.
pub fn replay(root: &Path, key: &str, witness: Witness) -> Option<String> {
    replay_in(&crate::lint_train::model_dir_pub(), root, key, witness)
}

/// [`replay`] against an explicit container base — the seam the tests drive.
fn replay_in(base: &Path, root: &Path, key: &str, witness: Witness) -> Option<String> {
    load_entries(base, root)
        .into_iter()
        .find(|(k, w, safe, _)| k == key && *w == witness.fold && *safe)
        .map(|(_, _, _, body)| body)
}

/// Store `body` for `(root, key)` under `witness`, keeping other keys' entries. A store
/// whose newest input mtime is inside the racy window of NOW is marked unreplayable — the
/// next full run (with the same inputs, a later store moment) re-stores it replayable.
pub fn store(root: &Path, key: &str, witness: Witness, body: &str) {
    store_in(&crate::lint_train::model_dir_pub(), root, key, witness, body, now_nanos())
}

/// [`store`] with explicit container base and store moment — the seams the tests drive.
fn store_in(base: &Path, root: &Path, key: &str, witness: Witness, body: &str, now: u128) {
    let mut entries: Vec<Entry> = load_entries(base, root);
    entries.retain(|(k, _, _, _)| k != key);
    let safe = witness.newest.saturating_add(RACY_WINDOW_NANOS) < now;
    entries.push((key.to_string(), witness.fold, safe, body.to_string()));
    let mut e = Enc::new();
    e.u(entries.len() as u64);
    for (k, w, safe, b) in &entries {
        e.str(k);
        e.fixed_u64((w >> 64) as u64);
        e.fixed_u64(*w as u64);
        e.boolean(*safe);
        e.str(b);
    }
    let path = replay_path(base, root);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, e.finish(kind::REPLAY, crate::lint_train::train_version()));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store moment safely past the racy window of `w.newest`.
    fn later(w: Witness) -> u128 {
        w.newest + RACY_WINDOW_NANOS + 1
    }

    /// The replay lifecycle with NO sleeps around the edits: miss before store, hit after a
    /// non-racy store, and any file change (content, addition) flips the witness — the stat
    /// sweep is kernel-synchronous, which is the whole point.
    #[test]
    fn an_edit_immediately_before_a_check_always_flips_the_witness() {
        let dir = std::env::temp_dir().join(format!("lint-replay-test-{}", std::process::id()));
        let base = dir.join("models");
        std::fs::create_dir_all(&base).unwrap();
        let key = "max=80|langs=None";

        let witness_of = |root: &Path| walk_witness(&crate::index::walk::walk_repo(root));

        let w0 = witness_of(&dir);
        assert_eq!(replay_in(&base, &dir, key, w0), None, "no memo before the first store");
        store_in(&base, &dir, key, w0, "VERDICT", later(w0));
        assert_eq!(replay_in(&base, &dir, key, w0).as_deref(), Some("VERDICT"));
        assert_eq!(replay_in(&base, &dir, "other", w0), None, "different args, different entry");

        for round in 0..5usize {
            // Distinct length per round: the witness must flip on the length alone even if
            // two rounds land in one mtime tick.
            std::fs::write(dir.join("touched.rs"), "x".repeat(round + 1)).unwrap();
            let w = witness_of(&dir);
            assert_ne!(w.fold, w0.fold, "edit round {round} did not flip the witness");
            assert_eq!(replay_in(&base, &dir, key, w), None, "flipped witness must not replay");
            store_in(&base, &dir, key, w, "VERDICT2", later(w));
            assert_eq!(replay_in(&base, &dir, key, w).as_deref(), Some("VERDICT2"));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A store made while its newest input is inside the racy window must refuse to
    /// replay — a same-length edit in the same mtime tick would be invisible to the fold,
    /// so currency cannot be proven yet. The next store (same inputs, later moment) heals.
    #[test]
    fn a_racy_store_never_replays_until_reaffirmed() {
        let dir = std::env::temp_dir().join(format!("lint-replay-racy-{}", std::process::id()));
        let base = dir.join("models");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(dir.join("a.rs"), "fn a() {}").unwrap();
        let key = "max=80|langs=None";
        let w = walk_witness(&crate::index::walk::walk_repo(&dir));

        store_in(&base, &dir, key, w, "RACY", w.newest + 1); // stored inside the window
        assert_eq!(replay_in(&base, &dir, key, w), None, "racy store must not replay");
        store_in(&base, &dir, key, w, "SAFE", later(w)); // reaffirmed once the window passed
        assert_eq!(replay_in(&base, &dir, key, w).as_deref(), Some("SAFE"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Deleting a file flips the witness even though every remaining file is unchanged.
    #[test]
    fn a_deleted_file_flips_the_witness() {
        let dir = std::env::temp_dir().join(format!("lint-replay-del-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "fn a() {}").unwrap();
        std::fs::write(dir.join("b.rs"), "fn b() {}").unwrap();
        let before = walk_witness(&crate::index::walk::walk_repo(&dir));
        std::fs::remove_file(dir.join("b.rs")).unwrap();
        let after = walk_witness(&crate::index::walk::walk_repo(&dir));
        assert_ne!(before.fold, after.fold);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
