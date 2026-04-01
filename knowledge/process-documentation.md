# Software Documentation — APIs, Architecture, Diátaxis & Docs-as-Code

## The Documentation Crisis

Most software documentation is organized by accident: whoever had time wrote it, wherever seemed convenient. Teams struggle with fragmentation: API docs live in Swagger; architecture lives in Confluence; runbooks scatter across wikis. Users hunt for answers in four places, find answers in three of them incompletely.

The problem isn't volume—it's *fragmentation of purpose*. Different stakeholders need different documentation types. A new developer needs a tutorial; an operator needs a reference; an architect needs explanation. Mixing these into one document confuses everyone.

## Diátaxis: The Four Documentation Needs

Diátaxis (Daniele Procida) proposes that complete documentation requires **four types**, each serving a different need:

| Type | Reader Need | Characteristic | Audience |
|------|-------------|-----------------|----------|
| **Tutorial** | Learn by doing | Lessons with steps; doesn't require prior knowledge | Beginners |
| **How-to guide** | Solve specific problem | Task-oriented; assumes existing knowledge | Practitioners |
| **Reference** | Look up facts | Complete, structured, searchable; no narrative | Developers in flow |
| **Explanation** | Understand concepts | Discusses rationale, trade-offs, history; not a task | Architects, maintainers |

The framework's power: It rejects the idea that one document serves all needs. A 100-page guide tries to be tutorial *and* reference and fails at both.

**Organization principle**: Separate physical locations for each type. Tutorial section doesn't link to reference (reader gets lost); reference doesn't include narrative (bloats the index). Cross-reference by *purpose*, not proximity.

## API Documentation: Specifications + Human Text

Modern APIs require two layers:

**Specification layer** (machine-readable): OpenAPI (REST) and AsyncAPI (async/event-driven) define every endpoint, parameter, response, and error in structured YAML/JSON. Tools auto-generate interactive docs (Swagger UI), client SDKs, and contract tests. Single source of truth.

OpenAPI covers REST semantics (HTTP methods, status codes, headers). AsyncAPI covers async messaging (channels, message payloads, broker protocols like AMQP, Kafka). The two don't compete—systems use both. An API gateway might have OpenAPI spec; its backend event streams use AsyncAPI. Tools like Fern and ReDoc render specs into human-friendly docs automatically.

**Human layer** (narrative): Specifications alone are incomplete. They explain *what* each endpoint does, not *why* or *when to use it*. Developers need:
- Conceptual overview (authentication flow, rate limits, pagination philosophy)
- Tutorials (walk through authentication → basic call → error handling)
- How-to guides (profile API calls, handle timeouts, optimize pagination)
- Explanation of design decisions (why it's REST vs. GraphQL, why pagination isn't cursor-based)

Tools like Swagger UI overlay specs with human text. The best practice: Keep spec as specification; add narrative separately, referencing the spec.

## C4 Model: Multi-Level Architecture Diagrams

C4 (Simon Brown) solves the architecture diagram problem. Most architecture diagrams are incomprehensible messes of boxes and arrows at random abstraction levels.

C4 provides four levels:

1. **Context**: System as a black box, external users, adjacent systems. One page, bird's-eye view. Who uses this system? What systems does it integrate with?

2. **Container**: Major internal components (web app, API server, database, message queue). Shows deployment topology and data flow between containers.

3. **Component**: Internals of one container (controllers, services, repositories). Useful for onboarding engineers to a specific service.

4. **Code**: Class/function relationships. Usually auto-generated (UML tools). Least useful for architecture communication.

Each level targets a specific audience. Executives read Context; devops reads Container; feature team reads Components. Avoid mixing levels in one diagram.

Notation is deliberately simple (boxes, arrows, labels). This encourages creation—non-architects can sketch C4 diagrams. Tools like Structurizr, Miro, and even draw.io support C4 templates.

## README-Driven Development

README-driven development (Tom Preston-Werner, 2010) inverts typical documentation timing: Write the README *before* code, describing intended usage, API shape, and examples.

**Mechanics**: README becomes API specification. Before writing code, ask: What does this library do for users? How do they install it? What's the hello-world example? What problems does it solve?

Writing the README forces clarification: If the API is hard to describe, it's hard to use. If the examples seem contrived, the use case is weak. The README is a usability probe *before* implementation, not a retrospective artifact.

Disadvantage: The README can become stale (like all documentation). Maintenance discipline required. Advantage: Clear intention from day one, fewer API surprises mid-implementation.

## Documentation-as-Code

Documentation-as-code treats docs like source: version-controlled, reviewed in pull requests, tested for validity, deployed via CI/CD.

**Mechanics**:
- Docs written in markdown or RST (not Word docs)
- Docs live in the repo alongside code (or linked submodule)
- Pull requests can update docs + code together
- CI validates docs (broken links, outdated API references against schema)
- Deploys publish to static site (Sphinx, MkDocs, Netlify)

**Advantages**: Docs stay in sync with code (both change in same PR). Discrepancies caught by CI. Anyone can fix typos. History is auditable.

**Disadvantage**: Assumes team comfort with terminals and markdown. Non-technical writers struggle. Some teams use hybrid: prose in docs-as-code, images/diagrams in wikis (inherent friction helps them stay curated).

## Trade-offs and Constraints

- **Comprehensiveness vs. maintenance**: Every doc feature must be maintained. Start minimal; add as patterns emerge.
- **Centralization vs. proximity**: Centralized doc site aids discovery; docs near code (docstrings, comments) aid maintenance. Many projects use both.
- **Automation vs. accuracy**: Auto-generated docs from code scale but can be incomplete. Manual narrative is laborious but more nuanced.
- **Specification vs. narrative**: Machine-readable specs (OpenAPI) are maintainable but incomplete. Human narrative explains why; specs explain what. Neither alone suffices.

## Frame: Audience and Purpose First

The question isn't "how much to document" but "which audiences and which needs?" Diátaxis helps. Then choose formats (specs, C4 diagrams, markdown, videos) that match.

See also: [process-technical-writing.md](process-technical-writing.md), [api-design.md](api-design.md), [architecture-patterns.md](architecture-patterns.md).