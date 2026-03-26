# License Drama in Open Source — Shifting the Commons

## The Core Tension

Open source licensing started with a simple bargain: authors gave away code freely, and users had freedom to use, modify, and distribute it. But this bargain broke down when large companies began monetizing open source projects without contributing back, and when companies that created infrastructure grew dependent on it yet offered no compensation. The result: a series of high-profile license changes that revealed fundamental disagreements about what open source means.

## The Business Source License (BSL) Era

### HashiCorp's Shift (August 2023)

HashiCorp, the company behind Terraform, Consul, and Vault, announced it was relicensing its most popular products under the Business Source License (BSL) 1.1. This was a watershed moment because:

- **BSL is not open source by OSSD standards**: Code is source-available but with restrictions. Specifically, BSL prohibits using the software for competitive purposes (offering it as a managed service) for a defined period, then converts to a standard open license after a specified year.
- **The stated reason was clear**: AWS, Google Cloud, and other hyperscalers were offering Terraform as a managed service, capturing the value HashiCorp created, without contributing back.
- **The community backlash was immediate**: Users felt betrayed. Forks appeared almost instantly (OpenTF/OpenTofu created a fork under the Mozilla Public License).

**Technical insight**: Terraform is declarative infrastructure-as-code that compiles to vendor-agnostic configuration. Its usefulness to competitors is precisely because it abstracts away the infrastructure layer. From HashiCorp's perspective, competitors were selling its own abstraction back to users. From the community's perspective, HashiCorp was reneging on the open source promise.

### Why BSL?

BSL was designed by Couchbase and later adopted by others as a middle ground:

- It preserves source availability (auditable, forkable code).
- It restricts commercial exploitation by competitors for a limited time.
- After the license conversion date (often 2-4 years), code converts to a permissive license (usually MIT or Apache 2.0).
- The restriction is specifically on "competitive use"—your own internal use, bug fixes, and modifications are allowed.

The theory: small companies can use the software for free, solo developers can use it freely, enterprises using it internally can use it freely, but if you try to package it as a commercial service to compete with the original vendor, you must negotiate a commercial license.

## The Elastic License / SSPL Wars

### Elastic's License Flip (February 2021)

Elasticsearch, owned by Elastic, changed its license from the Affero GPL (AGPL) to the Elastic License + Server-Side Public License (SSPL). Key details:

- **The Elastic License** is a proprietary license that restricts:
  - Public SaaS offerings (you can't run it as a managed service)
  - Hiding the source code
  - Bypassing license restrictions
- **SSPL** is a copyleft-adjacent license that requires:
  - If you run the software as a service, you must provide the complete source code (and build scripts, scripts to deploy updates, etc.) for offering that service publicly.

**Why the change**: AWS Elasticsearch is a direct competitor to Elastic's managed Elasticsearch service. AWS benefited (and still benefits) from Elasticsearch's code improvements, community, and reputation—but Elastic saw no revenue from this. The license change was explicitly designed to prevent managed offerings without a commercial agreement.

### MongoDB's SSPL Experiment (September 2018)

MongoDB tried a similar move with SSPL. Community reaction included:

- **Debian, Ubuntu, and other distributions rejecting it**: The SSPL was deemed too restrictive and incompatible with copyleft principles (it's too onerous to require full service source code disclosure).
- **The Open Source Initiative declined to recognize SSPL as an OSI-approved license**.
- **A fork appeared**: Percona forked MongoDB under the AGPL to continue as genuine open source.

The key difference: Elasticsearch did a dual license (Elastic License + SSPL), giving code users a choice. MongoDB tried pure SSPL initially, which created stronger community resistance.

## Redis: The Governance License (March 2024)

Redis Labs relicensed Redis under a dual license:

- Redis Source Available License (RSAL) + Commons Clause
- Later: switched to the Redis Source Available License (RSAL) + Redis Functional Source License (FSL)

The pattern: restrictive source-available licenses for managed service producers, permissive for others. Redis Labs (now part of Valkey governance discussions) was trying to prevent AWS ElastiCache from capturing value.

## The Open Source Definition Wars

These license changes triggered meta-debates about what "open source" means:

### The OSSD Perspective

The Open Source Definition (maintained by the Open Source Initiative) requires that open source software:

1. Be distributed with source code
2. Allow modifications and derived works
3. Impose no restrictions on fields of endeavor (can't forbid commercial use)
4. Not discriminate against any person or group
5. License must not restrict other software
6. Must be license-neutral (not specific to one distribution)

**Under OSSD, BSL, SSPL, Elastic License, and RSAL all fail check #3**: they restrict commercial use or specific use cases (SaaS offering).

### The Counterargument

The vendors behind these licenses argue:

- **OSSD is outdated**: it was written for a pre-SaaS world. The original premise: software costs money to copy. In a SaaS world, copying is free—the value is in the infrastructure and service, not the software.
- **Predatory capture**: The definition was written when IBM was the fear (proprietary monopoly). Now the fear is Google/Amazon using OSSD software to build competing services without contributing back and without licensing friction.
- **Open source is not altruism**: It's a business model. Companies should be able to protect their revenue model.

### The Copyleft vs. Permissive Divide Reopens

These debates resurfaced the old tension:

- **Copyleft advocates** (GPL, AGPL, SSPL) argue: "If you use our work, you must share yours."
- **Permissive advocates** (MIT, Apache 2.0) argue: "Use it however you want, even commercially, no strings attached."

The license wars revealed a third position:

- **Governance advocates**: "Open source was always about community control. If we lose control to hyperscalers, open source loses its meaning, regardless of legal licenses."

## Technical Consequences

These license changes triggered infrastructure decisions:

- **Distributed databases**: Projects like Postgres and MySQL stayed permissively licensed (GPL for MySQL, PostgreSQL License). Companies like Neon and Supabase built managed services on top, paying Postgres developers.
- **Terraform alternatives**: OpenTofu (fork) and other infrastructure-as-code tools gained adoption specifically to avoid BSL.
- **Elasticsearch alternatives**: Projects like Meilisearch and Zinc gained interest.
- **Redis alternatives**: Valkey (Redis fork under BSL) and RESP-compatible databases promoted as alternatives.

## The Embedded CS Insight

These license dramas expose a fundamental asymmetry in open source: **source code is not the scarce resource in SaaS economies**.

Under traditional software economics (shrink-wrap, on-premise):

- Protecting source code makes sense: if a competitor has your code, they can clone your product.
- Permissive licensing works: you give away code, you monetize through services (support, training, consulting), or directly via paid licenses.

Under SaaS economics:

- Source code is published anyway (in SaaS, the service is the product, not the code).
- The scarce resource is: deployment infrastructure, data, uptime, and the community that builds around the project.
- Permissive licensing means competitors can capture this value with no obligation to contribute back.

The license battles are really about control: who gets to monetize the infrastructure and community that form around a project?

## See Also

- software-licensing.md — detailed breakdown of specific licenses and their compatibility
- open-source-sustainability.md — funding models and governance structures
- popculture-open-source-burnout.md — the human context driving these tensions