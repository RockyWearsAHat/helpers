# Transactional Outbox Pattern — Solving the Dual Write Problem

## Overview

The **transactional outbox pattern** solves a critical problem in event-driven systems: ensuring that when a service writes data to its own database, it also publishes an event atomically. Without this, events can be lost if the publisher crashes after writing data but before publishing the event, leading to inconsistent systems.

The pattern works by writing both the business data AND the event to the same database in a single transaction, then using a background publisher (polling, CDC, or streaming) to reliably publish events to the message broker.

---

## The Dual Write Problem

### The Unreliable Approach

Naive event publishing violates atomicity:

```python
def create_order(order_data):
    # Step 1: Write to database
    db.execute("INSERT INTO orders ...")
    
    # Step 2: Publish event
    message_broker.publish("OrderCreated", order_data)
```

**Failure scenarios:**

1. **Database succeeds, publish fails:** Event lost, data persists inconsistently
   ```
   Database: ✓ Order written
   Event: ✗ Failed to reach broker (network issue, broker down)
   Result: Inconsistency—data exists but downstream systems never react
   ```

2. **Process crashes between steps:**
   ```
   Database: ✓ Order written, commit acknowledged
   Service: ✗ Crashes before publish
   Result: Event never recovered; downstream misses it
   ```

3. **No idempotency:** Retry mechanisms don't know if the event was published before the crash

**This violates eventual consistency guarantees.** Saga choreography requires reliable event publishing.

### Why Transactions Alone Don't Work

You might try:
```python
tx = db.begin_transaction()
try:
    db.insert_order(tx, order_data)
    message_broker.publish(...)  # Not transactional!
    tx.commit()
except:
    tx.rollback()
```

**Problem:** Database and message broker are separate systems. A distributed transaction across them requires two-phase commit (2PC), which is:
- Expensive and slow (blocking locks)
- Failure-prone (coordinator failures leave participants in unknown state)
- Incompatible with modern streaming (Kafka cannot participate in 2PC)

---

## Outbox Table Design

### The Core Idea

Instead of publishing directly to the broker, write events to an **outbox table** in the same database and transaction as the business data:

```sql
CREATE TABLE orders (
  id INT PRIMARY KEY,
  customer_id INT,
  status VARCHAR(50),
  created_at TIMESTAMP
);

CREATE TABLE outbox (
  id INT PRIMARY KEY AUTO_INCREMENT,
  aggregate_id INT,
  aggregate_type VARCHAR(50),
  event_type VARCHAR(255),
  event_data JSON,
  published BOOLEAN DEFAULT FALSE,
  created_at TIMESTAMP DEFAULT NOW()
);
```

### Atomic Write

Now both business data and event go into one transaction:

```python
def create_order(order_data):
    with db.transaction():
        # Write business data
        order = db.insert("orders", order_data)
        
        # Write to outbox (same transaction)
        db.insert("outbox", {
            "aggregate_id": order.id,
            "aggregate_type": "Order",
            "event_type": "OrderCreated",
            "event_data": json.dumps(order.to_dict())
        })
        # If either insert fails, entire transaction rolls back
```

**Guarantee:** If the service crashes, the database is in a consistent state. Either both writes succeeded (and can be recovered) or both rolled back (and nothing happened).

### Outbox Table Characteristics

**Columns:**
- `id`: Unique outbox entry ID (enables ordering)
- `aggregate_id`: Reference to the business entity (e.g., order ID)
- `aggregate_type`: Type of entity (e.g., "Order", "Payment")
- `event_type`: Semantic event name (e.g., "OrderCreated")
- `event_data`: Full event payload (JSON)
- `published`: Boolean flag (true = sent to broker)
- `created_at`: Insertion timestamp (used for ordering and debugging)

**Indexes:**
```sql
CREATE INDEX idx_outbox_published ON outbox(published, created_at);
CREATE INDEX idx_outbox_aggregate ON outbox(aggregate_type, aggregate_id);
```

The first index is critical—the publisher scans for unpublished rows frequently.

---

## Polling Publisher

### Periodic Scanner

A background job (running in the service, or in a dedicated worker) polls the outbox table periodically:

```python
import time
from datetime import datetime

def polling_publisher(db, broker, poll_interval=5):
    """Poll outbox every N seconds; publish and mark as done"""
    while True:
        try:
            # Fetch unpublished events, ordered by creation
            rows = db.query("""
                SELECT * FROM outbox 
                WHERE published = FALSE 
                ORDER BY created_at ASC 
                LIMIT 100
            """)
            
            for row in rows:
                try:
                    # Publish to broker
                    broker.publish(row['event_type'], row['event_data'])
                    
                    # Mark as published
                    db.execute(
                        "UPDATE outbox SET published = TRUE WHERE id = ?",
                        row['id']
                    )
                except Exception as e:
                    logger.error(f"Failed to publish outbox {row['id']}: {e}")
                    # Retry on next poll
            
            time.sleep(poll_interval)
        except Exception as e:
            logger.error(f"Polling error: {e}")
            time.sleep(poll_interval)
```

### Trade-offs

**Pros:**
- Simple to implement (no CDC infrastructure required)
- Works with any database
- Easy to debug (visible in outbox table; can replay manually)

**Cons:**
- Latency: Poll interval (5-30s typical) introduces delay before events are published
- Database load: Repeated full table scans
- Clock skew: If servers have misaligned clocks, ordering may break
- Polling overhead: Queries run even if no new events

---

## CDC-Based Publishing (Debezium, etc.)

### How CDC Works

Instead of polling, **Change Data Capture** (CDC) streams database changes directly to Kafka:

1. CDC tool (e.g., Debezium) reads the database transaction log (WAL) in real-time
2. Translates each INSERT/UPDATE into a Kafka event
3. Publishes to a CDC topic

```
Database transaction log
  ├─ INSERT INTO outbox (aggregate_id=123, event_type='OrderCreated', ...)
  └─ CDC reads WAL → Publishes to Kafka topic "db.outbox"
```

### Publisher Service

A simpler publisher consumes the CDC stream and republishes to business topics:

```python
def cdc_republisher(cdc_consumer, business_broker):
    """Consume CDC stream; republish to business topics"""
    for message in cdc_consumer:
        outbox_record = parse_cdc(message)
        
        # Skip if already published
        if outbox_record['published']:
            continue
        
        # Republish to business topic
        business_broker.publish(
            outbox_record['event_type'],
            outbox_record['event_data']
        )
        
        # Mark as published in database
        db.execute(
            "UPDATE outbox SET published = TRUE WHERE id = ?",
            outbox_record['id']
        )
```

### Trade-offs

**Pros:**
- Real-time: CDC reads transaction log immediately (typically <100ms)
- Efficient: No repeated polling; streams only changes
- Automatic: CDC tool handles ordering and durability

**Cons:**
- Operational complexity: Requires CDC infrastructure (Debezium connector, Kafka Connect cluster)
- CDC tool reliability: Another system to monitor and operate
- Database-specific: Needs WAL/redo log (not all databases support CDC)
- Initial snapshot: First run can be expensive for large tables

**Best for:** High-volume systems where latency and database load matter.

---

## Idempotent Consumers

### The Challenge

Events published from the outbox may be delivered multiple times if the republisher crashes after publishing but before marking `published = TRUE`:

```
Publisher: Publishes OrderCreated to Kafka (✓)
Publisher: Attempts to update outbox.published = TRUE
Publisher: Crashes before update completes
Recovery: Publisher restarts, sees unpublished event
Publisher: Publishes OrderCreated again (duplicate)
```

**Consumers must handle duplicates safely.**

### Idempotency via Event ID

Each outbox event must carry a globally unique, deterministic ID:

```python
# In outbox table
db.insert("outbox", {
    "id": f"order-{order.id}-created",  # Deterministic
    "aggregate_id": order.id,
    "event_type": "OrderCreated",
    "event_data": {...}
})
```

Consumers deduplicate:

```python
class DownstreamService:
    def on_order_created(self, event):
        event_id = event.get("id")
        
        # Check if already processed
        if db.get_processed_event(event_id):
            return  # Idempotent: skip
        
        # Process event
        self.handle_order_created(event)
        
        # Mark as processed
        db.mark_processed_event(event_id)
```

### Storing Idempotency Markers

Pair idempotency deduplication with the business effect in an atomic transaction:

```python
def on_order_created(self, event):
    with db.transaction():
        if db.get_processed_event(event.id):
            return  # Already done
        
        # Write business effect
        self.reserve_inventory(event)
        
        # Mark as processed (same transaction)
        db.mark_processed_event(event.id)
```

If the service crashes, either both the effect and deduplication record exist, or neither does.

---

## Ordering Guarantees

### Single Aggregate Ordering

Events from the same aggregate (e.g., a single order) must be delivered in order:

```
Order 123: created → paid → shipped → delivered
Events must arrive at consumers in this order
```

**Implementation:** Partition events by aggregate ID:

```python
# On publish, extract aggregate ID
partition_key = event['aggregate_id']
broker.publish(event, partition=hash(partition_key) % num_partitions)
```

Kafka (or similar) ensures all messages with the same partition key stay ordered.

### Multiple Aggregates (Weak Ordering)

Events from different aggregates may arrive out of order—this is acceptable:

```
Order 123: created → paid
Order 456: created (arrives before Order 123 is paid)
```

Consumers should handle out-of-order events for different aggregates. If strict global ordering is required, use a single partition (trading throughput for order).

---

## Cleanup Strategies

### Why Cleanup Matters

The outbox table grows unbounded. Old events waste disk space and slow queries:

```sql
-- Thousands of rows, mostly old and published
SELECT COUNT(*) FROM outbox WHERE published = TRUE;
-- 2,847,549 rows
```

### Time-Based Cleanup

Delete old published events after a retention period:

```python
def cleanup_outbox(db, retention_days=7):
    """Delete published events older than 7 days"""
    cutoff = datetime.now() - timedelta(days=retention_days)
    db.execute("""
        DELETE FROM outbox 
        WHERE published = TRUE 
        AND created_at < ?
    """, cutoff)
```

**Schedule:** Run periodically (e.g., daily at 2 AM) via cron or task scheduler.

**Retention period trade-off:**
- Short (1-3 days): Saves space but loses audit trail quickly
- Long (30+ days): Better for debugging and replays, but larger table
- Very long (1 year+): Legal/compliance requirement for some domains

### Archival vs. Deletion

For regulated environments, archive old events instead of deleting:

```python
def archive_outbox(db, archive_storage, retention_days=7):
    """Archive published events to long-term storage"""
    cutoff = datetime.now() - timedelta(days=retention_days)
    
    rows = db.query("""
        SELECT * FROM outbox 
        WHERE published = TRUE 
        AND created_at < ?
    """, cutoff)
    
    # Write to archive (S3, data lake, etc.)
    archive_storage.write_batch(rows)
    
    # Delete from live table
    db.execute("""
        DELETE FROM outbox 
        WHERE published = TRUE 
        AND created_at < ?
    """, cutoff)
```

---

## Implementation with Kafka Connect

### Debezium Source Connector

Deploy Debezium to stream the outbox table to Kafka:

```json
{
  "name": "outbox-connector",
  "config": {
    "connector.class": "io.debezium.connector.mysql.MySqlConnector",
    "database.hostname": "mysql-server",
    "database.port": 3306,
    "database.user": "debezium",
    "database.password": "secret",
    "database.server.id": 184054,
    "database.server.name": "myserver",
    "table.include.list": "myapp.outbox",
    "plugin.name": "pgoutput"
  }
}
```

Debezium publishes to topic `myserver.myapp.outbox` (source topic).

### Stream Processor (Kafka Streams / Flink)

Transform outbox records into domain events:

```python
from pyspark.sql import SparkSession

spark = SparkSession.builder.appName("OutboxRepublisher").getOrCreate()

outbox_stream = spark \
    .readStream \
    .format("kafka") \
    .option("kafka.bootstrap.servers", "kafka:9092") \
    .option("subscribe", "myserver.myapp.outbox") \
    .load()

def transform_outbox(row):
    """Convert CDC outbox record to domain event"""
    payload = row['after']  # Debezium includes 'after' (new state)
    return {
        "event_type": payload['event_type'],
        "data": payload['event_data'],
        "timestamp": payload['created_at']
    }

domain_events = outbox_stream \
    .select(transform_outbox(col("value")).alias("event")) \
    .select("event.event_type", "event.data", "event.timestamp")

domain_events \
    .writeStream \
    .format("kafka") \
    .option("kafka.bootstrap.servers", "kafka:9092") \
    .option("topic", "domain-events") \
    .option("checkpointLocation", "/tmp/checkpoint") \
    .start()
```

Republished events appear on the `domain-events` topic for consumers to subscribe to.

### Advantages of Kafka Connect

- **Exactly-once semantics** (with idempotent writes)
- **Scalable:** Handles millions of events/sec
- **Fault-tolerant:** Offsets stored in Kafka; retries automatic
- **No polling:** Real-time, event-driven

---

## Monitoring and Observability

### Key Metrics

```
1. Outbox publish lag: MAX(created_at - published_at) for unpublished rows
   Alert if > 1 minute (indicates publisher failure)

2. Outbox size: COUNT(*) WHERE published = FALSE
   Alert if growing unbounded

3. Publisher error rate: Errors / total publish attempts
   Alert if > 0.1% (indicates systematic failures)

4. Idempotency deduplication rate: Duplicate events / total events
   Track to understand retry patterns
```

### Operational Runbook

**Symptom: Outbox falling behind (many unpublished rows)**

```
1. Check publisher service logs
   $ kubectl logs -f deployment/publisher-service
   
2. Verify broker connectivity
   $ telnet kafka-broker 9092
   
3. Check database performance
   SELECT COUNT(*) FROM outbox WHERE published = FALSE;
   
4. Restart publisher if stuck
   $ kubectl rollout restart deployment/publisher-service
```

**Symptom: Old published events not cleaned up**

```
1. Verify cleanup job ran
   $ SELECT MAX(created_at) FROM outbox;
   
2. Check cleanup job logs
   $ kubectl logs -f job/cleanup-outbox
   
3. Manually trigger cleanup
   DELETE FROM outbox WHERE published = TRUE AND created_at < NOW() - INTERVAL 7 DAY;
```

---

## See Also

- [patterns-saga-choreography.md](patterns-saga-choreography.md) — Choreographed sagas depend on reliable outbox publishing
- [architecture-domain-events.md](architecture-domain-events.md) — Event design and publication patterns
- [patterns-idempotency.md](patterns-idempotency.md) — Idempotent operation semantics