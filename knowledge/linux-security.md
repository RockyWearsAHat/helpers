# Linux Security — Capabilities, seccomp, AppArmor, SELinux, Audit, PAM, SSH, Permissions & Firewalls

## Overview

Linux security spans multiple overlapping layers: **discretionary access control (DAC)** via file permissions and ACLs, **mandatory access control (MAC)** via SELinux or AppArmor, **capability-based privilege restriction**, **system call filtering** via seccomp, **audit logging**, and **authentication/authorization** via PAM and sudo. Understanding these mechanisms and their interactions is essential for hardening systems, configuring containers, and implementing defense-in-depth. This note covers the landscape of Linux security primitives and best practices.

## File Permissions and Discretionary Access Control

### Traditional Permissions: User, Group, Other

Every file has an owner (user), group, and three-bit mode for each category:

```
-rw-r--r-- alice staff 4096 Nov 18 12:34 config.txt
```

Bits:
- **Owner (user)**: rw- (6) = read + write
- **Group**: r-- (4) = read only
- **Other**: r-- (4) = read only

Directories use the same bits with different semantics: **x** (execute) means "traverse" (access contents).

**Limitations:**
- Can only deny access to groups or "other"; cannot specify per-user rules beyond owner
- Can only restrict, not grant selectively within a group
- No delegation: only owner can change permissions

### Setuid and Setgid Bits

Special bits allow privilege escalation:

- **Setuid (mode 04000)**: Binary runs with owner's UID, not the caller's. Example: `sudo` binary is setuid root. Dangerous: any vulnerability could escalate privileges.
- **Setgid (mode 02000)**: Binary runs with the group's GID. Directories with setgid cause newly created files to inherit the directory's group (useful for shared directories).
- **Sticky bit (mode 01000)**: On a directory, only owner (or root) can delete files, not just anyone with write permission. Used on `/tmp` to prevent users deleting each other's files.

Example: `sudo` permission check is done inside sudo; malicious sudo binary would be catastrophic.

### ACLs (Access Control Lists)

POSIX ACLs extend permissions with per-user and per-group rules:

```bash
setfacl -m u:bob:rw config.txt
getfacl config.txt
```

Bob can read/write, even if not the owner and not in the group. ACLs are stored as extended attributes on the filesystem; not all filesystems support them.

## Linux Capabilities

### Motivation

Traditionally, Unix uses two privilege levels: unprivileged (UID > 0) and root (UID 0). Root can do anything; non-root cannot. This is coarse-grained: a web server running as root can read all user data, kill other processes, etc.

**Capabilities** divide root's powers into fine-grained privileges. A process can have a subset of capabilities without full root.

### Capability Set

Each process has three capability sets:

- **Permitted (P)**: Capabilities the process can use.
- **Effective (E)**: Capabilities currently active (can be a subset of P).
- **Inheritable (I)**: Capabilities passed to child processes on exec.

On file, additional bits:

- **Permitted (fp)**: Capabilities the file grants if executed.
- **Effective (fe)**: Capabilities automatically enabled on exec.
- **Inheritable (fi)**: Capabilities inherited by child.

### Common Capabilities

- **CAP_NET_BIND_SERVICE**: Bind to port < 1024
- **CAP_NET_ADMIN**: Network admin (configure interfaces, routing)
- **CAP_SYS_ADMIN**: System administration (broad power, includes mount, namespace creation)
- **CAP_SYS_PTRACE**: Attach debugger to processes
- **CAP_SETUID / CAP_SETGID**: Change UID/GID
- **CAP_DAC_OVERRIDE**: Bypass file permission checks
- **CAP_SYS_MODULE**: Load kernel modules
- **CAP_KILL**: Send signals to other processes

Full list: `man 7 capabilities` (41 capabilities in recent kernels).

### Usage

Drop unnecessary capabilities in a service:

```bash
setcap cap_net_bind_service=ep /usr/bin/myapp
getcap /usr/bin/myapp
```

Or in systemd:

```ini
[Service]
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
```

Process runs with only CAP_NET_BIND_SERVICE; cannot read arbitrary files, kill other processes, etc. Limits blast radius if pwned.

## seccomp-bpf (Secure Computing Mode)

### Motivation

Even with capability restriction, a process can call harmful syscalls. seccomp allows filtering which system calls are allowed.

### seccomp-bpf (mode 2)

**seccomp-bpf** (Berkley Packet Filter mode) is the modern seccomp implementation. A userland program loads a BPF filter into the kernel. The kernel runs the filter for each syscall, allowing/denying based on the result.

**Filter example (pseudocode):**
```
Load syscall number into register
If syscall == SYS_open: allow
If syscall == SYS_openat: allow
If syscall == SYS_read: allow
If syscall == SYS_write: allow
Else: deny (SIGSYS)
```

**Practical usage with libseccomp:**
```c
#include <seccomp.h>
scmp_filter_ctx ctx = seccomp_init(SCMP_ACT_KILL);
seccomp_rule_add(ctx, SCMP_ACT_ALLOW, SCMP_SYS(read), 0);
seccomp_rule_add(ctx, SCMP_ACT_ALLOW, SCMP_SYS(write), 0);
seccomp_load(ctx);
```

After `seccomp_load()`, only read/write syscalls are allowed; all others trigger SIGSYS and kill the process.

**systemd seccomp:**
```ini
[Service]
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM
```

Denied syscalls return EPERM instead of SIGSYS (process continues but call fails). Preset `@system-service` includes common syscalls allowed for most services.

**Overhead:** Minimal (a few cycles per syscall for filter evaluation).

## AppArmor vs. SELinux

Both provide **Mandatory Access Control (MAC)**: the kernel (not the user/owner) enforces policy, regardless of DAC permissions.

### AppArmor

**AppArmor** (Canonical, used on Ubuntu) profiles processes by name. A profile specifies what files, capabilities, and signals a process can access.

**Profile example:**
```
/usr/bin/myapp {
  /etc/myapp.conf r,
  /var/log/myapp.log w,
  /tmp/** rw,
  deny /etc/shadow r,
  caps NET_BIND_SERVICE,
}
```

`myapp` can read `/etc/myapp.conf`, write to `/var/log/myapp.log`, read/write `/tmp/`, but cannot read `/etc/shadow`, and has only CAP_NET_BIND_SERVICE.

**Workflow:**
1. Load profile into kernel: `apparmor_parser -r /etc/apparmor.d/myapp`
2. Kernel enforces policy for all processes matching the profile

**Advantages:**
- Path-based (intuitive; administrators think in terms of files)
- Simple syntax
- Faster implementation

**Disadvantages:**
- Same binary name matches same profile (less flexible for multiple contexts)
- Per-binary focus (less system-wide control)

### SELinux

**SELinux** (NSA, used on Red Hat/CentOS) uses **type enforcement**: everything (files, processes, sockets) has a security context (SELinux type). Policy rules define which transitions are allowed.

**Context format:** `user:role:type:level`
Example: `system_u:system_r:httpd_t:s0`

**Policy rule:**
```
allow httpd_t httpd_sys_rw_content_t:file { read write };
```

httpd processes (type httpd_t) can read/write files tagged httpd_sys_rw_content_t.

**Workflow:**
1. Label files: `chcon -t httpd_sys_rw_content_t /var/www/html`
2. Load policy: `semodule -i policy.pp`
3. Kernel enforces transition rules

**Advantages:**
- Type-based (more expressive; same binary in different roles has different context)
- System-wide policy (coherent across all processes)
- Fine-grained resource controls

**Disadvantages:**
- Steep learning curve (more complex language)
- Harder debugging (denied access can be cryptic without audit logs)
- More overhead than AppArmor

**Note:** AppArmor and SELinux can coexist but are often mutually exclusive per distro (Ubuntu ships AppArmor; Red Hat ships SELinux).

## Audit Framework

The **audit framework** records security-relevant events (file access, syscall invocation, policy decisions) to a kernel buffer, then writes to userland via `auditd` daemon.

### Audit Rules

```bash
auditctl -w /etc/shadow -p wa -k file_access
auditctl -a exit,always -F arch=b64 -S execve -F uid=0 -k exec_as_root
```

First rule: monitor `/etc/shadow` for write/attribute changes, tag with key `file_access`. Second rule: log all execve syscalls by root (UID 0).

### Audit Log Analysis

```bash
auditctl -l                       # List active rules
ausearch -k file_access           # Find logged events by key
```

**Output example:**
```
type=EXECVE msg=audit(...): argc=3 a0="/bin/bash" a1="-c" a2="echo hello"
type=PROCTITLE msg=audit(...): proctitle=/bin/bash -c echo hello
```

**Use cases:**
- Compliance (HIPAA, PCI DSS require audit trails)
- Intrusion detection (detect suspicious syscall patterns)
- Debugging policy violations (why was access denied? audit log shows the decision)

## PAM (Pluggable Authentication Modules)

**PAM** is a framework for authentication and session management. Instead of hardcoding password checks, services call PAM, and PAM loads configurable modules (pam_unix, pam_ldap, pam_google_authenticator, etc.).

### PAM Configuration

`/etc/pam.d/sshd`:
```
auth       required     pam_unix.so nullok try_first_pass
auth       required     pam_google_authenticator.so
session    required     pam_limits.so
session    required     pam_unix.so
```

Rules are processed top-down. Each rule specifies:
- **Interface**: auth (authentication), account (account validity), session (session setup), password (password change)
- **Control**: required (must succeed), requisite (must succeed, stop if fail), sufficient (if success, no more checks), optional
- **Module**: .so file implementing the check
- **Arguments**: module-specific options

On SSH login, PAM checks unix password (pam_unix) then requires 2FA (pam_google_authenticator), then sets up session limits.

### Common Modules

- **pam_unix.so**: Plain /etc/passwd authentication
- **pam_ldap.so**: LDAP directory authentication
- **pam_google_authenticator.so**: TOTP 2FA
- **pam_limits.so**: Set ulimits (max open files, processes)
- **pam_tally2.so**: Lock account after failed attempts

## sudo Configuration

**sudo** (superuser do) allows unprivileged users to run commands as another user (usually root).

### Process

1. User runs: `sudo /bin/bash`
2. sudo is setuid root; it prompts for the user's password
3. sudo checks /etc/sudoers (must not be world-writable)
4. If allowed, sudo execs `/bin/bash` as root

### sudoers Policy

`/etc/sudoers` (edited with `visudo` for syntax checking):

```
alice ALL=(ALL) NOPASSWD: /usr/bin/reboot
bob ALL=(root) /tmp/cleanup.sh

# Group syntax
%wheel ALL=(ALL) ALL

# Defaults
Defaults env_reset
Defaults lecture="never"
```

alice can run `/usr/bin/reboot` as any user without a password. bob can run `/tmp/cleanup.sh` as root (password required). wheel group members can run anything as root.

**Audit:** sudo logs to syslog; check `/var/log/auth.log` (Debian) or `/var/log/secure` (Red Hat).

## SSH Hardening

### Key Concepts

SSH uses **public-key cryptography**. Server has a host key (private). Client trusts the server's public key (usually via `~/.ssh/known_hosts`). User authentication is typically password or public key.

### Best Practices

```
# /etc/ssh/sshd_config
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes
AuthorizedKeysFile .ssh/authorized_keys
X11Forwarding no
MaxAuthTries 3
LoginGraceTime 30s
PermitEmptyPasswords no
```

- **PermitRootLogin no**: Force use of sudo, not direct root SSH. Auditable, granular.
- **PasswordAuthentication no**: Require public key (immune to brute force, keylogging).
- **X11Forwarding no**: Disable X11 tunnel (attack surface).
- **MaxAuthTries 3**: Fail fast on brute-force attempts.

### User Public Keys

User creates a keypair: `ssh-keygen -t ed25519`. Private key stays on user's machine; public key is appended to `~/.ssh/authorized_keys` on the server.

On login, server challenges client to prove ownership of private key (zero-knowledge proof). Password is never sent.

## Firewall: nftables and firewalld

### nftables

See **linux-networking.md** for detailed coverage. Modern packet filtering with efficient set-based rules, atomic reload.

### firewalld

**firewalld** (Red Hat/CentOS) is a dynamic firewall daemon. Instead of static iptables/nftables rules, firewalld manages **zones** and **services**.

```bash
firewall-cmd --zone=public --permanent --add-service=http
firewall-cmd --permanent --add-port=8080/tcp --zone=public
firewall-cmd --reload
```

Zones (public, private, dmz, etc.) have different default policies. Services are predefined port/protocol combinations. Advantages: no downtime for rule changes; easier for non-experts. Disadvantage: less fine-grained control than raw nftables.

## CIS Benchmarks and Best Practices

**CIS (Center for Internet Security) Benchmarks** are community-developed hardening guidelines for systems and software. Each benchmark is a numbered control with a statement and rationale.

**Example controls (Linux):**
- Remove unnecessary packages
- Disable unnecessary services
- Set file permissions on sensitive files (600 or more restrictive)
- Set kernel parameters (sysctl) for hardening
- Configure sudo for logging and restrictions
- Enforce strong password policy (pam_pwquality)
- Restrict SSH access (AllowUsers, DenyUsers)
- Enable audit logging

Tools like **lynis** (open source) scan a system and report CIS benchmark compliance gaps. Adoption varies by risk profile; high-security environments (PCI DSS) require strict CIS compliance.

## Principle of Least Privilege

Least privilege is the overarching security principle:

- Drop capabilities not needed for a service to function
- Use seccomp to restrict syscalls
- Apply AppArmor/SELinux to confine file access
- Run services as unprivileged users
- Use read-only root filesystems in containers
- Disable SSH root login; require sudo
- Audit all sensitive actions

Compromise of one narrow-privilege service does not gain attacker full system control.

## See Also

- **linux-systemd.md** — systemd security hardening (ProtectSystem, NoNewPrivileges)
- **linux-networking.md** — Firewall and netfilter rules
- **security-container.md** — Container runtime security (similar primitives)
- **security-compliance-frameworks.md** — SOC 2, PCI DSS, HIPAA requirements