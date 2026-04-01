# GitHub Actions: Advanced Workflows, Security, and Composition

## Composite Actions: Modular Workflow Logic

Composite actions package workflow steps into reusable units. They reduce duplication, enable version control, and simplify complex pipelines.

```yaml
# .github/actions/deploy/action.yml
name: "Deploy Application"
description: "Deploy with custom build configuration"
inputs:
  environment:
    description: "Deployment target"
    required: true
  ref:
    description: "Git ref to deploy"
    required: false
    default: ${{ github.ref }}
  dry-run:
    description: "Skip actual deployment"
    required: false
    default: "false"
outputs:
  deployment-id:
    description: "ID of created deployment"
    value: ${{ steps.deploy.outputs.id }}
  deployment-url:
    description: "URL of deployed application"
    value: ${{ steps.deploy.outputs.url }}
runs:
  using: "composite"
  steps:
    - uses: actions/checkout@v4
      with:
        ref: ${{ inputs.ref }}
    - uses: actions/setup-node@v4
      with:
        node-version: 20
    - name: Install dependencies
      run: npm ci
      shell: bash
    - name: Build
      run: npm run build
      shell: bash
    - id: deploy
      name: Deploy
      run: |
        DEPLOY_ID=$(./scripts/deploy.sh ${{ inputs.environment }} ${{ inputs.dry-run }})
        echo "id=$DEPLOY_ID" >> $GITHUB_OUTPUT
        echo "url=https://${{ inputs.environment }}.example.com" >> $GITHUB_OUTPUT
      shell: bash
```

**Usage in workflow:**

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: ./.github/actions/deploy
        with:
          environment: staging
      - uses: ./.github/actions/deploy
        with:
          environment: production
          dry-run: "true"
```

**Patterns:**
- Input parameters control behavior (no logic duplication across workflows)
- Output parameters pass results to downstream steps
- Use `shell: bash` explicitly (required in composite actions; scripts not inherited)
- Composite actions can nest (call other composites or GitHub Marketplace actions)

---

## Reusable Workflows: Cross-Repository Orchestration

Reusable workflows enable organization-wide CI/CD standards: a central testing workflow, deployment patterns, or security scanning runs consistently across all projects.

```yaml
# .github/workflows/test.yml (must be in default branch)
name: Test Suite
on:
  workflow_call:
    inputs:
      node-version:
        type: string
        required: false
        default: "20"
      test-timeout:
        type: number
        required: false
        default: 600
    secrets:
      npm-token:
        required: false
    outputs:
      coverage:
        description: "Test coverage percentage"
        value: ${{ jobs.test.outputs.coverage }}

jobs:
  test:
    runs-on: ubuntu-latest
    outputs:
      coverage: ${{ steps.coverage.outputs.result }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ inputs.node-version }}
          registry-url: "https://npm.pkg.github.com"
      - run: npm ci
        env:
          NODE_AUTH_TOKEN: ${{ secrets.npm-token }}
      - run: npm test -- --coverage
        timeout-minutes: ${{ inputs.test-timeout }}
      - id: coverage
        run: echo "result=$(grep -oP 'Total.*?\K\d+(?=%)' coverage/coverage-summary.json)" >> $GITHUB_OUTPUT
```

**Caller workflow:**

```yaml
jobs:
  test-node:
    uses: ./.github/workflows/test.yml
    with:
      node-version: "18"
      test-timeout: 300
    secrets:
      npm-token: ${{ secrets.NPM_TOKEN }}

  test-node-next:
    uses: ./.github/workflows/test.yml
    with:
      node-version: "21"

  deploy:
    needs: [test-node, test-node-next]
    runs-on: ubuntu-latest
    steps:
      - run: echo "Coverage: ${{ needs.test-node.outputs.coverage }}"
```

**Key constraints:**
- Reusable workflow must be in `.github/workflows/` and must exist on default branch
- Caller can pass inputs, secrets (caller chooses which to forward); can access outputs via `needs.<job>.outputs.<name>`
- Reusable workflows cannot call other reusable workflows (no nesting)
- Timeout and other job settings are set by caller via workflow inputs

---

## Matrix Strategies: Parameterized Job Execution

Matrix runs the same job with multiple input combinations, generating separate job instances for each combination.

```yaml
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        node-version: [18, 20, 21]
        include:
          - os: ubuntu-latest
            node-version: 20
            is-canary: true
        exclude:
          - os: macos-latest
            node-version: 18  # M1 Macs don't support old Node versions
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node-version }}
      - run: npm test
        if: matrix.is-canary != true
      - run: npm test -- --experimental-features
        if: matrix.is-canary == true
```

**Via JSON file:**

```yaml
strategy:
  matrix:
    include: ${{ fromJson(secrets.TEST_MATRIX) }}
```

Where `TEST_MATRIX` secret contains:

```json
[
  {"os": "ubuntu-latest", "node-version": 20},
  {"os": "windows-latest", "node-version": 18}
]
```

**Matrix coordination:**
- `include`: Add or override combinations
- `exclude`: Remove combinations (e.g., unsupported OS/version pairs)
- Max 256 jobs per workflow run; overflowing matrix is capped silently

---

## Caching: Layer-Based Storage for Dependency Management

Cache layers: node_modules → .npm cache → compiled artifacts. Multiple cache scopes and restore strategy.

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: "npm"  # setup-node auto-handles npm cache key+restore
      - run: npm ci
      - run: npm run build
      - uses: actions/cache@v4
        with:
          path: dist/
          key: dist-${{ github.sha }}  # unique per commit
          restore-keys:
            - dist-                    # fallback: any older build
      - uses: actions/upload-artifact@v4
        with:
          name: dist
          path: dist/
          retention-days: 1

  deploy:
    needs: build
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: dist
          path: dist/
```

**Cache key semantics:**
- Key computed via SHA256; exact match wins
- Restore keys are attempted in order (prefix matching)
- Cache is per branch; branches share if scopes overlap
- Default retention: 7 days; manual cleanup via `cache.delete()`
- Size limit: 10GB per repository; LRU eviction

**Gotchas:**
- Caching `node_modules` works but is slower than letting `npm ci` rebuild (npm's cache lookup is fast)
- Package-lock.json changes invalidate cache (by design; prevents installing stale lockfile)
- Cache key should include dependency manifest hash (e.g., `hashFiles('**/package-lock.json')`)

---

## Artifacts: Preserving Build Outputs Across Jobs

Artifacts store files (binaries, coverage reports, test results) between jobs or for download.

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: npm run build
      - run: npm test -- --coverage --json --outputFile=coverage.json
      - uses: actions/upload-artifact@v4
        with:
          name: coverage-${{ matrix.os }}
          path: coverage.json
          retention-days: 30
          compression-level: 6  # 0-9; 0=no compression

  report:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: coverage-data/  # downloads all artifacts into this dir
      - run: |
          find coverage-data -name 'coverage.json' | \
          xargs jq -s 'add | {total: (100 * .branches.covered / .branches.total)}' > summary.json
          cat summary.json
```

**Limitations:**
- 5GB per artifact; 100GB per workflow run
- Retention 0-400 days (default: 90)
- Must be downloaded explicitly (artifacts don't auto-flow between jobs like outputs)
- Download only works after workflow completes (not mid-run)

---

## Environments and Deployment Protection

Environments gate deployment with rules and provide secret/variable scope isolation.

```yaml
# Repository settings: Environments → production

environments:
  production:
    # Require approval from these teams before deployment
    protected_branches:
      - main
    deployment_branches:
      - main
    reviewers:
      - type: Team
        id: 1234567  # DevOps team
    wait_timer: 3600  # 1 hour delay before deployment allowed

jobs:
  deploy:
    environment:
      name: production
      url: https://api.example.com
    steps:
      - run: curl -X POST https://deploy-api.example.com/deploy \
          -H "Authorization: Bearer ${{ secrets.DEPLOY_TOKEN }}"
      - name: Create deployment status
        uses: actions/github-script@v7
        with:
          script: |
            github.rest.repos.createDeploymentStatus({
              owner: context.repo.owner,
              repo: context.repo.repo,
              deployment_id: context.payload.deployment.id,
              state: 'success',
              environment_url: 'https://api.example.com'
            })
```

**Custom deployment protection rules** (via GitHub Apps):

```yaml
environments:
  production:
    deployment_protection_rules:
      - type: required_status_checks
        parameters:
          required_contexts: [security-scan, performance-test]
      - type: custom  # Requires GitHub App integration
```

---

## OIDC Token-Based Authentication: Eliminating Long-Lived Credentials

OIDC tokens automatically generated by GitHub Actions; exchanging them for cloud credentials avoids storing AWS keys, GCP service accounts, or similar.

```yaml
permissions:
  id-token: write  # Required to request OIDC token
  contents: read

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::123456789012:role/GitHubActionsRole
          aws-region: us-east-1
          # No credentials needed; OIDC token exchanged for STS temporary credentials
      - run: aws s3 cp dist/ s3://my-bucket/dist/ --recursive

      - uses: google-github-actions/auth@v2
        with:
          workload_identity_provider: projects/123456/locations/global/workloadIdentityPools/github-pool/providers/github-provider
          service_account: github-actions@my-project.iam.gserviceaccount.com

      - uses: hashicorp/setup-terraform@v3
        with:
          cli_config_credentials_token: ${{ secrets.TF_API_TOKEN }}
          # Or use OIDC:
          # token: ${{ env.ACTIONS_ID_TOKEN_REQUEST_TOKEN }}
```

**OIDC token structure (JWT):**

```json
{
  "iss": "https://token.actions.githubusercontent.com",
  "sub": "repo:owner/repo:ref:refs/heads/main",
  "aud": "sts.amazonaws.com",
  "iat": 1234567890,
  "exp": 1234571490,
  "actor": "github-user",
  "repository": "owner/repo",
  "ref": "refs/heads/main"
}
```

Entity (cloud role) validates claims: `sub` must match expected repo, `ref` must match branch, `actor` must be in approved list. No static secret rotation needed.

---

## Self-Hosted Runners: Security and Scaling

Self-hosted runners execute workflows on your hardware, network, or private cloud. Trade managed simplicity for control and cost.

### Security Considerations

```yaml
runs-on: [self-hosted, linux, x64]
```

**Risk model:**
- Untrusted workflow code (from PRs) runs with access to runner filesystem, network, secrets
- Runner can exfiltrate repository code or secrets
- Compromised runner can pivot to other infrastructure

**Hardening practices:**

1. **Isolate runners:** Dedicated network (VPC/subnet), no internet unless necessary, security groups restrict ingress
2. **Ephemeral runners:** Spin up per job, tear down after completion (AWS/GCP Spot instances + actions/runner controller)
3. **Limit repository scope:** Disable "public repositories" setting; restrict to private or trusted repos
4. **Use runner groups:** Assign specific runners to specific repos (Organization/Enterprise setting)
5. **Secrets rotation:** Rotate runner credentials and API tokens on schedule
6. **Monitor activity:** Log workflow executions, job logs, artifact downloads

### actions/runner-controller (ARC)

Deploy Actions Runners as Kubernetes pods; auto-scale based on job queue depth.

```yaml
# Helm chart values
runners:
  - name: linux-runners
    runnerGroup: linux-runners
    replicas: 1
    limits:
      cpu: 4
      memory: 8Gi
    labels:
      - linux
      - x64
      - actions-runner-controller
    rbac:
      install: true
    # Ephemeral mode: pod torn down after each job
    ephemeral: true
    persistentVolume:
      enabled: false
```

Runner pod receives job from GitHub, executes workflow, exits. Kubernetes re-queues if load increases.

---

## Secrets Masking and Output Handling

Secrets are masked in logs (replaced with `***`) but output context is not safe.

```yaml
steps:
  - name: Dangerous output
    run: echo "Token: ${{ secrets.API_TOKEN }}" # ✓ Masked in logs

  - name: Output to env
    run: echo "MY_SECRET=${{ secrets.API_TOKEN }}" >> $GITHUB_ENV
    # ✗ DO NOT DO THIS; secrets in env leaks via job output

  - name: Set job output
    id: creds
    run: echo "token=${{ secrets.API_TOKEN }}" >> $GITHUB_OUTPUT
    # ✗ Outputs are NOT masked; visible in workflow run UI

  - name: Safe: Use secret directly
    run: |
      curl -H "Authorization: Bearer ${{ secrets.API_TOKEN }}" https://api.example.com
      # ✓ GitHub masks the token in public logs

  - name: Conditionally use secret
    if: ${{ secrets.FEATURE_FLAG == 'true' }}  # ✓ Comparison is safe
    run: echo "Feature enabled"
```

**Safe patterns:**
- Pass secrets as environment variables to commands (token, API keys)
- Use secrets only in step conditions (comparisons are not logged)
- Never set outputs that contain secrets
- Never write secrets to files checked into git
- Use `run: |` with string parameters (not matrix or variables containing secrets)

---

## Expression Language: Functions and Context Access

Expressions evaluate at workflow parse time or runtime; not all contexts available at all times.

```yaml
steps:
  - name: Context access timing
    env:
      # Available at parse time
      REPO: ${{ github.repository }}
      SHA: ${{ github.sha }}
    run: echo "Repo: $REPO, Commit: $SHA"

  - id: check
    run: |
      if [[ ${{ github.event_name }} == "pull_request" ]]; then
        echo "action=test" >> $GITHUB_OUTPUT
      else
        echo "action=deploy" >> $GITHUB_OUTPUT
      fi

  - name: Use step output
    run: echo "Action: ${{ steps.check.outputs.action }}"

  - name: Conditional based on outputs
    if: steps.check.outputs.action == 'deploy'
    run: ./deploy.sh

  - name: Test result check
    if: failure()  # implicit: always() && job.status == 'failure'
    run: echo "Job failed, running cleanup"
```

**Expression truth table:**
- `true`, non-empty string, nonzero number → truthy
- `false`, empty string, `null` → falsy
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=` (numeric or string)
- Logical: `&&`, `||`, `!`
- Functions: `contains()`, `startsWith()`, `endsWith()`, `format()`, `hashFiles()`, `fromJSON()`, `toJSON()`

---

## Cross-Workflow Communication Patterns

### Status Check / Workflow Dispatch Coordination

```yaml
# Central workflow: triggered by external event
name: Central Deployment
on:
  workflow_dispatch:
    inputs:
      version:
        type: string
        required: true

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - run: echo "Deploying version ${{ github.event.inputs.version }}"

# Triggered by another workflow
      - name: Dispatch dependent workflow
        uses: actions/github-script@v7
        with:
          script: |
            await github.rest.actions.createWorkflowDispatch({
              owner: context.repo.owner,
              repo: context.repo.repo,
              workflow_id: 'deploy.yml',
              ref: 'main',
              inputs: {
                version: '${{ github.event.inputs.version }}'
              }
            })

  wait-and-notify:
    needs: deploy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/github-script@v7
        with:
          script: |
            let retries = 30;
            while (retries-- > 0) {
              const runs = await github.rest.actions.listWorkflowRuns({
                owner: context.repo.owner,
                repo: context.repo.repo,
                workflow_id: 'deploy.yml',
                status: 'completed'
              });
              const latestRun = runs.data.workflow_runs[0];
              if (latestRun && latestRun.conclusion === 'success') {
                console.log('Deployment successful');
                break;
              }
              await new Promise(r => setTimeout(r, 10000));
            }
```

---

## Performance Tuning

- **Concurrency groups:** Cancel in-progress runs when new commit pushed (dev workflow)
- **Job timeout:** Set realistic timeout to avoid hanging runs burning credits
- **Runner specs:** Match runner CPU/RAM to workload (large matrix + compile time benefits from +CPU)
- **Cache key precision:** Overly general key defeats cache; overly specific key misses opportunities
- **Artifact size:** Keep artifacts small; upload only what's needed downstream