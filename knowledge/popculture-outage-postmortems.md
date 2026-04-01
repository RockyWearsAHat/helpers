# Famous Outages as CS Lessons

## Overview

Major platform outages are live case studies in distributed systems, networking, risk management, and human factors. These incidents are dissected in postmortems, taught in SRE courses, and referenced as "war stories" in engineering discussions. Each reveals a different class of failure mode.

---

## AWS us-east-1 Outage (April 2011)

**Duration**: ~4 hours  
**Scope**: Amazon's largest US region; cascading impact on Netflix, Heroku, Foursquare, others  
**Root cause**: Elastic Load Balancing (ELB) algorithm + network storm

A routing configuration change caused the ELB tier to start checking backend health in an aggressive loop. Healthy servers, under sudden traffic surge from incoming requests, began responding slowly. The ELB interpreted slowness as failure and redirected traffic to other servers. Those servers also got overloaded and slowed down. The result: a thundering herd of health checks that prevented any server from recovering.

**Critical insight**: Load balancers themselves can fail. An algorithm that is correct under normal load can amplify failure under extreme load. There's no automatic circuit breaker or backoff between health checks and retries.

**Post-mortem lessons**:
- Health check algorithms need exponential backoff and jitter, not tight loops
- Cascading failures start small and accelerate if feedback loops are positive
- Single-region deployments are vulnerable; multi-region isn't optional for critical services

**Pop culture**: AWS became more transparent about regional failures after this. The outage accelerated the industry's shift toward multi-region architecture and chaos engineering.

---

## Facebook BGP Outage (October 2021)

**Duration**: ~6 hours  
**Scope**: Facebook, Instagram, WhatsApp, Messenger all globally unreachable  
**Root cause**: BGP configuration error during routine maintenance

Facebook's engineering team deployed a configuration change to its Border Gateway Protocol (BGP) routers. The change was syntactically valid but logically wrong: it withdrew all BGP announcements for the IP prefixes Facebook owns. This meant internet routers around the world stopped knowing the path to reach Facebook's servers.

**Network-level impact**:
- DNS resolution still worked (Facebook's DNS was in separate infrastructure)
- But DNS responses pointed to IPs that were unreachable
- Facebook's internal networks also isolated (same BGP infrastructure)
- Data center engineers couldn't access buildings to fix it remotely from outside
- Physical access for manual recovery took hours

**Compounding factors**:
- The change was correct according to testing (in a test lab with fake BGP peers)
- No staged rollout; changes went live to all routers
- Internal and external networks used the same BGP infrastructure
- Access controls prevented quick rollback from remote

**The lesson**: BGP is the internet's routing protocol. A misconfiguration at a single provider can make a significant portion of the internet unreachable. The incident revealed:
- BGP lacks simple validation (configuration changes don't automatically verify they're sensible)
- Network infrastructure should have redundancy and isolation separate from logical infrastructure
- Critical infrastructure needs physical access guarantees, not just remote control

**Pop culture**: Became the canonical example of "how one line of code broke the internet for 2 billion people." References in every network engineering talk.

---

## Cloudflare Regex Outage (July 2019)

**Duration**: ~30 minutes  
**Scope**: ~10% of Cloudflare's customer traffic  
**Root cause**: Catastrophic backtracking in regular expression (ReDoS)

Cloudflare shipped a Web Application Firewall (WAF) rule containing a regex with catastrophic backtracking:

```regex
(.*)*@
```

This regex can be catastrophic on certain inputs because the quantifiers nest. When the regex engine tries to match this against a long string without "@", it attempts exponentially many backtracking paths:
- `.*` matches the entire string once
- `.*` matches everything except the last character
- `.*` matches everything except the last two characters
- ...and so on, for N characters = 2^N attempts

A 30-character string without "@" triggers ~2³⁰ (billions) of regex matching attempts, freezing the WAF.

**The fix**: Rewrite the regex to avoid backtracking:

```regex
[^@]*@
```

This runs in linear time: no nesting quantifiers, no exponential blowup.

**The lesson**: Regex performance is non-obvious. A regex that runs fast on normal inputs can hang on adversarial inputs. The incident demonstrated:
- Performance is a security property (performance bugs can be exploited like DDoS)
- Complex regexes need static analysis (linters can detect catastrophic backtracking)
- Changes to WAF rules need capacity testing, not just functional testing

**Pop culture**: Went viral in the security + programming communities. "Regex performance" is now a standard talk topic.

---

## GitLab Database Deletion (February 2017)

**Incident**: GitLab.com production database accidentally deleted  
**Recovery time**: Data recovered from backups from 18 hours prior; eventual full recovery from replicas  
**Root cause**: Wrong command run on wrong host by sleep-deprived engineer

A GitLab engineer on-call, debugging a performance issue, ran an administrative command to delete certain records from a PostgreSQL database. The command was intended for a staging database, but due to environment variable confusion, it ran against production.

The command:

```sql
DELETE FROM events WHERE created_at < X;
```

This deleted 300 GB of production data spanning 6 hours of activity.

**Compounding factors**:
- On-call engineer was sleep-deprived and context-switching under pressure
- SQL didn't require confirmation ("Are you sure?")
- Backups existed but were uploaded to an object store and took time to restore
- No real-time replication from another data center

**Recovery process**:
- Team discovered the deletion ~15 minutes later (users reported missing data)
- Recovered from 6-hour-old backup
- Lost 6 hours of repository activity, issue tracking, issues
- Full recovery from replicas took days

**The lesson**:
- Destructive operations need multiple safeguards: confirmation prompts, dry-run modes, canary deletes, role-based access control
- Sleep-deprived engineers should not run destructive commands (automate or escalate)
- Backups are necessary but slow; real-time replication (streaming replicas) provides faster recovery
- The boundary between staging and production must be physically or logically isolated

**Pop culture**: Became a poster child for incident response and learning organizations. GitLab's public postmortem was praised for transparency. The incident changed how teams think about data deletion safeguards.

---

## CrowdStrike July 2024 Outage

**Incident**: CrowdStrike deployed a buggy content update affecting Windows machines globally  
**Impact**: 8+ million Windows devices failed to boot or experienced critical failures  
**Duration**: Multiple hours for initial recovery (~24 hours for full mitigation)  
**Root cause**: Content file parsing error + lack of validation in deployment pipeline

CrowdStrike's Falcon sensor software is a kernel-mode driver installed on endpoints. Falcon periodically downloads content definitions (malware signatures, detection rules). On July 19, 2024, CrowdStrike deployed a malformed content file. The driver crashed during startup, causing the kernel to crash, and Windows failed to boot.

**Technical cascade**:
1. Malformed content file deployed via CrowdStrike's update mechanism
2. Falcon driver loaded the file at kernel startup
3. Buffer overread/access violation in parsing code
4. Kernel panic → Windows fails to boot
5. Safe mode also loaded the driver
6. Machines required manual recovery via Windows Recovery Environment

**Global scope**:
- Affected airlines, hospitals, financial institutions, broadcasters
- Some institutions couldn't process transactions or dispense cash
- Airlines grounded flights; emergency rooms had to revert to paper charts

**The lesson**:
- Kernel-mode drivers are high-privilege; a single update affects all systems immediately
- Content validation is critical; processing untrusted content in kernel mode is extremely risky
- No staged rollout; updates went to all machines at once
- Recovery:Manual recovery on millions of devices required physical access or remote hands-on support (booting into WRE, editing registry)
- Testing: How did this not trigger in pre-deployment testing? Likely the test environment didn't catch the parsing edge case.

**Pop culture**: One of the most visible outages in tech history. Became instant meme material ("CrowdStrike tried to kill Windows today"). Widely discussed as an example of cascading failure across critical infrastructure.

---

## GitLab CI/CD Cascading Failure (2019)

**Incident**: Cascading database connection exhaustion  
**Duration**: ~1 hour  
**Root cause**: Inefficient query + connection pool saturation

GitLab's PostgreSQL began experiencing slow queries during peak traffic. The web tier's connection pool (a fixed number of connections from app servers to the database) became saturated. New requests to the API queued for a connection. The queue backed up, web servers became unresponsive, and load balancers started health check failures. This cascaded to the load balancer removing healthy app servers from rotation, further reducing capacity.

**The pattern**: One component (database performance) degrades → connection pool exhausts → client requests queue → response times balloon → health checks fail → servers removed from rotation → remaining servers overload → system enters death spiral.

**Recovery**: Manual intervention to identify the slow query, kill long-running transactions, and increase the connection pool limit temporarily.

**The lesson**:
- Connection pooling is a bounded resource; saturation is a single point of failure
- Slow queries don't just affect query latency; they cascade through resource contention
- Observability should track pool saturation, query times, and queue depth, not just error rates
- Health checks can turn graceful degradation into catastrophic failure if they're too strict

---

## Cross-Cutting Patterns

### 1. **Cascading Failures**
AWS ELB health-check storm, GitLab connection pool exhaustion, CrowdStrike kernel crash. When one layer fails, feedback loops can amplify the failure.

### 2. **Change Management**
Facebook's BGP change, GitLab deletion, CrowdStrike content update. All were routine operations that went awry. Safeguards:
- Canary deploys (1% of traffic/machines first)
- Staged rollouts
- Validation and testing
- Dry-run modes
- Rollback capability

### 3. **Blast Radius and Blast Containment**
CrowdStrike and Cloudflare affected millions globally. AWS and Facebook were regional or infrastructure-level. Containment could have reduced impact:
- Gradual rollout instead of global deploy
- Feature flags to disable the problematic code path
- Isolation between update and execution

### 4. **Recovery Speed vs. Recovery Completeness**
AWS and GitLab recovered fast but lost data. True recovery took longer. The tradeoff:
- Fast recovery: serve stale data, accept data loss
- Complete recovery: take time, replay from replicas

### 5. **Human Factors**
GitLab engineer was sleep-deprived and context-switching. Humans under pressure make mistakes. Safeguards:
- Automation instead of manual commands
- Confirmation prompts for destructive operations
- Two-person rules for critical operations
- On-call rotation that prevents exhaustion

---

## See Also

- **sre-postmortems.md** — How to write and conduct blameless postmortems
- **sre-incident-management.md** — On-call practices, incident response
- **monitoring-incident-tooling.md** — Observability and alerting foundations
- **system-design-distributed.md** — Cascading failure modes, resilience patterns
- **devops-policy-as-code.md** — Change management and deployment validation