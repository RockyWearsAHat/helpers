# Process: Postmortem Writing — Analysis, Learning, and Blameless Culture

## What Postmortems Are (And Are Not)

A postmortem is a structured investigation after an incident: what happened, why, and what changes prevent recurrence. **Not** a blame assignment. Not a punishment. Not a root cause conclusion (almost never one single cause).

The goal: **organizational learning**. Incidents reveal system fragility. The postmortem extracts the lesson so the system becomes more resilient.

Blameless postmortems work because they align incentives: engineers are honest about what went wrong when honest doesn't mean career risk. Defensive postmortems hide information: "It wasn't me, it was the oncall engineer before me," and the systemic failure never gets fixed.

## Postmortem Triggers and Timing

A postmortem is warranted if:
- Customer-facing outage (production incident causing impact)
- Service degradation exceeding SLO
- Data loss or integrity issue
- Security incident
- Near-miss / what-if scenario (sometimes; risky if overused)

**When to write:** Within 24–48 hours while memory is fresh but enough time has passed to think clearly. Too soon: emotions high, details fuzzy. Too late: context forgotten.

**Who writes:** Often the incident commander (IC) or engineering lead, with key participants (the person whose code was involved, the oncall who responded, the SRE who mitigated). No blame-assignment author; collaborative sprint.

## Anatomy of a Strong Postmortem

### Executive Summary

1–2 paragraph overview: What happened in plain English, impact, and resolution.

```
On 2025-03-24 at 08:15 UTC, a database migration in the billing service 
introduced a column without a NOT NULL constraint. When this column was 
populated by a batch job, NULL values were written, causing billing 
calculations to fail. Approximately 12,000 transactions were unprocessed 
over 47 minutes until the migration was rolled back. No data was lost.
```

An executive reader (non-technical) should understand the **business impact** here.

### Timeline (Forensic Accuracy)

Reconstruct exactly what happened, with timestamps:

```
08:15 UTC — Deployment of billing-service v3.4.0 begins (automated canary: 5% of traffic)
08:16 UTC — Database migration runs (adds `discount_applied` column, NOT NULL constraint deferred)
08:18 UTC — Canary passes health checks; deployment continues to 50%
08:21 UTC — Alerta: "Billing processing latency > 5min (p95)" fires on canary cohort
08:23 UTC — Batch job starts scheduled daily reconciliation; writes bulk UPDATE to new column
08:24 UTC — UPDATE fails: column requires NOT NULL, but no default specified for existing rows
08:25 UTC — Batch job rolls back; retries in loop; oncall paged
08:26 UTC — IC page escalates to database team
08:27 UTC — Database team identifies migration issue; recommends rollback
08:29 UTC — Deployment rolled back to v3.3.9
08:32 UTC — Service returns to normal; alarm clears
08:45 UTC — All-clear announced; IC closes incident
```

**Why accurate timeline matters:** It answers hidden questions. "Why didn't we catch this in staging?" (Because the batch job doesn't run in staging). "Why did rollback take 3 minutes?" (CI re-ran all checks). The timeline is evidence.

### Impact Quantification

Vague: "Some transactions failed."
Precise:

```
Duration: 08:23–08:32 (9 minutes of degradation; 47 minutes of canary exposure)
Transactions failed: 12,047 (0.3% of daily volume)
Processing delay: Transactions processed 6–24 hours late (reconciliation re-ran next day)
Revenue impact: $0 (transactions were retried; no data loss; no customer refund required)
Severity: Page 1 (required incident commander escalation; multiple teams; business communication)
Scope: Canary cohort affected; full rollout was prevented
Repeat risk: If canary had been 100% instead of 5%, impact would be total billing-service outage
```

Quantification drives priority: It's the difference between "we should fix this when convenient" and "this is a P1 that prevents shipping anything else until fixed."

### Contributing Factors (Not Root Cause)

Distinguish **immediate cause** (what broke) from **factors that allowed it** (why it mattered).

Immediate cause: Migration added a NOT NULL column without a default; batch job expected a default.

Contributing factors:
- No integration test that runs the batch job in staging (would have caught immediately)
- Migration lacked a default value; would have been safe with `DEFAULT ''` or `DEFAULT 0`
- Canary deployment was only 5% of traffic; if 100%, outage would have been caught sooner and been more severe
- Batch job retried in loop without alerting on repeated failure (resilience flaw)
- Rollback procedure required re-running all CI checks (took 3 min; could be faster)
- No pre-deployment checklist for "does this migration have a backward-compatible default?"

**Each factor is actionable.** "Root cause" is often vague ("human error," "insufficient testing") and ends the investigation instead of deepening it.

### Five Whys (and Its Limitations)

A classic technique: keep asking "Why?" until you reach systemic failure.

```
Q1: Why did the deployment break billing?
    Because the migration added a NOT NULL column without a default.

Q2: Why did it not have a default?
    Because the developer wrote the migration manually and forgot the DEFAULT clause.

Q3: Why was this not caught before deployment?
    Because there's no integration test that runs migrations + batch job together.

Q4: Why isn't there such a test?
    Because migration testing was assumed to be a DBA job; developers didn't own it.

Q5: Why is that assumption in place?
    Because database schema changes were historically handled by DBAs; developers were not trained.
```

By Q5, you've moved past "person forgot" to "process doesn't distribute responsibility." That's fixable.

**Limitations of Five Whys:** It assumes a linear chain. Reality is often a confluence: missing test + missing default + insufficient canary size all mattered. Stop when you reach systemic factors; don't force artificial additional Whys.

### Ishikawa Diagram (Alternative to Five Whys)

Organizes contributing factors by category (People, Process, Technology, Tools, Environment):

```
People:
  ├─ Developer unfamiliar with migration best practices
  ├─ No clear ownership of migration safety
  └─ Batch job developer didn't consult on schema changes

Process:
  ├─ No pre-deployment migration review checklist
  ├─ Migration testing not in CI
  ├─ Batch job not run in pre-staging
  └─ Canary threshold too conservative (5% instead of 25%)

Technology:
  ├─ Database doesn't enforce safe migrations natively
  ├─ Batch job vulnerability to unexpected NULLs
  └─ No schema lint tool to catch missing defaults

Tools:
  ├─ CI doesn't run integration tests with batch job
  ├─ Deployment rollback re-runs all checks (slow)
  └─ Alerting for batch failures (retrying silently instead)
```

Diagram reveals that it's not one factor; it's multiple layers of missing safeguards.

## Action Items (Effective vs. Vague)

**Ineffective action items:**
- "Improve testing" (vague; no owner; never gets done)
- "Better communication" (what communication?)
- "Don't make this mistake again" (you won't; but different mistakes will happen)
- Owner/due date left blank

**Effective action items:**
- `[Owner: DBE] Add integration test to CI: run full migration suite + batch job in staging before every deployment. Due: 2 weeks.` (Specific. Owner named. Deadline set. Measurable.)
- `[Owner: Platform] Add `.migratedefault` requirement to schema linter; fail CI if migration adds NOT NULL column without default. Due: 1 week.` (Specific.)
- `[Owner: DBE] Review canary deployment config; increase threshold from 5% to 25% for staged rollout of schema changes. Due: 1 week.` (Specific.)

Vague action items often become "technical debt" — listed, then forgotten. Specific, owned, deadlined items get done.

## Postmortem Template

```
# Incident Postmortem: [Service] [Date]

## Executive Summary
[1-2 sentences of business impact]

## Timeline
[Chronological events with timestamps and decision points]

## Impact
- Duration: [start time] to [end time] ([X minutes of outage])
- Affected: [X transactions / Y users / Z% of traffic]
- Revenue: [if quantifiable]
- Severity: [P1/P2/P3]

## Immediate Cause
[What broke?]

## Contributing Factors

### Process
- [Factor 1]
- [Factor 2]

### Technology
- [Factor 1]

### Organization
- [Factor 1]

## Timelines and What We'll Do

### Immediate (< 1 week)
- [Action] — [Owner] — [Due date]

### Short-term (1–4 weeks)
- [Action] — [Owner] — [Due date]

### Long-term (1–3 months)
- [Action] — [Owner] — [Due date]

## Lessons Learned
[2-3 sentences capturing the core lesson for the organization]

## Appendix: Investigation Notes
[Slack logs, screen recordings, error stack traces, etc.]
```

## Sharing and Follow-Up

**Where postmortems live:** Shared wiki or internal doc system (Notion, Confluence, Google Drive). Searchable and discoverable.

**Who reads them:** Engineering, product, support. Non-engineers benefit from the executive summary and lessons.

**Follow-up:** 1–2 weeks after resolution, team checks: Have action items been started? Are blockers surfaced? Mid-term retrospective moves items to long-term if unblocked.

**Metric:** Track time-to-close on postmortem action items. If they average 60 days to close, you're learning. If they accumulate for a year, your incident response process is theater.

## Blameless Culture Is a Choice

Blameless postmortems don't happen accidentally. They require:
- **Leadership commitment** to not punish based on postmortem content
- **Skilled facilitation** (someone who can ask "why?" without accusation)
- **Psychological safety** (team feels safe telling the truth)
- **Repetition** (one postmortem won't establish culture; consistent practice over months does)

When an engineer hides details in a postmortem because they fear retaliation, **you've failed at learning**.

---

## See Also

- [SRE Postmortems](sre-postmortems.md) — deeper SRE perspective on incident analysis
- [Incident Communication](process-incident-communication.md) — status page and stakeholder communication during incidents
- [Incident Management](sre-incident-management.md) — detection, response, and recovery phases