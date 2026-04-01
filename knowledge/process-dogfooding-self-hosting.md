# Dogfooding (Self-Hosting) — When and Why Products Use Their Own Software

## Overview

**Dogfooding**, or self-hosting, means using your own product internally as your primary tool or service—eating your own dog food. It's not testing (quality assurance before shipping) but operational practice: your team runs on the software you're building, experiencing friction and bugs firsthand before customers do.

Unlike beta testing (external early adopters) or canary releases (small production rollout), dogfooding is mandatory, widespread internal use. The entire team—engineering, product, sales—uses the product daily.

The practice originated at Microsoft in 1988 when manager Paul Maritz challenged the LAN Manager team to "eat our own dogfood," and it became a defining principle across the tech industry. Today, dogfooding is practiced by teams building developer tools, infrastructure, productivity software, and internal platforms.

## History and Rationale

### Origin: Microsoft, 1988

Paul Maritz's email to Brian Valentine (test manager for LAN Manager) challenged the team: why should customers use software the engineers didn't trust enough to use themselves? The phrase metaphorically captures a profound principle: **if you don't believe in your product, why should anyone else?**

By the early 1990s, Microsoft dogfooding became policy. Entire divisions—Office, Windows, Exchange—ran on internal builds weeks or months before public release. This practice contributed to product quality and cultural alignment: engineers understood user pain points viscerally, not theoretically.

### Why It Works

**Direct feedback loop:** Engineers experience the same workflow friction, bugs, and performance issues as end users. Not through telemetry or user reports—through deliberate, daily use.

**Incentive alignment:** When the product blocks your work, you fix it immediately. There's no "marketing will manage the complaint" or "we'll address it in the next release." Your calendar friction is *your* problem.

**Design empathy:** Understanding the product's role in users' workflows beats design sprints and interviews. You discover that a feature you thought was easy is actually a source of constant friction.

**Shipping confidence:** If the product works for you—under your own workflows, with your own standards—it likely works for similar users (though not all).

## Modern Examples

### Microsoft and VS Code

The VS Code team extensively dogfoods the codebase itself. The editor is written in TypeScript; the team's workflow is: edit TypeScript → compile → test → debug in VS Code → commit. A compiler bug, missing feature, or performance regression directly impedes the team's productivity.

The team also runs on Copilot agent mode (AI-assisted development via agents) internally across 200+ repositories before shipping it to customers. This means thousands of agent-driven interactions per week, surfacing bugs that QA alone wouldn't catch.

**Result:** When VS Code ships a feature, the team has spent thousands of hours using it.

### Google and Chromium

Google's Chrome team dogfoods pre-release Chromium builds internally. The entire company's internal systems—Gmail, Docs, Analytics—are tested on development versions before public release. Engineers encounter performance regressions, compatibility issues, and security bugs before they reach the public.

### Apple and iOS/macOS

Apple engineers use pre-release iOS and macOS versions on personal devices weeks or months before public release. This provides both breadth (hundreds of different hardware combinations, use patterns) and depth (daily, real-world usage).

### Figma Tool Development

Figma's design tool team uses their own product to design and iterate on Figma itself. The very UI you're using as a Figma user is built *with* Figma. This creates a tight feedback loop: UX problems surface immediately; feature underutilization is obvious.

## Benefits

### 1. Bugs Surface Fast

When your tool is broken, you know within minutes. No waiting for user reports or monitoring dashboards.

```
Bug discovery speed:
- Internal dogfooding: < 1 hour (you hit it)
- External beta: 1-7 days (reports come in)
- Production: 1-30 days (support tickets escalate)
```

### 2. Design Empathy and UX Clarity

Using your product reveals design anti-patterns you rationalize away in meetings.

**Example:** A CI/CD platform might look "fast enough" in demos, but when you run it 50 times a day, 3-second wait times become intolerable. You prioritize optimization because you feel the pain.

### 3. Workflow Integration Understanding

You discover where your product fits (and doesn't fit) in users' workflows.

**Example:** A code review tool team that reviews code *within the tool* (vs. via email/Slack links) understands bottlenecks: context switching, notification delays, integration with IDEs.

### 4. Culture Alignment

Dogfooding reinforces a "product first" culture. Building for users becomes personal; it's not abstract.

### 5. Feature Prioritization

When the product blocks your work, you prioritize fixes ruthlessly. This prevents feature bloat and focuses effort on high-impact improvements.

## Challenges and Risks

### 1. Survivorship Bias

Your team is *not* representative of all users.

**Problem:** Your team is technically sophisticated, internal, high-bandwidth communication. You work around limitations; external users hit walls. You find workarounds; external users abandon the tool.

**Mitigating factor:** For B2B or developer tools, dogfooding teams are often closer to the target user than for consumer products (e.g., VS Code team ≈ VS Code users; but Microsoft internal tools ≈ Microsoft employees, not small businesses).

### 2. Stockholm Syndrome

Prolonged exposure to friction normalizes bad UX. You stop noticing problems because you've adapted.

**Example:** A performance issue that should take 30 seconds but takes 5 minutes *feels normal* after months of use. You rationalize: "I just make coffee while waiting." A new user experiences it as unacceptable.

**Mitigation:** Regular fresh perspectives (new team members, user feedback) catch normalized problems.

### 3. Pressure to Prioritize Internal Needs

Internal dogfooding can skew priorities toward your team's unique workflows, not broader customer needs.

**Problem:** If 80% of your users care about feature A and 20% (your team) care about feature B, dogfooding bias might prioritize B because you hit it daily.

**Mitigation:** Separate dogfooding feedback from customer feedback; weight them appropriately.

### 4. Slowed Development

If your team is running production builds of unreleased software, crashes and bugs can block development velocity.

**Problem:** A major regression crashes everyone's work; nothing ships that day while it's triaged and fixed.

**Mitigation:** Staged rollout; not 100% internal adoption immediately (e.g., 50% of team runs new version for 1 week before full rollout).

## Dogfooding vs. Related Practices

### Dogfooding vs. Beta Testing

| Aspect | Dogfooding | Beta Testing |
|--------|-----------|--------------|
| **Who** | Internal team | External early adopters |
| **Adoption** | Mandatory | Voluntary |
| **Scope** | Deep, daily use | Broad, varied use cases |
| **Feedback speed** | Immediate | 1-7 days |
| **Investment** | High (team time) | Moderate (external users' time) |
| **Bias** | Team-specific workflows | Diverse, representative |

**Complementary:** Dogfood internally first (catch obvious bugs); then beta test with external users (validate fit).

### Dogfooding vs. Canary Releases

| Aspect | Dogfooding | Canary Release |
|--------|-----------|----------------|
| **Timing** | Pre-release | Post-release (production) |
| **Scope** | Usually internal only | 1-5% external users |
| **Risk** | Limited (internal only) | Higher (affects external users) |
| **Feedback** | Fast, structured | Via monitoring, support tickets |
| **Action** | Fix or delay release | Rollback or continue rollout |

**Sequential:** Dogfood → get feedback → release → canary rollout → full rollout.

### Dogfooding vs. Dogfooding Hybrid

**Hybrid dogfooding:** Partner with friendly customers (e.g., a startup using your B2B product) to run pre-release versions. They're external (so less survivorship bias) but collaborative (so fast feedback).

```
Spectrum of validation:
Internal dogfooding (1 team, high bias)
  ↓
Hybrid dogfooding (partner, medium bias)
  ↓
Beta (external, low bias, slow feedback)
  ↓
Canary (production, live feedback, high risk)
  ↓
General availability (full rollout)
```

## Dogfooding in Different Contexts

### Developer Tools (VS Code, Rust, Go)

High-fidelity dogfooding: the tool is used *to build itself*.

- **VS Code:** Written in TypeScript, edited in VS Code.
- **Rust compiler:** Built with Rust; team runs latest nightly daily.
- **Go:** Team runs bleeding-edge Go on internal projects.

**Why it works:** Developers are sophisticated, feedback is structural (compilation errors, performance), and the tool's primary use case (language/editor development) is directly tested.

### Infrastructure (Kubernetes, Terraform)

Platform teams run their own tools to manage internal infrastructure.

- **Kubernetes:** Google/Cloud Native Computing Foundation teams run Kubernetes to orchestrate their own services.
- **Terraform:** HashiCorp teams manage infrastructure with Terraform.

**Why it works:** Infrastructure changes are high-impact, high-visibility (breaking internal systems means no deploys). Dogfooding surfaces architectural issues early.

### Productivity Software (Figma, Notion, Slack)

Harder to dogfood: Figma internally uses Figma to design Figma. Notion uses Notion for internal wikis. But not all workflows are fully supported by internal use.

**Mitigation:** Focus dogfooding on *core use cases* (design in Figma, documentation in Notion, team chat in Slack) and supplement with user interviews for edge cases.

### Internal Platforms

The strongest case for dogfooding: internal tools by and for a company's engineers.

- Platform engineering teams dogfood deployment infrastructure, observability, and CI/CD.
- Why it works: The team is identical to the user; pain is immediate and actionable.

## Organizational Adoption Pattern

### Stage 1: Skepticism

"We're too busy shipping to also use our own product." → "Let the QA team test it." → Risks slipping to production.

### Stage 2: Pilot Program

A subset of the team runs production builds. Feedback starts flowing.

```
Adoption timeline of internal build:
Week 1: 30% of team on new version
  → "Performance is slow, but OK"
Week 2: 50% of team
  → "Workflow is broken for me; I'm reverting"
Week 3: Fix prioritized, rolled back
  → "Now 100% on new version; working"
```

### Stage 3: Systemic Dogfooding

Dogfooding becomes policy and infrastructure.

- **New versions:** Run on 100% of team for < 1 week before external release.
- **Staging environment:** Mirrored production, team uses it daily.
- **Rollback SLA:** If it breaks, fix or rollback within 2 hours.

### Stage 4: Sustained Practice

Feedback loops integrate into standard practices:

- Team collects dogfooding bugs in a dedicated kanban board.
- P1 bugs (block work) fixed before next release.
- Quarterly retrospectives on dogfooding effectiveness.

## Metrics and Outcomes

### Quality Metrics

- **Bug escape rate:** Bugs found internally vs. escaped to production. Dogfooding teams report 30-50% reduction.
- **Time-to-fix:** Critical bugs fixed within hours (dogfood) vs. days (after customer report).
- **Regression detection:** Flaky tests, performance degradation caught before release.

### Team Metrics

- **Velocity:** May dip initially (using unstable software), stabilize after stabilization.
- **Morale:** Mixed; some teams feel productive (tool fits workflow); others frustrated (blocked by bugs).

### Customer Perception

- **Feature fit:** Better alignment with user desires (team knows what matters).
- **Quality reputation:** Early bugs prevented; releases more stable.

## Conclusion

Dogfooding is not required for all software. It's most effective for:

- **Developer tools:** Language compilers, editors, build systems (team = user).
- **Infrastructure:** Kubernetes, Terraform, CI/CD (teams manage their own systems).
- **Internal platforms:** Tools built for a specific org's engineers (tight feedback loop).

Dogfooding is challenging or ineffective for:

- **Consumer products:** Internal team ≠ diverse users.
- **Niche markets:** Your team may not represent target customers.
- **Hardware/physical products:** Can't easily dogfood manufacturing or supply chain changes.

When dogfooding works, it's powerful: engineers experience user pain directly, culture aligns around product quality, and feedback loops compress. But it's not a replacement for user research, beta testing, or diverse perspectives. It's one tool in a quality toolkit, most effective when combined with broad user feedback and monitoring.