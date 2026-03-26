# Git Internals — Deep Dive Into Object Storage, Compression & Protocols

## Content-Addressable Storage & Object Model

Git's fundamental insight: store data not by naming (filenames) but by computing a hash of contents. Four object types form the core:

- **Blob**: Raw file contents, just bytes. No filename, no metadata beyond the hash.
- **Tree**: Directory listing. Each entry maps a name to a blob or tree, with file mode bits (regular file, executable, symlink).
- **Commit**: DAG node. Points to a tree (snapshot of all files), parent commit(s) (history), author/committer metadata, timestamp, message.
- **Tag**: Annotated reference. Points to a commit with tagger, timestamp, message. Lightweight tags are just direct refs to commits.

All objects are addressed by SHA-1 hash (transitioning to SHA-256). Within a repository, object identity is immutable—changing one byte changes the hash, creating a different object entirely. This immutability and content-origin guarantee makes Git tamper-evident by design.

```
commit abc1 → tree ghi3 → blob def2
                       ↘ blob jkl4
commit abc2 → tree ghi3 (same tree, reused)
      ↑
   parent ref
```

## Packfiles: Compression & Streaming

Raw objects are stored as individual files in `.git/objects/` (sharded by hash prefix). This is inefficient for large repositories and network transfer.

**Packfiles** solve this by:

1. **Batching objects together** into a single file (`.pack`)
2. **Delta compression**: Instead of storing full copies of similar objects, store one full object + a delta (set of byte-level changes) to reach another
3. **Indexing** (`.idx` files): Fast random access into packfiles without scanning linearly

### Delta Encoding

Consider two similar files in consecutive commits. Rather than store both in full:

```
Original blob A: [bytes 0–999]
New blob B:      [bytes 0–100] + [100 bytes change] + [bytes 110–999]

Stored as:
Blob A (full): [bytes 0–999]
Blob B (delta): "copy 100 bytes from offset 0; insert [new bytes]; copy 890 bytes from offset 110"
```

Delta chains can nest: B is a delta from A, C is a delta from B. Chains reduce size dramatically for similar files (e.g., source code, configuration) but create CPU cost on reads (must reconstruct by chaining deltas).

### Pack Index (`.idx`)

Maps object SHAs to byte offsets in the `.pack` file. Binary format allows O(log n) lookup. Without it, finding an object requires scanning the entire `.pack`.

## Merge Strategies

Git can merge using different algorithms, affecting the outcome when conflicts overlap.

### Recursive (Default in Git < 2.34)

The original workhorse strategy for merging two branches. Tree-thinking: recursively compute a common ancestor (lowest common ancestor, the most recent commit both branches share), then apply changes from both branches relative to that ancestor.

**Algorithm:**
1. Find merge base: LCA of the two commits
2. Diff: base → ours (what changed on this branch)
3. Diff: base → theirs (what changed on their branch)
4. Apply both diffs independently; if both changed the same region, conflict

**Behavior on criss-cross merge** (where both branches have multiple common ancestors):
- Produces a virtual "auto-generated" merge base by recursively merging candidates
- Can be slower and occasionally produces surprising results

### ORT (Optimal Recursive Merge, Git 2.34+)

A refined version of recursive that addresses criss-cross issues more intelligently.

**Improvements:**
- Computes multiple merge bases more efficiently
- Detects and handles content-level conflicts better (distinguishes "both added the same line" from "one deleted, one modified")
- Often resolves more automatically; reduces spurious conflicts

**Trade-off:** Slightly higher CPU cost but more predictable merge outcomes.

### Octopus

Merges three or more branches in one operation. Rarely used; mostly for release commits that pull multiple feature branches.

```bash
git merge branch-a branch-b branch-c
```

Constraint: No manual conflict resolution during octopus merge. If any conflict arises, the merge aborts. Requires all branches to be merge-clean.

### Resolve (Legacy)

Three-way merge using a single, stable merge base. Simpler algo than recursive; sees less use now.

### Ours (Resolve Strategy)

```bash
git merge -s ours feature  # Keep our version, discard theirs
```

Useful for "we decided to go a different direction." Merge succeeds and records the merge history, but all changes from the other branch are ignored.

## Reflog & Dangling Objects

### Reflog (Reference Log)

Every time HEAD, a branch, or any ref moves, Git records it locally. The reflog is not pushed; it's a local safety net.

```bash
# List HEAD reflog
git reflog
# Output: abc1234 HEAD@{0}: commit: fix bug
#         def5678 HEAD@{1}: rebase: checkout main
#         ...

# Go back in time
git checkout HEAD@{3}
```

Default expiration: 90 days for unreferenced entries. After `git commit --amend` or `git reset --hard`, the old commit doesn't vanish immediately; it's dangling but in the reflog.

### Dangling Objects & Garbage Collection

A commit becomes dangling when no ref (branch, tag, reflog entry) points to it:

```bash
# Find dangling objects
git fsck --lost-found

# Manually recover (creates a branch)
git branch recovered abc1234
```

**Garbage collection** (`git gc`) removes dangling objects older than the reflog expiration window (default 2 weeks). It also repacks loose objects into packfiles.

```bash
git gc --aggressive    # Repack more thoroughly; slower but smaller repo
git gc --prune=now     # Prune immediately (default: wait 2 weeks)
```

Risk: If you `git gc --prune=now` immediately after an accidental reset, you can lose the safety net. Most workflows run `git gc` only during maintenance windows.

## Fsck: Integrity Verification

`git fsck` scans the entire object database and checks:

- All objects are parseable (valid SHA, correct type)
- All objects referenced by other objects exist
- No circular dependencies
- DAG structure is intact

```bash
git fsck --full        # Deep check; slower
git fsck --unreachable # Show objects not reachable from any ref
```

Used to detect:
- Corruption from disk errors or interrupted writes
- Orphaned objects with no incoming references
- Pack corruption

Not foolproof (doesn't validate semantic correctness, only structural integrity), but good for catching filesystem-level corruption.

## Transfer Protocols: Fetch, Push, Clone

### SSH (Secure)

```bash
git clone ssh://git@github.com/user/repo.git
# or
git clone git@github.com:user/repo.git
```

Runs `git-upload-pack` on server (for fetch) or `git-receive-pack` (for push) as the authenticated user. Compressed; bidirectional.

### HTTPS (Widely Supported)

```bash
git clone https://github.com/user/repo.git
```

HTTP-based smart protocol (Git over HTTP). Server runs CGI or equivalent to handle requests. Can be painfully slow on poor connections; stateless per-request.

### Git Protocol (Legacy & Insecure)

```bash
git clone git://github.com/user/repo.git
```

Bare-bones, fast, read-only. No authentication, no encryption. Rare in modern practice; most servers disabled it.

### Local (File System)

```bash
git clone /path/to/repo.git
git clone file:///absolute/path/to/repo.git
```

Useful for testing or local cloning. No compression on wire (no wire involved), but can hardlink objects if both repos are on the same filesystem.

### Smart Protocol Negotiation

When fetching, client and server negotiate:

1. **Advertise refs**: Server lists branches, tags, and HEAD
2. **Negotiate wants**: Client says "I want these commits" and "I already have these"
3. **Compute delta**: Server determines minimal set of objects to send
4. **Send pack**: Transmits packfile with only new objects

This negotiation avoids sending already-present commits, saving bandwidth.

## Shallow Clones & Shallow History

Useful for large repositories: clone only the last N commits of history.

```bash
git clone --depth=1 repo.git      # Only tip commit
git clone --depth=50 repo.git     # Last 50 commits
```

Creates a shallow repository with a `.git/shallow` file marking commit boundaries. Later `git fetch --deepen` can unflatten. Limitations:

- Shallow commits have no local parents; rebasing against them fails
- Merging shallow branches requires care
- Some tools don't handle shallow repos well

Common for CI (no need to pull entire history) and bandwidth-constrained environments.

## Performance & Maintenance Tuning

### Repacking Strategy

Loose objects are slower to iterate than packed objects. Over time, repositories accumulate loose objects.

```bash
# Auto-repack when threshold reached
git config gc.autopacklimit 256   # Repack if 256+ packs exist

# Aggressive repacking (hourly/daily task)
git gc --aggressive
```

### Object Pool (For Forks & Mirrors)

In environments with many clones/forks of the same repo (GitHub Enterprise, for instance):

```bash
git clone --reference /shared/pool.git new-clone
```

New clone hardlinks objects from the pool, saving storage. Shared pool is marked as alternate object directory.

### Repository Size

Large repos (especially with binary files) benefit from:

- Git LFS (Large File Storage): Pointers in Git, actual files in separate storage
- Shallow cloning for CI
- Sparse checkout: Clone only needed directories
- Partial clone with `--filter=blob:none`: Fetch commits and trees, delay blob download