# Strangler Fig Pattern — Incremental Legacy Replacement

## Overview

The **strangler fig** pattern incrementally replaces a legacy system by routing traffic to new functionality while the old system continues to handle the remainder. That by analogy to the strangler fig tree, which grows around and gradually kills its host.

Instead of a "big bang" rewrite (high risk, long timeline, expensive), strangler decomposition:
1. Identifies a vertical slice of functionality (e.g., "user authentication")
2. Replicates it in a new service/codebase
3. Routes new requests to the new service
4. Gradually shifts traffic away from the legacy system
5. Decommissions legacy modules when fully replaced

**When it fits:** Modernizing a core monolith, extracting domains into microservices, or integrating incompatible systems with minimal downtime.

---

## Migration Mechanics

### Facade/Proxy Routing

A **routing layer** (API gateway, proxy, load balancer) sits in front of both systems and decides where to send requests.

```
Client → [Routing Proxy]
           ├── New UserService (10% traffic)
           └── Legacy UserModule (90% traffic)
```

The proxy uses **feature toggles** to control split:
```
if (user.id in CANARY[0, 1000]) {
  route to NewUserService
} else {
  route to LegacyUserModule
}
```

### Parallel Running

Both systems run simultaneously. Requests succeed or fail independently. The migration is **not atomic**—clients may see the old and new systems' states diverge then converge.

**Trade-off:** Increases operational complexity (two versions of the same logic) but enables true gradual transition.

---

## Incremental Replacement Phases

### Phase 1: Read-Only Facade

Extract the simplest operations first: **read-only pathways**. New service serves reads; writes still go to legacy.

```
UserService (new): GET /users/:id → New database read
Legacy:           POST /users     → Old monolith writes
```

**Why start here?** Reads are side-effect-free (mostly). If the new service is slightly stale, consequences are minor. Writes are riskier.

### Phase 2: Shadow Traffic

Run the new service on production write traffic, but **don't use the result**. Log the outcome.

```
1. Client sends POST /users
2. Proxy routes to Legacy (real)
3. Proxy also sends same POST to NewUserService (shadow)
4. NewUserService processes, logs outcome, result is discarded
5. Client sees Legacy result
```

**Purpose:** Detect bugs, performance issues, and data mismatches without risk. "Does the new service handle this request correctly?"

### Phase 3: Dark Launch

Release new functionality without exposing it to users yet. Internal testing, staging, canary groups.

```
FeatureToggle: new-dashboard-v2
├── Off for all users
├── On for internal staff
├── On for 0.1% of users (canary)
├── On for 5% of users
└── On for all users (full production)
```

### Phase 4: Gradual Traffic Shift

Shift real traffic incrementally. Two strategies:

**Time-based:**
```
Monday:  5% new, 95% legacy
Tuesday: 10% new, 90% legacy
Friday:  100% new
```

**Cohort-based:**
```
All users in region=US: 100% new
All users in region=EU: 50% new, 50% legacy
```

---

## Data Migration Strategies

### Dual-Write Pattern

During transition, writes go to **both** old and new databases.

```
UserService.create(userData):
  1. Write to legacy database
  2. Write to new database
  3. Return success if both succeed
```

**Pros:** New database gets populated; old system remains source of truth during transition.

**Cons:** Extra latency, disk space, and complexity. Requires compensating writes if they diverge (reconciliation).

**Failure case:** New write succeeds, legacy write fails. Old and new diverge. Need detection + correction (event sourcing, CDC, or periodic sync).

### Change Data Capture (CDC)

Stream old system's changes → new system. Achieved via:
- Database logs (PostgreSQL WAL, MySQL binlog)
- Event publishing (legacy publishes updates to Kafka)
- Polling (periodically query legacy for changes, push to new)

```
Legacy DB → [CDC] → [Transform] → New DB
```

**Where it breaks:** If legacy is an opaque commercial system without CDC, you're limited to polling or dual-write.

### Event Interception

Intercept domain events from legacy; new system subscribes.

```
Legacy publishes: "UserCreated(userId=123)"
    ↓
New service subscribes, creates user in new database
```

**Works well when:** Legacy already publishes events (or you can instrument it to).

**Doesn't work when:** Legacy is procedural, event-less code.

---

## Anti-Corruption Layer

A **translation layer** between legacy and new, preventing legacy concepts from polluting new code.

```
Legacy User Model:
{
  "user_id": 123,
  "usr_nm": "Alice",
  "creat_ts": 1234567890,
  "is_activ": 1
}

Anti-Corruption Layer:
User toLegacy(UserDTO dto) {
  return new User(
    id: dto.userId,
    name: dto.name,
    createdAt: DateTime.fromUnixTime(dto.createdTs),
    active: dto.isActive == 1
  )
}

New Service (clean interface):
public class UserService {
  private User legacyUser;
  public String getName() { return legacyUser.name; }
}
```

**Benefit:** New code is clean and domain-aligned; legacy's warts are isolated.

---

## Feature Toggle Migration

Use toggles to control which customers see the new version.

```json
{
  "features": {
    "new-checkout-flow": {
      "enabled_for_users": ["user-123", "user-456"],
      "enabled_for_regions": ["US", "CA"],
      "enabled_for_percentage": 5,
      "rollout_schedule": [
        {"date": "2025-04-01", "percentage": 10},
        {"date": "2025-04-08", "percentage": 25},
        {"date": "2025-04-15", "percentage": 100}
      ]
    }
  }
}
```

Toggles enable:
- A/B testing (compare outcomes between new and old)
- Instant rollback (flip toggle, traffic shifts back)
- Safe experimentation (canary → small cohort → full)

---

## Measuring Progress

### Metrics

| Metric                   | Interpretation                                      |
| ------------------------ | --------------------------------------------------- |
| % Traffic on New Service | Global progress; not always linear due to hotspots |
| Error Rate (new vs old)  | Is new service more stable? (Often: no, then yes)   |
| Latency (new vs old)     | New initially slower; should improve as optimized   |
| Feature Parity          | Number of old operations not yet in new service    |
| Database Size Ratio     | New DB should catch up to old as data syncs         |

### Monitoring

Track **divergence** between old and new:
```
Consistency check every hour:
  Query old DB for user count, last updated: 50,000  users, 2025-03-25 14:30
  Query new DB for user count, last updated: 49,950 users, 2025-03-25 14:25
  Divergence: 50 users behind, 5 minutes lag
```

If divergence grows, pause migration; diagnose (CDC lag, dual-write failures, new system bugs).

---

## Common Pitfalls

### The "Almost Done" Trap

Months of "we're 95% done" → 10% of effort left is 90% of the time (corner cases, edge states, rare features). Keep legacy and new both operational longer than expected.

**Antidote:** Set decommissioning deadline; accept that some edge cases in legacy won't be handled by new (with mitigation plan).

### Inconsistent Dual-Write

Dual-write without reconciliation → persistent divergence. Old system becomes the "real" data; new system is a cache. Eventually they disagree on business rules.

**Antidote:** Implement CDC or periodic reconciliation audit.

### Premature Decommissioning

Delete legacy code before being 100% certain no customer uses old paths. Customers hit 404, then escalate.

**Antidote:** Keep legacy codepath for 1-2 quarters after last traffic is observed. Monitor for reappearance.

---

## Strangler vs. Other Strategies

| Strategy        | Timeline | Risk     | Cost | Team Effort |
| --------------- | -------- | -------- | ---- | ----------- |
| Big-Bang Rewrite| Months   | Very High| High | High        |
| Strangler Fig   | Quarters | Low-Med  | Med  | High        |
| Pure New Build  | Months   | Very High| High | High        |
| Maintain Legacy | Years    | Low      | Low  | Low         |

**Strangler is longest but lowest-risk.** Use it when legacy is critical and downtime is expensive.

---

## See Also

- Modular Monolith (strangler within a single codebase)
- Anti-Corruption Layer (DDD pattern, essential for strangler)
- Feature Toggle Patterns
- Microservices Architecture
- Database Migrations