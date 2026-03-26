# Apache Kafka

## Core Concepts

Kafka is a distributed, append-only commit log designed for high-throughput, fault-tolerant, real-time data streaming. It serves as a durable message bus, event store, and stream processing platform.

### Architecture Overview

```
Producers → [Topic: Partition 0] → Consumer Group A (Consumer 1, Consumer 2)
            [Topic: Partition 1] → Consumer Group B (Consumer 3)
            [Topic: Partition 2]
                    ↓
              Broker Cluster (3+ nodes)
              Replication across brokers
```

| Concept   | Description                                                   |
| --------- | ------------------------------------------------------------- |
| Topic     | Named stream of records, a logical channel                    |
| Partition | Ordered, immutable sequence of records within a topic         |
| Offset    | Sequential ID for each record within a partition (0, 1, 2...) |
| Broker    | Single Kafka server that stores partitions and serves clients |
| Cluster   | Group of brokers working together                             |
| Record    | Key + value + timestamp + optional headers                    |

### Topics and Partitions

```bash
# Create topic with 6 partitions and replication factor 3
kafka-topics.sh --bootstrap-server localhost:9092 \
    --create --topic orders \
    --partitions 6 --replication-factor 3

# Describe topic
kafka-topics.sh --bootstrap-server localhost:9092 --describe --topic orders

# Alter partitions (can only increase, never decrease)
kafka-topics.sh --bootstrap-server localhost:9092 \
    --alter --topic orders --partitions 12
```

**Partition count guidelines**:

- More partitions = higher throughput (parallel consumers) but more resource usage
- Desired throughput / per-consumer throughput = minimum partitions
- Can only increase partitions — cannot decrease (would break key ordering)
- All records with the same key go to the same partition (hash(key) % numPartitions)
- `null` key → round-robin across partitions

### Offsets and Retention

```
Partition 0: [0][1][2][3][4][5][6][7][8][9][10][11]...
                              ↑ committed offset (consumer group)
                                          ↑ current position
                                                    ↑ log-end offset
```

- **Retention by time**: Default 7 days (`retention.ms`). Records deleted after this age.
- **Retention by size**: `retention.bytes` — delete oldest segments when total size exceeds limit.
- **Compacted topics**: Keep only the latest value per key (log compaction). Useful for changelogs/state stores.
- **Infinite retention**: Set `retention.ms=-1` for event sourcing / permanent storage.

## Producers

### Configuration

```java
Properties props = new Properties();
props.put("bootstrap.servers", "broker1:9092,broker2:9092");
props.put("key.serializer", "org.apache.kafka.common.serialization.StringSerializer");
props.put("value.serializer", "org.apache.kafka.common.serialization.StringSerializer");

// Durability settings
props.put("acks", "all");              // wait for all ISR replicas
props.put("retries", Integer.MAX_VALUE);
props.put("enable.idempotence", true); // exactly-once per partition

// Performance tuning
props.put("batch.size", 16384);        // bytes per batch
props.put("linger.ms", 5);            // wait up to 5ms to fill batch
props.put("compression.type", "lz4"); // snappy, gzip, lz4, zstd
props.put("buffer.memory", 33554432); // 32 MB send buffer

KafkaProducer<String, String> producer = new KafkaProducer<>(props);
```

### Acks Levels

| Setting         | Durability  | Latency | Description                                           |
| --------------- | ----------- | ------- | ----------------------------------------------------- |
| `acks=0`        | None        | Lowest  | Fire and forget, no broker acknowledgment             |
| `acks=1`        | Leader only | Low     | Leader writes to local log, doesn't wait for replicas |
| `acks=all` (-1) | Full ISR    | Higher  | All in-sync replicas must acknowledge                 |

With `acks=all` + `min.insync.replicas=2` + RF=3: tolerates 1 broker failure with no data loss.

### Idempotent Producer

```java
props.put("enable.idempotence", true); // implies acks=all, retries=MAX
```

Each producer instance gets a Producer ID (PID). Each message gets a sequence number per partition. Brokers deduplicate based on (PID, partition, sequence) — retries don't cause duplicates.

### Sending Records

```java
// Async send with callback
producer.send(new ProducerRecord<>("orders", orderId, orderJson), (metadata, exception) -> {
    if (exception != null) {
        log.error("Send failed for key={}", orderId, exception);
    } else {
        log.debug("Sent to partition={} offset={}", metadata.partition(), metadata.offset());
    }
});

// Sync send (blocks until ack)
RecordMetadata metadata = producer.send(record).get();

// Send to specific partition
new ProducerRecord<>("orders", 2, orderId, orderJson); // partition 2

// Custom partitioner
props.put("partitioner.class", "com.example.RegionPartitioner");
```

## Consumers

### Consumer Groups

Each consumer group is an independent subscriber. Within a group, each partition is assigned to exactly one consumer.

```
Topic: orders (6 partitions)

Consumer Group "payment-service" (3 consumers):
  Consumer 1 → Partition 0, 1
  Consumer 2 → Partition 2, 3
  Consumer 3 → Partition 4, 5

Consumer Group "analytics" (2 consumers):
  Consumer A → Partition 0, 1, 2
  Consumer B → Partition 3, 4, 5
```

- Adding consumers beyond partition count = idle consumers
- Consumer failure → rebalance redistributes partitions to remaining consumers
- Each group tracks its own offsets independently

### Consumer Configuration

```java
Properties props = new Properties();
props.put("bootstrap.servers", "broker1:9092");
props.put("group.id", "payment-service");
props.put("key.deserializer", "org.apache.kafka.common.serialization.StringDeserializer");
props.put("value.deserializer", "org.apache.kafka.common.serialization.StringDeserializer");

// Offset management
props.put("enable.auto.commit", false);        // manual offset control
props.put("auto.offset.reset", "earliest");    // or "latest" for new groups

// Session management
props.put("session.timeout.ms", 30000);        // consumer considered dead after this
props.put("heartbeat.interval.ms", 10000);     // heartbeat frequency (< 1/3 session timeout)
props.put("max.poll.interval.ms", 300000);     // max time between poll() calls
props.put("max.poll.records", 500);            // max records per poll

KafkaConsumer<String, String> consumer = new KafkaConsumer<>(props);
```

### Consuming Records

```java
consumer.subscribe(Arrays.asList("orders"));

while (true) {
    ConsumerRecords<String, String> records = consumer.poll(Duration.ofMillis(100));
    for (ConsumerRecord<String, String> record : records) {
        processOrder(record.key(), record.value());
    }
    // Manual commit after processing
    consumer.commitSync();
    // Or async commit
    consumer.commitAsync((offsets, exception) -> {
        if (exception != null) log.warn("Commit failed", exception);
    });
}

// Commit specific offsets
Map<TopicPartition, OffsetAndMetadata> offsets = new HashMap<>();
offsets.put(new TopicPartition("orders", 0), new OffsetAndMetadata(lastProcessedOffset + 1));
consumer.commitSync(offsets);

// Seek to specific offset (replay)
consumer.seek(new TopicPartition("orders", 0), 0); // back to beginning
```

### Rebalancing Strategies

| Strategy                    | Behavior                                      | Use Case               |
| --------------------------- | --------------------------------------------- | ---------------------- |
| `RangeAssignor`             | Assign ranges of partitions per topic         | Default, simple        |
| `RoundRobinAssignor`        | Distribute partitions evenly across consumers | Better balance         |
| `StickyAssignor`            | Minimize partition movement during rebalance  | Reduce reprocessing    |
| `CooperativeStickyAssignor` | Incremental rebalance — no stop-the-world     | Production recommended |

```java
props.put("partition.assignment.strategy",
    "org.apache.kafka.clients.consumer.CooperativeStickyAssignor");
```

**Cooperative rebalancing** (incremental): Consumers don't revoke all partitions during rebalance — only the ones that need to move. This eliminates the "stop-the-world" pause of eager rebalancing.

## Brokers and Replication

### Replication

Each partition has one **leader** and zero or more **followers**. All reads and writes go through the leader. Followers replicate from the leader.

```
Partition 0:  Leader=Broker1, Followers=Broker2,Broker3
Partition 1:  Leader=Broker2, Followers=Broker1,Broker3
Partition 2:  Leader=Broker3, Followers=Broker1,Broker2
```

### ISR (In-Sync Replicas)

The ISR is the set of replicas that are fully caught up with the leader.

- Follower falls out of ISR if it hasn't fetched for `replica.lag.time.max.ms` (default 30s)
- `acks=all` only waits for ISR replicas
- `min.insync.replicas`: Minimum ISR size for `acks=all` writes to succeed. Set to 2 with RF=3 for durability.
- If ISR shrinks below `min.insync.replicas`, producers get `NotEnoughReplicasException`

### KRaft (Kafka Raft)

Kafka 3.3+ replaces ZooKeeper with a built-in Raft-based metadata quorum.

```properties
# KRaft mode configuration
process.roles=broker,controller    # combined mode (or separate)
node.id=1
controller.quorum.voters=1@broker1:9093,2@broker2:9093,3@broker3:9093
controller.listener.names=CONTROLLER
```

**KRaft advantages over ZooKeeper**:

- Simpler operations (no separate ZK cluster)
- Faster controller failover (seconds vs minutes)
- Better scalability (millions of partitions)
- Single security model

### Key Broker Settings

| Setting                      | Default      | Purpose                                    |
| ---------------------------- | ------------ | ------------------------------------------ |
| `num.partitions`             | 1            | Default partitions for auto-created topics |
| `default.replication.factor` | 1            | Default RF for auto-created topics         |
| `min.insync.replicas`        | 1            | Min ISR for acks=all                       |
| `log.retention.hours`        | 168 (7 days) | How long to keep records                   |
| `log.segment.bytes`          | 1 GB         | Size of each log segment file              |
| `message.max.bytes`          | 1 MB         | Max message size                           |
| `num.io.threads`             | 8            | I/O threads for disk operations            |
| `num.network.threads`        | 3            | Threads for network requests               |

## Schema Registry

Centralizes schema management for Kafka topics. Supports Avro, Protobuf, and JSON Schema.

```bash
# Register a schema
curl -X POST -H "Content-Type: application/vnd.schemaregistry.v1+json" \
    --data '{"schema": "{\"type\":\"record\",\"name\":\"Order\",\"fields\":[{\"name\":\"id\",\"type\":\"string\"},{\"name\":\"amount\",\"type\":\"double\"}]}"}' \
    http://localhost:8081/subjects/orders-value/versions

# Get latest schema
curl http://localhost:8081/subjects/orders-value/versions/latest

# Check compatibility
curl -X POST -H "Content-Type: application/vnd.schemaregistry.v1+json" \
    --data '{"schema": "..."}' \
    http://localhost:8081/compatibility/subjects/orders-value/versions/latest
```

### Compatibility Modes

| Mode                 | Allowed Changes                                                       |
| -------------------- | --------------------------------------------------------------------- |
| `BACKWARD` (default) | New schema can read old data. Can add optional fields, delete fields. |
| `FORWARD`            | Old schema can read new data. Can delete optional fields, add fields. |
| `FULL`               | Both backward and forward compatible                                  |
| `NONE`               | No compatibility check                                                |

## Kafka Streams

Client library for building stream processing applications. No separate cluster needed — runs within your JVM.

```java
StreamsBuilder builder = new StreamsBuilder();

// Stream from topic
KStream<String, Order> orders = builder.stream("orders",
    Consumed.with(Serdes.String(), orderSerde));

// Stateless operations
KStream<String, Order> filtered = orders
    .filter((key, order) -> order.getAmount() > 100)
    .mapValues(order -> enrichOrder(order))
    .peek((key, order) -> log.info("Processing order: {}", key));

// Branch
Map<String, KStream<String, Order>> branches = orders.split(Named.as("split-"))
    .branch((key, order) -> order.isPriority(), Branched.as("priority"))
    .branch((key, order) -> !order.isPriority(), Branched.as("standard"))
    .noDefaultBranch();

// Stateful: aggregation with windowing
KTable<Windowed<String>, Long> orderCounts = orders
    .groupByKey()
    .windowedBy(TimeWindows.ofSizeWithNoGrace(Duration.ofMinutes(5)))
    .count(Materialized.as("order-counts"));

// Join stream to table (enrichment)
KTable<String, Customer> customers = builder.table("customers");
KStream<String, EnrichedOrder> enriched = orders.join(customers,
    (order, customer) -> new EnrichedOrder(order, customer));

// Write to output topic
enriched.to("enriched-orders", Produced.with(Serdes.String(), enrichedOrderSerde));

KafkaStreams streams = new KafkaStreams(builder.build(), config);
streams.start();
```

### State Stores

Kafka Streams maintains local state in RocksDB, backed by changelog topics for fault tolerance.

```java
// Interactive queries: query local state store
ReadOnlyWindowStore<String, Long> store =
    streams.store(StoreQueryParameters.fromNameAndType("order-counts", QueryableStoreTypes.windowStore()));
```

## Kafka Connect

Framework for streaming data between Kafka and external systems without writing code.

```json
{
  "name": "postgres-source",
  "config": {
    "connector.class": "io.debezium.connector.postgresql.PostgresConnector",
    "database.hostname": "postgres",
    "database.port": "5432",
    "database.user": "replicator",
    "database.password": "${secrets:pg-password}",
    "database.dbname": "orders",
    "topic.prefix": "cdc",
    "table.include.list": "public.orders,public.customers",
    "plugin.name": "pgoutput",
    "transforms": "route",
    "transforms.route.type": "org.apache.kafka.connect.transforms.RegexRouter",
    "transforms.route.regex": "cdc\\.public\\.(.*)",
    "transforms.route.replacement": "$1-events"
  }
}
```

**Source connectors**: Databases (Debezium CDC), files, HTTP APIs → Kafka  
**Sink connectors**: Kafka → Elasticsearch, S3, JDBC databases, HDFS

### Single Message Transforms (SMTs)

```json
"transforms": "unwrap,route,timestamp",
"transforms.unwrap.type": "io.debezium.transforms.ExtractNewRecordState",
"transforms.route.type": "org.apache.kafka.connect.transforms.RegexRouter",
"transforms.timestamp.type": "org.apache.kafka.connect.transforms.InsertField$Value",
"transforms.timestamp.timestamp.field": "processed_at"
```

## Transactions (Exactly-Once Semantics)

End-to-end exactly-once processing: consume → process → produce atomically.

```java
// Producer config for transactions
props.put("transactional.id", "order-processor-1"); // unique per instance
props.put("enable.idempotence", true);

producer.initTransactions();

while (true) {
    ConsumerRecords<String, String> records = consumer.poll(Duration.ofMillis(100));
    producer.beginTransaction();
    try {
        for (ConsumerRecord<String, String> record : records) {
            // Process and produce to output topic
            producer.send(new ProducerRecord<>("output", record.key(), process(record.value())));
        }
        // Commit consumer offsets as part of the transaction
        producer.sendOffsetsToTransaction(
            currentOffsets(records), consumer.groupMetadata());
        producer.commitTransaction();
    } catch (Exception e) {
        producer.abortTransaction();
    }
}
```

Requirements: `transactional.id` set, idempotence enabled, consumer `isolation.level=read_committed`.

## Operational Commands

```bash
# Consumer group management
kafka-consumer-groups.sh --bootstrap-server localhost:9092 --list
kafka-consumer-groups.sh --bootstrap-server localhost:9092 --describe --group payment-service

# Reset offsets (group must be stopped)
kafka-consumer-groups.sh --bootstrap-server localhost:9092 \
    --group payment-service --topic orders \
    --reset-offsets --to-earliest --execute

# Performance testing
kafka-producer-perf-test.sh --topic test --num-records 1000000 --record-size 1024 \
    --throughput -1 --producer-props bootstrap.servers=localhost:9092

kafka-consumer-perf-test.sh --bootstrap-server localhost:9092 \
    --topic test --messages 1000000

# Log compaction status
kafka-log-dirs.sh --bootstrap-server localhost:9092 --describe --topic-list orders
```
