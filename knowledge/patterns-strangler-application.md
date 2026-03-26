# Strangler Application Pattern — Routing, Dual-Write, and Canary Migration

## Overview

The **strangler application pattern** (or "strangler fig") incrementally replaces a legacy monolith by routing new traffic to a new service while the old system continues operating. Like a fig tree strangling its host, the new service gradually takes over until the old system is decommissioned.

Rather than a risky rewrite, strangler enables a measured, low-risk modernization where confidence builds with each percentage of migrated traffic.

This note focuses on **practical implementation details**: routing strategies, dual-write migration, data synchronization, and measuring migration confidence.

---

## Routing Strategies

### 1. Path-Based Routing

Route requests based on URL path prefix:

```
GET /api/users/:id        → New UserService
POST /api/users           → New UserService
GET /api/orders/:id       → Legacy OrderService (not yet migrated)
POST /api/orders          → Legacy OrderService
```

**API Gateway configuration (nginx):**

```nginx
upstream new_service {
  server user-service:8080;
}

upstream legacy_service {
  server legacy-monolith:3000;
}

server {
  listen 80;
  
  # New service: routes /api/users/* to new service
  location /api/users/ {
    proxy_pass http://new_service;
  }
  
  # Everything else: legacy
  location / {
    proxy_pass http://legacy_service;
  }
}
```

**Pros:**
- Simple to understand and implement
- Perfect for vertical slices (entire domains move together)
- No traffic duplica‌tion or shadowing needed

**Cons:**
- Assumes domain boundaries are clean (often not true in monoliths)
- Cannot split a single endpoint between systems
- Requires refactoring gateway rules as you migrate

**Use when:** You're extracting entire domains (Users, Orders, etc.) one at a time.

### 2. Header-Based Routing

Route based on request headers or cookies (feature flags):

```
GET /api/orders/123
  X-Canary: true        → New OrderService
  [no header]           → Legacy OrderService
```

**Gateway configuration:**

```nginx
location /api/orders/ {
  if ($http_x_canary = "true") {
    proxy_pass http://new_orders_service;
  }
  if ($http_x_canary != "true") {
    proxy_pass http://legacy_service;
  }
}
```

**Client-side opt-in (E2E tests, internal engineers):**

```javascript
// In test client or internal tools
$http.defaults.headers.common['X-Canary'] = 'true';
// Requests now go to new service
```

**Pros:**
- Enables opt-in testing without percentage-based rollout
- Easy for developers to test new service manually
- Can route individual users/test accounts

**Cons:**
- Requires client cooperation (header must be passed through)
- Not suitable for gradual percentage rollout (need multiple conditions)

**Use when:** You want early opt-in from internal teams before production rollout.

### 3. User/Account-Based Routing

Route based on user properties (account tier, region, cohort):

```
GET /api/orders/123
  user.region == "us-west"    → New OrderService (if feature enabled for region)
  user.tier == "beta"         → New OrderService (beta users canary test)
  [everyone else]             → Legacy OrderService
```

**Implementation:**

```python
def route_order_request(user, request):
    """Route based on user cohort"""
    # Experiment: 10% of US users
    if user.region == "us-west" and hash(user.id) % 100 < 10:
        return route_to("new_order_service", request)
    
    # Beta cohort always gets new service
    if user.cohort == "beta":
        return route_to("new_order_service", request)
    
    # Default: legacy
    return route_to("legacy_service", request)
```

**Pros:**
- Deterministic per user (same user always goes to same service, reproducible)
- Enables progressive rollout (10% → 25% → 50% → 100%)
- Works well with feature flags and A/B testing frameworks

**Cons:**
- Requires user context (not available for unauthenticated endpoints)
- Requires synchronization across stateless routers
- Hash function must be stable (same user always hashes to same bucket)

**Use when:** You want gradual canary rollout to real users.

### 4. Time-Based Routing

Route based on time-of-day or request timestamp:

```
Monday-Friday 9 AM - 5 PM    → New service (business hours monitoring)
After 5 PM / weekends        → Legacy (safer backup)
```

**Rarely used alone, but useful in combination:**

```python
def route_critical_service(request):
    """Strict routing for high-traffic periods"""
    hour = datetime.now().hour
    
    if 9 <= hour < 17:  # Business hours
        return route_to("new_service", request)
    else:
        return route_to("legacy_service", request)
```

**Use when:** You want to test new service during controlled hours with ops team standing by.

---

## Dual-Write Migration

### The Problem

Simply routing requests to the new service doesn't migrate data. The new service needs a copy (or reference) to the old data.

Options:

1. **Read-through** (read old, only if not in new)
2. **Dual-write** (write to both systems simultaneously)
3. **CDC-based sync** (stream changes from legacy database to new)

### Dual-Write Pattern

Write every mutation to both the legacy system and the new system:

```python
def update_order_status(order_id, new_status):
    """Update in both old and new systems"""
    
    # Write to legacy
    legacy_db.execute(
        "UPDATE orders SET status = ? WHERE id = ?",
        new_status, order_id
    )
    
    # Write to new service
    new_service.update_order(order_id, status=new_status)
    
    return {"status": "ok"}
```

**Challenges:**

1. **Write failures:** If new service write fails, should you retry? Fail the request? Async queue the write?
2. **Consistency:** Both systems must interpret updates identically
3. **Backwards compatibility:** Legacy system must accept new data formats

### Dual-Write Strategy

```python
def update_order_status(order_id, new_status):
    """Dual-write with error handling"""
    
    # Write to legacy (known, stable)
    try:
        legacy_db.execute(
            "UPDATE orders SET status = ? WHERE id = ?",
            new_status, order_id
        )
    except Exception as e:
        logger.error(f"Legacy write failed: {e}")
        return {"error": "write_failed"}, 500
    
    # Write to new service (async, don't block request)
    try:
        # Queue the write for async processing
        queue.enqueue(
            "update_new_service",
            order_id=order_id,
            status=new_status
        )
    except Exception as e:
        logger.warning(f"Failed to queue new service write: {e}")
        # Don't fail the request; async retry will happen
    
    return {"status": "ok"}
```

**Key rule:** Make the legacy write synchronous (mandatory), and new write asynchronous (best-effort). This ensures the legacy system is always updated, and the request doesn't timeout waiting for the new service.

---

## Event Interception

### CDC (Change Data Capture) for Legacy Sync

If dual-write is too invasive, intercept changes from the legacy database using CDC:

```
Legacy Database
  ↓
CDC Tool (Debezium)
  ↓
Kafka Stream
  ↓
New Service (consume changes, update local store)
```

**Benefits:** No code changes in legacy service; new service self-synchronizes.

### Example: Syncing Orders with CDC

```python
import json
from kafka import KafkaConsumer

consumer = KafkaConsumer(
    'legacy.order_changes',
    bootstrap_servers=['kafka:9092'],
    value_deserializer=lambda m: json.loads(m.decode('utf-8'))
)

new_db = connect_to_new_database()

for message in consumer:
    change = message.value
    
    if change['op'] == 'insert':
        new_db.insert('orders', change['after'])
    elif change['op'] == 'update':
        new_db.update('orders', **change['after'])
    elif change['op'] == 'delete':
        new_db.delete('orders', id=change['before']['id'])
```

**Pros:**
- No changes to legacy service
- Asynchronous and decoupled
- Full history available in Kafka

**Cons:**
- Requires operational infrastructure (CDC + Kafka)
- Initial snapshot (bulk copy of existing data) must happen first
- Lag: Orders appear in new service after CDC pipeline processes them (~1-5s typical)

---

## Parallel Running and Verification

### Shadow Traffic

Route reads from the legacy system to both, comparing results:

```python
def get_order(order_id):
    """Read from both; compare; return legacy"""
    
    # Primary read (return to user)
    legacy_order = legacy_db.get_order(order_id)
    
    # Shadow read (compare, don't return)
    try:
        new_order = new_service.get_order(order_id)
        
        # Compare results
        if not orders_equal(legacy_order, new_order):
            logger.warning(f"Data mismatch for order {order_id}: "
                           f"legacy={legacy_order}, new={new_order}")
            metrics.increment("shadow_mismatch")
    except Exception as e:
        logger.error(f"Shadow read failed: {e}")
        metrics.increment("shadow_error")
    
    return legacy_order
```

**Runbook:** Monitor `shadow_mismatch` metric. If > 0%, investigate why data differs.

### Replay Testing

Use production traffic logs to replay requests against the new service and compare responses:

```
1. Capture production requests (headers, body, user context)
2. Replay against both old and new service
3. Compare responses (status code, body)
4. Generate report: % matching, % errors, latency differences
```

**Example:**

```bash
# Capture production traffic for 1 hour
curl http://legacy-api/orders --save-requests > /tmp/production-requests.jsonl

# Replay against new service
python replay_test.py \
  --requests /tmp/production-requests.jsonl \
  --new-service http://new-api \
  --legacy-service http://legacy-api \
  --output report.html
```

**Report output:**

```
Total requests replayed: 10,523
Matching responses: 10,489 (99.7%)
Mismatches: 34 (0.3%)
  - Different status codes: 12
  - Different response bodies: 22
  - New service errors: 5
Average latency: Legacy 150ms, New 120ms (20% faster)
```

---

## Data Synchronization

### 1. Initial Bulk Copy

Before routing any traffic, bulk-copy existing data from legacy to new:

```sql
-- Run once during migration planning
INSERT INTO new_db.orders 
SELECT * FROM legacy_db.orders 
WHERE created_at < '2026-03-25 00:00:00';
```

**Challenges:**
- Large tables may take hours
- Must be done offline (risk of inconsistency if legacy is live)
- Schema differences must be handled

### 2. Dual-Write with Backfill

After bulk copy, start dual-writing for new changes. Periodically scan for gaps:

```python
def backfill_missing_records():
    """Find records in legacy but not in new; sync them"""
    
    legacy_ids = set(
        row['id'] for row in legacy_db.query(
            "SELECT id FROM orders WHERE updated_at > ?"
            [datetime.now() - timedelta(hours=1)]
        )
    )
    
    new_ids = set(
        row['id'] for row in new_db.query(
            "SELECT id FROM orders WHERE updated_at > ?"
            [datetime.now() - timedelta(hours=1)]
        )
    )
    
    missing = legacy_ids - new_ids
    
    for order_id in missing:
        legacy_order = legacy_db.get_order(order_id)
        new_db.upsert_order(legacy_order)
```

**Run as a scheduled job** (every 5-10 minutes) to catch any missed writes.

### 3. Snapshot Verification

Periodically compare total record counts and checksums:

```python
def verify_data_sync():
    """Ensure new system has all data"""
    
    legacy_count = legacy_db.query("SELECT COUNT(*) FROM orders")[0][0]
    new_count = new_db.query("SELECT COUNT(*) FROM orders")[0][0]
    
    if legacy_count != new_count:
        logger.error(f"Count mismatch: legacy={legacy_count}, new={new_count}")
        return False
    
    # Checksum first N rows for spot check
    legacy_checksum = legacy_db.query(
        "SELECT MD5(GROUP_CONCAT(id)) FROM orders LIMIT 1000"
    )[0][0]
    
    new_checksum = new_db.query(
        "SELECT MD5(GROUP_CONCAT(id)) FROM orders LIMIT 1000"
    )[0][0]
    
    if legacy_checksum != new_checksum:
        logger.error(f"Checksum mismatch: {legacy_checksum} vs {new_checksum}")
        return False
    
    return True
```

---

## Canary Migration

### Staged Rollout

Progressively shift traffic from legacy to new:

```
Phase 1: Internal testing (0% users, 100% internal tools canary)
Phase 2: Beta users (5% random users, 95% legacy)
Phase 3: Ramping canary (10% → 25% → 50%)
Phase 4: Full cutover (100% new, legacy as backup only)
Phase 5: Decommission legacy (after monitoring shows no issues)
```

### Metrics to Track

**Success Rate:**

```
success_rate = successful_requests / total_requests

If new service success_rate < 99.9% (vs. legacy 99.95%), 
  → rollback automatically
```

**Latency:**

```
p50_latency = 50th percentile response time
p99_latency = 99th percentile response time

If new service p99_latency > legacy * 1.5 (50% slower), 
  → hold rollout, investigate
```

**Error Breakdown:**

```
errors_by_type = {
  "timeout": 12,
  "not_found": 5,
  "internal_error": 2
}

If "internal_error" rate > 0.1%, 
  → rollback; assess error
```

### Automated Rollback

```python
def evaluate_canary():
    """Check metrics; rollback if threshold breached"""
    
    metrics = get_metrics(
        service="new_order_service",
        time_window=timedelta(minutes=10)
    )
    
    # Success rate check
    if metrics['success_rate'] < 0.999:
        logger.critical("Success rate too low; rolling back")
        change_traffic_weight(legacy=100, new=0)
        alert_oncall("Rollback triggered: low success rate")
        return
    
    # Latency check
    if metrics['p99_latency'] > metrics['legacy_p99'] * 1.5:
        logger.critical("Latency spike; rolling back")
        change_traffic_weight(legacy=100, new=0)
        alert_oncall("Rollback triggered: latency spike")
        return
    
    # All good; progressively increase capacity
    current_weight = get_traffic_weight('new')
    if current_weight < 100 and metrics['error_rate'] < 0.001:
        new_weight = min(100, current_weight + 10)
        change_traffic_weight(legacy=100-new_weight, new=new_weight)
        logger.info(f"Canary progressing: {current_weight}% → {new_weight}%")
```

---

## Measuring Migration Confidence

### Confidence Framework

Build confidence through multiple signals:

| Signal | Low Confidence | Medium Confidence | High Confidence |
|--------|---|---|---|
| **Data sync** | Checksums don't match | 99% of records match | 100% match verified |
| **Success rate** | < 99.5% | 99.5% - 99.95% | > 99.95% |
| **Latency** | 2x legacy | 1.1x - 1.5x legacy | < 1.1x legacy |
| **Error rate** | > 0.5% | 0.01% - 0.5% | < 0.01% |
| **Canary traffic** | < 5% | 5% - 50% | > 50% |
| **Duration at traffic level** | < 1 hour | 1-24 hours | > 24 hours |

**Decision rule:**

- **Proceed to next phase** when ALL signals show Medium or better
- **Hold** if ANY signal shows Low confidence; investigate
- **Rollback** if signal degrades while live traffic is on new service

### Operational Checklist

```
Before shipping Phase N:

[✓] Data verification: count match, checksum match, sample records verified
[✓] Metrics baseline: Record legacy service metrics (success rate, latency)
[✓] Alert rules: Set thresholds (success_rate, error_rate, latency_p99)
[✓] Runbook: Document rollback procedure and escalation contacts
[✓] Shadow traffic: Run 30+ minutes, zero errors in comparison
[✓] Stakeholder approval: Team lead + ops engineer sign off

Shipping Phase N:

[✓] Set traffic split (e.g., 10% new / 90% legacy)
[✓] Monitor metrics continuously for 30 minutes
[✓] If metrics stable, hold for 4-24 hours (phase-dependent)
[✓] If metrics degrade, trigger rollback immediately
[✓] Document decision (proceed/hold/rollback) with notes
```

### Example: Phase 2 (5% Canary)

```
Start time: 2026-03-25 14:00 UTC
Traffic split: 5% new, 95% legacy
Target duration: 24 hours
Success criteria:
  - Success rate >= 99.95%
  - Error rate <= 0.05%
  - p99 latency <= 200ms (legacy is 150ms)
  - No data mismatches
  - No automated alerts triggered

Monitoring:
  - Check metrics every 5 minutes for first 30 minutes
  - Hourly checks after hour 1
  - Daily check after 24 hours if stable

Decision:
  - PROCEED if all criteria met → Phase 3 (10% canary)
  - HOLD if criteria not met → Investigate, retry after fix
  - ROLLBACK if regression → Revert to 100% legacy
```

---

## See Also

- [architecture-strangler-fig.md](architecture-strangler-fig.md) — General strangler pattern concepts
- [devops-database-migrations.md](devops-database-migrations.md) — Schema coordination during strangler migration
- [devops-feature-flags.md](devops-feature-flags.md) — Feature flag infrastructure for routing