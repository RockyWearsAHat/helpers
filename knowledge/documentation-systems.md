# Documentation Systems — Docs-as-Code, Site Generators & API Documentation

## Overview

Documentation systems are the infrastructure and tooling that convert source documentation (Markdown, AsciiDoc, reStructuredText, OpenAPI specs) into user-facing websites, PDFs, and interactive tools. The shift to docs-as-code treats documentation like software: versioned, reviewed, built by CI/CD, and deployed alongside code.

Documentation systems span multiple concerns: static site generation (Docusaurus, MkDocs, VitePress), API documentation (OpenAPI, AsyncAPI), architectural decision capture (ADRs), and diagramming tools (Mermaid, PlantUML, D2).

## Docs-as-Code Philosophy

Docs-as-code means documentation lives in source control alongside code, versioned with the same commit, reviewed in the same PR workflow. Benefits:

- **Version alignment**: Documentation for v1.0 is tagged with v1.0 code. No stale docs linked to outdated APIs.
- **Single PR for code + docs**: A feature PR includes code changes and documentation changes, reviewed together.
- **Diffs and blame**: `git blame docs/api.md` shows who wrote which sentence and when. Changes are logged like code changes.
- **Automation**: CI/CD can check link validity, spelling, broken references, and even test code examples embedded in docs.
- **Reusability**: Documentation fragments (snippets, examples) can be included programmatically, staying in sync with source code.

Tools supporting docs-as-code: Markdown editors (VS Code), documentation linters (markdownlint, vale), link checkers, and site generators that turn `.md` files into websites.

## Static Site Generators

Static site generators (SSGs) compile documentation source files into a prebuilt website (HTML, CSS, JS), deployable as static files (GitHub Pages, CDN, S3). Advantages over server-rendered docs: faster, cheaper, CDN-friendly, no runtime.

### Generator Comparison

**Docusaurus** (Meta/Facebook): React-based, JavaScript/TypeScript. Purpose-built for documentation, strong MDX support (JSX in Markdown), built-in versioning (multiple doc versions side-by-side), search, dark mode. Opinionated structure but flexible. Large ecosystem. Best for: Teams wanting rich interactive docs, React components embedded in docs, versioning.

**MkDocs** (Python): Python-based, Markdown-first, minimal configuration. Lightweight, fast, simple plugin system. Material for MkDocs theme is popular and polished. Best for: Python projects, teams prioritizing simplicity, quick setup.

**VitePress** (Vue): Vue-powered, built on Vite (fast bundler). Minimal config, Vue components in Markdown, lightweight. Newer than Docusaurus/MkDocs but gaining adoption. Best for: Vue/JavaScript projects, teams wanting modern tooling and speed.

**Astro** (general-purpose) with **Starlight** (theme): Astro is a static site builder for any content; Starlight is a documentation theme for Astro. Both are young but emphasize performance. Astro supports multiple template languages (JSX, Vue, Svelte), framework flexibility, and islands architecture (JS only where needed). Best for: Teams building custom doc sites, integrating docs with product marketing, multiframework projects.

**Hugo** (Go): Extremely fast, mature, huge ecosystem of themes. Markdown-first but less documentation-specific than Docusaurus. Best for: Static blogs, documentation that doesn't need interactive components, teams valuing speed.

### Choosing a Generator

- **Want versioning built-in?** → Docusaurus
- **Python project, simplicity?** → MkDocs + Material
- **Modern tooling, speed?** → VitePress or Astro Starlight
- **Maximum flexibility?** → Hugo or Astro (build custom sites)

Each trades off ease-of-setup (Docusaurus, MkDocs) vs. control and customization (Hugo, Astro).

## Markdown Flavor & File Organization

Most SSGs use **GitHub Flavored Markdown** (GFM): tables, strikethrough, task lists, code highlighting. Some support **CommonMark** (more minimal) or **MDX** (Markdown + JSX in Docusaurus/Astro).

File structure typically mirrors site structure:
```
docs/
  index.md          → /
  guide/
    getting-started.md → /guide/getting-started
    troubleshooting.md → /guide/troubleshooting
  api/
    overview.md     → /api/overview
    authentication.md → /api/authentication
```

Frontmatter (YAML at the top of each `.md` file) stores metadata:
```yaml
---
title: Getting Started
sidebar_position: 1
---
```

## API Documentation

API documentation converts OpenAPI/Swagger specifications into interactive, browsable documentation. Two layers:

**Specification layer**: OpenAPI 3.0/3.1 (formerly Swagger) or AsyncAPI (for event-driven APIs) defines the API in machine-readable YAML/JSON. Specifies endpoints, parameters, responses, auth methods, examples.

**Documentation layer**: Tools render the specification as a website, often with a try-it-out console (send actual HTTP requests from the browser).

### OpenAPI & AsyncAPI

**OpenAPI** (OAS 3.0/3.1) is the standard for REST APIs. Defines:
- Paths (endpoints), methods (GET, POST, etc.), parameters
- Request/response schemas (JSON Schema)
- Authentication schemes (API key, OAuth 2.0, mTLS)
- Examples and default values

Typically written by hand (version control friendly) or auto-generated from code annotations (Python FastAPI, Go Echo, TypeScript NestJS).

**AsyncAPI** is the OpenAPI equivalent for Kafka, AMQP, WebSocket, and other async/event-driven APIs. Defines channels, messages, schemas, auth.

### Documentation Tools

**Swagger UI** (open-source): Standard interactive API documentation from OpenAPI specs. Includes a "Try It Out" console (execute requests from the browser).

**Redoc** (scalable-media): Beautiful, responsive OpenAPI documentation. Read-only (no Try It Out), emphasis on design. Good for public APIs.

**Stoplight** (SaaS): Full documentation and mocking platform. Builds on OpenAPI, includes design tools, publishing, versioning.

**API Blueprint**: Older alternative to OpenAPI. Uses a domain-specific language (simpler than YAML but less precise). Less common now but still used.

API documentation ideally includes:
- Overview and authentication flow
- Endpoint reference (try-it-out console)
- Error codes and troubleshooting
- Code examples (curl, Python, JavaScript)
- Webhook reference (if applicable)
- Rate limiting, quotas, billing

## Architecture Decision Records (ADRs)

ADRs document significant architectural decisions: technology choices, trade-offs, and the reasoning. Stored as Markdown files, versioned with code, often reviewed like RFCs.

Template (Nygard format):
```markdown
# [ADR-0001] Use PostgreSQL for Primary Datastore

## Status
Accepted

## Context
We need a primary relational database for user data. Candidates: PostgreSQL, MySQL, CockroachDB.

## Decision
We will use PostgreSQL.

## Consequences
- Benefit: Mature, ACID, advanced JSON support, strong community
- Cost: Requires separate operational expertise
- Risk: Limited horizontal scaling (mitigated by read replicas)
```

ADRs serve several purposes:
- **Onboarding**: New engineers understand past decisions without re-litigating them.
- **Audit trail**: Why was technology X chosen? When was it reconsidered?
- **Decision quality**: Force teams to articulate trade-offs, not just go with the first idea.

Best practices: Number them sequentially (ADR-0001, ADR-0002), mark status (Accepted, Rejected, Deprecated, Superseded), include consequences. Tools like adr-tools automate scaffolding.

## Diagramming Tools

Complex systems need visual representations. Three categories:

**Mermaid** (JavaScript-based): Text-based diagram syntax, renders in browser. Lightweight, GitHub-native (displayed in `.md` files), supports flowcharts, sequence diagrams, Gantt charts, state machines. No installation needed (works in GitHub markdown preview). Trade-off: limited styling control.

**PlantUML** (Java-based): Text syntax, generates PNG/SVG server-side or client-side. Supports UML (class, sequence, state), deployment, and custom diagrams. More powerful than Mermaid but requires a build step or server.

**D2** (Terrastruct): Modern text-to-diagram language. Clean syntax, powerful layout, creates architecture/system diagrams elegantly. Newer than Mermaid/PlantUML, gaining adoption.

Example (Mermaid flowchart in Markdown):
```
graph TD
  A[Request] --> B{Authenticated?}
  B -->|No| C[Return 401]
  B -->|Yes| D[Check Permission]
  D -->|No| E[Return 403]
  D -->|Yes| F[Execute Action]
```

When to use:
- **Embedded in docs**: Mermaid (no build step, GitHub-friendly)
- **Complex UML or deployment diagrams**: PlantUML
- **Architecture/system diagrams**: D2
- **Sequence diagrams and flowcharts**: Any of the three

## Documentation Site Deployment

Once built, documentation is static HTML deployed via:
- **GitHub Pages**: Free, built-in, simplest for public docs
- **Vercel/Netlify**: CI/CD integration, automatic preview deployments on PRs
- **AWS S3 + CloudFront**: Controlled, scalable, pay-per-use
- **Corporate CDN or on-prem**: For private/enterprise docs

Ideal workflow:
1. PR adds doc changes (Markdown, diagrams, OpenAPI specs)
2. CI builds the site, generates a preview deployment link
3. Reviewer checks preview, approves
4. Merge triggers production build and deployment
5. Users see updated docs within seconds

## Site Search

Good documentation requires search. Options:

**Client-side search**: Index pre-built at build time, shipped with the site (JSON). Small (<100 pages), no server needed. Tools: Algolia DocSearch (SaaS), lunr (library).

**Server-side search**: Database or search engine (Elasticsearch, Meilisearch). Scales to large sites, allows queries, but requires a server.

**Integrated search**: Many SSGs (Docusaurus, VitePress) have plugins for Algolia or Meilisearch. Docusaurus DocSearch is popular (free for open source).

## Documentation Quality Metrics

- **Freshness**: Docs updated within the last 30 days? Last major version covered?
- **Completeness**: All public APIs documented? Do code examples match current API?
- **Links**: Broken links, dead references?
- **Searchability**: Can users find what they want?
- **User feedback**: View counts, bounce rates, explicit feedback forms ("Was this helpful?").

## See Also

- **API Design** — Designing APIs with documentation in mind
- **Technical Writing** — Documentation structure and narrative
- **Developer Onboarding** — Documentation's role in onboarding
- **DevOps** — CI/CD for documentation deployment