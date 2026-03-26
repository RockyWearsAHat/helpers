# Incident Communication — Status Pages, Severity Levels & Post-Incident Learning

## The Purpose

Incident communication serves three distinct audiences, often **simultaneously**:

1. **Internal responders** (engineers, on-call, management) — need real-time operational facts to coordinate mitigation
2. **Customers/stakeholders** — need **status updates** on impact, ETA, and advice (do I retry? will my job lose data?)
3. **Public** — perception of reliability, transparency, and competence accumulates over time

These audiences have fundamentally different information needs. Conflating them leads to customer-facing comms that either bore operations teams or confuse customers with jargon.

---

## Severity Levels and Escalation

### The Problem with Vague Severity

Most teams inherit severity definitions that are imprecise ("P1 = important," "P2 = less important"). This creates friction:

- On-call engineers guess whether a problem is P2 or P1, delaying escalation
- Incident leads delay declaring severity, so comms don't start until damage is done
- Retros argue about what the severity "really was"

**Good severity frameworks tie impact to automation thresholds:**

```
SEV-1 (Critical):
  ├─ Automation threshold: Auto-page on-call engineer + manager
  ├─ Impact: Complete loss of service for paying customers
  ├─ Scope: Affects >50% of traffic or core business flow (payments, login, etc.)
  ├─ Customer comms: Initiate within 5 minutes
  └─ Update frequency: Every 15 minutes

SEV-2 (Major):
  ├─ Automation threshold: Auto-page on-call engineer (not manager)
  ├─ Impact: Partial loss of service; customers experience degradation
  ├─ Scope: Affects 10-50% of traffic or non-critical flows (analytics, recommendations)
  ├─ Customer comms: Initiate at responder discretion; update hourly
  └─ Other: Acceptable to mitigate rather than resolve immediately

SEV-3 (Minor):
  ├─ Automation threshold: Create ticket, don't page
  ├─ Impact: Non-user-facing issue, poor UX, or isolated customer impact
  ├─ Scope: Affects <10% of traffic or single customer
  ├─ Customer comms: Post-incident notification if external impact confirmed
  └─ Other: Not typically declared as "incident," handled as troubleshooting
```

The key: **Severity drives automation and response time**, not the other way around. Define impact thresholds first, then assign severity.

### Escalation Procedures

Clear escalation rules prevent both under-response and panic:

- **Initial assessment** (first 10 min): On-call engineer makes severity call
- **Escalation triggers**: If SEV-2+ lasts >15 minutes without clear mitigation path, escalate to incident commander + manager
- **Decision authority**: Define who can declare SEV-1 (usually on-call + manager; EMTs can also declare if undisputed)
- **Threshold reset**: If an incident was declared SEV-1 but stabilizes as SEV-2 after 1 hour, formally downgrade (this reduces alert fatigue)

Bad escalation is either:
- **Too slow** — responders page up when they should have immediately → confidence erodes
- **Too loose** — every alert pages the manager → alert fatigue → manager stops checking Slack

---

## Customer Communication Templates

### Status Page Levels

Most status page services (Statuspage.io, Instatus, Atlassian Status Page) define:

- **Operational** — No impact; system working normally
- **Degraded Performance** — Service available, slower than usual; users can work but frustrated
- **Partial Outage** — Some components affected; some users unable to work
- **Major Outage** — Core service unavailable; customers cannot use product

**The template structure** (each update):

```
# [HH:MM UTC] — [Status: Investigating | Identified | Monitoring | Resolved]

**Current Status:** [Plain English: what's broken for customers]

**Impact:** [Affected regions, features, percentage of users]

**What We're Doing:** [Current actions: rebooting service X, rolling back change Y, etc.]

**Next Update:** [Specific time or "every 15 minutes"]

---
[Previous updates in chronological order, oldest last]
```

**Key principles:**
- Use **specific language**, not jargon. "Database replication lag" is jargon; "some API requests slower than normal" is specific.
- Acknowledge **uncertainty explicitly**. "We're investigating whether payments are affected" is better than silence.
- **Update frequency must be predictable** (even if just to say "no new information, still investigating")
- **Avoid shame and blame.** "Full service restored after accidental config deletion" → "Full service restored"

### Internal Incident Comms (Slack, incident.io)

Use a dedicated thread or incident channel. Key information:

```
🚨 [INCIDENT] [SEV-1] 10:23 UTC — Payment API errors
├─ Impact: ~5,000 failed transactions (0.2% of daily volume)
├─ Affected Services: payments-api, payout processing
├─ Incident Lead: @alice (page @bob if no response in 10 min)
├─ Status Page Updated: yes
├─ Customer Comms Started: no (internal-only incident; customers retrying succeeds)
└─ Diagnosis: [Detailed, real-time, updates below this]
```

Then thread updates every 5-10 minutes, pulling real-time metrics, log errors, and actions taken. This becomes the **incident timeline** for retro.

---

## Post-Incident Review and Timeline Construction

### The Blameless Model

A "blameless" postmortem isn't about absolving individuals—it's about **studying organizational failure**, not personal failure.

**The premise:** If one engineer caused an outage, that engineer is not the root cause. The root cause is the system that allowed one person's mistake to cause a company-wide outage.

**In practice:**

- **Separate judgment from investigation.** Never use the word "blamed" or "should have known." Focus on: "What conditions made this mistake reasonable at the time?"
- **Ask 'why' 5 times** (Toyota Andon / 5-Why method), but stop when you reach a **systemic friction point** (lack of automation, unclear runbook, understaffed on-call), not a person's competence
- **Identify failure chains**, not single failures. "Engineer forgot to enable replication" is boring. "Replication wasn't enabled, monitoring didn't alert, dashboards didn't flag it, and the runbook was stale" is actionable.

### Timeline Construction

The incident thread in Slack becomes the primary timeline. Retro facilitators then:

1. **Verify timestamps** against logs (Slack timestamps are often estimates)
2. **Reconstruct decision points:** When did the on-call engineer realize scope? When did they escalate? Was communication clear?
3. **Note information asymmetries:** What did customer support not know? What did on-call not see?
4. **Document delays explicitly:** "Detection took 8 minutes because alert didn't fire" is a finding worth capturing.

---

## Cross-Team Responsibilities

- **On-call engineer:** Real-time diagnosis, mitigation decisions, incident thread updates
- **Incident commander** (management): Escalation decisions, customer comms approval, resource coordination
- **Customer support:** Monitor customer reports, relay sentiment to incident lead, track public social media
- **Status page owner:** Update customer-facing status every 15 minutes
- **Retro facilitator:** Collects timeline, schedules async + sync postmortem, drives action items

---

## See Also

- [SRE Incident Management](sre-incident-management.md) — Response, detection, recovery lifecycle
- [SRE On-Call](sre-on-call.md) — On-call training and burnout prevention
- [SRE Postmortems](sre-postmortems.md) — Learning from incidents
- [Security Incident Response](security-incident-response.md) — Response procedures for security incidents