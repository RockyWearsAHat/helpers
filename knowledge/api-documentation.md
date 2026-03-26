# API Documentation — OpenAPI, AsyncAPI, API-First Design & Interactive Docs

## Overview

API documentation is the user manual for developers consuming the API. Quality separates excellent APIs (learned in minutes) from frustrating ones (weeks of struggle). Modern API documentation is machine-readable (OpenAPI/AsyncAPI), interactive (Swagger UI, Redoc), generative (SDKs auto-built from specs), and testable (spec validation, mock servers). Documentation *is* specification; the two are inseparable.

## OpenAPI Specification (Swagger)

Standard, language-agnostic format for describing REST APIs. Version 3.1 (latest) is YAML/JSON with full JSON Schema support.

### Minimal Example

```yaml
openapi: 3.1.0
info:
  title: Order API
  version: 1.0.0
  description: Manage orders
  contact:
    name: Support
    email: support@example.com
  license:
    name: MIT
servers:
  - url: https://api.example.com/v1
    description: Production
  - url: https://staging-api.example.com/v1
    description: Staging
paths:
  /orders:
    get:
      summary: List orders
      operationId: listOrders
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
            default: 20
          description: Max results
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Order'
        '401':
          description: Unauthorized
          content:
            application/problem+json:
              schema:
                $ref: '#/components/schemas/ProblemDetail'
    post:
      summary: Create order
      operationId: createOrder
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateOrderRequest'
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Order'
        '422':
          description: Validation error
          content:
            application/problem+json:
              schema:
                $ref: '#/components/schemas/ProblemDetail'
components:
  schemas:
    Order:
      type: object
      required:
        - id
        - userId
        - createdAt
      properties:
        id:
          type: string
          description: Unique order ID
        userId:
          type: string
        status:
          type: string
          enum: [pending, shipped, delivered, cancelled]
        total:
          type: number
          format: double
        createdAt:
          type: string
          format: date-time
    CreateOrderRequest:
      type: object
      required:
        - items
      properties:
        items:
          type: array
          items:
            type: object
            properties:
              productId:
                type: string
              quantity:
                type: integer
    ProblemDetail:
      type: object
      properties:
        type:
          type: string
        status:
          type: integer
        detail:
          type: string
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
    apiKey:
      type: apiKey
      in: header
      name: X-API-Key
security:
  - bearerAuth: []
  - apiKey: []
```

### Key Sections

| Section          | Purpose                                              |
| ---------------- | ---------------------------------------------------- |
| `openapi`        | Spec version (3.1.0 current)                         |
| `info`           | Title, version, description, contact, license       |
| `servers`        | Base URLs (production, staging)                      |
| `paths`          | API endpoints and operations (GET, POST, etc.)      |
| `components`     | Reusable schemas, security schemes, parameters      |
| `security`       | Default authentication for all operations           |

### Schemas & JSON Schema Support

OpenAPI 3.1 fully supports JSON Schema, enabling:

- **$ref:** `$ref: '#/components/schemas/Order'` — references reusable schema
- **oneOf/anyOf/allOf:** polymorphic types, composition
- **Discriminator:** disambiguate oneOf variants
- **Examples:** `example: { "id": "order_123", "status": "shipped" }`
- **Constraints:** `minLength, maxLength, pattern, minimum, maximum`

```yaml
components:
  schemas:
    Response:
      oneOf:
        - $ref: '#/components/schemas/SuccessResponse'
        - $ref: '#/components/schemas/ErrorResponse'
      discriminator:
        propertyName: type
        mapping:
          success: '#/components/schemas/SuccessResponse'
          error: '#/components/schemas/ErrorResponse'
```

## AsyncAPI

OpenAPI for event-driven/async APIs (message queues, webhooks, pub/sub). Describes message-based communication.

### Structure

```yaml
asyncapi: 3.0.0
info:
  title: Order Events
  version: 1.0.0
channels:
  orders/created:
    address: orders.created
    messages:
      OrderCreated:
        payload:
          type: object
          properties:
            orderId:
              type: string
            customerId:
              type: string
            amount:
              type: number
            createdAt:
              type: string
              format: date-time
  orders/updated:
    address: orders.updated
    messages:
      OrderUpdated:
        payload:
          $ref: '#/components/schemas/Order'
servers:
  kafka:
    host: kafka.example.com:9092
    protocol: kafka
    protocolVersion: '3.0'
  rabbitmq:
    host: rabbitmq.example.com:5672
    protocol: amqp
    security:
      - $ref: '#/components/securitySchemes/basicAuth'
```

| Section    | Purpose                      |
| ---------- | ---------------------------- |
| `channels` | Topics/queues and messages   |
| `servers`  | Message broker connection    |
| `messages` | Payload schema for each      |
| `bindings` | Protocol-specific details    |

## API-First Design

Design API contract *before* implementation. Benefits:

1. **Early feedback** — Clients review API shape; catch design issues before code
2. **Parallel work** — Mock server allows frontend to develop while backend builds
3. **Contract as law** — Spec is the reference; implementation follows
4. **Testing** — Validated against spec; drift detected early
5. **Documentation** — Generated from spec; always in sync

### Process

1. **Design:** Write OpenAPI spec in editor or Swagger Editor
2. **Review:** Share spec; get feedback on resource naming, error handling, pagination
3. **Mock:** Generate mock server; test against it
4. **Implement:** Backend implements to match spec
5. **Validate:** Runtime spec validation ensures implementation stays on spec
6. **Generate:** SDKs, docs, tests auto-generated from spec

## Interactive Documentation Generators

### Swagger UI

Official OpenAPI visualizer. Interactive, browser-based, allows "Try it out."

```html
<link rel="stylesheet" type="text/css" href="https://cdnjs.cloudflare.com/ajax/libs/swagger-ui/4.18.3/swagger-ui.min.css"/>
<script src="https://cdnjs.cloudflare.com/ajax/libs/swagger-ui/4.18.3/swagger-ui.bundle.min.js"></script>
<script>
SwaggerUIBundle({
  url: "https://api.example.com/openapi.json",
  dom_id: '#swagger-ui',
  presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
  layout: "BaseLayout"
})
</script>
```

**Features:**
- Interactive "Try it out" button (make real requests)
- Request/response examples
- Schema visualization
- Syntax highlighting
- Authentication support

### Redoc

Alternative documentation generator, optimized for readability.

```html
<link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
<style>
  body { margin: 0; padding: 0; }
</style>
<redoc spec-url="https://api.example.com/openapi.json"></redoc>
<script src="https://cdn.jsdelivr.net/npm/redoc@latest/bundles/redoc.standalone.js"></script>
```

**Strengths:**
- Cleaner, three-column layout
- Better for reading long docs
- No "Try it out" (read-only)
- Fast, static rendering

### Alternatives

- **RapidOC** — lightweight, customizable
- **Elements** — deprecated but still used
- **StopLight Elements** — hosted + self-hosted options

## SDK Generation

Auto-generate client libraries from OpenAPI spec.

### Tools

- **openapi-generator** (Java-based, many languages)
- **swagger-codegen** (older, less maintained)
- **Speakeasy** (modern, maintains own specs)
- **fern** (schema-driven code generation)
- **liblab** (CLI tools for SDK generation)

### Example

```bash
npx @openapitools/openapi-generator-cli generate \
  -i openapi.json \
  -g typescript \
  -o ./client-sdk
npm publish ./client-sdk
```

Generated SDK includes:

- **Type definitions** from schema
- **API client class** with methods for each operation
- **Error handling** based on response definitions
- **Examples** from spec
- **Docs** extracted from descriptions
- **TypeScript/null safety** if language supports

## Schema Validation & Spec Compliance

### Runtime Validation

Validate requests/responses against OpenAPI spec. Catches bugs early.

Tools:
- **prism** (Stoplight) — mock server + validation
- **openapi-middleware** (Node.js Express middleware)
- **connexion** (Python Flask wrapper)
- **Goreleaser/gorilla** (Go)

Example (Express middleware):

```javascript
const OpenApiValidator = require('express-openapi-validator');

app.use(OpenApiValidator.middleware({
  apiSpec: './openapi.json',
  validateRequests: true,
  validateResponses: true,
  validateFormats: 'full'
}));

app.get('/orders', (req, res) => {
  // If request doesn't match spec, validator rejects before handler runs
  res.json({ orders: [...] });
});
```

### Spec Linting

Validate spec syntax and best practices.

Tools:
- **spectral** (Stoplight, rule-based)
- **swagger-cli** (basic validation)
- **dredd** (API testing against spec)

Example rules (Spectral):

```yaml
rules:
  operation-description:
    description: Every operation must have a description
    given: $.paths[*][get,post,put,patch,delete]
    severity: error
    then:
      field: description
      function: truthy
  path-parameters-camelCase:
    description: Path parameters must be camelCase
    given: $.paths[*].parameters[?(@.in == 'path')]
    severity: warn
    then:
      field: name
      function: pattern
      functionOptions:
        match: "^[a-z][a-zA-Z0-9]*$"
```

## Mock Servers

Develop against spec without implementing backend.

### Prism (Stoplight)

```bash
npm install -g @stoplight/prism-cli
prism mock openapi.json --host 0.0.0.0 --port 3000
```

Server responds to requests matching spec:
- Returns 200 with example response
- Validates request against schema
- Rejects out-of-spec requests with 400

### Custom Mock

```python
from openapi_spec_validator import validate_spec
from openapi_spec_validator.exceptions import OpenAPIValidationError

# Validate spec
spec = yaml.safe_load(open('openapi.json'))
validate_spec(spec)

# Serve mock responses
@app.get('/orders')
def list_orders():
    # Extract example from spec
    schema = spec['paths']['/orders']['get']['responses']['200']['schema']
    return schema.get('example', {})
```

## Documentation Testing

Docs get out of sync; test them.

### Dredd (API Testing)

Validates that implementation matches OpenAPI spec.

```bash
npm install -g dredd
dredd openapi.json http://localhost:3000
```

Sends requests for each operation; compares real response against schema.

### Examples in Spec

```yaml
responses:
  '200':
    description: Success
    content:
      application/json:
        schema:
          $ref: '#/components/schemas/Order'
        examples:
          standard:
            value:
              id: order_123
              status: shipped
              total: 99.99
          premium:
            value:
              id: order_456
              status: delivered
              total: 299.99
```

## Documentation Maintenance

### Single Source of Truth

- Spec is authoritative; docs generated from it
- Avoid manual docs separate from spec (they diverge)
- Use `description` fields generously in spec
- Examples in spec keep docs current

### Changelog & Versioning

Include changelog in spec:

```yaml
info:
  version: 1.2.0
  x-changelog:
    1.2.0:
      - Added /orders/{id}/tracking endpoint
      - Deprecated X-Rate-Limit-* headers (use RateLimit-* instead)
    1.1.0:
      - Added pagination to list operations
```

## Related Concepts

See also: [api-design](api-design.md), [api-versioning](api-versioning.md), [api-error-handling](api-error-handling.md), [process-documentation](process-documentation.md), [process-technical-writing](process-technical-writing.md)