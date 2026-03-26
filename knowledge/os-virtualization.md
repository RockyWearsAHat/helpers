# Virtualization — Hardware-Assisted Emulation, Hypervisors, Paravirtualization, and Isolation Trade-Offs

## Overview

Virtualization creates isolated execution environments: multiple operating systems (or multiple instances of the same OS) appear to run independently on one physical machine. Modern virtualization uses hardware support (VT-x, AMD-V) to trap privileged instructions at minimal cost, enabling near-native performance. Hypervisors (KVM, Xen) mediate CPU, memory, and device access between VMs. Paravirtualization (virtio) accelerates I/O by having the guest kernel cooperate. Containers offer lighter-weight isolation without separate kernels. Understanding these mechanisms reveals why some workloads run efficiently virtualized while others (low-latency systems, extreme multi-tenancy) push against the abstraction.

## Hardware-Assisted Virtualization (VT-x, AMD-V)

Early hypervisors (QEMU without hardware support) emulated everything: CPU instructions, memory, devices. Extremely slow; rare and academic. Modern CPUs have **Virtual Machine Extensions** that support direct execution of guest machine code with hardware-enforced privilege separation.

### VT-x (Intel Virtualization Technology)

VT-x adds a **VMX root mode** (hypervisor runs here) and **VMX non-root mode** (guest runs here). Privileged instructions in non-root mode **trap** to the hypervisor.

```
Guest OS attempts: mov %cr0, %rax  (read control register)
│
Hardware detects: This is a privileged instruction and we're in VMX non-root mode
│
Hardware trap: Transfer control to hypervisor with exit reason "CR read"
│
Hypervisor code:
  Read actual %cr0 value from the guest's virtual PCB
  Inject result back into guest's register context
  Resume guest
│
Guest resumes: Unaware of the trap; sees the value it requested
```

### Extension Page Tables (EPT)

Virtual memory usually requires two-level translation: VA → GPA (guest physical) → HPA (host physical). Without hardware support, hypervisor traps every page fault, killing performance.

**EPT (Intel) / NPT (AMD)** do multi-level translation in hardware:

```
Guest process: VA → GPA (guest page tables)
Hardware (EPT walk): GPA → HPA (EPT tables)
Result: Direct memory access with nested page table walk
```

**Cost:** One extra layer of page table walks (TLB hit-rate drops; memory latency increases). But far cheaper than software virtualization.

### Nested TLB and shadowing trade-offs

- **EPT walk (slow)**: 3–4 page table levels walked; slow on TLB miss
- **Shadow page tables**: Hypervisor maintains HPA-keyed page tables that directly map VA → HPA, intercepting guest page-table writes. Faster TLB hits but harder to implement (invalidation on guest page table updates)

Most modern hypervisors use EPT, trading slower misses for simpler, more maintainable code.

## Hypervisors: Type 1 and Type 2

Hypervisors are software layers managing virtual machines. Two architectural flavors:

### Type 1 (Bare-Metal): KVM, Xen

Runs directly on hardware (no host OS). Manages hardware, CPUs, memory, devices. Highly privileged; critical for security.

#### KVM (Kernel-based Virtual Machine)

KVM is not a full hypervisor; it's a Linux kernel module that adds VMX support. The Linux kernel handles process management, memory, devices; KVM adds VM-specific logic.

```
VM execution loop (simplified):

┌─────────────────────────────────────────────────────────────┐
│ QEMU process (userspace)                                     │
│  ├─ Emulates devices (disk, network)                         │
│  ├─ Translates I/O requests to host calls                    │
│  └─ Calls ioctl(KVM_RUN) repeatedly                          │
│                                                               │
│ Kernel VT-x handling (KVM)                                    │
│  ├─ Enter VMX non-root; run guest until trap                 │
│  ├─ On exit (I/O, interrupt, exception): return to userspace │
│  └─ Resume on next ioctl(KVM_RUN)                            │
└─────────────────────────────────────────────────────────────┘

Performance implication:
  Heavy I/O workloads → many VM exits → context switches → overhead
  Compute-heavy → few exits → close to native performance
```

**Architecture**: Linux scheduler handles vCPU threads (one thread per vCPU). If host is overcommitted, context switching thrashes.

**Strength:** Simple, leverages Linux ecosystem, good for cloud (tight integration).

#### Xen: Hypervisor-First Architecture

Xen is a standalone hypervisor. Domain 0 (privileged, runs unmodified Linux) mediates I/O.

```
┌─────────────────────────────────────┐
│  Xen Hypervisor (privileged)        │
│  ├─ CPU scheduling                   │
│  ├─ Memory management (EPT/NPT)      │
│  └─ Hypercall interface              │
├─────────────────────────────────────┤
│ Domain 0 (Linux Kernel)              │
│  ├─ Driver stack (disk, network)     │
│  └─ Device model (for PV guests)     │
├─────────────────────────────────────┤
│ DomainU guests (VMs)                 │
│  ├─ Guest OS (Linux, Windows, etc.)  │
│  └─ Applications                      │
└─────────────────────────────────────┘
```

**Strengths:** Minimal hypervisor (smaller TCB, easier formal verification); strong security isolation (Xen microkernel philosophy).

**Trade-off:** Less direct integration with host kernel; driver stack lives in Domain 0.

### Type 2 (Hosted): VirtualBox, QEMU

Runs on a host OS (macOS, Windows, Linux). Host OS handles hardware, scheduling, memory; hypervisor runs as a user application or kernel extension.

```
┌─────────────────────────────────────┐
│  Host OS (Windows, macOS, Linux)    │
│  ├─ Process scheduling               │
│  ├─ Real hardware drivers            │
│  └─ Memory management                │
├─────────────────────────────────────┤
│  VirtualBox (kernel module + userspace daemon)
│  └─ Manages VMs as host processes    │
├─────────────────────────────────────┤
│  Guest OS (runs as host process)     │
│  └─ Applications                      │
└─────────────────────────────────────┘
```

**VirtualBox:**
- User-friendly GUI
- Portable (Mac, Windows, Linux)
- Leverages host OS resource management (more layers, less performance)
- Suitable for development; not production clusters

**QEMU:**
- Full system emulator (can emulate non-native architectures, e.g., ARM on x86)
- Used as KVM backend for device emulation (QEMU + KVM together form production hypervisor)
- Slower than native KVM due to additional userspace emulation

## Paravirtualization: Cooperative Guest

Full virtualization makes guest OS unaware it's virtualized (hardware traps handle everything). Paravirtualization has the guest OS *cooperate* with the hypervisor, recognizing VirtualCalls and optimizing for them.

### Xen PV (Paravirtual)

Guest OS (PV Linux) calls **hypercalls** instead of raw instructions:

```c
// Native kernel: mov %rax, %cr3  (load page table)
// PV guest: xen_set_pte_at(addr, pte)
  ├─ This is a hypercall (trap to Xen)
  ├─ Xen validates and updates EPT
  └─ Returns to guest

Benefit: Xen validates all page table updates; if page table is read-only at EPT, can't be modified by guest
```

**Trade-off:** Guest kernel must be modified (not Windows); much better performance (fewer traps, more predictable).

### Virtio: Standard Paravirtual I/O

Most modern hypervisors use virtio devices: paravirtual block, network, and console devices standardized by QEMU/KVM community.

```
Guest I/O request:
  1. Guest writes to virtio queue (shared memory)
  2. Guest sends interrupt to host ("buffer ready")
  3. Host (QEMU) reads queue, processes request
  4. Host writes response back to shared virtio queue
  5. Host sends interrupt to guest ("response ready")
  6. Guest reads response; returns to app
```

**Benefit vs. full emulation:**
- No instruction decoding overhead (direct shared-memory communication)
- Faster: ~900 MB/sec (vs. ~50 MB/sec with full e1000 emulation)

**Implementation:**
- Guest kernel has virtio driver (virtio_blk, virtio_net)
- Host QEMU implements backend (translates virtio to real I/O)

**Standardization:** Virtio industry spec (OASIS), supported by KVM, QEMU, Hyper-V, cloud providers.

## Nested Virtualization

A hypervisor (e.g., KVM) runs inside a VM. Useful for testing hypervisor code, multi-cloud deployments.

```
Physical CPU: VT-x enabled
├─ KVM (hypervisor 1) running VMs
├─ VM1 (Linux + KVM hypervisor 2)
│  └─ VM1.1 (Linux)
│  └─ VM1.2 (Linux)
```

### Treble-fault Problem

When nested hypervisor 2 causes an exception, nested hypervisor 1 must handle it. On real hardware, some exceptions (#NMI, #MCE) can't be redirected to nested VMs.

### Performance Impact

Each level adds a translation layer:
- Level 1 (hardware):  IPT (guest PA → host PA)
- Level 2 (nested VM):  EPT2 (guest VA → guest PA) + IPT (guest PA → host PA)

EPT2 walks are expensive; nested VM performance degrades significantly (typically 30–60% slowdown).

**Modern fix:** Support nested EPT directly in hardware (Intel Xeon v2+), allowing nested VMs to have their own page table hierarchies.

## Containers vs. VMs: Trade-Off Space

Containers (Linux namespaces + cgroups) and VMs offer different isolation levels:

### VMs: Strong Isolation, Higher Overhead

```
VM boundary: Separate kernel
├─ Hypervisor validates all cross-boundary ops
├─ Guest can't directly access host resources
├─ Crash of one VM doesn't affect others
├─ Secure multi-tenant (AWS, cloud providers)
└─ Overhead: Each VM boots separate kernel, uses ~100s MB RAM
```

### Containers: Weak Isolation, Low Overhead

```
Container boundary: Shared kernel (Linux)
├─ Namespaces and cgroups partition kernel resources
├─ Host kernel still controls all hardware
├─ Exploit in container may break out to host (recent: CVE-2021-22555)
├─ Efficient density (1000s containers on one host)
└─ Overhead: ~1–2 MB per container (just userspace library)
```

**Hybrid:** unikernels + containers, or MicroVMs.

## MicroVMs: Lightweight Virtualization

MicroVMs minimize VM size by stripping the guest kernel to essentials.

### Firecracker (AWS)

- Minimal KVM wrapper: ~1.3 MB per instance
- Boots in <100 ms
- No complex QEMU emulation; virtio only
- Designed for AWS Lambda, serverless

```
Cold start latency:
  Traditional VM: 2–3 seconds (BIOS, kernel boot, init)
  Firecracker:    <100 ms (minimal kernel, pre-configured)
```

**Trade-off:** No hardware support for GPUs, custom devices; simple workloads only.

### gVisor (Google)

gVisor runs a guest OS in a userspace sandbox, implementing Linux syscalls. Each gVisor instance is a heavy process (heavier than Firecracker, lighter than traditional VM).

```
Host kernel ← syscall interception ← gVisor (userspace) ← Guest apps
```

**Benefit:** Security: zero host kernel exploits from guest.

**Cost:** All syscalls translated through gVisor (significant overhead vs. native containers).

## Unikernels: Single Address Space

Unikernels statically link application code with a minimal kernel, producing a single binary that's both OS and app.

```
Traditional OS:
  ├─ Kernel (millions of lines)
  ├─ Libc (hundreds of K)
  └─ Application (your code)

Unikernel:
  └─ Statically linked: kernel + libc + app = single image (~MB)
```

### MirageOS, Unikraft

```ocaml
(* MirageOS: specify what you need *)
let () =
  let open Mirage in
  main "Unikernel.Main" (console @-> stackv4 @-> job)
(* Compiler links only TLS, network stack, console driver *)
```

**Advantages:**
- Minimal attack surface (no unnecessary kernel subsystems)
- Tiny deployment (MB, fast boot)
- Deterministic performance (no scheduler contention)

**Trade-offs:**
- Language-specific (OCaml, Rust, Haskell mostly)
- Limited portability (can't run arbitrary binaries)
- Debugging harder (no shell, minimal tools)

**Use case:** Microservices with narrow, known dependencies; IoT; embedded.

## Resource Allocation and Performance

### CPU Scheduling Under Overcommit

If vCPUs > pCPUs, hypervisor context-switches vCPU threads:

```
Host: 4 pCPUS, 8 vCPUs
VM1: 4 vCPUs
VM2: 4 vCPUs

Scheduler maps vCPU threads into 4 cores; each core gets 2 vCPUs
Result: Frequent context switches, cache misses, low performance
```

**Mitigation:**
- Avoid overcommit (1:1 or 1.5:1 vCPU:pCPU ratio)
- Pin vCPUs to pCPUs (sacrifice elasticity for predictability)

### Memory Ballooning and Swapping

Hypervisor can't directly reclaim guest memory. **Balloon driver** in guest cooperates:

```
Host memory low:
  1. Hypervisor signals guest balloon driver
  2. Guest balloon allocates memory (pressuring applications)
  3. Guest returns memory to hypervisor
  4. Hypervisor repurposes freed RAM
```

Alternative: guests evict dirty pages to swap (hypervisor-managed).

**Trade-off:** Responsiveness vs. density; overcommitting RAM risks swapping (SSD I/O is slow).

## See Also

- **os-containers-internals** — Namespaces, cgroups (same isolation primitives)
- **devops-docker** — Containers as a distribution mechanism
- **infrastructure-container-networking** — Virtual network interfaces, veth, bridge
- **cloud-gcp-compute** — GCP's managed compute (VMs vs. Compute Engine)