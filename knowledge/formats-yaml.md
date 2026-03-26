# YAML — Structure, Features, Security & Trade-offs

**YAML** (YAML Ain't Markup Language) is a human-friendly data serialization format designed for configuration files and data interchange. Its philosophy prioritizes readability and minimal syntax, differentiating it sharply from formats like JSON and XML. However, this expressiveness introduces complexity, security risks, and implementation variability that complicate its role as a standard format.

## Core Syntax and Structure

YAML represents data through indentation (whitespace-significant) and minimal delimiters. Keys and values are separated by colons; lists use dashes. Strings don't require quotes unless they contain special characters.

**Fundamental types:**
- Scalars: strings, integers, floats, booleans (true/false, yes/no), null
- Collections: lists (ordered) and maps (key-value pairs)
- Multiline strings: `|` (literal, preserves newlines), `>` (folded, converts newlines to spaces)

**Anchors (`&`) and aliases (`*`)** enable reference and reuse:

```yaml
defaults: &base-config
  timeout: 30
  retries: 3

service_a:
  <<: *base-config      # Merge operator: includes all keys from base-config
  port: 8000

service_b:
  <<: *base-config
  port: 8001
```

The `<<` merge key is non-standard but widely supported. It copies referenced keys into the current map.

**YAML 1.1 vs 1.2:**
- **1.1** (most deployments): loose type inference. Strings like `yes`, `no`, `on`, `off`, `1.0`, `0x1A` are interpreted as booleans, floats, or hex integers. This introduces surprise behaviors.
- **1.2** (newer, Web standard): stricter. Boolean-like strings remain strings unless explicitly tagged as boolean. More predictable but breaks backward compatibility.

Most production systems still use YAML 1.1 parsers (PyYAML, Ruby YAML, Kubernetes). This mismatch between spec versions is a persistent source of confusion.

## Tags and Custom Types

Tags (`!tag-name`) annotate scalars or collections with explicit types:

```yaml
date: !timestamp 2025-01-15T10:30:00Z
binary: !!binary |
  SGVsbG8gV29ybGQ=
custom: !python/object:module.ClassName
  arg1: value1
```

Tags enable custom deserialization. Libraries implement `!timestamp`, `!binary`, `!float` etc. Problem: **tags are not standardized across implementations**. Custom tags can be exploited if a parser deserializes untrusted input using object constructors (`!python/object`, `!ruby/object`).

## Security Risks

YAML's flexibility creates several well-known attack vectors:

### Deserialization Attacks
Untrusted YAML parsed with unsafe deserializers can execute arbitrary code. The classic Python example:

```yaml
!!python/object/apply:os.system ['rm -rf /']
```

A naive YAML parser that instantiates Python objects processes this tag and executes the command. **Mitigation:** Use safe loaders (PyYAML's `yaml.safe_load()`, not `yaml.load()`) that refuse to instantiate arbitrary objects.

### Billion Laughs (Exponential Entity Expansion)
Anchors and aliases can create exponential amplification:

```yaml
a: &ref
  - a
  - a
  - a
  - a
b: &b [*ref, *ref, *ref, *ref]
c: [*b, *b, *b, *b]
```

Deeply nested aliases multiply the data size exponentially. A small YAML file expands to gigabytes in memory during parsing. **Mitigation:** Parsers should limit alias depth and total size growth.

### The "Norway" Problem
A surprising edge case in YAML 1.1:

```yaml
NO: No
```

The key `NO` is interpreted as a boolean (no) in YAML 1.1, not a string. Similarly, `YES`, `TRUE`, `FALSE` cause confusion. Strings like `1.0` are parsed as floats. This breaks data that looks like strings but gets retyped. **Mitigation:** Use quotes (`"NO": No`) or switch to YAML 1.2. Kubernetes and many tools default to YAML 1.1, making this a real gotcha.

## Alternatives

### TOML (Tom's Obvious, Minimal Language)
Goal: configuration with explicit, minimal syntax. No anchors, no custom types, stricter semantics. Better for static configuration, harder for dynamic data. Gaining adoption in Rust/Python ecosystems (Cargo.toml, pyproject.toml).

### JSON
Strict, unambiguous, universal parser support. No comments, less readable for humans. Works well for generated or validated data; friction for hand-editing.

### DHALL
Strongly typed, functional configuration language. Type checking, immutability, URL imports. Steeper learning curve; less widely adopted.

### CUE
Configuration language with inference and validation. Schemas and instances coexist in the same language. Used in some DevOps tools (Cue Lang, Pulumi). Emerging, smaller ecosystem.

### HCL (HashiCorp Configuration Language)
Used in Terraform, Nomad, Packer. Block-based syntax, easier to parse than YAML, less expressive.

## YAML in Kubernetes

Kubernetes manifests are YAML 1.1. A few gotchas:
- Port `8000` and `08000` parse differently (one is decimal, one is octal in YAML 1.1).
- Boolean fields (`replicas: true`) confuse users; fields like `startupProbe.enabled` expect booleans, but the YAML parser is liberal about what counts as true/false.
- Multi-document YAML (separated by `---`) is standard; each document is a separate Kubernetes resource.

## Linters and Validation

**Strict YAML linters:**
- `yamllint` (Python): enforces consistent indentation, line length, trailing spaces, key ordering.
- `yq` (Go): YAML query language; inspect, filter, and transform YAML.
- `jq` / `jq-compatible tools`: for JSON-centric workflows, convert YAML to JSON first.

**Schema validation:**
- JSON Schema can validate YAML (convert YAML to JSON, validate schema).
- YAML-specific schema tools (e.g., Kubernetes CRD validation) use JSON Schema internally.
- No universal YAML Schema standard equivalent to JSON Schema.

**Strict mode parsing:**
- PyYAML: use `safe_load()` with strict options (`allow_unicode=True`, disable merge keys if needed).
- Ruby YAML: similar safe/unsafe distinction.
- Go's `gopkg.in/yaml.v3`: strict mode available; respects YAML 1.1 vs 1.2 with `v3.MapSlice` and other type safety features.

## When to Use YAML

**Good fit:**
- Configuration with comments and human editing (Kubernetes, Ansible, many DevOps tools)
- Nested structure with moderate depth
- Teams comfortable with whitespace-sensitive syntax

**Poor fit:**
- Strict data interchange (use JSON or Protocol Buffers)
- Untrusted input (use safe parsers; validate schema)
- High-performance parsing (JSON and Protocol Buffers are faster)
- Data with uncertain type semantics (YAML's type inference is surprising)

**In practice:**
YAML dominates infrastructure and configuration tooling (Kubernetes, Docker Compose, Terraform, Ansible, GitHub Actions) not because it's ideal, but because early adoption created network effects. For new projects, consider whether YAML's readability advantage justifies its complexity and parsing variability.

## Related

See also: [data-formats-config.md](data-formats-config.md), [data-serialization-formats.md](data-serialization-formats.md), [configuration-management.md](configuration-management.md), [kubernetes-helm-patterns.md](kubernetes-helm-patterns.md)