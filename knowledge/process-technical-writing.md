# Technical Writing — Documentation, ADRs, RFCs & Design Documents

## The Role of Technical Writing in Engineering

Technical writing serves as the connective tissue between engineering teams, their users, and their future selves. Codebases outlive the tenure of their original authors; documentation captures the reasoning, context, and intent that code alone cannot convey. The investment in writing pays compound returns — every hour spent documenting a design decision saves multiples of that time when the next engineer encounters the same system boundary.

The value proposition shifts depending on organizational scale. In small teams where everyone holds shared context, heavy documentation can feel like overhead. In larger organizations where knowledge silos form naturally, written artifacts become the primary vehicle for cross-team understanding. The challenge is calibrating the investment to the context rather than applying a universal prescription.

Writing also functions as a thinking tool. The discipline of explaining a design in prose frequently reveals gaps, contradictions, or unstated assumptions that remained invisible during implementation. Many teams report that the act of writing an RFC or design document improved the design itself, independent of any feedback received.

## The Audience Spectrum

Technical documents serve audiences with fundamentally different needs, expertise levels, and goals. The same system might require:

| Audience              | Primary Need               | Typical Format                                 |
| --------------------- | -------------------------- | ---------------------------------------------- |
| New team members      | Orientation, mental models | Getting-started guides, architecture overviews |
| Day-to-day developers | Task completion, API usage | How-to guides, API reference                   |
| Operators / SREs      | Runbooks, failure modes    | Operational docs, troubleshooting trees        |
| External integrators  | Contract clarity, examples | API docs, SDKs, tutorials                      |
| Future maintainers    | Rationale, trade-offs      | ADRs, design documents                         |
| Decision makers       | Impact, scope, risk        | RFCs, proposals, one-pagers                    |

A common failure mode is writing documentation that serves the author's mental model rather than the reader's needs. The author knows the system deeply and unconsciously fills gaps; the reader encounters those gaps as confusion. Effective technical writing requires empathy for what the reader does _not_ know.

Progressive disclosure — layering information from high-level overview to implementation detail — allows documents to serve multiple audiences within a single artifact. A design document might open with a one-paragraph summary, followed by context and motivation, then detailed design, then appendices with data or calculations.

## Architecture Decision Records (ADRs)

ADRs capture the _why_ behind architectural decisions — the context in which a choice was made, the alternatives considered, and the trade-offs accepted. They address a persistent problem in software engineering: code shows _what_ was built, commit history shows _when_, but neither reliably records _why one approach was chosen over another_.

### Typical ADR Structure

| Section      | Purpose                                                                                            |
| ------------ | -------------------------------------------------------------------------------------------------- |
| Title        | Short descriptor with a sequential number (e.g., "ADR-0042: Use event sourcing for order history") |
| Status       | Proposed, Accepted, Deprecated, Superseded                                                         |
| Context      | The situation, constraints, and forces at play when the decision was made                          |
| Decision     | The choice that was made                                                                           |
| Consequences | The expected outcomes — both positive and negative                                                 |

Some teams extend this with sections for alternatives considered, participants in the decision, and links to related ADRs.

### What Makes ADRs Effective

The context section carries the most long-term value. Recording that "we chose PostgreSQL" is marginally useful; recording that "we chose PostgreSQL because our team had deep operational experience with it, our query patterns were primarily relational, and the managed offering on our cloud provider met our availability requirements" gives future engineers the information they need to evaluate whether the decision still holds.

ADRs are immutable records. When a decision is reversed or superseded, the original ADR is marked as such and a new ADR records the new decision with its own context. This creates an archaeological record of how the system's design evolved and why.

The discipline of writing ADRs also creates a forcing function for deliberate decision-making. Teams that adopt ADRs often report that the practice surfaces disagreements earlier and produces more thoughtful architecture, because the act of writing down alternatives and trade-offs demands explicit reasoning rather than implicit consensus.

### Adoption Considerations

- ADRs work best when they capture decisions of lasting consequence — database choices, API styles, authentication mechanisms — not every implementation detail
- Lightweight formats reduce the activation energy for writing; heavyweight templates create friction that leads to abandonment
- Storing ADRs alongside the code (e.g., in a `docs/adr/` directory) keeps them discoverable and version-controlled
- The biggest risk is staleness — ADRs that no one reads or updates become misleading artifacts

## Request for Comments (RFCs)

The RFC process (borrowed from internet standards culture) structures collaborative design through written proposals. An engineer writes a document proposing a change, design, or new system, and distributes it for feedback before implementation begins. The process trades up-front writing time for reduced rework, broader input, and documented rationale.

### The RFC Lifecycle

1. **Draft** — The author writes the proposal, often iterating with a small group
2. **Review** — The document is shared broadly; reviewers leave comments, questions, and alternatives
3. **Discussion** — Open questions are resolved through threaded discussion or synchronous meetings
4. **Decision** — The proposal is accepted (possibly with modifications), rejected, or deferred
5. **Archive** — The final document is preserved as a record of the design and the discourse that shaped it

### RFC Design Considerations

RFCs serve multiple purposes simultaneously: they communicate a proposed design, they solicit feedback, they document the decision for posterity, and they distribute knowledge across the team. Tension between these purposes is natural — a document optimized for soliciting feedback (which highlights open questions and alternatives) looks different from one optimized for posterity (which presents the final design cleanly).

The scope of who reviews an RFC varies by organization. Some teams require broad review from all engineers; others scope review to affected teams plus opt-in participants. Broad review increases knowledge sharing but can create bottlenecks and dilute feedback quality.

A common challenge is the "silent approval" problem — reviewers who say nothing may genuinely approve, or they may not have read the document. Explicit approval mechanisms (sign-offs, thumbs-up, "LGTM" comments) help distinguish affirmative approval from passive non-objection.

## Design Documents

Design documents describe a proposed system, feature, or significant change with enough detail to evaluate feasibility, identify risks, and guide implementation. They sit between the strategic level of an RFC and the tactical level of a task description.

### Common Structure

| Section                 | Content                                                            |
| ----------------------- | ------------------------------------------------------------------ |
| Overview / Summary      | One-paragraph description accessible to anyone in the organization |
| Context and Motivation  | Why this work matters now; what problem it solves                  |
| Goals                   | What success looks like; measurable where possible                 |
| Non-Goals               | What this design explicitly does NOT address                       |
| Proposed Design         | The technical approach in detail                                   |
| Alternatives Considered | Other approaches and why they were not chosen                      |
| Risks and Mitigations   | What could go wrong; how the design accounts for it                |
| Open Questions          | Unresolved issues where input is sought                            |
| Timeline / Milestones   | Rough phasing, if applicable                                       |

### The Non-Goals Section

The non-goals section is among the most valuable parts of a design document. By explicitly stating what the design does _not_ attempt to do, it:

- Prevents scope creep during implementation by providing a reference point for "that's out of scope"
- Signals to reviewers which concerns are deliberately deferred vs accidentally overlooked
- Manages expectations across stakeholders about what the delivered system will and will not do

Effective non-goals are specific and explain the reasoning. "Performance optimization is a non-goal" is less useful than "We are not optimizing for sub-millisecond latency in this phase; the expected p99 of 50ms is acceptable for the current user base, and we will revisit if traffic projections materialize."

## README as the Project's Front Door

The README is often the first document a new contributor, user, or evaluator encounters. It serves as a routing document — helping the reader quickly determine what the project does, whether it's relevant to them, and where to go next.

Effective READMEs typically address:

- **What** — A concise description of what the project does and who it's for
- **Why** — The problem it solves or the value it provides
- **How** — Quick start instructions to get the reader productive
- **Where** — Links to deeper documentation, contributing guides, and support channels

The tension in README design is between completeness and scannability. A README that contains every detail becomes a wall of text that no one reads. A README that's too sparse fails to orient the reader. Many projects resolve this by keeping the README focused on orientation and linking to more detailed documents.

## API Documentation

API documentation encompasses several distinct content types, each serving a different reader need:

| Type         | Purpose                                                           | Example                        |
| ------------ | ----------------------------------------------------------------- | ------------------------------ |
| Reference    | Exhaustive catalog of endpoints, parameters, types, return values | Swagger/OpenAPI spec, Javadoc  |
| Tutorial     | Guided learning experience building toward a goal                 | "Build your first integration" |
| How-to Guide | Task-oriented steps for a specific scenario                       | "Authenticate with OAuth 2.0"  |
| Conceptual   | Explanation of underlying models, patterns, architecture          | "Understanding rate limiting"  |

### Reference Documentation

Reference docs aim for completeness and precision. Every public API surface should be documented with its parameters, return types, error cases, and behavioral notes. Generated documentation (from code comments, OpenAPI specs, type definitions) ensures reference docs stay synchronized with the implementation.

However, reference docs alone are insufficient. Knowing that `POST /orders` accepts a `lineItems` array tells the reader the shape of the API but not the workflow for creating an order, handling partial failures, or integrating with the payment system.

### The Completeness-Maintenance Trade-off

More comprehensive documentation provides more value — until it falls out of sync with the code, at which point it becomes actively harmful. Misleading documentation is worse than no documentation, because readers trust it and waste time pursuing incorrect approaches.

Strategies for managing this include generating docs from code, testing code examples as part of CI, and establishing documentation review as part of the change process.

## The Diátaxis Framework

Diátaxis (from Greek: "through" + "arrangement") organizes documentation into four categories based on two axes: whether the reader is _learning_ or _working_, and whether they need _theoretical_ or _practical_ knowledge.

```
                  Learning          Working
                ┌───────────────┬───────────────┐
  Practical     │  Tutorials    │  How-to Guides│
                ├───────────────┼───────────────┤
  Theoretical   │  Explanation  │  Reference    │
                └───────────────┴───────────────┘
```

- **Tutorials** — Learning-oriented, practical. Guide the reader through a series of steps to build understanding. Prioritize the reader's experience over efficiency.
- **How-to Guides** — Task-oriented, practical. Provide steps to achieve a specific goal. Assume the reader already has context and wants to get something done.
- **Explanation** — Understanding-oriented, theoretical. Discuss concepts, architecture, design philosophy. Help the reader build a mental model.
- **Reference** — Information-oriented, theoretical. Provide authoritative, precise descriptions of the system's components.

The framework's primary contribution is the insight that mixing these modes within a single document creates confusion. A tutorial that pauses for theoretical explanations loses its flow. A reference document that includes tutorial-style walkthroughs buries the information experienced users need.

## Writing for Skimming

Research on reading behavior consistently shows that most readers skim documents before (or instead of) reading them linearly. Technical writing that accommodates skimming is more effective than writing optimized for linear reading.

### Techniques

- **Front-load key information.** Place conclusions, decisions, and recommendations at the top. The inverted pyramid structure (most important → least important) serves skimmers and busy readers.
- **Use descriptive headings.** "Authentication Design" conveys more than "Section 3." Headings function as a table of contents for the skimming reader.
- **Prefer bullet points and tables for structured information.** Dense paragraphs hide information; structured formats expose it.
- **Bold key terms and findings.** Visual emphasis guides the eye to the important content.
- **Keep paragraphs short.** Long blocks of text discourage reading. Three to five sentences per paragraph is a practical guideline.
- **Use progressive disclosure.** Start with a summary, then expand with detail. Readers who need depth can continue; those who don't can stop.

## Code Comments

Code comments occupy a contested space in software engineering. There is broad agreement on certain principles but significant disagreement on the appropriate density and style.

### Areas of Relative Consensus

- Comments that explain _why_ (the reasoning, constraints, or context behind a decision) are more durable than comments that explain _what_ (which the code itself conveys)
- Comments that describe non-obvious behavior, edge cases, or workarounds for external constraints add value that the code cannot provide
- Comments that restate the code in English ("increment counter by one") add noise without value
- Outdated comments that contradict the code are actively harmful

### The Tension

One perspective holds that well-written code is largely self-documenting, and that comments are a code smell indicating that the code should be made clearer. On this view, the appropriate response to "this needs a comment" is usually to refactor the code — extract a method, rename a variable, simplify the logic.

The counterpoint is that code operates at the implementation level and cannot convey strategic intent, historical context, or domain-specific reasoning. No amount of clean code explains why a particular algorithm was chosen over a theoretically faster one, or why a workaround exists for a bug in a third-party dependency.

In practice, most teams land on a middle position: code should be as clear as possible on its own, and comments should fill the gaps that code cannot address — the _why_, the _constraints_, and the _non-obvious_.

## Changelog Maintenance

Changelogs serve as a communication channel between producers and consumers of software. They document what changed, when, and (ideally) why it matters to the reader.

### Audience and Framing

An effective changelog is written from the consumer's perspective, not the developer's. "Refactored the query builder internals" describes what the developer did. "Fixed incorrect results when filtering by date range across time zones" describes what changed for the user.

Common changelog categories include:

- **Added** — New features or capabilities
- **Changed** — Modifications to existing behavior
- **Deprecated** — Features that will be removed in a future version
- **Removed** — Features that have been removed
- **Fixed** — Bug fixes
- **Security** — Changes addressing vulnerabilities

### Automation Considerations

Generating changelogs from commit messages trades editorial quality for consistency and completeness. Conventional commit formats (e.g., `feat:`, `fix:`, `chore:`) enable automated categorization. The trade-off is that commit messages optimized for development context ("fix null check in user serializer") may not serve the changelog reader's needs ("fixed crash when loading profiles with missing email addresses").

Some teams adopt a hybrid approach: automated aggregation of commit messages produces a draft, which a human edits into user-facing language for release notes.

## The Documentation-Code Gap

Documentation and code diverge over time. Code changes are frequent and incremental; documentation updates are easy to forget. The result is a growing gap between what the documentation describes and what the system actually does.

### Strategies for Reducing the Gap

| Strategy                | Mechanism                                                         | Trade-off                                                  |
| ----------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------- |
| Generated documentation | Docs derived from code (types, schemas, annotations)              | Always accurate but limited in explanatory depth           |
| Docs-as-code            | Documentation stored alongside code, reviewed in the same process | Increases change friction but catches drift at review time |
| Tested examples         | Code samples in docs executed in CI                               | Catches broken examples but adds build complexity          |
| Documentation ownership | Named owners responsible for accuracy of specific documents       | Creates accountability but adds organizational overhead    |
| Expiration dates        | Documents marked with review-by dates                             | Triggers review but requires process discipline            |
| Living documents        | Documents stored in wikis or collaborative tools for easy updates | Low friction but hard to version and review                |

No strategy eliminates the gap entirely. The most effective approaches combine multiple techniques: generated reference docs ensure API accuracy, docs-as-code keeps conceptual docs in the review pipeline, and tested examples prevent the most embarrassing form of documentation failure — code that doesn't work.

## Diagrams and Visual Communication

Diagrams convey structural and relational information that prose handles poorly. System architecture, data flow, sequence interactions, and state transitions are all more naturally expressed visually than textually.

### Diagram Categories

| Type                         | Best For                                                        |
| ---------------------------- | --------------------------------------------------------------- |
| Architecture diagrams        | System boundaries, component relationships, deployment topology |
| Sequence diagrams            | Interaction flows between components over time                  |
| Data flow diagrams           | How data moves through a system, where it's transformed         |
| State diagrams               | Lifecycle of an entity through its possible states              |
| Entity-relationship diagrams | Data models and their relationships                             |
| Flowcharts                   | Decision logic, process workflows                               |

### Diagram-as-Code

Tools that render diagrams from text descriptions (Mermaid, PlantUML, D2, Graphviz) enable diagrams to live in version control, be reviewed in pull requests, and stay synchronized with the documentation around them. The trade-off is reduced visual flexibility compared to manual diagramming tools — but the maintenance benefits often outweigh the aesthetic limitations.

### Common Pitfalls

- **Over-detailed diagrams** that try to show every component and connection become unreadable. Effective diagrams show the right level of abstraction for their audience.
- **Orphaned diagrams** that aren't referenced from any document get stale without anyone noticing.
- **Inconsistent notation** across diagrams in the same project forces readers to relearn conventions. Establishing a consistent visual language across a project's documentation reduces cognitive load.

## Writing Quality Signals

Several indicators suggest whether technical writing is serving its purpose:

- **Discovery** — Can readers find the document they need? Poor discoverability means documentation exists but provides no value.
- **Currency** — Does the document reflect the current state of the system? Stale docs erode trust in all documentation.
- **Actionability** — Can the reader accomplish their goal after reading? Documentation that informs without enabling is incomplete.
- **Feedback loops** — Do readers have a way to report issues, ask questions, or suggest improvements? Documentation without feedback mechanisms degrades silently.

The ultimate test of technical writing is whether it changes behavior — whether engineers make better decisions, users complete tasks more efficiently, or new contributors become productive more quickly because the documentation exists.
