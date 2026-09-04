const { strict: assert } = require('assert');
const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const TEST_DIR = path.dirname(__filename);
const PROJECT_ROOT = path.dirname(TEST_DIR);
const PACKAGE_JSON_PATH = path.join(PROJECT_ROOT, 'package.json');

console.log('TEST: package.json is valid JSON');
const packageJsonContent = fs.readFileSync(PACKAGE_JSON_PATH, 'utf8');
const pkg = JSON.parse(packageJsonContent);
assert(pkg.name, 'package.json must have a name field');
assert(pkg.version, 'package.json must have a version field');
console.log('  ✓ package.json is valid JSON with required fields');

console.log('\nTEST: npm ls --depth=0 has no ERR! lines');
const npmLsOutput = execSync('npm ls --depth=0 2>&1', {
  cwd: PROJECT_ROOT,
  encoding: 'utf8'
});
const hasErrors = npmLsOutput.includes('ERR!');
assert(!hasErrors, `npm ls should not contain ERR! lines. Output:\n${npmLsOutput}`);
console.log('  ✓ npm ls --depth=0 returns valid dependency tree with no errors');

console.log('\nTEST: npm ls output matches expected format');
const lsLines = npmLsOutput.split('\n').filter(l => l.trim());
const depLine = lsLines.find(l => l.match(/^[a-z@]/));
assert(depLine, 'npm ls output should contain at least one dependency line');
console.log('  ✓ npm ls contains properly formatted dependency lines');

console.log('\n✅ All tests passed');
