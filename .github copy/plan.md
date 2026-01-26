# Plan: OG-DK and DKC — HLE BIOS Investigation

**Status:** 🔴 NOT STARTED
**Goal:** Fix OG-DK rendering (~98% black) through HLE BIOS improvements

---

## Context

### Critical User Requirement

User does NOT have official GBA BIOS. Solutions requiring LLE BIOS are NOT acceptable.

### Root Cause Analysis

**OG-DK (Classic NES Series: Donkey Kong) — FDKE**

- Game boots, calls SWI 0x11, uploads NES emulator code to IWRAM 0x03007400
- IWRAM code enters tight loop at 0x03007424-0x0300743C
- VCOUNT polling works correctly (scanlines advance 12→13→14...)
- Palette RAM stays all zeros — display init never completes
- Frame analysis: ~1.8% non-black (some border elements render)

**DKC (Donkey Kong Country) — A5NE**

- Renders 99.6% non-black pixels (working well)
- User reports "5%" issues but hasn't specified what

### Boot Sequence (OG-DK)

```
Step 1:   PC=0x08000000 (ROM entry)
Step 10:  PC=0x080000EC (SWI 0x11)
Step 20:  PC=0x0300741C (IWRAM)
Step 30+: Looping 0x03007424→0x0300742C→0x03007434→0x0300743C
```

---

## Steps

### Step 1: Add IWRAM loop trace — `src/emulator/gba/GBAMemory.cpp`

**Operation:** INSERT_AFTER
**Anchor:** (after line ~1570, inside Read16 after OGDK_VCOUNT trace)

```cpp
  // OG-DK investigation: trace reads in the initialization loop area.
  // The loop at 0x03007424-0x0300743C is polling something.
  // Enable with: AIO_TRACE_OGDK_INIT_LOOP=1
  static const bool traceOgdkInitLoop =
      EnvTruthy(std::getenv("AIO_TRACE_OGDK_INIT_LOOP"));
  if (traceOgdkInitLoop && cpu && region == 0x03) {
    const uint32_t iwramOff = address & 0x7FFF;
    const uint32_t pc = (uint32_t)cpu->GetRegister(15);
    // Focus on reads from IWRAM when PC is in the init loop range
    if (pc >= 0x03007400 && pc < 0x03007500) {
      static int logCount = 0;
      if (logCount < 500) {
        AIO::Emulator::Common::Logger::Instance().LogFmt(
            AIO::Emulator::Common::LogLevel::Info, "OGDK_INIT",
            "R16 PC=0x%08x addr=0x%08x val=0x%04x",
            (unsigned)pc, (unsigned)address, (unsigned)val);
        logCount++;
      }
    }
  }
```

**Verify:** `grep -A5 "OGDK_INIT" src/emulator/gba/GBAMemory.cpp`

---

### Step 2: Add IO read trace during init loop — `src/emulator/gba/GBAMemory.cpp`

Add to same trace block, also log IO register reads:

```cpp
  // Also trace IO reads during the init loop
  if (traceOgdkInitLoop && cpu && region == 0x04) {
    const uint32_t pc = (uint32_t)cpu->GetRegister(15);
    if (pc >= 0x03007400 && pc < 0x03007500) {
      static int ioLogCount = 0;
      if (ioLogCount < 200) {
        AIO::Emulator::Common::Logger::Instance().LogFmt(
            AIO::Emulator::Common::LogLevel::Info, "OGDK_INIT",
            "IO_R16 PC=0x%08x off=0x%03x val=0x%04x",
            (unsigned)pc, (unsigned)(address & 0x3FF), (unsigned)val);
        ioLogCount++;
      }
    }
  }
```

**Verify:** `make build && AIO_TRACE_OGDK_INIT_LOOP=1 ./build/bin/AIOServer --headless --rom OG-DK.gba --headless-max-ms 200 2>&1 | grep OGDK_INIT | head -30`

---

### Step 3: Analyze trace output

After running Step 2, examine what the loop is polling. Expected findings:

- IWRAM address being checked (e.g., 0x03007FF8 BIOS_IF)
- IO register being polled (e.g., IME, IE, IF, DISPSTAT)
- Value that needs to change for loop to exit

---

### Step 4: Fix based on findings

Once we know what the loop waits for, implement the fix. Potential fixes:

**If polling BIOS_IF (0x03007FF8):**
Ensure VBlank IRQ sets this flag even if IME=0

**If polling specific IWRAM flag:**
Check if BIOS boot sequence should initialize it

**If polling IO register incorrectly:**
Verify our IO register implementation matches GBATEK

---

## Test Strategy

1. `make build` — compiles without errors
2. `ctest --output-on-failure` — 135 tests pass
3. OG-DK test after fix:

```bash
./build/bin/AIOServer --headless --rom OG-DK.gba \
  --headless-max-ms 5000 --headless-dump-ppm ogdk.ppm --headless-dump-ms 4500
# Success: nonBlackRatio > 0.50
```

---

## Handoff

Run `@Implement` to add the trace code (Steps 1-2), then analyze output to determine fix.
