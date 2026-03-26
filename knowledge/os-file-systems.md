# File Systems — Storage Abstraction, Allocation Strategies & Modern Designs

## Overview

A **file system** is a kernel abstraction that organizes persistent data into named files and directories, handles concurrent access, ensures consistency on power failure, and manages physical storage allocation (disk blocks). Design decisions (inode-based vs extent-based, journaling vs copy-on-write, B-tree vs bitmap allocation) profoundly impact performance, durability, capacity, and administrative complexity.

## Inode-Based File Systems (Ext4, XFS)

### Inode Structure

An **inode** is a kernel data structure representing a file (or directory). It stores metadata and pointers to data blocks:

```
Inode 1234 (file.txt, 12 KB):
  Mode: Regular file, permissions 0644
  UID/GID: Owner (user 1000, group 1000)
  Size: 12,288 bytes
  Timestamps: Created, Modified, Accessed (in nanoseconds for Ext4)
  Link count: Number of hard links
  Block pointers:
    Direct pointers (12 blocks of 4KB each = 48KB inline)
    Single indirect (pointer to block containing pointers)
    Double indirect (pointer to block of pointers to blocks)
    Triple indirect (deeper tree for very large files)
  File type: Regular, directory, symlink, device
  Reference count: In-memory; incremented when file opened
```

**Ext4 Extents**: Modern ext4 uses extents (ranges of contiguous blocks) instead of block pointers, reducing metadata overhead and fragmentation. Up to 4 extents can be stored directly in the inode; larger files use a B+ tree of extents.

### Directory Structure

Directories are special files (mode bit flags them as directory). Contents are key-value pairs (filename → inode number):

```
Directory Entry:
  Inode number (4 bytes)
  Entry length (2 bytes)
  Name length (1 byte)
  Type (1 byte): file, directory, symlink, etc.
  Filename (variable length, null-terminated or counted)
  Padding to align to entry length
```

A directory lookup (e.g., `/home/user/file.txt`) traverses:
1. Root inode (inode 2 by convention).
2. Read directory contents; find "home" entry → inode X.
3. Read inode X directory; find "user" entry → inode Y.
4. Read inode Y directory; find "file.txt" entry → inode Z.
5. Read inode Z to stat the file.

This is a **walk**, costing multiple I/Os. Caching (dcache in Linux) speeds repeated lookups.

### Block Allocation

Ext4 and XFS allocate blocks from a **free space bitmap** or **extent-based allocator**:

**Bitmap Approach**:
- Global bitmap tracks which blocks are free (1 bit per block).
- Allocator searches bitmap for first/best free block.
- Allocation: set bit to 1 (occupied).
- Deallocation: set bit to 0 (free).
- Fragmentation: over time, free blocks scatter; allocation cost rises.

**Extent-Based Allocation** (XFS):
- Free space tracked as extents (ranges) in B-tree (FsB-tree for "Free Space B-tree").
- Allocator searches tree for extent matching request size.
- Avoids fragmentation by consolidating free ranges.
- Better scalability for large filesystems.

### Journaling

**Journal** is a log of updates written before they're applied to the filesystem. On crash, filesystem replays log to recover consistency.

**Metadata journaling** (ext4 default):
1. Update logged to journal.
2. Journal committed (fsync'd to disk).
3. Update applied to filesystem in place.
4. Journal entry removed.

On crash: replays uncommitted journal entries before mounting, ensuring metadata consistency.

**Full journaling** includes data:
- Slower (write data twice: to journal, then to place).
- Stronger guarantee: data is recoverable without "lost+found" orphan files.

**Ordered journaling** (older ext3):
- Metadata journaled; data written before metadata logged.
- Prevents data corruption (data reaches disk before metadata refers to it).
- Slower than metadata-only, but faster than full.

### Inode Limits

Ext4: Pre-allocated inode table; **232 inodes maximum**. For 1TB filesystem with 4KB blocks (268M blocks), many blocks become inode table. Choosing inode count at `mkfs` time is a long-term commitment.

XFS: **Dynamic inode allocation**. Inodes allocated on demand; no fixed limit. Better scalability.

## Copy-on-Write (CoW) File Systems (ZFS, Btrfs, APFS)

CoW replaces journaling with a different durability model: **never overwrite data in place; write updates to new blocks, then atomically redirect pointers**.

### Architecture

```
Original state:
  Block tree:
    Inode 100 → [Block 1000, Block 1001, Block 1002]

Modification of inode 100 (say, add data):
  Write new data to Blocks [2000, 2001, 2002]
  Write new inode to Block 3000
  Update parent directory pointer (atomically: one disk write)

On crash:
  Either old state (Block 1000,... → Inode 100 at old address) OR new state (Block 2000,... → Inode 100 at new address)
  No intermediate corruption possible
```

**Benefits**:
- **Atomicity**: Single block write (the root pointer) determines consistency. No journal replays.
- **Snapshots**: At any point in time, take snapshot by referencing current tree root. Cheap; blocks are shared until modified (CoW semantics).
- **Clones**: Duplicate file by sharing blocks until modification.

**Costs**:
- **Write Amplification**: Every modification creates new blocks. Old blocks become garbage until not referenced by any snapshot (leads to garbage collection overhead).
- **SSD Performance**: SSDs, especially consumer-grade, suffer from write amplification (limited erase cycles). APFS documented to have poor performance on HDDs due to CoW write amplification.
- **Complexity**: Tree management more intricate than journaling.

### ZFS (Zettabyte File System)

Sun Microsystems design (now open source). Key features:

- **Pool-Based**: Filesystem operates over a storage pool (multiple disks managed together).
- **Integrated RAID**: Built-in RAID 0/1/Z/Z2/Z3 (Z = RAID-like parity; Z2 = dual parity; Z3 = triple parity).
- **Snapshot & Clone**: Instant snapshots, cheap clones.
- **Compression**: Transparent per-block compression (gzip, zstd).
- **Self-Healing**: Checksums detect corruption; automatically repair from parity/mirrors.
- **B-tree Metadata**: Scalable, hierarchical.

**Drawbacks**: High memory overhead (ARC cache); slow with small pools; not suitable for resource-constrained systems.

### Btrfs (B-tree File System)

Linux-native CoW filesystem. Similar to ZFS but tighter kernel integration.

- **Profile-Based RAID**: Metadata RAID and data RAID configured independently.
- **Subvolumes**: Hierarchical CoW snapshots at volume level.
- **Checksums**: Optional per-block checksums (disabled by default for performance).
- **Quotas**: Per-subvolume quota enforcement.
- **Scrub**: Offline verification of checksums.

**Status**: Marked stable in Linux 5.0+, but experienced bugs in early versions. Production use is viable but requires caution.

### APFS (Apple File System)

macOS Catalina+ and iOS 10.3+ filesystem.

- **CoW-based**: Every write creates new blocks; old blocks reclaimed (similar to ZFS/Btrfs).
- **B-tree Structure**: File and folder B-trees; efficient lookups, range queries.
- **Encryption**: Per-file encryption (can be hardware-backed).
- **64-bit Inodes**: 2^64 possible files (no inode exhaustion).
- **Nanosecond Timestamps**: Better precision than HFS+ (1-second granularity).
- **Sparse Files**: Efficient handling of sparse files.
- **Hard Links**: Full support (unlike modern NTFS).

**Limitation**: Write amplification leads to performance degredation on spinning disks and excessive writes reducing SSD lifespan.

## VFS (Virtual File System) Abstraction

VFS is a kernel layer that abstracts across filesystem implementations:

```
Application
    ↓
POSIX API (open, read, write, fsync, unlink, mkdir)
    ↓
VFS Layer
    ↓
Concrete Implementation (ext4, XFS, NFS, /proc, /sys)
```

VFS defines standard operations:
- `inode_operations`: lookup, create, unlink, link, chmod, etc.
- `file_operations`: open, release, read, write, mmap, fsync, etc.
- `dentry_operations`: cached directory lookups.

Each filesystem provides implementations. Kernel generalizes across them. Example:

```c
// Application code
fd = open("/path/to/file", O_RDONLY);
read(fd, buf, 1024);

// Kernel VFS dispatch
VFS.open() → [Is /path on ext4? Yes] → ext4.open()
VFS.read() → [What inode type?] → ext4.read_block() → read physical block
```

## Permissions Model (POSIX)

Each file/directory has:

```
-rw-r--r-- user group size date name
```

- **Owner (User)**: File creator; can change permissions, ownership (if root).
- **Group**: Sets of users (e.g., "developers", "admins").
- **Permissions**: Read (r=4), Write (w=2), Execute (x=1) for Owner, Group, Others.
  - `chmod 755 script.sh` = rwxr-xr-x (owner all, group/others execute + read).
  - `chmod g+w file` = Add write to group.

**Execute**: On files, execute permission allows process creation. On directories, execute permission allows traversal (e.g., `cd /var/log` requires execute on all parent directories: `/`, `/var`). Read permission on directory allows listing contents; write allows creating/deleting files.

**Special Bits**:
- **Setuid (s, 4000)**: File executed with owner's privilege, not executor's.
- **Setgid (s, 2000)**: File executed with group's privilege; inherited by files created in directory.
- **Sticky (t, 1000)**: Only owner/root can delete file in directory (e.g., `/tmp`).

## Hard Links vs Symbolic Links

### Hard Link

Multiple directory entries pointing to the same inode:

```bash
ln file.txt link.txt          # Create hard link
ls -i file.txt link.txt       # Same inode number
```

**Properties**:
- Both entries refer to the same inode and data blocks.
- Deleting one entry decrements inode link count; inode freed when count reaches 0.
- Cannot cross filesystems (inode numbers are per-filesystem).
- Cannot link directories (would create loops).
- New entry indistinguishable from original (except inode).

### Symbolic (Soft) Link

Directory entry containing a path to another file:

```bash
ln -s file.txt symlink.txt    # Create symbolic link
ls -i file.txt symlink.txt    # Different inode numbers
readlink symlink.txt          # Outputs: file.txt
```

**Properties**:
- Symlink is a separate inode containing the target path (string).
- Reading symlink automatically redirects to target (transparently in most contexts).
- Can cross filesystems.
- Can link directories and create cycles (kernel detects and errors).
- If target deleted, symlink becomes dangling.

## FUSE (Filesystem in Userspace)

FUSE allows filesystems to run outside the kernel (in user processes). Example:

```
User Application
        ↓
Kernel FUSE Interface
        ↓
FUSE Daemon (user process)
        ↓
Custom Implementation (e.g., S3FS, encryptFS, bindFS)
```

**Pros**: Modularity; no kernel patches needed; isolation.
**Cons**: Performance overhead (kernel ↔ userspace context switch for each operation); not suitable for high-frequency I/O.

**Examples**:
- **SSHFS**: Mount remote SSH directory locally.
- **S3FS**: Access S3 bucket as mounted filesystem.
- **EncryptFS**: Transparent file encryption.
- **Mergerfs**: Union mount multiple directories.

## Network File Systems (NFS, CIFS/SMB)

### NFS (Network File System)

RPC-based protocol for remote file access. Stateless design (server doesn't maintain connection state).

```
Client                  NFS Server
  ├─ LOOKUP /var/log    ──→  [return handle]
  ├─ READ handle        ──→  [return data]
  ├─ WRITE handle, data ──→  [ack]
```

**Versions**: NFSv3 (1992, still common), NFSv4 (2003, stateful with leases, more secure).

**Cons**: Unsecured by default (no auth; relies on IP trusted network); async write replies create data loss risk; fragile to network delays (NFS clients hang waiting for server).

### CIFS/SMB (Server Message Block)

Windows-native protocol (SMB 3.0+ cross-platform).

```
Client                SMB Server
  ├─ NEGOTIATE        ──→  [dialect, capabilities]
  ├─ SETUP (auth)     ──→  [session ID]
  ├─ TREE_CONNECT     ──→  [tree ID]
  ├─ CREATE file      ──→  [file handle]
  ├─ READ file        ──→  [data]
```

**Features**: Host-based auth; file locking; oplocks (client-side caching leases); more complex, more stateful than NFSv3.

**Performance**: Overhead of auth and state; better for LAN (not WAN).

## Modern Approaches & Trends

### Tiered Storage

Filesystems (e.g., Btrfs) support multiple device types (SSD for hot, HDD for cold). Intelligent tiering moves data based on access patterns.

### Cloud-Native

- **S3-like Object Storage**: Immutable, append-only semantics (easier consistency guarantees).
- **Distributed Filesystems**: Metadata distributed (e.g., Ceph FS, GlusterFS); no single SPOF.
- **Distributed Transactions**: Multi-block consistency via consensus (Raft, Paxos).

### Learned Index Structures

Research explores ML-based B-tree replacements (model learns data distribution, reduces lookup iterations). Early-stage; not production-ready.

## See Also

- **data-engineering-streaming** — Distributed ledgers, immutable logs.
- **architecture-data-mesh** — Data organization at scale.
- **memory-management** — Virtual memory, paging, caching.