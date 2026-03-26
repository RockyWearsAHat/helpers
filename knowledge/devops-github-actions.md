# GitHub Actions

## Workflow Syntax

```yaml
name: CI/CD Pipeline
on:
  push:
    branches: [main, develop]
    paths-ignore: ["docs/**", "*.md"]
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened]
  schedule:
    - cron: "0 6 * * 1" # Monday 6am UTC
  workflow_dispatch:
    inputs:
      environment:
        description: "Deploy target"
        required: true
        type: choice
        options: [staging, production]
      dry_run:
        description: "Dry run only"
        type: boolean
        default: true

permissions:
  contents: read
  pull-requests: write

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true # cancel older runs on same branch

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm test
```

## Runners

| Runner           | OS                  | Notes                          |
| ---------------- | ------------------- | ------------------------------ |
| `ubuntu-latest`  | Ubuntu 22.04        | Most common, 7GB RAM, 14GB SSD |
| `ubuntu-24.04`   | Ubuntu 24.04        | Pin specific version           |
| `macos-latest`   | macOS 14 (Sonoma)   | ARM64 (M1). 10x cost vs Linux. |
| `macos-13`       | macOS 13 (Ventura)  | Intel. Use for x86 builds.     |
| `windows-latest` | Windows Server 2022 | 2x cost vs Linux               |

### Self-hosted runners

```yaml
runs-on: [self-hosted, linux, x64, gpu] # label matching
```

Use for: GPU workloads, private network access, cost optimization at scale, custom hardware. Manage with **actions/runner** or **actions-runner-controller** (ARC) for Kubernetes.

## Contexts and Expressions

```yaml
env:
  DEPLOY_ENV: ${{ github.event.inputs.environment || 'staging' }}

steps:
  - name: Conditional step
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    run: echo "Deploying to production"

  - name: Check PR label
    if: contains(github.event.pull_request.labels.*.name, 'deploy')
    run: echo "Has deploy label"

  - name: Always run (even on failure)
    if: always()
    run: echo "Cleanup"

  - name: Only on failure
    if: failure()
    run: echo "Something failed"
```

### Key Contexts

| Context   | Common Properties                                                              |
| --------- | ------------------------------------------------------------------------------ |
| `github`  | `.sha`, `.ref`, `.event_name`, `.actor`, `.repository`, `.run_id`, `.workflow` |
| `env`     | Environment variables                                                          |
| `vars`    | Repository/org/environment variables                                           |
| `secrets` | Repository/org/environment secrets                                             |
| `job`     | `.status`, `.container`, `.services`                                           |
| `steps`   | `steps.<id>.outputs.<name>`, `steps.<id>.outcome`                              |
| `runner`  | `.os`, `.arch`, `.temp`, `.tool_cache`                                         |
| `matrix`  | Current matrix combination values                                              |
| `needs`   | Outputs from dependent jobs                                                    |

### Expression Functions

```yaml
# String
${{ contains('hello world', 'hello') }}
${{ startsWith(github.ref, 'refs/tags/') }}
${{ format('Hello {0}', github.actor) }}

# JSON
${{ toJSON(github.event) }}
${{ fromJSON(steps.metadata.outputs.json) }}

# Status
${{ success() }}    # default implicit condition
${{ failure() }}    # previous step failed
${{ always() }}     # always run
${{ cancelled() }}  # workflow cancelled
```

## Secrets and Variables

```yaml
jobs:
  deploy:
    environment: production # activates environment secrets
    steps:
      - run: |
          curl -H "Authorization: Bearer ${{ secrets.API_TOKEN }}" \
               ${{ vars.API_BASE_URL }}/deploy
```

**Hierarchy** (highest precedence wins): environment secrets/vars → repository secrets/vars → organization secrets/vars.

**OIDC** (no long-lived credentials):

```yaml
permissions:
  id-token: write
  contents: read

steps:
  - uses: aws-actions/configure-aws-credentials@v4
    with:
      role-to-assume: arn:aws:iam::123456789012:role/GitHubActionsRole
      aws-region: us-east-1
      # No access key/secret needed — uses OIDC token exchange
```

## Environments

```yaml
jobs:
  deploy-staging:
    environment: staging
    # ...

  deploy-production:
    needs: deploy-staging
    environment:
      name: production
      url: https://app.example.com
    # ...
```

**Protection rules** (configured in repo settings): required reviewers, wait timer, branch restrictions, custom deployment protection rules (via GitHub Apps).

## Caching and Artifacts

```yaml
# Caching dependencies
- uses: actions/cache@v4
  with:
    path: |
      ~/.npm
      node_modules
    key: ${{ runner.os }}-node-${{ hashFiles('**/package-lock.json') }}
    restore-keys: |
      ${{ runner.os }}-node-

# Upload artifact
- uses: actions/upload-artifact@v4
  with:
    name: build-output
    path: dist/
    retention-days: 5
    if-no-files-found: error

# Download in another job
- uses: actions/download-artifact@v4
  with:
    name: build-output
    path: dist/
```

**Cache vs Artifact**: Cache persists between runs of the same workflow (dependencies, build caches). Artifacts persist within a single run across jobs (build outputs, test results, logs). Cache limit: 10GB per repo. Artifact retention: configurable, default 90 days.

## Matrix Strategies

```yaml
jobs:
  test:
    strategy:
      fail-fast: false # don't cancel other matrix jobs on failure
      max-parallel: 4
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        node: [18, 20, 22]
        exclude:
          - os: macos-latest
            node: 18
        include:
          - os: ubuntu-latest
            node: 22
            coverage: true # extra variable for this combo
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node }}
      - run: npm test
      - if: matrix.coverage
        run: npm run coverage
```

### Dynamic Matrix

```yaml
jobs:
  prepare:
    outputs:
      matrix: ${{ steps.set-matrix.outputs.matrix }}
    steps:
      - id: set-matrix
        run: |
          echo "matrix=$(jq -c . matrix.json)" >> "$GITHUB_OUTPUT"

  build:
    needs: prepare
    strategy:
      matrix: ${{ fromJSON(needs.prepare.outputs.matrix) }}
```

## Reusable Workflows

```yaml
# .github/workflows/deploy-reusable.yml
name: Reusable Deploy
on:
  workflow_call:
    inputs:
      environment:
        required: true
        type: string
      version:
        required: true
        type: string
    secrets:
      DEPLOY_TOKEN:
        required: true
    outputs:
      deploy_url:
        description: "Deployed URL"
        value: ${{ jobs.deploy.outputs.url }}

jobs:
  deploy:
    runs-on: ubuntu-latest
    outputs:
      url: ${{ steps.deploy.outputs.url }}
    steps:
      - run: echo "Deploying ${{ inputs.version }} to ${{ inputs.environment }}"
```

```yaml
# Caller workflow
jobs:
  deploy-staging:
    uses: ./.github/workflows/deploy-reusable.yml
    with:
      environment: staging
      version: ${{ needs.build.outputs.version }}
    secrets:
      DEPLOY_TOKEN: ${{ secrets.DEPLOY_TOKEN }}
    # Alternative: secrets: inherit   (pass all secrets)
```

## Action Development

### Composite Action

```yaml
# .github/actions/setup-project/action.yml
name: "Setup Project"
description: "Install deps and build"
inputs:
  node-version:
    description: "Node.js version"
    default: "20"
outputs:
  cache-hit:
    description: "Whether cache was hit"
    value: ${{ steps.cache.outputs.cache-hit }}
runs:
  using: "composite"
  steps:
    - uses: actions/setup-node@v4
      with:
        node-version: ${{ inputs.node-version }}
    - id: cache
      uses: actions/cache@v4
      with:
        path: node_modules
        key: ${{ runner.os }}-node-${{ hashFiles('package-lock.json') }}
    - if: steps.cache.outputs.cache-hit != 'true'
      run: npm ci
      shell: bash
    - run: npm run build
      shell: bash
```

### Action Types

| Type           | Language                  | Use Case                               |
| -------------- | ------------------------- | -------------------------------------- |
| **Composite**  | YAML (calls other steps)  | Orchestration, reusable step sequences |
| **JavaScript** | Node.js 20                | Fast startup, API calls, complex logic |
| **Docker**     | Any language in container | Custom runtime, system dependencies    |

### JavaScript Action Core

```javascript
// index.js
const core = require("@actions/core");
const github = require("@actions/github");

async function run() {
  try {
    const token = core.getInput("github-token", { required: true });
    const octokit = github.getOctokit(token);

    const { data: pr } = await octokit.rest.pulls.get({
      ...github.context.repo,
      pull_number: github.context.payload.pull_request.number,
    });

    core.setOutput("pr-title", pr.title);
    core.info(`Processing PR: ${pr.title}`);
  } catch (error) {
    core.setFailed(error.message);
  }
}

run();
```

## Job Outputs and Dependencies

```yaml
jobs:
  build:
    outputs:
      version: ${{ steps.version.outputs.value }}
      image: ${{ steps.build.outputs.image }}
    steps:
      - id: version
        run: echo "value=$(cat VERSION)" >> "$GITHUB_OUTPUT"

  deploy:
    needs: [build, test]           # parallel deps
    if: needs.build.result == 'success'
    steps:
      - run: echo "Deploying ${{ needs.build.outputs.version }}"

  notify:
    needs: [deploy]
    if: always()                   # run even if deploy fails
    steps:
      - run: echo "Deploy status: ${{ needs.deploy.result }}"
```

## Services (Sidecar Containers)

```yaml
jobs:
  integration-test:
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: testpass
          POSTGRES_DB: testdb
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7
        ports:
          - 6379:6379
    steps:
      - run: npm run test:integration
        env:
          DATABASE_URL: postgresql://postgres:testpass@localhost:5432/testdb
          REDIS_URL: redis://localhost:6379
```

## Security Patterns

```yaml
# Pin actions to commit SHA (not tag — tags can be moved)
- uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11  # v4.1.1

# Minimize permissions
permissions:
  contents: read

# Never echo secrets
- run: |
    # Risky: echo "${{ secrets.TOKEN }}"
    # Safer: mask in logs
    echo "::add-mask::$SECRET_VALUE"

# Restrict workflow triggers for public repos
on:
  pull_request_target:     # runs in base context — careful with checkout
```

**Supply chain**: use `actions/dependency-review-action` to block PRs introducing vulnerable deps. Use Dependabot to keep action versions current. Restrict `GITHUB_TOKEN` permissions to minimum needed.

## Workflow Commands

```bash
# Set output
echo "name=value" >> "$GITHUB_OUTPUT"

# Set environment variable for subsequent steps
echo "MY_VAR=hello" >> "$GITHUB_ENV"

# Add to PATH
echo "/custom/bin" >> "$GITHUB_PATH"

# Logging
echo "::notice file=app.js,line=1::Check this"
echo "::warning::Deprecation notice"
echo "::error file=src/main.js,line=10,col=5::Missing semicolon"

# Grouping
echo "::group::Install Dependencies"
npm ci
echo "::endgroup::"

# Job summary (Markdown)
echo "### Build Results :rocket:" >> "$GITHUB_STEP_SUMMARY"
echo "| Test | Result |" >> "$GITHUB_STEP_SUMMARY"
echo "| --- | --- |" >> "$GITHUB_STEP_SUMMARY"
echo "| Unit | :white_check_mark: |" >> "$GITHUB_STEP_SUMMARY"
```
