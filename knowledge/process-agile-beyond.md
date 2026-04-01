# Agile Beyond Scrum — Kanban, Shape Up, Extreme Programming & Hybrid Models

## The Agile Spectrum

Scrum dominates enterprise frameworks, but it represents one point on a spectrum. The core tension: **Scrum enforces time-boxing to create predictability; other frameworks prioritize flow and responsiveness**. Each trades predictability, flexibility, and feedback cadence differently.

## Kanban: Flow Over Ceremonies

Kanban inverts Scrum's time-boxed batches into continuous flow. Rather than planning sprints, teams visualize work stages on a board and pull items as capacity allows.

**Core mechanism: WIP limits.** Each workflow stage (Design, Implementation, Review, Deploy) has a maximum number of concurrent items. When a stage hits its limit, no new work enters until something completes. This prevents context switching and reveals bottlenecks immediately.

**Flow metrics** measure system health differently than velocity:
- **Lead time**: Wall-clock from request to completion. Reveals total system latency.
- **Cycle time**: Time spent actively working (excluding waits). Shows actual developer productivity.
- **Throughput**: Items completed per period. Enables forecasting without estimating individual work.
- **Cumulative flow diagrams**: Visualize bottlenecks—a thick horizontal band at "Review" signals approval delays, not engineering delays.

**Classes of service** partition work by urgency. Standard work flows through normal WIP; urgent requests bypass WIP limits or have separate lanes. This prevents "everything is urgent" scenarios and forces explicit prioritization.

Kanban suits maintenance, support, and operational work better than large, uncertain projects. It surfaces system capacity constraints ruthlessly but requires discipline—without ceremony (standups, retrospectives), teams can drift without reflection.

## Scrumban: Hybrid Predictability

Scrumban preserves Scrum's time-boxed planning and ceremony rhythm while adopting Kanban's continuous flow within the sprint. Teams plan work for a sprint but don't refine assignments—instead, they pull work from a backlog board in priority order, respecting WIP limits.

This pattern suits:
- Teams transitioning from Scrum to Kanban (less shock)
- Mixed workloads (some predictable roadmap work, some reactive support)
- Scaling to multiple teams (sprint boundaries enable cross-team sync)

The downside: Scrumban combines complexity of both systems. Teams often revert to pure Kanban or pure Scrum once they stabilize.

## Shape Up: Betting on Appetite

Shape Up (Ryan Singer, Basecamp) rejects velocity and estimation altogether. Instead, teams work in **6-week cycles** with fixed timebox and negotiable scope.

**The cycle:** Executives define strategic "bets"—problems worth solving. Teams write **pitches** (problem, appetite, solution sketch, risks). If a pitch is approved, a small team owns it for 6 weeks, no mid-cycle interruptions.

**Hill charts** visualize progress: "Figuring it out" phase (uphill, uncertain) transitions to "Making it" phase (downhill, execution). Unlike burndown charts, they acknowledge that learning time isn't wasted time—it's necessary before building.

Key insight: **Appetite replaces estimation.** "How long?" becomes "How much time can we afford?" This reframe prevents scope creep (the problem changes shape to fit the timebox) and surfaces hard tradeoffs early.

Trade-offs: Requires strong executive discipline (no mid-cycle pivots, no "just add..."), suits product-driven orgs (not consulting), assumes work can fit neat 6-week cycles (long R&D doesn't fit).

## Extreme Programming: Technical Excellence as Process

XP (Kent Beck, 1999) treats code quality and developer discipline as *process*, not ethics. Four foundational practices:

1. **Pair programming**: Two developers at one workstation. Driver codes, navigator reviews. Roles rotate every 15-30 minutes. Reduces bugs, spreads knowledge, prevents bottlenecks on key individuals.
   
2. **Test-driven development (TDD)**: Write failing tests before code. Tests define contracts; code implements them. Tight feedback loop (minutes, not hours). Disadvantage: slower initial development, higher discipline required.

3. **Continuous integration**: Merge to main multiple times daily, not weekly. Requires automated tests and automated deploys. Prevents "merge day" chaos and integration surprises.

4. **Collective code ownership**: No "my code, your code"—anyone can change anything, so refactoring happens continuously. Reduces handoff delays, requires strong test discipline.

XP assumes: Small co-located teams, domains where tests are natural (business logic, not UI), culture valuing engineering over shipping speed. Many orgs adopt XP practices selectively (TDD, CI) without the full ensemble.

## SAFe: Scaling with Ceremony

Scaled Agile Framework (SAFe) applies Scrum patterns to large organizations—program increments (multi-team sprints), release trains, synchronization ceremonies. It's heavyweight, prescriptive, and controversial.

Critics: SAFe maps hierarchical org structures to process (planners → teams) rather than challenging hierarchy. Adoption often increases process overhead without proportional delivery gains. Advocates: Large enterprises need coordination across dozens of teams; SAFe's cadence provides it.

## Integration with Team Topologies

Modern organizations combine agile process choice with explicit team structure. If teams have unclear boundaries, no process fixes it. **Team Topologies** vocabulary: Stream-aligned teams (own feature value from end-to-end) use flexible cadences; platform teams (provide services to stream teams) use Kanban to respond to demand; enabling teams (coach and unblock) use event-driven responsiveness.

The integration: Choose process to match team mission. Stream teams might use Scrum for predictability; platform teams need Kanban to handle variable demand.

## Frame: Process Fits Mission

No process is universally best. Kanban fits service/support work. Scrum fits predictable feature delivery. Shape Up fits product-decisive orgs. XP practices fit teams valuing code quality above ship speed. The question isn't "which is best" but "which fits our constraints and what we measure?"

See also: [process-technical-writing.md](process-technical-writing.md), [process-team-topologies.md](process-team-topologies.md), [process-code-review.md](process-code-review.md).