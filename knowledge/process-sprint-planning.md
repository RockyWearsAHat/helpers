# Sprint Planning — Estimation, Velocity, Capacity & Backlog Refinement

## Sprint Fundamentals

A **sprint** is a fixed time-boxed iteration (typically 1-2 weeks) where a team commits to completing a set of work items, then reflects on what was accomplished and adjusts.

Sprint planning operates at the intersection of four disciplines: **estimation** (how much effort?), **capacity** (how much capacity does the team have?), **velocity** (what's the team's historical throughput?), and **prioritization** (what's worth doing first?).

See also: [process-agile-beyond.md](process-agile-beyond.md), [process-team-topologies.md](process-team-topologies.md)

## Story Points vs. Time Estimates

### Story Points: Relative Sizing

Story points measure **relative complexity and effort**, not time. A story pointing at 5 doesn't mean "5 hours;" it means "roughly twice as complex as a 2 or 3 point story."

**Advantages**:
- Accounts for unknown unknowns: A story might take 8 hours or 4 weeks depending on what you discover. Points embrace uncertainty
- Decouples from individuals: A 5-point story takes 5 hours for Engineer A and 3 for Engineer B. Points reflect the work's complexity, not an individual's speed
- Enables velocity trending: "The team completes ~40 points per sprint" is meaningful across sprints and team members
- Scales to portfolio level: Can sum points across teams to understand org-level throughput

**Disadvantages**:
- Requires calibration: Team must agree on point scale (is 5 medium or large?)
- Can become cargo cult: Points alone don't predict actual time; only velocity anchors points to reality
- Estimation is still hard: Relative sizing is easier than absolute time, but still requires reasoning about unknowns

### Time Estimates: Absolute Duration

Time estimates (hours, days) directly represent expected effort.

**Advantages**:
- Explicit and auditable: "Does this story really take 8 hours? Let me check my assumptions"
- Useful for capacity planning: "We have 120 engineering hours next week; which stories fit?"
- Easier for non-technical stakeholders: Nobody needs to explain what "8 hours" means

**Disadvantages**:
- Individual variance: Same deliverable takes different engineers different time. Whose estimate matters?
- Cognitive bias: Humans systematically underestimate time. Adding 40% as a fudge factor is cargo cult; better to track actuals and calibrate
- Feels like micromanagement: Estimating every 2-hour task creates friction

### Hybrid: Points + Time-Boxing

Some teams use **points for planning** and **time-boxing for execution**:

1. Estimate stories in story points during planning
2. During sprint, track actual **elapsed time** or **time spent working** (not wall-clock time)
3. Retrospectives: Compare points-estimated velocity to actual time spent. "We planned 40 points; spent 160 actual engineering hours; some over-time items happened"
4. Adjust next sprint's point targets or time-box if pattern emerges

## Estimation Techniques

### Planning Poker

Team sits together (or in video call) and estimates a story:

1. **Someone reads the story** aloud; clarifying questions answered
2. **Team simultaneously shows cards** (or numbers) representing their estimate in points
3. **If agreement**: Story gets that point value
4. **If disagreement**: Lowest and highest estimator explain their reasoning. Re-estimate if light bulbs turn on; else, take the average or slightly higher value (to be conservative)

**Advantages**: Builds team calibration; reveals assumptions; quick
**Disadvantages**: Can be theatrical; expert can anchor others' estimates; doesn't work well for very large stories

### T-Shirt Sizing (XS, S, M, L, XL)

Quick relative categorization before fine-grained estimation.

- **XS**: One-liner fixes. < 2 hours
- **S**: Small feature or bug fix. 2-8 hours
- **M**: Complex feature or refactor. 1-3 days (8-24 hours)
- **L**: Major feature or architectural change. 1-2 weeks of work
- **XL**: Epic or multi-week initiative. Break down further

**Use**: Triage backlog quickly. Anything XL or L gets broken into smaller stories before sprint planning.

### Three-Point Estimation

Estimate three scenarios for each story:

- **Optimistic (O)**: Everything goes right, no surprises. Lower bound
- **Pessimistic (P)**: Things go wrong, we discover unknowns, rework needed. Upper bound
- **Most likely (M)**: Middle-ground expectation

**Formula**: (O + 4M + P) / 6

Captures uncertainty; gives higher weight to realistic estimates than to extremes.

**Advantage**: Makes uncertainty explicit; less overconfident
**Disadvantage**: Takes more time; teams often skip the nuance and just do O, M, P → average them

## Velocity and Capacity Planning

### Velocity Defined

**Velocity** is the number of story points a team completes per sprint (or the amount of time spent, depending on your metric).

Team velocities are highly individual; there's no "good" velocity in absolute terms. A velocity of 20 points/sprint for Team A doesn't mean Team B should aim for 20.

### Calculating Velocity

At the end of a sprint:
- Sum the story points of all **completed** stories (not estimates for partially-done work)
- That sum is the sprint's velocity

**Multi-sprint trend**: Average the last 3-4 sprints to get a "typical" or "average" velocity. Use this for forward planning.

### Velocity Trending

Over 4-8 sprints, patterns emerge:

- **Stable velocity**: Team completes ~X points per sprint. Predictable
- **Declining velocity**: Team velocity steadily drops. Signals: Technical debt accumulation, unplanned interruptions (on-call, emergencies), team churn, or estimation creep
- **Volatile velocity**: Swings wildly (20, 40, 25, 35). Signals: Inconsistent estimation, scope creep mid-sprint, dependencies on other teams, or stories sized inconsistently

**Use**: A stable velocity enables **predictable capacity planning**. If velocity is 35±5 points/sprint, the team will deliver ~35 points next sprint (with some variance).

### Capacity Adjustment

Capacity planning adjusts for known events:

- **Holidays**: 2-week holiday? Maybe 80% of typical velocity
- **Oncall rotation**: Team on-call this sprint? Maybe 75% 
- **Unplanned context-switching**: This usually isn't known up-front, but observed retrospectively
- **Major initiatives**: Team splitting effort between sprint stories and a larger project? Reduce sprint point target

**Formula**: Adjusted Capacity = Average Velocity × Adjustment Factor

Example: "Average velocity is 40 points. This sprint has 3 engineers on-call and 1 at a conference. Target: 40 × 0.7 = 28 points."

## Backlog Refinement

Backlog refinement (or "grooming") is ongoing pre-planning work that prepares stories for sprints.

### What Gets Refined

- **User stories and feature requests**: Clarified, broken into smaller stories, prioritized
- **Technical debt**: Estimated and sequenced
- **Bugs**: Sized and prioritized
- **Unknowns**: Story flagged as "needs spike" or research

### When It Happens

- **Continuous throughout the sprint**, not just sprint planning
- Typical: 10-15% of team capacity allocated to refinement (alongside sprint work)
- **Or** dedicated refinement meetings: 1-2 hours mid-sprint, team lead + tech lead groom the backlog for the next sprint

### Refinement Discipline

A story is **ready for sprint planning** when:
- **Clear acceptance criteria**: What does "done" mean?
- **Estimated**: Team has made a rough sizing estimate
- **Dependencies identified**: Does this block anything? Is it blocked by anything?
- **Unknowns surfaced**: Major technical risk called out; if too risky, maybe a spike is needed
- **Prioritized relative to other stories**: Not arbitrary; aligned with product goals

A story is **not ready** if:
- Acceptance criteria are vague ("Make it faster")
- Ambiguous scope ("Handle edge cases" — which edge cases?)
- Estimates are missing or wildly uncertain
- Critical dependencies aren't clear

### Spike Stories

When significant uncertainty exists, a **spike** (or research task) is created:

- Time-boxed (1-2 days, not open-ended)
- Goal: Reduce uncertainty enough to estimate the real work
- Output: Proposal or decision, not production code
- Follow-up: Real story, now estimable, queued for a future sprint

**Example**: "Spike: Evaluate database options for analytics workload (1 day). Outcome: Decisions on PostgreSQL vs. Redshift vs. BigQuery + estimate for implementation"

## Anti-Patterns and Correctives

### Pattern: Scope Creep Mid-Sprint

**Problem**: Team commits to 40 points, then manager adds 15 more urgent stories. Sprint never ends; velocity metric becomes meaningless.

**Corrective**:
- **Sprint boundary is sacred**: Accept that sprints are fixed. Urgent items go into next sprint
- **Exception process**: If truly emergency (production incident, compliance violation), pull item in but *pull something else out* to maintain commitment
- **Track interrupt frequency**: If >20% of work is unplanned, investigate why and address root cause

### Pattern: Estimation Inflation

**Problem**: Team consistently estimates stories at "worst case" and completes them in 60% of estimated points. Velocity signal is meaningless.

**Corrective**:
- **Track actuals**: Time spent vs. estimated points. If consistently 40% better than estimated, team's point scale is off
- **Recalibrate**: "We're all pointing at worst case. Let's be more honest about likely case. Adjust scale or estimation discipline"
- **Three-point estimation**: Forces explicit thinking about optimistic vs. realistic scenarios

### Pattern: Story Size Distribution Explosion

**Problem**: Backlog has a mix: 1-point trivial fixes, 8-point medium features, 40-point epics. Planning meeting becomes political negotiation about story size.

**Corrective**:
- **Policy**: No story larger than 13-points. Anything bigger gets split before planning
- **Rule of thumb**: A story should be completable by one engineer in 2-5 days. If it takes longer, it's an epic
- **Split stories**: Use independent axis (happy path vs. edge cases, UI vs. API layer) to divide large work

### Pattern: Velocity as Productivity Metric

**Problem**: Manager uses team velocity (points per sprint) to compare teams or measure "productivity."

**Corrective**:
- **Educate**: Velocity is *forecasting tool*, not productivity measure. Two teams with identical velocity might have wildly different impact if stories point differently
- **Watch for gaming**: If velocity is a metric, teams will point optimistically to look good. Velocity becomes useless for planning
- **Context matters**: A team delivering 20 points/sprint of high-impact work (revenue customers, critical reliability) is more valuable than a team delivering 60 points/sprint of tech debt reduction

## Sprint Planning Meeting Structure

Typical 2-hour sprint planning for a 2-week sprint:

1. **Business context** (15 min): Product lead summarizes priorities, asks, dependencies, business context
2. **Team capacity** (10 min): How many points can the team realistically commit to this sprint?
3. **Walkthrough & estimation** (60-75 min): Product lead presents backlog; team estimates and asks clarifying questions
4. **Commitment** (15 min): Team reaches consensus on which stories to tackle; clarify definition of done; identify risks
5. **Closing** (5 min): Recap, communicate sprint goal, address blockers

**Note**: Stories should already be refined before this meeting. If refinement is poor, planning becomes re-hashing of vague requirements, not actual planning.