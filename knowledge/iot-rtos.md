# Real-Time Operating Systems (RTOS) — FreeRTOS, Zephyr, and Design Patterns

## Overview

RTOS kernels provide primitives for deterministic, prioritized task execution. Designed for embedded and IoT devices where timing guarantees matter: a thermometer reading delayed by 100 ms is acceptable; an airbag deployment delayed by 10 ms is catastrophic. RTOS minimizes latency and provides scheduling guarantees. FreeRTOS dominates microcontroller (*µC*) space; Zephyr is emerging as unified kernel for IoT.

## FreeRTOS — Lightweight Preemptive Multitasking

FreeRTOS runs on ARM Cortex-M (STM32, NXP, Nordic), Cortex-A (soft real-time), and RISC-V. Kernel: ~10 KB binary; runs in RAM.

### Core Primitives

**Tasks (Threads)**: Independently scheduled sequential code. Each task has stack, program counter, registers. Context switch: kernel saves task state, loads next task to run.

```c
void task_read_sensor(void *param) {
    while(1) {
        int value = read_adc();
        printf("Sensor: %d\n", value);
        vTaskDelay(pdMS_TO_TICKS(100)); // Sleep 100 ms
    }
}

xTaskCreate(task_read_sensor, "Sensor", 128, NULL, tskIDLE_PRIORITY + 1, NULL);
```

**Priorities**: Each task has integer priority (0 = idle, higher = runs first). Scheduler chooses highest-priority ready task. Two tasks of equal priority run round-robin (time-sliced).

**Scheduling**: Preemptive (interrupt latency < 1 µs typical). When higher-priority task wakes, kernel immediately context-switches.

**Queues**: Thread-safe message passing. Task A sends message (pointer or small struct) to queue; Task B receives, processes. Decouples producer/consumer timing.

```c
xQueueSend(queue_handle, &sensor_value, portMAX_DELAY);
xQueueReceive(queue_handle, &received_value, pdMS_TO_TICKS(1000));
```

**Semaphores**: Binary (mutex-like) or counting. Used for synchronization and mutual exclusion.

```c
xSemaphoreTake(mutex, portMAX_DELAY); // Wait until available
// Critical section
xSemaphoreGive(mutex); // Release
```

**Mutexes with Priority Inheritance**: Standard mutex can cause priority inversion (low-priority task holds lock, high-priority task waits). FreeRTOS mutexes with priority inheritance: when high-priority task waits, temporary lock holder inherits priority, runs to completion, releases lock, reverts to original priority. Prevents unpredictable latency.

### Tick-Based Timing

FreeRTOS runs a system tick interrupt (1–1000 Hz, configurable). Each tick, kernel checks if any tasks should wake or change state. Task delays (`vTaskDelay(100)`) are in ticks, not milliseconds; conversion via `pdMS_TO_TICKS()`.

**Downside**: Minimum delay granularity is one tick. Tick=1 kHz → minimum delay ~1 ms. Fine for typical sensors; inadequate for high-frequency audio.

### Memory Model

**Static Allocation**: Pre-allocate all task stacks and kernel objects (queues, semaphores) at compile-time. Predictable memory footprint; no fragmentation. Suitable for hard real-time.

**Dynamic Allocation**: `pvPortMalloc()` allocates at runtime (akin to `malloc`). Heap size fixed; malloc returns NULL if exhausted. Predictable but no garbage collection; memory leaks possible.

### No Protection Across Tasks

FreeRTOS runs in user space (not privileged mode on ARM). No memory isolation between tasks; one task can corrupt another's memory. Acceptable for small deeply-embedded systems; problematic for complex IoT with third-party code.

## Zephyr — Modern RTOS for IoT

Zephyr is Linux Foundation project; open source, supports Cortex-M, Cortex-A, RISC-V, x86, Xtensa. Targets connected IoT: Wi-Fi, Bluetooth, Thread. Higher-level abstraction than FreeRTOS but heavier (kernel ~50 KB).

### Device Tree (DT)

Hardware configuration is declarative, not hardcoded. Device tree describes SoC peripherals (UART, SPI, GPIO, ADC), addresses, interrupts, properties. Kernel queries DT at boot to bind drivers.

Example fragment (SoC with multiple UARTs):

```dts
uart0: uart@40000000 {
    compatible = "vendor,uart";
    reg = <0x40000000 0x100>;
    interrupts = <8>;
    status = "okay";
};
```

**Benefit**: Single firmware binary boots on multiple boards (different UART addresses) by swapping device tree; no recompilation.

### Driver Framework

Zephyr abstracts device drivers behind standard APIs (GPIO, SPI, I2C, ADC). Application uses high-level calls; drivers handle hardware specifics. Pluggable: swap I2C driver for different SoC without application changes.

```c
const struct device *adc_dev = DEVICE_DT_GET(DT_NODELABEL(adc0));
adc_read(adc_dev, &sequence, &buffer);
```

### Networking

Built-in IPv6, Bluetooth LE, Thread, CoAP, MQTT. Unlike FreeRTOS (bare kernel), Zephyr is turnkey for connected IoT. Reduces integration time.

### POSIX-Like API

Zephyr supports POSIX subset (threads, mutexes, condition variables, message queues). Easier port from Linux code; steeper learning curve than FreeRTOS's simplified API.

### Memory Protection Unit (MPU) Support

Zephyr can enforce memory isolation between tasks using ARM MPU/MMU. Each task gets protected region; access violation triggers fault. Prevents one task corrupting another; more robust than FreeRTOS but higher overhead.

## Hard vs Soft Real-Time Guarantees

**Hard Real-Time**: Missing deadline = system failure. Example: airbag, cardiac pacemaker. OS must guarantee every operation meets deadline under all conditions. Requires:
- Bounded interrupt latency (e.g., <10 µs).
- Deterministic scheduler (no unbounded waits for locks).
- Provable analysis via rate-monotonic or deadline-monotonic scheduling.

**Soft Real-Time**: Missing deadline is undesirable but not catastrophic. Example: video playback (skip frame, continue). OS prioritizes average latency + throughput over worst-case guarantees.

**Firm Real-Time**: In-between. Deadline miss has value loss (e.g., sensor reading loses relevance) but system continues. Most IoT devices are firm: a temperature reading 200 ms late is less useful but not a failure.

FreeRTOS and Zephyr are often firm real-time; certifying hard real-time requires formal proof of scheduling and interrupt handling, which both can achieve but not out-of-the-box.

## Priority Scheduling and Priority Inversion

### Rate Monotonic Scheduling (RMS)

Assign priorities inversely to task period. Task executing every 10 ms gets higher priority than task executing every 100 ms. RMS is optimal for fixed-priority scheduling: if a priority assignment exists to meet all deadlines, RMS finds it.

Example:
- Task A: period 10 ms → priority 3 (high)
- Task B: period 50 ms → priority 2
- Task C: period 100 ms → priority 1 (low)

Schedulability: compute utilization $U = \sum \frac{C_i}{T_i}$ where $C_i$ is task running time, $T_i$ is period. If $U \leq 0.69 \cdot n$ (n = number of tasks), RMS guarantees all deadlines met.

### Priority Inversion Problem

Lower-priority task acquires lock; higher-priority task waits. Example: GPS logger (low priority) holding I2C bus; accelerometer ISR tries to read I2C (triggered by high-priority timer), blocks on low-priority task. Latency spike.

**Solution**: Priority inheritance mutex. When high-priority task waits, lock holder inherits high priority, runs to completion, releases, reverts. Bounds inversion duration.

FreeRTOS mutexes implement priority inheritance; simple semaphores do not (use mutexes for locks).

## Watchdog Timers

RTOS does not prevent infinite loops or deadlock. Watchdog timer (hardware) fires if not periodically petted (reset). If firmware gets stuck, watchdog fires, resets the chip.

```c
// Watchdog setup (chip-specific)
kick_watchdog(); // Reset timer in main loop

// If main loop never reaches here, watchdog fires in ~1 second
```

Essential for reliability in unattended IoT devices (gateway in field, cannot be reset manually).

## POSIX Compliance

POSIX (Portable Operating System Interface) standard defines APIs for multithreading, semaphores, signals (on Unix systems). Most RTOS kernels claim partial POSIX compliance:

- **FreeRTOS**: No POSIX, custom API. Easier to learn; less portable code.
- **Zephyr**: Optional POSIX subsystem (pthread, semaphore, message queues).
- **Linux (even on embedded)**: Full POSIX; code portable to desktop Linux for development.

POSIX compliance aids porting but blurs the line between embedded OS and general-purpose OS. Trade-off: POSIX features add size/overhead.

## Safety Certification (IEC 61508, ISO 26262)

Critical systems (automotive, medical) require formal certification that OS does not cause failures. Certifications:

- **IEC 61508**: Functional safety, generic standard.
- **ISO 26262**: Automotive functional safety (ASIL levels A–D, D = most critical).

Certified RTOS kernels:
- **FreeRTOS**: Partial certification available via commercial partner Wittenstein.
- **Zephyr**: Not heavily certified yet; used in non-critical paths of certified systems.

Certification involves:
- Traceability: every requirement traced to test.
- Formal proof of scheduling and timing.
- Fault injection testing.
- Code review.

Cost: $10k–$100k+ for full certification. Overkill for hobby IoT; necessary for medical devices.

## RIOT OS — Alternative

RIOT is another open-source RTOS targeting IoT, emphasizing standards compliance (POSIX, IEEE 802.15.4) and modular design. Smaller than Zephyr (~15 KB kernel), but less mainstream. Used in research and specialized deployments (mesh networks, low-power wireless).

## Typical Deployment Pattern

Modern IoT device combines:
1. **Bootloader** (proprietary, vendor-locked). Boots from flash, initializes hardware, loads kernel.
2. **RTOS kernel** (FreeRTOS or Zephyr). Multitasking, drivers.
3. **Application tasks** (user code). Business logic.
4. **Networking stack** (MQTT client, HTTP). Communicates with cloud/gateway.
5. **Watchdog** (hardware). Resets if hung.

Stack footprint: ~50–200 KB for FreeRTOS; ~200 KB for Zephyr with networking.

## Key Insight

RTOS is a multiplexing layer. Without it, firmware is a state machine in a loop; adding RTOS enables cleaner, more modular code with priority-based concurrency. Cost: ~1% CPU overhead for context switching, ~10 KB kernel. Benefit: predictable latency, decoupled tasks, no priority-inversion surprises if you follow patterns.