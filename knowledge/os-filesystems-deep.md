# Filesystem Internals — ext4, XFS, ZFS, Btrfs, and Specialized Filesystems

## Overview

Filesystems are the abstraction layer between applications (files, directories, metadata) and physical storage (disks, SSDs). Design choices profoundly impact performance, durability, administrative complexity, and hardware utilization. Modern filesystems diverge sharply: ext4 optimizes for compatibility and simplicity; XFS prioritizes scalability; ZFS guarantees correctness through checksums and copy-on-write; Btrfs adds snapshots and subvolumes; specialized filesystems (tmpfs, procfs, FUSE) solve specific niches. Understanding their internals reveals why each fits certain workloads and fails at others.

## Ext4: Block Allocation, Extents, and Journaling

Ext4 is the default Linux filesystem: stable, well-understood, moderate performance. It evolved from ext3 by adding extents (replacing block lists) and delayed allocation (batching writes).

### Extents Instead of Block Pointers

Ext3 used a block pointer list: for a 1 GB file, the inode had to store thousands of pointers, wasting space and requiring multi-level indirection.

**Ext4 extents** are ranges of contiguous blocks:
```
Extent: [physical_block_start, physical_blocks_count, logical_block_start]

Example: File of 1 million contiguous blocks
Ext3: Store 1,000,000 block pointers (4MB per file)
Ext4: Store 1 extent: (start, count=1000000, begin_at_0) = 12 bytes
```

Inodes store up to 4 extents directly; larger files use a B+ tree of extents.

**Benefits:**
- Reduced metadata (smaller inode tables, less I/O)
- Faster file access (fewer indirection levels)
- Less fragmentation (extents group proximate blocks)

### Journaling: The Consistency Problem

Ext4 uses ordered journaling: before writing to main data area, write intent log to journal.

```
Write sequence:
1. App calls write(); kernel buffers data
2. Before flushing to main area:
   - Write journal entry: "About to modify blocks 100–110, add extent X"
   - Wait for journal flush to disk
3. Write actual blocks 100–110
4. Write commit record to journal
5. Data is durable; old journal entry can be discarded
```

On crash during step 3:
- Recover logs journal
- Journal showed incomplete write; discard it
- File is unchanged (safe)

**Trade-off:** Journal overhead (write twice: journal + data), but consistency guaranteed.

### Delayed Allocation: Batch Writes

Ext4 delays extent allocation until flush time, bundling writes together.

```
Normal (eager) allocation:
  App writes block 0 → alloc extent [8000–8999], write journal
  App writes block 1 → alloc extent [9000–9999], write journal
  Result: 2 extents, 2 journal entries, fragmentation

Delayed allocation:
  App writes block 0 → buffer in memory
  App writes block 1 → buffer in memory
  Flush (at sync, timeout, memory pressure):
    Allocate contiguous extent [8000–8999] for both writes
    Write journal once, write data once
  Result: 1 extent, 1 journal entry, better locality
```

**Benefit:** Improves locality, reduces fragmentation, fewer I/O ops.

**Risk:** On hard crash, unwritten delayed allocations are lost (user data, not metadata).

### Limitation: Block Groups and Scalability

Ext4 divides the disk into block groups (typically 128 MB each). Allocator tries to keep inodes and data in the same group; cross-group gaps arise quickly on large filesystems.

Size limit: 16 TB (with 4 KB blocks). Scalability becomes problematic beyond 100 TB.

## XFS: Scalability via B+ Trees and Allocation Groups

XFS (originally SGI) is designed for very large filesystems (100s of TB) and high concurrency.

### B+ Tree Structure for Everything

XFS uses B+ trees for:
- **Inode allocation**: Tree of inodes sorted by inode number
- **Block allocation**: Tree of free extents (allocation groups maintain local free-space trees)
- **Directories**: Tree of (name → inode) sorted by name

B+ trees provide O(log N) lookups and allow efficient range scans (e.g., list all files starting with "A").

```
Directory tree (simplified):
              ["m"]
           /        \
    ["a"–"l"]      ["n"–"z"]
    /  |  \        /  |  \
 "apple" "berry" "lemon" "orange" ...
```

**Benefit:** Predictable performance on large directories (ext4 hash-based directories can thrash).

### Allocation Groups (AGs): Scalable Free Space

Instead of one global free-space tree, XFS partitions disk into allocation groups (AGs), each with its own free-space and inode trees.

```
Disk layout:
┌──────────────────────┬──────────────────────┬──────────────────────┐
│      AG 0            │      AG 1            │      AG 2            │
│   [AG metadata       │   [AG metadata       │   [AG metadata       │
│    Free-space trees   │    Free-space trees   │    Free-space trees  │
│    Inode trees...]    │    Inode trees...]    │    Inode trees...]   │
└──────────────────────┴──────────────────────┴──────────────────────┘

Multiple threads can allocate from different AGs in parallel → scales with core count
```

**Trade-off:** Better concurrency and scalability, but slightly higher metadata overhead.

### Extent-Based Allocation and Sparse Files

Like ext4, XFS uses extents. Sparse files (lots of zeros) are represented compactly.

```c
// Sparse file: 1 TB of zeros
File size: 1,099,511,627,776 bytes
Allocated extents: 0 (all pages are holes, filled on read)
```

XFS also supports **reflink** (copy-on-write clones): multiple files can share extents until one is modified.

## ZFS: Correctness and Reliability via CoW and Checksums

ZFS (from Sun/Oracle, open on Linux via OpenZFS) is radically different: it abandons traditional SSD/HDD assumptions and treats storage as a pool of devices.

### Copy-on-Write (CoW) Model

ZFS never overwrites data in place. All writes create new blocks; old versions remain until all references are released.

```
File: "data.txt" (blocks A, B, C)

Overwrite block B:
  Old: A → B → C
  New: A → B' → C
  (Original B is still on disk until no snapshots reference it)
```

**Benefits:**
- Simplifies crash recovery: on-disk state is always consistent (no journal needed)
- Enables snapshots (cheap point-in-time copies)
- Enables RAID-Z parity data is always consistent with user data

**Overhead:** Every write is an append, creating garbage collection (striving for contiguous free space).

### Checksums: Detect and Correct Bitrot

ZFS checksums every block (SHA-256 default, but configurable). On read:

```
Read block X:
  1. Check SHA-256(data) against stored checksum
  2a. Match: return data
  2b. Mismatch: bitrot detected
      - If RAID-Z: recover block from parity
      - Else: I/O error
```

**Use case:** Large capacity arrays prone to silent data corruption (cosmic rays, aging NAND).

### RAID-Z: Parity-Based Protection

ZFS RAID-Z is like traditional RAID-5/6 but with variable stripe widths per record (not per disk) and integrated with CoW.

```
RAID-Z (single parity):
  Data blocks: D1, D2, D3, D4
  Parity: P = D1 XOR D2 XOR D3 XOR D4
  Can recover from any 1 disk failure

RAID-Z2 (dual parity):
  P1 = D1 XOR D2 XOR D3 XOR D4
  P2 = (D1 * 2) XOR (D2 * 4) XOR (D3 * 8) XOR (D4 * 16) [Galois field]
  Can recover from any 2 disk failures
```

**Advantage:** Parity written atomically with data (CoW guarantees); no "RAID hole" (partial parity write on crash).

### Deduplication: Share Identical Blocks

ZFS can detect and deduplicate identical blocks across the filesystem.

```
File A: Hash(content) → blockid_1234
File B: Hash(content) → blockid_1234 (same!)
Result: One physical block, referenced twice
```

**Benefit:** Space savings on similar data (e.g., VM images, backups).

**Cost:** Heavy memory overhead (dedup table hash must fit in RAM). Typically only for small dedupe sets or specific workloads.

### Pooled Storage and Expansion

```
zpool create tank raidz sda sdb sdc
# Add more disk: zpool add tank raidz sdd sde sdf
# Automatically expanded; rebalancing happens in background
```

CoW abstraction allows seamless expansion without traditional LVM complications.

## Btrfs: In-Kernel CoW and Subvolumes

Btrfs (B-tree filesystem) is a modern in-kernel Linux filesystem combining ideas from ZFS.

### Subvolumes and Snapshots

Unlike directories, subvolumes are independent B-tree hierarchies within a filesystem.

```
Filesystem "myfs":
  Subvolume "root":     /home, /var, /etc
  Subvolume "data":     /data, /databases
  Subvolume "backup":   snapshot of "root" at T1
```

Snapshots are instant, cheap copies (CoW, point-in-time).

```bash
btrfs subvolume snapshot /myfs /myfs/backup_2025
# Instant; uses no additional space until divergence
btrfs subvolume list /myfs  # Show hierarchy
```

**Benefit:** Easy backups, time-travel recovery.

**Risk:** Snapshot rot; accumulated snapshots consume space and slow down.

### RAID and Device Balancing

```bash
mkfs.btrfs -m raid1 -d raid1 /dev/sda /dev/sdb
# Metadata and data both mirrored across two disks
```

Unlike ZFS, Btrfs integrates storage device management; similar pooling model.

### Limitation: Stability

Despite being in-kernel, Btrfs has historically suffered stability issues (data corruption bugs, recovery problems). Linux distributions still default to ext4 for safety.

## Tmpfs: In-Memory Filesystem

Tmpfs is a filesystem backed by RAM (or swap). Useful for temporary files without disk I/O.

```bash
mount -t tmpfs -o size=2G tmpfs /tmp
# Creates a 2 GB in-memory filesystem
```

**Characteristics:**
- Fast (no disk I/O)
- Non-persistent (lost on reboot)
- Size limit necessary (otherwise fills memory)
- Subject to memory reclaim (kernel can evict to make room)

**Use case:** /tmp, /run, compile caches.

## Procfs and Sysfs: Kernel Interfaces

Procfs and sysfs expose kernel state as files. They're not real filesystems (no disk backing).

```
# Procfs: Process and system information
cat /proc/cpuinfo   # CPU details
cat /proc/meminfo   # Memory usage
cat /proc/[pid]/maps  # Process memory map

# Sysfs: Device and subsystem state (more structured than procfs)
cat /sys/devices/pci0000\:00/0000\:00\:1f.2/host0/target0\:0\:0/0\:0\:0\:0/vendor
# Device information
```

**Design:**
- Read-only mostly (some sysfs nodes are writable for tuning)
- Generated dynamically by kernel drivers
- Hierarchical (reflects hardware topology, subsystems)

**Limitation:** No POSIX semantics (can't lseek, partial reads non-atomic).

## FUSE: User-Space Filesystems

FUSE (Filesystem in Userspace) lets userspace daemons implement filesystems, bypassing kernel code.

```c
// sshfs: SSH-based remote filesystem
sshfs user@host:/remote /localdir
# Kernel FUSE forwards read/write ops to sshfs daemon
# Daemon translates to SSH, returns results

// s3fs: S3-backed filesystem
s3fs bucket:/ /s3mount
# Read from S3 object store via HTTP as files
```

**Advantages:**
- Easy to implement (userspace code, standard libs)
- No kernel modifications
- Can leverage rich libraries (XML parsing, HTTP clients)

**Trade-offs:**
- Context switches (read → kernel → userspace daemon) slow it down significantly vs. in-kernel
- Userspace crash doesn't crash kernel, but filesystem hangs
- Security: daemon runs as a user, so access control must be enforced there

## Performance and Durability Trade-Offs

| Filesystem | Crash Consistency | Performance | Scalability | Snapshots | Use Case |
|------------|-------------------|-------------|-------------|-----------|----------|
| **ext4** | Journaling (journal vs. ordered) | Good | Moderate (~16TB) | No | General-purpose, defaults |
| **XFS** | Journal (v5: metadata only) | Very good | Excellent (100s TB) | No | Large-scale data centers |
| **ZFS** | CoW (always consistent) | Good (write-heavy penalty) | Excellent | Yes (native) | High reliability, storage pools |
| **Btrfs** | CoW (always consistent) | Good | Very good | Yes (native) | Modern systems; trade stability for features |
| **Tmpfs** | None (ephemeral) | Fastest | Limited (RAM) | No | Temporary data |

## See Also

- **os-file-systems** — High-level abstraction (inodes, directories, allocation)
- **database-internals-storage** — B-trees, page-based storage
- **infrastructure-container-networking** — Overlay filesystems in containers (layered images)
- **devops-docker** — Image layers and union filesystems