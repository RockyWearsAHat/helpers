# Observability: Alerting

Alerting bridges the gap between metrics (what the system is doing) and human response (what to do about it). Poor alerting causes alert fatigue (too many false positives) or alert blindness (missed real incidents).

## Symptom-Based vs Cause-Based Alerting

**Cause-based alerting** monitors infrastructure: CPU > 80%, disk full, pod crashes.

```
Alert "HighCPU": max(node_cpu_utilization) > 0.8
Alert "DiskFull": (disk_used / disk_total) > 0.9
Alert "PodCrashLoop": rate(container_restarts[10m]) > 0.1
```

**Problem:** These don't tell you if customers are actually experiencing problems. A pod crash is only bad if it's serving traffic.

**Symptom-based alerting** monitors user experience: error rate, latency, availability.

```
Alert "HighErrorRate": rate(http_requests_total{status=~"5.."}[5m]) > 0.01  // >1% errors
Alert "SlowAPI": histogram_quantile(0.95, http_request_duration[5m]) > 1s
Alert "ServiceDown": up{job="checkout"} == 0
```

**Advantage:** Direct correlation with user impact. If error rate is normal, nobody cares about CPU.

**Recommendation:** **Primary** alerts on symptoms. **Secondary** alerts on causes (for infrastructure teams).

```
Alert severity=P1: Error rate spike (symptom, pages on-call)
Alert severity=P2: High CPU on node (cause, creates ticket only if symptoms worsen)
```

## Alert Fatigue: False Positives and Noise Reduction

**Alert fatigue:** On-call engineers stop responding to alerts when most are false positives or non-critical.

### Root Causes

1. **Threshold too sensitive:** Alert fires when system is operating normally.

```
Alert "RequestLatencyHigh": p95(latency) > 200ms
# Problem: p95 naturally varies; 200ms is sometimes reached during peak load
# Result: Fires 50 times per day; engineer ignores
```

2. **Transient failures:** Single failure spike triggers alert.

```
Alert "PaymentAPIDown": (requests with status=500) > 0
# Problem: API returns 1-2 errors/min during normal operation; false positives
```

3. **Cascading alerts:** One root cause fires 10 related alerts.

```
Service A fails
  ├─ Alert: Service A down (symptom)
  ├─ Alert: Service A error rate high (symptom)
  ├─ Alert: Service A latency high (symptom)
  ├─ Alert: Database CPU high (possibly related cause)
  ├─ Alert: Memory leak in Service A (possibly related cause)
  └─ ... 5 more related alerts
  
# Engineer wakes up with 10 Slack messages; doesn't know which to fix first
```

### Mitigation Strategies

**1. Use thresholds that reflect real SLOs.**

```
SLO: Error rate < 0.1% (99.9%)
Alert "ErrorRateBreach": rate(errors[5m]) > 0.001  // 0.1%

# Don't alert at 1% error; alert *near* the boundary
Alert "ErrorRateWarning": rate(errors[5m]) > 0.0008  // 80% of budget

# Account for variance; use sustained breaches
Alert "ErrorRateP1": rate(errors[5m]) > 0.001 for 2m  // sustained for 2 minutes
```

**2. Duration threshold:** Require sustained breach, not single spike.

```
Bad:  Alert fires if error_rate > 1% (single sample)
Good: Alert fires if error_rate > 1% for 2 minutes (multiple samples)
```

**3. Suppress related alerts; promote root cause only.**

```yaml
# Alertmanager grouping
group_by: ['alertname', 'service']
routes:
  - match: {severity: 'critical'}
    group_wait: 0s
    group_interval: 5m
    repeat_interval: 1h
    
  - match: {severity: 'warning'}
    group_wait: 30s  # Wait 30s to batch related alerts
    group_interval: 5m
```

**Effect:** If Service A has 10 related alerts, Alertmanager groups them and fires once. Engineer sees one notification, not ten.

**4. Filter noisy signals at collection time.**

```yaml
# Drop false-positive metrics before they reach Prometheus
metric_relabeling:
  - source_labels: [instance]
    regex: "test-.*"
    action: drop  # Don't ingest test instances
  
  - source_labels: [pod_label_app]
    regex: "(.*.)"
    target_label: app
    # Only keep pods with app label
```

## Multi-Window, Multi-Burn-Rate SLO Alerting

**SLO** (Service Level Objective): Target reliability. E.g., 99.9% of requests complete successfully within 500ms.

**Error budget:** How much unreliability can the service tolerate? For 99.9% SLO over 30 days: 0.1% × 30 days = ~4.3 minutes of downtime allowed.

**Problem with naive alerting:**

```
Alert "SLOBreach": error_rate > 0.001  // 0.1%
# This fires AFTER error budget is exhausted
# Too late; customers already saw degradation
```

**Solution: Multi-window alerting.**

Alert when error budget is being _consumed too fast_, not when it's already empty.

```
# 30-day SLO: 99.9% (0.1% error budget)

# Fast burn: consume entire budget in 1 hour
# If error_rate > 3% for 1 hour, alert immediately
# (3% for 1h = entire 0.1% daily budget gone)
Alert "FastBurn": rate(errors[1h]) > 0.03 for 1m

# Slow burn: consume entire budget in 1 day
# If error_rate > 0.12% for 6 hours, budget exhausted by end of day
Alert "SlowBurn": rate(errors[6h]) > 0.0012 for 15m
```

**Interpretation:**

```
Fast burn triggers → Page immediately (incident in progress)
Slow burn triggers → Create ticket (degradation, needs attention but not emergency)
Both trigger → Severe incident (error rate stayed high for hours)
```

### Multi-Burn-Rate Rule Example

From Google SRE book:

```yaml
groups:
  - name: slo_alerts
    interval: 1m
    rules:
      # Monthly SLO: 99.9% = 0.1% error budget
      
      - alert: ErrorBudgetBurning_Fast
        expr: |
          rate(http_requests_total{status=~"5.."}[1h]) > 0.03
        for: 1m
        labels:
          severity: critical
          slo: "99.9"
        annotations:
          summary: "Error budget burning fast (1h window)"
          
      - alert: ErrorBudgetBurning_Slow
        expr: |
          rate(http_requests_total{status=~"5.."}[6h]) > 0.0012
        for: 15m
        labels:
          severity: warning
          slo: "99.9"
        annotations:
          summary: "Error budget burning slowly (6h window)"
      
      - alert: ErrorBudgetExhausted
        expr: |
          rate(http_requests_total{status=~"5.."}[30d]) > 0.001
        for: 1m
        labels:
          severity: warning
          slo: "99.9"
        annotations:
          summary: "Monthly error budget exhausted"
```

## Alert Routing and Escalation

**Alert routing:** Different alerts go to different teams and schedules.

```yaml
# Alertmanager config
routes:
  - receiver: 'pagerduty'
    group_by: ['alertname', 'service']
    routes:
      # Payment service SLO breach → Page payments team immediately
      - match:
          service: 'payment'
          severity: 'critical'
        receiver: 'pagerduty-payments'
        group_wait: 0s
        repeat_interval: 5m
      
      # Database alerts → Page SRE team
      - match:
          service: 'database'
          severity: 'critical'
        receiver: 'pagerduty-sre'
        group_wait: 10s
      
      # Non-critical → Slack only
      - match:
          severity: 'warning'
        receiver: 'slack-eng'
        group_wait: 30s
        repeat_interval: 8h  // Repeat every 8h unless resolved
```

### Escalation

```yaml
# PagerDuty escalation policy
Escalation Policy: Payment Team
├─ Level 1 (5 min): Alice (primary on-call)
├─ Level 2 (10 min): Bob (backup if Alice doesn't ack)
├─ Level 3 (15 min): Carlos (manager)
└─ Level 4 (20 min): On-call leadership
```

**Effect:** If Alice doesn't acknowledge alert in 5 minutes, PagerDuty notifies Bob, then Carlos, etc.

## Alert Correlation and Deduplication

**Problem:** Same root cause fires multiple alerts at once.

```
Database failover:
  ├─ Alert: database-primary unreachable  (1:00)
  ├─ Alert: database-replica promoted     (1:00)
  ├─ Alert: service-a errors spike        (1:00)
  ├─ Alert: service-b errors spike        (1:00)
  ├─ Alert: connection pool exhaustion    (1:01)
  └─ Alert: replication lag                (1:02)

# Engineer gets 6 alerts; only needs to fix database
```

### Correlation by Label

Alertmanager groups related alerts:

```yaml
group_by: ['alertname', 'service', 'severity']  # Group by service
group_wait: 10s  # Wait 10s to collect all related alerts
# Sends one notification with all 6 alerts listed, grouped by service
```

### Deduplication by Fingerprint

Prometheus generates fingerprint (hash of metric labels). Same fingerprint = same alert; deduplicated.

```
Alert 1: {alertname=DatabaseDown, instance=prod-db-1}
Alert 2: {alertname=DatabaseDown, instance=prod-db-1}  # Same fingerprint
# Deduplicated; fires once
```

### Root Cause Alerting

Use recording rules to compute derived alerts (e.g., "Database failover occurred"):

```yaml
- alert: DatabaseFailover
  expr: count(up{job="database"} == 0) > 0 and count(up{job="database"}) > 0
  for: 10s
  # Alerting indicates standby took over; silence replica lag and connection errors
```

## Runbook Linking

**Runbook:** Step-by-step instructions for responding to an alert.

```yaml
alert: "PaymentServiceHighErrorRate"
expr: rate(payment_errors[5m]) > 0.01
annotations:
  summary: "Payment service error rate >1%"
  runbook: "https://wiki.example.com/runbooks/payment-service-errors"
  dashboard: "https://grafana.example.com/d/payment-service"
  severity: "critical"
```

When alert fires:
```
PagerDuty / Slack notification:
  Payment Service High Error Rate
  ├─ Dashboard: grafana.example.com/d/payment-service
  ├─ Runbook: wiki.example.com/runbooks/payment-service-errors
  ├─ Error rate: 2.5% (SLO: <0.1%)
  └─ Duration: 5 minutes
```

On-call engineer clicks runbook → gets:

```
# Payment Service Error Rate Spike

1. Check Dashboard
   - View payment service error latency graph
   - Check dependencies (auth service, payment processor)

2. Check Recent Deployments
   - Did payment-service deploy in last 5 min?
   - If yes, rollback: kubectl rollout undo -n prod deployment/payment-service

3. Check Dependencies
   - Is auth-service responding? (check up{job="auth"})
   - Is payment processor API responding? (check outbound HTTP errors)

4. If local services healthy:
   - Check database CPU/connections
   - Check Stripe API status page

5. Escalate if:
   - Error rate still >1% after 10 min
   - Database is down
   - Payment processor is down (page them)
```

**Benefit:** Reduces MTTR (mean time to recovery); on-call engineer doesn't debug from scratch at 3am.

## Alert-as-Code Patterns

Alerts defined as code (YAML or Terraform) alongside service code.

**Benefits:**
- Version control; audit trail of alert changes
- Code review; catch bad thresholds before alerting
- Environment parity; same alert config in prod and staging
- Automated alert management

**Example: Prometheus alert as Helm values:**

```yaml
# payment-service/values.yaml
alerts:
  errorBudgetBurningFast:
    expr: rate(payment_errors[1h]) > 0.03
    for: 1m
    severity: critical
    
  errorBudgetBurningSlow:
    expr: rate(payment_errors[6h]) > 0.0012
    for: 15m
    severity: warning
    
  latencyHigh:
    expr: histogram_quantile(0.95, rate(payment_latency[5m])) > 1s
    for: 5m
    severity: warning
```

**Deployment:**

```bash
# Helm generates Prometheus alerts from values
helm template payment-service . | grep -A 10 "PrometheusRule"
# Output: alerting rules configured in Prometheus
```

## PagerDuty / OpsGenie Integration

Both platforms provide on-call scheduling and incident response orchestration.

### PagerDuty

**Workflow:**
```
Alertmanager ──→ PagerDuty Webhook ──→ Incident created
                                       ├─ Check escalation policy
                                       ├─ Page primary on-call (SMS + app)
                                       ├─ Start incident timeline
                                       └─ Create slack channel #incident-123
```

**Example webhook config:**

```yaml
# Alertmanager config
receivers:
  - name: pagerduty
    pagerduty_configs:
      - service_key: "YOUR_SERVICE_KEY"
        description: '{{ .GroupLabels.alertname }}'
        details:
          firing: '{{ range .Alerts.Firing }}{{ .Labels.instance }} {{ end }}'
          firing_count: '{{ len .Alerts.Firing }}'
```

**Features:**
- On-call scheduling with timezone handoff
- Escalation policies (1st → 2nd → 3rd level)
- Incident response apps (war rooms, status pages)
- Incident analytics (MTTF, MTTR, on-call load)

### OpsGenie (Atlassian)

Similar to PagerDuty; emphasis on alert deduplication and team-based routing.

```yaml
# OpsGenie Alertmanager integration
receivers:
  - name: opsgenie
    opsgenie_configs:
      - api_key: "YOUR_API_KEY"
        description: '{{ .GroupLabels.alertname }}'
        tags:
          - '{{ .GroupLabels.service }}'
          - '{{ .GroupLabels.severity }}'
```

**Comparison:**

| Aspect | PagerDuty | OpsGenie |
|--------|-----------|----------|
| On-call scheduling | Mature; complex | Simpler UI |
| Integration | Deeper with observability tools | Broader enterprise integration |
| Cost | ~$50-100/user/month | ~$30-50/user/month |
| Incident response | Rich war rooms and runbooks | Team-focused; lighter weight |

## See Also

- sre-slo-engineering (SLI/SLO definition)
- observability-distributed-tracing (incident diagnosis)
- sre-on-call.md (on-call practices)