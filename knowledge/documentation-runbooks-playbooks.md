# Operational Documentation — Runbooks, Playbooks & Decision Trees

## The Cost of Undocumented Procedures

When the payment system fails at 3 AM:

- On-call engineer pages Slack; nobody responds for 5 minutes because nobody knows who owns payments
- When they do respond, they spend 20 minutes figuring out which dashboards to check
- They misdiagnose the issue first time because they confuse the v1 API with v2
- They restart the wrong service
- Customers see downtime for 45 minutes; it should have been 8

This scenario repeats because operational knowledge lives in **people's heads, not documents**.

**Runbooks** and **playbooks** are operational knowledge made executable: structured guides that reduce time-to-mitigation, improve decision quality, and scale expertise beyond individual engineers.

---

## Distinction: Runbooks vs. Playbooks

The terminology varies across organizations, but a useful distinction:

### Runbooks

Step-by-step **linear instructions** for a specific procedure. "How to deploy the service," "How to add a user to the VPN," "How to rotate credentials."

Assumes: We know what to do. We need to execute it correctly and consistently.

**Audience:** Any on-call engineer, even unfamiliar with the system.

**Typical length:** 5–20 steps.

### Playbooks

**Decision trees** with conditional logic for incident response. "We're getting 500 errors—what do we check first?" "Database is down—should we fail over to read replica?"

Assumes: We don't know the root cause yet. We need to diagnose systematically.

**Audience:** Incident responders with domain knowledge.

**Typical length:** Longer, branching, include examples and escalation paths.

Many organizations use the terms interchangeably; use whichever your team understands, but document both:
1. **Linear procedures** (runbooks) for known tasks
2. **Decision trees** (playbooks) for ambiguous situations

---

## Runbook Structure

### Anatomy of a Good Runbook

```markdown
# [System Name] Runbook

## Overview
What is this procedure? When do we follow it?
One-line decision rule: "Follow this if X"

## Prerequisites
- Access to [tool/service]
- Permissions: [these roles]
- Time estimate: [X–Y minutes]

## Before You Start
- Notify stakeholders? (message board, Slack channel)
- Is this reversible? (can we undo?)
- Any time-sensitive constraints? ("only after 9 PM on Tuesdays")

## Step-by-Step Instructions

### 1. Verify the precondition
Command: `kubectl get pods -l app=payment`
Expected output: All pods are Running (if not, see Troubleshooting)

### 2. Create a backup
Command: `pg_dump prod_db > /backups/prod_db_$(date +%s).sql`
Expected output: File created in /backups/

### 3. Run the migration
Command: `migrate --direction=up --count=1`
Expected output: `[OK] Migrated from version 5 to 6`

### 4. Verify the result
Command: `SELECT version FROM schema_version;`
Expected output: Version should be 6

## Rollback (if needed)
If anything fails after step 2, undo:
```
migrate --direction=down --count=1
```
Then restart the service:
```
systemctl restart payment-service
```

## Success Criteria
- Logs show no errors
- Health check endpoint returns 200
- Metrics show normal traffic

## Who to Contact if This Fails
- **Primary:** @db-oncall (Slack)
- **Escalation:** @platform-lead (email)

## Examples
### Example 1: Typical execution (5 minutes)
(Walk through with actual example commands and output)

## Recent Changes
- 2026-03-20: Added backup requirement after data loss incident
- 2026-02-15: Updated command for new CLI version

## See Also
- Related runbook: [Instance Sizing](instance-sizing.md)
- Architecture docs: [Payment System](payment-system.md)
```

### Runbook Writing Discipline

**Do:**
- Test every single command in the runbook before publishing ("Did we forget a flag?")
- Include expected output, not just commands ("If output doesn't match, you're in the wrong state")
- Break into discrete, verifiable steps (before/after states for each step)
- Provide rollback instructions (can the procedure be undone?)
- Keep Prerequisites and Success Criteria crisp (10 seconds to understand if this applies)
- Include examples with real output (not generic prose)

**Don't:**
- Assume context ("Go to the dashboard" — which dashboard? Where?)
- Mix procedures ("After deploying, also check the cache"—-separate into two runbooks)
- Use jargon without explanation (that DBA might not know your app's term for "stale index")
- Skip the rollback plan
- Write procedures you haven't tested

---

## Playbooks: Decision Trees for Incident Response

A playbook guides responders through **diagnosis under uncertainty**. Unlike runbooks, execution depends on what you find.

### Playbook Structure: Symptoms → Diagnosis → Remediation

```markdown
# Payment Processing Outage Playbook

## When to Use This
- Symptom: Payment transactions returning 500 errors OR timing out
- Symptom: Order volume is 0 for > 5 minutes (should be ~50/min)

## Severity Assessment (immediate)

### Is this active?
1. Check current transaction count: `curl https://api.internal/metrics/payments/tps`
   - If **0 transactions**: **SEV-1** (immediate mitigatation needed)
   - If **< 5 TPS**: **SEV-2** (monitor before escalating)
   - If **normal (50 TPS)**: False alarm; stop here

### Scope assessment
1. Run: `curl -s https://api.internal/payments/health | jq .partitions`
   - If **all green**: Logic layer issue (below)
   - If **some red**: Infrastructure issue (skip to "Infrastructure Checks")

## Phase 1: Quick Diagnosis (< 2 minutes)

### Check 1: Is the payment service running?
```bash
kubectl get pods -l app=payment-api -n prod
```
**If no pods are running:**
- → Go to "Remediation: Restart"

**If all pods are running:**
- → Go to Check 2

### Check 2: Are there errors in logs?
```bash
kubectl logs -l app=payment-api -n prod --tail=50
```
**If you see "Connection refused":**
- → Database is down (go to "Database Issues" below)

**If you see "Timeout calling stripe":**
- → External dependency issue (go to "External Dependency Issues")

**If you see "Out of memory" or similar:**
- → Resource exhaustion (go to "Resource Exhaustion")

**If logs look normal:**
- → Go to Check 3

### Check 3: Database connectivity
```bash
psql -h prod-db.internal -U payment_user -c "SELECT 1;"
```
**If connection fails:**
- → Database is down (see "Database Issues" below)

**If connection succeeds but queries are slow:**
```bash
SELECT count(*) FROM pg_stat_activity WHERE state = 'active';
```
- If **> 50 active queries**: → Query load issue (see "Database Load")
- If **normal**: → Go to Check 4

### Check 4: External dependency (Stripe)
```bash
curl -s -w "%{http_code}" https://status.stripe.com/api/v2/incidents.json | head -20
```
**If Stripe status page shows incidents:**
- → Stripe is down; mitigate below under "External Dependency Issues"

**If Stripe is up but we're still failing:**
- → Our API key might be revoked (go to "Credential Issues")

## Phase 2: Diagnosis Results → Remediation

### Database Issues
**Symptom recognized:** Connection refused or queries timing out.

**Immediate action:**
1. Check replica lag: `SELECT pg_last_wal_receive_lsn(), pg_last_wal_replay_lsn();`
2. If lag > 1 minute: Consider failover
3. Run: `CALL sys_procs.force_reconnect_all();` (prepared statement to disconnect stale connections)

**If database is still down after 30 seconds:**
```bash
kubectl get nodes
# If nodes are NotReady: Infrastructure issue—page infrastructure team
# If nodes are Ready but DB pod is Pending: Disk full?
kubectl describe pod <db-pod-name> -n prod
```

**Escalation:** If unresolved in 2 minutes, page @db-oncall

### Query Load Issue
**Symptom:** Database up but hundreds of active queries.

**Diagnosis:**
```bash
SELECT query, count(*) FROM pg_stat_activity GROUP BY query ORDER BY count DESC LIMIT 5;
```

**If you see a single query dominating (80% of load):**
- Kill it: `SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE query ILIKE '%slow_query%';`
- Investigate why it's running (may be scheduled job or slow client)

**If queries are diverse (normal distribution):**
- Check if a deployment just went out: `kubectl rollout history deployment/payment-api -n prod`
- Check metrics for app memory/CPU spike

### External Dependency Issues (Stripe)
**Symptom:** Stripe API errors or timeouts.

**Immediate:**
1. Check our timeout setting: `cat config/payment-stripe.yml | grep timeout`
2. Check circuit breaker status: `curl http://localhost:9090/metrics | grep circuit_breaker`
3. If circuit breaker is open: Wait 2 minutes, then restart payment-api pods

**If Stripe is actually down:**
- Notification expected from Stripe status page; monitor
- For customer communication: post to #customer-comms with ETA (check Stripe's status page)
- Do NOT retry failed transactions automatically; they'll pile up and fail again

### Resource Exhaustion
**Symptom:** "Out of memory" or "Disk full" in logs.

**Check disk:**
```bash
kubectl exec <payment-pod> -- df -h
# If root / filesystem > 85%: Usually Docker logs are culprit
```

**Clear logs (temporary):**
```bash
kubectl logs --tail=10 <pod> > /tmp/pod.log && kubectl delete pod <pod> -n prod
```
(Pod will restart and rebuild)

**Check memory:**
```bash
kubectl top pods -l app=payment-api -n prod
```
If all pods are > 90% of request limit: Increase resource requests in helm chart

**Escalation:** Page @platform-team if resource exhaustion persists

### Credential Issues
**Symptom:** "Invalid API key" or "Unauthorized" errors from Stripe.

**Check current key:**
```bash
kubectl get secret stripe-api-keys -n prod -o yaml | grep api_key | head -1
```

**Verify the key matches Stripe dashboard:**
- Login to Stripe (use password manager)
- Dashboard → Developers → API Keys
- Compare full key (obfuscated in secret anyway, but prefix should match)

**If key is wrong:**
- Rotate new key from Stripe dashboard
- Update secret: `kubectl set env deployment/payment-api STRIPE_API_KEY=sk_live_... -n prod`
- Restart pods (should have rolling restart, not immediate downtime)

## Phase 3: Mitigation Actions (in severity order)

### SEV-1 Mitigation (0-5 minutes)
1. **Fail over traffic:** If database primary is down, trigger failover to read replica
   ```bash
   kubectl set env statefulset/postgres PRIMARY=replica-1 -n prod
   ```
2. **Drain circuit breaker:** If external dependency down, return user-friendly error to clients (don't lose requests)
   ```bash
   # Drain in-flight requests
   kubectl scale deployment/payment-api --replicas=0 -n prod
   # Then restart cleanly
   kubectl scale deployment/payment-api --replicas=3 -n prod
   ```
3. **Notify stakeholders:** Post in #status-page with incident ID, post on status page

### SEV-2 Actions (5-15 minutes)
1. Gather logs and metrics
2. Alert relevant team via Slack
3. Continue diagnosis if mitigation didn't work

## Recovery Verification

After any mitigation, verify:
1. Transaction rate returning to normal: `curl https://api.internal/metrics/payments/tps`
2. Error rate dropping: Check dashboard, look for red → green
3. Customer complaints stopping: Monitor support inbox for incoming errors

## Escalation Path
- **Database:** @db-oncall
- **Infrastructure:** @platform-lead
- **Stripe integration:** @payments-team
- **If all else fails:** @engineering-manager

## Post-Incident
1. Add finding to [Postmortem Template](postmortem-template.md)
2. If new scenario discovered: Update this playbook
3. Document the root cause and timeline
```

### Playbook Writing: Key Principles

**1. Branch on observable facts, not guesses**

Bad branch logic:
```
IF recent deploy THEN payment-api bug ELSE database issue
```

Good:
```
IF logs show "Connection refused" THEN database issue
ELSE IF logs show application exception THEN code issue
```

Observable facts are reproducible by the next responder.

**2. Provide exact commands, not vague descriptions**

Bad:
```
Check the database
Look at logs
See if anything's weird
```

Good:
```
kubectl logs -l app=payment-api --tail=50
kubectl describe pod payment-api-7d6f8c9b
curl -s http://localhost:9090/metrics | grep payment_errors
```

**3. Include expected output**

```bash
$ kubectl get nodes
NAME                       STATUS   ROLES    AGE
ip-10-0-1-100.ec2.inter    Ready    master   45d
ip-10-0-2-50.ec2.inter     Ready    node     45d

EXPECTED: All nodes should be Ready (if not, infra is down)
```

**4. Make decision trees shallow (3 levels max)**

Deep nested trees become unusable. If you have 5+ levels of branching:
- Split into separate playbooks (one per major component)
- Create flowcharts (visual tree, not nested Markdown)

**5. Test playbooks with new team members**

Have someone unfamiliar with the system follow the playbook (simulated incident). Watch for:
- Where they got confused  
- Commands they had to modify (path was wrong)
- Missing prerequisites
- Ambiguous branch decisions

---

## Living Documentation: Keeping Playbooks Fresh

Playbooks become stale quickly because systems change faster than documentation.

### Pattern 1: Break Playbooks on Deploy

Intentionally break a playbook section when code changes:

```python
# In payment_service/main.py
config.log_tag = "PAYMENT_API_V2_PAYMENT_ROUTE"
# NOTE: Playbook section "Check logs" references V1 tag.
# Update playbooks/payment-outage.md when you change this!
```

In code review: "You changed the log tag? Update the playbook."

### Pattern 2: Automated False Alarms

Add a check to your playbook that will definitely fail if the playbook is stale:

```markdown
### Freshness Check (always do this first)
Run the payment API health endpoint:
```bash
curl -s http://payment-api:9090/health
```

Expected response in 2026 format (v2):
```json
{
  "status": "healthy",
  "version": "2.x"
}
```

If version is "1.x": This playbook is outdated and doesn't match the current system.
Notify @payments-team to update.
```

When engineers see this fail, they know to update the playbook—your documentation quality improves automatically.

### Pattern 3: Ownership Labels

```markdown
# Database Failover Playbook

**Owner:** @db-team  
**Last reviewed:** 2026-02-15  
**Next review due:** 2026-05-15  

[Playbook content...]
```

In your incident management tool (PagerDuty, Rootly), require that owners review their playbooks quarterly.

### Pattern 4: Incident-Driven Updates

After every incident, update the playbook:

```markdown
## Version History
- 2026-03-20: Added "Check circuit breaker status" after incident-001 (false timeouts)
- 2026-02-28: Clarified recovery verification steps
- 2026-02-01: Initial version
```

End every postmortem with: "Should we update the playbook?" The answer is usually yes.

---

## Post-Incident Documentation

After resolving an incident, document:

### Incident Report Structure

```markdown
# Incident Report: XXXX

**Incident ID:** INC-2026-0042  
**Date:** 2026-03-20, 03:45–03:52 UTC (7 minutes total)  
**Severity:** SEV-1 (customers affected)  
**Owner:** @payment-team  

## Timeline
- **03:45** Alert fires: Payment transaction rate drops to 0
- **03:46** On-call engineer joins Slack; starts diagnosis
- **03:48** Identifies: Database connection pool exhausted (500 max, 500 active)
- **03:49** Restart payment-api pods (rolling restart, no downtime)
- **03:50** Transactions resume, rate returns to 50 TPS
- **03:52** Incident marked resolved

## Impact
- 7 minutes of degradation
- ~2,100 transaction failures (estimate based on normal 50 TPS × 7 min)
- Revenue lost: ~$350 (15% average transaction size)

## Root Cause
New feature added SQL queries to every transaction that was missing index. Each query scanned full table (1M rows), taking 200ms. This throttled the connection pool.

The feature was deployed but load testing was skipped due to schedule pressure.

## What Went Well
- Alert fired immediately (within 30 sec of condition)
- On-call engineer knew to check database connections (diagnosis was fast)
- Rolling restart worked correctly (no customers saw deployment error)

## What Went Wrong
- No load test before deploy
- Connection pool monitoring was not set up (we only noticed via alert, not proactively)
- Feature implementation didn't involve DB team for index review

## Action Items
1. **BLOCK NEXT FEATURE**: Add database load testing to deployment checklist (owns: @qa-lead, due 2026-04-01)
2. **PERMANENT**: Add connection pool monitoring to dashboard (owns: @observability-team, due 2026-03-27)
3. **PERMANENT**: Require DB team review for queries touching large tables (owns: @tech-lead, due 2026-03-25)

## Lessons
- Feature flags allow shipping code safely, but still need database review
- Missing indexes are silent failures (database doesn't warn, just slow)
- Postmortems are for learning, not blaming—nobody "failed to load test"

## Playbook Updates
**Playbook changed:** `playbooks/payment-outage.md`
Added step: "If connection pool > 80% utilization: Check for newly deployed queries without indexes"
```

---

## See Also

- [sre-incident-management.md](sre-incident-management.md) — full incident lifecycle and severity levels
- [sre-postmortems.md](sre-postmortems.md) — how to run effective postmortems
- [process-incident-communication.md](process-incident-communication.md) — stakeholder communication during incidents
- [monitoring-incident-tooling.md](monitoring-incident-tooling.md) — PagerDuty, Rootly, and automation