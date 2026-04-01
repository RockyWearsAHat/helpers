# Container Internals — Namespaces, cgroups, Overlay Filesystems, and Security

## Overview

Containers are not a single technology but a composition of Linux kernel features: **namespaces** (isolation), **cgroups** (resource limits), **overlay filesystems** (layered images), and **seccomp** (syscall filtering). Understanding these layers enables building custom containers, debugging container issues, and implementing secure and efficient container runtimes.

## Linux Namespaces: Kernel Resource Isolation

A namespace is a wrapper around a global system resource that makes it look local to processes within that namespace. The Linux kernel provides 8 namespace types.

### Process ID (PID) Namespace

Each namespace has its own process ID space. PID 1 in a namespace is the init process for that namespace.

**Hierarchy**:
```
Host namespace (PID 1: systemd)
  ├─ Container 1 namespace (PID 1: sh)
  │   └─ Sleep process (PID 5 inside container)
  └─ Container 2 namespace (PID 1: init)
```

The host sees the sleep process as PID 100 (some high number), but inside the container, it's PID 5. Both exist in the same host kernel; the namespace is purely a visibility filter.

**Implication**: `kill(5, SIGTERM)` inside the container kills the process; from the host, you send `kill(100, SIGTERM)`. The mapping is transparent.

### Mount (mnt) Namespace

Each namespace has its own mount table. Mounting a filesystem in one namespace doesn't affect others.

**Example**:
```bash
# Host: mount filesystem at /mnt/host
mount /dev/sda1 /mnt/host

# Container namespace: mount different filesystem at /mnt
mount /dev/sdb1 /mnt
# Processes in container see /mnt, but host sees /mnt/host
```

**Root filesystem isolation**: Containers use **chroot** (or **pivot_root**) to change their root to a different directory. Combined with a mount namespace, the container sees a completely different filesystem tree.

### Network (net) Namespace

Each namespace has its own network interfaces, routing table, firewall rules, and socket namespace.

**Isolation**:
```
Host namespace:     eth0, eth1, default route 10.0.0.1
Container namespace: lo (loopback only), isolated routing
```

Containers can't see or talk to host network interfaces directly (unless explicitly connected via veth pairs or bridges).

### Unix Timesharing System (uts) Namespace

Each namespace has its own hostname and NIS domain name.

```bash
# Host: uname -n → "myhost"
# Container: uname -n → "container-123"
```

Cosmetic but important for container identity.

### User (user) Namespace

Each namespace maps UIDs/GIDs differently. The container root (UID 0) can be mapped to an unprivileged user on the host.

**Mapping**:
```
Container: UID 0 (root) ↔ Host: UID 100000 (unprivileged)
Container: UID 1 → Host: UID 100001
```

**Advantage**: A compromise of the container doesn't give the attacker true root on the host. Mitigation for privilege escalation.

**Challenge**: Filesystem permissions must align. Files owned by container UID 0 must be readable by host UID 100000.

### IPC (ipc) Namespace

Each namespace has its own SysV IPC resources (shared memory, semaphores, message queues).

```bash
# Host: ipcmk -M 1024 → create shared memory in host IPC namespace
# Container: ipcmk -M 1024 → create shared memory in container IPC namespace
# Separate resources, no conflicts
```

### Control Group (cgroup) Namespace

Each namespace has its own view of the cgroup hierarchy. Containers don't see host cgroups, simplifying resource management abstraction.

### Time Namespace (Recent)

Each namespace can have a different time offset (e.g., for testing time-dependent code or running multiple instances of time-dependent software).

## Creating Namespaces: unshare and clone

**unshare()** syscall creates a new namespace for the calling process:

```c
unshare(CLONE_NEWNET | CLONE_NEWPID);
// Process now has new PID and network namespaces
```

**clone()** (used by fork) can create namespaces for child processes:

```c
clone(child_main, child_stack, CLONE_NEWNET | CLONE_NEWPID, NULL);
// Child process gets new namespaces
```

**Container runtime** (Docker, containerd) calls these syscalls to set up namespace isolation.

## cgroups: Resource Limits and Accounting

**cgroups** (control groups) limit and account for CPU, memory, I/O, and network resources used by a group of processes.

### cgroups v1: Per-Resource Hierarchies

Each resource gets its own hierarchy:

```
/sys/fs/cgroup/
├─ cpu/
│   └─ docker/
│       └─ container-xyz/
│           ├─ cpu.shares (CPU weight)
│           └─ cpu.cfs_quota_us (CPU time limit)
├─ memory/
│   └─ docker/
│       └─ container-xyz/
│           ├─ memory.limit_in_bytes (max memory)
│           └─ memory.usage_in_bytes (current usage)
└─ devices/
    └─ docker/
        └─ container-xyz/
            └─ devices.list (allowed devices)
```

**Multiple hierarchies** allow independent control. You can put a process in the CPU cgroup `/cpu/container-xyz` and memory cgroup `/memory/other-group` simultaneously.

**Limitation**: Complex to manage, inconsistent semantics across controllers.

### cgroups v2: Unified Hierarchy

Single unified hierarchy, all controllers:

```
/sys/fs/cgroup/
└─ docker/
    └─ container-xyz/
        ├─ cpu.max (CPU time limit)
        ├─ memory.max (max memory)
        ├─ io.max (I/O limits)
        └─ (all in one place)
```

**Advantages**: 
- Simpler configuration
- Consistent semantics
- Better nested cgroup support
- Lower CPU overhead

**Disadvantage**: Kubernetes/Docker still supporting both v1 and v2 during transition.

### Common Resource Limits

**memory.limit_in_bytes**: Maximum memory. Processes exceeding this are killed (OOM killer).

**cpu.cfs_quota_us / cpu.cfs_period_us**: CPU time limit. Quota per period. E.g., 50,000 µs per 100,000 µs period = 50% of 1 CPU.

**blkio.throttle.read_bps_device**: Block I/O read bandwidth limit.

**net_cls.classid**: Network traffic classification (for QoS).

## Overlay Filesystems: Layered Images

Container images are built from layers—each Dockerfile instruction creates a layer. At runtime, layers are combined using an **overlay filesystem**.

### OverlayFS (Linux Kernel Implementation)

```
Lower layer (read-only): base image
    ├─ bin/
    ├─ etc/
    └─ usr/

Upper layer (read-write): container changes
    ├─ etc/nginx.conf (modified)
    └─ var/log/ (new directory)

Merged view (what container sees):
    ├─ bin/ (from lower)
    ├─ etc/nginx.conf (from upper, shadowing lower)
    ├─ etc/ other files (from lower)
    ├─ usr/ (from lower)
    └─ var/log/ (from upper)
```

**Mechanism**:
- Read from lower or upper (upper takes precedence)
- Write goes to upper layer
- Delete is marked in upper as **whiteout** (special file)
- Multiple lower layers can be stacked

**Efficiency**: Large base layers (OS packages, Python interpreter) can be shared among containers. Only the upper layer (delta) is per-container.

### Whiteout Files

Deleting a file in the upper layer creates a whiteout marker; the original file in lower is not deleted.

```
Lower: /usr/local/bin/old-script
Upper whiteout: /usr/local/bin/old-script (special character device)
Result: Merged view shows file deleted
```

## seccomp-bpf: Syscall Filtering

**seccomp** (secure computing mode) restricts which syscalls a process can invoke. **seccomp-bpf** allows fine-grained filtering using Berkeley Packet Filter programs.

### Syscall Whitelist/Blacklist

Default Docker seccomp profile whitelists ~300 syscalls and blacklists dangerous ones (e.g., `kexec_load`, raw sockets for non-privileged containers).

```c
// Simplified: allow read, close; deny execve
struct sock_filter filter[] = {
    // Load syscall number into accumulator
    BPF_STMT(BPF_LD | BPF_W | BPF_ABS, ...),
    
    // If read (syscall 0) → ALLOW
    BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 1),
    BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
    
    // If close (syscall 3) → ALLOW
    BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 3, 0, 1),
    BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
    
    // Default → KILL
    BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL),
};
```

### Granular Rules

Seccomp can also filter by syscall arguments (e.g., allow `open()` only with certain flags, block `ptrace()` unless caller holds CAP_SYS_PTRACE`).

## pivot_root: Changing Root

**chroot()** changes a process's root directory but leaves `/proc`, `/sys` pointing to the host. **pivot_root()** atomically swaps root:

```bash
# Prepare new root
mkdir -p /new/root
cd /new/root

# Prepare old root storage (must be on different mount)
mkdir old_root

# Atomically swap
pivot_root . old_root

# Now:
# - New root is /
# - Old root (host root) is /old_root
# - Unmount old root to clean up
umount -l /old_root
```

**Container runtime** uses this to isolate the container's filesystem from the host.

## Virtual Ethernet Pairs (veth) and Container Networking

Each container connects to host networking via **veth pairs** (virtual Ethernet cable).

```
Host namespace          Container namespace
┌─ eth0 ←→ veth-ctr ─┬─ eth0
└─────────────────────┘
            ↓
    Host network stack
```

The container's eth0 is one end of the veth pair; the host sees the other end. When the container sends packets on eth0, they appear on the host-side veth. A bridge or routing rule decided where they go.

### Container Bridge

Multiple containers connect to a Linux bridge:

```
        ┌─── veth-ctr1 ←→ Container 1 eth0
        │
br-docker ─┼─── veth-ctr2 ←→ Container 2 eth0
        │
        └─── eth0 (host)
```

The bridge acts as a Layer 2 switch, forwarding frames between veth pairs and the host interface.

## Building a Container from Scratch

A minimal container requires:

1. **New namespaces**: `unshare(CLONE_NEWPID | CLONE_NEWNET | CLONE_NEWMNT | ...)`
2. **Mount a rootfs**: `mount()` or `pivot_root()` to isolate filesystem
3. **Set hostname**: `sethostname()`
4. **Configure network**: Create veth pair, move to container namespace
5. **Run init**: `exec /bin/bash` or similar
6. **Apply resource limits** (optional): Write to cgroup files
7. **Apply seccomp** (optional): Load seccomp filter

```c
// Pseudocode: minimal container
int child = fork();
if (child == 0) {
    unshare(CLONE_NEWPID | CLONE_NEWNET | CLONE_NEWMNT);
    chroot("/path/to/rootfs");
    sethostname("container");
    // ... configure network veth pair ...
    exec_init_process();
}
```

Modern runtimes (containerd, runc) handle this complexity, but the underlying mechanism is these kernel features.

## Container Security Considerations

**Privilege separation**: User namespaces map container root to unprivileged host user.

**Capability dropping**: Use `CAP_DROP` to remove unnecessary privs (e.g., drop CAP_SYS_ADMIN if not needed).

**seccomp filtering**: Reduce kernel surface exposed to container.

**Mount restrictions**: No write access to `/sys`, `/proc` from container (remount read-only).

**Network isolation**: Container can only reach networks explicitly connected (veth pairs).

**Cgroup limits**: Prevent container from consuming all host resources (DoS).

## See Also

- [OS Networking Stack](os-networking-stack.md) — network namespaces and veth pairs in detail
- [Security Hardening](security-container.md) — container security best practices
- [Containers & Orchestration](containers-orchestration.md) — Docker, Kubernetes overview