# Snapshot Testing — Jest Snapshots, Inline Snapshots, and Update Workflows

## Concept: Capturing Expected State

Snapshot testing captures the current output of a function or component and stores it as a reference file. On subsequent test runs, the new output is compared to the snapshot. If they differ, the test fails, prompting the developer to review and approve the change or fix the regression.

Snapshots are useful for components with large, complex outputs — render trees, serialized objects, API response structures — where writing assertion-by-assertion would be tedious and fragile.

## Jest Snapshots: Disk-Based References

### How They Work

Jest snapshots are written to `.snap` files alongside test files. When a snapshot test runs:

1. The function/component executes
2. Output is serialized and compared to the stored snapshot
3. On mismatch, Jest shows a diff and fails the test
4. Developer runs with `-u` flag to update snapshots if the change is intentional

Example:

```javascript
test('renders user card', () => {
  const html = renderUserCard({ name: 'Alice', role: 'Admin' });
  expect(html).toMatchSnapshot();
});
```

The snapshot file stores:
```javascript
exports[`renders user card 1`] = `
<div class="card">
  <h2>Alice</h2>
  <span>Admin</span>
</div>
`;
```

### Update Workflows

The snapshot update workflow is both feature and footgun:

- **Intentional updates**: `jest --updateSnapshot` or `-u` updates all snapshots. Use when you've deliberately changed component behavior.
- **Partial updates**: `jest -t 'test name' -u` updates only matching tests, reducing accident surface.
- **Review before update**: Always inspect diffs before updating. Snapshot updates can hide bugs — a typo in code can pass unexpectedly.

**Risk**: Running `-u` blindly in CI or after refactoring can commit unintended changes. CI should reject snapshot mutations; only developers can approve them locally.

### Serializers: Controlling Snapshot Format

By default, Jest serializes React components to JSX-like strings, plain objects to JSON, etc. Custom serializers let you normalize unstable values:

```javascript
expect.addSnapshotSerializer({
  test: (val) => val instanceof Date,
  print: (val) => `Date(${val.toISOString()})`
});
```

This prevents timestamps or random IDs from breaking snapshots. Common customizations:

- Normalize generated IDs, UUIDs, timestamps
- Strip volatile environment variables
- Format large nested structures for readability
- Hide platform-specific paths

Serializers make snapshots more stable across runs and environments.

## Inline Snapshots: Snapshots in Test Code

Inline snapshots embed the expected output directly in the test function, avoiding a separate `.snap` file:

```javascript
test('calculates discount', () => {
  const price = calculateFinal(100, 0.2);
  expect(price).toMatchInlineSnapshot(`85`);
});
```

When the snapshot changes, Jest shows the diff and offer to update the test file inline. Tooling (VS Code Jest extension) can auto-update inline snapshots.

### Tradeoffs

| Aspect | Snapshots | Inline |
|--------|-----------|--------|
| **Visibility** | Separate file; need to open `.snap` | In test code; visible inline |
| **Readability** | Separate from logic | Mixed with test |
| **Large outputs** | Cleaner (one per file) | Clutters test code |
| **Updates** | Central `.snap` review | File edits scattered |
| **Diff review** | Git shows `.snap` diffs clearly | Git diffs blend with code changes |

Use **inline** for small, focused assertions (2-3 lines). Use **disk-based** for large or numerous snapshots (React render trees, API responses).

## When Snapshots Help

1. **Large, auto-generated structures**: Render trees, serialized protest objects, parse trees. Writing line-by-line assertions is tedious and fragile.
2. **Regression detection**: Snapshots catch unexpected changes in output shape, even if the code path isn't broken.
3. **Truth recording**: Complex algorithms (image compression, layout engines) where the "right" output is hard to specify — snapshot becomes the source of truth.
4. **Refactoring safety**: When refactoring, snapshots confirm the output shape hasn't regressed, even if internal logic changes.

## When Snapshots Hurt

1. **Brittle updates**: Minor formatting changes (whitespace, element order) break snapshots. Developers blindly update with `-u`, missing real bugs.
2. **Poor feedback**: Snapshots show diffs but don't explain *why* the change happened. Large diffs are hard to reason about.
3. **False confidence**: A passing snapshot provides no guarantee of correctness — only that output matches the recorded state. If the original snapshot was wrong, all future tests pass.
4. **Maintenance burden**: As code evolves, snapshot files grow and become hard to review in VCS. Each `.snap` file can represent hundreds of assertions.
5. **Coupling to implementation**: Snapshots couple tests to exact output format. Small refactors that don't change behavior force snapshot updates.

**Better approach for specificity**: Use snapshots for shape/structure; pair with targeted assertions for correctness:

```javascript
test('renders user card', () => {
  const output = renderUserCard(userData);
  expect(output).toMatchSnapshot();  // Catches unexpected shape changes
  
  // Verify key behaviors
  expect(output.find('.name').text()).toBe('Alice');
  expect(output).toContainElement(screen.getByRole('button', { name: /follow/i }));
});
```

## Golden File Testing: Snapshots Beyond Jest

Golden file testing (also called approval testing) is similar to snapshots but typically used in other languages and test frameworks:

- Output is serialized to a file (e.g., `.golden`, `.approved.txt`)
- On test failure, diff is shown; developer approves with a tool or by renaming
- Workflows vary: some auto-update, others require explicit approval

The philosophy is identical to snapshots — capture expected output once, detect regressions via comparison. The mechanics differ by language and tooling.

## Approval Testing: Interactive Approval Workflow

Approval testing formalizes the comparison and approval:

1. Test runs, producing actual output
2. If diff from expected, test fails and shows a diff tool
3. Developer reviews visually (image diff viewer, text viewer)
4. Developer approves the diff if correct, moves the "approved" file over the baseline
5. Test passes on re-run

Tools like ApprovalTests.NET, approval, and pytest-approve implement this. It's stricter than Jest's `-u` because it requires explicit visual review before each update, reducing blind approvals.

Use approval testing for high-stakes outputs (UI rendering, code generation, visual output) where a casual `-u` update is risky.

## Snapshot Maintenance Patterns

### 1. Large Snapshots: Break Them Up

Don't snapshot entire pages or responses. Snapshot components or logical sections independently:

```javascript
// Bad: huge snapshot
test('page renders', () => {
  expect(renderEntirePage()).toMatchSnapshot();
});

// Better: separate concerns
test('header renders', () => expect(renderHeader()).toMatchSnapshot());
test('sidebar renders', () => expect(renderSidebar()).toMatchSnapshot());
test('user card renders', () => expect(renderUserCard()).toMatchSnapshot());
```

Smaller snapshots are easier to review and update.

### 2. Use Serializers for Stability

Strip non-deterministic values (IDs, timestamps, session tokens) via serializers:

```javascript
expect.addSnapshotSerializer({
  test: (val) => typeof val === 'string' && /^[0-9a-f-]{36}$/.test(val),
  print: () => 'UUID(...)'
});
```

Prevents snapshot churn from randomized data.

### 3. Snapshot Diffing in CI

In CI, reject snapshot mutations automatically. Only allow snapshot changes via pull request review:

```javascript
// jest.config.js
module.exports = {
  snapshotFormat: { printBasicPrototype: false },
  // CI should reject -u; snapshot changes must be approved
};
```

### 4. Archive Old Snapshots

Over time, `.snap` files grow. Periodically archive unused tests and clean snapshots:

```bash
jest --findRelatedTests <file> --listTests | xargs grep -l "test(" | xargs wc -l
```

## Alternatives and Complements

- **Property-based testing** (see testing-advanced-patterns): Generate many inputs, verify invariants — doesn't snapshot, but catches subtle bugs.
- **Contract testing** (see testing-contract): Verify API consumers and providers match contracts — similar verification philosophy, but targeted at API compatibility.
- **Visual regression testing** (see testing-visual-regression): Screenshot and pixel-diff UIs — snapshot for images.

## Summary

Snapshots are valuable for large, complex outputs that are tedious to assert line-by-line. Use them deliberately: pair with targeted assertions, maintain serializers for stability, and review diffs carefully before updating. Avoid snapshot bloat and blind updates. Combine snapshots with other testing strategies for comprehensive coverage.