# System Boot Process — BIOS, UEFI, Bootloaders, and Init Systems

## Overview

The **system boot process** transforms a powered-off computer into a running operating system. From firmware initialization (BIOS/UEFI) through bootloader (GRUB/systemd-boot), kernel loading, initramfs, and init system (systemd/OpenRC), each stage performs essential setup: hardware discovery, secure verification, device initialization, and transition to userspace. Understanding this chain is essential for system administration, kernel development, and debugging boot issues.

## Power-On & Firmware: POST and BIOS vs. UEFI

When power is applied, the CPU fetches the first instruction from a fixed address (e.g., 0xFFFF0000 on x86), which points to **firmware**.

### BIOS (Basic Input/Output System, Legacy 1981-present)

Older firmware standard, still present on many systems:

```
BIOS Boot Sequence (x86):
1. Power-On Self Test (POST): Initialize CPU, RAM, basic hardware
   - Memory test, CPU self-check, interrupt tables
2. Enumerate hardware: Scan PCI bus for devices (NIC, GPU, disk controllers)
3. Search for bootable devices (first disk with bootable partition flag)
4. Load bootloader: Read first block (MBR, sector 0) from disk into RAM at 0x7C00
5. Jump to bootloader code
6. Bootloader responsibility: Locate kernel, set up environment, jump to kernel
```

**Legacy limitations:**
- 16-bit real mode (1 MB addressable memory)
- MBR partitions limited to 2 TB
- No cryptographic verification (no secure boot chain)
- No built-in network boot beyond PXE
- Firmware not standard (each BIOS vendor, different quirks)

### UEFI (Unified Extensible Firmware Interface, Modern 2005-present)

Modern standard, replacing BIOS:

```
UEFI Boot Sequence (x86-64):
1. SEC Phase (Security): Initialize CPU, memory protection, TPM (if present)
2. PEI Phase (Pre-EFI Init): Basic platform initialization, RAM up
3. DXE Phase (Driver Execution Environment):
   - Load platform drivers (chipset, NIC, storage)
   - Enumerate hardware (PCI, USB)
4. BDS Phase (Boot Device Select):
   - Display boot menu
   - Locate bootloader candidates on EFI System Partition (ESP, typically FAT32)
   - Load bootloader (typically systemd-boot or GRUB2)
5. Jump to bootloader
6. BDS Phase continues (if bootloader hands control back)
```

**Improvements:**
- 64-bit, no real-mode limitations
- GPT partitions support 8 ZB
- Secure Boot chain: Verify bootloader certificate before executing
- Measured Boot: Log all early boot measurements to TPM
- Standard firmware API (accessible to bootloaders via EFI Services)
- Built-in network boot, USB support

**Secure Boot:** UEFI firmware checks bootloader's UEFI Secure Boot certificate. If unsigned or wrong cert → refuse boot. Protects against bootkit/rootkit installation before OS loads.

## Bootloader: GRUB vs. systemd-boot

The bootloader:
1. Outputs boot menu (select OS, kernel parameters)
2. Locates kernel binary and initramfs on disk
3. Loads both into RAM
4. Sets up environment (memory map, command-line arguments)
5. Hands control to kernel

### GRUB2 (GNU GRand Unified Bootloader)

Most widely used bootloader, supports BIOS and UEFI:

```
GRUB2 stages:
  boot.img (512 bytes, MBR or PBR)
    ↓ (loads)
  core.img (compressed, ~40 KB, on disk after MBR, often in GPT gap)
    ↓ (loads)
  Normal module (modules/normal.mod, from /boot/grub)
    ↓ (loads)
  Full GRUB2 environment: menus, filesystem support, pre-boot scripts

User sees: GRUB menu, time-limited countdown, edit kernel parameters

Recovery: Press 'e' to edit, edit cmdline/kernel parameters, boot modified
```

**Configuration**: `/boot/grub/grub.cfg` (auto-generated, do not edit directly). Edit `/etc/default/grub` + run `grub-mkconfig`.

```
# /etc/default/grub
GRUB_DEFAULT=0
GRUB_TIMEOUT=5
GRUB_CMDLINE_LINUX="ro quiet"
GRUB_HIDDEN_TIMEOUT=0

# /boot/grub/grub.cfg (generated)
menuentry 'Ubuntu 24.04' {
  insmod gzio
  insmod part_gpt
  insmod ext2
  set root='hd0,gpt2'
  linux /boot/vmlinuz-... ro root=/dev/sda2 ro quiet splash
  initrd /boot/initrd.img-...
}
```

### systemd-boot

Minimal, UEFI-only bootloader:

```
ESP layout:
  /boot/ (or /efi/)
    ├── EFI/
    │   ├── BOOT/
    │   │   └── BOOTX64.EFI (fallback entry)
    │   └── systemd/
    │       └── systemd-bootx64.efi
    └── loader/
        ├── loader.conf (config)
        ├── entries/
        │   ├── ubuntu.conf
        │   └── fedora.conf
        └── ...
```

**Configuration**: `/boot/loader/entries/ubuntu.conf`

```
title Ubuntu 24.04
linux /vmlinuz-...
initrd /initrd.img-...
options root=/dev/sda2 ro quiet splash
```

**Simpler** than GRUB2 (smaller code, fewer features), **faster**, but UEFI-only. Often preferred on modern systems.

## Kernel: bzImage and Loading

The bootloader loads the **kernel image** (typically `/boot/vmlinuz-*` or `/boot/Image`). Recent kernels use **bzImage** format (compressed):

```
bzImage structure:
  [16-byte header: "HdrS" magic, version, kernel size]
  [bootsector code (if BIOS boot)]
  [setup code: Detects CPU, enables protected mode, sets up paging]
  [compressed kernel (gzip)]

Bootloader:
  1. Loads entire bzImage into RAM
  2. Zeroes BSS segment (uninitialized data)
  3. Sets registers: ESI=param_block_address (kernel args)
  4. Jumps to kernel entry point (start_kernel)

Kernel:
  1. Early entry: Decompress its own image
  2. Set up virtual memory (paging tables for kernel space)
  3. Enable interrupts
  4. Call start_kernel() C function
```

**Kernel parameters** passed via bootloader command-line:

```
root=/dev/sda2         # Root filesystem device
rw                     # Mount read-write (default: ro)
quiet                  # Suppress most log output
debug                  # Enable debug logging
console=ttyS0,115200  # Serial console
rd.systemd.debug       # systemd debug mode (initramfs)
enforcing=0            # SELinux permissive mode
rd_NO_PLYMOUTH         # Disable splash
```

## Initramfs (Initial RAM Filesystem)

After decompressing, the kernel mounts the **initramfs**: a minimal filesystem image containing:
- Essential drivers (disk controllers, filesystems)
- Device setup tools (mdev/udev, LVM, RAID, cryptsetup)
- init script (typically systemd or busybox init)

### Why Initramfs?

Modern systems have complex storage (LVM, LUKS, RAID, network mounts). Kernel cannot include all drivers compiled-in (bloat). Solution: **Initramfs includes only needed drivers for this system**, reducing kernel size and boot time.

```
Initramfs boot:
  1. Kernel mounts initramfs (cpio-format archive in RAM)
  2. Kernel runs /init (symlink to /lib/systemd/systemd-init or /bin/busybox)
  3. Initramfs tasks:
     - Load drivers (disk, network, GPU)
     - Set up RAID/LVM
     - Decrypt LUKS volumes
     - Find and mount root filesystem
  4. Switch root: pivot_root() → /sysroot becomes /
  5. Execute /sbin/init (systemd or runit)
  6. Initramfs discarded (memory freed)
```

### Building Initramfs

Linux distributions compute initramfs at install/kernel-update time:

```
# Debian/Ubuntu
mkinitramfs -o /boot/initrd.img-... <kernel-version>

# Fedora/RHEL
dracut /boot/initramfs-....img <kernel-version>

# Arch
mkinitcpio -p linux

Result: Binary image (cpio format), compressed (gzip/xz/lz4)
Location: /boot/initrd.img-* or /boot/initramfs-*
```

## Init Systems: Lifecycle from Userspace

Once root filesystem is mounted and initramfs exits, the kernel executes `/sbin/init` (or `/sbin/init` → `/lib/systemd/systemd-init` symlink on modern systems):

### SysV Init (Legacy, 1980s-2010s)

```
Lifecycle:
  /sbin/init (PID 1, parent of all processes)
  ├── Read /etc/inittab (defines runlevels and services)
  ├── Start /etc/rc.sysinit (early setup: hostname, fsck, mount filesystems)
  ├── Start /etc/rc.d/rcN.d/* services (N = runlevel: 0=halt, 1=single, 3=multi, 5=gui, etc.)
  ├── Loop: Reap zombies, re-fork failed services

Runlevels:
  0 → Halt
  1 → Single-user (recovery)
  2-4 → Multi-user (vary by distro)
  5 → GUI (X11/Wayland)
  6 → Reboot

Service startup:
  /etc/rc.d/rc3.d/S50sshd → /etc/rc.d/init.d/sshd start
  (S = start symlink, 50 = priority order)

Drawbacks:
  - Synchronous (wait for each service to start)
  - Complex shell scripts
  - Hard to track dependencies
  - No cgroup/namespace tracking
```

### systemd (Modern, 2010-present)

Replaces SysV init. Units describe services, targets, timers, mounts:

```
/usr/lib/systemd/system/ (vendor units)
/etc/systemd/system/ (admin overrides)

Example: /etc/systemd/system/multi-user.target.wants/sshd.service → /usr/lib/systemd/system/ssh.service

ssh.service:
  [Unit]
  Description=OpenSSH service
  After=network.target

  [Service]
  Type=notify
  ExecStart=/usr/sbin/sshd -D
  Restart=always

  [Install]
  WantedBy=multi-user.target

Systemd lifecycle:
  1. Parse all .service, .target, .mount units
  2. Build dependency graph
  3. Start services in parallel (respecting dependencies)
  4. Track with cgroups: CPU, memory, I/O accounting

Benefits:
  - Parallel startup (faster boot)
  - Socket activation (start service on first client)
  - Automatic restart on crash
  - Logging (journalctl integration)
  - Timer units (cron replacement)
```

### OpenRC (Gentoo, Alpine)

Simpler alternative to systemd:

```
/etc/init.d/sshd

start() {
  ebegin "Starting SSH..."
  /usr/sbin/sshd -D
  eend $?
}

stop() {
  ebegin "Stopping SSH..."
  killall sshd
  eend $?
}

depend() {
  need net
}

rc-service sshd start
rc-update add sshd default  # Run on boot
```

Smaller, more portable, simpler dependency tracking.

## Device Tree (Embedded Linux, ARM)

Rather than probing PCI bus (x86), embedded systems describe hardware in a **device tree**:

```
devicetree (DTS, human-readable):
  / {
    model = "Raspberry Pi 4";
    compatible = "raspberrypi,4";
    
    memory@0 {
      device_type = "memory";
      reg = <0x0 0x80000000>;  // 2 GB
    };
    
    soc {
      compatible = "simple-bus";
      
      uart@7e201000 {
        compatible = "brcm,bcm2835-aux-uart";
        reg = <0x7e201000 0x40>;
        interrupts = <0x5a>;
      };
      
      gpio@7e200000 {
        ...
      };
    };
  };

Bootloader (U-Boot):
  1. Loads DTB (device tree blob, .dtb, flattened binary)
  2. Passes DTB to kernel via register or memory location
  3. Kernel parses DTB, discovers peripherals
  4. Loads drivers for listed devices
```

**Benefit**: Same kernel binary runs on multiple hardware variants (Raspberry Pi revisions, dev boards). Bootloader provides device tree for specific board.

## Secure Boot Chain & Measured Boot

Modern UEFI systems can enforce a **secure boot chain**:

```
Firmware (UEFI):
  ├── Verifies bootloader signature (against Secure Boot DB)
  ├── Loads bootloader if valid
  ├── Measures bootloader hash to TPM PCR[4]

Bootloader (systemd-boot):
  ├── Verifies kernel EFI file signature (using kernel-install generated signatures)
  ├── Loads kernel
  ├── Measures kernel hash + initramfs to TPM PCR[5]
  ├── Loads initramfs (UEFI stub or manually)

Attestation:
  TPM PCRs (Platform Configuration Registers) extended with each measurement:
    PCR[0] = UEFI firmware
    PCR[1] = UEFI config
    PCR[4] = Bootloader
    PCR[5] = Kernel + cmdline + initramfs

Post-boot:
  tpm2_pcrread sha256:4,5 → hash values
  Compare against known-good hashes
  If different: Boot modified or compromised
```

**Limitations:**
- Secure Boot chain breaks if user disables Secure Boot or enrolls backdoor cert
- TPM measurements are only as good as verification tools (can be compromised post-boot)
- Does not prevent data exfiltration at runtime

## Kernel Panic & Recovery

If kernel encounters fatal condition (NULL dereference, divide-by-zero, unhandled exception):

```
Kernel Panic:
  1. Print stack trace (register state, backtrace)
  2. Print memory dump (if enabled, CONFIG_PANIC_DUMP_STACK)
  3. Hang or reboot (depends on /proc/sys/kernel/panic)
     -1: Hang forever (require manual recovery)
     N > 0: Reboot after N seconds

Recovery:
  /proc/sys/kernel/panic_on_oops: If 1, panic on user-space oops (kernel bug)
  /proc/sys/kernel/kexec_load_disabled: If set, prevent kexec (emergency reboot)
  Serial console: Connect via serial to log panic output (data center environments)
```

## See Also

- [OS Process Management](os-process-management.md) — Init system (PID 1), process lifecycle
- [OS File Systems](os-file-systems.md) — Root filesystem, journaling, recovery
- [Systems Reasoning](systems-reasoning.md) — Why filesystem fsync matters during boot