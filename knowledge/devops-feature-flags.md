# Feature Flags — Evaluation Strategies, Lifecycle & DevOps Patterns

## Overview

Feature flags decouple **code deployment** from **feature release**. Code is deployed to production in a disabled state (dark launch), then gradually rolled out to users by flipping flags server-side. This enables continuous deployment without releasing half-baked features, running A/B experiments, and killing problematic features without redeploy.

## Flag Types

### Release Flags

Control rollout of completed features to production users. Lifecycle: off (development) → on (fully rolled out) → removed (cleanup).

```python
if get_flag("checkout_v2_enabled"):
    return CheckoutV2()
else:
    return CheckoutV1()
```

**Purpose:** Canary rollout (5% → 25% → 100%), staged feature release.

### Experiment Flags

Run A/B tests or multivariate experiments. Flags assign users to cohorts; metrics compared across variants.

```typescript
const variant = get_flag("search_algorithm", { treatment: "control" });
if (variant === "treatment") {
  results = newAlgorithm.search(query);
  track("search_experiment", { variant: "treatment" });
} else {
  results = oldAlgorithm.search(query);
  track("search_experiment", { variant: "control" });
}
```

**Purpose:** Measure business impact before full rollout. Analyze: conversion, latency, error rate by variant.

### Ops Flags (Runtime Toggles)

Instant on/off switches for operational decisions: circuit breakers, rate limit adjustments, debug logging.

```go
if get_flag("debug_slow_queries") {
    query.LogLevel = DEBUG
}

if !get_flag("billing_service_available") {
    return Errors.SERVICE_UNAVAILABLE
}
```

**Purpose:** Respond to incidents / high load without redeploy. Enable/disable features in response to metrics.

### Permission/Beta Flags

Control feature access by user cohort: beta testers, enterprise customers, public.

```sql
SELECT * FROM features
WHERE user_id = $1 AND (
  public = true OR
  user_id IN (SELECT user_id FROM beta_signup) OR
  organization_id IN (SELECT org_id FROM enterprise_customers)
)
```

**Purpose:** Feature access control; staged rollout to specific user segments.

## Flag Evaluation Models

### Server-Side Evaluation

Flag state evaluated on each request by backend server.

```javascript
// Backend evaluates flag, returns rendered response
app.get("/checkout", (req, res) => {
  const useNewCheckout = featureFlags.isEnabled("checkout_v2", {
    userId: req.user.id,
    org: req.user.org
  });
  res.render("checkout", { useNew: useNewCheckout });
});
```

**Trade-offs:**
- ✓ Full control over targeting; can use server-side context (user roles, org attributes)
- ✓ Easy to update flags without client redeploy
- ✗ Adds latency per request (flag evaluation + network to flag service)
- ✗ Cannot disable features for specific clients if service goes down

### Client-Side Evaluation

Flag state fetched at app initialization; evaluated locally in browser/mobile.

```javascript
// Client fetches all flags at startup
const flags = await fetch("https://flags-api.com/flags", {
  headers: { "Authorization": `Bearer ${apiKey}` }
});

// Later, evaluate locally
if (flags["checkout_v2_enabled"]) {
  showCheckoutV2();
}
```

**Trade-offs:**
- ✓ Fast evaluation (no network round-trip per flag check)
- ✓ Works offline; can default gracefully if flag service unavailable
- ✗ Flag state may be stale (fetched at startup; real-time changes take minutes)
- ✗ All flag logic exposed in client; security-sensitive targeting not possible

### Edge Evaluation

Flag evaluated at edge (CDN / proxy layer) before reaching origin server.

```
User Request → Cloudflare Worker → Evaluate Flag → Origin (conditional routing)
```

**Example:** Cloudflare Workers routing based on feature flag:

```javascript
// Cloudflare Worker
export default async (request) => {
  const flagState = await featureFlagService.get("new_api_version");
  if (flagState.enabled) {
    return fetch("https://api-v2.example.com" + request.url);
  } else {
    return fetch("https://api-v1.example.com" + request.url);
  }
};
```

**Trade-offs:**
- ✓ Lowest latency (decisions at CDN edge, no origin round-trip)
- ✓ Can implement request-level routing (A/B tests at edge)
- ✗ Limited targeting complexity (can't access full user context)
- ✗ Still requires flag service integration at edge

### Hybrid Approaches

Combine models based on use case:
- **Static/infrequent flags** (release flags for new features) → client-side; bootstrap on app start
- **User-targeted flags** (permission-based) → server-side; use user ID from request
- **Traffic-based flags** (canary rollout) → edge or server-side; evaluate on request path
- **Operational toggles** (kill switches) → server-side; lowest latency requirement critical

## Flag Lifecycle & Technical Debt

### Lifecycle Stages

1. **Development** — Flag created; code merged with flag disabled; team tests locally
2. **Staging** → flag enabled for staging environment; QA validates feature
3. **Canary** — flag enabled for small % of prod traffic (5%, 25%, 50%, 100%)
4. **Stable** — flag enabled for all traffic; flag logic no longer checked in conditional branches
5. **Cleanup** → feature fully stable; remove flag from code + flag service

### Dead Code Accumulation

Flags that are never cleaned up accumulate as technical debt:

```python
# This flag has been 100% enabled for 6 months; still in code
if get_flag("new_search_algorithm"):
    results = newAlgorithm(query)
    # This can be removed; new algorithm is now default
else:
    results = oldAlgorithm(query)  # Dead code
```

**Problems:**
- Code paths never executed in production (untested)
- Testing complexity: M flags = 2^M possible code paths
- Cognitive load: developers must understand conditional logic for abandoned features
- Risk: if flag accidentally disabled, users see old/buggy behavior

### Flag Cleanup Strategy

1. **Audit:** Find all flags in code; track their age, last change date
2. **Evaluate:** Is flag still needed? Is feature fully stable?
3. **Remove:** Delete flag from code; remove from flag service
4. **Archive:** Document removed flags for rollback if needed (rarely)

**Heuristic:** If flag has been at 100% for >1 quarter, strongly consider cleanup.

## Targeting Rules & Percentage Rollout

### Basic Targeting

Flags enable based on user attributes, environment, timing:

```json
{
  "flag": "checkout_v2",
  "rules": [
    {
      "match": { "userId": "user123" },
      "enabled": true
    },
    {
      "match": { "org": "enterprise-customer" },
      "enabled": true
    },
    {
      "match": { "country": "CA" },
      "enabled": false
    }
  ],
  "default": false
}
```

### Percentage Rollout

Deterministically enable flag for N% of user base using consistent hashing:

```
hash(flag_key + user_id) % 100 < percentage
```

**Behavior:** Same user always gets same treatment (consistent); no flicker on refresh; distributes evenly across users.

```javascript
function isEnabledForUser(flagKey, userId, percentage) {
  const hash = MurmurHash3(flagKey + userId);
  return (hash % 100) < percentage;
}

// If percentage=25, flag enabled for ~25% of users
// Same user always gets same value (no flickering)
```

### Scheduling Rollout

Ramp up percentage over time:

```json
{
  "flag": "checkout_v2",
  "rollout": [
    { "startTime": "2025-03-26T00:00Z", "percentage": 5 },
    { "startTime": "2025-03-27T00:00Z", "percentage": 25 },
    { "startTime": "2025-03-28T00:00Z", "percentage": 50 },
    { "startTime": "2025-03-29T00:00Z", "percentage": 100 }
  ]
}
```

Enables gradual rollout without manual intervention. Pause/revert by adjusting percentage.

## Kill Switches

**Kill switch:** Emergency flag to instantly disable a feature if it's causing harm (high error rate, data corruption, security breach).

### Characteristics

- **Latency < 100ms:** Kill switch decision must be fast; seconds-long bypass unacceptable if system is degrading
- **Always-on by default:** Feature disabled unless explicitly enabled (fail-safe)
- **Independent of feature flag:** Separate mechanism; if flag service fails, kill switch still works
- **Tested regularly:** Untested kill switches fail when needed. Practice drills: flip switch, verify feature disabled

### Implementation

Server-side kill switch:

```javascript
async function handleCheckout(req, res) {
  // Kill switch checked first (lowest latency)
  if (!await killSwitch.isDisabled("checkout")) {
    return res.status(503).send("Checkout temporarily disabled");
  }

  // Normal flow
  const useNewCheckout = await featureFlags.isEnabled("checkout_v2", req.user);
  // ...
}
```

Local kill switch (requires no network call):

```javascript
// Read from local file / env variable
const KILL_SWITCH = process.env.KILL_SWITCH_CHECKOUT === "true";

if (KILL_SWITCH) {
  throw new CheckoutDisabledError();
}
```

## Feature Flag Platforms

### LaunchDarkly (Market Leader)

Enterprise-grade flag service. Strengths:
- Powerful targeting rules (user attributes, organizations, custom contexts)
- Real-time flag updates (webhooks notify clients of changes)
- Experimentation framework (statistics, significance testing)
- SDKs for all languages; server + client-side support
- Audit trail / compliance reporting

Limitations: SaaS only, vendor lock-in, pricing per monthly active user.

### Flagsmith (Open-Source Friendly)

Open-source + managed SaaS. Can self-host. Strengths:
- Lightweight, self-hostable option
- Simple rule engine; good for smaller orgs
- Cost-effective (open-source option free)

Limitations: Fewer advanced targeting features than LaunchDarkly; smaller community.

### OpenFeature (Standardized Interface)

vendor-agnostic standard for feature flag SDKs. Enables switching between providers without code changes.

```typescript
// OpenFeature API (vendor-agnostic)
const client = OpenFeature.getClient();
const isEnabled = client.getBooleanValue("checkout_v2", false);
```

Grabs flags from configured **provider**:

```typescript
OpenFeature.setProvider(new LaunchDarklyProvider(token));
// Or switch to different provider:
OpenFeature.setProvider(new FlagsmithProvider(token));
```

**Purpose:** Avoid vendor lock-in; standardize flag API across teams. Providers: LaunchDarkly, Flagsmith, CloudBees, Harness.

### In-House Solutions

Small teams often build lightweight flag systems:

```python
# Simple in-process evaluation
FEATURE_FLAGS = {
    "checkout_v2": lambda user: user.id in [1, 2, 3],  # Hardcoded users
    "search_beta": lambda user: hash(user.id) % 100 < 25,  # 25% of users
}

def is_enabled(flag_name, user):
    return FEATURE_FLAGS[flag_name](user)
```

**Pros:** No external dependency; full control; minimal latency.  
**Cons:** No UI for non-engineers; no real-time updates; targeting rules hard to change without code.

## Best Practices

1. **Name flags consistently.** Use prefixes: `release_`, `experiment_`, `ops_`, `permission_`.
2. **Set expiration dates.** Flags should have owner + cleanup date. Alert if cleanup missed.
3. **Test flag variations.** Unit tests for both enabled/disabled code paths; don't skip disabled paths.
4. **Avoid flag combinatorics.** If >5 independent flags, combinatorial testing becomes infeasible. Use feature branches for complex features.
5. **Monitor flag performance.** Track: time spent in flag evaluation, flag staleness (client vs server), rollout skew.
6. **Use percentage rollout for canary.** Distribution via hashing ensures even spread; reduces bias from user cohort selection.
7. **Kill switches need testing.** Include kill switch testing in incident response drills. Practice flipping switches under load.
8. **Separate deployment from release.** Deploy code with flag off; enable flag through separate release process. Enables instant rollback.

## See Also

- [Progressive Delivery — Canary, Blue-Green, Feature Flags](progressive-delivery.md)
- [DevOps — CI/CD Patterns](devops-cicd-patterns.md)
- [Architecture — Resilience Patterns](architecture-resilience.md)