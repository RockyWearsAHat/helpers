# Google Cloud Run — Container-Based Serverless, Request Handling & Concurrency

## Overview

**Google Cloud Run** is a fully managed, serverless compute platform for stateless HTTP containers. Developers package code in a container image and deploy; Cloud Run handles provisioning, scaling, infrastructure, and auto-shutdown when idle. Pricing is consumption-based: only running containers incur cost.

## Core Model

### Containers as the Unit of Deployment

Unlike cloud functions (language-specific), Cloud Run accepts any OCI container:
- Custom runtimes and languages
- Dependency bundling in image
- Simplified supply chain vs. monolithic binaries
- Single binary deployments common (Go, Rust, Bun)

### Request Handling & Concurrency

**Concurrency model**:
- Each Container Instance processes **up to N** concurrent requests (default: 80)
- Requests beyond N-concurrent queue and wait
- Exceeding max concurrency is queued, not rejected

**Request lifetime**:
- Single HTTP request routed to one container instance
- Container handles request-response cycle
- No persistent connection reuse across requests
- Timeout: 60 minutes (configurable down to 15 seconds)

**Stateless requirement**:
- Shared mutable state not possible across requests (different instances)
- External storage (Cloud Firestore, Memorystore) for state
- Distributed tracing correlates requests despite multi-instance processing

### Cold Starts

When demand increases, new container instances spin up:
- Image downloaded, container runtime started, application initialized
- ~0.5-2s typical for minimal images; longer for complex runtimes
- Mitigation: keep images small, minimize startup code, use compiled languages

## Scaling & Min/Max Instances

### Autoscaling Behavior

Cloud Run scales from zero:
- No instances running when idle (zero cost)
- Requests arrive → new instances spun up
- As load increases, more instances created
- Scale down when demand drops

### Min Instances

Reserve minimum instances to avoid cold starts:
- Cost: running instances charged regardless of traffic
- Trade-off: improved latency + higher baseline cost
- Suitable for latency-sensitive services or predictable baseline load

### Max Instances

Limit maximum concurrent instances:
- Prevents runaway costs or resource exhaustion
- Default: auto (limited by project quota)
- If max reached, additional requests queue (not rejected)
- Consider coupled with request timeout for queue SLA

## Memory & CPU

### CPU Allocation

- Default: 1 CPU (shared among concurrent requests)
- Always-on option: 2 CPUs (not suspended between requests)
- CPU shared by concurrent requests unless always-on enabled

### Memory

- Range: 128 MB to 32 GB per instance
- Higher memory often correlates with CPU allocation in billing
- Affects cold start time and runtime performance

### Performance Profile

Memory and CPU impact latency and throughput:
- 128 MB: function-like, slow, low cost
- 1-4 GB: typical web services, balanced
- 8+ GB: data-intensive, ML workloads

## Cloud Run Jobs

Separate offering for batch and asynchronous work:

- **Services**: handle HTTP requests (above)
- **Jobs**: run to completion and exit, triggered by schedule or event
- Payment: based on CPU-seconds and memory-seconds, not always-on instances
- Suitable for: data processing, backups, scheduled tasks

Example: nightly stats aggregation job processes data, writes to storage, then terminates.

## Deployment & Traffic Management

### Revisions

Each deployment creates a new revision:
- Blue-green deployments trivial (redirect traffic)
- Instant rollback to previous revision
- Traffic splitting: 90% to current, 10% to canary
- Immutable image per revision

### Traffic Splitting

Route % of requests to multiple revisions:
- Canary deployments (5% to new version)
- A/B testing
- Gradual rollouts

Traffic split configured per service; Cloud Run routes requests based on policy.

## Integration with GCP Ecosystem

### Pub/Sub

Cloud Run can be triggered by Pub/Sub messages:
- Topic → subscription → Cloud Run service
- At-least-once delivery (retry on failure)
- Filter by attributes

### Cloud Tasks

Enqueue work for Cloud Run services:
- HTTP POST to Cloud Run URL
- Configurable retries and rate limiting
- Useful for: distributed job queues, deferred processing

### Cloud Scheduler

Cron jobs triggering Cloud Run:
- Scheduled HTTP POST to service
- Can pass arguments via Pub/Sub or query params

### Cloud Trace & Logging

- Automatic request logging to Cloud Logging
- Structured traces exportable to OpenTelemetry
- Service integrates with Application Performance Monitoring (APM)

## Security & Identity

### Unauthenticated vs. Authenticated Access

- **Unauthenticated (public)**: any internet client can invoke (API, webhook)
- **Requires authentication**: limited to IAM-authorized identities (service accounts, users)

### Service Account Identity

Cloud Run services execute with a service account:
- Binds IAM roles (read Cloud Storage, call Cloud APIs, etc.)
- Workload Identity for accessing GCP services from containerized code

### Custom Domains

- Cloud Run URLs are auto-generated (hash-based)
- Map custom domain via Cloud Domains or external registrar
- TLS automatic for custom domains

## Comparison to Alternatives

### vs. Cloud Functions
- Cloud Functions: language-specific, built-in triggers
- Cloud Run: arbitrary containers, HTTP-only (triggers via integration)

### vs. App Engine
- App Engine: multi-instance management, traffic splitting built-in
- Cloud Run: simpler API, faster deployment, more flexible containers
- App Engine Standard still used for legacy apps

### vs. GKE
- GKE: full Kubernetes control, long-running services, complex orchestration
- Cloud Run: minimal ops, pay-per-request, no cluster management

## Common Patterns

### Stateless Web Service

Express/FastAPI server running in a container, responds to HTTP. Scale automatically with traffic. Typical use case.

### Async Task Queue

Cloud Tasks or Pub/Sub enqueues work → Cloud Run processes → returns HTTP 200. External storage persists state.

### Webhook Handler

Third-party service posts events → Cloud Run processes → triggers other GCP services (BigQuery inserts, Cloud Logging, etc.). Scales invisibly.

### Scheduled Job

Cloud Scheduler periodically POST to Cloud Run endpoint → job processes data and terminates. Change Feed or event bus pattern.

## Related

See also: [cloud-gcp-compute.md](cloud-gcp-compute.md), [cloud-serverless-patterns.md](cloud-serverless-patterns.md), [architecture-async-messaging.md](architecture-async-messaging.md), [cloud-finops.md](cloud-finops.md)