# Cloud Migration — 7Rs, Assessment, Migration Waves & Cutover Planning

## The 7Rs Framework

AWS's Cloud Adoption Framework (CAF) defines seven migration patterns, collectively the "7Rs," categorizing how workloads transition to the cloud.

### 1. Rehost (Lift and Shift)

Move applications unchanged to cloud compute. The simplest, fastest migration with minimal engineering.

**Pattern**: Virtual machine on premises → Virtual machine in cloud. Database server → RDS or EC2-hosted equivalent.

**Characteristics**:
- Fastest to execute (weeks, not months)
- Lowest engineering effort
- No application rewrite
- Immediate cloud cost (no optimization)
- Licensing may improve (BYOL, cloud-native pricing)

**Gotchas**: Infrastructure-as-is often doesn't match cloud assumptions. Network topologies differ; performance characteristics change with I/O, CPU, memory ratios. Post-rehost tuning is common.

**Use case**: Breaking-the-chains migration—get off legacy infrastructure quickly, optimize later. Risk-averse organizations preferring proven approaches.

### 2. Replatform (Lift, Tinker and Shift)

Minimal application changes to leverage cloud-native managed services without full refactor.

**Pattern**: Self-managed database → RDS, load-balanced VMs → elastic load balancer + autoscaling, self-hosted CI/CD → CodePipeline.

**Characteristics**:
- 10-20% application changes (config, non-functional requirements)
- Moderate engineering effort
- Faster than refactor, more optimized than rehost
- Better cost efficiency, operational simplicity
- Risk of misalignment if managed service model doesn't match application needs

**Use case**: Modernize infrastructure without deep application redesign. Organizations ready to use managed services but preferring stability over architectural upheaval.

### 3. Refactor (Re-architect)

Rearchitect applications for cloud-native patterns: microservices, serverless, container orchestration, event-driven design.

**Pattern**: Monolith → Microservices on ECS/EKS, batch jobs → Lambda, stateful service → DynamoDB.

**Characteristics**:
- Weeks to months of engineering
- Highest upfront investment
- Largest long-term cloud benefits (scalability, cost, velocity)
- Highest risk (new architecture can fail differently)
- Demands deep cloud expertise, testing discipline

**Use case**: Business requires new capabilities (global scale, real-time responsiveness) or licenses drive ROI justifying engineering expense.

### 4. Repurchase (Replace)

Abandon legacy systems, adopt SaaS or managed equivalents.

**Pattern**: In-house CRM → Salesforce, custom ERP → NetSuite, self-hosted wiki → Confluence Cloud.

**Characteristics**:
- Operational lift (data migration, user training)
- Licensing negotiation, vendor lock-in risk
- Often faster adoption than rehost + optimization
- Feature overlap mismatch (SaaS may not match custom requirements)

**Use case**: Legacy system maintenance is expensive and feature set is commoditized. Vendor maturity reduces migration risk.

### 5. Retire

Decommission workloads no longer needed. Often the highest ROI—cost savings are immediate.

**Pattern**: Legacy reporting system replaced by BI tool, deprecated batch job serving no business purpose.

**Characteristics**:
- Zero engineering, immediate cost savings
- Data archaeology may be required to confirm safety
- Stakeholder alignment crucial (unused != unneeded)

**Use case**: Portfolio assessment reveals redundancy. Cost pressure demands quick wins.

### 6. Retain

Keep workloads on premises or legacy infrastructure, intentionally.

**Pattern**: Specialized hardware requiring low latency, compliance requiring air-gap, systems requiring specific OS/hardware combinations unavailable in cloud.

**Characteristics**:
- No migration cost, no new infrastructure
- Ongoing operational expense of maintaining legacy systems
- Drift risk—modernization happens around retained workloads

**Use case**: Technical or compliance constraints genuine, not organizational risk aversion. True hybrid scenarios.

### 7. Relocate

Move infrastructure between on-premises and cloud data centers, typically for compliance or cost optimization.

**Pattern**: Workloads using VMware → VMware Cloud on AWS, relocating physical servers for geographic redundancy.

**Characteristics**:
- Cloud-like experience without full cloud architecture
- Higher cost than cloud-native (licensing, overhead)
- Reduced complexity vs. traditional cloud migration

**Use case**: Compliance requiring geographic locality, organizations lacking cloud expertise, temporary staging before full cloud adoption.

## Discovery and Assessment

### Assessment Framework

Structured assessment informs 7R categorization and cost estimation.

**Inventory Phase**:
- **Infrastructure audit**: VMs, databases, storage, networking, load balancers. Capture CPU, memory, disk, network I/O.
- **Application dependency mapping**: Interdependencies, data flows, external integrations.
- **License analysis**: BYOL opportunities, cloud-friendly licenses, proprietary constraints.
- **Performance baselines**: Current latency, throughput, utilization under typical and peak load.

Tools: AWS Application Discovery Service, CloudScape, CloudPhysics, or manual discovery via agents.

### Cost Analysis

Estimate cloud cost pre-migration:

- **Compute**: CPU/memory sizing, instance type, reserved instances vs. on-demand.
- **Storage**: Disk I/O requirements, EBS gp3/gp2/io1 selection, S3 vs. EBS vs. EFS.
- **Network**: Data transfer cost (often overlooked), NAT gateway, load balancer.
- **Licensing**: Per-core, per-socket, or flat rate in cloud vs. premises.

Typical finding: Direct rehost costs 20-30% more than on-premises before optimization. Replatform and refactor can achieve 40-60% cost savings after 6-12 months of tuning.

### TCO Comparison

Total Cost of Ownership includes:

- **CapEx**: Premises hardware, replacement cycles; cloud annual commitment.
- **OpEx**: Facilities, power, staffing, maintenance; cloud metered usage.
- **Hidden costs**: Compliance infrastructure, disaster recovery, training.

Cloud often wins long-term, but rehost is expensive short-term without immediate optimization.

## Migration Waves and Sequencing

Parallel migration of all workloads is high-risk and resource-intensive. Staged waves reduce risk and allow learning.

### Wave 1: "Quick Wins"

Migrate simple, non-critical applications with low dependency complexity. Build organizational confidence and refine processes.

**Characteristics**:
- Low-risk failures (non-business-critical)
- Fast execution (proves capability)
- Demonstrates ROI (cost savings visible early)

**Examples**: Development environments, proof-of-concept systems, redundant databases, archived datasets.

### Wave 2: "Core Applications"

Migrate key business workloads once processes solidify. Build on Wave 1 learnings.

**Characteristics**:
- Higher complexity, integrated dependencies
- Cross-functional involvement (multiple teams, stakeholders)
- Longer cutover windows, more rigorous testing

### Wave 3: "Complex & Scale"

Migrate complex, high-value, highly-integrated systems. Often the longest phase.

**Characteristics**:
- Significant refactor or replatform investment
- Global coordination (time zones, regulatory compliance)
- Extended validation periods

### Wave Planning

**Sequencing factors**:
- **Dependency**: Move dependencies before dependents.
- **Risk**: Low-risk first, high-risk last.
- **Speed**: Quick wins early for momentum.
- **Resource): Batch capacity limits; stagger to avoid bottlenecks.

**Example sequencing**:
1. Dev/test environments (low risk, fast)
2. Standalone applications (dependencies clear)
3. Integrated applications (dependencies resolved, migration experience high)
4. Mission-critical systems (most mature processes, largest team)

## Database Migration

Database migration often dominates project timelines. Data volume, complexity, and downtime tolerance drive strategy.

### Migration Tools

**AWS Database Migration Service (DMS)**:
- Continuous data sync from source to target
- Supports heterogeneous migrations (MySQL → PostgreSQL on RDS)
- Change Data Capture (CDC) enables low-downtime cutover

**Oracle Schema Conversion Tool (SCT)**:
- Converts schemas across databases (Oracle → PostgreSQL, SQL Server)
- Limited to schema; data migration via DMS

### Migration Patterns

**Full Load + CDC**:
1. Perform initial (full) load of data and schema.
2. Enable Change Data Capture to capture ongoing changes from source.
3. At cutover, apply accumulated changes, switch application connections.
4. Assumes source database remains online during migration.

**Homogeneous (same engine)**:
- Snapshot-based: Export snapshot, restore in cloud. Straightforward if compatible versions.
- Logical backup: Dump schema/data, import. Slower but platform-independent.

**Heterogeneous (different engines)**:
- Schema conversion required (structure differs between MySQL, PostgreSQL, Oracle).
- Data type mapping, stored procedure rewriting.
- Higher complexity; SCT automates many conversions but manual review essential.

### Parallel Testing

Run source and target in parallel during migration:

- **Dual-write testing**: Application writes to both old and new simultaneously, validates consistency.
- **Replay testing**: Capture real workload, replay against target, compare results.

Parallel testing catches data quality and performance issues before production cutover.

## Testing Strategies

### Test Levels

**Functional testing**: Does the migrated system work? All features, integrations, workflows intact?

**Performance testing**: Does it perform acceptably? Is cloud-hosted application meeting SLAs?

**Load testing**: Autoscaling, elasticity, burst handling.

**Disaster recovery testing**: Backup, restoration, failover mechanisms.

### Production-like Testing Environment

Most failures emerge under production load. Create a test environment replicating production scale:

- **Data volume**: Full or representative subset of production data.
- **Workload**: Production-representative application load (peak hours, batch jobs, integrations).
- **Configuration**: Identical to target production (instance types, database parameters, networking).

Test environments are expensive but catch integration issues early.

## Cutover Planning

Cutover is the moment traffic switches from old to new—highest risk period.

### Cutover Strategies

**Big Bang**:
- Single switchover from old to new; discontinuous.
- **Risk**: Total failure = business outage.
- **Advantage**: Simple, no parallel running cost.
- **Use case**: Small systems, high confidence, acceptable downtime window.

**Phased Cutover**:
- Routes percentage of traffic to new system incrementally (5% → 25% → 50% → 100%).
- Detects issues at small scale before full impact.
- **Risk**: Partial failures affect subset of users.
- **Advantage**: Controlled rollback, damage limitation.
- **Use case**: Large systems, high confidence, minimal allowed downtime.

**Dual-Run**:
- Run old and new in parallel; application writes to both, reads from new; compares results.
- **Risk**: Complex routing, data inconsistency if mismatch detected.
- **Advantage**: Instant rollback capability; validate correctness before full cutover.
- **Use case**: Mission-critical systems where validation trump downtime window.

### Rollback Planning

Every cutover plan must include rollback:

- **Time horizon**: How long can you run both systems if cutover fails? (24 hours, 1 week?)
- **Data consistency**: Can rollback restore consistency if new system wrote data?
- **Automated vs. manual**: Automated rollback reduces MTTR but adds complexity/risk.

Realistic rollback includes:
- Database transaction rollback or restore from backup.
- Application server traffic re-routing to old infrastructure.
- Cache invalidation and session re-establishment.
- Notification to users of anomaly (optional).

### Cutover Window

Choose maintenance windows (low traffic, staffing available, rollback capability matured):

- **Weekday early morning**: Users less active, day shifts can respond.
- **Avoid month/quarter-end**: Finance systems busy.
- **Extend window**: 4-8 hours for large systems; 1-2 hours for smaller.
- **Standby staff**: On-call, documented runbooks, communication channels open.

## Post-Migration Optimization

Rehost rarely optimizes cost. Post-migration tuning realizes cloud benefits:

**Instance right-sizing**: Identify over-provisioned instances, move to smaller types.

**Storage optimization**: Migrate EBS to gp3 (cheaper), archive cold data to Glacier.

**Reserved instances**: Commit to baseline load with RIs (30-40% discount).

**Serverless refactor**: Convert batch jobs to Lambda, reduce always-on compute.

**Architecture**: Implement caching, CDN, autoscaling.

## Summary

The 7Rs provide a mental model for categorizing workloads: rehost for speed, replatform for optimization, refactor for transformation. Assessment drives strategy; waves manage risk and learning. Database migration is often the constraint; testing and planning mitigate cutover risk. Post-migration optimization realizes cloud value. Most mature organizations follow a 2-3 year migration trajectory: waves 1-2 deliver quick wins; wave 3 captures long-tail complexity.