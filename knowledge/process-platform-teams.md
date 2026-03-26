# Platform Engineering Teams — Team Topology, Product Thinking & Adoption

## Beyond Infrastructure Teams

"Platform engineering" often gets conflated with infrastructure operations. The distinction: **Ops maintains existing systems; platform teams design experiences**. The mental shift is crucial. Infrastructure teams respond to outages. Platform teams invent self-service workflows that prevent outages.

This reframing came from Manuel Pais and Matthew Skelton (Team Topologies): Stream-aligned teams deliver business value; platform teams reduce cognitive load for stream teams by providing curated abstractions; enabling teams (often short-lived) coach and unblock.

## Platform-as-Product Thinking

A platform is not a checkbox. It's a **product with internal customers** (developers, operators, security teams). Successful platforms adopt product mindset:

- **Developer experience (DX) metrics**: How fast can a dev go from idea to deployed service? What's the cognitive load? Measure: onboarding time, time-to-first-deploy, API discovery friction, documentation completeness.
- **Usage data**: Track adoption of platform features. Which templates do teams use? Which tools are abandoned? Why?
- **Feedback loops**: Regular surveys, office hours, feedback channels. Ask: What's slowing you down? What would help?
- **Roadmap transparency**: Platforms without clear roadmaps fragment—teams build alternatives ("shadow IT").

Many organizations treat platform like infrastructure: "We built it; go use it." Then wonder why adoption stalls. Real adoption requires *customer development*—just like external products.

## Golden Paths: Scaffolded Workflows

Golden paths are **lightweight, documented workflows for common tasks**: deploy a microservice, add a database, configure monitoring. Not prescriptive (one way to do it); instead, a recommended starting point that teams can diverge from intentionally.

Technically, golden paths combine:
- **Templates** (scaffold boilerplate: dockerfile, github actions workflows, helm charts)
- **Documentation** (how to use the template, what it sets up, when to customize vs. use as-is)
- **Integration** (templates auto-generate via IDP portals, reducing copy-paste errors)
- **Governance** (embedded policy: resource limits, security scans, compliance checklists)

Trade-off: Golden paths reduce decision overhead but risk homogeneity. Teams optimize for "works with the platform" rather than "works for our use case." Successful organizations balance: strict guardrails for security/compliance, flexibility for architecture choices.

Adoption requires *incentives*, not mandates. A golden path that saves a team 3 hours spreads organically. One that adds friction gets bypassed.

## Developer Experience vs. Real Outcomes

DX metrics are seductive but incomplete. Simple metrics:
- Onboarding time (days to first deploy)
- Service provisioning latency (request → running service)
- Documentation quality (findability, freshness)

These matter. But they can hide failure. A team might provision services in 30 minutes (great metric!) but spend 3 days debugging the template's assumptions (bad outcome). Measure both: *ease of first mile* AND *ease of debugging assumptions*.

More nuanced metric: **cognitive load reduction**. Does the platform let developers think about business logic, or do they spend cognitive effort on infrastructure? Observability tools, secret management, CI/CD—when they disappear into the platform, learning burden drops.

## Internal Developer Platforms: Portals and Registries

IDPs consolidate platform capabilities:

**Self-service portals** (Backstage, Port, Cortex) provide GUIs for provisioning. No CLI expertise required. Typical features:
- Service templates (auto-generate boilerplate from catalog)
- Secrets management (provision API keys, database credentials)
- Permissions workflows (request access, auto-approve for certain resources)
- Status dashboards (see deployed services, links to monitoring/logs)

**Registries** maintain centralized truth about systems: which services exist, who owns them, what resources they use. Enables:
- Auto-discovery (new devs see what exists)
- Governance (audit who owns what, compliance checks)
- DX tools (IDE plugins that jump from code to service docs)

Tools differ in trade-offs:
- **Backstage** (Spotify, open-source): Highly extensible, plugin-based. Teams build custom workflows. Requires engineering effort.
- **Port**: No-code portal, managed SaaS. Easier adoption, less customization.
- **Cortex**: Focused on governance and metrics. Less template-driven, more visibility into system state.

Choice depends on: Do you want a platform *platform* (Backstage, requires platform team discipline) or a turnkey *tool* (Port/Cortex, less control)?

## Team Topology Integration

Platform teams sit between stream teams and enabling teams:

**Stream teams** (product feature teams) are the primary customers. Platform reduces their infrastructure cognitive load so they focus on features.

**Enabling teams** (SRE, security, data engineering) inform platform design. Security writes pod policies; SRE writes observability templates. Good platforms codify enabling team knowledge so stream teams don't need to ask.

**Interaction modes**:
- **Facilitate** (default): Stream teams self-serve; platform provides templates and docs.
- **Tour of duty** (temporary): Platform engineers embed in stream teams for complex migrations (e.g., moving to new database tech).
- **X-as-a-service** (specialized): Platform provides managed services (databases, message queues, registries) that stream teams consume, not build.

## API Contracts Between Platform and Product

As platforms scale, team boundaries matter. **API contracts** define what platform provides and what stream teams promise:

Platform provides:
- Service template(s) with defined lifecycle (deployment, scaling, updates)
- Observability (metrics, logs, traces automatically collected)
- Secrets and configuration management
- Networking policies and service discovery

Stream teams promise:
- Use platform templates (or justify deviation)
- Populate required metadata (owner, runbook, on-call)
- Respect resource quotas and cost guardrails
- Participate in platform feedback/adoption cycles

When platforms lack clear contracts, scope creep kills them. Stream teams demand custom features; platform team drowns in one-offs. Clear contracts enforce boundaries.

## Adoption Patterns

Successful platform adoption follows predictable stages:

1. **Early stage**: One team uses platform. Platform team embeds heavily. MVP mindset—minimal features, gather feedback.

2. **Scaling**: 5-10 teams. Platform validates that templates work broadly. Create governance, investment in documentation. Early pain points drive feature prioritization.

3. **Maturity**: 50+ teams. Platform is *expected* infrastructure. Feedback becomes noise signal—filter ruthlessly. Focus shifts to preventing regression and evolving for new needs (serverless adoption, etc.).

Obstacles:
- **Competing solutions**: Teams build shadow IT when platform feels wrong. Don't fight it—learn what platform missed and adapt.
- **Adoption pressure**: Mandates breed resentment. Incentivize adoption: faster onboarding, fewer ops tickets, clearer compliance.
- **Maintenance burden**: Unless platform team has dedicated headcount, it becomes someone's side job and deteriorates. Platforms need real investment.

## Frame: Build for Developers, Not Architects

Great platforms are adopted because they solve problems, not because they're mandated. They make developers' lives tangibly easier. That requires respect for developer time, clear communication of benefits, and willingness to evolve.

See also: [platform-engineering.md](platform-engineering.md), [process-team-topologies.md](process-team-topologies.md), [devops-observability-patterns.md](devops-observability-patterns.md).