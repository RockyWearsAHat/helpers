# Architecture Decision Records — ADRs, Decision Logs & Governance

## What Problem ADRs Solve

Every significant architecture decision is forgotten within months:

- Why did we choose PostgreSQL instead of MongoDB? *"Someone said it during a sprint planning"*
- Why can't we migrate to gRPC for internal services? *"Security concerns?" "Performance?" "Someone had a bad experience?"*
- Why do we have both S3 and DynamoDB when we could just use a database? *"Dunno, been there forever"*

Without recorded decisions, teams:
- Re-litigate settled questions every 12-18 months
- Cannot maintain coherent architectural vision across leadership changes
- Spend retros debating history instead of planning futures
- New engineers can't separate "we tried this and it failed" from "nobody ever tried this"

**ADRs fix this by creating organizational memory in a machine-readable, time-stamped, accessible format.**

---

## The ADR Format

No universal standard exists. The most common frameworks:

### MADR (Markdown Any Decision Record)

The MADR format captures context and choices clearly:

```markdown
# [n] — [Brief Title]

**Date:** YYYY-MM-DD  
**Status:** Proposed | Accepted | Deprecated | Superseded By ADR-[n]  
**Decided By:** Team, person(s) responsible  

## Context

### Problem Statement
What constraint or question triggered this decision?

### Relevant Context
- Current tech stack, business constraints, team skills
- Prior attempts or related decisions
- Time pressure, cost pressure, scaling concerns

## Options Considered

### Option 1: [Name]
Pros:
- 
Cons:
- 
Effort: [Days of work]
Risk: [Low/Medium/High]

### Option 2: [Name]
[Similar structure]

## Decision

We chose **Option 1** because:
- [Key pro that won]
- [Key pro that won]

Acceptance criteria (how we'll know this was right):
- [Measurable outcome]
- [Measurable outcome]

## Consequences

### Positive
- Can now build feature X faster
- Reduced operational burden via [tool]

### Negative
- Locked into vendor X for [duration]
- Learning curve delay of 3 weeks for team

### Risks to Monitor
- If [condition], we should revisit this decision
- Monitor [metric] monthly; if [threshold], escalate

## Implementation Notes

- Ticket: PROJ-1234
- Rollout: [Timeline]
- Rollback plan: [If applicable]
```

### Lightweight ADRs

For smaller decisions, a shorter template suffices:

```markdown
# [n] — [Title]

Status: [Accepted/Proposed]  
Date: YYYY-MM-DD  

**Problem:** [1-2 sentences]  
**Decision:** [1-2 sentences]  
**Because:** [Why this won vs. others]  
**Risk:** [If wrong, what breaks]  
```

---

## Decision Log Management

### Where to Store ADRs

- **Preferred:** `docs/architecture/decisions/` in the repo (version-controlled, searchable, reviewable in PRs)
- **Alternative:** Wiki, Notion, or Confluence (acceptable if searchable and auto-backupped)
- **Mistake:** Email threads, Slack, or private docs (lost when people leave, unsearchable)

### Versioning and Numbering

- Start at 0001 or 0000; increment sequentially
- Use 4 digits: ADR-0042, not ADR-42 (sorting works alphabetically)
- **Never reuse numbers** (even if an ADR is deprecated)
- Store in a single directory, not scattered across the codebase

### Status States

```
Proposed   → A decision is being considered
   ↓
Accepted   → Decision committed; implementation started
   ↓
Deprecated → Decision still applies, but no longer recommended for new systems
   ↓
Superseded → Replaced by another ADR (link to it); old decision no longer applies
```

A decision can also move:
- **Accepted → Rejected** (during team review, before implementation)
- **Proposed → Withdrawn** (proposer realized it's not ready)

### Review and Approval

**Not all ADRs need sign-off**, but high-impact ones do:

- **Structural (new service, data store, framework):** Architecture lead + 2+ experienced engineers
- **Security (auth system, data encryption, API design):** Security team + architect
- **Operational (deployment strategy, monitoring, backup):** SRE/platform team
- **Low-impact (local caching strategy, error handling pattern):** Proposed by one, reviewed asynchronously by relevant people

Use PR reviews in your repo for this—it creates a paper trail and ensures visibility.

---

## When to Write ADRs

**Write an ADR if:**
- The decision will affect code written over multiple sprints
- Multiple reasonable options exist with significant tradeoffs
- The decision is either high-risk or constrains future changes
- The team has disagreed on this before (or will at retro)
- New team members will ask "why did we do this?" within a year

**Don't write an ADR if:**
- It's a local choice (variable naming, specific function structure)
- It expires soon (experiment for 2 weeks, then decide)
- It's fully documented elsewhere (e.g., a database migration guide)
- The decision was forced by a constraint with zero tradeoff ("we use AWS because half the team is AWS-certified")

---

## Superseding Decisions

When circumstances change and an old decision becomes wrong:

1. **Create a new ADR** that explicitly states it supersedes the old one
2. **Link both directions** — new ADR links to old; old ADR's status becomes "Superseded By ADR-0099"
3. **Explain why the world changed** — old decision wasn't wrong at the time; it's just outdated now
4. **Don't delete the old ADR** — future team members need to see why we thought PostgreSQL was insufficient before we added Elasticsearch

Example status update:
```
Status: Superseded By ADR-0099 (2023-08-15)
Reason: Scaling to 100K requests/sec required CQRS; PostgreSQL replication became bottleneck
```

---

## Team Buy-In and Communication

ADRs fail when:
- Only architects write them; engineers ignore them
- Decisions are finalized before team input
- ADRs live in a place engineers don't know about

**Make ADRs part of your culture:**

1. **Announce proposals early** (Slack, all-hands) — let debate happen before formal ADR
2. **Encourage counter-proposals** — "Option A vs Option B" decisions show more nuance than a single option
3. **Link ADRs to decisions in retros** — when a painful architectural choice gets discussed, say "That's ADR-0015; let's revisit it"
4. **Celebrate decisions that were proven right** — retro: "Remember ADR-0007? We thought this might scale to 1M users, and it did."

---

## Architectural Fitness Functions

An architectural decision should come with **testable assumptions** about why it was right:

```
Decision: Use DynamoDB for session storage
Fitness Assumptions:
  - Read latency <10ms at p99 (check: APM every day)
  - Cost <$2000/month (check: AWS billing monthly)
  - Can scale to 1M concurrent sessions (check: load test quarterly)

If any assumption fails for 30 days, escalate to architecture review
```

Tools and practices:
- **Metrics dashboards:** Link ADRs to dashboards showing their key metrics
- **Automated alerts:** If a fitness function degrades, alert the team
- **Scheduled reviews:** Every 12 months, check assumptions against reality

This prevents architectural decisions from becoming zombie decisions—choices nobody remembers, but also nobody challenges.

---

## Tools and Automation

- **adr-tools** (GitHub) — shell scripts to create ADR template, list them, link to related ADRs. Run `adr new "Title"` → generates ADR-0000.md
- **ADR GitHub Action** — Auto-generates an index of all ADRs in your repo
- **Log-based** — `git log docs/architecture/decisions/ | grep "ADR"` to find recent decisions
- **Search** — `grep -r "Superseded By" docs/architecture/decisions/` to find deprecated patterns

None of these are required. A text editor and a `decisions/` folder suffice.

---

## See Also

- [Technical Writing — Documentation, ADRs, RFCs & Design Documents](process-technical-writing.md) — ADRs as documentation
- [Architecture Patterns](architecture-patterns.md) — Common patterns referenced in ADRs
- [Code Review Practices](code-review-practices.md) — How ADRs relate to review discipline