# Git Internals — Deep: Storage Model, Content Addressing, and Compression

## Content-Addressed Object Model

Git's foundational design organizes all data around **content addressing**: an object's identity is the SHA-1 hash of its contents. This guarantees that identical content always maps to the same hash and that any modification invalidates the hash, making repositories tamper-evident.

Four object types form the complete graph:

- **Blob**: Raw file contents. No metadata—just bytes. A blob is the same whether it came from `main`, a feature branch, or a deleted file.
- **Tree**: Represents a directory at a single point in time. Maps names (file/subdirectory names) to blob or tree SHAs. Includes mode bits: `100644` (regular file), `100755` (executable), `120000` (symlink).
- **Commit**: A DAG node. Contains: root tree SHA, parent commit SHA(s), author name/email, commit timestamp, committer name/email, commit timestamp, and message. The identity of a commit is immutable once created.
- **Tag**: Lightweight—just a ref to a commit SHA. Annotated tags are objects: contain the referred commit SHA, tagger name/email, timestamp, and message.

Example structure for a single commit:

```
Commit abc123 → Tree ghi456 (root)
                  ├─ blob def789 (file.txt, mode 100644)
                  ├─ blob jkl012 (file.sh, mode 100755)
                  └─ Tree mno345 (src/)
                       └─ blob pqr678 (main.py, mode 100644)
Commit stu901 → Tree ghi456 (same tree—reused, not duplicated)
     parent: abc123
```

This deduplication is automatic: two commits with identical directory trees both reference the same tree object.

## SHA-1 Content Addressing

Every object is identified by `sha1(type + length + contents)`. The hash is 40 hex characters (160 bits). Git stores objects with a directory structure based on the hash: the first two hex digits become a directory, the remaining 38 become the filename within `.git/objects/xy/`.

```
Object abc123def... is stored as .git/objects/ab/c123def...
```

This sharding spreads objects across 256 directories, avoiding filesystem inode limits and improving lookup performance.

**SHA-1 transition to SHA-256**: Git is migrating to SHA-256 for resistance to collision attacks. New objects can use SHA-256 in repositories configured with `objectFormat=sha256`. Legacy repositories remain SHA-1 until explicitly converted.

### Collision Resistance and Integrity

Because object identity is purely content-based, any attempt to modify an object changes its SHA, creating a new object entirely. Existing refs and commits that pointed to the old hash now reference a different object, making unauthorized edits detectable. This design provides cryptographic integrity without a central authority.

## Storage Formats: Loose and Packed Objects

### Loose Objects

When you make a commit, Git writes the objects as individual files. Loose objects are deflate-compressed (zlib) but otherwise readable:

```bash
git cat-file -p abc123  # Reads the object, decompresses, displays contents
```

A repository composed only of loose objects is called unpacked. For many commits and files, this produces deep directory trees with many small files—inefficient for network transfer and backup.

### Packfiles: Compression and Delta Encoding

**Packfiles** solve this by combining many objects into a single file with delta compression. When you run `git gc` (garbage collection) or push to a remote, Git packs loose objects.

**Structure of a `.pack` file:**

1. **Header**: Format version, count of objects
2. **Object entries**: Each object stored as either full (base) or delta (relative changes from another object)
3. **Checksum**: SHA-1 of the entire pack (integrity check)

**Companion `.idx` file:**

For each object SHA in the `.pack`, the `.idx` file stores its byte offset in the `.pack`. This enables O(log n) binary search, avoiding linear scans.

### Delta Compression Algorithm

Two versions of a source file typically differ in small, localized ways. Rather than store both in full:

```
Version A: "Hello World. This is a test file.\nLine 2\nLine 3\n"
Version B: "Hello World. This is a test file.\nLine 2 modified\nLine 3\n"

Stored as:
Base object A (full): "Hello World. This is a test file.\nLine 2\nLine 3\n"
Delta to B:
  - copy 30 bytes from offset 0 of A
  - insert "modified"
  - copy 8 bytes from offset 38 of A
```

Git computes deltas during packing. The `repack` process analyzes similarity between blobs and trees, grouping similar objects into delta chains. Chains can nest: A → B → C, where B is delta(A) and C is delta(B). On read, Git reconstructs by following the chain.

**Delta chain depth tradeoff:**

- Deeper chains compress better (smaller storage)
- Deeper chains cost more CPU to reconstruct (more iterations)
- `git config` setting `core.deltaBaseCacheLimit` and `core.maxDeltaChain` tune this

## The `.git/` Directory Structure

Understanding `.git/` layout is essential for recovery and optimization:

```
.git/
  objects/          # All blobs, trees, commits, tags
    [00-ff]/        # 256 shards for loose objects
    pack/           # Packfiles and index files
  refs/             # Branch and tag references
    heads/          # Local branches (one file per branch)
    remotes/        # Remote tracking branches
    tags/           # Annotated tag refs
  HEAD              # Text file containing current branch
  config            # Local repo settings
  description       # One-line repo description (GitHub uses this)
  hooks/            # Client-side scripts (pre-commit, post-checkout, etc.)
  info/             # Git metadata
    exclude         # Like .gitignore, not committed
    refs            # Alternate object stores, shallow file
  logs/             # Reflog entries
    HEAD
    refs/heads/[branch]
  index             # Staging area (binary format)
```

**Key files:**

- `HEAD` contains the current symbolic reference: `ref: refs/heads/main` or a detached commit hash.
- `config` contains repo-local settings (overrides `~/.gitconfig`).
- `.git/index` is the staging area—a binary snapshot of what's staged to commit.
- `logs/` contains reflog entries: timestamps and ref changes for each branch and HEAD.

## Merge Strategies

Git supports multiple merge algorithms; the strategy affects how conflicts are detected and resolved.

### Recursive (Git < 2.34 default)

Computes the **lowest common ancestor (LCA)** of the two commits being merged, then applies changes from both commits relative to the LCA.

```
Main:      A → B → C
Feature:   A → B → D → E
LCA: B
```

Three-way diff: (B→C) vs. (B→D vs B→E). When both change the same region, conflict.

On criss-cross merges (multiple common ancestors), recursive generates a virtual merge base by recursively merging candidates. This can be surprising and slower.

### ORT (Optimal Recursive Merge, Git 2.34+)

Refinement of recursive using a cleaner approach to multiple merge bases. Handles content-level conflicts better (distinguishes "both added the same line" from "one deleted, one modified"). Generally fewer spurious conflicts and better performance.

### Resolve

Three-way merge using a single, stable merge base. Simpler than recursive. Rarely used in modern workflows.

### Ours

Merges but keeps all content from the current branch, discarding the other branch's changes (while recording the merge). Useful for "we decided on a different direction."

```bash
git merge -s ours feature  # Accept theirs silently
```

### Octopus

Merges three or more branches simultaneously. Fails if any conflict arises—no manual resolution possible. Mostly decorative for release commits pulling multiple feature branches.

## Reflog and Dangling Objects

The **reflog** records every movement of HEAD and branches. It's a local safety net for recovering from accidental operations.

```bash
git reflog              # List HEAD reflog: HEAD@{0}, HEAD@{1}, ...
git reflog show refs/heads/main    # Reflog for main branch
git show HEAD@{n}       # View commit at that reflog entry
git reset --hard HEAD@{1}  # Undo last operation by returning to previous state
```

**Dangling objects** are commits, trees, or blobs that no ref points to. They persist until garbage collection. Common sources:

- Commits amended with `git commit --amend`
- Commits reset away with `git reset --hard`
- Commits deleted with `git branch -D` (local only)
- Commits rebased and abandoned

**Finding dangling objects:**

```bash
git fsck --lost-found    # Scans object database, reports dangling
git log -g --all         # Show all refs including dangling commits
```

**Recovery:**

```bash
git branch recovered cd1234  # Create branch from dangling commit SHA
```

Dangling objects are garbage-collected after the reflog expiration window (default 90 days for unreferenced objects, 2 weeks for reflog entries). See `git config gc.reflogExpire` and `git config gc.reflogExpireUnreachable`.

## Garbage Collection and Maintenance

`git gc` performs repository maintenance:

1. **pack**: Combines loose objects into packfiles with delta compression
2. **prune**: Removes dangling objects older than the expiration window
3. **repack**: Combines existing packfiles for better layout
4. **rerere**: Records and replays conflict resolutions

```bash
git gc                  # Automatic maintenance (respects expiration windows)
git gc --aggressive     # More thorough repacking but slower
git gc --prune=now      # Immediately remove dangling objects (risky!)
```

**Aggressive packing** increases delta chain length and spends more CPU analyzing similarity. Useful before large backups but overkill for daily operations.

**Never run `git gc --prune=now` immediately after an accidental reset.** The object might still be referenced by the reflog, but immediate pruning ignores the expiration window and deletes it.

## Fsck: Object Database Integrity

`git fsck` scans all objects and checks:

- All objects are valid (parseable, correct type)
- All referenced objects exist
- DAG structure is intact (no cycles)
- No object corruption

```bash
git fsck                      # Check current repo
git fsck --full               # Include unreachable objects
git fsck --lost-found         # Create .git/lost-found/ with dangling objects
git fsck repo-path            # Check specific repository
```

Output shows errors like:

- `missing [type] [sha]` — referenced object not found
- `broken link from [obj1] ([field]) to [obj2]` — broken reference
- `dangling [type] [sha]` — no ref points to this object

Corruption is rare (SHA mismatch catches it), but can occur from disk corruption, interrupted writes, or repository cloning from corrupted source. `git fsck --full` followed by manual recovery is the standard approach.

## Shallow Repositories and Partial Clones

**Shallow clones** limit history depth to save bandwidth:

```bash
git clone --depth 1 https://example.com/repo.git  # Clone with 1 commit of history
```

Shallow clones suppress old commits but keep full working tree. They're useful for CI systems that don't need history. **Limitation:** You can't push from a shallow clone to a non-shallow repository without first `git fetch --unshallow`.

**Partial clones** use object filters to fetch only some objects:

```bash
git clone --filter=blob:none https://example.com/repo.git  # Fetch tree/commit, download blobs on-demand
```

Differs from sparse-checkout: partial clone downloads objects on-demand; sparse-checkout skips checking out certain paths locally but still downloads all objects.

---

_Sources: Pro Git v2 (https://git-scm.com/book/en/v2), Git documentation (https://git-scm.com/docs), Git internals paper (Scott Chacon, Kenichi Ogasawara)_