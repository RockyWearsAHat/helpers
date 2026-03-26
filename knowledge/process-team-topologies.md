# Team Topologies — Organizational Design for Software Delivery

## Conway's Law and Its Implications

Melvin Conway observed in 1967 that organizations which design systems are constrained to produce designs that copy the communication structures of those organizations. This observation, later dubbed Conway's Law, has proven remarkably durable across decades of software engineering practice.

The implication runs deep: if three teams own a compiler, the result tends to be a three-pass compiler — not because three passes represent optimal design, but because the organizational structure imposes that shape. System boundaries tend to align with team boundaries because coordination across teams carries friction that coordination within teams does not.

| Organizational pattern              | Likely system outcome                        |
| ----------------------------------- | -------------------------------------------- |
| Single co-located team              | Monolithic, tightly integrated               |
| Multiple siloed teams               | Modular with well-defined interfaces         |
| Geographically distributed teams    | Services with asynchronous communication     |
| Teams organized by technology layer | Layered architecture (frontend/backend/data) |
| Teams organized by business domain  | Domain-oriented services                     |

Conway's Law operates as a force, not a rule — intentional effort can push against it, but the gravitational pull of organizational structure reasserts itself over time, particularly as systems evolve and teams make local decisions.

## The Inverse Conway Maneuver

If organizational structure shapes system architecture, then deliberately designing team structure becomes an architectural decision. The inverse Conway maneuver — a term popularized by Thoughtworks — involves reorganizing teams to match the desired target architecture rather than allowing the existing org chart to dictate system design.

This approach treats team topology as a first-class architectural concern:

- Identify the desired system architecture
- Design team boundaries to match desired service or module boundaries
- Establish communication patterns between teams that mirror desired system interfaces
- Allow Conway's Law to work in your favor rather than against you

The maneuver carries risks. Reorganizations disrupt existing relationships and knowledge networks. The desired architecture may itself be flawed, and encoding it into organizational structure makes it harder to change. Success depends on correctly identifying the target architecture and maintaining alignment as both the organization and system evolve.

## The Four Fundamental Team Types

Team Topologies, as formalized by Matthew Skelton and Manuel Pais, proposes four fundamental team types that compose to cover the needs of modern software delivery organizations.

### Stream-Aligned Teams

Stream-aligned teams are organized around a flow of work — typically a business domain, a product area, a user journey, or a specific customer segment. They are the primary team type, and most teams in an organization should be stream-aligned.

Characteristics:

- Own the full lifecycle of their part of the system (build, run, support)
- Aligned to a single stream of work that delivers value
- Cross-functional — contain the skills needed to deliver without handoffs
- Measured by flow metrics: lead time, deployment frequency, change failure rate
- Minimize dependencies on other teams for day-to-day delivery

The stream orientation means these teams feel the direct pressure of user needs and can respond without coordination overhead. They internalize the feedback loop from production.

### Enabling Teams

Enabling teams exist to help stream-aligned teams acquire missing capabilities. They do not build features directly but instead detect capability gaps and help close them through education, tooling support, and temporary collaboration.

- Composed of specialists in a particular domain (testing, security, observability, cloud infrastructure)
- Engage with stream-aligned teams for bounded periods to transfer knowledge
- Measure success by the degree to which stream-aligned teams become self-sufficient
- Avoid becoming permanent dependencies or gatekeepers

The enabling team pattern recognizes that stream-aligned teams cannot be expert in everything simultaneously. Rather than creating permanent dependencies on specialist teams, enabling teams raise the capability baseline across the organization.

### Complicated Subsystem Teams

Some components require deep specialist knowledge that would be unreasonable to expect every stream-aligned team to maintain. Complicated subsystem teams own these components — mathematical engines, codec implementations, real-time processing systems, domain-specific algorithms.

- Justified only when the subsystem requires specialist knowledge most engineers lack
- Reduce cognitive load on stream-aligned teams by encapsulating deep complexity
- Provide their subsystem as a service or library with a clear API
- Should be rare — overuse fragments ownership and creates bottlenecks

The key distinction from a component team is justification through genuine complexity, not organizational convenience.

### Platform Teams

Platform teams provide internal services that accelerate stream-aligned team delivery. They treat their internal consumers as customers and their platform capabilities as products.

- Reduce cognitive load by abstracting infrastructure, tooling, and cross-cutting concerns
- Provide self-service capabilities with clear documentation and APIs
- Operate as a product organization serving internal developer customers
- Balance standardization with flexibility — overly rigid platforms become bottlenecks

## Interaction Modes

The relationships between teams matter as much as the team types themselves. Three fundamental interaction modes govern how teams work together.

### Collaboration

Two teams work closely together for a defined period to discover or build something. Collaboration is high-bandwidth but expensive — it blurs boundaries and increases coordination overhead.

- Appropriate during discovery phases when boundaries are unclear
- Should be time-boxed — permanent collaboration indicates misaligned boundaries
- Both teams invest significant capacity in the joint work
- Expect messy, overlapping ownership during the collaboration period

### X-as-a-Service

One team provides a capability that another team consumes through a well-defined interface. The consuming team does not need to understand the internals.

- Clear ownership boundaries reduce coordination overhead
- The providing team manages the interface contract and operational concerns
- Works well when the capability is stable and well-understood
- Requires investment in documentation, API design, and operational excellence

### Facilitating

One team helps another team learn or adopt a new capability. The facilitating team does not do the work but guides the other team through it.

- Knowledge transfer is the primary goal
- The facilitating team brings expertise; the receiving team retains ownership
- Time-boxed — success means the receiving team no longer needs help
- Common pattern for enabling teams working with stream-aligned teams

| Interaction mode | When appropriate                   | Cost              | Duration   |
| ---------------- | ---------------------------------- | ----------------- | ---------- |
| Collaboration    | Discovery, unclear boundaries      | High coordination | Time-boxed |
| X-as-a-Service   | Stable, well-understood capability | Low coordination  | Ongoing    |
| Facilitating     | Capability gap, knowledge transfer | Moderate          | Time-boxed |

## Cognitive Load as a Team Sizing Principle

Cognitive load theory, borrowed from educational psychology, provides a framework for reasoning about team capacity. A team has a finite cognitive budget — the total complexity it can effectively understand, maintain, and evolve.

Three types of cognitive load apply to software teams:

- **Intrinsic** — the inherent complexity of the problem domain the team works in
- **Extraneous** — complexity imposed by tooling, processes, environment, and organizational friction
- **Germane** — the effort required to learn and build mental models of the domain

When total cognitive load exceeds a team's capacity, quality suffers — shortcuts emerge, incidents increase, delivery slows. The implication for team design is that team boundaries should be drawn to keep cognitive load within manageable limits.

Factors that increase a team's cognitive load:

- Owning too many services or components
- Working across multiple disparate domains
- Managing complex infrastructure alongside application development
- Navigating unclear or frequently changing requirements
- Coordinating with many other teams simultaneously
- Maintaining legacy code alongside new development

Platform teams reduce cognitive load by abstracting infrastructure concerns. Complicated subsystem teams reduce it by encapsulating deep specialist knowledge. The goal is to keep stream-aligned teams focused on their domain rather than spread across too many concerns.

## Team APIs

The concept of a team API extends the API metaphor from software to organizational design. A team API defines how other teams interact with a given team — what they can expect, how to request things, and what commitments exist.

Elements of a team API:

- **Code and artifacts** — repositories, services, libraries the team owns
- **Communication channels** — how to reach the team, preferred contact methods
- **Documentation** — what information the team publishes about its work
- **Workflow** — how to request changes, report issues, propose collaboration
- **Service levels** — response time expectations, operational commitments
- **Boundaries** — what the team does and does not own

Making team APIs explicit reduces ambiguity and allows teams to manage their interaction surface deliberately rather than reactively. It also makes organizational dependencies visible and manageable.

## The Platform as a Product

Internal developer platforms succeed or fail based on whether they are treated as products serving developer customers or as mandated infrastructure imposed top-down.

Product-oriented platform thinking involves:

- Understanding developer workflows and pain points through research
- Providing self-service capabilities that do not require tickets or manual intervention
- Maintaining documentation, tutorials, and migration guides
- Measuring adoption, satisfaction, and developer productivity
- Iterating based on feedback rather than assumption
- Offering sensible defaults with escape hatches for unusual requirements

Platforms that mandate adoption without earning it tend to generate workarounds. Teams route around friction — if the platform is harder to use than the alternative, adoption will be shallow regardless of organizational mandates.

The thinnest viable platform provides just enough abstraction to accelerate delivery without becoming a constraint on capability. Too thin and teams duplicate effort; too thick and the platform becomes a bottleneck or a straitjacket.

## Feature Teams vs Component Teams

Two competing approaches to team organization create different trade-off profiles.

**Feature teams** own end-to-end delivery of a feature across all layers of the stack. A feature team building a checkout flow owns the frontend, backend, data, and infrastructure for that feature.

- Advantages: reduced handoffs, faster flow, direct accountability for outcomes
- Challenges: requires broad skill sets, risk of duplicated infrastructure, potential for inconsistency across features

**Component teams** own a specific technical layer or component — the database layer, the notification service, the mobile app.

- Advantages: deep technical expertise, consistent implementation within a layer, simpler mental model per team
- Challenges: cross-team coordination for every feature, handoff delays, diffusion of responsibility for end-to-end outcomes

| Dimension             | Feature teams                        | Component teams                      |
| --------------------- | ------------------------------------ | ------------------------------------ |
| Delivery speed        | Faster (fewer dependencies)          | Slower (coordination overhead)       |
| Technical consistency | Lower (each team makes own choices)  | Higher (single team owns each layer) |
| Knowledge breadth     | Broader per engineer                 | Deeper per engineer                  |
| End-to-end ownership  | Clear                                | Fragmented                           |
| Scaling challenge     | Code duplication, divergent patterns | Bottleneck teams, queuing            |

Most organizations operate with a blend. Stream-aligned teams (which resemble feature teams) handle the majority of delivery, while a small number of complicated subsystem and platform teams own shared components.

## Two-Pizza Teams and Communication Overhead

Amazon's "two-pizza team" heuristic — a team should be small enough to feed with two pizzas — encodes an empirical observation about communication overhead.

The number of communication channels in a group of n people is n(n-1)/2. A team of 5 has 10 channels; a team of 10 has 45; a team of 15 has 105. Communication overhead grows quadratically while productive capacity grows linearly.

Small teams (5-9 people) tend to:

- Maintain shared mental models more easily
- Make decisions faster with less coordination cost
- Build stronger trust and psychological safety
- Feel individual accountability more acutely

Larger teams provide more specialization and capacity but at the cost of coordination overhead, diffusion of responsibility, and the formation of informal sub-groups.

The optimal team size depends on the complexity and breadth of the domain the team owns, the maturity of the codebase and tooling, and the degree of collaboration required with other teams. The two-pizza heuristic serves as a starting point, not a universal law.

## Scaling Patterns

As organizations grow, they adopt frameworks and models to coordinate across many teams. Several prominent approaches exist, each optimizing for different outcomes.

### Spotify Model

Popularized (and frequently misunderstood) as a scaling framework, the Spotify model describes squads (small cross-functional teams), tribes (collections of related squads), chapters (communities of practice across squads sharing a specialty), and guilds (informal interest groups).

- Optimizes for team autonomy and alignment through shared mission rather than process
- Assumes high trust and mature engineering culture
- The model as commonly described is a snapshot of one company at one time — Spotify itself evolved beyond it
- Risk: without strong alignment mechanisms, autonomy drifts into fragmentation

### SAFe (Scaled Agile Framework)

SAFe provides prescriptive ceremony, role, and artifact definitions for coordinating large numbers of teams. It defines Agile Release Trains (ARTs) as groups of teams delivering on a shared cadence.

- Optimizes for predictability and coordination across large enterprises
- Provides clear structure for organizations transitioning from traditional project management
- Criticized for heavy process overhead and diminished team autonomy
- Trade-off: coordination certainty at the cost of agility and innovation speed

### LeSS (Large-Scale Scrum)

LeSS takes a minimalist approach — fewer roles, artifacts, and ceremonies than SAFe, with a focus on keeping Scrum's core principles while scaling to multiple teams.

- Optimizes for simplicity and keeping the organization as flat as possible
- Requires whole-product focus — all teams work from a single product backlog
- Challenges: requires significant organizational change, does not provide as much structural guidance
- Trade-off: simplicity at the cost of coordination support for very large organizations

## The Sociotechnical System Perspective

Organizations and their technical systems form a single sociotechnical system. Changes to one part ripple through the other — a microservices architecture requires different team structures than a monolith, and a hierarchical organization produces different software than a flat one.

This perspective implies that:

- Architectural decisions are organizational decisions and vice versa
- Optimizing technology without considering the organizational context produces suboptimal results
- Team interactions, communication patterns, and incentive structures shape code as much as design documents
- Technical debt often has organizational roots — unclear ownership, misaligned incentives, coordination failures
- Successful transformation requires changing both technical systems and organizational structures simultaneously

## Brooks's Law

Fred Brooks observed in "The Mythical Man-Month" (1975) that adding manpower to a late software project makes it later. The underlying dynamics:

- New team members require ramp-up time during which they are net consumers of existing members' time
- Communication overhead increases with each new person added
- Tasks that require sequential intellectual effort cannot be parallelized — nine women cannot make a baby in one month
- The partitioning of work across more people increases integration complexity

Brooks's Law is not absolute — it applies most strongly to projects that are already behind schedule and to work that requires tight coordination. Projects with well-defined, independent work packages can absorb new members more effectively. But the core insight endures: coordination costs are real, ramp-up takes time, and the relationship between team size and output is sublinear.

The organizational implication is that team sizing decisions made early matter more than heroic staffing changes made late. Investing in team structure, clear boundaries, and reduced coordination overhead pays dividends when pressure arrives — because adding people under pressure rarely produces the expected result.
