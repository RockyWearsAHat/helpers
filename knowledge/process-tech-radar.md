# Technology Radar — Evaluation Framework, Rings, and Team-Specific Assessment

## What Is a Technology Radar?

A **technology radar** is a systematic framework for evaluating, tracking, and communicating decisions about technologies, frameworks, platforms, and programming languages across an organization. Unlike informal technology discussions scattered across Slack or wiki pages, a radar makes technology recommendations visible, discoverable, and continuously evolving.

A technology radar visualizes technologies arranged in four concentric rings representing maturity and organizational readiness, and divides each ring into four quadrants representing categories of concern.

See also: [process-technical-leadership.md](process-technical-leadership.md), [process-architecture-decisions.md](process-architecture-decisions.md)

## The Four Rings

### Adopt

Technologies the organization has thoroughly validated and committed to using broadly.

- **Characteristics**: Proven in production, significant installed base within the org, documented patterns and standards, tooling support exists
- **Examples**: Python for data pipelines (if that's broadly enabled), PostgreSQL for transactional systems, Kubernetes for orchestration
- **Responsibility**: Teams use Adopt technologies by default. Deviations require explicit justification. Expect strong tooling, library support, and internal expertise
- **Organizational commitment**: Training budgets, procurement, production support

### Trial

Technologies the organization believes have merit and should gain limited production exposure to validate feasibility and discover operational patterns before org-wide commitment.

- **Characteristics**: Evaluated by experienced teams, deployed to non-critical systems or limited traffic, learning captured and shared
- **Scope**: A specific team, a bounded system, or a fraction of traffic flowing through a system
- **Exit criteria**: Does this address the problem better than Adopt alternatives? What are the operational surprises? Will we maintain this?
- **Examples**: A new language frontend (Go services alongside Python), an emerging database (DuckDB for analytics), a novel operational pattern (GitOps vs. traditional CI/CD)
- **Risk**: Trial doesn't infinitely buffer a technology; decisions to graduate to Adopt or descend to Assess must happen within months, not years

### Assess

Technologies worth investigating and learning about, but not yet production-ready within the organization.

- **Characteristics**: Promising designs, proven elsewhere, worth understanding but not deploying broadly yet. Internal experiments or prototypes acceptable. Active research phase
- **Examples**: A new language gaining traction (Rust, if Python/Go cover current needs), emerging architectural patterns (event-driven if the org is Lambda-first), standards in flux (observability formats, testing frameworks)
- **Purpose**: Build organizational awareness and **reserve the option** to adopt quickly when the time is right. Assign engineers to stay current
- **Anti-pattern**: Endless assessment. Set explicit decision timelines — "Review this in Q3. Decide: Trial, Assess (continue), or Hold"

### Hold

Technologies the organization has deliberately chosen not to pursue, or is de-prioritizing.

- **Characteristics**: Explicit decision recorded (not just forgotten)
- **Reasons**: Better alternatives exist; architectural misalignment; vendor or community concerns; maintenance burden exceeds benefit; opportunity cost; risk profile unacceptable
- **Examples**: A framework the organization has outgrown. A platform deprecated by vendors. A language whose ecosystem gaps outweigh benefits for your use cases
- **Responsibility**: Teams must justify any exception. Hold is not absolute — reconsider when circumstances change
- **Archival**: After several years in Hold (typically 3+ years with no reconsideration), move to archive

## The Four Quadrants

Rings answer **maturity**. Quadrants organize by **concern type and product area**, ensuring the radar covers breadth:

### Techniques

Practices, methodologies, patterns, deployment models, testing strategies, code organization approaches.

- **Examples**:
  - Adopt: Trunk-based development, automated testing, continuous deployment, infrastructure-as-code, monorepo patterns
  - Trial: Feature flagging at scale, canary deployments, zero-trust networking
  - Assess: Chaos engineering adoption, GitOps, event-driven teams
  - Hold: Waterfall delivery, manual testing at scale (acceptable only in specific contexts)

### Tools

Specific software products and services: CI/CD systems, observability platforms, version control systems, project management tools, security scanners.

- **Examples**:
  - Adopt: GitHub/GitLab (version control), Datadog/Prometheus + Grafana (observability), PagerDuty (incident response)
  - Trial: New observability vendor, alternative CI/CD, automated code review platforms
  - Assess: Emerging APM tools, security scanning startups
  - Hold: Deprecated platforms, vendors with poor support track records, tools creating lock-in

### Platforms

Infrastructure abstractions, messaging systems, application servers, container orchestration, compute models.

- **Examples**:
  - Adopt: Kubernetes (if standardized), PostgreSQL, Redis, managed cloud services (AWS, GCP, Azure)
  - Trial: New managed services, alternative orchestration (Nomad if Kubernetes is the standard), emerging infrastructure-as-code approaches
  - Assess: Edge computing platforms, new database architectures, alternative container runtimes
  - Hold: On-premise equivalents (if cloud-first), deprecated platforms

### Languages

Programming languages, domain-specific languages, configuration languages.

- **Examples**:
  - Adopt: Python, JavaScript/Node, Java, Go (depending on org)
  - Trial: Rust (for systems programming), TypeScript (if adopting from JavaScript), Kotlin (JVM alternative)
  - Assess: Emerging languages, niche languages for specific problems (Julia for numerical computing, Elixir for distributed systems)
  - Hold: Legacy languages the org is actively migrating away from

## Building a Team-Specific Radar

Org-wide radars are necessary but insufficient. Teams must operate radars scoped to their domain:

### Scaling Across Teams

- **Central radar** (org-level): Standards, risk, procurement, governance. Maintained by architects and technical leaders. Updated quarterly
- **Domain radars** (team/squad-level): Specific to product domains. Can move technologies through rings faster than org radar (3-month assessment cycles vs. annual)
- **Cross-team alignment**: Quarterly sync on domain radars to surface conflicts and shared learnings. Prevent teams from re-optimizing the same problems independently

### Process

1. **Identify drivers**: What problems does the team face? What's slowing delivery? What operational surprises appeared last quarter?
2. **Nominate technologies**: Technologies addressing those drivers land in Assess. Include rationale: What gap does this fill?
3. **Assign owners**: One or two engineers per technology take responsibility for learning, prototyping, and communicating findings
4. **Plan trials**: Move promising candidates to Trial with explicit success criteria and timelines
5. **Gather learning**: Capture deployment patterns, operational costs, team velocity impact, hiring implications
6. **Decide**: Adopt, continue Assess (unlikely; most technologies should exit within 6-12 months), or Hold

### Calibration

Avoid radaritis — excessive bike-shedding about ring placement. A technology in Trial vs. Assess is less important than clarity about:

- **Why is this on the radar?** What problem does it solve?
- **Who is responsible for the decision?** Assign ownership; don't let it drift
- **What does success look like?** Exit criteria, not vague approval
- **When does this get revisited?** Decisions are not permanent, but they're not ephemeral either

## Communicating the Radar

Radars are only valuable if discoverable and understood. Common communication patterns:

- **Visualization**: Radar chart (concentric rings) or matrix (rings × quadrants). Visual makes the technology landscape comprehensible
- **Documentation**: Each entry includes: technology name, ring, quadrant, owner, summary, rationale, trial criteria (if Trial or Assess)
- **Regular reviews**: Org-wide tech talks quarterly. Team leads discuss what moved, why, and what's next
- **Discoverability**: Wiki, GitHub, Notion. Discoverable during project kickoff: "What's the org's position on messaging systems?" (check the radar)

## Anti-Patterns

- **Radar drift**: Entries stuck in Assess indefinitely. Explicitly sunset entries or graduate/hold them
- **Disagreement avoidance**: Radar becomes a laundry list of everything anyone suggested rather than actual recommendations
- **Leadership capture**: Radar reflects one architect's preferences, not team learning
- **No transitions**: All rings stay static. Good radar shows *motion* — technologies graduating and retiring
- **Divorced from reality**: Org adopts a technology but radar never updates; radar becomes fiction