# Engineering Metrics — DORA, SPACE, and Measuring Developer Productivity

## The Measurement Problem

Engineering metrics—quantifying what teams do and how well they do it—present a paradox: measurement is necessary for accountability and improvement, but poorly chosen metrics incentivize gaming, short-termism, and misalignment.

**Goodhart's Law** states: "When a measure becomes a target, it ceases to be a good measure." If deployment frequency is the metric, teams deploy trash frequently. If code coverage is the target, teams comment out test failures. Metric systems must be broad, multi-dimensional, and interpreted with nuance.

See also: [sre-reliability-engineering.md](sre-reliability-engineering.md), [devops-observability-patterns.md](devops-observability-patterns.md), [process-agile-beyond.md](process-agile-beyond.md)

## DORA Metrics

**DORA** (DevOps Research and Assessment) is a set of four metrics developed by Google that correlate strongly with organizational performance and business outcomes:

### 1. Deployment Frequency

How often does code reach production?

- **Definition**: Number of deployments to production per unit time (day, week, month)
- **Calculation**: Count of production deployments in measurement period ÷ number of periods
- **Elite performers**: Multiple times per day (>1 deployment/day)
- **High performers**: Once per day to once per week
- **Medium performers**: Once per month to once per quarter
- **Low performers**: Fewer than once per quarter

**Why it matters**: Deployment frequency correlates with team velocity and organizational agility. Teams that deploy daily fix problems faster, ship features faster, and can respond to market changes. High frequency requires automation, confidence in testing, and small, focused changes.

**Pitfalls**: 
- Deploying broken code frequently is not a win
- Counting "manual hotfixes to production" as deployments inflates the metric
- Automation with poor testing can increase frequency while decreasing reliability

**Healthy range**: 1x/day to 1x/week for most teams. Higher frequencies require exceptional testing discipline.

### 2. Lead Time for Changes

How long does it take to go from code commit to production?

- **Definition**: Time from code commit (that will eventually deploy) to that code running in production
- **Calculation**: Median time between commit and production deployment (or 50th percentile; consider P95)
- **Elite performers**: < 1 day
- **High performers**: 1-7 days
- **Medium performers**: 1-4 weeks
- **Low performers**: > 4 weeks

**Why it matters**: Lead time is a proxy for cycle time and organizational efficiency. Short lead times mean feedback loops are fast—bugs detected quickly, experiments validated quickly, learning accelerates.

**Pitfalls**:
- Measuring from PR creation (not commit) inflates the number by including review time
- Counting time waiting for deployment windows (e.g., "only deploy on Tuesdays") as team delay
- Tooling that auto-commits or auto-merges can artificially reduce lead time while hiding quality issues

**Healthy range**: 1-7 days. Anything > 4 weeks suggests process friction (complex approval chains, rare deployments, manual gates).

### 3. Mean Time to Recovery (MTTR)

When an incident occurs, how fast does the team restore service?

- **Definition**: Average time from incident detection to service restoration
- **Calculation**: Sum of recovery times ÷ number of incidents, or median time to recovery
- **Elite performers**: < 1 hour
- **High performers**: < 1 hour (typically)
- **Medium performers**: 1-24 hours
- **Low performers**: > 24 hours

**Why it matters**: MTTR reflects operational maturity. A team with fast MTTR has runbooks, monitoring alerting, on-call discipline, and blameless incident culture. They recover quickly because they practice.

**Pitfalls**:
- Counting only "major" incidents (excluding pages, P4 bugs) underrepresents true recovery burden
- Measuring from ticket creation (not detection) inflates MTTR if alerting is poor
- Gaming: Closing incidents prematurely to make MTTR look good

**Healthy range**: < 1-2 hours for critical services. Services with strong observability and runbooks typically hit this.

### 4. Change Failure Rate

What fraction of deployments result in production incidents requiring hotfixes, rollbacks, or manual patches?

- **Definition**: (Number of deployments causing incidents) ÷ (total number of deployments)
- **Elite performers**: 0-15%
- **High performers**: 15-45%
- **Medium performers**: 45-60%
- **Low performers**: > 60%

**Why it matters**: CFR measures deployment quality. High CFR means something in the change pipeline isn't working: testing is insufficient, changes are too large, or code review isn't catching issues. Low CFR (without slow deployments or tiny changes) suggests good practices.

**Pitfalls**:
- Defining "incident" ambiguously (does a 30-second latency spike count?)
- Not accounting for change size: A change affecting 1,000 lines is riskier than 5
- Gaming: Making lots of tiny, trivial changes to inflate total deployment count and dilute CFR

**Healthy range**: 15-30%. Anything > 45% suggests either inadequate testing or deployments that are too large and risky.

## SPACE Framework

DORA captures deployment velocity and reliability, but **not the qualitative experience** of engineers or the entire value delivery chain. **SPACE** (introduced by Forsgren et al.) expands the perspective:

### S: Satisfaction & Well-Being

How satisfied are engineers? Are they burned out?

- **Measurement**: Employee engagement surveys, retention rates, self-reported well-being (1-10 scale)
- **Why it matters**: Satisfied engineers stay, learn, and produce higher quality. Burnout manifests as quality issues months later
- **Pitfall**: Using satisfaction as a standalone metric without understanding root causes (overwork, unclear priorities, poor tools)

### P: Performance

Team's ability to deliver value, quality, and reliability.

- **Components**: Deployment frequency, lead time, change failure rate (overlaps with DORA)
- **Extensions**: Code quality (defect escape rate), system reliability (SLO attainment), security (vulnerability detection/fix time)
- **Why it matters**: DORA-style metrics narrowly focus on deployment performance; SPACE adds quality dimension

### A: Activity

What are engineers actually doing?

- **Measurement**: Commits per person (weak indicator), pull requests reviewed, time in meetings, deep work hours
- **Why it matters**: "Activity" is contextual. Writing 1,000 lines of code might be good or bad depending on problem complexity. The volume reveal little; composition reveals more
- **Pitfall**: Using commits/LOC as productivity proxy (cargo cult metrics). Gaming is immediate and obvious

### C: Collaboration & Communication

How well do engineers work together?

- **Measurement**: Cross-team dependencies, ease of finding experts, time spent on code review, knowledge sharing (pair sessions, docs written)
- **Why it matters**: Collaboration scales delivery—teams that share knowledge and unblock each other outperform siloed teams
- **Pitfall**: Excessive collaboration metrics (e.g., "minimum code reviews per day") create theatre without substance

### E: Efficiency & Flow

How much time does the engineer spend in deep work vs. context-switching?

- **Measurement**: Uninterrupted coding time (calendar blocks, IDE data), meetings per day, time waiting for builds/deploys, time in meetings
- **Why it matters**: Flow is where complex thinking happens. Too many interruptions (on-call, meetings, Slack) degrade output quality
- **Pitfall**: Measuring "deep work hours" can incentivize poor communication (ignoring Slack, missing meetings needed for async-first team)

## Engineering Metrics in Practice

### Multi-Dimensional Dashboard

Healthy teams use a dashboard combining metrics from multiple dimensions:

```
DORA:
├─ Deployment frequency: 2x/week (green)
├─ Lead time: 2-3 days (green)
├─ MTTR: 30 min (green)
└─ Change failure rate: 20% (yellow — watch)

SPACE:
├─ Satisfaction (survey): 7.2/10 (orange — declining from 7.8)
├─ Code review turnaround: 4 hours median (green)
├─ Deep work: 4-5 hrs/day (green)
└─ Meeting load: 8 hrs/week (yellow — increasing)

SLO:
├─ Availability: 99.95% (green, target 99.9%)
├─ Latency P99: 150ms (green, target < 200ms)
└─ Error rate: 0.02% (green)
```

The dashboard tells a story: Team is performing well operationally (green DORA), code quality is solid, but satisfaction is declining and team is over-meeting. Investigation: "Why are meetings up?" (Onboarding new PM; temporary; will revert). No action needed; transparency suffices.

### Avoiding Goodhart's Law

1. **Don't optimize for a single metric**: Deployment frequency alone incentivizes shipping garbage. Pair it with MTTR and CFR
2. **Involve the team in interpretation**: "Our deployment frequency is up but so is CFR—let's understand why before celebrating"
3. **Metrics are inputs to decisions, not decisions themselves**: "This metric moved" triggers investigation; the finding, not the metric, drives action
4. **Quarterly review of metrics themselves**: "Is this metric still useful? Is it being gamed? Should we stop measuring this?"
5. **Qualitative + quantitative**: Metrics are incomplete without team context. "Velocity up" + "satisfaction down" = sign of crunch

## Anti-Patterns

### Pattern: Leading with Utilization

**Problem**: Team measured on "% billable time" or "lines of code per day." Incentivizes busywork, discourages refactoring, penalizes thinking.

**Corrective**: Replace with outcome metrics (features shipped, incidents resolved, customer impact).

### Pattern: Metrics as Performance Reviews

**Problem**: Individual engineer's metrics are used in performance review. Engineer A has 50 commits; Engineer B has 15. Engineer B is deemed less productive.

**Corrective**: Never use code metrics as individual performance measures. Use for team-level insights only. Personal growth assessments require managerial judgment, peer review, customer feedback.

### Pattern: Vanity Metrics

**Problem**: "Our avg PR merge time is 30 minutes!" But what matters is lead time and MTTR, not internal process speed. PR review time is noise if code quality is low.

**Corrective**: Focus on customer-impacting outcomes (lead time, reliability, customer satisfaction). Internal efficiency metrics matter only if they correlate with outcomes.

### Pattern: Metric Creep

**Problem**: Dashboard grows every quarter. "Let's also track time-in-review, commit frequency, documentation pages written, test coverage, cyclomatic complexity..." Decision paralysis ensues.

**Corrective**: Keep the dashboard **lean** — 8-12 top-line metrics max. Annual review: "Are all these still used in decisions? Drop any that are noise."