# SRE Postmortems — Blameless Analysis & Learning Organizations

## Why Postmortems Matter

An incident is a failure of expectation: something did not work as planned. A postmortem is the structured investigation of what happened, why, and what to change so it doesn't happen the same way again. Done well, postmortems are the primary mechanism for building institutional knowledge. Done poorly—focused on blame—they teach silence and defensive behavior, making systems less reliable over time.

The core insight: **The human is not the root cause.** When an engineer makes a mistake, that mistake revealed a gap in the system (missing monitor, unclear runbook, inadequate testing, poor on-call handoff). The postmortem's job is to find and fix the gap, not to punish the person.

## Blameless Postmortem Structure

A postmortem document typically contains:

### Timeline

Reconstruct what happened, with timestamps where possible. Start from first alert/detection, trace through response actions, and end when service returned to normal. Include:
- When the issue started (often unknown; use "first detected" instead)
- Each decision and action taken
- When customer impact began/ended
- When the incident was declared resolved

The timeline is forensic evidence. It becomes the basis for follow-up questions: "Why did it take 20 minutes to escalate? Was the runbook unclear? Were the right people on-call?"

### Impact

Quantify the blast radius. Avoid vague language like "users experienced issues." Be specific:

- **Duration**: Incident started at 14:05 UTC, resolved at 14:47 UTC (42 minutes)
- **User impact**: 8% of customer base affected; ~15k users unable to complete transactions
- **Revenue impact**: Estimated $200k in lost transactions (if calculable; useful for prioritization)
- **Scope**: Affected US region only; API tier 1 and 2; read operations unaffected
- **Severity**: Page 1 incident, required VP escalation, 8 engineers on-call
- **Business criticality**: Feature was non-critical; user data was not compromised

Impact quantification drives priority and resource allocation for fixes.

### Root Cause(s)

Distinguish **the immediate cause** (what broke) from **contributing factors** (why the break mattered).

**Example incident**: Database connection pool exhaustion caused request timeouts.

Immediate cause: Deployment increased log volume, which flooded the database with logging I/O, exhausting the connection pool.

Contributing factors:
- Connection pool size was never tuned (assumption: default was sufficient)
- No alert on connection pool utilization (unable to detect the problem early)
- Logging configuration was not code-reviewed (change shipped untested)
- Database was shared across 3 services, so one service's logging burst affected others

Listing contributing factors clarifies the system weaknesses, not individual mistakes.

### Five Whys—And Its Limitations

The classic technique: Ask "why?" five times, descending from surface symptom to root cause.

```
Q1: Why did requests timeout?
A: Connection pool was exhausted.

Q2: Why was the connection pool exhausted?
A: Too many connections created because logging I/O was overwhelming.

Q3: Why was logging I/O overwhelming?
A: New log statement added in recent deployment.

Q4: Why wasn't this caught before deployment?
A: No log volume limits tested; no code review flagged the risk.

Q5: Why don't we test logging impact?
A: Logging is often treated as "always safe"; no standard practice for load-testing it.
```

**Limitations of Five Whys**:

- **Stops too early**: Reaching "human error" and stopping there misses systemic gaps (why didn't monitoring catch it? Why was the right person not on-call?).
- **Oversimplifies causality**: Often multiple independent causes are necessary for failure. One-thread narratives ("this person made a mistake") are seductive but false.
- **Depends on who's asking**: Different people ask different Whys, leading to inconsistent depth.

**Better approach**: Combine Five Whys with systemic thinking. Don't accept "someone didn't check" as a root cause; ask "what made it easy to skip that check? How can we make the right behavior the path of least resistance?"

## Systemic vs. Individual Causes

The blameless postmortem philosophy hinges on this distinction:

**Individual cause** ("John deployed bad code without testing") → drives blame, shame, silence.

**Systemic cause** ("No automated pre-deployment testing; code reviews don't require test verification; John had no template to follow") → drives improvement, shared ownership, learning.

Systemic thinking does not mean individuals have no agency. It means: given the constraints, incentives, and information available to them, what would a reasonable person have done? Then, fix the constraints.

Example: Engineer oncall at 3am saw a warning they didn't recognize, assumed it was a known false positive, and didn't page their team lead. Systemic question: Why was this warning's significance not immediately clear? (Fix: better alert naming, runbook linked to alert, clearer severity indicators.)

## Action Items

Postmortems without follow-through are theater. Every postmortem generates action items—changes to prevent recurrence or improve detection/response.

**Categorize by priority**:

- **Urgently fix** (within 1 week): High-impact, low-effort changes that directly prevent recurrence. Example: "Enable health checks on database connection pool; page if > 80% utilized."
- **Fix soon** (within 4 weeks): Medium-effort improvements that reduce risk or improve observability. Example: "Add load testing to CI pipeline for logging changes."
- **Backlog** (track but deprioritize): Valuable but lower-impact, higher-effort refactorings. Example: "Migrate services to separate database clusters to prevent cross-service impact."
- **Monitor** (no action, but document the learning): Observations without clear fixes. Example: "Noted that team's on-call rotation had coverage gap during morning shift transition; monitor for recurrence."

**Assign owners**, set deadlines, and track in the same system as bugs. Postmortem action items compete with feature work; visibility ensures they don't silently slip.

## Facilitation Techniques

A skilled facilitator makes or breaks the postmortem.

### Psychological Safety

- **No scorecard**: Do not use postmortems to evaluate performance or build termination cases.
- **Default to curiosity**: "Walk me through your thinking: what made sense to do in that moment?" vs. "Why did you not follow the runbook?"
- **Separate description from evaluation**: "Here's what happened" (factual). "Here's what we'll change" (improvement). No mixing.
- **Normalize mistakes**: "Everyone here has shipped bugs. This incident isn't unique; it's an opportunity to improve."

### Effective Questioning

- **Ask for context, not justification**: "What information did you have available at that time?" (not "How could you not know X?")
- **Decompose sequences**: "OK, so the alert fired, then what? What was your next action?" (building accurate timeline)
- **Seek specific details**: "How long between alert and escalation?" (measurable), not "Was there a delay?" (subjective)
- **Invite alternative causes**: "What else could have caused this?" (combats single-narrative bias)

### Common Pitfalls

- **Talking past the truth**: Pressure to find quick answers means shallow analysis. Insist on data and specifics.
- **Blaming the last person in the chain**: "If only Sarah had reviewed the config change, this wouldn't have happened." Ignore this; ask upstream: Why wasn't the change's impact obviously critical? Why wasn't there automated validation?
- **Accepting "human error" as a root cause**: It's never the root; it's the failure mode that revealed systemic gaps.

## Postmortem Culture

### Organizational Prerequisites

Blameless postmortems require leadership buy-in. If incidents are used to threaten engineers' employment, no one will participate honestly. Instead:

- **Publish postmortems openly** (internally, or with chosen customers). Transparency signals that learning, not blame prevention, is the goal.
- **Leadership attends postmortems**. If a director sits in weekly, engineers know the company values the discussion.
- **Act on findings**: If an action item is marked "urgent" but sits unprioritized for months, engineers learn postmortems are Theater. Follow through.
- **Track trends**: Monthly, review all postmortems. If the same system appears 5 times, fund a redesign. If one person appears frequently, it might indicate they caught anomalies;Details matter.

### Frequency & Timing

- **Major incidents**: Postmortem within 2–5 days, while details are fresh but emotions have cooled
- **Minor incidents**: Weekly roll-up (batch 5–10 small incidents, analyze patterns together)
- **Near-misses**: Postmortems on near-misses before they cause actual outages; lowest-cost learning
- **Recurring patterns**: If the same root cause appears 3x in 6 months, postmortem the postmortem—why didn't action items prevent recurrence?

### Action Item Follow-Through

Common failure: Action items are written, assigned, then ignored. Strategies:

- **Public status**: Every postmortem action has a status (not started, in progress, deployed, verified, closed). Weekly updates visible to all.
- **Accountability without blame**: "Service X's telemetry improvements assigned to team Y" is not accusation; it's coordination.
- **Verification**: Don't mark "add alert for X" as done until the alert has fired in staging and on-call has verified it's actionable.
- **Sunset old actions**: After 3 months, if an action hasn't been started, either staff it properly or archive it (noting why).

## Learning Organizations

A learning organization systematizes incident response into continuous improvement. Markers:

- **Postmortems are routine, not shameful**: Multiple per month are normal and expected (signals you're finding and fixing problems)
- **Blameless by default**: If someone is defensive in a postmortem, leadership signals that further blaming won't happen
- **Metrics tracked**: How many action items were completed? How many incidents were repeats of previous incidents? Closed rates trending down over year?
- **Patterns aggregated**: Quarterly analysis of incident themes (e.g., 30% were database-related; 40% involved human coordination failures; 20% were preventable with better monitoring)
- **Systemic changes made**: When a pattern emerges (e.g., 5 incidents in 3 months related to batch job failures), budget a project to redesign batch job orchestration
- **Culture reinforced**: New hires are onboarded to postmortem discussions early; they learn that mistakes are normal and improvement is continuous

The opposite: postmortems where no one speaks honestly, action items silently disappear, the same incident repeats yearly, and leadership blames engineers for "not being careful enough." These organizations are spending engineering effort fighting the last incident rather than preventing the next one.

## Connection to Broader Reliability

Postmortems are the feedback loop that **closes the control system** for reliability. Monitoring tells you something is broken (deviation from SLO). The incident response stops the bleeding (mitigation). The postmortem explains why the deviation occurred (root cause) and institutes a fix (improvement). Over time, this loop drives the system toward higher reliability. Without postmortems—just incident response with no analysis—systems stagnate, and engineers learn learned helplessness ("incidents happen; there's nothing we can do").