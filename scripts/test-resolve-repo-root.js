#!/usr/bin/env node

// scripts/test-resolve-repo-root.js
// Tests that resolveRepoRoot correctly finds a git repo even when
// process.cwd() is NOT inside one (the bug this fixes).

"use strict";

const os = require("os");
const path = require("path");
const { findRepoRoot, resolveRepoRoot } = require("../lib/mcp-git");

function normalizeRepoPath(value) {
  let normalized = String(value || "").trim();
  const msysMatch = normalized.match(/^\/([A-Za-z])\/(.*)$/);
  if (msysMatch) {
    normalized = `${msysMatch[1]}:/${msysMatch[2]}`;
  }
  normalized = normalized.replace(/\\/g, "/");
  if (/^[A-Z]:/.test(normalized)) {
    normalized = `${normalized[0].toLowerCase()}${normalized.slice(1)}`;
  }
  return normalized.replace(/\/+$/, "");
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

let passed = 0;
let failed = 0;

function assert(condition, label) {
  if (condition) {
    passed++;
    process.stderr.write(`  PASS: ${label}\n`);
  } else {
    failed++;
    process.stderr.write(`  FAIL: ${label}\n`);
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

async function main() {
  const repoRoot = normalizeRepoPath(path.resolve(__dirname, ".."));
  const badDir = os.tmpdir();
  const nestedDir = path.join(repoRoot, "scripts");

  assert(
    normalizeRepoPath(findRepoRoot(nestedDir)) === repoRoot,
    "findRepoRoot walks up from a nested directory",
  );
  assert(findRepoRoot(badDir) === "", "findRepoRoot returns empty outside a repo");

  // 1. With GSH_WORKSPACE_ROOTS pointing to the repo — should resolve.
  process.env.GSH_WORKSPACE_ROOTS = JSON.stringify([repoRoot]);
  {
    const result = await resolveRepoRoot();
    assert(
      normalizeRepoPath(result) === repoRoot,
      "GSH_WORKSPACE_ROOTS set to repo root",
    );
  }

  // 2. With GSH_WORKSPACE_ROOTS pointing to a non-repo dir, but __dirname
  //    is inside the repo — should fall through and succeed via __dirname.
  process.env.GSH_WORKSPACE_ROOTS = JSON.stringify([badDir]);
  {
    // __dirname is scripts/, which IS inside the repo
    const result = await resolveRepoRoot();
    assert(
      normalizeRepoPath(result) === repoRoot,
      "Bad GSH_WORKSPACE_ROOTS, falls through to __dirname",
    );
  }

  // 3. With GSH_WORKSPACE_ROOTS unset and cwd outside any repo.
  //    __dirname (scripts/) should still resolve.
  delete process.env.GSH_WORKSPACE_ROOTS;
  const origCwd = process.cwd;
  process.cwd = () => badDir;
  {
    const result = await resolveRepoRoot();
    assert(
      normalizeRepoPath(result) === repoRoot,
      "No env, bad cwd, falls through to __dirname",
    );
  }
  process.cwd = origCwd;

  // 4. With GSH_WORKSPACE_ROOTS malformed JSON — should not throw,
  //    should fall through gracefully.
  process.env.GSH_WORKSPACE_ROOTS = "not-json";
  {
    const result = await resolveRepoRoot();
    assert(
      normalizeRepoPath(result) === repoRoot,
      "Malformed GSH_WORKSPACE_ROOTS, graceful fallback",
    );
  }
  delete process.env.GSH_WORKSPACE_ROOTS;

  // 5. Multiple roots, first is bad, second is good.
  process.env.GSH_WORKSPACE_ROOTS = JSON.stringify([badDir, repoRoot]);
  {
    const result = await resolveRepoRoot();
    assert(
      normalizeRepoPath(result) === repoRoot,
      "Multiple roots, first bad, second good",
    );
  }
  delete process.env.GSH_WORKSPACE_ROOTS;

  // 6. All candidates invalid — should throw.
  process.env.GSH_WORKSPACE_ROOTS = JSON.stringify([badDir]);
  process.cwd = () => badDir;
  {
    // We can't override __dirname easily, but we CAN set GSH_WORKSPACE_ROOTS
    // to only bad paths and a cwd that's also bad. __dirname is still in the
    // repo, so this test verifies the fallback chain ends at __dirname.
    // To truly test the throw, we'd need to run in a subprocess.
    // Instead, verify that with bad env + bad cwd, __dirname saves us.
    const result = await resolveRepoRoot();
    assert(
      normalizeRepoPath(result) === repoRoot,
      "Bad env + bad cwd, __dirname saves the day",
    );
  }
  process.cwd = origCwd;
  delete process.env.GSH_WORKSPACE_ROOTS;

  // Summary
  process.stderr.write(
    `\nresolveRepoRoot: ${passed} passed, ${failed} failed\n`,
  );
  if (failed > 0) {
    process.stdout.write(`TEST_SUMMARY: fail ${failed}/${passed + failed}\n`);
    process.exit(1);
  }
  process.stdout.write(`TEST_SUMMARY: pass ${passed}/${passed + failed}\n`);
}

main().catch((err) => {
  process.stderr.write(`Unexpected error: ${err.message}\n${err.stack}\n`);
  process.stdout.write("TEST_SUMMARY: fail 1/1\n");
  process.exit(1);
});
