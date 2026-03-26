# Remote Engineering — Async Communication, Documentation & Organizational Design

## The Fundamental Shift: Writing-First Culture

Remote work is fundamentally different from co-located work because information transfer changes. In an office, context flows through ambient awareness: overhearing conversations, hallway discussions, reading someone's screen, observing what's being discussed. Remote work eliminates this ambient context layer, forcing explicit communication.

The natural adaptation is a writing-first culture where important information is documented asynchronously. This isn't a stylistic choice—it's a requirement. Without written records, information lives in Slack messages, slides shown in meetings, or people's heads. New team members inherit a void. Decisions aren't repeatable.

Writing-first doesn't mean eliminating synchronous communication. It means synchronous communication is for alignment and debate, not information transfer. Before a meeting, pre-read a document. During the meeting, discuss. After the meeting, document the decision. The document becomes the source of truth.

## Documentation-Driven Development

Documentation-driven development treats docs as a primary artifact alongside code.

**The practice:**
1. Before implementing a feature, write it up: problem statement, proposed solution, trade-offs considered, changes to APIs or data models
2. Circulate for feedback asynchronously (24-48 hour review window)
3. Incorporate feedback; update the doc
4. Implement the code to match the doc
5. Update the doc if implementation reveals new thinking
6. The doc becomes the maintenance guide

**Why it matters in remote work:** Code is discovered, not communicated. If a new engineer encounters a system, they read the code first. If code is written to an undocumented design, they reverse-engineer the design by reading commits and guessing at intentions. A written design doc gives them context upfront.

**When it's overuse:** Documenting every bug fix or minor code change creates overhead; reserve documentation-driven development for features, architectural changes, and cross-team decisions.

## Async Communication Patterns

### Asynchronous vs. Synchronous Distinctions

**Good candidates for async:**
- Information transfer (sharing context, announcing decisions)
- Status updates
- Feedback on documents or proposals
- Design discussions (everyone comments over time)
- Questions that need thought (give people time to craft answers)

**Need synchronous:**
- Crisis response (production incident)
- Rapid back-and-forths where latency kills flow (pair debugging)
- High-bandwidth communication (difficult feedback conversations, negotiations)
- Group decisions under time pressure
- Consensus-building when async stalling has occurred

### "Decide and announce" vs. "discuss then decide"

When async communication is slow or contentious, teams sometimes abandon it entirely, jumping to synchronous meetings. A better pattern:

1. **Propose asynchronously** with reasoning. Give 24-48 hours for feedback.
2. **Publish decision asynchronously.** Explain why you weighed feedback as you did. If major objections emerge, escalate to sync discussion.
3. **Move, don't debate endlessly.** Waiting forever for consensus kills velocity. Set a decision deadline.

Async cultures run on trust: a good-faith decision-maker with transparent reasoning, even if others disagree, is more sustainable than consensus-seeking that stalls.

## Timezone Overlap and Calendar Design

Remote teams spanning multiple timezones face a real constraint: not everyone can attend every meeting.

**Common anti-patterns:**
- Mandatory all-hands meetings at times that exclude half the team (requires people to wake up at 6 AM or stay until 10 PM)
- No timezone awareness; meetings scheduled at whatever time is convenient for the organizer
- All-sync culture; anything not discussed in real-time is not valued

**Sustainable patterns:**
- **Core overlap window:** Teams working across 4-5 timezones may define a 2-hour core overlap where synchronous collaboration happens. Outside this window, async.
- **Rotating meeting times:** All-hands meetings at multiple times, or recorded so people in other zones can review
- **Async-first culture:** Synchronous meetings are the exception, not the default. Anything discussed in a meeting is documented afterward.
- **Explicit expectations:** "This team operates on timezone UTC+0 coverage" or "Core hours 10am-3pm Pacific, but no meetings required outside core overlap"

**Calendar systems that help:**
- Shared calendars showing timezone-aware working hours
- Tools that suggest meeting times given geography (World Time Buddy, Calendly's timezone handling)
- Policy around email response time vs. Slack (async Slack may take 12-24h; email is for async-by-design)

## Communication Tools for Remote Teams

### Synchronous (Real-time)

**Video call tools** (Zoom, Google Meet, Slack Huddles): High information bandwidth; good for discussions, pairing, crisis response. Fatigue when overused. Screen sharing enables pair programming or technical discussions.

**Instant messaging** (Slack, Teams, Discord): Fast ephemeral communication. Works for brief Q&A, not for decisions. Archived but unsearchable for future context. Time zones create latency (everyone online at different times).

### Asynchronous (Leave a Record)

**Shared documents** (Google Docs, Notion, Confluence): Best for design docs, proposals, status reports. Asynchronous commenting allows time for thought. Searchable and referenceable.

**Issue trackers / task systems** (Jira, GitHub Issues, Linear): Designed for async tracking and decision-making. Good for work coordination; can be misused for status communication (become too detailed).

**Email:** Formal, documented, async by design. Slow for iteration; good for decisions, policies, important announcements.

**Internal blogs / wikis:** Async knowledge sharing. Searchable; can reference with permalinks. Requires discipline to update. Good for runbooks, FAQ, technical deep-dives.

**Recorded video / screencasts:** High information density. Async equivalent of a talk. Effective for onboarding, technical explanation, demos; time-consuming to produce.

### The Tool Stack Trap

Many remote teams accumulate too many tools: email for some things, Slack for others, Jira for tracking, Confluence for docs, Notion for wiki, GitHub for code. Information spreads across systems; new people don't know where to look.

Sustainable tool choices:
- **Async-first system** (e.g., Confluence or Notion) as the source of truth for decisions, designs, operational knowledge
- **Sync system** (e.g., Slack) for brief Q&A and collaboration within projects, not as a record-keeping system
- **Code system** (e.g., GitHub) for code and code-related decisions (ADRs in the repo)
- Minimize the number of systems; consolidate where possible

## Culture Building in Remote Teams

### Onboarding Remote Engineers

Remote onboarding is harder than co-located because new hires don't absorb culture by osmosis. It requires intentional structure:

- **Assigned mentor/buddy** for first month; regular 1:1s
- **Written onboarding plan** (first day, first week, first month milestones)
- **Pair programming or shadow sessions** for complex systems
- **Recorded walkthroughs** of key systems/processes
- **Social time** (async ok: team Slack channel, office hours, virtual coffee chats)
- **First substantial task:** Something simple enough not to block on approvals, complex enough to learn the system

Remote onboarding typically takes 1-2x as long as co-located. This is normal; budget for it.

### Hybrid: The Worst of Both Worlds

Hybrid teams (some co-located, some remote) often fail because:
- Co-located people have ambient context; remote people don't (and feel left out)
- Meetings designed for co-located people (in a conference room with shared screen/whiteboard); remote people can't contribute
- Communication defaults to real-time (good for co-located, bad for remote)

**Making hybrid work:**
- Require everyone to join calls remotely, even if in the same office (equalize participation)
- Treat the "office" as a meeting facility, not a default collaboration space
- Document everything as if everyone is remote; being co-located is just a physical artifact
- Rotate office days so remote people aren't always the odd ones out

## Decision Documentation

Decisions that aren't documented are rediscovered repeatedly. A team solves a problem, ships the solution, and 6 months later, a new team member asks "Why is it done this way?" The original reasoning is lost.

**What to document:**
- Architecture decisions (ADRs in the codebase or in Confluence)
- Process decisions ("We use GitHub-flow because...", "We require 2 approvals for...")
- Trade-off rationales ("We chose Postgres over MongoDB because...")
- Lessons learned from incidents (blameless postmortems)

**Format:**
- **Title:** "We chose Postgres over MongoDB for the data store"
- **Status:** Proposed, Accepted, Deprecated
- **Context:** What was the problem? Why did we need to decide?
- **Decision:** What did we decide?
- **Consequences:** What are the implications? (positive and negative)
- **Alternatives considered:** Why not X, Y, or Z?

**Where:**
- Architectural decisions: In the repo as ADRs (`docs/adr/0001-chosen-db.md`)
- Process decisions: In a team wiki or handbook
- Code-specific rationale: In code comments (but prefer comments for "why", not "what")

## Security and Autonomy in Remote Work

Remote work requires trust. Excessive surveillance ("always-on webcams", keystroke logging, GPS tracking) is counterproductive:
- It erodes trust and increases turnover
- It doesn't improve quality or productivity
- It attracts regulatory and legal risk

Sustainable remote policies:
- Minimal surveillance; trust employees to manage their time
- Asynchronous check-ins via status/progress updates, not micro-management
- Focus on output (shipped features, fixed bugs) not activity (hours online)
- Respect work-life boundaries and timezone limits (don't expect instant Slack responses)

## See Also

- [Process Technical Writing — Documentation, ADRs, RFCs & Design Documents](process-technical-writing.md) — Detailed guide to writing design artifacts
- [Process Code Review](process-code-review.md) — Async knowledge transfer mechanism
- [Process Developer Onboarding](process-developer-onboarding.md) — Remote-specific onboarding challenges