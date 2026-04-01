# Technical Writing Patterns — Documentation Types, READMEs, Changelogs & Docs-as-Code

## The Four Modes of Documentation (Diátaxis)

Daniele Procida's Diátaxis framework resolves the chaos of most documentation. It observes that complete documentation requires **four types**, each serving a distinct reader need. Mixing them into one document confuses everyone. The framework is empirically grounded in analyzing hundreds of successful and failed docs.

| Mode | Reader State | Purpose | Tone | Author Discipline |
|------|------|---------|------|---|
| **Tutorial** | Beginner, wants to learn | Get started quickly, hands-on lesson | Encouraging, simple examples | Write for first experience; don't assume knowledge |
| **How-to Guide** | Practitioner with knowledge, solving specific problem | Recipe to accomplish exact task | Action-oriented, assumes competence | Be specific; don't explain basics |
| **Reference** | Developer mid-flow, needs facts | Complete, searchable, structured | Neutral, comprehensive | Exhaustive, organized, indexed |
| **Explanation** | Learner or architect, wants understanding | Discuss rationale, trade-offs, history | Analytical, conversational | Explain *why*, not just *what* |

**Organization principle**: Hold each mode in physically separate locations. Tutorial sections don't link to API reference; reference sections don't include narrative. Cross-references happen by *semantic need*, not proximity.

**Anti-pattern**: Trying to build one document that explains everything satisfies no one. A 100-page guide intended as both tutorial AND reference becomes bloated, unnavigable, and confusing to new learners.

## README: Your Product's First Impression

A README is the entry point to your project. It must answer four questions in 60 seconds:

1. **What is this?** (one line: "X is a tool for Y purpose")
2. **Why should I care?** (one paragraph: problem solved, not feature list)
3. **How does it work?** (quick example/screenshot)
4. **Where do I go next?** (links to install, docs, contribute)

**Structure:**

```
# Project Name

One-sentence description. Link to live demo or screenshot if applicable.

## Problem & Why This Matters

1-2 paragraphs. Describe the problem solved. Avoid adjectives ("blazingly fast" 
is noise). Focus on concrete pain points.

## Features (Optional)

Bullet bullets only if you have clear differentiators. Generic feature lists 
("logging", "error handling") are white noise.

## Quick Start

```bash
npm install project-name
# OR
cargo install project-name
```

Then a minimal example that demonstrates success:

```javascript
import { createClient } from 'project-name'
const client = createClient({ apiKey: 'xxx' })
const data = await client.fetch('endpoint')
console.log(data) // Shows something meaningful
```

Don't make users dig for a happy path. The quick start should work in 30 seconds.

## Installation

Link to full install instructions. Examples: different OS, different package managers, 
development mode, Docker.

## Documentation

Link to docs, not summary. Link to tutorials, API reference, architecture notes.

## Contributing

Link to CONTRIBUTING.md. One-liner: "Report bugs, submit PRs. See CONTRIBUTING.md."

## License

One line: "MIT License. See LICENSE file."
```

**Common mistakes:**

- Assuming readers know the problem. Explain the problem before pitching the solution.
- Feature-dumping. Five features are noise; one strong example is compelling.
- Making the quick start require 10 steps. Reduce to 3-4.
- Orphaning links. If CONTRIBUTING.md doesn't exist yet, say so and link anyway (shows intent).

## API Documentation

API docs have two layers: **specification** (machine-readable contract) and **narrative** (human guidance).

**Specification layer (OpenAPI / AsyncAPI):**

Write your spec first. It's your contract.

```yaml
paths:
  /users/{id}:
    get:
      summary: Retrieve a user by ID
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: integer
      responses:
        '200':
          description: User found
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
        '404':
          description: User not found
```

Spec covers: endpoints, HTTP methods, parameters, request/response bodies, error codes, headers, auth. Tools (Swagger UI, ReDoc, Fern) auto-generate interactive docs from specs.

**Narrative layer:**

Specs tell you *what* each endpoint does; narratives explain *when* and *why*.

- **Conceptual overview**: Authentication flow. Rate limits and backoff strategy. Pagination philosophy (offset vs. cursor).
- **Tutorial**: Walk through authentication → fetching user → creating a resource → error handling.
- **How-to guides**: Profile API calls with your HTTP client. Handle 429 (rate limit) errors. Implement exponential backoff.
- **Explanations**: Why is pagination cursor-based? Trade-offs of REST vs. GraphQL. Design history and evolution.

Tools like Fern, Stoplight, and custom Markdown pipelines let you write narrative docs alongside specs.

## Changelog and Release Notes

Users want to know what changed. Two documents serve different purposes:

**Changelog** (continuous record):

- One entry per release.
- Sections: Added, Changed, Deprecated, Removed, Fixed, Security.
- Every user-facing change listed. Internal refactors omitted.
- Format: [Keep a Changelog](https://keepachangelog.com/) is the standard.

```
## [2.1.0] - 2024-03-20

### Added
- Support for custom polling intervals in client configuration
- New `debug` flag for verbose logging

### Changed
- API response pagination now uses cursor-based offsets (breaking for offset-based code)

### Fixed
- Memory leak in connection pool when retrying failed requests
- Race condition in multi-threaded client initialization

### Security
- Updated TLS cipher suite to exclude deprecated algorithms
```

**Release notes** (marketing + migration):

- Targeted communication. What's new? What's important to you?
- Emphasis on what users care about, not every internal change.
- Migration guidance for breaking changes.
- Format: Markdown or blog post. Distributed via email, changelog, release page.

```
# v2.1.0 Release: Cursor-Based Pagination

[Download](#) | [Migration Guide](#migration)

## What's New

- **Cursor-based pagination**: cursor-based pagination is faster at scale and 
  handles real-time data better. See [migration guide](#migration).
- **Custom polling**: Configure polling intervals per client: 
  `client.poll({ interval: 5000 })`.

## Breaking Changes

Offset-based pagination (`offset`, `limit`) is deprecated. Use cursors 
(`after`, `limit`). [Automated migration tool available](#codemods).

## Migration

```javascript
// Old (offset-based)
const page1 = await client.fetch('/users', { offset: 0, limit: 20 })

// New (cursor-based)
const page1 = await client.fetch('/users', { limit: 20 })
const page2 = await client.fetch('/users', { limit: 20, after: page1.nextCursor })
```
```

**Anti-patterns:**

- Treating changelog as commit message dump. "Fixed typo in error message" is not user-facing.
- Hiding breaking changes. Call them out. Release notes are the place to soften the blow with migration guides.
- Release notes that aren't readable once published. Test them — do links work? Is formatting clear?

## Writing for Different Audiences

Technical writing isn't one size fits all. Calibrate for reader level.

**Beginner audience:**

- Explain concepts before diving into code.
- Use analogies. "A transaction is like a book transaction — either it commits fully or rolls back."
- Build on prior steps. Don't reference concepts from a section they skipped.
- Celebrate small wins. "Congratulations, your API call worked!"

**Intermediate (practitioners):**

- Assume existing knowledge of fundamentals. Don't explain what a loop is.
- Jump to the useful part. "Here's how to optimize query time."
- Provide extensibility patterns. How do you customize, hook in, or extend?

**Advanced (architects, maintainers):**

- Focus on rationale and constraints. Why was this design chosen? What are the trade-offs?
- Discuss failure modes. "This approach doesn't work well when X. In that case, consider Y."
- Link to papers, RFCs, or prior art. Architects want to understand the genealogy.

**Common mistake**: Marketing to one audience in a guide meant for another. A tutorial should not position your tool as "the best"; it should simply teach. Positioning is for marketing, not docs.

## Docs-as-Code: Markdown, MDX, Docusaurus, MkDocs

Documentation should version, review, and deploy like code.

**Source format:**

- **Markdown**: Plain-text, readable, version-controllable. `.md` files in git.
- **MDX**: Markdown + embedded React components. Allows interactive examples, live code snippets.
  ```
  # API Rate Limiting

  Here's how to check your current quota:
  <CodeExample endpoint="/quota" language="javascript" />
  ```

**Static site generators:**

- **Docusaurus** (React-based, JavaScript ecosystem): Built-in search, versioning, i18n, themes. Steep learning curve but powerful.
- **MkDocs** (Python-based): Lightweight, fast, minimal. Material for MkDocs theme is popular and beautiful.
- **Hugo** (Go-based): Fastest for large docs. Less opinionated; requires more customization.
- **Nextra** (TypeScript + Next.js): Modern ergonomics; tight VS Code integration.

**Workflow:**

1. Write `.md` or `.mdx` files in a `docs/` folder.
2. CI builds the static HTML on every push to main.
3. Deploy to GitHub Pages, Netlify, or Vercel.
4. Review PRs with preview links before merging.

**Benefits:**

- Docs reviewed like code PRs.
- Docs version with releases. Old docs available for old versions.
- Search works (static generators index docs at build time).
- No CMS lock-in. Docs are portable.

**Structure example:**

```
docs/
├── index.md                      # Landing page
├── getting-started.md            # Tutorial
├── guides/
│   ├── authentication.md         # How-to: Set up auth
│   ├── rate-limiting.md          # How-to: Handle limits
├── api/
│   ├── users.md                  # Reference: users endpoint
│   ├── posts.md                  # Reference: posts endpoint
├── explanations/
│   ├── architecture.md           # Why design this way
│   ├── performance-model.md      # How scaling works
```

## Cross-Cutting: Tone and Precision

Effective technical writing is:

- **Precise**: "You can call the endpoint" is weak. "Call POST /auth/token with your API key as the `Authorization` header" is precise.
- **Active voice**: "The system rejects requests without valid tokens" beats "Requests without valid tokens are rejected."
- **Concrete over abstract**: Show code examples; don't explain in English what code shows better.
- **Humble**: Acknowledge limitations. "This approach works up to X throughput" is better than "This is infinitely scalable."

## See Also

- [Software Documentation — APIs, Architecture, Diátaxis & Docs-as-Code](process-documentation.md)
- [Process: Technical Writing — Documentation, ADRs, RFCs & Design Documents](process-technical-writing.md)