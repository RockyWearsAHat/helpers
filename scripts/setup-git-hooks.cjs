#!/usr/bin/env node
"use strict";
const fs = require("fs");
const path = require("path");

const hookDir = path.join(__dirname, "..", ".git", "hooks");
const preCommitHook = path.join(hookDir, "pre-commit");

// Minimal pre-commit hook that runs eslint
const hookContent = `#!/bin/bash
# Pre-commit hook: Run eslint on staged files before commit

files=$(git diff --cached --name-only --diff-filter=ACMR -- '*.js' '*.jsx' '*.ts' '*.tsx' 2>/dev/null)
if [[ -n "$files" ]]; then
  npx eslint --fix $files 2>/dev/null
  eslint_exit=$?
  git add $files 2>/dev/null
  if [[ $eslint_exit -ne 0 ]]; then
    exit $eslint_exit
  fi
fi
exit 0
`;

try {
  if (!fs.existsSync(hookDir)) {
    console.log("[git-hooks] .git/hooks directory not found — skipping (not in git repo)");
    process.exit(0);
  }

  fs.writeFileSync(preCommitHook, hookContent, { mode: 0o755 });
  console.log("[git-hooks] pre-commit hook installed");
} catch (e) {
  console.warn(`[git-hooks] could not install hook: ${e.message}`);
}
