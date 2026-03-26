# Virtual Memory — Paging, Page Tables, Demand Paging & Page Replacement

## Overview

**Virtual memory** is a kernel abstraction that gives each process the illusion of a large, contiguous address space, decoupled from physical RAM. Paging is the primary implementation: the kernel divides memory into fixed-size pages, maintains page tables that map virtual addresses to physical frames, and uses on-demand loading and replacement algorithms to manage the disparity between process address space and available physical memory. This enables process isolation, overcommitment of memory, and efficient memory use.

## Address Translation: Virtual to Physical

A process works with **virtual addresses** (logical). The hardware (Memory Management Unit/MMU) translates them to **physical addresses** (RAM) using **page tables**.

```
Virtual Address (64-bit):
  +-------- Page Offset (bottom 12 bits for 4KB pages)
  |     +-- Page Number (top 52 bits)
  |     |
  +-----+----------+
  | PgN | Offset   |
  +-----+----------+
   (VPN)  (VPO)

Physical Address:
  +-------- Frame Offset (same as page offset)
  |     +-- Frame Number/PFN
  |     |
  +-----+----------+
  | PFN | Offset   |
  +-----+----------+
   (PPN)  (PPO)

Page table entry: Virtual Page Number (VPN) → Physical Frame Number (PFN)
```

### Page Table Structure

A **page table** is a kernel-managed data structure storing page table entries (PTEs). Each PTE maps one virtual page to a physical frame and carries metadata:

```
Page Table Entry (64-bit, x86-64):
  Bit 0: Present (1 = page in RAM, 0 = on disk, TLB exception, demand-loaded)
  Bit 1: Writable (1 = page writable)
  Bit 2: User accessible (1 = user mode can access)
  Bits 3-11: Available for OS use (dirty, referenced, accessed, etc.)
  Bits 12-51: Physical Frame Number (PFN)
  Bits 52-62: Available
  Bit 63: Execution Disable (NX bit; 1 = no execute)
```

**Multi-level page tables** reduce memory overhead. Instead of a flat table mapping all virtual pages, pages are indexed hierarchically:

```
x86-64 (4-level paging, assuming present-at-every-level):
Virtual Address: [L4 (9) | L3 (9) | L2 (9) | L1 (9) | Offset (12)]

PGD (Page Global Directory, 512 entries)
 |-- PMD (Page Middle Directory, 512 entries per PGD)
      |-- PTE (Page Table Entry, 512 entries per PMD)
           |-- Physical Frame Number (PFN)

Example: VN=0xFFF_FFFFF_FFFF → L4=511, L3=511, L2=511, L1=511 → top of kernel space
Example: VN=0x000_000_000_001 → L4=0, L3=0, L2=0, L1=1 → user space
```

On a page table walk, the kernel (or hardware, depending on architecture) performs a tree traversal. Each level indexes into the next table; the final level provides the PFN. If any level's present bit is 0, a page fault occurs.

### Translation Lookaside Buffer (TLB)

**Page table walks are expensive** — each requires multiple memory accesses. The **TLB** is a small, hardware-managed cache of recently-used VPN → PFN mappings:

```
TLB (typical: 32–4096 entries, fully associative or set-associative):
  +----+-----+-------+--------+
  | VPN| PFN | Valid | ASID   | (Address Space ID, per-process)
  +----+-----+-------+--------+
  ...
```

On a virtual address translation:
1. **TLB hit**: PFN retrieved immediately (~1 cycle)
2. **TLB miss**: Kernel or hardware walks page table (~100+ cycles), result inserted into TLB
3. **TLB flush**: On context switch (in most designs), all entries invalidated unless tagged with ASID (Address Space ID)

TLB misses are a major performance bottleneck. Large working sets, high page table levels, and frequent context switches all increase miss rates.

## Demand Paging

**Demand paging** defers loading a page into physical RAM until the program actually accesses it. The kernel sets the present bit to 0 for unloaded pages. When accessed, a **page fault** exception occurs:

```
Page Fault Handler:
1. Process issues load/store to virtual address
2. TLB miss → page table walk
3. PTE present bit = 0 → page fault exception
4. Kernel inspects page state:
   - If on disk (swapped out): Allocate free frame, read page from swap, set PTE
   - If not allocated: Allocate zero-filled frame, set PTE
   - If in page cache: Use cached copy, set PTE
   (If no free frames: invoke page replacement algorithm)
5. Update TLB
6. Resume process at faulting instruction
```

**Benefits:**
- Overcommitment: Total virtual address space > physical RAM
- Lazy initialization: Data allocated but not used stays on disk
- Efficient startup: Program binary loaded on-demand

**Cost:**
- Page faults block the process, introducing latency
- Thrashing: If working set > physical memory, constant faults and replacements tank throughput

## Page Replacement Algorithms

When physical memory is exhausted and a fault occurs, the kernel evicts a page to make room. Which page to evict is the **page replacement problem**.

### Optimal Algorithm (offline, never implemented)

Evict the page whose next access is furthest in the future. Requires prescience — used only as a benchmark. **Belady's anomaly**: increasing page frames can increase fault rate for certain access patterns and certain algorithms (not optimal).

### First-In-First-Out (FIFO)

```
Frame 1: Page A (loaded at time 0)
Frame 2: Page B (loaded at time 10)
Frame 3: Page C (loaded at time 20)

New fault at time 30: Evict Page A (oldest arrival)
```

Simple, low overhead. Poor performance: doesn't account for access frequency or recency. Vulnerable to Belady's anomaly.

### Least Recently Used (LRU)

Evict the page not accessed for the longest time. Approximates optimal for sequential and temporal-locality patterns:

```
Access sequence: A B C A B D (fault, evict C)
Frame state: [A, B, D]

Next: C (fault, evict D or B? depends on further accesses)
```

**Implementation**: Maintain a doubly-linked list; on each access, move page to head. Evict tail. Cost: O(1) list operations, but per-access overhead.

**Hardware approximation**: Reference bit set by MMU on every access. Periodically (e.g., every 100ms), clear all reference bits. During replacement, scan PTEs; evict first page with reference bit = 0 (not accessed recently).

### Clock Algorithm (Second-Chance)

Balances LRU approximation with low overhead:

```
Circular list of frames with reference bits:
  Frame A (ref=0) → Frame B (ref=1) → Frame C (ref=0) → Frame A ...
                    (clock hand)

Replacement step:
  Scan frames: If ref=0, evict. If ref=1, clear ref bit, advance pointer.
  Repeat until finding a ref=0 frame.
```

Lower overhead than LRU (bit-based, not list operations). Good practical performance.

### Aging

Like clock but with byte-valued age counters:

```
On each periodic interrupt (~10ms), scan all PTEs:
  If ref=1: age >>= 1; age |= 0x80 (shift right, set high bit)
  If ref=0: age >>= 1 (shift right, age decays)
  Clear ref bit

Evict: pick page with minimum age (least recently used period)
```

Decays recent accesses over time. Single byte per page. Better approximation of LRU than pure reference bit.

## Working Set Model & Thrashing

**Working set**: The set of pages a process needs to run efficiently (fits in physical memory without excessive faults).

**Thrashing**: When working set > physical memory, the system spends more time paging than executing. Fault rate skyrockets, throughput collapses.

```
Performance vs. Fault Rate (typical):
Throughput
    ^
    |     ***
    |    **   **
    |   *       **
    |  **         ****  ← thrashing region
    +--+--+--+--+--+--+--+--> Physical Memory Allocated
       0  5  10 15 20 25 30

At ~15 units: working set size equals available memory (optimal)
Beyond 20 units: thrashing — most of CPU time spent in swap I/O
```

**Mitigation:**
- **Admission control**: Don't admit processes if total working set > memory (load shedding)
- **Pre-paging**: Load predicted pages before fault (overhead vs. benefit trade-off)
- **Swap to SSD**: Slower than RAM but faster than traditional disk

## Page Size, Huge Pages & Fragmentation

### Standard Page Size

Most systems use **4 KB pages**. Trade-off:
- **Smaller pages**: Lower internal fragmentation, better memory efficiency, more TLB misses
- **Larger pages**: Fewer TLB misses, lower page table overhead, more memory wasted if data doesn't fill page

```
x86-64: 4 KB standard
ARM64: 4 KB or 16 KB
MIPS: 4 KB or 16 KB
PowerPC: typically 4 KB
```

### Huge Pages (Large Pages)

Physical pages larger than standard (e.g., 2 MB or 1 GB). Reduces TLB pressure for large working sets:

```
Standard: 1 TB RAM → 256M page table entries (4 KB pages)
Huge:     1 TB RAM → 512K page table entries (2 MB pages)

TLB coverage:
4 KB pages, 2048-entry TLB: Up to 8 MB resident (2048 * 4 KB)
2 MB pages, 512-entry TLB: Up to 1 GB resident (512 * 2 MB)
```

**Trade-offs:**
- **Pro**: Fewer TLB misses, lower page table overhead, reduced TLB pollution
- **Con**: Wasteful if process memory doesn't align to huge page boundaries; HugeTLB requires explicit mmap(2) flags; fragmentation if pages cannot be allocated

### Transparent Huge Pages (THP, Linux)

Kernel automatically promotes physically contiguous standard pages into huge page mappings. Reduces TLB misses without application changes. **Downside**: unpredictable latency (promotion/demotion done synchronously) and fragmentation if demotion needed.

## Memory-Mapped Files & Zero-Copy I/O

**Memory-mapping** maps a file (or device) directly into a process's address space. Reads/writes to the mapped region update the file:

```
Example (mmap):
  fd = open("data.bin")
  ptr = mmap(NULL, 1 GB, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)
  // Virtual addresses [ptr, ptr+1GB) now map file contents
  // Access via memory operations (load/store), not read(2)/write(2)
  
  Changes via ptr: → kernel page cache → disk (when fsync called)
```

**Benefits:**
- No explicit read(2)/write(2) calls; CPU operates on raw data
- Kernel page cache backs both memory map and file I/O
- Multiple processes can map same file read-only and see updates (Copy-on-Write variants exist for private mappings)

**Risks:**
- Bus faults on mapping region → process crashes (SIGSEGV)
- File truncation or deletion while mapped → undefined behavior (platform-specific)
- Large mappings can cause VM pressure

## ASLR (Address Space Layout Randomization)

**ASLR** randomizes the base addresses of memory regions (code, heap, stack, libraries) on each process execution. Defeats exploits that hard-code addresses for ROP or buffer overflow attacks:

```
Without ASLR (same process, multiple runs):
  Heap base: 0x555555554000
  Heap base: 0x555555554000  ← Same address, predictable
  
With ASLR (same executable, randomized):
  Heap base: 0x562d12734000
  Heap base: 0x555555554000  ← Different, unpredictable
```

Linux enables ASLR by default (`/proc/sys/kernel/randomize_va_space`). Some workloads disable it for reproducibility or because address-dependent code exists.

## See Also

- [OS Process Management](os-process-management.md) — Process lifecycle, scheduling, IPC
- [File Systems](os-file-systems.md) — Persistent storage, journaling, performance
- [Systems Reasoning](systems-reasoning.md) — Why page cache writes matter, fsync semantics