# Linux systemd — Unit Files, Dependencies, Resource Control, Socket Activation & Security

## Overview

**systemd** is the init system and system manager on modern Linux distributions. It replaces SysV init with a declarative, dependency-based model for managing services, sockets, timers, and mounts. systemd enforces resource limits via cgroups v2, provides a unified logging backend (journald), and implements socket activation for efficient service startup. Understanding systemd unit files, dependency relationships, and security hardening options is essential for writing robust services, containers, and embedded systems.

## Unit Files and Types

### The Unit Framework

systemd organizes system components into **units**, each described by a file in `/etc/systemd/system/`, `/run/systemd/system/`, or packaged distributions like `/usr/lib/systemd/system/`. Each unit file (e.g., `nginx.service`) declares type, configuration, and dependencies.

**Unit types:**
- **.service**: A daemon or process (e.g., web server, database).
- **.socket**: A listening socket (TCP, UDP, Unix) that triggers service activation.
- **.timer**: A scheduled task (cron-like). Can be transient.
- **.mount**: A filesystem mount point.
- **.target**: A logical grouping (e.g., `multi-user.target` represents "system ready for multi-user login").
- **.path**: Triggered when a file/directory changes.

### Service Unit Anatomy

A typical service file:

```ini
[Unit]
Description=My Web Service
After=network.target
Wants=my-config.service

[Service]
Type=simple
ExecStart=/usr/bin/myapp --config /etc/myapp.conf
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

**[Unit] section:**
- **Description**: Human-readable name.
- **After / Before**: Ordering constraints (After=network.target means "start after networking is up").
- **Requires / Wants**: Dependencies. Requires stops this unit if the dependency fails; Wants continues even if the dependency fails.
- **Conflicts**: Units that should not run simultaneously.

**[Service] section:**
- **Type**: Specifies how systemd determines the service is "started":
  - `simple`: ExecStart process runs in foreground; systemd considers it started when fork completes. Default.
  - `forking`: ExecStart forks a child; systemd waits for child to exit, then considers service started. Used by daemons that detach.
  - `oneshot`: ExecStart runs once; service exits after completion. Used for one-time setup tasks.
  - `dbus`: Service is started when it appears on D-Bus.
  - `notify`: Service sends a readiness notification via sd_notify() (see below).
- **ExecStart / ExecStop / ExecReload**: Commands to run.
- **Restart**: Policy (on-failure, always, on-abnormal). Automatically restart if the process exits with a non-zero code or signal.
- **RestartSec**: Delay before restarting.
- **User / Group**: UID/GID to run as.
- **Environment / EnvironmentFile**: Set environment variables.

**[Install] section:**
- **WantedBy**: Which targets or services should want this unit. `systemctl enable` creates a symlink in the WantedBy target's `.wants/` directory.

### Type=notify and sd_notify()

For services where determining startup completion is complex (e.g., web server binding to multiple ports), use **type=notify**. The service calls `sd_notify("READY=1")` when ready. systemd waits for this signal before considering the service fully started (timeout: `TimeoutStartSec`).

```c
#include <systemd/sd-daemon.h>
// ... setup code ...
sd_notify(0, "READY=1");
pause();  // or main event loop
```

## Dependencies and Ordering

### Dependency Relationships

- **After / Before**: Ordering constraints only. A→After B means "start A after B", but doesn't require B to start. B can fail, and A still starts.
- **Wants**: Soft dependency. Systemd starts the wanted unit, but if it fails, the dependent unit still runs. Commonly used for optional services.
- **Requires**: Hard dependency. If the required unit does not start or stops, the dependent unit stops. Creates mutual enforcement.
- **BindsTo**: Stronger than Requires. If the bound-to unit stops, the binding unit also stops immediately.

**Example:** A web app service:
```ini
After=postgresql.service networking.target
Wants=redis-server.service
Requires=ssl-certificate.mount
```

Interpretation: Start after PostgreSQL and networking are ready. Try to start Redis (but continue if it fails). Mount SSL certificates before starting (fail if mount fails).

### Cycle Detection

systemd builds a dependency graph and detects cycles, partially ordering units topologically. Circular dependencies are errors and prevent startup.

## Resource Control via cgroups

### cgroups Architecture

systemd automatically creates a **cgroup** (control group) for each service, allowing resource limits on CPU, memory, I/O, and devices.

**Access:**
```bash
systemctl set-property myservice.service MemoryLimit=512M
systemctl set-property myservice.service CPUQuota=50%
systemctl daemon-reload
```

Or in the unit file:

```ini
[Service]
MemoryLimit=512M
CPUQuota=50%
TasksMax=100
IOWeight=50
```

**Common limits:**

- **MemoryLimit / MemoryMax**: Maximum memory allowed. Exceeding triggers OOM killer. Note: this is RSS, not VSZ.
- **CPUQuota**: Max CPU (e.g., 50% = max 1 core on dual-core). Implemented via cpu cgroup v2.
- **TasksMax**: Maximum number of processes/threads.
- **IOWeight / IOReadBandwidthMax / IOWriteBandwidthMax**: Disk I/O limits (requires cgroup v2).
- **DevicePolicy**: Restrict device access (DeviceAllow, DeviceDeny).

### Memory and OOM Behavior

When a cgroup exceeds MemoryLimit, the kernel's OOM killer selects a process within the cgroup and sends SIGKILL. systemd can restart the service:

```ini
Restart=on-failure
OOMPolicy=restart
```

If the cgroup consistently hits the limit, the service thrashes and systemd eventually stops restarting (RestartForceExitStatus=137 for SIGKILL).

## journald: Centralized Logging

### journald

**journald** is systemd's logging daemon. All processes' stdout, stderr, and syslog messages are captured (if connected to a terminal, also printed there). Logs are stored in structured binary format in `/var/log/journal/` or `/run/log/journal/`.

**Query logs:**
```bash
journalctl -u nginx.service -n 50 --since today
journalctl -p err   # Error and higher severity
journalctl -x       # Show explanatory messages (if available in catalog)
```

**Benefits:**
- Unified logging: no separate syslog, app logs, stderr.
- Structured fields: journalctl can filter by field (user, unit, exit code).
- Binary format: compact, preserves field types (e.g., timestamps as epoch, not strings).
- Rate limiting: protects against log bombs (defaults: 1000 msgs/10s per unit).

**Downsides:**
- Binary format not human-readable directly; requires journalctl.
- Retention: volatile journal (lost on reboot) in some distros; persistent journal requires explicit config.

### Log Levels

Seven levels (DEBUG, INFO, NOTICE, WARNING, ERR, CRIT, ALERT, EMERG) correspond to syslog severity. journalctl filters by minimum level: `-p warning` shows WARNING and above.

## Socket Activation

### Motivation

Traditionally, a server starts at boot and listens on a port. If the server is rarely used, it consumes memory and CPU idle. **Socket activation** delays service startup until a connection arrives.

### Mechanics

1. systemd creates and binds the listening socket (defined in a `.socket` unit).
2. When a connection arrives, systemd starts the associated `.service` unit and passes the socket file descriptor.
3. The service inherits the socket (via file descriptor inheritance) and accepts connections.

**Socket unit example:**
```ini
[Unit]
Description=My Service Socket
Before=my-service.service

[Socket]
ListenStream=0.0.0.0:9000
Accept=no

[Install]
WantedBy=sockets.target
```

**Service unit:**
```ini
[Unit]
Description=My Service

[Service]
Type=simple
ExecStart=/usr/bin/myapp
Sockets=my-service.socket

[Install]
Also=my-service.socket
```

When the socket receives a connection, systemd starts my-service.service and passes fd=3 (the listening socket). The service calls `accept(3)` to handle the connection.

**Accept=yes** variant: systemd accepts the connection and passes the connected socket to the service (one connection per invocation). Used for inetd-style daemons.

## Transient Units

**Transient units** are created at runtime via `systemctl` or D-Bus, exist only in memory, and are garbage-collected when the system restarts or the unit is stopped. Useful for one-shot tasks or temporary services:

```bash
systemctl start --create-dbus-service --transient my-task.service \
    --exec /usr/bin/my-script arg1 arg2
```

The unit is not persisted to disk; it lives in `/run/systemd/transient/`.

## Security Hardening

### ProtectSystem and ProtectHome

- **ProtectSystem=strict**: Root filesystem is read-only. `/etc`, `/usr`, `/boot` cannot be modified. Service can only write to directories explicitly allowed (via ReadWritePaths or tmpfs mounts). Prevents accidental corruption or privilege escalation via filesystem modification.
- **ProtectHome=yes**: User home directories are inaccessible (mounted read-only or hidden).

Used together: `ProtectSystem=strict ProtectHome=yes` confines the service to a minimal writable environment (e.g., `/tmp`, `/var/empty`).

### NoNewPrivileges

```ini
NoNewPrivileges=yes
```

Sets the `no_new_privs` prctl flag. Prevents the process (and children) from gaining additional capabilities via `setuid` binaries or file capabilities. If the service is compromised, an attacker cannot escalate privileges via a setuid helper.

### Capability Restrictions

Drop unnecessary Linux capabilities:

```ini
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
SecureBits=keep-caps-locked
```

**CAP_NET_BIND_SERVICE**: Allows binding to ports < 1024. Most services don't need other capabilities. Dropping CAP_SYS_ADMIN (system admin), CAP_SYS_MODULE (load modules), etc., limits damage if the service is pwned.

### Private Directories

```ini
PrivateTmp=yes
PrivateDevices=yes
PrivateNetwork=yes
```

- **PrivateTmp=yes**: Service sees a private tmpfs `/tmp`, isolated from other processes' `/tmp`.
- **PrivateDevices=yes**: Only `/dev/null`, `/dev/zero`, `/dev/random` are accessible; `/dev/sda`, `/dev/mem` hidden.
- **PrivateNetwork=yes**: Service has its own network namespace. Only loopback accessible unless ExecAddMemberships brings it into another namespace.

### DynamicUser

```ini
DynamicUser=yes
```

systemd allocates a transient UID for the service (if User= is not set). No persistent user entry; UID is recycled after the service stops. Reduces attack surface (no actual user account).

### RestrictAddressFamilies and SystemCallFilter

Restrict socket families and system calls:

```ini
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM
```

Service cannot create IPv6 sockets if AF_INET6 is omitted. SystemCallFilter=@system-service is a preset; denied syscalls return EPERM.

## networkd: Network Configuration

**systemd-networkd** is a network configuration daemon. Instead of `/etc/network/interfaces` (Debian) or `/etc/sysconfig/network-scripts` (RedHat), networkd reads `.network` and `.link` files:

```ini
[Match]
Name=eth0

[Network]
DHCP=yes
Address=10.0.0.1/24
DNS=8.8.8.8
```

Advantages: declarative, idempotent, works with systemd-resolved for DNS. Used in systemd-based distributions and containers.

## resolved: DNS Resolution

**systemd-resolved** replaces `/etc/resolv.conf`. It provides DNS caching, DNSSEC validation, and multi-NIC resolution.

Query status:
```bash
resolvectl
```

## See Also

- **os-process-management.md** — Process lifecycle and signals
- **linux-security.md** — Capabilities and security isolation
- **devops-kubernetes.md** — systemd in container orchestration (kubelet)
- **infrastructure-container-networking.md** — Network namespaces and veth