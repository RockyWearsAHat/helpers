# MQTT — Publish/Subscribe for Constrained Networks

## Overview

MQTT (Message Queuing Telemetry Transport) is a lightweight, publish/subscribe protocol designed for IoT and constrained networks. Standardized as OASIS MQTT 3.1.1 (2014) and modernized with MQTT 5.0 (2019), it runs over TCP and prioritizes low bandwidth, minimal overhead, and predictable behavior in high-latency or unreliable connections.

The protocol's core abstraction: clients publish messages to topics, brokers route messages to subscribed clients. No direct peer-to-peer addressing. This decoupling is MQTT's strength — publishers and subscribers need not know each other, enabling dynamic, asynchronous system composition.

## The Protocol: Core Mechanics

### Publish/Subscribe Model

A **publisher** sends a message tagged with a topic name (e.g., `sensor/temperature`). The **broker** receives it and forwards it to all **subscribers** listening to that topic (wildcard subscriptions supported: `sensor/+` matches single level, `sensor/#` matches multi-level). Publishers and subscribers connect independently to the broker; neither needs to know the other exists.

**Implication**: Natural fit for IoT sensor networks and event-driven systems where topology is dynamic or unknown at design time.

### Quality of Service (QoS)

MQTT guarantees message delivery semantics, negotiated per subscription:

| QoS | Name            | Guarantee                                  | Use Case                              |
|-----|------------------|---------------------------------------------|---------------------------------------|
| 0   | At most once     | Fire-and-forget; TCP provides no guarantee | Telemetry, sensor samples, non-critical |
| 1   | At least once    | Broker persists; client retries until ACK  | Financial, order, accounting data    |
| 2   | Exactly once     | Four-way handshake (PUBLISH → PUBREC → PUBREL → PUBCOMP) | Mission-critical, no duplication allowed |

**Trade-off**: QoS 2 requires 4-message round-trip per delivery; QoS 0 is single packet. Most IoT deployments use QoS 0 or 1. QoS 2 adds per-message state at broker and client — significant overhead at scale.

**Durability**: QoS 1/2 implies broker-side persistence. Broker caches undelivered messages for disconnected subscribers (session state). QoS 0 messages are dropped if subscriber offline.

### Session Persistence and Offline Messaging

A client connection is identified by a **client ID**. If the client disconnects unexpectedly:

- **Clean session=true** (default, MQTT 3.1.1): Broker discards session state. Reconnection is fresh.
- **Clean session=false**: Broker persists subscriptions and QoS 1/2 messages. Reconnected client receives buffered messages.

**MQTT 5.0 refinement**: `sessionExpiryInterval` replaces binary clean-session logic. Broker keeps state for N seconds after disconnect. Useful for mobile clients that temporarily lose connectivity.

### Retained Messages

A message can be marked **retained** when published. The broker stores the last retained message per topic. Any client subscribing to that topic immediately receives the retained message, regardless of publication time. Enables a form of **state cache**: subscribe to `status/device` and get the current status without waiting.

**Memory implication**: Each topic can have one retained message. Careless use (many high-volume topics with retention) can exhaust broker memory.

## MQTT 5.0 Enhancements

MQTT 5.0 added:

- **Shared subscriptions**: Multiple clients subscribe as a group; broker load-balances messages across the group (instead of fanning out to all). Enables consumer groups like Kafka.
- **Message expiry**: Publisher can set how long a message is valid. Broker discards expired messages; subscribers never see stale data.
- **Topic aliases**: Client and broker negotiate numeric aliases for long topic names, reducing bandwidth.
- **Reason codes**: Operations return codes indicating why they failed (e.g., topic filter invalid, quota exceeded).
- **User properties**: Arbitrary key-value metadata in messages; application-layer signaling without protocol change.

## Topic Design Patterns

Topics are arbitrary UTF-8 strings. Conventions emerge:

```
Building/Floor/Room/Sensor/Type     # hierarchical: house automation
vehicle/{id}/gps/latitude           # templates: fleet tracking
alert/severity/{level}/{source}     # query-friendly: monitoring
```

**Multi-level wildcards** (`#`) must terminate a subscription; `sensor/#/status` is invalid. Use deliberate hierarchy to avoid over-broad subscriptions.

## Security

- **Authentication**: Username/password (unencrypted by default); TLS encrypts credentials in transit.
- **Authorization**: Broker enforces topic ACLs per client (subscribe/publish restrictions).
- **Encryption**: TLS 1.2+ wraps the entire connection. Some deployments use certificates; others rely on infrastructure-level encryption (VPN, containerized networks).
- **Payload confidentiality**: Application can additionally encrypt payloads (end-to-end), but MQTT itself doesn't mandate it.

**Gap**: No built-in field-level encryption or key rotation API. Application must handle.

## MQTT vs. Alternatives

| Protocol   | Model              | Overhead   | Latency    | Use Case                  |
|------------|-------------------|------------|-----------|---------------------------|
| MQTT       | Pub/Sub           | ~2 bytes   | 10-100ms  | IoT sensors, telemetry    |
| AMQP       | Pub/Sub + RPC     | ~8 bytes   | <100ms    | Enterprise messaging, queues |
| HTTP       | Request/response  | ~100 bytes | >100ms    | Web, REST APIs            |
| CoAP       | Request/response  | ~4 bytes   | <100ms    | Ultra-constrained IoT     |
| WebSocket  | Bidirectional     | ~2 bytes   | <50ms     | Browser real-time         |

**Decision tree**: Ultra-low power (99% sleep)? → CoAP. Existing broker infrastructure? → AMQP (RabbitMQ) or MQTT. Browser clients? → WebSocket. Pure request/response? → HTTP/REST.

## Broker Implementations

- **Mosquitto** (C, FOSS): Minimal, embeddable. Single-threaded event loop. ~25MB binary.
- **EMQX** (Erlang): Clustered, scales horizontally. Built-in rule engine for transformations.
- **HiveMQ** (Java): Enterprise, commercial support. Plugin architecture.
- **AWS IoT Core / Azure IoT Hub**: Cloud-hosted, managed MQTT.

Single broker handles 100K–1M concurrent connections depending on hardware and message rate.

## Scaling Patterns

### Bridge / Cluster

Multiple brokers subscribe to each other's `$SYS/#` topics, forming a mesh. Messages published to broker A are forwarded to all connected brokers. Clients switch between brokers for failover.

### Tree Hierarchy

Brokers form a tree: leaf brokers handle client connections; upstream brokers aggregate and route. Central broker acts as hub. Reduces message fanout at scale.

### Partition by Topic

Different topics live on different brokers. Clients must know which broker holds their topic. Adds operational complexity but isolates failure domains.

## Limitations

- **No request/response**: No built-in correlation. Implement with reply-to topic convention or application ID headers.
- **Broker is single point of trust**: All routing logic is broker-side. Misconfigured broker can leak data.
- **No explicit flow control**: QoS provides delivery guarantees but not throughput control. Broker can be overwhelmed by fast publishers.
- **Topic naming is convention, not schema**: No schema registry or validation. Typos in topic names silently fail.

**Common pitfall**: Designing with QoS 2 everywhere "for safety" without measuring broker overhead. QoS 2 can saturate broker processing before network fills.

## See Also

- [iot-embedded-patterns.md](iot-embedded-patterns.md) — Embedded systems communication
- [distributed-messaging.md](distributed-messaging.md) — Message queue patterns
- [web-websockets.md](web-websockets.md) — Bidirectional web communication