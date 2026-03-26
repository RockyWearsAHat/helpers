# IoT and Embedded Systems Patterns

Embedded systems and the Internet of Things represent computing under constraints — limited memory, processing power, energy budgets, and connectivity — where the engineering trade-offs differ fundamentally from server or desktop software. The patterns that emerge in this domain reflect a constant negotiation between capability, reliability, power consumption, and cost at scale.

## Computing Under Constraints

Embedded devices span a wide spectrum, from 8-bit microcontrollers with 2KB of RAM to application processors running Linux. The constraints shape every design decision:

| Resource     | Typical Range      | Impact on Design                                       |
| ------------ | ------------------ | ------------------------------------------------------ |
| RAM          | 2KB – 512MB        | Data structure choices, dynamic allocation feasibility |
| Flash/ROM    | 16KB – 4GB         | Code size limits, OTA update strategies                |
| CPU          | 8MHz – 1.5GHz      | Algorithm complexity budgets, real-time feasibility    |
| Power        | Microwatts – watts | Sleep mode strategies, communication frequency         |
| Connectivity | None – cellular    | Protocol selection, local vs cloud processing          |
| Unit cost    | $0.10 – $50        | Drives hardware selection; margin pressure at scale    |

A design that works with 512MB of RAM and gigahertz processing may be completely infeasible on a device with 64KB and a 48MHz clock. The discipline of embedded engineering is understanding which trade-offs are available at each point on the spectrum.

## Real-Time Systems

### Hard vs Soft Real-Time

Real-time does not mean "fast" — it means "predictable." A system that always responds within a guaranteed deadline is real-time, even if that deadline is measured in seconds.

| Category       | Deadline Guarantee          | Consequence of Miss                         | Examples                                      |
| -------------- | --------------------------- | ------------------------------------------- | --------------------------------------------- |
| Hard real-time | Absolute — must never miss  | System failure, safety hazard               | Anti-lock brakes, pacemakers, flight controls |
| Firm real-time | Occasional misses tolerable | Result becomes worthless but no catastrophe | Video frame rendering, sensor sampling        |
| Soft real-time | Statistical — usually meets | Degraded quality of service                 | Audio streaming, UI responsiveness            |

The distinction matters because hard real-time systems require formal analysis of worst-case execution time (WCET), while soft real-time systems can rely on statistical guarantees and graceful degradation.

### RTOS Concepts

A Real-Time Operating System provides deterministic task scheduling, bounded interrupt latency, and synchronization primitives designed for timing guarantees.

**Task scheduling** — Most RTOS implementations use priority-based preemptive scheduling. The highest-priority ready task always runs. Rate-monotonic scheduling assigns priorities based on task frequency (higher frequency = higher priority) and provides mathematical guarantees about schedulability.

**Priority inversion** — When a low-priority task holds a resource needed by a high-priority task, and a medium-priority task preempts the low-priority one, the high-priority task is effectively blocked by the medium-priority task. Solutions include:

- **Priority inheritance**: The low-priority task temporarily inherits the high-priority of the blocked task
- **Priority ceiling**: Mutexes are assigned the priority of the highest-priority task that may lock them
- **Lock-free designs**: Avoid shared mutexes entirely using atomic operations and wait-free data structures

**Interrupt handling** — Interrupts preempt all task-level code. Best practice splits interrupt processing into a brief ISR (acknowledge hardware, copy critical data) and a deferred handler (process the data at task level). ISR duration directly impacts worst-case latency for all lower-priority interrupts.

## Memory Constraints and Strategies

### The Dynamic Allocation Problem

On systems with limited RAM, `malloc`/`free` pose multiple risks:

- **Fragmentation**: Over time, freed blocks create gaps too small for new allocations, wasting memory without releasing it
- **Non-deterministic timing**: Allocation may require searching free lists, with variable duration
- **Exhaustion**: No swap space or virtual memory to fall back on
- **Heap corruption**: Buffer overflows in one allocation corrupt adjacent allocations — devastating to debug on devices without MMUs

Many safety-critical systems prohibit dynamic allocation entirely after initialization. Alternatives include:

- **Static allocation**: All buffers sized and allocated at compile time
- **Pool allocators**: Pre-allocated blocks of fixed sizes, O(1) allocate/free
- **Stack-only patterns**: Automatic variables and fixed-size local buffers
- **Region-based allocation**: Allocate from a region, free the entire region at once

### Memory-Mapped I/O

Embedded systems interact with hardware through memory-mapped registers — specific memory addresses that map to hardware functionality. Reading address `0x40021000` might return a GPIO port's pin states; writing to it sets output levels. This requires:

- **Volatile qualifiers**: Preventing the compiler from optimizing away "redundant" reads/writes that actually communicate with hardware
- **Bit manipulation**: Setting, clearing, and testing individual bits within registers
- **Access ordering**: Some hardware requires register accesses in specific sequences; compilers and CPUs may reorder unless explicitly prevented with memory barriers

## Communication Protocols

### MQTT — Lightweight Pub/Sub

MQTT (Message Queuing Telemetry Transport) uses a publish/subscribe model with a central broker. Designed for constrained networks:

| Feature           | Detail                                                                |
| ----------------- | --------------------------------------------------------------------- |
| Transport         | TCP (typically port 1883, 8883 with TLS)                              |
| Message overhead  | As low as 2 bytes header                                              |
| QoS levels        | 0 (at most once), 1 (at least once), 2 (exactly once)                 |
| Retained messages | Broker stores last message per topic for new subscribers              |
| Last Will         | Broker publishes a pre-set message if client disconnects ungracefully |
| Topic structure   | Hierarchical strings: `building/floor3/temperature`                   |

The three QoS levels represent a bandwidth-reliability trade-off. QoS 0 uses minimal bandwidth but may lose messages. QoS 2 guarantees exactly-once delivery but requires a four-step handshake per message.

### CoAP — Constrained RESTful Communication

CoAP (Constrained Application Protocol) brings REST semantics to constrained environments:

- Runs over UDP rather than TCP, reducing overhead
- Supports GET, PUT, POST, DELETE with similar semantics to HTTP
- Adds an OBSERVE option for subscription-like behavior
- Uses compact binary headers instead of text
- Supports DTLS for security over unreliable transport

CoAP suits request-response patterns where devices expose resources. MQTT suits event-driven patterns where data flows toward collectors. Many deployments use both — CoAP for device configuration and management, MQTT for telemetry streams.

### BLE — Bluetooth Low Energy

BLE provides short-range communication optimized for intermittent data transfer:

- **Advertising**: Devices broadcast small data packets without establishing connections — useful for beacons and presence detection
- **GATT profiles**: Connected devices expose services containing characteristics (typed data endpoints) that can be read, written, or subscribed to
- **Connection intervals**: Negotiated timing for communication windows; longer intervals save power at the cost of latency
- **Throughput**: Theoretical maximum around 1-2 Mbps; practical throughput significantly lower depending on connection parameters

### LPWAN — Long Range, Low Power

For devices that need kilometer-range communication on battery power, LPWAN technologies trade data rate for range and energy efficiency:

| Technology   | Range   | Data Rate      | Power    | Spectrum             | Network                       |
| ------------ | ------- | -------------- | -------- | -------------------- | ----------------------------- |
| LoRa/LoRaWAN | 2-15 km | 0.3-50 kbps    | Very low | Unlicensed ISM bands | Community or private gateways |
| NB-IoT       | 1-10 km | Up to 250 kbps | Low      | Licensed cellular    | Carrier infrastructure        |
| Sigfox       | 3-50 km | 100-600 bps    | Very low | Unlicensed           | Sigfox network operator       |
| LTE-M        | 1-10 km | Up to 1 Mbps   | Moderate | Licensed cellular    | Carrier infrastructure        |

LoRa operates in unlicensed spectrum, enabling private networks without carrier contracts but requiring gateway infrastructure. NB-IoT and LTE-M leverage existing cellular infrastructure but incur subscription costs. The choice involves coverage availability, message frequency, payload size, and deployment economics.

## The Edge-Cloud Continuum

### What to Process Where

Not all data needs to reach the cloud. The decision of where to process depends on:

| Factor              | Process at Edge                      | Process in Cloud                     |
| ------------------- | ------------------------------------ | ------------------------------------ |
| Latency requirement | Milliseconds matter                  | Seconds acceptable                   |
| Bandwidth cost      | Raw data volume is high              | Data is compact or infrequent        |
| Privacy             | Sensitive data should not leave site | Data aggregation needed across sites |
| Compute requirement | Simple filtering, thresholding       | ML training, complex analytics       |
| Connectivity        | Intermittent or unavailable          | Reliable connection                  |
| Regulatory          | Data sovereignty requirements        | No geographic restrictions           |

**Edge processing patterns** include local thresholding (only transmit anomalies), data aggregation (send hourly averages instead of per-second readings), and local inference (run a pre-trained model on-device).

**Fog computing** refers to an intermediate layer — gateways or local servers that aggregate data from many edge devices before forwarding summarized results to the cloud. This reduces cloud bandwidth and provides a local coordination point.

## Over-the-Air (OTA) Updates

Updating firmware on deployed devices is one of the most challenging aspects of IoT engineering. A failed update can render a device permanently inoperable ("bricked") in a location that may be physically inaccessible.

### Update Strategies

| Strategy           | Mechanism                                              | Trade-off                                                          |
| ------------------ | ------------------------------------------------------ | ------------------------------------------------------------------ |
| A/B partitioning   | Two firmware slots; write to inactive, swap on success | Requires double flash space but enables atomic rollback            |
| Differential/delta | Send only binary differences from current firmware     | Saves bandwidth but increases complexity; version-specific patches |
| Container-based    | Update application containers on a Linux-based device  | Flexible but requires more capable hardware                        |
| Incremental        | Update individual components rather than full firmware | Fine-grained but complex dependency management                     |

### Safety Requirements

- **Atomic swap**: The transition from old to new firmware should be indivisible — either the new firmware runs or the old one remains
- **Rollback capability**: If new firmware fails validation (boot test, self-check), automatic reversion to the previous version
- **Cryptographic verification**: Firmware images must be signed; the device verifies signatures before installation to prevent tampering
- **Power-loss resilience**: An update interrupted by power loss must not corrupt the device; A/B schemes inherently provide this
- **Version anti-rollback**: Prevent downgrading to older firmware with known vulnerabilities

## Sensor Fusion

Individual sensors provide noisy, incomplete measurements. Sensor fusion combines multiple data sources to produce more accurate and reliable estimates than any single sensor provides.

### Approaches

- **Complementary filtering**: Combines sensors with different noise characteristics. A gyroscope provides smooth short-term rotation but drifts over time; an accelerometer provides noisy but drift-free orientation. A complementary filter weights gyro data for fast changes and accelerometer data for long-term correction.

- **Kalman filtering**: A recursive algorithm that maintains a state estimate with uncertainty, predicts the next state using a physics model, then corrects the prediction using sensor measurements weighted by their respective uncertainties. Extended and Unscented Kalman filters handle nonlinear systems.

- **Particle filtering**: Represents the state distribution as a set of weighted samples ("particles"). Handles highly nonlinear and multimodal distributions where Kalman approaches struggle, at higher computational cost.

The choice depends on computational budget, system linearity, and whether the state distribution is unimodal. A low-power IMU might use a complementary filter; a navigation system might use an extended Kalman filter; a robot localizing in an ambiguous environment might use particle filtering.

## Power Management

For battery-powered or energy-harvesting devices, power consumption determines operational lifetime — often measured in months or years.

### Duty Cycling

Most IoT devices spend the majority of their time doing nothing. Duty cycling alternates between active and sleep states:

```
Wake → Sample sensor → Process data → Transmit if needed → Sleep (repeat)
```

A device that wakes for 10ms every 60 seconds has a duty cycle of ~0.017%. At such low duty cycles, sleep-mode current (microamps) dominates the average power budget, making sleep-mode efficiency critical.

### Sleep Modes

Microcontrollers typically offer multiple sleep states with different power-preservation trade-offs:

| Mode               | Power    | Wake Time            | State Preserved                              |
| ------------------ | -------- | -------------------- | -------------------------------------------- |
| Idle/Sleep         | Moderate | Microseconds         | CPU halted, peripherals active, RAM retained |
| Deep sleep         | Low      | Milliseconds         | Most peripherals off, RAM retained           |
| Shutdown/hibernate | Very low | Tens of milliseconds | Only RTC and wake logic powered; RAM lost    |

Deeper sleep saves more power but costs wake-up time and potentially requires re-initialization. The optimal strategy depends on how frequently the device must wake and how quickly it must respond.

### Energy Harvesting

Devices that harvest ambient energy (solar, thermal, vibration, RF) face an additional design dimension — the power budget varies with environmental conditions. Supercapacitors or small batteries buffer harvested energy, and the firmware must adapt its duty cycle to available energy, potentially entering emergency low-power modes when reserves are depleted.

## Reliability in Harsh Environments

Deployed IoT devices face conditions that development environments rarely simulate — temperature extremes, humidity, vibration, power fluctuations, and electromagnetic interference.

### Watchdog Timers

A hardware watchdog timer must be periodically reset ("kicked") by the firmware. If the firmware hangs or crashes and fails to kick the watchdog, the timer expires and forces a hardware reset. Design considerations:

- Watchdog timeout must be longer than the longest legitimate processing period
- Kicking the watchdog should only happen in the main control path, not in interrupt handlers (which could continue running even if the main loop is stuck)
- Some designs use a "windowed" watchdog that resets if kicked too early OR too late, catching both hang and runaway-loop conditions

### Additional Reliability Patterns

- **CRC/checksum on stored data**: Detecting corruption in flash storage or RAM
- **Redundant storage**: Storing critical configuration in multiple locations with voting
- **Brown-out detection**: Cleanly shutting down before supply voltage drops below safe operating levels
- **Defensive coding**: Bounds checking on all array accesses, default cases in all switch statements, timeout on all blocking operations
- **Stack canaries**: Detecting stack overflow on systems without MMU protection

## Security in Constrained Environments

Securing devices with limited processing power, no operating system, and physical accessibility to attackers presents unique challenges.

### Lightweight Cryptography

Standard cryptographic algorithms (AES-256, RSA-2048) may be too computationally expensive or require too much RAM for the smallest microcontrollers. Lightweight alternatives include:

- **ASCON**: Selected by NIST as the standard for lightweight authenticated encryption and hashing
- **ChaCha20-Poly1305**: Efficient on processors without hardware AES acceleration
- **Curve25519**: Elliptic curve key exchange with small key sizes and efficient implementation
- **PHOTON/SPONGENT**: Lightweight hash functions for highly constrained devices

The trade-off is between security margin and computational cost. Lightweight primitives are designed to provide adequate security within tighter resource budgets, not to be "less secure."

### Secure Boot and Attestation

**Secure boot** establishes a chain of trust from hardware to application code. Each stage of the boot process verifies the cryptographic signature of the next stage before executing it. A hardware root of trust (ROM bootloader or secure element) anchors the chain.

**Remote attestation** allows a server to verify that a device is running expected firmware. The device generates a cryptographic measurement of its software state, signed by a hardware-protected key. This detects firmware tampering without requiring physical access.

### Physical Security Considerations

Unlike servers in locked data centers, IoT devices may be physically accessible to attackers. Attack vectors include:

- JTAG/SWD debug port access for firmware extraction or modification
- Bus snooping (I2C, SPI, UART) for data interception
- Side-channel attacks (power analysis, electromagnetic emanation) for key extraction
- Flash dumping via direct chip access
- Glitching (voltage or clock manipulation) to bypass security checks

Countermeasures range from disabling debug ports in production firmware to using secure elements that resist physical probing.

## Digital Twins

A digital twin is a virtual representation of a physical device or system, maintained in synchronization with its real-world counterpart through sensor data and communication.

### Applications

- **Simulation**: Test firmware updates or configuration changes against the digital twin before deploying to physical devices
- **Monitoring**: Visualize device state, predict maintenance needs, detect anomalies by comparing expected (simulated) vs actual (reported) behavior
- **Fleet management**: Aggregate digital twins to understand system-wide behavior patterns
- **Historical analysis**: The digital twin retains full state history even when the physical device only reports current state

### Implementation Spectrum

Digital twins range from simple state mirrors (a database record reflecting last-reported sensor values) to high-fidelity physics simulations that model thermal behavior, mechanical wear, or chemical processes. The appropriate fidelity depends on what questions the twin needs to answer.

## Cross-Cutting Design Tensions

### Connectivity vs Power

Every radio transmission costs significant energy relative to local computation. Designs must balance reporting frequency against battery life. Approaches include batching multiple readings into single transmissions, transmitting only when values change beyond a threshold, and negotiating communication schedules with gateways.

### Local Intelligence vs Simplicity

More on-device processing reduces cloud dependency and bandwidth but increases firmware complexity, flash usage, and development cost. A temperature sensor that simply reports readings is simpler and more reliable than one running anomaly detection models locally — but the latter reduces communication costs and enables faster response to anomalies.

### Updatability vs Security vs Reliability

OTA updates enable bug fixes and feature additions but introduce attack surface (compromised update server) and reliability risk (botched update). Disabling updates maximizes stability but prevents patching vulnerabilities. The balance depends on threat model, deployment accessibility, and product lifecycle expectations.

### Interoperability vs Optimization

Standard protocols (MQTT, CoAP, HTTP) enable multi-vendor ecosystems but may not be optimal for specific use cases. Custom binary protocols minimize bandwidth and processing but create vendor lock-in and integration challenges. The trend toward standardization reflects the ecosystem value of interoperability, even at some efficiency cost.
