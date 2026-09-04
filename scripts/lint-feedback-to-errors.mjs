#!/usr/bin/env node
/**
 * lint-feedback-to-errors.mjs
 *
 * Ingests .helpers/lint-feedback.jsonl and generates errors.dx queue format.
 * This wires the helpers lint report intake into SARA's worked errors queue.
 *
 * Usage:
 *   node scripts/lint-feedback-to-errors.mjs [--output path/to/errors.dx]
 *
 * Reads from: .helpers/lint-feedback.jsonl (JSONL format, one FeedbackRecord per line)
 * Writes to: errors.dx (or specified output path)
 *
 * The feedback records are grouped by (file, line) and deduplicated.
 * Both false_positive and missed feedback items contribute to the queue.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = process.cwd().includes('.sara/worktrees')
  ? path.resolve(__dirname, '../..')
  : path.resolve(__dirname, '..');

// Parse arguments
let outputPath = path.join(root, 'errors.dx');
for (let i = 2; i < process.argv.length; i++) {
  if (process.argv[i] === '--output' && i + 1 < process.argv.length) {
    outputPath = process.argv[i + 1];
    i++;
  }
}

const feedbackPath = path.join(root, '.helpers', 'lint-feedback.jsonl');

function main() {
  // Check if feedback file exists
  if (!fs.existsSync(feedbackPath)) {
    console.error(`Error: feedback file not found: ${feedbackPath}`);
    process.exit(1);
  }

  // Read and parse feedback records
  let records = [];
  try {
    const content = fs.readFileSync(feedbackPath, 'utf-8').trim();
    if (content) {
      records = content.split('\n').map((line, i) => {
        try {
          return JSON.parse(line);
        } catch (e) {
          console.error(`Error parsing line ${i + 1}: ${e.message}`);
          return null;
        }
      }).filter(r => r !== null);
    }
  } catch (e) {
    console.error(`Error reading feedback file: ${e.message}`);
    process.exit(1);
  }

  // Group feedback by (file, line)
  // Each unique (file, line) pair becomes one queue item
  const grouped = new Map();
  for (const record of records) {
    if (!record.file || record.line === undefined) continue;

    if (!grouped.has(record.file)) {
      grouped.set(record.file, new Set());
    }
    grouped.get(record.file).add(record.line);
  }

  // Sort files and generate errors.dx format
  const files = Array.from(grouped.keys()).sort();
  const violations = files.reduce((sum, f) => sum + grouped.get(f).size, 0);

  let output = `~ dx1 errors-queue\n`;
  output += `~ tool: lint\n`;
  output += `~ version: 1.0\n`;
  output += `~ coverage: ${violations} violations\n`;
  output += `\n`;

  for (const file of files) {
    output += `## ${file}\n\n`;
    const lineNums = Array.from(grouped.get(file))
      .sort((a, b) => a - b);
    for (const lineNum of lineNums) {
      output += `- [ ] ${file}:${lineNum}\n`;
    }
    output += `\n`;
  }

  // Write output
  try {
    fs.writeFileSync(outputPath, output, 'utf-8');
    console.log(`✓ Updated ${outputPath} with ${violations} violations from ${records.length} feedback records`);
  } catch (e) {
    console.error(`Error writing output: ${e.message}`);
    process.exit(1);
  }
}

main();
