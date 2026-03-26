# Embedded Bus Protocols — I2C, SPI, UART, CAN, USB, PCIe

## Overview

Embedded systems interconnect chips via short-range synchronous and asynchronous buses. Protocols differ in pin count, speed, distance, duplex mode, and master/slave architecture. No universal bus; choice depends on data rate, device density on bus, and power constraints. I2C dominates sensor interconnects; SPI dominates flash/display; UART for console/legacy; CAN for automotive; USB for host/device; PCIe for high-bandwidth.

## I2C — Inter-Integrated Circuit (TWI)

I2C (Philips, now NXP) connects multiple slave devices to a single master over two wires: SDA (data), SCL (clock). Open-drain outputs; pull-ups (typically 4.7 kΩ) hold lines high; devices pull low to transmit 0.

### Multi-Master Support

I2C allows multiple masters (rare in practice). When two masters want to transmit, lower bitrate wins (wired-AND logic on open-drain). Requires:** Arbitration**: Masters compare their transmitted bit against bus state; if mismatch, lose arbitration and stop. Example:
- Master A outputs 1 (releases bus), sees 0 (Master B transmitting). Master A loses, waits.
- Master B outputs 0 (pulls low), sees 0 (match), continues.

**Clock Stretching**: Slave can hold SCL low after a bit, pausing transmission. Master waits. Slave releases when ready. Used for slow slaves or to synchronize multiple masters.

### Addressing

7-bit address (0–127) or 10-bit. Master sends address + R/W bit (0=write, 1=read). Slave with matching address responds with ACK (pulls SDA low during 9th bit). If no device on bus, SDA remains high (NACK).

**Address Conflicts**: Multiple devices with same address cannot coexist. Design-time constraint; some devices have address pins to select from range (e.g., address 0x50–0x57 for eight EEPROM variants).

### Speed Modes

- **Standard**: 100 kbit/s (original).
- **Fast**: 400 kbit/s (typical today).
- **Fast Plus**: 1 Mbit/s (requires lower pull-ups: 2.2 kΩ, shorter wires).
- **High Speed (HS)**: 3.4 Mbit/s (requires separate HS master, multiplexer, special signaling).

Speed limited by RC time constant of pull-up + line capacitance; higher speeds demand shorter traces and stronger pull-ups.

### Typical Devices

- Sensors: temperature (TMP36), accelerometer (MPU6050), barometer.
- Memory: EEPROM (24Cxx), RTC (DS1307).
- Audio codec, power management IC (PMIC).

Daisy-chaining: up to 128 devices per I2C bus (7-bit addressing).

### Weak Points

- **Slow**: 400 kbit/s → ~50 bytes/ms. Inadequate for real-time streaming.
- **Clock stretching** adds unpredictable latency; complex for real-time systems.
- **Electrical fragility**: Vulnerable to noise; termination issues cause bit errors.
- **Silicon bugs**: Some chips have non-standard I2C implementation, breaking multi-master.

## SPI — Serial Peripheral Interface

SPI connects a master to multiple slaves over four wires: MOSI (Master Out, Slave In), MISO (Master In, Slave Out), SCLK (clock), CS (Chip Select). Synchronous: clock driven by master; data valid on clock edge.

### Full-Duplex

Unlike I2C (half-duplex, master writes or reads), SPI transmits and receives simultaneously. Master clocks data out MOSI; slave shifts data in; slave clocks data out MISO; master shifts data in. Both happen at same clock edge.

**Implication**: SPI is faster (10–100 Mbit/s typical; 200+ Mbit achievable). No arbitration needed (master owns clock); no addressing logic.

### Chip Select (CS)

SPI uses separate CS line per slave (active low, typically). Master pulls CS low before transaction, releases after. Only selected slave drives MISO; others tri-state (high-impedance). Multiple CS lines scale up to ~8 slaves per SPI bus before pin count becomes limiting.

### Clock Modes (CPOL, CPHA)

Two settings control clock polarity and phase:
- **CPOL=0**: Idle clock low. Data sampled on rising edge.
- **CPOL=1**: Idle clock high. Data sampled on falling edge.
- **CPHA=0**: Data valid on leading clock edge.
- **CPHA=1**: Data valid on trailing clock edge.

Four combinations (modes 0–3). Master and slave must use same mode or, no data corruption; mode not negotiated, hardcoded. Mismatched mode → garbled data.

### No Slave-to-Master Initiation

Master always clocks. Slave cannot push data; master must poll periodically. Inconvenient for interrupt-driven sensors (accel detects motion, wants to notify master immediately). Workaround: slave asserts interrupt line (separate wire); master polls SPI.

### Streaming

High-speed SPI streams audio/image data. Example: SPI camera (OV5640) → 24 Mbit/s YUV stream → Cortex-A8. Requires DMA controller to keep up; CPU cannot handle bit-by-bit interrupts.

### Wire Distance

SPI signals are digital (no differential pairs). Susceptible to noise on longer buses (>1 meter). Typically constrained to on-board communication; off-board SPI requires twisted-pair shielding.

## UART — Universal Asynchronous Receiver Transmitter

UART is asynchronous serial (no clock). Data rate agreed beforehand (baud rate: 9600, 115200, etc.). Each byte encapsulated: 1 start bit (0), 8 data bits, 1 stop bit (1). Slave can transmit anytime; full-duplex.

### Voltages

**RS-232**: Legacy ±12V (RS-232), TIA-232, decays obsolete. Needs external transceiver (MAX232 charge pump).

**TTL/CMOS**: 0–3.3 V (modern). Arduino, microcontrollers, FTDI adapters. No transceiver; direct connection.

### Slow but Ubiquitous

9600 baud = ~960 bytes/s. Glacial by today's standards but sufficient for sensor/debug logs. Console output, legacy equipment.

### Handshake Lines

Full RS-232 includes RTS (Request To Send), CTS (Clear To Send), DTR (Data Terminal Ready), DSR (Data Set Ready), DCD (Data Carrier Detect). Rarely used in modern embedded (most use 2-wire: TX, RX only). Handshake-ready UARTs sit unused.

### Half-Duplex Variants

Some UARTs can operate in half-duplex mode (one wire, both master/slave share). Requires external transceiver (RS-485). Used in industrial automation (Modbus RTU).

## CAN Bus — Controller Area Network

CAN (ISO 11898) is automotive standard. Differential pair bus (CAN_H, CAN_L, balanced ~2.5V each). Devices tap into bus; multiple nodes transmit, all nodes receive (broadcast). Collisions resolved by arbitration (dominant = 0, recessive = 1).

### Arbitration

Two nodes transmit CAN IDs simultaneously. Bits compared: if both output dominant (0), bus is dominant and matches both. If one outputs recessive (1) and other outputs dominant, dominant wins and recessive loses arbitration. Lower CAN ID wins (gets through).

Example:
- Node A (ID 0x123 = 0001_0010_0011): Transmits 0, sees 0, continues.
- Node B (ID 0x456 = 0100_0101_0110): Transmits 0, sees 0, continues... until bit 10.
- Node A outputs 1 (recessive), sees 0 (Node B). Node A backs off. Node B continues.

**Benefit**: Guaranteed lowest-ID message is uninterrupted; no collision or retry. Real-time capable.

### Messages and Payloads

Standard CAN: 11-bit ID, 0–8 byte payload, ~1 Mbit/s.
Extended CAN: 29-bit ID, same 8 bytes, lower bit rate (competitive with standard).

**CAN FD** (CAN with Flexible Data-rate): Variable payload (8–64 bytes), higher bitrates (5–10 Mbit/s) during data phase (post-arbitration).

### Robustness

Differential signaling (not single-ended like I2C) improves noise immunity. Common in harsh automotive/industrial environments. Error detection via CRC; repeated collisions or errors → node disconnect (error frame).

### Bandwidth

1 Mbit/s seems fast but 8-byte payloads at 100 Hz = only 6.4 kbit/s effective throughput after 11-bit ID, CAN headers (~64 bits/message). Not suitable for real-time video; adequate for control/diagnostics.

## USB — Universal Serial Bus

USB endpoint-based: devices expose endpoints (0, 1, 2, ...). Host sends packets to endpoints. Endpoint zero (default) handles control setup; other endpoints handle specific functions (bulk data, interrupt polling, isochronous streaming).

### Descriptors

Device describes itself: device class (HID, CDC, mass storage), endpoints, speeds (Full-Speed 12 Mbps, High-Speed 480 Mbps, Super-Speed 5 Gbps).

**Enumeration**: Host reads device descriptor, assigns address, sets config. Device then operates with Endpoint 1 for interrupt/bulk transfers, Endpoint 2 for data, etc.

### Transfer Types

- **Control**: Host command/response (setup device). Used only by USB stack, not applications.
- **Bulk**: Large data, no guaranteed timing. Disk/printer/network transfers.
- **Interrupt**: Polling interval (host asks device every N ms). Keyboards, mice, sensors.
- **Isochronous**: Real-time streaming (audio/video). No retransmission on error; latency constant.

### Power Delivery

USB 2.0: 500 mA max per device. USB 3.0: 900 mA. USB PD (Power Delivery): up to 240 W negotiated via CC lines. Modern IoT uses USB PD for battery charging + data.

### Complexity

USB stack is large (~50 KB in Linux kernel); microcontrollers often use vendor USB library. Non-trivial to implement from scratch. Benefit: ubiquitous host support (every PC has USB).

## PCIe — PCI Express

High-speed point-to-point bus for CPUs/GPUs/NICs. Hierarchical topology (switched fabric), not broadcast like CAN. Lanes (1x, 4x, 16x) determine bandwidth; each lane ~250 MB/s per direction (PCIe 3.0, dual-lane capable).

### Transaction Layer Packets (TLPs)

Messages encapsulated as TLPs: address, data, tags. Root complex (CPU) initiates transactions. Endpoints (GPU, NIC) respond. Switches route dynamically.

### Scale

PCIe can address 256 devices × 256 functions per device = vast namespace. Enumeration at boot; tree discovery, driver loading.

### Not for Embedded

PCIe is overkill for embedded IoT. System-level bus: motherboard to discrete cards. Requires external clock generator, multiple lanes, complex drivers. ARM SoCs rarely use PCIe (exception: high-end ARM servers).

## Bus Comparison Table

| Bus | Speed | Distance | Duplex | Master/Slave | Use |
|-----|-------|----------|--------|------------------|-----|
| I2C | 100–400 kbit/s | ~1 meter | Half | M/S cluster | Sensors, memory |
| SPI | 10–100 Mbit/s | ~30 cm | Full | 1M/nS | Flash, displays |
| UART | 9600–1.5 Mbit/s | ~100 meters (RS-485) | Full | Async | Console, legacy |
| CAN | 1 Mbit/s | ~40 meters | Broadcast | Multi-M (arb) | Automotive, industrial |
| USB | 12 Mbps–5 Gbps | 5 meters | Full | Host/Device | PC peripherals |
| PCIe | 0.25–4 GB/s | 30 cm | Full | Root/Endpoints | CPU/discrete cards |

## Electrical Considerations

**Impedance**: High-speed (SPI >50 Mbit/s, USB, PCIe) require terminated stub lengths and controlled impedance (~85 Ω differential for PCIe). Low-speed (I2C, UART) < flexible.

**Pull-ups/Pull-downs**: I2C (pull-up), UART (no termination typical), CAN (differential termination 120 Ω).

**EMI**: High-speed buses radiate; shielding, filtering, ground planes reduce interference.

**Noise**: I2C susceptible to noise spikes (open-drain, slow rise time); SPI TLL more robust but shorter distance.

## Emerging: MIPI CSI-2, LVDS

**MIPI CSI-2** (Camera Serial Interface): Low-power serial for image sensors. Used in phones: 4 lanes × 1 Gbps each → 4 Gbps (entire camera raw video stream). Replaces older parallel camera interfaces.

**LVDS** (Low-Voltage Differential Signaling): High-speed differential (similar to PCIe principle). Used in LCD panels. Gradually replaced by DSI/MIPI.

## Key Insight

Embedded communication is a negotiation: speed vs. complexity vs. distance. I2C is simplest, slowest, most robust to interference. SPI is faster, simpler logic, short-distance only. CAN is automotive-optimized, real-time-friendly. USB/PCIe are PC-centric standards, overkill for IoT but ubiquitous. Choose based on system constraints and cost; mixing protocols is normal: UART for debug, I2C for sensors, SPI for flash, Ethernet for cloud.