# Incident Tooling — incident.io, PagerDuty, Opsgenie, Rootly & On-Call Automation

## The Incident Lifecycle and Tooling

Modern incident management spans multiple phases: **detection** (alerts fire), **declaration** (incident created), **investigation** (what's broken?), **mitigation** (how do we fix it?), **resolution** (it's fixed), **postmortem** (what did we learn?).

Incident tooling coordinates these phases across on-call engineers, responders, stakeholders, and status page communications.

## On-Call Scheduling & Escalation

### Rotation Models

**Primary/Secondary Split**:
- Primary on-call is first point of contact. If no response within 5–15 minutes, escalation fires.
- Secondary backs up primary. Joins if primary is overwhelmed or unavailable.
- Tertiary (optional) for major incidents or when both are engaged.

**Shift Duration**:
- 1 week (most common): Predictable, low context switching, burnout risk if incidents cluster.
- 2 weeks: Deeper context, but longer stretches of on-call.
- 24-hour or 12-hour: High frequency rotation, suitable for geographically distributed teams (follow-the-sun).

**Override Workflows**:
- On-call outside your shift? An override allows ad-hoc schedule changes.
- Common triggers: sick leave, conference attendance, oncall fatigue.

### Escalation Policies

Escalation policy defines the chain:

```
Layer 1: Primary on-call engineer (5 min timeout)
    ↓ (if no ack)
Layer 2: Secondary on-call engineer (5 min timeout)
    ↓ (if no ack)
Layer 3: Team lead (10 min timeout)
    ↓ (if no ack)
Layer 4: VP Engineering (notify only)
```

**Escalation Timeout**: How long before escalating to next layer (5–15 minutes typical). Too short = false escalations. Too long = incident sits unacknowledged.

**Acknowledgement Required**: Engineer must explicitly acknowledge alert via Slack/SMS/app to reset timeout.

### PagerDuty Integration Example

```yaml
# Define escalation policy in PagerDuty
Policy: Platform On-Call
Schedules:
  - Primary: schedule_platform_primary (1-week rotations)
  - Secondary: schedule_platform_secondary (offset by 3.5 days)

Escalation Rules:
  1. Primary on-call: 5 min timeout
  2. Secondary on-call: 5 min timeout
  3. Platform lead: 10 min timeout

Services attached: API, Database, Cache, Message Queue
```

## Alert Routing & Deduplication

### Alert Aggregation

Multiple alerts can fire for the same incident (CPU spike → memory pressure → OOMKill). Tooling should **group related alerts**:

```
Alert Group: Database Performance Degradation
├── Alert 1: db-cpu > 90% (Prometheus)
├── Alert 2: db-slow-queries > 1000 (New Relic)
└── Alert 3: db-replication-lag > 10s (Custom)

→ Creates single incident, not three separate ones
```

**Grouping rules** (by service, by criticality, by symptom):

- Group by service tag: All API alerts → one incident per service
- Deduplicate identical alerts within a time window
- Cross-correlate alerts from different backends (a database lag alert might correlate with an application error alert)

### Incident Severity Levels

Define severity based on impact:

- **SEV-1 (Critical)**: Complete outage, customer-facing impact, revenue loss. Page exec immediately.
- **SEV-2 (High)**: Significant degradation, most users impacted. Page entire team.
- **SEV-3 (Medium)**: Impact limited to subset of users or non-critical features. Page on-call.
- **SEV-4 (Low)**: Internal tooling, no customer impact. Create ticket, no page.

**Auto-assignment** of severity based on alert labels (database outage = SEV-1, logging lag = SEV-3).

## Incident Declaration & War Rooms

### War Room

When major incident is declared, a war room is opened:

- **Dedicated channel** (Slack, Teams, Discord) for synchronous comms
- **Zoom call** (or equivalent) for voice sync when necessary
- **Shared incident document** with timeline, decisions, user messaging
- **Clear roles**: Incident Commander (IC), Subject Matter Experts (SMEs), Customer Comms Lead

**IC responsibilities**:
- Sets priority and severity
- Delegates investigation tasks
- Coordinates escalation to other teams
- Decides go/no-go for mitigations
- Communicates status regularly (updates every 15 min)

### Incident Creation

Typical flow:

```
Alert fires → Alert routing service → Incident created
                                      ├─ Severity determined (auto or manual)
                                      ├─ Team assigned (by service tag)
                                      ├─ Escalation policy triggered
                                      ├─ War room channel auto-created
                                      └─ On-call engineer paged
```

## Incident.io

incident.io is a modern incident management platform (acquisition by Grafana, but standalone).

### Key Features

**Incident Creation & Fields**:
- Severity (1–5)
- Impact (count affected users, services, revenue)
- Status (investigating, mitigating, resolved, postmortem)
- Assigned team, IC role
- Custom fields (product area, feature flag, database shard)

**Workflow Automation**:
```yaml
When: Incident reaches SEV-2
Then:
  - Create Slack channel
  - Schedule postmortem for 24h after resolution
  - Page VP Engineering
  - Create Jira ticket
  - Update status page
```

**Postmortem Automation**:
- Auto-draft postmortem from incident chat transcript
- Prompt for root causes, action items
- Assign action items to team members
- Track follow-up on future incidents

**Integrations**:
- Slack (create, update, status)
- PagerDuty (incident syncing)
- GitHub/GitLab (auto-link commits)
- Datadog, New Relic (context enrichment)

### Example: incident.io Workflow

```
1. Alert fires on Prometheus
2. Grafana alert → incident.io webhook (creates incident)
3. incident.io posts to #incidents-war-room Slack channel
4. Links to Grafana dashboard, runbook, related changes
5. IC joins Slack channel, types "!incident sev-1"
6. incident.io escalates to executives, creates zoom link
7. Team investigates, posts findings in war room
8. Incident resolved, IC types "!incident resolved"
9. incident.io schedules postmortem in 24h, drafts notes
10. Postmortem: team reviews causes, creates follow-up issues
```

## PagerDuty

PagerDuty is the incumbent on-call and incident management platform.

### Oncall Scheduling

```yaml
Team: Platform
Schedules:
  - Primary:
      Shift 1 (Mon–Fri): alice, bob, charlie (1 week each, rotating)
      Shift 2 (Sat–Sun): rotating from standby pool
  
  - Secondary:
      Same as primary, offset by 3.5 days
```

Users can:
- Override their schedule (away, sick, conference)
- Swap shifts with teammates
- Take on-call swaps (casual, not guaranteed)
- View upcoming rotations

### Incident Management

```yaml
Incident:
  Title: "Database replication lag > 5m"
  Service: database-service
  Urgency: High (auto-escalation if unacknowledged)
  Escalation Policy: database-team-primary → database-team-secondary → vp-eng
  
  Timeline:
    - 15:23: Incident triggered
    - 15:24: alice@pagerduty paged
    - 15:28: alice acknowledged
    - 15:45: escalated to bob (alice didn't resolve)
    - 16:02: bob mitigated (restarted replica)
    - 16:05: Incident resolved
```

### Integrations

| System | Integration | Benefit |
|--------|-------------|---------|
| Prometheus/AlertManager | Webhook | Alerts become PagerDuty incidents |
| Slack | Add-on | Manage PagerDuty from Slack (/pd, @pagerduty) |
| GitHub | BiDirectional | Link commits to incidents, auto-resolve when PR merges |
| Jira | BiDirectional | Sync incident to ticket, track fixes |
| Custom Webhook | Receiver | Any monitoring system can trigger PagerDuty |

## Opsgenie (Atlassian)

Opsgenie is Atlassian's on-call and alerting platform (alternative to PagerDuty). Similar functionality, Jira-native integrations.

### Key Differences from PagerDuty

- **Cheaper** (especially at large scale)
- **Jira integration** (native, since both Atlassian)
- **Simpler UI** (less feature-rich, but easier to learn)
- **Escalation less granular** (strengths: fast paging, weaknesses: less workflow control)

### Opsgenie Configuration

```yaml
On-Call Policy: platform-primary
  - Escalation 1: alice (5 min)
  - Escalation 2: bob (5 min)
  - Escalation 3: #platform-team Slack channel (notify only)

Recipients:
  - phone, SMS (alice), SMS (bob)

Alert Rules:
  - If from: datadog + tag: critical → auto-assign to platform-primary policy
  - If from: datadog + tag: warning → create task (no page)
```

## Rootly

Rootly is a newer, specialized incident automation platform focused on **incident postmortems and learning**.

### Focus

Unlike PagerDuty/Opsgenie (paging and routing), Rootly emphasizes **post-incident learning**:

- **Postmortem Automation**: Auto-draft notes from incident chat, timeline, metrics.
- **Trends Analysis**: Track root cause patterns, repeat issues, team skills gaps.
- **Action Item Tracking**: Assign follow-ups, track closure, prevent ticket debt.
- **Runbook Linking**: During incident, auto-surface runbook based on alert type.

### Integration Example

```yaml
Slack integration:
  When someone types "!rootly start incident"
  Rootly:
    - Creates incident
    - Starts recording Slack thread
    - Links to related runbooks
    - Surfaces recent similar incidents ("We had a similar DB replication lag last week")
    
  After resolution:
    - Records postmortem meeting transcription
    - Drafts incident report with timeline and metrics
    - Recommends action items based on root cause
    - Tracks action items through to closure
```

## Status Pages

Status pages communicate incidents to external stakeholders (customers, partners, public).

### Platforms

- **Atlassian StatusPage** (formerly Statuspage.io): Dedicated, integrates with Jira/Opsgenie.
- **incident.io Status**: Built-in status page in incident.io.
- **Custom** (Vercel, Hashicorp): Self-hosted React dashboards.

### Components & Updates

```
Component Status:
├─ API Service: Operational
├─ Database: Degraded Performance (investigating)
│  └─ Updates posted 15:23, 15:35, 16:05
└─ Cache: Operational

Incident Timeline:
2025-03-20 15:23 UTC - Replication lag detected, investigating
2025-03-20 15:35 UTC - Root cause identified (network saturation)
2025-03-20 16:05 UTC - Mitigation applied, traffic rerouted
```

**Auto-updates**: Incident tooling can auto-create status page updates when incident status changes (investigating → mitigating → resolved).

**Manual writes**: Customer Comms Lead writes human-readable updates (technical jargon → plain English).

## Runbooks

Runbooks are standalone documents or in-tool guides for responding to specific incidents.

### Runbook Structure

```markdown
# Database Replication Lag > 5 Minutes

## Detection
Alert: db-replication-lag-high (from Prometheus)

## Symptoms
- Read replicas report "replica lag" > 5m
- Stale reads on secondary DC (user data 5m behind)

## Root Causes
1. Replica can't keep up with primary write volume
2. Network saturation between primary and replica
3. Replica is under-resourced (CPU spikes during catch-up)

## Triage
1. SSH to replica: `aws ssm start-session --target db-replica-01`
2. Check replica status: `select host, seconds_behind_master from replication_status;`
3. Check primary write rate: `show status like 'Questions';`

## Mitigation Steps
1. **Immediate**: Scale up replica (add vCPU, increase memory)
2. **Intermediate**: Reduce write load on primary (disable non-critical batch jobs)
3. **Long-term**: Consider read-scaling architecture (sharding, federation)

## Escalation
If lag doesn't decrease within 15 min, escalate to Database team lead (include metrics from Datadog)
```

**Integration**: Most incident tools (incident.io, Rootly) auto-link to runbooks based on incident type or alert metadata.

## Automation & Integration Points

### Alert → Incident Flow

```yaml
Prometheus Alert
  ↓
  AlertManager/webhook
    ↓
    [Event Router: correlate by service]
    ↓
    PagerDuty/incident.io
      ├─ Assign severity
      ├─ Create war room
      └─ Page on-call (based on escalation policy)
    ↓
    Slack notification
      ├─ Links to dashboard
      ├─ Links to runbook
      └─ Links to incident page
    ↓
    On-call engineer joins, starts investigation
```

### Incident → Postmortem Flow

```yaml
Incident marked as resolved
  ↓
  incident.io/Rootly
    ├─ Captures incident timeline (duration, impact metrics)
    ├─ Extracts chat transcript
    ├─ Surfaces related changes/deployments (from Git)
    └─ Schedules postmortem (24h later)
  ↓
  Postmortem meeting
    ├─ Auto-transcribed (speech-to-text)
    ├─ Root cause analysis
    └─ Action items assigned
  ↓
  Jira/GitHub tickets auto-created
    ├─ Track action item progress
    └─ Link to incident for context
```

## Related Topics

See also: [sre-on-call.md](sre-on-call.md), [sre-incident-management.md](sre-incident-management.md), [observability-alerting.md](observability-alerting.md), [process-incident-communication.md](process-incident-communication.md), [sre-postmortems.md](sre-postmortems.md).