# Azure Compute Concepts — VMs, Containers & Serverless

Azure's compute portfolio spans a spectrum from full infrastructure control (virtual machines) through managed containers (AKS, Container Instances) to fully abstracted serverless (Functions). Each position on this spectrum trades operational control for management overhead reduction, and the appropriate choice depends on workload characteristics, team expertise, and organizational constraints.

## The Compute Spectrum

| Abstraction Level       | Service          | Control                 | Ops Overhead                            | Cold Start               | Cost Model               |
| ----------------------- | ---------------- | ----------------------- | --------------------------------------- | ------------------------ | ------------------------ |
| Infrastructure          | Virtual Machines | Full OS access          | Highest — patching, networking, scaling | None (always running)    | Per-second/reserved      |
| Managed infra           | VM Scale Sets    | OS + auto-scaling rules | Medium — still OS patching              | Provisioning lag         | Per-instance             |
| Container orchestration | AKS              | Container-level         | Medium — cluster management             | Pod scheduling           | Node-level billing       |
| Container instances     | ACI              | Container-level         | Low — no cluster                        | Container pull time      | Per-second, per-resource |
| PaaS                    | App Service      | Application-level       | Low                                     | Warm instances available | Tier-based               |
| Serverless              | Functions        | Function-level          | Minimal                                 | Possible (consumption)   | Per-execution            |

The tension throughout: more abstraction means less operational burden but also less control over networking, performance tuning, OS-level configuration, and resource scheduling. Teams with deep infrastructure expertise may prefer VM-level control; teams optimizing for delivery speed may favor PaaS or serverless.

## Azure Virtual Machines

### Series and Families

Azure organizes VM sizes into families based on workload profile. The naming convention follows a pattern: family letter + optional subfamily/features + size (e.g., `D4s_v5` — D-series, 4 vCPUs, premium storage capable, version 5).

| Family   | Profile                                              | Typical Workloads                                        |
| -------- | ---------------------------------------------------- | -------------------------------------------------------- |
| B-series | Burstable — baseline CPU with credit accumulation    | Dev/test, low-traffic web servers, small databases       |
| D-series | General purpose — balanced CPU-to-memory             | Application servers, mid-tier databases, enterprise apps |
| E-series | Memory-optimized — high memory-to-CPU ratio          | In-memory caches, SAP HANA, large relational databases   |
| F-series | Compute-optimized — high CPU-to-memory               | Batch processing, gaming servers, analytics              |
| L-series | Storage-optimized — high disk throughput and IOPS    | Data warehousing, large NoSQL databases, log analytics   |
| N-series | GPU-enabled — NVIDIA GPUs attached                   | ML training/inference, rendering, HPC simulations        |
| M-series | Memory-intensive — up to multiple TB RAM             | SAP HANA production, extreme in-memory workloads         |
| H-series | High-performance compute — high-bandwidth networking | Fluid dynamics, molecular modeling, financial risk       |

Selecting the wrong family leads to either wasted spend (oversized) or performance bottlenecks (undersized). Workload profiling before selection — measuring actual CPU, memory, disk, and network patterns — tends to produce better outcomes than guessing based on application category.

### Burstable Economics

B-series VMs accumulate CPU credits during idle periods and spend them during bursts. This works well for workloads with genuinely intermittent CPU needs. Sustained high-CPU workloads on B-series exhaust credits and throttle to baseline, often producing worse price-performance than appropriately sized D-series instances.

### Generation Considerations

Newer VM generations (v5, v6) typically offer improved price-performance through updated processor architectures. However, migration between generations may require redeployment, and some legacy software has specific processor compatibility requirements. The cost advantage of newer generations compounds over time, making periodic right-sizing reviews valuable.

## Availability and Redundancy

### Availability Sets

Availability sets distribute VMs across **fault domains** (separate physical racks) and **update domains** (groups that restart sequentially during platform maintenance). Within a single datacenter:

- **Fault domains** (up to 3): Protect against rack-level hardware failures — power supply, network switch, etc.
- **Update domains** (up to 20): Ensure not all VMs restart simultaneously during platform updates.

Availability sets provide resilience within a datacenter but not against datacenter-level events.

### Availability Zones

Availability zones represent physically separate datacenters within an Azure region, each with independent power, cooling, and networking. Deploying across zones protects against datacenter-level failures.

| Approach                | Failure Scope Protected | Latency Impact             | Cost Factor                  |
| ----------------------- | ----------------------- | -------------------------- | ---------------------------- |
| Single VM (Premium SSD) | Disk failure            | None                       | Baseline                     |
| Availability Set        | Rack failure            | None (same DC)             | Minimal                      |
| Availability Zones      | Datacenter failure      | Sub-millisecond cross-zone | ~Same per-VM, more instances |
| Cross-region            | Regional disaster       | Measurable (ms-100ms+)     | Significantly higher         |

The choice involves trade-offs between resilience scope, operational complexity, and cost. Many production workloads target zone redundancy as a pragmatic balance — protecting against the most common significant failure mode without the complexity of multi-region architectures.

## Virtual Machine Scale Sets

Scale Sets manage groups of identical VMs that can grow and shrink based on demand signals. Key concepts:

- **Scaling rules**: Metric-based (CPU, memory, queue depth, custom metrics) or schedule-based (time-of-day patterns).
- **Scaling profiles**: Different min/max/default instance counts for different scenarios — predictable load patterns benefit from schedule-based scaling alongside reactive metric-based rules.
- **Instance repair**: Automatic replacement of unhealthy instances based on health probe status.
- **Update policies**: Rolling updates with configurable batch sizes and pause durations — balancing update speed against availability during deployments.

**Scale-in considerations**: Determining which instances to remove during scale-in events matters. Policies include newest-first, oldest-first, and balanced across zones. Stateful workloads may need drain signals before termination.

**Overprovisioning**: Scale Sets can provision extra instances during scale-out and release them once the target count is healthy, reducing the time window where capacity is insufficient.

### Flexible vs Uniform Orchestration

- **Uniform**: All VMs share the same configuration — traditional scale set behavior.
- **Flexible**: Mixed VM sizes and types within one set — useful when workloads benefit from heterogeneous instances or gradual migration between VM generations.

## Azure Kubernetes Service (AKS)

AKS provides managed Kubernetes where Azure handles the control plane (API server, etcd, scheduler, controller manager). Operators manage worker nodes and workload configuration.

### Node Pools

Node pools group worker nodes with common configurations. A cluster typically has:

- **System node pool**: Runs Kubernetes system pods (CoreDNS, metrics-server). Requires stable, always-available nodes.
- **User node pools**: Run application workloads. Can have different VM sizes, scaling policies, and node counts.

Multiple node pools enable workload-appropriate sizing:

```
System pool:  3x D2s_v5   (small, stable)
General pool: 5x D4s_v5   (balanced, autoscaled 3-10)
GPU pool:     2x NC6s_v3  (GPU, for inference, autoscaled 0-4)
Batch pool:   0x F8s_v2   (compute-optimized, scaled from 0 on demand)
```

Scaling a pool to zero when idle eliminates its compute cost while maintaining the pool definition for rapid scale-out.

### Virtual Nodes

Virtual nodes use Azure Container Instances as a backing compute layer, appearing as a node in the Kubernetes cluster with theoretically unlimited capacity. Pods scheduled on virtual nodes start as ACI containers.

Trade-offs:

- Rapid burst capacity without pre-provisioning nodes.
- Per-second billing aligns with truly intermittent workloads.
- Not all Kubernetes features translate — DaemonSets, privileged containers, and persistent volumes behave differently or are unavailable on virtual nodes.
- Networking between virtual-node pods and regular-node pods adds complexity.

### AKS Operational Considerations

- **Upgrade strategy**: Kubernetes versions require periodic upgrades. AKS supports blue-green node pool upgrades — create a new pool with the target version, cordon/drain the old pool, delete it.
- **Networking models**: Kubenet (simpler, NAT-based) vs Azure CNI (pods get VNet IPs directly). CNI provides easier integration with Azure networking but consumes more IP addresses.
- **RBAC integration**: AKS integrates with Azure Active Directory for Kubernetes RBAC, bridging identity management across cloud and cluster layers.

## Azure Container Instances (ACI)

ACI runs containers without cluster management. Suitable for:

- Burst workloads from AKS (via virtual nodes).
- Short-lived batch jobs and task runners.
- CI/CD build agents.
- Quick prototyping and testing.

Container groups in ACI share a lifecycle, networking, and storage volumes — conceptually similar to Kubernetes pods. However, ACI lacks the scheduling, service discovery, and self-healing capabilities of an orchestrator, making it unsuitable as a general-purpose production platform for complex microservice architectures.

**Sidecar patterns** work within container groups — a primary container alongside logging, proxy, or init containers sharing the same network namespace.

## Azure Functions

### Hosting Plans

| Plan                    | Scaling                     | Max Timeout                         | Idle Behavior               | Cost Basis                       |
| ----------------------- | --------------------------- | ----------------------------------- | --------------------------- | -------------------------------- |
| Consumption             | Auto, event-driven, to zero | 5-10 min (configurable)             | Scales to zero              | Per-execution + GB-s             |
| Premium (Elastic)       | Pre-warmed, event-driven    | 60 min+ (unlimited in some configs) | Minimum instances stay warm | Per-second on pre-warmed + burst |
| Dedicated (App Service) | Manual/auto within plan     | Unlimited                           | Always running              | App Service plan pricing         |

**Consumption plan cold starts**: When scaled to zero, the first invocation requires runtime initialization — loading the function host, dependencies, and establishing connections. Cold start impact varies by runtime (compiled languages faster than interpreted), function size, and dependency complexity.

**Premium plan pre-warming**: Maintains a configurable number of warm instances, reducing cold start impact at the cost of continuous baseline spend. Effective when consistent low-latency is required but workload volume doesn't justify an always-running service.

### Durable Functions

Durable Functions extend the serverless model to stateful orchestration patterns:

- **Function chaining**: Sequential execution with state passed between steps.
- **Fan-out/fan-in**: Parallel execution of multiple activities with aggregation of results.
- **Async HTTP APIs**: Long-running operations with polling endpoints.
- **Monitor pattern**: Recurring checks with configurable intervals and timeouts.
- **Human interaction**: Workflows that pause waiting for external events or approvals.

State is managed via storage (typically Azure Storage tables and queues), making orchestrations durable across function restarts. This mechanism enables long-running workflows in a serverless context, though the programming model introduces complexity compared to simple request-response functions.

## Azure App Service

App Service provides PaaS hosting for web applications, APIs, and mobile backends. Key concepts:

### Deployment Slots

Slots are live instances of the app with separate configurations and hostnames. Common pattern:

```
Production slot  ← receives live traffic
Staging slot     ← deploy here, test, then swap
```

**Slot swapping** redirects traffic from one slot to another without cold starts — the target slot is already warmed. Post-swap, the previous production code sits in the staging slot, enabling instant rollback.

Swap operations include a warm-up phase where App Service sends requests to the target slot's root path (and configured warm-up paths) before completing the swap.

### Scaling Dimensions

- **Scale up**: Change the App Service plan tier (more CPU, memory per instance).
- **Scale out**: Add instances behind the load balancer (metric or schedule based).

The plan tier determines available features (custom domains, SSL, slots, VNet integration) alongside compute resources. Scaling out distributes load but requires the application to handle multiple instances correctly — session affinity, shared state, distributed caching.

### Traffic Routing

App Service supports percentage-based traffic splitting across deployment slots — useful for canary deployments and A/B testing. A configurable percentage of requests route to a non-production slot, enabling gradual rollout with monitoring.

## Azure Spring Apps

A managed service for running Spring Boot applications (and Polyglot applications in Enterprise tier). Handles infrastructure concerns:

- Managed Spring Cloud components (config server, service registry, gateway).
- Built-in monitoring integration.
- Blue-green deployment support.
- Auto-scaling based on metrics.

Relevant primarily for organizations with significant Spring/Java investment where the managed service overhead reduction justifies the platform specificity. The Enterprise tier adds VMware Tanzu components for larger-scale Spring ecosystems.

## Spot and Low-Priority Instances

Azure Spot VMs use spare datacenter capacity at significant discounts (up to 90%) with the trade-off of potential eviction when Azure needs the capacity back.

**Characteristics**:

- No availability SLA — evictions happen with 30 seconds notice.
- Pricing fluctuates based on capacity and demand.
- Eviction policies: stop-deallocate or delete.

**Appropriate for**: Batch processing, dev/test, CI/CD, stateless scale-out tiers, fault-tolerant workloads, training ML models with checkpointing.

**Inappropriate for**: Workloads requiring guaranteed availability, stateful services without eviction handling, latency-sensitive production traffic.

In AKS, spot node pools can run interruptible workloads alongside regular node pools for critical services, using node selectors and tolerations to control scheduling.

## Choosing Along the Spectrum

The compute choice often reflects organizational context as much as technical requirements:

| Factor          | Favors IaaS (VMs)                   | Favors PaaS/Serverless          |
| --------------- | ----------------------------------- | ------------------------------- |
| Existing skills | Infrastructure/ops expertise        | Application development focus   |
| Compliance      | Strict OS-level audit requirements  | Standard web workloads          |
| Legacy software | Requires specific OS/runtime config | Cloud-native design             |
| Cost pattern    | Steady-state utilization            | Variable/intermittent workloads |
| Time to market  | Established provisioning pipelines  | Rapid iteration priority        |
| Customization   | Kernel modules, custom networking   | Standard runtime environments   |

Many organizations operate across multiple compute tiers simultaneously — VMs for legacy systems, AKS for microservices, Functions for event processing, App Service for web frontends. The architectural challenge lies not in choosing one model but in managing the operational complexity of heterogeneous compute within a coherent networking and security boundary.

## Networking Considerations Across Compute

Compute choices affect network architecture. VMs integrate directly into virtual networks. AKS requires careful subnet sizing (especially with Azure CNI). Functions and App Service require VNet integration configuration for private resource access. Container Instances support VNet deployment but with constraints.

Private endpoints, service endpoints, and VNet integration each address different network isolation needs across the compute spectrum. The pattern of "compute service in a VNet talking to PaaS services via private endpoints" appears across most production architectures regardless of the specific compute choice.

## Cost Optimization Patterns

- **Reserved instances**: 1- or 3-year commitments for predictable workloads reduce per-hour costs significantly.
- **Spot instances**: Opportunistic use for fault-tolerant workloads.
- **Autoscaling**: Right-sizing to actual demand rather than peak provisioning.
- **Scale to zero**: Serverless and AKS node pools that scale to zero during idle periods.
- **Dev/test pricing**: Reduced rates for non-production workloads.
- **Hybrid benefit**: Using existing Windows Server or SQL Server licenses to reduce Azure VM costs.

Cost optimization works best as continuous practice — workload patterns change, new VM series offer better price-performance, and reserved instance commitments need periodic review against actual utilization.
