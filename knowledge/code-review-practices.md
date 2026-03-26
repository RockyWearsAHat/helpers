# Code Review Practices

## For Authors (Submitting Code)

1. **Self-review first.** Read your own diff before requesting review. Catch the obvious issues yourself.
2. **Keep changes small.** 200-400 lines of meaningful code per review. Studies show review quality drops dramatically above 400 lines (Microsoft Research).
3. **One concern per PR.** Don't mix bug fixes, new features, and refactoring in one review.
4. **Write a clear description.** Explain: what changed, why, how to test, any trade-offs or decisions.
5. **Annotate complex changes.** Leave comments on your own PR pointing out non-obvious design decisions.
6. **Include tests.** Changed behavior without tests is an incomplete PR.
7. **Run CI before requesting review.** Don't waste reviewer time on code that doesn't build or pass tests.

## For Reviewers

### What to Focus On

- **Correctness**: Does the code do what it claims? Are edge cases handled?
- **Design**: Does it fit the architecture? Is it in the right place? Right level of abstraction?
- **Readability**: Can you understand it without the PR description? Will someone understand it in 6 months?
- **Security**: Input validation, authentication checks, injection risks, secret handling.
- **Performance**: Obvious inefficiencies (N+1 queries, unnecessary loops, missing indexes).
- **Testing**: Are the right things tested? Are the tests meaningful or just coverage padding?

### What NOT to Focus On

- Style issues that linters can catch. Automate formatting (Prettier, Black, gofmt, rustfmt).
- Personal preference that doesn't affect maintainability.
- Nitpicks on code you didn't need to change for this PR (separate issue).

### How to Give Feedback

**Be kind, specific, and constructive:**

- Bad: "This is wrong."
- Good: "This will throw if `user` is null — the endpoint doesn't require auth. Consider adding a null check or making auth required."

**Use prefixes to signal intent:**

- `nit:` — Minor style suggestion. Take it or leave it.
- `question:` — I don't understand this, please explain.
- `suggestion:` — Consider this alternative approach.
- `blocking:` — This must be addressed before merge.
- `praise:` — Nice solution! Good pattern to acknowledge good work.

**Explain the why, not just the what.** Don't just say "use X instead of Y." Explain why X is better in this context.

**Assume good intent.** The author had reasons for their choices. Ask before assuming they're wrong.

## Review Process

### Two-reviewer model

Two active reviewers is the sweet spot for most teams (Microsoft, Google research). More reviewers means diminishing returns.

### Review turnaround

Aim for < 24 hours to first review. Blocking PRs for days kills flow and morale.

### Approve with comments

If feedback is minor (nits, suggestions), approve and trust the author to address them. Don't block merges for trivial issues.

### Automated checks

Automate everything you can:

- **Linting/formatting**: Pre-commit hooks or CI.
- **Type checking**: TypeScript, mypy, Flow.
- **Security scanning**: Dependabot, Snyk, CodeQL.
- **Test coverage thresholds**: Fail CI if coverage drops.
- **Complexity metrics**: Flag functions/files above thresholds.

This frees human reviewers to focus on design, correctness, and architecture — the things machines can't judge.

## Review Checklist

- [ ] Does the code work? (Correctness)
- [ ] Are there tests for the changed behavior?
- [ ] Is the code readable without the PR description?
- [ ] Are error cases handled?
- [ ] Is there any security concern? (Input validation, auth, injection)
- [ ] Are there any performance red flags? (N+1, unbounded loops)
- [ ] Does it follow project conventions?
- [ ] Is the commit history clean? (Logical commits, not "fix" "fix2" "wip")
- [ ] Does the PR description explain what and why?

---

_Sources: Microsoft Engineering Playbook, Google Engineering Practices, Michaela Greiler (Code Review Best Practices from Microsoft), SmartBear (Cisco study on code review effectiveness)_
