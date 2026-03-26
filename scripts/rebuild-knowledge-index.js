#!/usr/bin/env node
"use strict";

/**
 * Standalone TF-IDF index builder for knowledge/*.md files.
 * Produces knowledge/_index.json — the same format as the MCP server's
 * build_knowledge_index tool, extracted so CI can run it without MCP.
 *
 * Usage:  node scripts/rebuild-knowledge-index.js [knowledge-dir]
 *         Defaults to knowledge/ at the repo root.
 */

const fs = require("fs");
const fsp = require("fs/promises");
const path = require("path");

const KNOWLEDGE_ROOT = (() => {
  const explicit = process.argv[2];
  if (explicit) return path.resolve(explicit);
  const repoRoot = path.resolve(__dirname, "..");
  const atRoot = path.join(repoRoot, "knowledge");
  const atGithub = path.join(repoRoot, ".github", "knowledge");
  try {
    if (fs.readdirSync(atRoot).some((f) => f.endsWith(".md"))) return atRoot;
  } catch {
    /* not found */
  }
  return atGithub;
})();

const INDEX_PATH = path.join(KNOWLEDGE_ROOT, "_index.json");

// ─── Stopwords (same set as git-research-mcp) ──────────────────────────────
const STOPWORDS = new Set([
  "a","about","above","after","again","against","all","am","an","and","any",
  "are","aren","as","at","be","because","been","before","being","below",
  "between","both","but","by","can","could","did","do","does","doing","don",
  "down","during","each","few","for","from","further","get","got","had","has",
  "have","having","he","her","here","hers","herself","him","himself","his",
  "how","if","in","into","is","isn","it","its","itself","just","let","like",
  "ll","may","me","might","more","most","must","my","myself","need","no",
  "nor","not","now","of","off","on","once","only","or","other","our","ours",
  "ourselves","out","over","own","re","same","shall","she","should","so",
  "some","such","than","that","the","their","theirs","them","themselves",
  "then","there","these","they","this","those","through","to","too","under",
  "until","up","us","use","used","using","ve","very","was","we","were","what",
  "when","where","which","while","who","whom","why","will","with","would",
  "you","your",
]);

// ─── Tokenizer ──────────────────────────────────────────────────────────────
function tokenize(text) {
  const cleaned = text
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/`[^`\n]+`/g, " ")
    .replace(/https?:\/\/\S+/g, " ")
    .replace(/[#*_\[\]()>|~^=+]/g, " ")
    .replace(/\b\d+\b/g, " ")
    .toLowerCase();
  return cleaned
    .split(/[^a-z]+/)
    .filter((t) => t.length >= 3 && !STOPWORDS.has(t));
}

function getTitle(text, fallback) {
  const m = text.match(/^#\s+(.+)/m);
  return m ? m[1].replace(/[*_`]/g, "").trim() : fallback;
}

// ─── TF-IDF math ────────────────────────────────────────────────────────────
function computeTF(tokens) {
  const freq = {};
  for (const t of tokens) freq[t] = (freq[t] || 0) + 1;
  const total = tokens.length || 1;
  const tf = {};
  for (const [term, count] of Object.entries(freq)) tf[term] = count / total;
  return tf;
}

function computeIDF(allTFs, docCount) {
  const df = {};
  for (const tf of allTFs)
    for (const term of Object.keys(tf)) df[term] = (df[term] || 0) + 1;
  const idf = {};
  for (const [term, count] of Object.entries(df))
    idf[term] = Math.log((docCount + 1) / (count + 1)) + 1;
  return idf;
}

function l2Normalize(vec) {
  const mag = Math.sqrt(Object.values(vec).reduce((s, v) => s + v * v, 0));
  if (mag === 0) return {};
  const out = {};
  for (const [t, v] of Object.entries(vec)) out[t] = v / mag;
  return out;
}

function cosineSim(a, b) {
  let sum = 0;
  const [smaller, larger] =
    Object.keys(a).length <= Object.keys(b).length ? [a, b] : [b, a];
  for (const [t, v] of Object.entries(smaller))
    if (larger[t] !== undefined) sum += v * larger[t];
  return sum;
}

// ─── Main ───────────────────────────────────────────────────────────────────
async function build() {
  const entries = await fsp.readdir(KNOWLEDGE_ROOT);
  const mdFiles = entries.filter(
    (f) => f.endsWith(".md") && !f.startsWith("_"),
  );
  if (!mdFiles.length) {
    console.error("No markdown files found in", KNOWLEDGE_ROOT);
    process.exit(1);
  }

  console.log(`Indexing ${mdFiles.length} files in ${KNOWLEDGE_ROOT}`);

  // Phase 1: read + tokenize
  const docs = [];
  for (const filename of mdFiles) {
    const text = await fsp.readFile(path.join(KNOWLEDGE_ROOT, filename), "utf8");
    const title = getTitle(text, path.basename(filename, ".md"));
    docs.push({ filename, title, tf: computeTF(tokenize(text)) });
  }

  // Phase 2: corpus IDF
  const idf = computeIDF(docs.map((d) => d.tf), docs.length);

  // Phase 3: TF-IDF vectors + posting lists
  const fileData = {};
  const posting = {};
  for (const doc of docs) {
    const tfidf = {};
    for (const [term, tfVal] of Object.entries(doc.tf))
      if (idf[term]) tfidf[term] = tfVal * idf[term];

    const sorted = Object.entries(tfidf).sort((a, b) => b[1] - a[1]);
    const topTerms = sorted.slice(0, 15).map(([t]) => t);
    const sparseRaw = {};
    for (const [term, val] of sorted.slice(0, 120)) sparseRaw[term] = val;
    const normVec = l2Normalize(sparseRaw);

    for (const term of Object.keys(normVec)) {
      if (!posting[term]) posting[term] = [];
      posting[term].push(doc.filename);
    }

    fileData[doc.filename] = {
      title: doc.title,
      top_terms: topTerms,
      norm_vec: normVec,
      related: [],
    };
  }

  // Phase 4: cosine similarity for related files
  const names = Object.keys(fileData);
  for (let i = 0; i < names.length; i++) {
    const a = names[i];
    const sims = [];
    for (let j = 0; j < names.length; j++) {
      if (i === j) continue;
      const sim = cosineSim(fileData[a].norm_vec, fileData[names[j]].norm_vec);
      if (sim > 0.03) sims.push({ name: names[j], sim });
    }
    sims.sort((a, b) => b.sim - a.sim);
    fileData[a].related = sims.slice(0, 5).map((s) => s.name);
  }

  const index = {
    version: 1,
    built_at: new Date().toISOString(),
    file_count: docs.length,
    idf,
    files: fileData,
    posting,
  };

  await fsp.writeFile(INDEX_PATH, JSON.stringify(index, null, 2), "utf8");
  console.log(
    `Index built: ${docs.length} files, ${Object.keys(idf).length} terms → ${INDEX_PATH}`,
  );
}

build().catch((err) => {
  console.error("Index build failed:", err.message);
  process.exit(1);
});
