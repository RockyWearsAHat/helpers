# Code Generation — Patterns, Schema-Driven Development & Trade-offs

Code generation is the practice of producing source code, configuration, or other artifacts programmatically rather than writing them by hand. It transforms one representation — a schema, model, specification, or template — into another, typically targeting a specific language, framework, or runtime. The technique spans from simple scaffolding scripts to sophisticated compiler backends, touching nearly every domain in software engineering.

## The Generator/Template Pattern

Most code generation systems follow a common structural pattern:

```
Input Specification + Templates + Generator Logic = Output Code
```

The three components serve distinct roles:

| Component           | Role                                                | Examples                                                          |
| ------------------- | --------------------------------------------------- | ----------------------------------------------------------------- |
| Input specification | What to generate — the data model or contract       | OpenAPI spec, database schema, protobuf definition, DSL source    |
| Templates           | How to emit — the shape of the output               | Mustache/Handlebars templates, string interpolation, AST builders |
| Generator logic     | When and why — orchestration, validation, decisions | CLI tool, build plugin, compiler pass                             |

This separation enables reuse: the same specification can drive generators targeting different languages, frameworks, or output formats. The same templates can be applied to different specifications conforming to the same meta-schema.

### Template Approaches

**String-based templates** interpolate values into text with placeholder syntax. They are easy to understand and write, but offer no structural guarantees — a template can produce syntactically invalid output, and the error surfaces only when the output is compiled or parsed.

**AST-based generation** constructs output as a syntax tree, then serializes it to source text. This approach guarantees structural validity (no mismatched brackets or missing semicolons) and can integrate with formatters for consistent output style. The cost is greater complexity in the generator itself.

**Quasi-quotation** combines both: a template syntax embedded in the host language that is checked at compile time and produces AST fragments. This provides structural safety with template convenience, but is available only in languages that support it.

## Schema-First Development

Schema-first (or contract-first) development treats a formal specification as the single source of truth for a system boundary. All implementations on both sides of the boundary are derived — whether generated or manually written to conform — from that specification.

### Common Schema-First Workflows

**API-first with OpenAPI/Swagger:**

1. Define the API contract in an OpenAPI specification
2. Generate server stubs (routing, request parsing, validation)
3. Generate client SDKs in target languages
4. Generate documentation from the same spec
5. Runtime validation enforces the contract

**Database-schema-first:**

1. Define the database schema (DDL or migration files)
2. Generate ORM models or data access objects
3. Generate type-safe query builders
4. Generate migration scripts for schema evolution

**Protocol-first with IDL (Interface Definition Language):**

1. Define message types and service interfaces in an IDL (Protocol Buffers, Thrift, Avro, Cap'n Proto)
2. Generate serialization/deserialization code per language
3. Generate RPC client and server stubs
4. Generate schema evolution compatibility checks

**Event-schema-first:**

1. Define event schemas (JSON Schema, Avro)
2. Generate producer and consumer code
3. Generate schema registry integration
4. Generate compatibility validation

### Benefits of Schema-First

- **Single source of truth** — the schema is authoritative; implementations derive from it
- **Cross-language consistency** — clients and servers in different languages agree on the contract
- **Evolution management** — schema versioning and compatibility rules are defined once
- **Documentation stays current** — generated from the same artifact that drives implementation
- **Validation is automatic** — generated code includes schema-conformance checks

### Tensions in Schema-First

- Schemas must be expressive enough to capture real constraints, or hand-written validation code fills the gap
- Schema design requires upfront investment that feels premature in early, exploratory development
- Generated code may not match the idioms of the target language, creating friction
- Schema evolution rules can be constraining when breaking changes are genuinely needed

## Generated Code as a Build Artifact

A critical principle in code generation: **generated files are outputs, not sources**. They should be treated like compiled binaries — produced by the build system, not edited by hand.

Implications of this principle:

| Practice                                             | Rationale                                                                 |
| ---------------------------------------------------- | ------------------------------------------------------------------------- |
| Regenerate on every build                            | Ensures generated code matches current specifications                     |
| Do not commit generated files (or mark them clearly) | Prevents merge conflicts in artifacts and confusion about source of truth |
| Never hand-edit generated files                      | Edits are overwritten on the next generation pass                         |
| Include a "do not edit" header in generated files    | Signals intent to future developers                                       |
| Version the generator and templates, not the output  | The specification + generator version fully determines the output         |

Some projects do commit generated files — for build simplicity, for environments where the generator isn't available, or when the generation step is expensive. This is a pragmatic trade-off, not a best practice. When generated files are committed, they need clear marking and discipline to keep them synchronized with their sources.

## Hand-Written vs Generated Code

The decision to generate code rather than write it by hand involves several trade-offs:

| Dimension                    | Generated                                     | Hand-written                        |
| ---------------------------- | --------------------------------------------- | ----------------------------------- |
| Consistency                  | Uniform across all outputs                    | Varies with author and context      |
| Flexibility                  | Constrained by generator capabilities         | Unlimited                           |
| Maintenance on schema change | Regenerate (possibly automated)               | Manual update per file              |
| Debugging                    | May be harder to trace through                | Familiar and direct                 |
| Performance                  | Can be optimized for specific patterns        | Can be optimized for specific cases |
| Readability                  | Template-shaped, sometimes verbose            | Author-controlled                   |
| Onboarding                   | Requires understanding the generation process | Requires understanding the code     |
| Error messages               | May reference generated locations             | Reference familiar source           |

There is no universal answer. Generation tends to win when:

- The same pattern repeats across many instances (CRUD operations, serialization for many types)
- Consistency across instances is more important than individual optimization
- The input specification changes frequently and propagation must be reliable

Hand-written code tends to win when:

- Each instance has unique logic that doesn't fit a template
- Performance tuning requires case-by-case optimization
- The code is written once and rarely changes

## API Client Generation

Generating API clients from specifications is among the most widespread code generation practices.

### What Gets Generated

- HTTP client wrappers with typed request/response objects
- Authentication and header management
- Request serialization and response deserialization
- Error handling and retry logic
- Pagination helpers
- Documentation and inline type annotations

### Generator Strategies

**Full client generation** produces a complete, ready-to-use SDK. The developer imports it and calls methods. This maximizes convenience but can feel opaque — the generated client may make assumptions about HTTP libraries, error handling, or configuration that don't match the project's conventions.

**Type-only generation** produces type definitions and interfaces but leaves the HTTP transport to the developer. This provides type safety without dictating implementation choices.

**Thin wrapper generation** produces a minimal layer — URL construction, parameter mapping, type annotations — that wraps a standard HTTP client. This balances type safety with transparency.

## ORM and Data Access Generation

Generating data access code from database schemas reduces the manual mapping between database tables and application objects.

Approaches range from:

- Generating complete ORM model classes with relationships, validations, and queries
- Generating type-safe query builders that compose SQL programmatically
- Generating raw SQL with type-checked parameter binding and result mapping
- Generating migration files from schema diffs

The generator's output philosophy matters: some generators produce code intended to be extended (base classes with hooks for custom behavior), while others produce sealed outputs that should never be modified.

## Serialization Code Generation

Protocol Buffers, Thrift, Avro, Cap'n Proto, FlatBuffers, and similar systems define data schemas in IDLs and generate language-specific serialization code.

Key dimensions where these systems differ:

| Dimension              | Approaches                                                                  |
| ---------------------- | --------------------------------------------------------------------------- |
| Schema evolution       | Field numbering (protobuf), schema registry (Avro), both (Thrift)           |
| Encoding format        | Binary (most), JSON-compatible (some), zero-copy (FlatBuffers, Cap'n Proto) |
| Code style             | Builder pattern, direct field access, immutable value objects               |
| RPC integration        | Built-in service definitions, or schema-only                                |
| Cross-language support | Varies from 2-3 languages to dozens                                         |

The generated code handles the tedious, error-prone work of binary encoding, field ordering, backward compatibility, and type coercion — tasks that are both critical to get right and unrewarding to implement by hand.

## Scaffolding — Generating Starting Points

Scaffolding tools generate project structure, boilerplate files, and initial code to accelerate project setup. Unlike other code generation, scaffolding output is explicitly intended to be modified.

Characteristics of scaffolding:

- Runs once (or rarely) at project initialization
- Generates files the developer will own and edit
- Prioritizes getting started quickly over ongoing automation
- Often interactive, prompting for project name, options, and configuration
- Output may include comments indicating where customization is expected

The tension: scaffolding that generates too little is unhelpful; scaffolding that generates too much creates files the developer doesn't understand and doesn't need. Effective scaffolding produces the minimum viable starting point for the target framework and development workflow.

## Partial Generation — Customization Points

Pure generation (the generator owns 100% of the output) is clean but inflexible. Real-world code generation often needs customization points where hand-written code integrates with generated code.

### Strategies for Partial Generation

**Inheritance-based:** Generate a base class; developers subclass it to add custom behavior. Regeneration replaces the base class without affecting subclasses. Works well in object-oriented languages but can create deep inheritance hierarchies.

**Mixin/composition-based:** Generate core functionality; developers compose it with custom modules. Avoids inheritance depth but requires the generator to expose a composable interface.

**Protected regions:** Mark sections of generated files as "protected" — the generator overwrites everything except protected regions, which preserve manual edits across regeneration. Fragile: if the generated structure changes around a protected region, the region may become invalid.

**Partial classes/files:** Some languages allow a class definition to span multiple files. The generator owns one file; the developer owns another. Both contribute to the final class.

**Extension points:** Generate code with explicit hooks (callbacks, event handlers, strategy interfaces) that developers implement. The generated code calls the hooks at appropriate points.

**Configuration-driven:** The generator accepts configuration that controls its output, reducing the need for post-generation editing. This shifts customization from source editing to generator input.

## Source-Level vs Binary-Level Generation

Code generation can target different levels of the compilation pipeline:

**Source-level generation** produces human-readable source code. Benefits include inspectability, debuggability (breakpoints work), and compatibility with standard toolchains. Drawbacks include formatting concerns, potential for hand-editing (violating the "don't edit generated code" principle), and generation overhead in the build.

**Binary-level generation** produces bytecode, intermediate representation, or machine code directly. Benefits include skipping the compilation step, potentially tighter control over output, and no temptation to hand-edit. Drawbacks include harder debugging, dependency on binary format stability, and less transparency.

**Hybrid approaches** generate source that is immediately compiled in memory, never written to disk. This provides some transparency (the source can be logged or inspected on demand) without the file management overhead.

## DSLs as Code Generation Input

Domain-specific languages (DSLs) serve as high-level inputs to code generators, allowing domain experts to describe intent in familiar terms while the generator handles implementation details.

### External DSLs

Separate languages with their own syntax and parsers:

- SQL as a DSL for data manipulation
- Regular expressions as a DSL for pattern matching
- Build system languages (Make targets, build rules)
- Configuration languages (HCL, YAML-based specs)

### Embedded DSLs

Subsets or extensions of a host language that read as domain-specific:

- Query builders that read like SQL but are host-language expressions
- Test DSLs that read like specifications
- Configuration DSLs using the host language's object literals

The generator's job is translating DSL expressions into efficient target-language code. The quality of this translation — how readable, debuggable, and performant the output is — largely determines whether the DSL is adopted or abandoned.

## Testing Generated Code

Testing in code generation systems requires a strategy that accounts for the separation between the generator and its output.

### Test the Generator

- Feed known inputs and verify the output matches expected code
- Test edge cases: empty schemas, deeply nested types, reserved words, unusual characters
- Test schema evolution scenarios: adding fields, removing fields, changing types
- Verify the generator rejects invalid inputs with clear errors
- Snapshot testing: compare generated output against approved baselines

### Test the Output

- Compile the generated code (catches syntax and type errors)
- Run generated tests (some generators produce test code alongside implementation)
- Integration tests exercising generated client/server pairs
- Verify serialization round-trips (serialize then deserialize, check equality)
- Performance benchmarks for generated serialization/deserialization paths

### Test Both Together

- End-to-end tests from specification change through regeneration to passing integration tests
- Compatibility tests: generate code from old and new schema versions, verify interop
- Fuzz testing: generate random valid schemas, generate code, compile, and run basic operations

The testing strategy depends on who owns the generator. For third-party generators, testing the output is sufficient. For in-house generators, testing the generator logic is essential.

## The Maintenance Burden

Code generation introduces a distinctive maintenance cost: the **regeneration cascade**. When a schema changes:

1. The schema must be updated
2. The generator must be rerun
3. All generated code must be updated (possibly across multiple languages/repos)
4. Hand-written code that depends on generated code may need updating
5. Tests that depend on generated structures may need updating

In a well-automated system, steps 2-3 happen as part of the build. In a poorly automated system, they happen manually, inconsistently, and late.

Additional maintenance concerns:

- **Generator version upgrades** may change output format, requiring widespread updates
- **Template drift** — when templates are maintained separately from the generator, they can fall out of sync
- **Multi-repo generation** — when generated code spans repositories, coordination becomes a release engineering problem
- **Generator bugs** may silently produce incorrect code that passes compilation but fails at runtime

## When Code Generation Becomes Over-Engineering

Signals that code generation may be adding more complexity than it removes:

- The generator is more complex than the code it produces
- Schema changes are rarer than the overhead of maintaining the generation pipeline
- The generated code is heavily customized via escape hatches, undermining the consistency benefit
- Developers routinely debug through generated code, negating the productivity benefit
- The generation step adds significant time to the build
- A single implementation would suffice — generation presupposes multiple targets that may never materialize
- The team spends more time maintaining the generator than they would maintaining hand-written code

Code generation is a tool for managing repetition and enforcing consistency at scale. When the scale doesn't justify the tooling investment, simpler approaches — shared libraries, copy-and-adapt patterns, even manual synchronization — may be more appropriate.

## Relationship to Other Concepts

Code generation connects to several adjacent areas:

- **Metaprogramming** — code generation is metaprogramming applied as a build step rather than a language feature
- **Compiler design** — code generators share techniques with compiler backends (IR, optimization, emission)
- **Build systems** — code generation must integrate with incremental builds, dependency tracking, and caching
- **API design** — schema-first generation is a forcing function for explicit, versioned API contracts
- **Type systems** — generated code often leverages advanced type features to provide compile-time safety
- **DevOps** — infrastructure-as-code tools (Terraform, Pulumi) are code generators targeting cloud provider APIs
- **Model-driven development** — UML-to-code and similar approaches are code generation from visual models
