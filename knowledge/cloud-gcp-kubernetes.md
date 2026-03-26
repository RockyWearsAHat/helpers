# Google Kubernetes Engine (GKE) — Managed Kubernetes, Multi-Zone Clusters & Anthos

## Overview

**Google Kubernetes Engine (GKE)** is Google's managed Kubernetes platform, deeply integrated with Google Cloud services and optimized for cloud-native workloads. GKE offers two operational models (Autopilot and Standard) and can extend on-premises infrastructure via Anthos.

## Core Architecture

### GKE Standard vs. Autopilot

**GKE Standard**:
- Full control over node configuration (machine types, boot disks, networking)
- Pay for nodes even when idle
- Manual scaling and upgrades
- Suitable for specialized workload requirements

**GKE Autopilot**:
- Fully managed nodes; users specify workload needs
- Google manages scaling, upgrades, patching
- Pay-per-pod model (similar to serverless)
- Opinionated: enforces Workload Identity, network policies, security standards
- Recommended for new clusters unless specific control required

### Node Auto-Provisioning

**Standard clusters** can enable node auto-provisioning:
- Automatically creates node pools matching pod requirements
- Analyzes pending pods and selects appropriate machine types
- Combines regular scaling with pool diversification
- Reduces manual pool management for heterogeneous workloads

Example: A pending GPU pod auto-provisions a node pool with GPU-enabled machines; a CPU-heavy pod provisions a CPU-optimized pool.

### Resources vs. Workload Pools

- GKE uses **workload pools** (conceptually distinct from traditional node pools)
- Autopilot maintains workload pools behind the scenes
- Standard clusters may have explicit node pools for control

## Multi-Cluster & Hybrid Deployment

### Anthos Platform

Anthos extends Kubernetes beyond Google Cloud:

**Anthos GKE (on-premises)**:
- Deploy GKE clusters in private datacenters
- Same control plane APIs as cloud-hosted GKE
- Hybrid service mesh for cross-cluster communication

**Anthos on VMs**:
- Run Kubernetes on VMs across multiple clouds
- Single pane of glass for policy, security, and monitoring

**Config Connector**:
- Kubernetes CRDs represent Google Cloud resources
- Define cloud resources via Kubernetes manifests
- GitOps-driven infrastructure management
- Example: Kubernetes Secret CRD maps to Secret Manager

## Identity & Access

### Workload Identity

Replaces node-level service accounts and avoids credential rotation headaches:

- Each Kubernetes Service Account binds to a Google Cloud Service Account
- Pods inherit the SA identity without credentials
- Fine-grain RBAC per workload vs. per-node
- Encrypted, short-lived tokens fetched via metadata server
- Recommended for all production deployments

**Diagram**: Pod SA → (mapped to) Google Cloud SA → IAM roles → Cloud resources

### Binary Authorization

Enforces that only container images signed by trusted keys can be deployed:
- Policy-driven: require attestations before allowing pods
- Integrates with CI/CD pipelines (sign during build)
- Prevents unsigned or tampered images
- Auditable for compliance

## Networking

### VPC-native Clusters

Standard GKE model:
- Pods receive IPs from VPC subnets (not overlay)
- Direct VPC integration with Cloud NAT, Cloud Routes, security policies
- Support for VPC-SC (VPC Service Controls) for resource isolation

### Networking Policies & Service Mesh

**Network Policies**:
- Built-in Calico integration
- Stateful firewall-like rules per pod/namespace

**Anthos Service Mesh**:
- Istio distribution optimized for GKE
- Mutual TLS, distributed tracing, traffic management
- ASM observability dashboards

## Autoscaling & Load Balancing

### Cluster Autoscaler

Automatic node (or workload) scaling based on pending pods:
- Respects min/max cluster size
- Removes underutilized nodes
- Integrates with Workload Identity for secure scale-up

### Horizontal Pod Autoscaler (HPA)

Scales pod replicas on CPU, memory, or custom metrics from Cloud Monitoring.

### Vertical Pod Autoscaler (VPA)

Recommends CPU/memory requests based on actual usage. Can auto-update deployments (not recommended alongside HPA for same metrics).

### Load Balancing

GKE exposes services via Google Cloud Load Balancing:
- **Ingress Controller**: deploys Cloud Load Balancer (L7)
- **Service type LoadBalancer**: deploys Network Load Balancer (L4)
- Internal load balancing for private services
- Cross-zone and cross-region load balancing

## Monitoring & Logging

### Cloud Monitoring (Stackdriver)

- Pod, node, and cluster metrics (CPU, memory, disk, network)
- Custom metrics collection via OpenTelemetry
- Alerting and dashboard creation
- Integration with Kubernetes events

### Cloud Logging

- Pod container logs (stdout/stderr)
- Audit logs of API calls
- Cloud Audit Logs integration
- Log-based metrics and alerting

### GKE Security Command Center Integration

- Vulnerability scanning of container images
- Runtime security monitoring
- Compliance findings dashboard

## Release & Upgrade Strategy

- GKE follows upstream Kubernetes releases closely (~2-3 weeks)
- Autopilot clusters auto-upgrade (with user maintenance windows)
- Standard clusters can defer upgrades or set auto-upgrade schedules
- Node pools upgrade independently with PDB respect

## Cost Optimization

- **Autopilot**: priced per pod; fits unpredictable workloads
- **Standard + Committed Use Discounts**: 25-30% savings for sustained usage
- **Preemptible VMs**: 60-90% discount for fault-tolerant workloads
- **Multi-cluster ingress**: share load balancer across clusters

## Related

See also: [cloud-gcp-compute.md](cloud-gcp-compute.md), [cloud-gcp-networking.md](cloud-gcp-networking.md), [security-cloud-security.md](security-cloud-security.md), [architecture-service-mesh.md](architecture-service-mesh.md)