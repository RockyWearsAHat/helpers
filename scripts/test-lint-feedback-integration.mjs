#!/usr/bin/env node
/**
 * Test: lint-feedback-to-errors integration
 *
 * Verifies that:
 * 1. The tool reads lint-feedback.jsonl correctly
 * 2. Groups feedback by (file, line)
 * 3. Generates proper errors.dx format
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, '..');
const feedbackPath = path.join(root, '.helpers', 'lint-feedback.jsonl');
const tmpOutput = path.join(root, 'errors.test.dx');

function test(name, fn) {
  try {
    fn();
    console.log(`✓ ${name}`);
    return true;
  } catch (e) {
    console.error(`✗ ${name}: ${e.message}`);
    return false;
  }
}

let passed = 0;
let failed = 0;

// Test 1: Feedback file exists
if (test('Feedback file exists', () => {
  if (!fs.existsSync(feedbackPath)) {
    throw new Error(`File not found: ${feedbackPath}`);
  }
})) passed++; else failed++;

// Test 2: Feedback file is valid JSONL
if (test('Feedback file is valid JSONL', () => {
  const content = fs.readFileSync(feedbackPath, 'utf-8').trim();
  const lines = content.split('\n');
  const testLines = lines.slice(0, 5);
  for (const line of testLines) {
    JSON.parse(line); // Will throw if invalid
  }
})) passed++; else failed++;

// Test 3: Script runs and creates output
if (test('Script generates output file', () => {
  const script = path.join(__dirname, 'lint-feedback-to-errors.mjs');
  execSync(`node "${script}" --output "${tmpOutput}"`, { stdio: 'pipe' });
  if (!fs.existsSync(tmpOutput)) {
    throw new Error('Output file not created');
  }
})) passed++; else failed++;

// Test 4: Output has correct format
if (test('Output has correct header', () => {
  const content = fs.readFileSync(tmpOutput, 'utf-8');
  const hasHeader = content.includes('~ dx1 errors-queue');
  const hasTool = content.includes('~ tool: lint');
  const hasCoverage = content.includes('~ coverage:') && content.includes('violations');
  if (!hasHeader || !hasTool || !hasCoverage) {
    throw new Error('Missing required headers');
  }
})) passed++; else failed++;

// Test 5: Output has file sections
if (test('Output has file sections', () => {
  const content = fs.readFileSync(tmpOutput, 'utf-8');
  const sections = (content.match(/^## /gm) || []).length;
  if (sections === 0) {
    throw new Error('No file sections found');
  }
})) passed++; else failed++;

// Test 6: Output has checklist items
if (test('Output has checklist items', () => {
  const content = fs.readFileSync(tmpOutput, 'utf-8');
  const items = content.match(/^- \[ \] .+:\d+$/gm) || [];
  if (items.length === 0) {
    throw new Error('No checklist items found');
  }
})) passed++; else failed++;

// Test 7: Violation count is consistent
if (test('Violation count is consistent', () => {
  const content = fs.readFileSync(tmpOutput, 'utf-8');
  const match = content.match(/~ coverage: (\d+) violations/);
  if (!match) {
    throw new Error('Could not parse coverage line');
  }
  const expectedCount = parseInt(match[1]);
  const actualCount = (content.match(/^- \[ \] /gm) || []).length;
  if (expectedCount !== actualCount) {
    throw new Error(`Coverage says ${expectedCount} but found ${actualCount} items`);
  }
})) passed++; else failed++;

// Test 8: Feedback records are grouped by (file, line)
if (test('Records are properly grouped', () => {
  const content = fs.readFileSync(feedbackPath, 'utf-8').trim();
  const records = content.split('\n').map(line => JSON.parse(line));
  const unique = new Set(records.map(r => `${r.file}:${r.line}`));
  const output = fs.readFileSync(tmpOutput, 'utf-8');
  const items = output.match(/^- \[ \] .+:\d+$/gm) || [];
  if (unique.size !== items.length) {
    throw new Error(`Expected ${unique.size} unique items, got ${items.length}`);
  }
})) passed++; else failed++;

// Cleanup
if (fs.existsSync(tmpOutput)) {
  fs.unlinkSync(tmpOutput);
}

console.log(`\nSummary: ${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);
