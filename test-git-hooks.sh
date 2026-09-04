#!/bin/bash
# Test: Verify git hooks are installed and linting runs on commit

echo "Test 1: Check if pre-commit hook exists"
if [ ! -f .git/hooks/pre-commit ]; then
  echo "FAIL: .git/hooks/pre-commit does not exist"
  exit 1
fi
echo "PASS: pre-commit hook exists"

echo "Test 2: Check if pre-commit hook is executable"
if [ ! -x .git/hooks/pre-commit ]; then
  echo "FAIL: pre-commit hook is not executable"
  exit 1
fi
echo "PASS: pre-commit hook is executable"

echo "Test 3: Check if pre-commit hook runs eslint"
if ! grep -q "eslint" .git/hooks/pre-commit; then
  echo "FAIL: eslint not found in pre-commit hook"
  exit 1
fi
echo "PASS: eslint found in pre-commit hook"

echo ""
echo "All tests passed!"
