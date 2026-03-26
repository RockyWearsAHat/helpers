# Storage Hardware

Storage hardware spans several orders of magnitude in capacity, latency, and cost, each tier suited to different roles in data systems.

## Hard Disk Drives (HDD)

**Physical Design** — one or more magnetic platters coated with ferromagnetic material, rotated at constant speed (5400-15000 RPM typical). Read/write heads float on air bearing near the surface, magnetically recording data. Platter rotation creates mechanical latency: seek time (head movement to track, ~2-12 ms typical) and rotational latency (waiting for data to rotate under head, ~2-4 ms at 7200 RPM).

**Sequential vs Random** — sequential throughput ~150-200 MB/s by reusing head position. Random I/O heavily penalized by seek + rotational latency, typically 50-100 random I/Os per second. Workload matters immensely: batch processing tolerates random I/O; transactional workloads suffer.

**Capacity & Density** — modern drives reach 10-20 TB in 3.5" form factor through techniques like Shingled Magnetic Recording (SMR, overlapping tracks) and continued areal density improvements. Cost per terabyte remains lower than SSD (~$10-20/TB for HDD, ~$50-100/TB for SSD).

**Reliability** — mechanical failure rates increase with age and temperature. AFR (Annual Failure Rate) from large-scale studies shows ~2% for consumer drives in typical datacenter; enterprise drives similar age show comparable rates, contradicting marketing claims. Disk failures show batching: hardware defects cluster (batch-correlated failures), escalating rebuild risk in RAID arrays of identical drives.

## Solid-State Drives (SSD)

**NAND Flash Technology** — transistors store charge in floating-gate cells. Cell types by bits-per-cell:
- **SLC** (1 bit): highest performance and endurance, lowest capacity, ~$2-3/GB
- **MLC** (2 bits): balanced, ~$0.5-1/GB
- **TLC** (3 bits): mainstream consumer, ~$0.1-0.2/GB
- **QLC** (4 bits): maximum capacity, limited endurance, ~$0.05-0.1/GB

Cells wear out after finite write cycles: SLC ~100k program-erase cycles; QLC ~1000-3000 cycles.

**Flash Translation Layer (FTL)** — firmware layer mapping logical blocks to physical cells, invisible to OS. Performs wear leveling (distributing writes across cells), garbage collection (reclaiming space from deleted data), and error correction (ECC). Adds write amplification: user writes to 1 LBA may result in 3-10x physical writes due to FTL bookkeeping.

**Wear Leveling** — moves infrequently-updated data to highly-worn blocks, equalizing wear. Without it, hot blocks fail in months; with it, uniform failure spread over years. Various algorithms (dynamic, static, hybrid) exist; firmware bugs in wear leveling remain a source of premature SSD failures.

**TRIM Command** — allows OS to inform SSD of deleted blocks, enabling FTL to reclaim space earlier. Improves performance in sustained write workloads; absence degrades performance as unused space fills with invalid data.

**Performance** — sequential read/write: 500-7000 MB/s depending on interface and generation. Random I/O: 20k-500k IOPS. Latency: 0.05-0.2 ms for reads (vs 2-12 ms for HDD). Endurance: 600-3000 TB written (TBW) for consumer drives; enterprise drives higher.

**Data Retention** — unpowered NAND slowly leaks charge. TLC/QLC under typical conditions: data loss possible after 1-2 years without power. Consumer SSDs not suitable for long-term archival.

## NVMe Protocol

**Interface** — replaces SATA/SAS with direct PCIe attachment. Phase-In: NVMe 1.0 (2013), 1.2 (2014), 1.3 (2017), 1.4 (2019), 2.0 (2021).

**Command Queues** — SATA/AHCI queues 32 commands; NVMe supports 64k queues × 64k entries each, enabling massive parallelism. Low latency command processing (no SATA HCMD register polling required).

**Form Factors** — M.2 2280 most common; smaller (2230) and larger (22110) exist. Enterprise uses U.2/U.3 (2.5" form factor) or EDSFF (1U/2U suitable).

**Advanced Features** — NVMe 2.0 added Zoned Namespaces (ZNS, exposes zones as sequential-write-required regions, reduces FTL complexity), Key-Value (KV, hardware-native key-value operations).

## RAID and Storage Tiering

**RAID Levels** — distribute data across multiple drives:
- **RAID 0**: striping for capacity/bandwidth; no redundancy; single-drive failure = total data loss
- **RAID 1**: mirroring for redundancy; 50% capacity overhead; read throughput improves (parallel reads from two mirrors)
- **RAID 5**: striping with XOR parity; one drive failure tolerance; read performance good; write performance hit (parity recomputation)
- **RAID 6**: dual parity (P and Q using Galois field math); two drive failure tolerance; higher write overhead than RAID 5
- **RAID 10**: striped mirrors (RAID 1 of RAID 0s), higher cost but faster rebuild and good write performance

**Rebuild Risk** — RAID 5 with large drives: rebuilding a 10 TB drive takes 24-48 hours during which another failure is catastrophic. Large capacity + slow rebuild = high URE (Unrecoverable Read Error) risk. RAID 6 or RAID 10 preferred for high-availability systems with large drives.

**Rebuild Performance Tuning** — stripe size, chunk size, and resync parallelism affect rebuild speed. Smaller chunks provide better load distribution but increase overhead. OS/RAID controller can throttle rebuild to avoid I/O stalls on production workloads.

**Storage Tiering** — hot data on SSD, warm data on HDD, cold data on tape. Automatic tiering (OS or array controller moves data) reduces manual management but risks performance surprises. Tiering metadata overhead can be significant (~1% of capacity).

**Write Hole in UPS + RAID** — power loss during RAID write can leave parity inconsistent. Solution: battery-backed write cache (holds writes in NVRAM until safely on persistent media) or journal-based recovery (redo/undo logs). Enterprise arrays all use write caches.

**Erasure Coding** — generalization of RAID allowing k data blocks + m parity blocks (tolerates m concurrent failures). Used in distributed systems (Amazon EC2 RAID-like, Google Colossus); introduces network/CPU overhead compared to single-machine RAID.

## Persistent Memory (Intel Optane Deprecated)

3D XPoint technology offered byte-addressable non-volatile memory at DRAM-like latencies (~1 µs). Production discontinued 2021; alternative persistent memory technologies still exploratory.

---

See also: RAID recovery, FTL algorithms, endurance prediction models.