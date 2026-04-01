# OpenAPI Specification — Paths, Operations, Components & API Documentation

**OpenAPI** (formerly Swagger) is a standard for describing REST APIs in a machine-readable format. An OpenAPI document defines endpoints, parameters, request/response bodies, security schemes, and constraints. Tools consume OpenAPI specs to generate client SDKs, server stubs, documentation (Swagger UI), mock servers, and API validators. OpenAPI is the industry standard for REST API contracts.

## Document Structure

An OpenAPI document is JSON or YAML describing a single API:

```yaml
openapi: 3.1.0
info:
  title: Pet Store API
  version: 1.0.0
paths:
  /pets:
    get:
      summary: List all pets
      operationId: listPets
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Pet'
components:
  schemas:
    Pet:
      type: object
      properties:
        id:
          type: integer
        name:
          type: string
      required:
        - id
        - name
```

The top-level properties: `openapi` (version), `info` (title, version, description), `paths` (endpoints), `servers` (base URLs), `components` (reusable schemas and responses), `security` (default auth schemes).

## Paths and Operations

**Paths** map URL routes to operations. Each path can define operations for HTTP methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, TRACE):

```yaml
/pets/{petId}:
  get:
    operationId: getPet
    parameters:
      - name: petId
        in: path
        required: true
        schema:
          type: integer
    responses:
      '200':
        description: Pet found
      '404':
        description: Pet not found
  put:
    operationId: updatePet
    requestBody:
      required: true
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Pet'
    responses:
      '200':
        description: Pet updated
```

Path parameters (e.g., `{petId}`) are extracted from the URL. Each operation can define:
- `operationId`: unique identifier for the operation (used in code generation)
- `summary`, `description`: documentation
- `deprecated`: mark as deprecated
- `tags`: categorize operations (for grouping in documentation)

## Parameters

Parameters can appear in multiple locations:

```yaml
parameters:
  - name: status
    in: query          # query, header, path, cookie, or form in HTTP
    schema:
      type: string
      enum: [available, pending, sold]
    required: true
    description: Filter by status
  - name: X-Custom-Header
    in: header
    schema:
      type: string
  - name: skip
    in: query
    schema:
      type: integer
      default: 0
```

- `in: query`: URL query string (e.g., `?status=available`)
- `in: path`: URL path segment (must be wrappedby `{name}` in the path)
- `in: header`: HTTP header
- `in: cookie`: HTTP cookie
- `style` (e.g., `simple`, `form`): how arrays/objects are serialized

## Request Body

The `requestBody` defines what data clients send:

```yaml
requestBody:
  required: true
  content:
    application/json:
      schema:
        $ref: '#/components/schemas/Pet'
    application/xml:
      schema:
        $ref: '#/components/schemas/Pet'
  encoding:
    image:
      contentType: image/png
```

`content` maps MIME types to schemas. Multiple MIME types indicate the API accepts alternative serializations. `encoding` specifies how complex fields are serialized (e.g., file uploads, multipart form data).

## Responses

Each operation must define responses for different status codes:

```yaml
responses:
  '200':
    description: Success
    content:
      application/json:
        schema:
          $ref: '#/components/schemas/Pet'
  '400':
    description: Invalid input
    content:
      application/json:
        schema:
          $ref: '#/components/schemas/ErrorResponse'
  '401':
    description: Unauthorized
    headers:
      WWW-Authenticate:
        schema:
          type: string
  default:
    description: Unexpected error
    content:
      application/json:
        schema:
          $ref: '#/components/schemas/ErrorResponse'
```

Each status code has a description, optional content (response body), and optional headers. `default` catches unexpected status codes. Responses can include headers (e.g., `WWW-Authenticate` for 401).

## Components: Reusable Definitions

The `components` section centralizes reusable definitions:

```yaml
components:
  schemas:
    Pet:
      type: object
      properties:
        id:
          type: integer
        name:
          type: string
      required:
        - name
    ErrorResponse:
      type: object
      properties:
        code:
          type: integer
        message:
          type: string

  responses:
    UnauthorizedError:
      description: Unauthorized
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

  parameters:
    limitParam:
      name: limit
      in: query
      schema:
        type: integer
        default: 10

  headers:
    X-Rate-Limit:
      description: Requests per hour
      schema:
        type: integer

  securitySchemes:
    api_key:
      type: apiKey
      in: header
      name: X-API-Key
    oauth2:
      type: oauth2
      flows:
        authorizationCode:
          authorizationUrl: https://example.com/oauth/authorize
          tokenUrl: https://example.com/oauth/token
          scopes:
            read: Read access
            write: Write access
```

Reusable schemas, responses, parameters, headers, and security schemes reduce duplication. References use `$ref: '#/components/schemas/Pet'` to link.

## Security Schemes

Define authentication methods:

```yaml
security:
  - api_key: []
  - oauth2: [read, write]

components:
  securitySchemes:
    api_key:
      type: apiKey
      name: X-API-Key
      in: header
    oauth2:
      type: oauth2
      flows:
        implicit:
          authorizationUrl: https://example.com/authorize
          scopes:
            read: Read data
            write: Write data
    http:
      type: http
      scheme: bearer
      bearerFormat: JWT
```

Security schemes: `apiKey`, `http` (Bearer, Basic), `oauth2`, `openIdConnect`. Top-level `security` sets default; operations can override.

## Discriminator: Handling Polymorphism

When `anyOf`/`oneOf` schemas represent variants, use `discriminator` to clarify selection:

```yaml
components:
  schemas:
    Animal:
      type: object
      discriminator:
        propertyName: petType
      oneOf:
        - $ref: '#/components/schemas/Cat'
        - $ref: '#/components/schemas/Dog'
    Cat:
      type: object
      properties:
        petType:
          const: cat
        meow:
          type: string
    Dog:
      type: object
      properties:
        petType:
          const: dog
        bark:
          type: string
```

The `discriminator` tells tooling that the `petType` field determines which schema applies. Code generators use this to emit correct types instead of ambiguous unions.

## Links and Callbacks

**Links** define runtime relationships between operations:

```yaml
responses:
  '201':
    description: Pet created
    content:
      application/json:
        schema:
          $ref: '#/components/schemas/Pet'
    links:
      GetPetById:
        operationId: getPet
        parameters:
          petId: $response.body#/id
        requestBody: null
```

After a POST creates a pet, the link suggests that clients can use the returned `id` to call GET /pets/{petId}. Links are primarily documentation; tools rarely auto-follow them.

**Callbacks** define webhooks (server-to-client notifications):

```yaml
requestBody:
  content:
    application/json:
      schema:
        $ref: '#/components/schemas/Subscription'
callbacks:
  myWebhook:
    '{$request.body#/callbackUrl}':
      post:
        requestBody:
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Event'
        responses:
          '200':
            description: Webhook processed
```

Callbacks document where the server will POST events. Critical for event-driven APIs; often underused due to tooling limitations.

## OpenAPI Versions

- **2.0** (Swagger): Earlier standard. Still widely used. Limited features (no request body as first-class, no discriminator).
- **3.0.x:** Current mainstream. 3.0.0 (2017), 3.0.3 (latest in 3.0 line).
- **3.1.0:** Latest (released 2021). Full JSON Schema 2020-12 support (vs. OpenAPI's custom schema dialect), better multipart support.

**Migration friction:** 3.1 broke some tooling due to the shift to JSON Schema. Many tools still defaultto 3.0.

## Swagger UI and Documentation

**Swagger UI** renders OpenAPI specs as interactive documentation:

```html
<script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@3"></script>
<link rel="stylesheet" type="text/css" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@3/swagger-ui.css">

<div id="swagger-ui"></div>
<script>
  SwaggerUIBundle({
    url: "https://petstore.example.com/openapi.json",
    dom_id: '#swagger-ui',
    presets: [SwaggerUIBundle.presets.apis]
  });
</script>
```

Users see endpoints, parameters, try them out with live requests, see responses. Free, standard UI; easily customizable.

Alternatives: **Redoc** (beautiful, mobile-friendly), **API Extractor** (VSCode integration), custom implementations.

## Code Generation

Tools consume OpenAPI specs to generate client/server code:

- **openapi-generator**: Java-based, supports 50+ languages (TypeScript, Go, Python, Java, etc.)
- **swagger-codegen**: Older, similar scope
- **protobufs**: gRPC alternative for specific workflows
- Language-specific: **OpenAPI Generator CLI** for npm, Python packages

Example (TypeScript client from OpenAPI):

```bash
npx @openapitools/openapi-generator-cli generate \
  -i https://petstore.swagger.io/openapi.json \
  -g typescript-fetch \
  -o ./generated
```

Generated types match schema definitions; HTTP client code handles routing and serialization.

## When to Use OpenAPI

**Good fit:**
- REST APIs requiring documentation and client generation
- Microservices with multiple clients
- Public APIs where documentation is critical
- API-first development (design spec, generate code)

**Poor fit:**
- High-performance systems (overhead of semantic documentation; Protobuf/gRPC more efficient)
- GraphQL APIs (use schema language instead)
- Real-time APIs (Async API standard better fits)
- Simple internal services (overhead vs. handwritten docs)

**Adoption:** Standard for REST APIs, especially public/microservices. Ubiquitous in enterprise and startup API development.

## Related

See also: [api-documentation.md](api-documentation.md), [api-design.md](api-design.md), [formats-json-schema.md](formats-json-schema.md), [web-api-patterns.md](web-api-patterns.md), [api-versioning.md](api-versioning.md), [networking-api-gateway-protocols.md](networking-api-gateway-protocols.md)