# RFC Process — Engineering Proposals, Review, Consensus & Decision Making

## What Is an RFC?

An **RFC (Request for Comments)** is a structured proposal process for decisions that affect multiple teams, have architectural implications, or carry uncertainty about the right approach. RFC stands for "Request for Comments," a term borrowed from internet standards (IETF RFCs), but in engineering orgs it's evolved into a general decision-making framework.

An RFC is **not**:
- A bug report or minor code review comment
- A short-term tactical decision made by one team
- A communication mechanism that replaces code review or technical documentation
- A bureaucratic gate that freezes decision-making

An RFC **is**:
- A written proposal surfacing uncertainty and inviting informed input before committing resources
- A record of reasoning (not just the decision, but the alternatives and tradeoffs considered)
- A tool for **distributed decision-making** in organizations where full synchronous alignment is expensive or impossible
- A forcing function that improves design through the discipline of explaining it in prose

See also: [process-technical-writing.md](process-technical-writing.md), [process-architecture-decisions.md](process-architecture-decisions.md)

## RFC Scope

Not every decision warrants an RFC. Companies typically use criteria like:

### Criteria for RFC

- **Cross-team impact**: Affects more than one team or requires shared infrastructure investment
- **Irreversible or expensive-to-reverse**: Architectural decisions, data model changes, API contracts, infrastructure investments
- **Technical uncertainty**: Multiple reasonable approaches exist; proposal clarifies tradeoffs and recommends one
- **Organizational risk**: Affects hiring, learning curve, operational burden, vendor dependencies

### When Not to RFC

- **Contained decisions**: A team chooses a library for internal use; no shared dependency
- **Emergencies**: Real-time incident response doesn't RFC; retrospectives do
- **Fully-delegated ownership**: "Team X decides how Team X implements Y" doesn't need org-wide RFC unless results cross team boundaries
- **Routine refinement**: Incremental improvements to existing systems; rfc'd decisions don't get re-rfc'd for every iteration

## RFC Template Structure

Effective RFCs follow consistent structure:

### Executive Summary (1 paragraph)

Concise statement of what is being proposed. Readers should understand the proposal and its urgency in < 30 seconds.

Example: "Propose shifting from REST to GraphQL for client-facing APIs over the next two quarters to reduce client implementation complexity and enable more efficient data fetching."

### Problem Statement

What is the pain point? Why is the status quo insufficient? What's the cost of inaction?

- Concrete examples: Link to recent incidents, customer complaints, performance data, or code examples showing the problem
- Avoid hyperbole: "This will destroy the company" is advocacy, not analysis
- Quantify when possible: N teams are blocked, or M requests per second are wasted, or we've spent Q engineering hours on workarounds

### Proposed Solution

Detailed description of what is being proposed. Sufficiently concrete that engineers could begin implementation.

- Architecture diagrams for system-level proposals
- Code examples for language or pattern proposals
- Timeline and milestones for phased approaches
- Operational model: How will this be maintained, monitored, and versioned?

### Alternatives Considered

List other approaches that were rejected, and briefly explain why. Do not straw-man competitors.

**Example**:
- **gRPC for API efficiency**: Would reduce bandwidth, but incompatible with browser clients; GraphQL chosen for broader client support
- **Incremental REST improvement (query parameters)**: Avoids a full platform change, but complexity still grows linearly with client needs; GraphQL handles growth better

### Tradeoffs and Concern Surface

Explicitly list **what is being given up** by this choice and what remains uncertain.

- **Performance implications**: GraphQL queries can be expensive; requires query cost analysis and rate limiting
- **Team learning curve**: GraphQL debugging tooling less mature than REST; team familiarity low
- **Operational complexity**: New failure modes (nested query bombs, N+1 query problems); requires new monitoring
- **Reversibility**: If GraphQL proves misaligned with our use cases, migration cost is high

### Impact Analysis

Who is affected and how? Quantify effort:

- **Implementation cost**: Estimated engineer-weeks to build, test, deploy
- **Operational cost**: Ongoing maintenance, infrastructure, training
- **Migration effort**: Will existing systems need updating? Phased or cutover?
- **Dependencies**: What other projects must land first?

### Decision Criteria

How will we know if this was the right choice? Explicit, measurable success criteria.

- Client development velocity increases by X%
- API payload sizes decrease by Y%
- Time-to-implement new client features drops from A to B weeks
- No degradation in API query latency over REST (measured at P95, P99)

### Rollout Plan

If approved, how does this land in production?

- **Pilot**: Start with one client team or internal use case; measure and learn
- **Ramp**: Gradually increase proportion of traffic or teams
- **Monitoring**: What metrics are tracked during rollout? Kill switch if P99 latency increases beyond X%?
- **Communication**: How do other teams learn about this and adapt?

## RFC Process Workflow

### Submission

- **Write in the RFC location**: GitHub (monorepo `/rfcs/` folder), Wiki, or dedicated system (GitBook, Slite, AFS)
- **Propose as Pull Request or Draft**: Not finalized yet; explicitly invite feedback
- **CC relevant stakeholders**: Tag team leads, architects, people who will build or operate this
- **Set a deadline**: "Feedback due by [date], decision by [date]" — somewhere between 1-3 weeks depending on complexity

### Review Phase

- **Asynchronous by default**: People respond on their own schedule, not in meetings
- **Structured feedback formats**: Some teams use comment templates ("Question:", "Concern:", "Consideration:") to encourage specificity
- **Owner responsibility**: RFC author responds to substantive concerns; don't let feedback go unaddressed
- **Escalation mechanism**: If consensus isn't emerging, escalate to a decision-maker (tech lead, architect); don't extend review indefinitely
- **Living document**: Update the RFC based on feedback; final version reflects the conversation

### Decision

One outcome of four:

1. **Approved**: Execute as proposed. Move to implementation planning
2. **Approved with modifications**: Specific concerns addressed in revised proposal; approved conditionally
3. **Request more information**: Explicitly list what's needed (e.g., "run performance test on 10K concurrent GraphQL clients, then re-propose")
4. **Rejected**: Explain decision; leave historical record for future reference

**Ownership**: A senior engineer, architect, or tech lead decides. Not consensus voting; not endless discussion. Clear decision-maker prevents bike-shedding.

### Implementation & Retrospective

- **Tracking**: RFC number stays associated with the work (GitHub project links RFC #42, issues reference RFC #42)
- **Retrospective** (6-12 months post-launch): Did we achieve the success criteria? What surprised us operationally? Should we adjust course?
- **Learning captured**: Update knowledge base or team practices to reflect what was learned

## RFC vs. ADR (Architecture Decision Records)

| **RFC** | **ADR** |
|---|---|
| Proposal form; written *before* decision | Decision form; written *after* decision |
| Invites structured critique and alternatives | Captures what was decided and why (context, rationale, consequences) |
| Used when significant uncertainty or coordination needed | Used to document all decisions, including routine ones |
| Org-wide or cross-team scope | Team or component scope (though can be org-wide) |
| Audience: decision-makers and affected engineers | Audience: future readers of the codebase (next decade) |

**In practice**: An RFC may become an ADR. Write RFC to propose; once approved, extract the decision and rationale into ADR format for the codebase.

See also: [process-technical-writing.md](process-technical-writing.md)

## Anti-Patterns and Correctives

### Pattern: RFC as Rubber Stamp

**Problem**: RFCs are written, circulated, and approved with minimal feedback. Decision was already made; RFC is theater.

**Corrective**: 
- Require explicit feedback from at least 2-3 reviewers or decision-maker before approval
- Include dissenting views in final RFC (e.g., "Team X preferred Option 2 because...") — disagreement doesn't block, but it's transparent
- Track approval latency; if RFCs approve instantly, something is off

### Pattern: Endless RFC Cycles

**Problem**: RFC gets feedback, author revises, new feedback, new revision... proposal never lands.

**Corrective**:
- Set hard deadlines. "Feedback due Friday. Decision Monday. If consensus isn't clear, decision-maker chooses based on best available info"
- Escalate, don't elaborate. Once decision-maker is involved, defer to them for judgment; don't re-litigate points

### Pattern: RFC for Routine Decisions

**Problem**: Process becomes friction. Teams RFC every library upgrade, minor refactor, or team-contained choice.

**Corrective**:
- Explicit RFC criteria in the handbook (cross-team? irreversible? significant uncertainty?)
- Empower teams to decide locally when criteria aren't met
- Spot-check: Ask teams quarterly "Have you written any RFCs you now regret not writing?" Typically reveals the threshold

### Pattern: No Follow-Through on Retrospectives

**Problem**: RFC approved, shipped, then forgotten. Team never validates success criteria; organization doesn't learn.

**Corrective**:
- Schedule retrospectives in the RFC (e.g., "Review success criteria July 1"). Treat as commit
- Share findings with org. Quick 30-min retrospective talk, or written summary. Feeds back into future RFC reasoning

## Scaling RFC Practices

- **Small teams (< 50 engineers)**: One centralized RFC process. Monthly sync to review recent RFCs and decisions
- **Medium orgs (50-500)**: Central RFCs for org-wide decisions; team-level RFCs for domain decisions. Quarterly cross-team syncs to detect duplicated problem-solving
- **Large orgs (500+)**: Federated RFCs by domain (backend, frontend, infrastructure, data). Annual meta-RFC: "What have we learned about our RFC process itself?"