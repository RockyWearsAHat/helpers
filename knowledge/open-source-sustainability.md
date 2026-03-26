# Open Source Sustainability — Funding, Governance, Licensing & Maintainer Health

## The Sustainability Crisis

Open source powers the internet. Yet most maintainers are unpaid volunteers. Maintainers experience burnout at rates >2x the general developer population. Popular projects (npm, Babel, Node.js dependencies) provide critical infrastructure earning billions in value to enterprises while maintainers earn nothing.

Sustainability is a structural problem, not individual failure. A volunteer model works for hobby projects. For infrastructure that enterprises depend on, expecting unpaid care is exploitation.

## Funding Models

Viable funding models have emerged. Most sustainable projects combine multiple sources.

**GitHub Sponsors:**

- Users sponsor directly on GitHub. Simple one-click setup.
- Recurring monthly sponsorships ($5-$500/month typical).
- GitHub matches first-dollar donations up to $5,000 in the first year (varies by region).
- Low barrier; works for hobbyists and small teams.
- Trade-off: Income is inconsistent; no guarantee of continued funding.

**Sponsorship and Donations (Patreon, Open Collective, Liberapay):**

- Open Collective: Fund projects with transparent accounting (financials public). Low friction. Includes permissioning for multi-maintainer teams.
- Patreon: Creator-focused; for solo maintainers building a community (podcast, course, regular releases).
- Liberapay: Privacy-focused recurring donations.
- Trade-off: Requires active community building; funds are often modest ($500-$5,000/month for healthy projects).

**Commercial Support and Services:**

- Offer training, consulting, or priority support for enterprises.
- Example: Gatsby (JAMstack framework) offers enterprise support packages ($20K+/year).
- Example: Elastic (formerly Elastic{search}): Open source search + analysis, commercial support and hosted SaaS.
- Requires balancing: Open source must remain genuinely free, not artificially crippled.

**Dual Licensing (Open Core):**

- Core project is open source (MIT, GPL). Premium features are proprietary.
- Example: Supabase (Firebase alternative) is open source; self-hosting is free. Cloud-hosted version is commercial.
- Example: HashiCorp (Terraform, Vault): Open source core + cloud services (Terraform Cloud, Vault Cloud).
- Trade-off: Community views this skeptically if the open source gets neglected. Requires clear delineation between core and premium.

**Sponsorship by Enterprises:**

- Companies fund development because they depend on the project.
- Example: React (Facebook/Meta funds), TypeScript (Microsoft funds), Linux (Red Hat, Canonical, others fund).
- Example: The Kubernetes project: Google, Red Hat, Amazon, Microsoft collectively fund CNCF stewardship.
- Most sustainable but requires the project to become critical infrastructure.

**Grants and Fellowships:**

- nonprofits (Linux Foundation, Mozilla Foundation, Python Software Foundation) award grants.
- NLnet (European foundation) grants for open source (typically €10K-€50K).
- Rarely provide full-time funding but useful for specific initiatives (accessibility work, security audit).

**Hosting + Managed Services:**

- Sell managed/hosted variants of your software as a service.
- Example: Automattic (WordPress): Open source WordPress software; sells hosting and plugins.
- Example: Figma (not open source but model applies): Open the platform; sell cloud services and plugins.
- Trade-off: Requires building and operating infrastructure. High upfront cost.

## Maintainer Burnout: Recognition and Prevention

Burnout is a *structural* problem with individual consequences. It's not fixed by self-care alone.

**Symptoms and cycle:**

- Early: enthusiasm, manageable volume, good community.
- Middle: volume grows, issues accumulate, responses slow. Maintainer sacrifices personal time.
- Late: resentment ("I don't owe you this"), boundaries collapse, harassment increases.
- Crisis: abandonment or hostile takeover (unmaintained project acquires new stewards who bring problematic values).

**Prevention:**

- **Structured involvement**: Part-time or full-time dedicated role (funded position).
- **Shared stewardship**: Multiple maintainers rotate responsibilities. No single point of failure.
- **Reset boundaries**: Clearly communicate: "I respond to issues within 5 business days", "No support outside these channels", "No calls without scheduled appointment".
- **Explicit license to say no**: Refusing a feature request and explaining why is not rude; it's professional.
- **Community moderation**: Enforce code of conduct. Toxic contributors harm more than they help.
- **Paid leave and sabbaticals**: Maintainers working at 60%+ capacity for years need breaks. Build this into funding.

**Organizational responsibility:**

- If your org depends on open source, fund maintainers proportionally to your use.
- If you're a large cloud provider using an open source project, pay for the maintenance.
- Platform companies (GitHub, npm, PyPI) have responsibility to share revenue or fund critical infrastructure.

## Governance Models

Governance is how decisions get made: what features land, what contributors get commit rights, who speaks for the project.

**BDFL (Benevolent Dictator For Life):**

- One person has final say. Examples: Python (Guido van Rossum, now retired; Python now uses governance council), Linux (Linus Torvalds).
- Trade-off: Fast decisions, clear vision. Risk: Single point of failure, successor unclear, can become unbenevolent.

**Meritocracy / Contribution-Based:**

- Commit rights based on contributions and community trust.
- Examples: many successful open source projects (Kubernetes, Node.js started here).
- Trade-off: Clear incentives for contributors. Risk: Bias (who gets recognized?), accumulation of power by early contributors who no longer maintain.

**Foundation/Board Governance:**

- Project governed by board of elected/appointed members.
- Examples: Linux Foundation (governed by members), Apache Software Foundation (projects follow ASF governance).
- Trade-off: Legitimacy, diverse perspectives. Risk: Bureaucracy, slower decisions, political dynamics.

**Steering Committees / RFC Process:**

- Major decisions (breaking changes, licensing, roadmap) decided by committee or open RFC (Request for Comments) process.
- Examples: Rust (RFC process), Node.js (TSC — Technical Steering Committee).
- Trade-off: Transparent decision-making, community input. Risk: Slow, contentious debates.

**No Governance (Chaos Model):**

- Whoever contributes most controls the current direction.
- Trade-off: Fast early development. Risk: Unsustainable at scale, fork risk, inconsistent values.

## Licensing: Philosophy and Trade-offs

License choice determines what others can do with your code. No license = all rights reserved (no one can legally use it). Choose intentionally.

**Permissive licenses (MIT, Apache 2.0, BSD):**

- Others can use, modify, distribute, even commercially, with minimal restrictions.
- MIT: Simplest. Include license + copyright, then do anything.
- Apache 2.0: Includes patent protection. Longer but more explicit. Requires inclusion of CHANGES file.
- BSD 3-Clause: Includes non-endorsement clause.
- Trade-off: Code can be used by anyone, including proprietary software, with no profit-sharing.

**Copyleft licenses (GPL, AGPL, MPL):**

- Others can use, modify, distribute. *If they distribute*, they must share improvements under the same license.
- GPL v3: Strong copyleft. If you distribute a program using GPL code, you must open-source your derivative.
- AGPL v3: Network copyleft. If you run the program as a service (SaaS), you must open-source even if you don't distribute.
- MPL (Mozilla Public License): Weak copyleft. Derivatives of the file must be shared; linked code does not.
- Trade-off: Ensures improvements stay open. Risk: Enterprises reject GPL (risk of forced open-sourcing). Weaker copyleft (MPL) is a compromise.

**SSPL (Server Side Public License):**

- Created by MongoDB. Copyleft, but specifically targets SaaS companies: If you run the software as a service, you must open-source the service itself (infrastructure changes, tooling, etc.).
- Trade-off: Prevents commercial SaaS clones of your open source. Risk: Rejected by open source community (not OSI-approved) and enterprises (unclear legal standing). Mostly failed adoption.

**Dual licensing:**

- Offer a choice: Open under GPL for open source projects; commercial license for proprietary use.
- Example: Qt (GUI framework): GPL for open source, commercial license for apps.
- Trade-off: Funds development; enterprises can use without open-sourcing. Risk: Community views as exploitative if open source is de-facto unavailable.

**Practical recommendation:**

- **Library or utility for general use**: MIT or Apache 2.0 (permissive, adopted widely).
- **Infrastructure or SaaS substitute**: AGPL (prevent SaaS clones) or commercial support model.
- **Philosophical preference for open source**: GPL v3 (copyleft backs open source ecosystem).
- **Uncertainty**: MIT is safe; most enterprises accept it.

## Community Building and Contributor Onboarding

A project with good documentation and clear contribution process attracts more contributors.

**Signals of a healthy community:**

- Responsive maintainers (issues acknowledged within days, not months).
- Clear CONTRIBUTING.md with steps, style guide, what to expect.
- Good-first-issue labels and small, approachable bugs.
- Public roadmap or issue tracking.
- Regular releases and changelog.

**Onboarding:**

1. **CONTRIBUTING.md**: How to set up development environment (clear instructions). What the development workflow is (fork, branch, PR, review). What code standards apply (linting, formatting, tests, docs).
2. **Development guide**: `npm install`, `npm test`, `npm run build`. Commands should work on first try. If they don't, it's a bug in the docs.
3. **Good first issues**: Label small, self-contained tasks. Pair them with a mentor (assign a maintainer to support).
4. **Community space**: Discord, Slack, forum for async discussion. Mailing list for async updates. GitHub Discussions for public Q&A.
5. **Recognition**: Contributor list in README. Credit in release notes. Small gestures of appreciation matter.

## Security Responsibility

Open source maintainers have responsibility to handle security issues responsibly.

**Practices:**

- **Security.md or SECURITY.txt**: Document how to report security issues privately (email or GitHub Security Advisory).
- **Private handling**: Don't discuss on public issue trackers until fix is released.
- **Coordinated disclosure**: Give enterprises time to update before public announcement (typically 90 days).
- **Patch releases quickly**: Security fixes shouldn't wait for the next feature release.
- **Audit high-risk code**: Crypto, auth, privilege handling warrant code review by security experts.
- **Transparency**: If you can't maintain the project, say so. Unmanaged projects are security risks. Transfer ownership or archive.

**Supply chain risk:**

- Enterprises now track open source dependencies for security. Unmaintained projects in your dependency tree are liabilities.
- Consider: If I stopped maintaining this tomorrow, what would break downstream? If much, fund sustainability or consider sunsetting.

## Trends and Future

**Platform responsibility:**

- npm, PyPI, Crates.io (registries) have become critical infrastructure. They're beginning to fund or support high-risk maintenance.
- GitHub Sponsors is growing; GitHub (Microsoft) is investing in sustainability.

**Ecosystem maturation:**

- Open source is no longer fringe. Enterprises now invest in maintaining projects they depend on.
- Funding models are diversifying. Not all successful projects are 100% community-funded.

**Licensing evolution:**

- Copyleft (especially GPL) adoption declining in favor of permissive + commercial support model.
- New terms like SSPL attempted to capture value but faced resistance. More nuanced approaches (like Elastic's approach) are emerging.

## See Also

- [Open Source Practices — Contributing, Licensing, and Community](open-source-practices.md)