#!/usr/bin/env node
"use strict";

const fs = require("fs/promises");
const path = require("path");

const STOPWORDS = new Set([
  "a",
  "about",
  "above",
  "after",
  "again",
  "all",
  "also",
  "am",
  "an",
  "and",
  "any",
  "are",
  "as",
  "at",
  "be",
  "because",
  "been",
  "before",
  "being",
  "between",
  "both",
  "but",
  "by",
  "can",
  "could",
  "did",
  "do",
  "does",
  "doing",
  "down",
  "during",
  "each",
  "few",
  "for",
  "from",
  "further",
  "get",
  "got",
  "had",
  "has",
  "have",
  "having",
  "he",
  "her",
  "here",
  "him",
  "his",
  "how",
  "if",
  "in",
  "into",
  "is",
  "it",
  "its",
  "itself",
  "just",
  "let",
  "me",
  "more",
  "most",
  "must",
  "my",
  "new",
  "no",
  "nor",
  "not",
  "now",
  "of",
  "off",
  "on",
  "once",
  "only",
  "or",
  "other",
  "our",
  "out",
  "over",
  "own",
  "same",
  "she",
  "should",
  "so",
  "some",
  "such",
  "than",
  "that",
  "the",
  "their",
  "them",
  "then",
  "there",
  "these",
  "they",
  "this",
  "those",
  "through",
  "to",
  "too",
  "under",
  "until",
  "up",
  "use",
  "used",
  "using",
  "very",
  "via",
  "was",
  "we",
  "were",
  "what",
  "when",
  "where",
  "which",
  "while",
  "who",
  "will",
  "with",
  "would",
  "you",
  "your",
]);

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("--")) {
      throw new Error(`Unexpected argument: ${arg}`);
    }
    const key = arg.slice(2);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`Missing value for --${key}`);
    }
    args[key] = value;
    index += 1;
  }
  return args;
}

async function collectMarkdownFiles(rootDir) {
  const files = [];

  async function walk(currentDir) {
    const entries = await fs.readdir(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        await walk(fullPath);
        continue;
      }
      if (entry.isFile() && entry.name.endsWith(".md")) {
        files.push(fullPath);
      }
    }
  }

  await walk(rootDir);
  return files;
}

function getMarkdownTitle(text, fallbackTitle) {
  const match = text.match(/^#\s+(.+)$/m);
  return match ? match[1].trim() : fallbackTitle;
}

function tokenizeDocText(text) {
  const cleaned = text
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/`[^`\n]+`/g, " ")
    .replace(/https?:\/\/\S+/g, " ")
    .replace(/[#*_\[\]()>|~^=+]/g, " ")
    .replace(/\b\d+\b/g, " ")
    .toLowerCase();

  return cleaned
    .split(/[^a-z]+/)
    .filter((token) => token.length >= 3 && !STOPWORDS.has(token));
}

function computeDocTF(tokens) {
  const freq = {};
  for (const token of tokens) {
    freq[token] = (freq[token] || 0) + 1;
  }
  const total = tokens.length || 1;
  const tf = {};
  for (const [term, count] of Object.entries(freq)) {
    tf[term] = count / total;
  }
  return tf;
}

function computeCorpusIDF(allTFs, docCount) {
  const df = {};
  for (const tf of allTFs) {
    for (const term of Object.keys(tf)) {
      df[term] = (df[term] || 0) + 1;
    }
  }
  const idf = {};
  for (const [term, count] of Object.entries(df)) {
    idf[term] = Math.log((docCount + 1) / (count + 1)) + 1;
  }
  return idf;
}

function l2NormalizeVec(vec) {
  const magnitude = Math.sqrt(
    Object.values(vec).reduce((sum, value) => sum + value * value, 0),
  );
  if (magnitude === 0) {
    return {};
  }
  const normalized = {};
  for (const [term, value] of Object.entries(vec)) {
    normalized[term] = value / magnitude;
  }
  return normalized;
}

function cosineSim(left, right) {
  let sum = 0;
  const [smaller, larger] =
    Object.keys(left).length <= Object.keys(right).length
      ? [left, right]
      : [right, left];

  for (const [term, value] of Object.entries(smaller)) {
    if (larger[term] !== undefined) {
      sum += value * larger[term];
    }
  }

  return sum;
}

async function buildKnowledgeIndex(workspaceRoot, knowledgeRoot, indexPath) {
  const markdownFiles = await collectMarkdownFiles(knowledgeRoot);
  if (!markdownFiles.length) {
    throw new Error(
      `No markdown files found in knowledge directory: ${knowledgeRoot}`,
    );
  }

  const docs = [];
  for (const filePath of markdownFiles) {
    const text = await fs.readFile(filePath, "utf8");
    const filename = path.relative(knowledgeRoot, filePath).replace(/\\/g, "/");
    const title = getMarkdownTitle(text, path.basename(filename, ".md"));
    const tokens = tokenizeDocText(text);
    docs.push({ filename, title, tf: computeDocTF(tokens) });
  }

  const idf = computeCorpusIDF(
    docs.map((doc) => doc.tf),
    docs.length,
  );

  const fileData = {};
  const posting = {};
  for (const doc of docs) {
    const tfidf = {};
    for (const [term, tfValue] of Object.entries(doc.tf)) {
      if (idf[term]) {
        tfidf[term] = tfValue * idf[term];
      }
    }

    const sortedTerms = Object.entries(tfidf).sort(
      (left, right) => right[1] - left[1],
    );
    const topTerms = sortedTerms.slice(0, 15).map(([term]) => term);
    const sparse = {};
    for (const [term, value] of sortedTerms.slice(0, 120)) {
      sparse[term] = value;
    }
    const normVec = l2NormalizeVec(sparse);

    for (const term of Object.keys(normVec)) {
      if (!posting[term]) {
        posting[term] = [];
      }
      posting[term].push(doc.filename);
    }

    fileData[doc.filename] = {
      title: doc.title,
      top_terms: topTerms,
      norm_vec: normVec,
      related: [],
    };
  }

  const fileNames = Object.keys(fileData);
  for (let leftIndex = 0; leftIndex < fileNames.length; leftIndex += 1) {
    const nameA = fileNames[leftIndex];
    const sims = [];
    for (let rightIndex = 0; rightIndex < fileNames.length; rightIndex += 1) {
      if (leftIndex === rightIndex) {
        continue;
      }
      const nameB = fileNames[rightIndex];
      const sim = cosineSim(fileData[nameA].norm_vec, fileData[nameB].norm_vec);
      if (sim > 0.03) {
        sims.push({ name: nameB, sim });
      }
    }
    sims.sort((left, right) => right.sim - left.sim);
    fileData[nameA].related = sims.slice(0, 5).map((entry) => entry.name);
  }

  const index = {
    version: 1,
    built_at: new Date().toISOString(),
    file_count: docs.length,
    idf,
    files: fileData,
    posting,
  };

  await fs.mkdir(path.dirname(indexPath), { recursive: true });
  await fs.writeFile(indexPath, `${JSON.stringify(index, null, 2)}\n`, "utf8");

  return {
    action: "built",
    path: path.relative(workspaceRoot, indexPath).replace(/\\/g, "/"),
    file_count: docs.length,
    term_count: Object.keys(idf).length,
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const workspaceRoot = path.resolve(args["workspace-root"] || process.cwd());
  const knowledgeRoot = path.resolve(
    args["knowledge-root"] || path.join(workspaceRoot, "knowledge"),
  );
  const indexPath = path.resolve(
    args["index-path"] || path.join(knowledgeRoot, "_index.json"),
  );

  const result = await buildKnowledgeIndex(workspaceRoot, knowledgeRoot, indexPath);
  process.stdout.write(`${JSON.stringify(result)}\n`);
}

main().catch((error) => {
  process.stderr.write(`[build-knowledge-index] ${error.message}\n`);
  process.exit(1);
});