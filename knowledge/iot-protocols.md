# IoT Protocols — MQTT, CoAP, LoRaWAN, Zigbee, BLE, Thread/Matter

## Overview

IoT systems operate across a spectrum of network conditions, device capabilities, and power constraints. Protocols differ primarily in QoS guarantees, addressing, message overhead, range, and topology. Selection depends on deployment scale, latency tolerance, power budget, and infrastructure availability. No single protocol dominates; the right choice is context-specific.

## MQTT — Message Queuing Telemetry Transport

MQTT is a publish-subscribe protocol standardized by OASIS (versions 3.1.1 and 5.0). It assumes a reliable TCP/IP network and broker-based architecture.

### Core Concepts

**Publish-Subscribe Model**: Publishers send messages to topics; subscribers receive without direct connection. Decoupling enables loose integration and dynamic topology changes.

**Topics**: Hierarchical strings with `/` separators (e.g., `house/living-room/temperature`). Wildcards: `+` (single level), `#` (multi-level). No explicit topic creation; created on first publish.

**QoS Levels** (guarantees, not speeds):
- **QoS 0 (At Most Once)**: Fire-and-forget. Message loss acceptable. Smallest overhead.
- **QoS 1 (At Least Once)**: Packet IDs track delivery; broker retransmits until PUBACK received. Duplicates possible but unlikely in stable networks.
- **QoS 2 (Exactly Once)**: Four-way handshake (PUBLISH, PUBREC, PUBREL, PUBCOMP) ensures exactly one delivery. Broker maintains state across client restarts. Highest overhead.

Higher QoS increases CPU, memory, and bandwidth consumption — trade-offs with reliability.

**Retain Flag**: When publisher sets retain=true, broker stores the last message for each topic and delivers it immediately to new subscribers. Useful for state snapshots (e.g., "device last seen at X time") but creates stale data risk.

**Last Will and Testament (LWT)**: Client specifies a message with topic and QoS. Broker publishes it if the client disconnects ungracefully (TCP connection lost, not DISCONNECT). Enables fault detection without periodic heartbeats.

**Session Persistence**: MQTT 5.0 distinguishes clean vs persistent sessions. Persistent sessions survive client restarts; broker queues messages during offline periods (bounded by broker policy).

### AMQP — Advanced Message Queuing Protocol

AMQP is often mentioned alongside MQTT but serves different niches. It emphasizes reliable delivery, security, and interoperability in enterprise message brokers (RabbitMQ, Apache Qpid). Heavier overhead, richer routing semantics, but less suitable for resource-constrained IoT devices than MQTT. Rarely chosen for edge devices; used for cloud-side integration.

## CoAP — Constrained Application Protocol

CoAP (RFC 7252) is designed for extreme resource constraints: minimal memory, CPU, and bandwidth. It targets devices with kilobytes of RAM and intermittent connectivity.

### Architecture

**UDP Transport**: Stateless, low overhead. No connection setup or teardown. Suitable for unreliable networks (radio, satellite). Downside: no built-in reliability; application must handle retransmission.

**RESTful Design**: GET, PUT, POST, DELETE map to HTTP semantics. Stateless operations enable simple intermediaries and caching. Resources are URIs; state transfer via representations.

**Message Types**:
- **Confirmable (CON)**: Requires ACK. Sender retransmits with exponential backoff if no ACK within timeout (typically 2–8 seconds). Useful for reliable operations (control commands).
- **Non-Confirmable (NON)**: No ACK required. Fire-and-forget. Lower latency, higher loss for unreliable networks.

**Observe Option**: Client sends request with Observe flag; server sends initial response + streams updates when resource state changes. Enables efficient notifications without polling or subscriptions.

**Multicast Support**: Unlike HTTP, CoAP can use UDP multicast for discovery and group commands (e.g., "all lights in zone X turn off"). Reduces bandwidth for broadcast operations.

### Binary Format

CoAP messages are binary (not text like HTTP), reducing size. Typical CoAP packet: 4–40 bytes; HTTP with overhead: 200+ bytes. Critical for devices with meters of antenna drop or seconds of radio-on time.

### No Broker Required

CoAP is client-server, not publish-subscribe. Each device can act as a server. Simpler than MQTT for peer-to-peer or local networks but less flexible for fan-out notifications.

## LoRaWAN — Long Range Wide Area Network

LoRaWAN operates licensed (865–928 MHz, region-specific) spectrum. Designed for battery-powered devices with 7–10 year lifetimes at massive scale (millions of devices per gateway).

### Physical Layer — LoRa Modulation

**Chirp Spread Spectrum (CSS)**: Transmits data as long frequency sweeps (chirps). Slow sweep = long range and interference tolerance; faster sweep = higher data rate. Trade-off: range vs bitrate (50 bits/s to 50 kbps).

**Spreading Factor (SF)**: SF7–SF12 determines symbol duration. SF7 = fastest (1.3 ms/symbol), SF12 = slowest (41 ms/symbol). Higher SF penetrates walls and rain but consumes more time-on-air and energy.

**Bandwidth**: Typically 125 kHz channel width. Modulation is orthogonal; multiple SF6–SF7 transmissions can coexist on same channel without collision (but not SF7+SF8).

### Network Architecture

**Star Topology**: Devices (End Nodes) transmit uplink to any gateway in range. Gateways forward to network server. Network server routes downlink back through gateways. No mesh; gateways are not relayed through devices.

**Adaptive Data Rate (ADR)**: Network server adjusts each device's SF and transmit power based on uplink SNR and collision history. Goal: minimize time-on-air (energy, congestion) while maintaining link margin. Devices must trust network server; cannot override ADR without protocol violation.

**Classes**:
- **Class A**: Device receives downlink in two windows after uplink (1–2 seconds later). Lowest power. Only way to receive downlink; no downlink without uplink.
- **Class B**: Device wakes at beacon intervals (~128 seconds) and opens additional receive slots. Higher power, suitable for some sensor actuators.
- **Class C**: Device listens continuously. Highest power; typically powered mains devices or gateways.

### Regional Parameters

Frequency, SF ranges, duty cycles, and power limits vary by region (EU, US, Japan, etc.). License holders include telecom operators (Telia, Swisscom, KPN) and independent network operators. Interoperability across regions is poor; migrations between regions require device firmware changes.

## Zigbee and Z-Wave — Mesh Protocols for Home Automation

Both Zigbee and Z-Wave are mesh networks (devices relay through each other), not star topology. Designed for home automation: lights, thermostats, locks, sensors.

**Zigbee** (802.15.4): IEEE protocol with 3 device types:
- **Coordinator**: Central hub, starts network.
- **Router**: Can relay packets, has mains power.
- **End Device**: Battery-powered, can only talk to parents; associates with routers.

Mesh self-heals; if a path fails, network repents around obstacles. Uplink (device → hub) routes through routers; downlink (hub → device) uses memory of child relationships.

**Z-Wave**: Alternative mesh using different 900 MHz frequency (US/EU variants). Similar concept: coordinator, routers, end devices. Faster hops (~100 ms) than Zigbee but smaller devices per network (~200 vs 64k in Zigbee).

Both consume ~100 mA when relaying; end devices in sleep mode consume µA. Latency: 100–500 ms for mesh hop; discovery overhead on joining. Interoperability standardized but device support varies.

## BLE — Bluetooth Low Energy

BLE (Bluetooth 5.0+) is designed for intermittent, short-range (10–240 m depending on power class) wireless links between phones, wearables, and personal IoT.

**GATT — Generic Attribute Profile**: Defines services (logical groupings) and characteristics (individual attributes with read/write/notify permissions). A BLE device exposes a GATT server; a phone acts as GATT client, discovering and reading/writing characteristics.

Example: Heart rate monitor with `Heart Rate Service` containing `Heart Rate Measurement` characteristic (notify). Client subscribes; monitor sends notifications when HR changes.

**Advertising**: Devices broadcast small advertisement packets (31 bytes payload) periodically to be discovered. Scanners receive ads and can connect. Higher advertising interval = lower power but slower discovery.

**Connection Model**: Once connected, device enters negotiated connection interval (7.5 ms–4 s), power-on time per interval. Shorter interval = lower latency but higher power. No mesh in BLE native (though Bluetooth mesh layer, 802.15.4-based, adds one).

**Advantages**: Ubiquitous (all phones, IoS/Android/Windows), low power, mature ecosystem. **Disadvantages**: Single-hop range, limited throughput (1 Mbps), no mesh without additional protocol layer.

## Thread and Matter — IP-Based Home Automation

**Thread** (IEEE 802.15.4 mesh, IPv6): A modern mesh network enabling End-to-End encryption, IP routing. Every device is a router; self-healing. Used in Google Nest, Apple HomeKit, and others.

**Matter** (IP-based, application layer): Interoperability standard for smart home. Works over Thread, WiFi, or Ethernet. Defines device types (light, switch, thermostat), clusters (logical device functions), and attribute schemas. All Matter devices speak JSON-RPC-like commands; one phone app works with any Matter device.

Matter + Thread is the emerging standard for future-proof home automation: open, IP-native, no proprietary bridges needed.

## Protocol Selection Criteria

| Criteria | Best Fit |
|----------|----------|
| **Extreme range (>10 km)** | LoRaWAN, Sigfox |
| **Battery 5+ years** | LoRaWAN, Zigbee end devices |
| **Local mesh (<100 devices)** | Zigbee, Z-Wave, Thread |
| **Intermittent connectivity** | CoAP, LoRaWAN |
| **Broker required** | MQTT (easy fan-out) |
| **Mobile / wearable** | BLE |
| **Deterministic latency** | Zigbee (mesh hop ~100ms) |
| **IP through and through** | Thread/Matter, WiFi |
| **Extreme memory (KB)** | CoAP (UDP lightweight) |
| **Cloud-scale telemetry** | MQTT + backend (standard deploym't) |

## Header and Latency Comparison

| Protocol | Typical Packet | Latency | Overhead |
|----------|----------------|---------|----------|
| CoAP | 20–40 B | <100 ms (local) | Minimal |
| MQTT | 100–500 B | <1 s (broker) | TCP/IP + MQTT |
| LoRaWAN | 51 B (MAC+PHY) | 1–3 s + ADR jitter | Regional duty cycle |
| Zigbee | 30–127 B | 100–500 ms (mesh) | 802.15.4 PHY |
| BLE | 31–47 B (adv) | 10–100 ms (direct) | Connection overhead |

## Emerging: AMQP, Advanced MQTT, and MQTT-SN

**MQTT-SN** (MQTT for Sensor Networks) is a variant for UDP/satellite, reducing packet size by pre-registering topic IDs. Less common than MQTT; mainly in proprietary IoT platforms.

**AMQP** appears in enterprise message queues but is rare on IoT edge devices due to overhead and complexity.

## Key Takeaway

Protocols are not universal solutions. MQTT dominates cloud-connected telemetry; CoAP suits extreme constraints; LoRaWAN enables vast, sparse networks; Zigbee/Thread power local mesh; BLE dominates phones. Deep system understanding of your constraints (power, latency, range, scale, connectivity) determines the right choice. Multi-protocol deployments are common: LoRaWAN gateway pushes data to MQTT cloud backend; Thread mesh bridges to Matter cloud; BLE phone proxies to WiFi gateway.