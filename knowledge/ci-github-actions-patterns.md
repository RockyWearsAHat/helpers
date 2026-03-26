# CI/CD: GitHub Actions Patterns

## Overview

GitHub Actions is GitHub's native CI/CD platform built into every repository. Workflows are YAML files in `.github/workflows/` that respond to repository events (push, PR, schedule, manual trigger) and execute jobs on runners (GitHub-hosted or self-hosted). Patterns for composability, parameterization, parallelization, and security enable complex pipelines with minimal repetition.

Workflows are versioned in the repository, enabling immediate CI changes without external configuration drift.

## Reusable Workflows

Reusable workflows package common CI logic as first-class workflow templates, eliminating duplication across many repositories.

**Defining a reusable workflow** (`workflow-shared.yml`):

```yaml
name: Shared Workflow
on:
  workflow_call:
    inputs:
      environment:
        type: string
        required: true
    secrets:
      deploy_token:
        required: true
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Deploy
        run: deploy.sh
        env:
          DEPLOY_TOKEN: ${{ secrets.deploy_token }}
          ENV: ${{ inputs.environment }}
```

**Calling a reusable workflow** (`workflow-caller.yml`):

```yaml
jobs:
  call-deploy:
    uses: ./.github/workflows/workflow-shared.yml@v1
    with:
      environment: production
    secrets:
      deploy_token: ${{ secrets.DEPLOY_TOKEN }}
```

Reusable workflows support inputs (string, number, boolean) and secrets. The caller passes values via `with` and `secrets`. Reusable workflows can call other reusable workflows (max 10 levels of nesting).

Versioning: use `@ref` to pin to a branch, tag, or commit SHA. SHA is safest; branches/tags can move.

## Composite Actions

Composite actions group workflow steps into reusable units within a single job, allowing step-level composition.

**Defining a composite action** (`.github/actions/deploy/action.yml`):

```yaml
name: Deploy
description: Deploy application to environment
inputs:
  environment:
    description: Target environment
    required: true
outputs:
  deploy-url:
    value: ${{ steps.deploy.outputs.url }}
runs:
  using: composite
  steps:
    - uses: actions/checkout@v4
    - name: Build
      shell: bash
      run: npm run build
    - name: Deploy
      id: deploy
      shell: bash
      run: |
        ./scripts/deploy.sh ${{ inputs.environment }}
        echo "url=https://${{ inputs.environment}}.example.com" >> $GITHUB_OUTPUT
```

**Using a composite action**:

```yaml
steps:
  - uses: ./.github/actions/deploy
    with:
      environment: staging
    id: deploy
  - run: echo "Deployed to ${{ steps.deploy.outputs.deploy-url }}"
```

Composite actions support inputs, outputs, and multiple shells (bash, pwsh, etc.). They are lightweight and ideal for grouping setup or deployment logic.

Unlike JavaScript or Docker actions, composites do not require distribution; they live in the repository.

## Matrix Strategy

The matrix strategy creates multiple job runs from a single job definition, useful for testing against many environments or configurations.

```yaml
strategy:
  matrix:
    node-version: [18, 20, 22]
    os: [ubuntu-latest, windows-latest, macos-latest]
runs-on: ${{ matrix.os }}
steps:
  - uses: actions/setup-node@v4
    with:
      node-version: ${{ matrix.node-version }}
  - run: npm test
```

This creates 3 × 3 = 9 job runs (one per OS and Node version combination). Each run has a unique matrix context accessible in steps.

**Matrix inclusion/exclusion**:

```yaml
strategy:
  matrix:
    node: [18, 20]
    os: [ubuntu-latest, macos-latest]
    include:
      - node: 22
        os: ubuntu-latest
        experimental: true
    exclude:
      - os: macos-latest
        node: 18
```

`include` adds specific combinations; `exclude` removes them. This enables selective testing (e.g., only test Node 22 on Linux, exclude old OS/Node combos).

Matrix dimensions are limited; too many combinations cause excessive job sprawl. Aim for 10–20 matrix jobs; beyond that, consider multiple workflows.

## Caching with `actions/cache`

The cache action reduces build times by storing dependencies and build artifacts between runs.

```yaml
- uses: actions/cache@v4
  with:
    path: node_modules
    key: ${{ runner.os }}-npm-${{ hashFiles('package-lock.json') }}
    restore-keys: |
      ${{ runner.os }}-npm-
```

The `key` must be unique per cache entry. `hashFiles()` creates a hash of specified files; if dependencies change, the cache key changes and a new cache is created. `restore-keys` specifies fallback keys if the primary key misses (e.g., restore any npm cache for the OS if the exact lock file hash misses).

Cache is stored per branch. Pushing to a new branch creates a new cache; it does not inherit from main. This can slow branch-specific tests. Use GitHub's cache APIs (via `actions/cache`) to share caches across branches cautiously.

## Artifacts

Artifacts preserve build outputs and logs between jobs and make them available after the workflow completes.

```yaml
- name: Build
  run: npm run build

- uses: actions/upload-artifact@v4
  with:
    name: dist
    path: dist/
    retention-days: 30

# Later job
- uses: actions/download-artifact@v4
  with:
    name: dist
    path: ./dist

- run: npm run deploy -- ./dist
```

Artifacts are versioned per workflow run and downloadable from the Actions tab. Retention defaults to 90 days; set `retention-days` to override. Large artifacts (e.g., full node_modules) increase storage costs.

## OIDC for Cloud Authentication

OpenID Connect (OIDC) enables GitHub Actions to authenticate to cloud providers (AWS, Azure, GCP) without storing long-lived credentials as secrets.

**AWS example**:

```yaml
permissions:
  id-token: write
  contents: read

- uses: aws-actions/configure-aws-credentials@v4
  with:
    role-to-assume: arn:aws:iam::123456789:role/GitHubActionsRole
    aws-region: us-east-1

- run: aws s3 cp dist/ s3://my-bucket/
```

GitHub exchanges a short-lived OIDC token for temporary AWS credentials. The token is signed and verified by GitHub; you trust GitHub's key in your cloud provider config.

OIDC eliminates credential rotation, access key sprawl, and the risk of leaked secrets. Major clouds (AWS, Azure, GCP) support OIDC with GitHub.

## Environments and Protection Rules

Environments enable per-environment configuration and approval gates.

**Defining an environment** (Repository Settings → Environments):

- Set environment secrets (e.g., `PRODUCTION_DEPLOY_KEY`).
- Add deployment protection rules (e.g., require approval).
- Restrict which branches can deploy to the environment.

**Using an environment in a workflow**:

```yaml
jobs:
  deploy:
    environment: production
    runs-on: ubuntu-latest
    steps:
      - run: deploy.sh
        env:
          DEPLOY_KEY: ${{ secrets.DEPLOY_KEY }}
```

When a job specifies `environment: production`, GitHub checks protection rules before running. If approval is required and no request is pending, the job waits. Reviewers can approve or reject from the Actions tab.

This pattern ensures controlled deployments: only specific people can approve production changes, and all deployments are audited.

## Concurrency Control

The `concurrency` key ensures that only one workflow run for a given concurrency group executes at a time. Useful for deployments where concurrent deploys to the same environment would conflict.

```yaml
concurrency:
  group: deploy-production
  cancel-in-progress: false

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - run: deploy.sh
```

If `cancel-in-progress: true`, a new run cancels the previous run in the group. This is useful for ensuring the latest code deploys; older runs are discarded.

`group` can reference context (`${{ github.ref }}`) to create per-branch concurrency groups:

```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

## Self-Hosted Runners

GitHub-hosted runners are convenient but limited to public repositories and have resource constraints. Self-hosted runners run on your own hardware.

**Adding a self-hosted runner** (Repository Settings → Actions → Runners):

1. GitHub provides a shell script to register the runner.
2. Run the script on your machine or VM.
3. The runner polls GitHub for jobs and executes them.

**Using a self-hosted runner in a workflow**:

```yaml
runs-on: [self-hosted, linux, x64]
```

Specify labels that match runner tags. Self-hosted runners are useful for:

- Testing against specific hardware (GPU, high memory).
- Running tests that require private network access.
- Using tools/dependencies installed on the machine.
- Massive parallelization (100s of tests across many machines).

**Security**: Self-hosted runners can execute arbitrary code in workflows. Restrict runner access to trusted branches/workflows. Keep the runner process and host software up to date.

## Workflow Dispatch and Manual Triggers

The `workflow_dispatch` event allows manual trigger from the GitHub UI without a code push.

```yaml
on:
  workflow_dispatch:
    inputs:
      environment:
        description: Target environment
        required: true
        default: staging
        type: choice
        options:
          - staging
          - production
```

This creates a "Run workflow" button on the Actions tab with input fields. Useful for manual deployments, backfill operations, or one-off tasks.

## Job Dependencies and Output Passing

Jobs run in parallel by default. Use `needs` to establish dependencies.

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    outputs:
      image: ${{ steps.build.outputs.image }}
    steps:
      - id: build
        run: echo "image=myapp:latest" >> $GITHUB_OUTPUT

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - run: deploy ${{ needs.build.outputs.image }}
```

`deploy` waits for `build` to complete, then accesses outputs via `needs.build.outputs.image`. This pattern chains jobs while preserving parallelism: multiple independent jobs run in parallel; dependent jobs wait.

## Practical Patterns

1. **Branch protection rules**: Require status checks to pass (e.g., "Tests") before merging. Prevents broken code in main.
2. **Separate build and deploy**: Build once, then deploy that artifact to multiple environments. Avoids rebuilding per environment.
3. **Cache dependencies aggressively**: npm, pip, Maven caches reduce workflow time significantly.
4. **Use secrets wisely**: Never log secrets. Use `@GITHUB_ENV` or step outputs carefully; outputs may be logged.
5. **Environment promotion**: Staging → Production gate with manual approval. Ensures conservative deployments.

## See Also

- [devops-github-actions.md](devops-github-actions.md) — GitHub Actions basics and configuration
- [devops-github-actions-deep.md](devops-github-actions-deep.md) — Advanced workflows and security
- [devops-cicd-patterns.md](devops-cicd-patterns.md) — General CI/CD patterns (trunk-based, branching)
- [security-secrets-management.md](security-secrets-management.md) — Secret management and best practices