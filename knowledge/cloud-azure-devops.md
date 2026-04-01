# Azure DevOps — Pipelines, Boards, Repos & Artifact Management

Azure DevOps is Microsoft's integrated platform for software development, covering CI/CD (Pipelines), work tracking (Boards), source control (Repos), and package distribution (Artifacts). It bridges both traditional waterfall (work items, sprints) and modern CI/CD practices, with two execution models: YAML (infrastructure-as-code) and classic UI (visual pipeline builder).

## Pipelines: YAML vs Classic

### YAML Pipelines

YAML pipelines are **version-controlled, declarative definitions** of build and release workflows. The `azure-pipelines.yml` file lives in the repository root.

```yaml
trigger:
  - main
  - develop

pool:
  vmImage: ubuntu-latest

stages:
  - stage: Build
    jobs:
      - job: CompileAndTest
        steps:
          - task: UseDotNet@2
            inputs:
              version: 6.0.x
          - script: dotnet build
            displayName: Build
          - task: DotNetCoreCLI@2
            inputs:
              command: test
            displayName: Run Tests

  - stage: Deploy
    dependsOn: Build
    condition: succeeded()
    jobs:
      - deployment: DeployToStaging
        environment: staging
        strategy:
          rolling:
            maxParallel: 2
            preDeploy:
              - script: echo Pre-deployment validation
            deploy:
              - task: AzureWebApp@1
                inputs:
                  azureSubscription: $(serviceConnection)
                  appName: myapp-staging
                  package: $(Pipeline.Workspace)/build
```

**Key primitives:**
- **Triggers:** Branches, schedules, pull request filters
- **Stages:** Logical groupings (Build, Test, Deploy); can be conditional
- **Jobs:** Units within stages; run sequentially by default or parallel
- **Steps:** Individual tasks (script, command, custom task)
- **Conditions:** Skip/run based on environment variables or previous stage success

**Advantages:**
- Version control: Pipeline changes tracked in git
- Code review: Pull requests include pipeline modifications
- Reusability: Templates reduce duplication

### Classic Pipelines

Classic pipelines are **UI-driven, server-side definitions.** No YAML; the pipeline definition is stored in Azure DevOps and accessed via web UI.

**When to use classic:**
- Organizations with strict approval workflows (no code changes to pipeline)
- Legacy projects without git repository
- Visual drag-and-drop preferred by non-engineers

**When NOT to use classic:**
- Infrastructure-as-code discipline required
- Dynamic pipeline generation (cannot be done via UI)
- Modern CI/CD expectations (most orgs standardize on YAML)

## Triggers and Gates

### Build Triggers

Pipelines execute when:
- **Push trigger:** Code pushed to specified branch
- **Pull request trigger:** PR opened/updated targeting specified branch
- **Scheduled trigger:** Cron-style (daily, weekly, nightly builds)
- **Manual trigger:** Human clicks "Run" button

```yaml
trigger:
  branches:
    include:
      - main
      - develop
    exclude:
      - release/*
  paths:
    include:
      - src/**
      - azure-pipelines.yml
    exclude:
      - docs/**
```

### Deployment Gates

Deployment gates are **policy checks** that must pass before advancing to the next stage. They prevent problematic code (failing tests, security issues) from reaching production.

```yaml
- stage: Deploy
  jobs:
    - deployment: Deploy
      environment: production
      strategy:
        rolling:
          maxParallel: 1
```

Within the environment configuration (UI):
- **Approvals:** Human review required before deploy (email, Teams)
- **Gates:** Automated checks (pass/fail):
  - Query work items: "All bugs resolved?"
  - Invoke REST API: "Health checks passing?"
  - Publish code coverage: "Threshold met?"
  - Manual intervention: Operator manual approval

Gates execute asynchronously; pipeline waits for all gates to pass or timeout (default 60 min).

**Example gate:**
```
Gate: Invoke Azure Function
  – Hits endpoint: https://myapp.azurewebsites.net/api/health
  – Passes if HTTP 200 AND response body contains "healthy"
  – Fails if timeout or unexpected response
```

## Boards and Work Tracking

### Work Items

Work tracking in Azure DevOps uses **work items**, the fundamental unit of work:

- **Epic:** Large feature spanning multiple sprints
- **Feature:** Deliverable within one or more sprints
- **User Story:** Specific requirement from customer perspective
- **Task:** Implementation unit (dev, QA, design)
- **Bug:** Defect reported and tracked
- **Issue:** Support or unplanned work

Each work item has:
- **State:** New, Active, Resolved, Closed (customizable)
- **Assigned to:** Developer or team owner
- **Priority:** 1 (high) to 4 (low)
- **Sprint assignment:** Which iteration (sprint)
- **Story points:** Effort estimate (Fibonacci: 1, 2, 3, 5, 8...)

```
Epic: E-commerce Checkout System
├── Feature: Payment Processing
│   ├── User Story: Stripe integration
│   │   ├── Task: API setup
│   │   ├── Task: Token handling
│   │   └── Bug: Refunds failing
│   └── User Story: Apple Pay support
└── Feature: Order Tracking
    └── User Story: Webhook updates
```

### Scrum Framework: Sprints

Sprints organize work into **time-boxed iterations** (usually 2 weeks).

**Sprint lifecycle:**
1. **Planning:** Team commits to story points; backlog items assigned to sprint
2. **Execution:** Daily standups; work progresses through states (New → Active → Resolved)
3. **Review:** Demo completed work to stakeholders
4. **Retrospective:** Team reflects on process improvements
5. **Close:** Sprint closes; unfinished work returns to backlog

**Velocity:** Sum of story points completed in a sprint. Forecasts future capacity.

Example: Velocity 40 points/sprint (historical average) → plan 40 points for next sprint.

## Repos: Source Control

Azure DevOps Repos provides **Git repositories** (also supports TFVC, Microsoft's centralized version control, now deprecated).

- **Repositories:** One or more per project
- **Branches:** Feature branches, release branches, main
- **Pull requests:** Code review with approval policies
- **Branch policies:** Enforce minimum reviewers, builds must pass, comments resolved

```yaml
Branch policy: main
  – Min reviewers: 2
  – Auto-complete on approval
  – Dismiss stale pull request approvals on new push
  – Build validation (pipeline must pass)
  – Comment requirements: All must be resolved
```

Pull request workflow:
1. Developer creates branch from main
2. Pushes commits; creates PR
3. Reviewers comment and approve
4. CI pipeline (Pipelines) runs; must pass
5. Auto-complete merges to main when conditions met

## Artifacts: Package Management

Azure Artifacts is NuGet, npm, Maven, Python package distribution within Azure DevOps.

### Feeds

A **feed** is a container for packages of a language:

```
Organization
├── Feed: internal-libs (NuGet)
├── Feed: node-packages (npm)
└── Feed: py-packages (Python)
```

**Upstream sources:** Reference external registries (nuget.org, npmjs.com, pypi.org) as fallback.

```
My Feed (nuget-app)
  – Upstream: nuget.org
  – Publish privately built packages to my-feed
  – Consume from my-feed (local packages)
  – Fallback to nuget.org (missing packages)
```

## Self-Hosted Agents

A **self-hosted agent** is a VM or machine that runs pipeline jobs instead of Microsoft-hosted agents.

**Microsoft-hosted agents:**
- Short-lived (one job → destroy)
- Limited to 6 hours per job
- Hosted in Azure
- Cost: included in DevOps subscription (5 free, $40/month each additional)

**Self-hosted agents:**
- Long-lived (persistent VM)
- Can accumulate state, cache (faster builds)
- Support custom software (older runtimes, GPUs)
- Can be on-premises, private cloud

```yaml
pool:
  name: MyAgentPool
  demands:
    - Agent.OS -equals Linux
    - Agent.Version -gtVersion 2.144.0

agent configuration:
  – Download agent zip
  – ./config.sh --url https://dev.azure.com/org --auth pat --token $PAT --pool MyAgentPool
  – ./run.sh
```

**Agent pools** group related agents (all Linux agents, all on-premises agents, etc.).

## Service Connections

A **service connection** is a secure credential store for external services.

```yaml
- task: AzureWebApp@1
  inputs:
    azureSubscription: $(serviceConnection)  # References service connection
    appName: myapp
```

Azure DevOps stores credentials encrypted; developers reference by name, not value. Types:
- **Azure subscription** (service principal or managed identity)
- **GitHub** (PAT, OAuth)
- **Docker Registry** (username/password)
- **Kubernetes** (kubeconfig)

## Template Reuse

Templates reduce duplication across pipelines.

### Job Template

```yaml
# jobs/test.yml
parameters:
  - name: buildConfiguration
    type: string

jobs:
  - job: Test
    steps:
      - script: dotnet test -c ${{ parameters.buildConfiguration }}
```

**Usage:**
```yaml
# azure-pipelines.yml
stages:
  - stage: Test
    jobs:
      - template: jobs/test.yml
        parameters:
          buildConfiguration: Release
```

### Stage Template

```yaml
# stages/deploy.yml
parameters:
  - name: environmentName
    type: string
  - name: appName
    type: string

stages:
  - stage: Deploy${{ parameters.environmentName }}
    jobs:
      - deployment: Deploy
        environment: ${{ parameters.environmentName }}
        strategy:
          runOnce:
            deploy:
              - task: AzureWebApp@1
                inputs:
                  appName: ${{ parameters.appName }}
```

**Composable pipelines:** One line per environment → DRY patterns.

## Multi-Stage Pipelines: Build → Test → Deploy

A typical multi-stage pipeline:

1. **Build stage** (runs on every commit):
   - Compile source code
   - Run unit tests
   - Publish build artifacts

2. **Test stage** (depends on Build):
   - Integration tests
   - Smoke tests
   - Performance benchmarks

3. **Deploy-Staging** (depends on Test):
   - Deploy to staging environment
   - Run end-to-end tests
   - Require human approval gate

4. **Deploy-Production** (depends on Deploy-Staging):
   - Deploy to production
   - Blue-green or canary strategy
   - Health checks

Conditions:
- Test stage skipped if Build fails
- Staging deploy skipped if tests fail
- Production requires manual gate

## See Also

- `devops-cicd` — CI/CD concepts and patterns
- `cloud-aws-cicd` — GitHub Actions, AWS CodePipeline comparisons
- `process-team-topologies` — Organizational alignment with pipeline governance
- `architecture-twelve-factor` — Configuration and environment management principles