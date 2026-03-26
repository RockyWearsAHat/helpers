# Process: On-Call Practices — Rotations, Escalation, and Sustainable Response

## The On-Call Role

On-call engineers respond to production incidents during their shift. Alerts page them. They investigate, triage, and mitigate or escalate. The role is **reactive by design**: incidents are unpredictable. Some shifts are quiet; others involve cascading failures at 3am.

On-call is critical, stressful, and asymmetric: the engineer bears the harm (sleep disruption, context switching) while the organization reaps the benefit. Sustainable on-call requires intentional design.

## Rotation Design and Fairness

### Duration Tradeoffs

| Duration | Pros | Cons | Best For |
|----------|------|------|----------|
| **1 week** | Frequent rotation; short attention window | High ramp-up/down overhead; exhausting context switching | Teams with 10+ on-call capable engineers |
| **2 weeks** | Stability; time to build context; fair | Risk of fatigue if incidents cluster | Most teams; balances predictability and fairness |
| **1 month** | Deep ownership; minimal ramp time | Long burnout window; unfair if incidents cluster | Mature teams with proven incident patterns |
| **3 months+** | Extreme ownership | Unsustainable; retention risk; outdated knowledge | Avoid; only for specific senior roles |

Most mature teams converge on **2 weeks primary + secondary backup**.

### Primary and Secondary Model

**Primary on-call** is the first point of contact: alerts page them; they respond.

**Secondary on-call** (backup, shadow, or escalation) is:
- Called if primary doesn't acknowledge within 5–15 minutes (configurable per alert severity)
- Involved automatically for major incidents (Page 1) to reduce cognitive load
- Experience path for newer on-call engineers (supervised; never alone)

**Tertiary (rare):** Large incidents (cascading failures, multi-team) may need a third person for coordination.

### Fair Scheduling Rules

Fairness is both ethical and practical (unfair rotations cause attrition).

**Geographic fairness:** On-call coverage should align with incident risk. If incident rates spike at 9am US-Eastern, don't schedule a Europe-based engineer for US night shifts. Use:
- Geographic rotations (Europe team covers Europe hours; US team covers US hours)
- Accept lower utilization for global coverage
- Hire contractors for 24x7 coverage if budget allows

**Consecutive shift avoidance:** "You came off rotation 48 hours ago; here's your next shift" is burnout. Stagger rotations so recovery time is guaranteed.

```
Rotation schedule (8-person team, 2-week shifts):
Week 1-2:   Alice primary, Bob secondary
Week 3-4:   Charlie primary, Diana secondary
Week 5-6:   Eve primary, Frank secondary
...
By design: Alice's next shift comes 12 weeks later; plenty of recovery time
```

**Holiday coverage:** Explicitly plan who covers holidays. On-call during paid time off (and not being compensated) is quietly brutal. Options:
- Hire contractors for holiday weeks
- Rotate coverage + give compensatory time off later
- Accept temporarily lighter coverage during holidays

**Experience bias:** Senior engineers take heavier rotations than juniors. New on-call engineers benefit from lighter loads, more senior backup, and protected time to learn.

### Handoff Protocol

Transition between rotations is high-risk; a forgotten detail becomes the incoming engineer's emergency.

**Structured live handoff (not email):**

```
Outgoing to Incoming: 30-minute live call. Discuss:
- Current alerts that are false positives (engineer knowledge)
- Recent incidents and new failure modes discovered
- Deployments or infrastructure changes this week
- Known issues tracking
- Escalation paths ("If alert X fires with Y, page team lead")
- Quick wins (things that were almost escalated but prevented)
```

**Context document (maintained during shift):**

```markdown
## On-Call Context — March 24–30, 2025

### Past 48 hours
- API service had high latency 2025-03-23 03:15–03:42 UTC (database query issue; fixed in v1.4.2)
- S3 bucket IAM role misconfigured (fixed; no data loss)

### Deployments this week
- Payment service v2.0 (schema migration; watch for transaction delay alerts)
- Infrastructure: upgraded Kubernetes to 1.29

### Known Issues
- Database read replica occasionally lags by 2–5 seconds (tracked in JIRA-8234; SRE investigating next sprint)
- Batch job sometimes gets stuck; watchdog timeout at 10 minutes (needs refactor; scheduled for Q2)
- Alert: "High memory pressure" on production-db-3; threshold should be 85% not 70% (false positive; adjust in next cycle)

### Escalation Paths
If payment-processing-p95-latency alert fires WITH high-db-connections alert:
  → Page database team (Slack: @db-team-oncall)
If API service errors exceed 1% for 5 min:
  → Page platform team; it's likely a deployment issue
If alert fires but you're unsure: **Page secondary.** That's what they're there for.

### Contact List
- Platform Team Lead (Sean): sean@company → +1-555-0101
- Database Team Lead (Yuki): yuki@company → +1-555-0102
- Infra Team Slack: #infrastructure-on-call
```

Updated live during the shift. Incoming engineer reads before shift starts.

**Runbook updates:** If you discovered a procedure during your shift that isn't in runbooks, **document it immediately**. Turnover is when critical knowledge dies.

## Alert Quality and Signal-to-Noise

### The Alert Fatigue Crisis

Most on-call teams are drowning: hundreds of alerts per shift, 90%+ false positives or benign transients. Engineers learn to ignore pages. When a true critical alert arrives, it's just noise.

**Alert fatigue is a reliability risk**: Exhausted engineers miss critical alerts.

### Actionable Alerts

An alert should:

1. **Indicate human action is needed.** Not: "CPU 45% for 1 minute" (noisy). Yes: "Disk 95% full; purge old logs or scale up storage."
2. **Have clear context.** Alert message includes: affect (users? transactions? region?), history (is this recurring?), remediation (what do I do?).
3. **Be tuned to low false-positive rate.** Goal: 80%+ of pages represent actual problems. Acceptable false-positive rate: 20%.

**Before deploying an alert, ask:**
- What would I do if this fires at 3am?
- Is the alert actionable sane? (If the answer is "probably nothing," don't alert.)

### Alert Tuning Examples

**Too sensitive (false positive rate ~60%):**
```
Alert: APILatencyP95 > 200ms for 30 seconds
Problem: Normal variation; constantly triggers
Fix: Increase threshold to 500ms; require sustained 3+ minutes
```

**Too insensitive (misses real problems):**
```
Alert: DiskspaceFree < 1GB
Problem: Doesn't fire until you're nearly out of space
Fix: Alert at 20% free; gives operators time to act
```

**Good alert:**
```
Alert: TransactionFailureRate > 5% for 2+ consecutive minutes
Action: Re-check database connection pool; check recent deployments
Escalate if: Also have high latency or memory pressure alerts
```

### Runbooks and Context

A runbook is a decision tree for responding to alerts:

```
## Alert: "Database Connection Pool Exhausted"

### Is this a normal spike?
- Check transaction volume: `SELECT count(*) FROM transactions WHERE created_at > now() - interval '5 min'`
- If traffic is normal (no spike), continue. If spike, that's investigation.

### Immediate mitigation
1. Restart the connection pool (may bring brief latency spike)
   - SSH prod-api-1: `/opt/restart-pool.sh`
2. Monitor P95 latency for 2 minutes; should recover to baseline

### If still not recovered after 10 min
- Page database team (@db-team-oncall)
- Get recent deployments: check CI/CD pipeline for last 12 hours
- If new deployment: consider rollback

### If connection pool keeps exhausting after restart
- Scale up connection pool size (temporary): config change + rolling restart
- Contact senior on-call; this is likely a systemic issue
```

Runbooks save lives (or at least reduce drama). Without them, on-call becomes firefighting. With them, it's systematic problem-solving.

## Escalation Policy

An escalation policy defines who to contact if primary doesn't respond, or if a ticket exceeds thresholds.

**Example escalation policy:**

```
Severity  | Primary Response | Timeout | Secondary | Secondary Timeout | Tertiary
--------  | --------------- | ------- | --------- | -------------- | --------
P1 (critical) | ~2 min (immediate) | 5 min | Page secondary | 5 min | Page VP Eng
P2 (high)     | ~5 min          | 10 min | Page secondary | 10 min | Page manager
P3 (medium)   | ~15 min         | 30 min | (async only)   | N/A | N/A
P4 (low)      | ~1 hour         | (daily digest) | (none) | N/A | N/A
```

**Escalation triggers** (automatic, not discretionary):
- Primary doesn't acknowledge within timeout → page secondary
- Secondary doesn't respond within timeout → page VP / management
- Critical alert fire during on-call → **always** page secondary immediately (dual-on-call for P1)

---

## Compensation and Burnout Prevention

### Time-Off Compensation

On-call outside business hours is work. **Compensate.**

Options (pick one):
1. **Cash comp** (~$3–10 per hour on-call; $20–100 per page, depending on org size)
2. **Time-off comp** (page between 10pm–7am = one hour of PTO)
3. **Hybrid** (small cash comp + PTO for pages)
4. **Contractor on-call** (hire 3rd-party support; higher cost, zero internal load)

Option 1 or 2 shows engineers you value their sleep. Option "nothing" burns out your team within 12 months.

### Shift Frequency and Burden

If you have 8 engineers and 2-week rotations, each engineer is on-call every 16 weeks (4 months). That's sustainable.

If you have 4 engineers, each is on-call every 8 weeks (2 months). That's rough; consider:
- Hiring more on-call capable engineers
- Reducing on-call scope (off-call nights; only business hours)
- Contractor coverage for nights

If you have 2 engineers and both are on-call constantly, **you have a staffing problem, not an on-call problem.**

### Burnout Signals

Watch for:
- Frequent pages during an engineer's rotation (suggests system instability, not personal failure)
- Recurring alerts at same time of day (suggests periodic job failure; fix the job, not the on-call)
- Engineer requesting off-call (says "I don't want this anymore"; listen; probably burning out)
- High page volume across the team (alert fatigue; audit and tune alerts)

**Response:**
- Invest in reliability (reduce pages by fixing root causes)
- Audit alerts (remove low-signal noise)
- Add people to rotation if possible
- Consider rotating high-volume people to lower-volume rotations temporarily

## Knowledge Sharing and Training

### New On-Call Engineer Training

1. Shadow a senior on-call for 2–3 full shifts (8+ hours each)
2. Secondary on-call for 1–2 rotations (they handle incidents; new engineer watches and learns)
3. Primary on-call with senior secondary backup (senior available but not primary)
4. Independent primary

Timeline: 2–3 months from "never on-call" to "independent."

### Incident Reviews

After every incident (or every week during on-call), brief review:
- What went well?
- What was confusing?
- Did the runbook help or mislead?
- Action items: update runbooks, add context, tune alerts

Captures knowledge that would otherwise die.

---

## See Also

- [SRE On-Call Practices](sre-on-call.md) — detailed SRE perspective on rotations and alert quality
- [Incident Management](sre-incident-management.md) — detection, response, recovery phases
- [Monitoring and Alerting](observability-alerting.md) — alert design and signal tuning
- [Runbook Writing](operations-runbooks.md) — structured incident response procedures