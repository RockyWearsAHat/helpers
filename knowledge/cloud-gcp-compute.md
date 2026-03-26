# GCP Compute Concepts — VMs, Containers & Serverless

## The Compute Abstraction Spectrum

Cloud computing platforms offer a range of compute abstractions, each trading control for operational simplicity. At one end, virtual machines provide near-bare-metal flexibility; at the other, serverless functions abstract away infrastructure entirely. GCP's compute portfolio spans this spectrum with distinct services occupying different points.

| Abstraction Level     | GCP Service     | What You Manage                       | What GCP Manages                             |
| --------------------- | --------------- | ------------------------------------- | -------------------------------------------- |
| Infrastructure (IaaS) | Compute Engine  | OS, runtime, app, scaling             | Hardware, hypervisor, network fabric         |
| Managed Kubernetes    | GKE             | Containers, pod specs, deployments    | Control plane, node provisioning (Autopilot) |
| Container serverless  | Cloud Run       | Container image, concurrency settings | Infrastructure, scaling, TLS                 |
| PaaS                  | App Engine      | Application code, config              | Runtime, scaling, infrastructure             |
| FaaS                  | Cloud Functions | Function code, trigger config         | Everything else                              |

The further up this stack a workload moves, the less operational surface area remains — but so does the degree of customization available.

## Compute Engine — Virtual Machines

### Machine Families and Types

Compute Engine organizes VM configurations into machine families, each optimized for different workload profiles:

- **General-purpose (E2, N2, N2D, N1, T2D, T2A, C3, C3D)** — balanced CPU-to-memory ratios suitable for broad workloads. The E2 series uses a dynamic resource management model that can share physical cores, offering cost efficiency for bursty workloads at the trade-off of less predictable performance isolation.
- **Compute-optimized (C2, C2D, H3)** — highest per-core performance. Suited for batch processing, HPC, gaming servers, and CPU-bound analytics. The trade-off: higher per-vCPU cost.
- **Memory-optimized (M1, M2, M3)** — large memory-to-CPU ratios (up to multiple TB). SAP HANA, in-memory databases, and genomics workloads gravitate here. Cost per GB of RAM decreases at these scales, but minimum instance sizes are large.
- **Accelerator-optimized (A2, A3, G2)** — GPU-attached instances for ML training, inference, rendering, and simulation. GPU availability is zone-constrained, and pricing models differ significantly from CPU-only instances.
- **Storage-optimized (Z3)** — high disk throughput and IOPS for data-intensive workloads.

Custom machine types allow specifying exact vCPU and memory combinations when predefined types waste resources, though at a modest premium over the nearest predefined equivalent.

### Preemptible and Spot VMs

GCP offers significantly discounted VMs drawn from excess capacity:

| Characteristic         | Preemptible VMs (legacy)          | Spot VMs                                      |
| ---------------------- | --------------------------------- | --------------------------------------------- |
| Max lifetime           | 24 hours hard limit               | No fixed limit (but can be reclaimed anytime) |
| Preemption notice      | 30-second warning via ACPI signal | 30-second warning via ACPI signal             |
| Discount               | ~60-91% off on-demand             | ~60-91% off on-demand                         |
| Availability guarantee | None                              | None                                          |
| Live migration         | Not supported                     | Not supported                                 |

Spot VMs supersede preemptible VMs conceptually. The architectural implication: workloads must be fault-tolerant, checkpointable, or decomposable into idempotent units. Batch processing, rendering farms, and stateless workers benefit most. Stateful services with strict uptime requirements are poor candidates unless combined with on-demand fallback capacity.

### Sole-Tenant Nodes

Sole-tenant nodes dedicate physical server hardware to a single project. Motivations include:

- **Compliance** — regulatory requirements mandating physical isolation from other tenants
- **Licensing** — per-core or per-socket software licenses (Oracle, SQL Server) that benefit from hardware affinity
- **Performance isolation** — workloads sensitive to noisy-neighbor effects

The trade-off is cost: sole-tenant nodes carry a premium and require capacity planning since the entire node's resources are reserved regardless of utilization.

### Managed Instance Groups (MIGs)

MIGs layer orchestration on top of individual VMs, providing:

- **Autoscaling** — scale instance count based on CPU utilization, load balancing capacity, Cloud Monitoring metrics, or schedules. The scaling decision has inherent lag: metric collection → cooldown evaluation → instance creation → startup → health check pass. This pipeline means MIGs respond to load changes on the order of minutes, not seconds.
- **Autohealing** — health checks (HTTP, HTTPS, TCP, SSL) detect unhealthy instances and recreate them automatically. The initial delay and check interval configuration requires balancing between fast failure detection and avoiding false positives during instance startup.
- **Rolling updates** — deploy new instance templates with configurable max-surge and max-unavailable parameters, enabling zero-downtime deployments for stateless workloads. Canary deployments can be modeled by running two MIGs with traffic splitting at the load balancer.
- **Regional distribution** — spread instances across zones within a region for availability without cross-region latency.

MIGs work best for homogeneous stateless workloads. Stateful workloads (databases, message brokers) require stateful MIGs with per-instance configs and preserved disks, adding operational complexity.

## GKE — Managed Kubernetes

Google Kubernetes Engine provides managed Kubernetes, handling the control plane (API server, etcd, scheduler, controller manager) while users define workloads as Kubernetes resources.

### Node Pools

Node pools group VM instances with identical configurations within a cluster. Different node pools can use different machine types, enabling heterogeneous clusters:

```
Cluster
├── system-pool (e2-medium, 3 nodes) — kube-system workloads
├── general-pool (n2-standard-4, autoscale 2-20) — application workloads
├── gpu-pool (a2-highgpu-1g, autoscale 0-8) — ML inference
└── spot-pool (n2-standard-8, Spot VMs, autoscale 0-50) — batch jobs
```

Node auto-provisioning can create new node pools dynamically when existing pools cannot satisfy pod scheduling requirements, further reducing manual capacity planning.

### Autopilot vs Standard Mode

| Dimension              | Standard Mode                                    | Autopilot Mode                          |
| ---------------------- | ------------------------------------------------ | --------------------------------------- |
| Node management        | User provisions and configures node pools        | GCP manages nodes entirely              |
| Billing unit           | Per node (VM), regardless of pod utilization     | Per pod resource request                |
| Security posture       | User responsible for node OS hardening           | GCP enforces hardened node config       |
| Customization          | Full access to node configuration, SSH           | Restricted — no SSH, limited DaemonSets |
| Multi-tenancy controls | Full flexibility (taints, tolerations, affinity) | Managed isolation                       |
| GPU/TPU workloads      | Full control over accelerator node pools         | Supported with constraints              |

Autopilot eliminates node-level operations at the cost of reduced flexibility. Standard mode suits teams that need fine-grained control over the node layer — custom kernel parameters, specific OS images, privileged DaemonSets, or specialized hardware configurations.

### GKE and the Broader Ecosystem

GKE integrates with GCP's networking model (VPC-native clusters, Cloud NAT, Internal Load Balancers), security model (Workload Identity for pod-level IAM, Binary Authorization for image signing), and observability stack (Cloud Logging, Cloud Monitoring, managed Prometheus). These integrations reduce operational burden but create coupling to GCP-specific primitives.

## Cloud Run — Container Serverless

Cloud Run executes stateless containers without requiring cluster management. The model:

1. Deploy a container image that listens on a port
2. Cloud Run handles TLS termination, request routing, and autoscaling
3. Scale to zero when no requests arrive; scale up under load

### Cold Starts

When a Cloud Run service scales from zero (or adds instances beyond the current pool), new container instances must start. Cold start latency depends on:

- **Image size** — larger images take longer to pull and decompress
- **Runtime initialization** — JVM startup, dependency injection framework bootstrap, connection pool establishment
- **Minimum instances** — configuring a minimum instance count above zero eliminates cold starts at the cost of always-on billing

Cold start considerations create tension between cost optimization (scale-to-zero) and latency sensitivity. Services handling synchronous user requests may need minimum instances; asynchronous event processors may tolerate occasional cold starts.

### Concurrency Model

Cloud Run can deliver multiple concurrent requests to a single container instance (configurable, default 80, max 1000). This differs from Cloud Functions' default one-request-per-instance model:

- **High concurrency** — efficient for I/O-bound workloads (API proxies, web servers). One instance handles many requests, reducing instance count and cost.
- **Low concurrency (1)** — appropriate for CPU-intensive or non-thread-safe workloads. Each request gets a dedicated instance, similar to FaaS behavior.

The concurrency setting interacts with autoscaling: Cloud Run provisions instances based on concurrent request count divided by the concurrency setting, with additional factors for CPU utilization and request latency targets.

### Cloud Run Jobs

Beyond services (which respond to HTTP requests), Cloud Run Jobs execute container workloads to completion — batch processing, data pipelines, scheduled tasks. Jobs can run multiple parallel tasks, each executing the same container with different arguments or environment configurations.

## Cloud Functions — Event-Driven Compute

Cloud Functions execute code in response to events without managing servers or containers. Two generations exist with different characteristics:

| Aspect            | 1st Gen                             | 2nd Gen (built on Cloud Run)         |
| ----------------- | ----------------------------------- | ------------------------------------ |
| Concurrency       | 1 request per instance              | Up to 1000 concurrent requests       |
| Max timeout       | 9 minutes (HTTP), 9 minutes (event) | 60 minutes (HTTP), 9 minutes (event) |
| Instance size     | Up to 8 GB RAM, 2 vCPU              | Up to 32 GB RAM, 8 vCPU              |
| Traffic splitting | Not supported                       | Supported (revision-based)           |
| Minimum instances | Supported                           | Supported                            |

### Invocation Models

- **HTTP functions** — triggered by HTTP requests, return HTTP responses. Suited for webhooks, lightweight APIs, and synchronous integrations.
- **Event-driven functions** — triggered by CloudEvents from Pub/Sub, Cloud Storage, Firestore, and other GCP services. Processing is asynchronous; the function receives the event payload.
- **Background functions (1st gen)** — legacy event model, predating CloudEvents standardization.

### Execution Guarantees

Event-driven Cloud Functions provide at-least-once delivery. Functions may be invoked more than once for the same event during retries. Idempotency in function logic is essential to avoid duplicate side effects. Retry behavior is configurable — enabling retries means failed invocations are retried with exponential backoff, while disabling retries means a single attempt per event.

## App Engine — The Original PaaS

App Engine predates the container and serverless terminology, offering a fully managed platform in two flavors:

| Dimension      | Standard Environment                      | Flexible Environment                 |
| -------------- | ----------------------------------------- | ------------------------------------ |
| Scaling model  | Scale to zero, fast startup               | Minimum 1 instance, VM-based scaling |
| Runtime        | Sandbox with specific language versions   | Any Docker container                 |
| Pricing        | Per-instance-hour, free tier available    | Per-VM-hour, no free tier            |
| Network access | Limited (no raw sockets in some runtimes) | Full network access                  |
| SSH access     | Not available                             | Available                            |
| Local disk     | In-memory only                            | Ephemeral disk available             |
| Startup time   | Milliseconds to seconds                   | Minutes (VM provisioning)            |

Standard Environment suits lightweight web applications and APIs that benefit from rapid scaling and cost efficiency. Flexible Environment suits workloads requiring custom runtimes, native dependencies, or background processing — effectively managed VMs presented through App Engine's deployment model.

App Engine's traffic splitting between versions, automatic versioning, and built-in memcache (Standard) were innovative, though many of these capabilities now exist across other compute services in more flexible forms.

## Choosing the Right Compute Abstraction

No single axis determines the optimal compute service. Key considerations:

### Workload Characteristics Matrix

| Factor                     | Favors VMs                   | Favors Containers/GKE       | Favors Serverless       |
| -------------------------- | ---------------------------- | --------------------------- | ----------------------- |
| Stateful, long-running     | Strong fit                   | Good fit with StatefulSets  | Poor fit                |
| Stateless request/response | Overprovisioned              | Good fit                    | Optimal fit             |
| Bursty traffic             | Slow scaling                 | Moderate scaling            | Fast scaling            |
| GPU/specialized hardware   | Full control                 | Node pool flexibility       | Limited options         |
| Legacy applications        | Direct lift-and-shift        | Requires containerization   | Requires refactoring    |
| Microservices              | Operational overhead         | Natural fit                 | Good for small services |
| Team Kubernetes expertise  | Not required                 | Essential                   | Not required            |
| Cost at steady high load   | Predictable, efficient       | Efficient with right-sizing | Can be expensive        |
| Cost at variable/low load  | Wasteful if over-provisioned | Moderate (node overhead)    | Pay-per-use efficiency  |

### Hybrid Approaches

Many architectures combine multiple compute services:

- GKE for core microservices + Cloud Functions for event processing + Cloud Run for infrequent batch APIs
- Compute Engine for stateful databases + GKE for application tier + Cloud CDN for static content
- App Engine for legacy monolith + Cloud Run for new microservices during migration

## Cost Models Across the Spectrum

| Service             | Billing Unit                     | Committed Use Discounts             | Sustained Use Discounts   | Scale-to-Zero                |
| ------------------- | -------------------------------- | ----------------------------------- | ------------------------- | ---------------------------- |
| Compute Engine      | Per-second VM runtime            | 1-year, 3-year CUDs (up to 57% off) | Automatic (up to 30% off) | No (minimum 1 minute charge) |
| GKE Standard        | Node VMs + cluster fee           | Via underlying Compute Engine CUDs  | Via underlying VMs        | No (nodes always running)    |
| GKE Autopilot       | Per-pod resource request         | Pod-level CUDs available            | Not applicable            | Pod level only               |
| Cloud Run           | Per-request + CPU/memory-seconds | Not available                       | Not applicable            | Yes                          |
| Cloud Functions     | Per-invocation + compute-seconds | Not available                       | Not applicable            | Yes                          |
| App Engine Standard | Per-instance-hour                | Not available                       | Not applicable            | Yes                          |
| App Engine Flexible | Per-VM-hour                      | Not available                       | Not applicable            | No                           |

Committed Use Discounts (CUDs) require predicting resource needs — beneficial for stable baselines, wasteful if actual usage falls below commitments. Sustained Use Discounts apply automatically to Compute Engine VMs running more than 25% of a month, rewarding consistent usage without upfront commitment.

The cost-optimal architecture often mixes services: CUD-backed Compute Engine or GKE for predictable baseline load, with Cloud Run or Cloud Functions absorbing traffic spikes elastically.

## Operational Considerations

### Startup and Scaling Latency

```
Compute Engine VM:     ~30-90 seconds (boot + health check)
GKE Pod (warm node):   ~5-30 seconds (image pull + readiness)
GKE Pod (new node):    ~60-120 seconds (node provision + pod start)
Cloud Run (cold):      ~0.5-10 seconds (depends on image/runtime)
Cloud Run (warm):      ~10-50 milliseconds
Cloud Functions:       ~100ms-10 seconds (cold), ~10-50ms (warm)
```

These ranges are approximate and vary with configuration, image size, and runtime initialization. The gap between cold and warm response determines how aggressively a service can scale to zero without impacting user experience.

### Portability and Lock-In Spectrum

Compute Engine VMs offer the most portability (standard VMs). GKE workloads are portable across Kubernetes providers with effort, though GCP-specific integrations (Workload Identity, Config Connector) create soft coupling. Cloud Run services, being standard containers, are reasonably portable. Cloud Functions and App Engine create tighter coupling through proprietary APIs and deployment models.

The lock-in trade-off mirrors the management trade-off: more managed services reduce operational burden but increase dependency on provider-specific abstractions.
