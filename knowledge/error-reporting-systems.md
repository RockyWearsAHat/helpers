# Error Reporting & Monitoring Systems — Architecture, Integration & Aggregation

## Overview

Error reporting platforms (Sentry, Datadog, Bugsnag, Rollbar) collect, deduplicate, group, and alert on production errors. They provide source map deobfuscation, release tracking, alert fatigue mitigation, and integration with SLOs. Understanding their architecture helps teams configure them effectively and know their limits.

---

## Core Architecture

### Event Capture and Ingestion

Error reporting systems collect **events** via SDK instrumentation:

1. **SDK initialization:** Configure in app at startup (or module load)
2. **Capture:** SDK catches errors (global handlers, try/catch wrapping, framework hooks)
3. **Enrichment:** Add context (user, session, release, breadcrumbs, environment)
4. **Transport:** Send event payload to backend (batched, with retry logic)
5. **Deduplication:** Backend recognizes and deduplicates similar errors
6. **Aggregation:** Group into issues/projects for alerting

### Event Payload Structure

Typical error event contains:

```json
{
  "event_id": "uuid",
  "timestamp": "2026-03-26T10:00:00Z",
  "level": "error",
  "message": "TypeError: Cannot read property 'id' of undefined",
  "exception": {
    "values": [{
      "type": "TypeError",
      "value": "Cannot read property 'id' of undefined",
      "stacktrace": {
        "frames": [
          {
            "function": "fetchUser",
            "filename": "bundle.js",
            "lineno": 12345,
            "colno": 42,
            "in_app": true
          }
        ]
      }
    }]
  },
  "context": {
    "user": {
      "id": "user:123",
      "email": "user@example.com"
    },
    "environment": "production",
    "release": "app@1.2.3",
    "tags": {
      "browser": "Chrome 119",
      "feature_flag": "new_checkout"
    }
  },
  "breadcrumbs": [
    {
      "timestamp": "2026-03-26T09:59:50Z",
      "category": "user-action",
      "message": "Clicked checkout button",
      "level": "info"
    },
    {
      "timestamp": "2026-03-26T09:59:55Z",
      "category": "http",
      "method": "GET",
      "url": "/api/cart",
      "status_code": 200
    }
  ]
}
```

---

## Source Maps and Stack Trace Deobfuscation

### The Problem

Production JavaScript is minified:

```javascript
// original
function fetchUser(id) {
  const user = api.get(`/users/${id}`);
  return user.profile;
}

// minified
function a(e){return b.get(`/users/${e}`).profile}
```

When an error occurs, the stack trace shows minified names and line numbers that don't correspond to source code, making debugging impossible.

### Source Map Upload

Source maps (`.map` files) map minified code back to original source:

```json
{
  "version": 3,
  "sources": ["app.ts"],
  "names": ["fetchUser", "api", "get", "profile"],
  "mappings": "AAAA,SAAS,UAAU,CAAC,EAAE,CERQ,OAAQ,OACvC,GAAA",
  "sourcesContent": [...],
  "file": "bundle.js"
}
```

**Upload process (Sentry example):**

1. **Build integration:** Generate source maps during build

```javascript
// webpack.config.js
plugins: [
  new webpack.SourceMapDevToolPlugin({
    filename: '[file].map'
  })
]
```

2. **Upload via CLI or CI:**

```bash
# At release time
sentry-cli releases create myapp@1.2.3
sentry-cli releases files myapp@1.2.3 upload-sourcemap \
  --dist myapp@1.2.3 \
  dist/js/
```

3. **Release tracking:**

```javascript
Sentry.init({
  dsn: "...",
  release: "myapp@1.2.3",
  dist: "myapp@1.2.3"
});
```

The SDK includes `release` and `dist` in events. Backend matches events to uploaded source maps by release/dist.

4. **Deobfuscation:** Backend uses source maps to rewrite stack frames from minified to original locations.

### Best Practices

- Upload source maps immediately after release
- Keep source maps private (they expose source code)
- Set URL rewrites if source map naming doesn't match deployment URLs
- Verify uploads: `sentry-cli releases files [release] list`

---

## Error Grouping and Deduplication

### Fingerprinting

Systems group similar errors into **issues** via fingerprinting. Default fingerprinting uses:

- Exception type
- Exception message
- Stack trace top frames (in_app=true)

**Example:**

```
Issue: TypeError in fetchUser at app.ts:42
  Event 1: TypeError: user is undefined (Mar 26, 10:01)
  Event 2: TypeError: user is undefined (Mar 26, 10:05)
  Event 3: TypeError: user is undefined (Mar 26, 10:12)
  → Grouped as same issue
```

### Custom Fingerprinting

Sometimes the default groups too tightly or too loosely:

```javascript
Sentry.init({
  beforeSend: (event) => {
    // Group all cart checkout errors together
    if (event.tags?.section === 'checkout' && event.level === 'error') {
      event.fingerprint = ['checkout_error'];
    }
    return event;
  }
});
```

### Deduplication

Prevents alerting on every duplicate event:

- Within a time window (e.g., 1 hour), count all events for an issue
- Alert once per issue + status change (new, regressed, resolved)
- Some platforms deduplicate across releases ("is this a new issue in 1.2.3 or existing?")

---

## Alert Fatigue and Noise Reduction

### The Problem

High-frequency notifications cause alert fatigue—on-call engineers stop responding to alerts. Common sources:

- Same error firing 1000 times (one customer hitting it repeatedly)
- Transient errors (network spike, briefly degraded service)
- Low-severity warnings grouped with critical errors

### Mitigation Strategies

**1. Alert on issue creation, not recurrence**

```javascript
// Alert only on NEW issues, not every 100th occurrence
if (event.issue.isNew) {
  sendAlert({
    title: `New error: ${event.issue.title}`,
    severity: 'critical'
  });
}
```

**2. Alert on regression (reopen of resolved issue)**

```javascript
if (event.issue.status === 'regressed') {
  sendAlert('Issue regressed in production');
}
```

**3. Suppress known transient errors**

```javascript
Sentry.init({
  beforeSend: (event) => {
    if (event.exception?.values?.[0]?.value?.includes('ECONNREFUSED')) {
      event.level = 'warning';  // downgrade from error
    }
    return event;
  }
});
```

**4. Throttle by user/session**

Report only first error per user per time window (prevents single user spamming):

```javascript
Sentry.init({
  beforeSend: (event) => {
    const userId = event.user?.id;
    const now = Date.now();
    
    // Skip if same user reported error in last 5 minutes
    const lastError = window._lastErrorByUser?.[userId];
    if (lastError && now - lastError < 5 * 60 * 1000) {
      return null;  // don't send
    }
    
    window._lastErrorByUser = window._lastErrorByUser || {};
    window._lastErrorByUser[userId] = now;
    
    return event;
  }
});
```

**5. Error budgets**

Alert only when error rate exceeds SLO budget (tied to business SLOs):

```javascript
// Alert if error rate > 0.1% and remaining budget < 10%
if (errorRate > 0.001 && remainingErrorBudget < 0.1) {
  sendAlert('Error budget exhaustion alert');
}
```

---

## Release Tracking

Error reporting systems correlate errors to releases to answer: "Did this error start with version 1.2.0?"

### Integration

```javascript
Sentry.init({
  dsn: "...",
  release: "myapp@1.2.3",
  dist: "1"  // optional: web/mobile/backend differentiation
});
```

**Deploy hook:** Notify error service when new release is deployed

```bash
# Webhook when releasing to production
curl -X POST https://sentry.io/api/0/organizations/myorg/releases/ \
  -H 'Authorization: Bearer [token]' \
  -H 'Content-Type: application/json' \
  -d '{
    "version": "myapp@1.2.3",
    "projects": ["myapp"],
    "dateReleased": "2026-03-26T10:00:00Z"
  }'
```

### Release Dashboard

Platforms show:
- First/last occurrence of error in each release
- Error trend: did error rate increase after release?
- Diff: "New in 1.2.3", "Existing before 1.2.0"

This helps pinpoint which commit introduced the bug.

---

## Platform Comparison: Sentry, Datadog, Bugsnag, Rollbar

### Sentry

**Focus:** Specialized error tracking (not full APM).

**Strengths:**
- Excellent source map integration
- Precise fingerprinting
- Strong open-source roots (on-prem option)
- Good developer experience

**Weaknesses:**
- Add-on cost for non-error signals (performance, uptime)
- Limited context from outside SDK

**Integration:**
```javascript
import * as Sentry from "@sentry/react";

Sentry.init({ dsn: "..." });
Sentry.captureException(error);
```

### Datadog

**Focus:** Full observability (logs, metrics, traces, errors).

**Strengths:**
- Unified metrics + error correlation
- Deep infrastructure visibility
- Error budgets/SLO integration
- Real user monitoring

**Weaknesses:**
- Steeper pricing
- More complex setup
- Risk of noise without careful configuration

**Integration:**
```javascript
import { datadogRum } from '@datadog/browser-rum';

datadogRum.init({
  applicationId: "...",
  clientToken: "..."
});

window.addEventListener('error', (e) => {
  datadogRum.addError(e.error);
});
```

### Bugsnag

**Focus:** Error and crash monitoring with session replays.

**Strengths:**
- Session replays (see exactly what user was doing)
- Strong breadcrumb context
- Good alert configurability
- Per-user error tracking

**Weaknesses:**
- Session replay can be expensive at scale
- Less mature integrations

### Rollbar

**Focus:** Error tracking with release information.

**Strengths:**
- Excellent release/git integration
- Strong blame/code ownership (who owns this error?)
- Good on-prem options

**Weaknesses:**
- Smaller platform than Sentry/Datadog
- Less polished UI

---

## Integration Patterns

### Try/Catch with Context

```javascript
try {
  const result = await processPayment(order);
  Sentry.captureMessage('Payment processed', 'info', {
    tags: { order_id: order.id }
  });
} catch (error) {
  Sentry.captureException(error, {
    tags: { stage: 'checkout', order_id: order.id },
    level: 'error'
  });
  throw error;  // re-throw to caller
}
```

### Framework Hooks

**React Error Boundary:**

```jsx
import * as Sentry from '@sentry/react';

const SentryErrorBoundary = Sentry.withErrorBoundary(App, {
  fallback: <ErrorPage />
});
```

**Vue global handler:**

```javascript
app.config.errorHandler = (err, instance, info) => {
  Sentry.captureException(err, {
    contexts: {
      vue: {
        component: instance?.$options?.name,
        info
      }
    }
  });
};
```

### Feature Flag Context

```javascript
Sentry.setTag('feature_flag.new_checkout', isNewCheckoutEnabled);
```

Helps correlate errors to feature rollouts.

---

## SLO and Error Budget Integration

Error reporting platforms increasingly tie into SLO systems.

### Workflow

1. Define SLO: "99.9% of requests succeed" = 43 minutes error budget per 30 days
2. Track SLI: Count error rate continuously
3. Alert: "Remaining budget < 25% in last hour"
4. Dashboard: Show budget burn-down in error service

**Example (Sentry + custom integration):**

```javascript
// Calculate remaining error budget
const errorRate = errorCount / requestCount;
const budgetPercentUsed = (errorRate / sloErrorBudget) * 100;

if (budgetPercentUsed > 75) {
  Sentry.captureMessage('Error budget critical', 'warning', {
    contexts: {
      slo: {
        errorRate,
        budgetUsed: budgetPercentUsed
      }
    }
  });
}
```

---

## Best Practices

1. **Initialize early:** Before user code runs
2. **Attach context:** User ID, session, environment, feature flags
3. **Use beforeSend:** Filter noise, transform sensitive data
4. **Upload source maps:** For production JavaScript
5. **Set reasonable sampling:** Don't report every single error (or alert fatigue)
6. **Group intelligently:** Fingerprint by business logic, not just stack traces
7. **Track releases:** Correlate errors to code versions
8. **Break the glass:** Always log to console for on-call troubleshooting
9. **Monitor the monitor:** Alert on Sentry itself—if error reports drop, something's wrong

---

## See Also

- [Frontend Error Boundaries](error-boundaries-frontend.md)
- [Error Handling Language Patterns](error-handling-language-patterns.md)
- [SRE SLO Engineering](sre-slo-engineering.md)
- [Error Handling Distributed Systems](error-handling-distributed.md)
- [DevOps Observability Patterns](devops-observability-patterns.md)