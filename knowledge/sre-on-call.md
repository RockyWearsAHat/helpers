# SRE On-Call Practices — Rotations, Escalation & Burnout Prevention

## The On-Call Role

On-call engineers are responsible for responding to production incidents during their shift. They're the first human in the chain: alerts page them, they investigate, triage, mitigate (often), and escalate if needed. The role is unpredictable—some shifts are quiet, others involve multiple cascading failures at 3am.

On-call is simultaneously: critical (systems depend on fast incident response), stressful (interruptions and uncertainty), and asymmetric (the harm of undersleep is borne by the engineer, not the organization). **Sustainable on-call requires intentional design**, not hope that engineers will "just be available."

## Rotation Design

### Primary & Secondary Model

**Primary on-call** is the first point of contact. Phone rings, Slack notification fires, email lands—they respond.

**Secondary on-call** (or "shadow"/"backup") is escalated to if primary doesn't respond within a time window (typically 5–15 minutes), or if they need help during a major incident. Dual-on-call during complex incidents reduces cognitive load, prevents single points of failure, and gives newer on-call engineers supervised experience.

### Rotation Duration

Trade-offs by length:

| Duration | Pros | Cons |
|----------|------|------|
| **1 week** | Frequent rotation; short attention window; fair distribution | High context-switching overhead; ramping up/down eats 40% of shift |
| **2 weeks** | Balance of stability and rotation frequency; team usually prefers this | Risk of fatigue if heavy incidents cluster |
| **1 month** | Deep ownership; minimizes ramp time | Long burnout window; unfair if incidents cluster; high stakes if you miss something critical |

Most mature teams converge on **2 weeks**, with primary on-call handling alerts and secondary available for escalation. Some teams also maintain a "week of backup" (tertiary) for major incidents requiring 3+ people.

### Fair Scheduling

Fairness is both ethical and practical; unfair rotations create resentment and retention issues.

- **Exclude timezone mismatches**: If primary on-call is in Europe and incidents spike at 9am US time, they're sleeping through peak chaos. Use geographic rotations (Europe team covers Europe on-call) or accept lower utilization for global coverage.
- **Minimize consecutive shifts**: "You just came off rotation; here's your on-call shift in 2 days" is exhausting. Stagger rotations so recovery time is guaranteed—if teams are large enough (10+ engineers), a 2-week shift followed by 4 weeks off is fair.
- **Holiday coverage**: Explicitly plan who covers holidays. Rotating on-call during paid time off (and not compensating time-off) is quietly brutal. Option: hire contractors for holiday weeks, or rotate coverage + give compensatory time off later.
- **Bias toward experience**: Senior engineers should take slightly heavier rotations than juniors. New on-call engineers benefit from lighter loads (more prep time, more senior backup, fewer shifts per cycle).

### Handoff Protocols

The transition between rotations is high-risk. A bug or assumption forgotten during handoff becomes the new on-call's problem.

**Structured handoff**:
- **Live walkthrough** (not just email): Outgoing to incoming, live Slack or video call. Show them: current alerts (false positives to ignore?), recent incidents (new failure modes people should know), infrastructure changes deployed this week, known issues.
- **Context document**: Maintained throughout the shift, updated real-time with incidents, customer issues, deployments that might cause alerts. Incoming on-call reads it before shift starts.
- **Runbook updates**: If you discovered a procedure during your shift that isn't in runbooks, document it immediately. Turnover is when knowledge dies.
- **Escalation path clarity**: "If X alert fires together with Y, escalate to team lead. If Z happens, it's usually caused by..." Incidental knowledge is the most valuable.

## Alert Quality & Signal vs. Noise

### The Signal-to-Noise Crisis

Most on-call teams are drowning in low-signal alerts. The culprit: alerting threshold tuning is hard, so alerts are tuned conservatively (fire on any anomaly). Result: hundreds of alerts per shift, 90% of which are false positives or benign transients. Engineers learn to ignore pages, or page fatigue erodes their judgment.

**Alert fatigue is a reliability risk**: When a true critical alert arrives, it's just noise to an exhausted engineer.

### Actionable Alerts

An alert should trigger immediate investigation and possible action. Design each alert with:

1. **Clear condition**: "API p99 latency > 500ms for 5 min" is clear. "Traffic anomaly detected" (ML-based) is vague.
2. **Severity calibration**: Page (wake them up) only for issues that need immediate action. Warn or ticket lower-severity problems.
3. **Runbook**: Auto-link or embed: "If this alert fires, here's what to check / common causes / mitigation steps." An alert without a runbook is malpractice.
4. **Context data**: Alert includes recent metrics (traffic, error rate), recent deployments, current state (is database CPU spiking? Is this expected?).
5. **Thresholds learned from incident history**: If this alert has never correlated with real incidents, turn it off. If it correlates 1% of the time, it's probably noise.

### Alert Triage & Classification

**SEVx classification**:

- **SEV-1/Critical**: Immediate customer impact, metric clearly outside acceptable bounds, requires immediate human intervention. Page on-call. Examples: API returns 5xx errors, critical batch job fails, database unavailable.
- **SEV-2/High**: Customer impact likely soon, or partial impact now. Page on-call but can wait a few minutes. Examples: Increased error rate in monitoring system itself, slow degradation in latency, resource exhaustion not yet critical.
- **SEV-3/Medium**: Limited or no customer impact now, but should be investigated. Ticket creation or Slack notification only. Examples: Unusual disk usage pattern, memory leak early warning, niche feature degrading.
- **SEV-4/Low**: Observational (not actionable). No notification needed. Examples: A particular database table is growing, experiment cohort is off-target.

The key discipline: **Page only on SEV-1 and SEV-2 when there's genuine urgency.** Teams that page on everything train on-call engineers to ignore pages.

## Runbooks & Incident Response

### Runbook Structure

A runbook is a step-by-step guide for investigating and responding to an alert. Quality runbooks are the difference between a 2-minute resolution and a 30-minute debug session.

**Components**:

- **What this alert means**: Plain English. Not "p99_latency_spike_v3" but "this means the API is responding slowly to 50% of requests; usually caused by CPU saturation or database queries."
- **Common causes** (ranked by frequency): "In order of likelihood: database CPU too high, cache miss storm, or new deployment depleted connection pool."
- **Diagnostic commands**: Exact queries, CLI commands, logs to check. Example: `SELECT count(*) FROM event_log WHERE created_at > now() - interval '5 min';` with interpretation of expected value.
- **Remediation steps**: What can I do right now? Check if the mitigation is safe (won't make it worse). Examples: "Scale database read replicas," "Trigger failover," "Roll back last deployment," "Kill hanging process."
- **Escalation criteria**: "If CPU doesn't drop after 5 minutes, page the database team. If the cache restart fails, involve storage engineering."
- **Links**: To dashboards, relevant documentation, similar past incidents, relevant code sections.

### Incident Response Workflow

1. **Alert received**: Check runbook immediately (many may never need human action; self-heal in seconds)
2. **Triage** (30 sec): Is this real? What's the scope? (Is it one customer or all? One region or global?)
3. **Mitigation** (1–5 min): Apply quick fix if in runbook (scale, restart, rollback). Mitigate impact before investigating root cause.
4. **Communicate** (parallel): Status updates to status page, incident tracking system, maybe Slack. Reduce uncertainty, reduce secondary chaos.
5. **Investigate** (ongoing): Root cause analysis in parallel with mitigation. Was it a deployment? Traffic surge? Underlying resource exhaustion?
6. **Resolve** (varies): Fix the underlying issue or decide it's acceptable transient.
7. **Notify secondary/escalate if needed**: If this is beyond your authority or complex, escalate to senior on-call or on-call manager.
8. **Document**: Immediately capture timeline, what happened, what you did, what to watch. Becomes the postmortem's starting material.

## Toil Measurement & Burnout Prevention

### Defining On-Call Toil

**Toil** is unstructured, repetitive work: responding to pages, investigating false alerts, manually fixing the same problem repeatedly. Toil is the primary burnout vector in on-call.

**Telemetry to measure toil**:

- **Incident frequency**: How many pages per shift? Baseline target: 0–2 pages per 24-hour shift. >5/shift is unsustainable.
- **Time-to-resolution**: How long from alert to mitigation? Baseline: 5–15 min for common incidents, 30+ min for novel ones.
- **False positive rate**: How many pages require no action? Should trend <10%; >30% signals alert tuning failure.
- **Emergency wake-ups**: How many pages happen outside business hours (2pm–10am)? Track separately; high nighttime load is a burnout risk.
- **Repeat incidents**: Same root cause two weeks apart? That's a sign action items aren't being followed.

### Burnout Prevention Strategies

**Hard limits**:

- **No more than 2 emergency wake-ups per rotation**: If shifts are weekly and someone wakes up emergency 2+ nights, they'll be exhausted.
- **Guaranteed recovery time**: After heavy incident clusters, give on-call engineers a "light duty" shift (lower priority services, or pair with senior engineer).
- **Vacation is sacred**: On-call shifts should not overlap with vacation planned months prior. Switching shifts at last minute to accommodate coverage is a signal that staffing is insufficient.

**Soft improvements**:

- **Reduce pages by improving monitoring**: Automate detection, fix false positions, raise thresholds. Every page that never needs to page again is reclaimed time.
- **Runbook quality investment**: Time spent documenting reduces time future on-call engineers spend puzzled.
- **Incident review culture**: After every major incident, ask: "Could this have been prevented? Could it have been detected earlier? Could resolution have been faster?" Apply those lessons to reduce recurrence.
- **On-call stipend / compensation**: Some companies pay on-call engineers extra (e.g., $500/week) or give comp time (1.5 hours off per shift). Signaling respect for the burden improves retention.

## Escalation Paths

A clear escalation path prevents confusion during crises.

**Typical hierarchy**:

1. **On-call engineer**: First responder, follows runbook
2. **Escalation to team lead** (or senior on-call): If incident is beyond on-call's authority, or after X minutes of investigation with no resolution
3. **Escalation to manager/director**: If customer impact is severe, requires cross-team coordination, or needs executive decision
4. **Incident commander** (for large incidents): One person coordinates all responders, communicates status

**Escalation triggers**:

- **Time-based**: "If mitigation isn't successful after 10 minutes, page team lead"
- **Scope-based**: "If database is affected (not just one service), escalate immediately"
- **Request-based**: Original on-call can request help anytime, no judgment
- **Preference-based**: "If uncertain whether to escalate, page. False escalations are cheaper than delayed response."

## Tooling

On-call tooling (Pagerduty, Opsgenie, VictorOps, OpsLevel) handles scheduling, escalation routing, alert aggregation, and communication. Choose based on:

- **Integration with existing stack** (monitoring systems, incident tracking, communication platforms)
- **Scheduling flexibility** (geographic rotation, fairness features)
- **Escalation policy expressiveness** (can you encode "if this + that alert, then escalate to this team?")
- **Incident management** (timeline tracking, postmortem integration)
- **Cost** (usually per seat per month)

No tool solves the underlying problems (alert fatigue, coverage gaps, burnout). But good tooling removes friction.

## On-Call Culture

Sustainable on-call is built on shared understanding:

- **On-call isn't punishment**: It's a critical skill and rotation, not a burden assigned to junior engineers.
- **Mistakes are learning**: If on-call misses an escalation or misinterprets an alert, the response is coaching, not blame.
- **Infrastructure improvement is everyone's job**: If a particular service pages frequently, the team doesn't ask "why didn't on-call fix this?" but "why does this service need so much on-call attention? Let's improve it."
- **Trade-offs are explicit**: "We're choosing 99.9% reliability, which means roughly 1 page per month. We could target 99.99%, but that's 10× more toil and cost."

Without this culture, on-call becomes a source of turnover and reliability actually *decreases* as experienced engineers leave.