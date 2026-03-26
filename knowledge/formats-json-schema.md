# JSON Schema — Validation, Composition, Draft Versions & Tooling

**JSON Schema** is a declarative language for validating JSON documents. It defines the structure, types, constraints, and metadata of JSON data without runtime code. A schema is itself a JSON object describing what valid instances look like. Unlike protobuf (which requires code generation), JSON Schema validation happens at runtime; this flexibility supports dynamic validation and tooling (code generation, form building, documentation).

## Fundamental Concepts

A schema is a JSON object with keywords describing constraints:

```json
{
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "age": { "type": "integer", "minimum": 0 }
  },
  "required": ["name"]
}
```

This schema accepts objects with optional `name` (string) and `age` (non-negative integer) properties; `name` is required. Validators test whether a JSON document satisfies all constraints.

## Core Keywords

**Type constraints:** `type` restricts values to `string`, `number`, `integer`, `boolean`, `array`, `object`, or `null`. Scalars are independent; `null` is a type, not an absence marker.

**String constraints:**
- `minLength`, `maxLength`: restrict length
- `pattern`: regex matching
- `format`: semantic types like `email`, `uri`, `date-time`, `ipv4`, `hostname`

**Numeric constraints:**
- `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`
- `multipleOf`: value must be a multiple of a given number

**Array constraints:**
- `items`: schema for array elements (uniform) or `prefixItems` (tuple validation, first item matches items[0], second matches items[1], etc.)
- `minItems`, `maxItems`, `uniqueItems`

**Object constraints:**
- `properties`: map of property names to schemas
- `required`: list of mandatory property names
- `additionalProperties`: schema for properties not in `properties`, or `false` to forbid extra properties
- `minProperties`, `maxProperties`

## Composition and Reuse: Combinators

**Logical combinators** combine schemas:

```json
{
  "anyOf": [
    { "type": "string" },
    { "type": "number" }
  ]
}
```

This validates if the instance matches ANY schema in the list. Similarly:
- `allOf`: must match ALL schemas (useful for inheritance/mixins)
- `oneOf`: must match EXACTLY ONE schema (discriminated unions)
- `not`: must NOT match the schema

**$ref and References:**

```json
{
  "definitions": {
    "address": {
      "type": "object",
      "properties": {
        "street": { "type": "string" },
        "city": { "type": "string" }
      }
    }
  },
  "type": "object",
  "properties": {
    "home": { "$ref": "#/definitions/address" },
    "work": { "$ref": "#/definitions/address" }
  }
}
```

`$ref` is a pointer reference. `#/definitions/address` refers to a schema within the same document. URLs like `https://example.com/schemas/address.json#/properties/street` reference external schemas. This enables schema reuse and composition.

**Note:** `$ref` historically **short-circuits validation** — when `$ref` matches, other keywords in the same object are ignored (in older drafts). Modern drafts (2019-09, 2020-12) compose `$ref` with sibling keywords, making composition more intuitive.

## Conditional Logic: if/then/else

Apply different schemas conditionally:

```json
{
  "type": "object",
  "properties": {
    "type": { "enum": ["residential", "commercial"] },
    "squareFootage": { "type": "number" }
  },
  "if": { "properties": { "type": { "const": "commercial" } } },
  "then": {
    "properties": {
      "squareFootage": { "minimum": 5000 }
    }
  },
  "else": {
    "properties": {
      "squareFootage": { "maximum": 3000 }
    }
  }
}
```

If the condition matches, `then` applies; otherwise, `else` applies. Enables sophisticated validation workflows.

## Draft Versions and Stability

JSON Schema has multiple versions (drafts), each evolving the spec. Key timeline:
- **Draft 3, 4:** Early versions. Draft 4 (2013) was baseline for years.
- **Draft 6, 7:** Incremental improvements. Added `const`, `contains`, better `$ref` semantics.
- **2019-09:** Major revision. Renamed `definitions` to `$defs`, improved `$ref` (no longer short-circuits), added `unevaluatedProperties`.
- **2020-12:** Latest. Refined `$ref`, added `dependentRequired`.

**Stability:** Specs stabilize slowly. Tools support multiple drafts; documents must declare `$schema` to specify behavior:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": { ... }
}
```

Without `$schema`, validators default to an older draft (often draft 7). This causes surprises when upgrading validators or moving schemas between tools.

## Format Validation

`format` keyword suggests semantic validation for strings:

```json
{ "type": "string", "format": "email" }
```

Standard formats: `email`, `uri`, `date-time` (RFC 3339), `ipv4`, `ipv6`, `hostname`, `uuid`, `json-pointer`, `relative-json-pointer`.

**Key caveat:** Validators are inconsistent about format validation. Some perform strict checks; others are permissive. Use `format` as a hint, not a strict guarantee. For critical validation, add `pattern` constraints or custom logic.

## Custom Vocabularies

JSON Schema allows extension via custom keywords. A tool might define `x-golang-package` for code generation or `x-description` for documentation. Vocabularies are organized by namespace:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$vocabulary": {
    "https://json-schema.org/draft/2020-12/vocab/core": true,
    "https://my-org.com/vocab/custom": true
  }
}
```

This flexibility enables tools to extend JSON Schema without breaking compatibility.

## API Validation with OpenAPI

**OpenAPI** (REST API specification) uses JSON Schema (or a subset) for request/response validation. OpenAPI 3.1 uses JSON Schema 2020-12 directly. Earlier versions use an OpenAPI-specific schema dialect (compatible but not identical to JSON Schema).

```yaml
components:
  schemas:
    User:
      type: object
      properties:
        name:
          type: string
        email:
          type: string
          format: email
      required:
        - name
```

API validators use these schemas to check that request bodies and response objects conform.

## Form Generation

JSON Schema can drive form generation. A schema describing a data structure can be converted to HTML forms with field types, validation messages, and constraints:

- `type: boolean` → checkbox
- `enum` → dropdown/radio buttons
- `type: string, minLength: 5` → text input with required length
- `type: string, pattern: ...` → text input with regex validation

Tools like **JSON Schema Form** (various implementations) auto-generate forms. Useful for dynamic UIs, admin panels, and configuration tools.

## Code Generation

Schemas can generate strongly-typed classes in TypeScript, Go, Java, Python, etc.:

```ts
// Generated from JSON Schema
interface User {
  name: string;
  age?: number;
}

function validate(obj: unknown): obj is User {
  // generated validation logic
}
```

Tools: **quicktype**, **json-schema-to-typescript**, language-specific generators. Enables a schema-first workflow: define schema, generate client/server types, ensure consistency.

## Validators and Tooling

**AJV** (Another JSON Schema Validator, JavaScript/Node.js):
- Fast, modular, supports multiple draft versions
- Compiles schemas to code for performance
- CLI available

**Python:**
- `jsonschema` library (most common)
- `jsonschema-validation` (newer)

**Go:**
- `json-schema-proposal` (reference implementation)
- `gojsonschema` (popular third-party)

**Tooling:**
- **JSON Schema Store** (schemastore.org): community library of real-world schemas (GitHub Actions, VS Code settings, package.json, etc.)
- **VS Code extension**: syntax highlighting, validation for `.json` and `.yaml` files against schemas

## JSON Schema vs OpenAPI vs Protobuf

**JSON Schema:**
- Text-based, human-readable
- Runtime validation, no code generation required
- Flexible, extensible
- Inconsistent validator implementations

**OpenAPI:**
- REST API specification (includes routing, parameters, security)
- Uses JSON Schema for body/response modeling
- Integrates with API tooling (Swagger UI, code generators)

**Protobuf:**
- Binary, efficient, strict versioning rules
- Code generation mandatory
- gRPC integration
- Less flexible for exploratory APIs

**In practice:** JSON Schema for validation and form generation; OpenAPI for REST API documentation and client generation; Protobuf for high-performance RPC systems.

## When to Use JSON Schema

**Good fit:**
- Validating JSON documents (APIs, config)
- Documenting API contracts alongside OpenAPI
- Form generation from schema
- Dynamic validation without code changes

**Poor fit:**
- High-performance systems (Protobuf is faster)
- Strict backward compatibility requirements (Protobuf's rules are stricter)
- Non-JSON data (though YAML can be converted)

**Adoption:** Standard for REST APIs, config validation, and emerging in form generation frameworks.

## Related

See also: [api-documentation.md](api-documentation.md), [formats-openapi.md](formats-openapi.md), [formats-protobuf.md](formats-protobuf.md), [data-serialization-formats.md](data-serialization-formats.md), [typescript-runtime-validation.md](typescript-runtime-validation.md)