# Container Runtimes — OCI Spec, Low-Level & High-Level Runtimes, Isolation Models

## Overview

Container runtimes execute container processes on host systems. The stack has two governance layers: the **OCI Runtime Spec** (low-level spawning) and the **CRI** (Kubernetes Container Runtime Interface). Most users interact only with high-level runtimes like containerd or CRI-O, which orchestrate image management, network setup, and volume mounting before delegating to low-level runtimes for actual process execution.

The isolation model chosen fundamentally shapes security, overhead, and use cases: standard Linux containers (namespace isolation), VM-based (Kata), or user-space kernel (gVisor).

## OCI Runtime Specification

The Open Container Initiative Runtime Spec (currently v1.3) standardizes the interface for **low-level container runtimes** — tools that actually spawn and manage container processes. A conformant runtime accepts a bundle (filesystem + `config.json`) and executes it according to the spec.

### Bundle Format

```
bundle/
├── config.json          # OCI runtime config: namespaces, cgroups, mounts, hooks
├── rootfs/              # Container's root filesystem
├── state.json           # (populated by runtime) process state
└── container-id         # (populated by runtime) container ID file
```

### Key Configuration Elements

- **Namespaces**: pid, network, ipc, uts, user, mnt, cgroup
- **Cgroups**: resource limits (memory, CPU, I/O) via cgroup path or direct cgroupsv2
- **Mounts**: rootfs + bind mounts, mount options, read-only layers
- **User namespace remapping**: UID 0 in container → non-root host UID (key for rootless)
- **Hooks**: prestart, poststart, poststop for integration (e.g., Weave networking)
- **Linux-specific**: seccomp profiles, selinux context, capabilities, maskedpaths

Conformant runtimes: runc, crun, kata-runtime, gVisor runsc.

## Low-Level Runtimes

### runc

The reference OCI runtime, written in Go. Created by Docker, donated to OCI. Default for nearly all Linux container deployments.

**Architecture**: User-space daemon receives OCI bundle, re-execs itself into container namespace, sets up cgroups/seccomp/namespaces via C code, then execs the container's init process.

**Characteristics**:
- Full Linux namespace support (7 ns types)
- cgroup v1 + cgroup v2 support
- Seccomp profile enforcement
- Time overhead: ~100ms spawn
- Memory: ~10–15 MB per container

**Use case**: Standard Linux containers; acceptable for multi-tenant if seccomp/AppArmor/SELinux hardening is in place.

### crun

Alternative OCI runtime, written in C, maintained by Red Hat. Designed for speed and simplicity.

**Distinguishing traits**:
- ~10× faster startup (~10ms) than runc due to C implementation and less re-exec overhead
- Smaller memory footprint
- Drop-in replacement for runc
- Particularly strong on rootless container support

**When to choose crun**: Performance-sensitive workloads (serverless, short-lived functions), rootless environments, embedded systems. Both runc and crun can be used interchangeably; many distributions default to crun.

## High-Level Container Runtimes (CRI Interface)

The **Container Runtime Interface (CRI)** is a Kubernetes-specific plugin API. Kubelet communicates with a CRI implementation to start, stop, and manage pods. A CRI runtime handles image pull, volume setup, network configuration, then delegates to an OCI runtime.

### containerd

Graduated as CRI-compliant daemon since Docker 1.13. Originally extracted from Docker to be language-agnostic.

**Stack**:
- CRI plugin communicates with kubelet
- containerd daemon manages images, containers, snapshots
- OCI shim bridges to runc/crun/kata

**Strengths**:
- Mature, stable, industry standard
- Lazy image pulling support (eStargz)
- Snapshot plugins for efficient storage layering
- Broad ecosystem integration (Kubernetes, Docker, systemd, etc.)

**Namespace/storage**: `/run/containerd` for socket, `/var/lib/containerd` for storage, `/etc/containerd/config.toml` for configuration.

### CRI-O

A Kubernetes-native CRI runtime. Designed to be lightweight: only implement what Kubernetes needs, skip Docker compatibility.

**Attributes**:
- ~20% smaller memory footprint than containerd
- Simplicity-first design (no multi-purpose features)
- Strong Red Hat/Fedora backing
- Excellent cgroup v2 support

**Use case**: Kubernetes-only deployments where Docker compatibility isn't needed; resource-constrained environments (edge, embedded).

### Trade-offs

| Feature                | containerd | CRI-O |
|------------------------|-----------|-------|
| Ecosystem | Broad (Docker, Kubernetes, k3s) | Kubernetes-focused |
| Memory | ~80 MB | ~60 MB |
| Image speed | Fast, lazy pull support | Conservative |
| maturity | More widely deployed | Production-ready, fewer users |

## Specialized Isolation Models

### Kata Containers (VM-Based)

Lightweight virtual machines masquerading as containers. Each pod/container gets its own minimal hypervisor (QEMU, KVM, or Hyper-V).

**Architecture**: OCI shim spawns a minimalist Linux guest (5–30 MB), then runc runs inside the guest. From kubelet perspective, it's a normal container.

**Security model**: Hardware-enforced isolation. Kernel exploit in container cannot break host kernel; network namespace escape impossible.

**Trade-offs**:
- Overhead: ~50 MB memory, ~200ms startup (vs. runc's ~100ms)
- Better tenant isolation for multi-tenant SaaS
- Use case: Untrusted workloads, FaaS, strong compliance requirements
- Tradeoff: Reduced performance, compatibility (some syscalls may not be available in guest)

### gVisor (User-Space Kernel)

Syscall interception in user-space. When container makes a syscall, gVisor's user-space kernel (written in Go) intercepts and serves it.

**Architecture**: OCI shim runsc starts a gVisor Sentry (the user-space kernel). Container's syscalls don't hit host kernel; Sentry handles them, translating to host syscalls as needed to complete the operation.

**Security model**:
- Reduces kernel attack surface (fewer syscalls reach host kernel)
- Sandbox-escape prevention via syscall filtering
- Not true VM isolation but strong container isolation

**Characteristics**:
- Memory: ~50 MB per container
- Startup: ~150ms (slower than runc, faster than Kata)
- Overhead: ~10–20% CPU on syscall-heavy workloads
- Compatibility: Subset of syscalls (covers ~90% of workloads, misses esoteric ones)

**Use case**: Multi-tenant clusters where you want container isolation without VM overhead; Google Cloud Run uses gVisor.

### Comparison

| Model | Isolation | Speed | Memory | Compatibility |
|-------|-----------|-------|--------|---|
| runc (namespace) | Software | Fast | Low | Full POSIX |
| Kata (VM) | Hardware | Slow | Medium | Full, with overhead |
| gVisor (user-space kernel) | Syscall interception | Medium | Medium | ~90% |

## Container Lifecycle

The OCI runtime spec defines a state machine:

```
[created] → [running] → [paused] → [running] → [stopped]
  ↓                                              ↓
  └─────────────────────────────────────────────┘
```

Key operations:
- `create`: Allocate namespaces, cgroups, mounts; do not start init process
- `start`: Execute init process in prepared container
- `pause` / `resume`: Use cgroup freezer to suspend / resume (useful for checkpoint/restore)
- `delete`: Clean up cgroups, namespaces, mounts

Hooks allow external integration:
- `prestart`: Before container starts (e.g., setup networking plugin)
- `poststart`: After container started (e.g., log container start event)
- `poststop`: After container stopped (e.g., cleanup networking)

## Image Building Tools

Building container images outside Docker (which requires a daemon and privileges).

### BuildKit

Docker's official next-generation builder (since Docker 18.09). Written in Go.

**Key features**:
- Parallel layer building (exploit DAG to build independent layers concurrently)
- Cache mounts (preserve build caches across builds without re-layering)
- Session isolation (temporary files don't leak between builds)
- Multi-stage secrets (don't include secrets in final image)
- Rootless builder support (run as non-root; requires user namespaces)

```dockerfile
# syntax=docker/dockerfile:1.4
FROM golang:1.21 AS builder
RUN --mount=type=cache,target=/go/pkg/mod go build -o app .

FROM alpine
COPY --from=builder /app /app
```

BuildKit is the future; Docker and containerd both integrate it.

### Kaniko

Google's image builder for Kubernetes. Builds images in unprivileged containers (no DinD, no Docker daemon required).

**Mechanism**: Parses Dockerfile, executes each layer as a separate command, snapshotting filesystem changes. All layers happen in user-space; no privileged operations.

**Strengths**:
- Runs in unprivileged pods: `runAsNonRoot: true`
- No Docker daemon or socket required
- Fast with proper caching

**Limitation**: Slower than BuildKit on parallel builds; simpler feature set.

**Use case**: In-cluster image building in strict security contexts (no privilege escapes, audit compliance).

### Buildah

Red Hat's image building tool. Focuses on building OCI images without a daemon.

**Philosophy**: Low-level image construction. Treat Buildah like a shell for building images manually.

```bash
ctr=$(buildah from alpine:latest)
buildah run $ctr apk add --no-cache curl
buildah config --entrypoint '["/bin/sh"]' $ctr
buildah commit $ctr my-image
```

**Advantages**:
- Fine-grained control over each layer
- Runs rootless
- No Docker compatibility overhead

**Use case**: Scripted, custom image workflows; advanced use cases where Dockerfile is too rigid.

## Rootless Containers

Running containers without root privileges on the host. Strengthens isolation and reduces privilege escalation surface.

### User Namespace Remapping

The core mechanism: map container UID 0 to a non-root host UID (e.g., UID 100000–165535).

```
Container UID 0  →  [User Namespace]  →  Host UID 100000
Container UID 1  →                    →  Host UID 100001
```

If container is compromised and escapes to host, the attacker lands as UID 100000 (unprivileged on host), not root.

### Rootless runc / crun

Both support user namespace mode via `--rootless` flag or `rootless` config option.

```bash
runc --rootless run my-container
```

**Setup requirements**:
- User namespace support in kernel (Linux 3.10+)
- `/etc/subuid` and `/etc/subgid` entries (e.g., `user:100000:65536`)
- Networking via slirp4netns (user-space TAP) for non-privileged tap

**Trade-offs**:
- Network performance reduced ~10–20% (user-space networking overhead)
- Certain capabilities unavailable (CAP_NET_BIND_SERVICE requires special bridging)
- UID remapping adds complexity but significantly hardens multi-tenant scenarios

### Rootless containerd / CRI-O

Both support rootless mode; full Kubernetes support. Often paired with rootless BuildKit for end-to-end unprivileged workflows.

**Use case**: Multi-tenant Kubernetes clusters, untrusted workloads, security-first SaaS platforms.

## Architecture Decisions

**Single runtime vs. multiple**: Most clusters use one default runtime (containerd or CRI-O) but can mix per-node or per-workload (e.g., gVisor for untrusted, runc for performance-critical).

**VM-based isolation costs**: Kata trades startup time and memory for hard kernel isolation. Worth it for FaaS or compliance-driven multi-tenant. Not worth it for high-frequency, long-lived workloads.

**Rootless overhead**: Typically 5–15% on network-heavy workloads. Usually acceptable for the security gain in untrusted environments; skip for performance-critical pods.

**BuildKit vs. Kaniko**: BuildKit for local/CI with daemon; Kaniko for in-cluster unprivileged builds. Both are solid; choose by environment constraints, not perceived quality.

## See Also

- [Kubernetes Container Runtimes](https://kubernetes.io/docs/setup/production-environment/container-runtimes/)
- [security-container](security-container.md) — container security scanning and runtime policies
- [containers-orchestration](containers-orchestration.md) — Docker, Kubernetes overview
- [architecture-resilience](architecture-resilience.md) — sandboxing & fault isolation strategies