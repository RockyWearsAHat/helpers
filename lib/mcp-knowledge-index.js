"use strict";
// lib/mcp-knowledge-index.js — TF-IDF knowledge indexing and search
const fs = require("fs/promises");
const path = require("path");

module.exports = function createKnowledgeIndex(deps) {
  const {
    KNOWLEDGE_ROOT,
    REPO_KNOWLEDGE_ROOT,
    LOCAL_INDEX_PATH,
    WORKSPACE_ROOT,
    fetchCommunityIndex,
    escapeRegExp,
    tokenizeQuery,
    getMarkdownTitle,
    summarizeInline,
    readKnowledgeFileContent,
    searchKnowledgeCache,
    toPositiveInt,
  } = deps;

  // ─── Knowledge TF-IDF Search Index ──────────────────────────────────────────

  const INDEX_STOPWORDS = new Set([
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

  function buildKnowledgeSnippet(text, terms) {
    const compact = text.replace(/\s+/g, " ").trim();
    if (!compact) {
      return "No extractable text.";
    }

    let firstIndex = -1;
    for (const term of terms) {
      const index = compact.toLowerCase().indexOf(term);
      if (index !== -1 && (firstIndex === -1 || index < firstIndex)) {
        firstIndex = index;
      }
    }

    if (firstIndex === -1) {
      return summarizeInline(compact, 220);
    }

    const start = Math.max(0, firstIndex - 80);
    const end = Math.min(compact.length, firstIndex + 180);
    const prefix = start > 0 ? "..." : "";
    const suffix = end < compact.length ? "..." : "";
    return `${prefix}${compact.slice(start, end).trim()}${suffix}`;
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
      .filter((t) => t.length >= 3 && !INDEX_STOPWORDS.has(t));
  }

  function computeDocTF(tokens) {
    const freq = {};
    for (const t of tokens) freq[t] = (freq[t] || 0) + 1;
    const total = tokens.length || 1;
    const tf = {};
    for (const [term, count] of Object.entries(freq)) tf[term] = count / total;
    return tf;
  }

  function computeCorpusIDF(allTFs, docCount) {
    const df = {};
    for (const tf of allTFs) {
      for (const term of Object.keys(tf)) df[term] = (df[term] || 0) + 1;
    }
    const idf = {};
    for (const [term, count] of Object.entries(df)) {
      // Smoothed IDF: log((N+1)/(df+1)) + 1
      idf[term] = Math.log((docCount + 1) / (count + 1)) + 1;
    }
    return idf;
  }

  function l2NormalizeVec(vec) {
    const magnitude = Math.sqrt(
      Object.values(vec).reduce((s, v) => s + v * v, 0),
    );
    if (magnitude === 0) return {};
    const out = {};
    for (const [t, v] of Object.entries(vec)) out[t] = v / magnitude;
    return out;
  }

  function cosineSim(normVecA, normVecB) {
    // Both pre-normalized — dot product equals cosine similarity.
    let sum = 0;
    const [smaller, larger] =
      Object.keys(normVecA).length <= Object.keys(normVecB).length
        ? [normVecA, normVecB]
        : [normVecB, normVecA];
    for (const [term, val] of Object.entries(smaller)) {
      if (larger[term] !== undefined) sum += val * larger[term];
    }
    return sum;
  }

  async function buildKnowledgeIndex(_args) {
    // Build a TF-IDF index for the LOCAL workspace's .github/knowledge/ files.
    let entries;
    try {
      entries = await fs.readdir(KNOWLEDGE_ROOT);
    } catch (err) {
      throw new Error(
        `Cannot read workspace knowledge directory (${KNOWLEDGE_ROOT}): ${err.message}`,
      );
    }
    const mdFiles = entries.filter(
      (f) => f.endsWith(".md") && !f.startsWith("_"),
    );
    if (!mdFiles.length)
      throw new Error(
        "No markdown files found in workspace knowledge directory.",
      );

    // Phase 1: read files and compute per-doc term frequency.
    const docs = [];
    for (const filename of mdFiles) {
      const fp = path.join(KNOWLEDGE_ROOT, filename);
      const text = await fs.readFile(fp, "utf8");
      const title = getMarkdownTitle(text, path.basename(filename, ".md"));
      const tokens = tokenizeDocText(text);
      docs.push({ filename, title, tf: computeDocTF(tokens) });
    }

    // Phase 2: corpus IDF.
    const idf = computeCorpusIDF(
      docs.map((d) => d.tf),
      docs.length,
    );

    // Phase 3: TF-IDF vectors, posting list.
    const fileData = {};
    const posting = {};
    for (const doc of docs) {
      const tfidf = {};
      for (const [term, tfVal] of Object.entries(doc.tf)) {
        if (idf[term]) tfidf[term] = tfVal * idf[term];
      }
      // Keep top 120 terms per doc to keep index compact.
      const sortedTerms = Object.entries(tfidf).sort((a, b) => b[1] - a[1]);
      const topTerms = sortedTerms.slice(0, 15).map(([t]) => t);
      const sparseRaw = {};
      for (const [term, val] of sortedTerms.slice(0, 120))
        sparseRaw[term] = val;
      const normVec = l2NormalizeVec(sparseRaw);

      for (const term of Object.keys(normVec)) {
        if (!posting[term]) posting[term] = [];
        posting[term].push(doc.filename);
      }

      fileData[doc.filename] = {
        title: doc.title,
        top_terms: topTerms,
        norm_vec: normVec,
        related: [], // filled in next pass
      };
    }

    // Phase 4: precompute cosine similarity between all doc pairs (O(N²)).
    const fileNames = Object.keys(fileData);
    for (let i = 0; i < fileNames.length; i++) {
      const nameA = fileNames[i];
      const vecA = fileData[nameA].norm_vec;
      const sims = [];
      for (let j = 0; j < fileNames.length; j++) {
        if (i === j) continue;
        const nameB = fileNames[j];
        const sim = cosineSim(vecA, fileData[nameB].norm_vec);
        if (sim > 0.03) sims.push({ name: nameB, sim });
      }
      sims.sort((a, b) => b.sim - a.sim);
      fileData[nameA].related = sims.slice(0, 5).map((s) => s.name);
    }

    const index = {
      version: 1,
      built_at: new Date().toISOString(),
      file_count: docs.length,
      idf,
      files: fileData,
      posting,
    };

    await fs.writeFile(
      LOCAL_INDEX_PATH,
      JSON.stringify(index, null, 2),
      "utf8",
    );
    return {
      action: "built",
      path: path.relative(WORKSPACE_ROOT, LOCAL_INDEX_PATH),
      file_count: docs.length,
      term_count: Object.keys(idf).length,
    };
  }

  async function searchKnowledgeIndex(args) {
    const query = String(args.query || "").trim();
    if (!query)
      throw new Error("search_knowledge_index requires a non-empty query.");
    const maxResults = toPositiveInt(args.max_results, 5, 1, 20);

    // Load both indexes: local workspace + community (GitHub-hosted).
    // Either or both may be absent — that's fine.
    let localIndex = null;
    let communityIndex = null;

    try {
      const raw = await fs.readFile(LOCAL_INDEX_PATH, "utf8");
      localIndex = JSON.parse(raw);
    } catch {
      /* no local index — OK */
    }

    try {
      communityIndex = await fetchCommunityIndex();
    } catch {
      /* community unavailable — OK */
    }

    // If neither index is available, fall back to keyword search.
    if (!localIndex && !communityIndex) return searchKnowledgeCache(args);

    // Tokenize query: use both the aggressive doc tokenizer and a lighter pass
    // so short or hyphenated terms still match.
    const processedTerms = tokenizeDocText(query);
    const rawTerms = query
      .toLowerCase()
      .split(/[^a-z]+/)
      .filter((t) => t.length >= 2 && !INDEX_STOPWORDS.has(t));
    const queryTerms = [...new Set([...processedTerms, ...rawTerms])];

    if (!queryTerms.length) return searchKnowledgeCache(args);

    // Collect candidates from both indexes, tagging each with its source.
    // "local" = filename from the local workspace index
    // "community" = filename from the GitHub community index
    // A file present in both gets the local version (user override).
    const candidateSource = new Map(); // filename -> "local" | "community"
    const candidateIndex = new Map(); // filename -> index it came from

    function gatherCandidates(index, source) {
      if (!index || !index.posting) return;
      const indexTerms = Object.keys(index.posting);
      for (const qTerm of queryTerms) {
        if (index.posting[qTerm]) {
          for (const f of index.posting[qTerm]) {
            if (!candidateSource.has(f) || source === "local") {
              candidateSource.set(f, source);
              candidateIndex.set(f, index);
            }
          }
        }
        if (qTerm.length >= 3) {
          for (const iTerm of indexTerms) {
            if (
              iTerm !== qTerm &&
              (iTerm.startsWith(qTerm) ||
                (qTerm.length >= 5 && iTerm.includes(qTerm)))
            ) {
              if (index.posting[iTerm]) {
                for (const f of index.posting[iTerm]) {
                  if (!candidateSource.has(f) || source === "local") {
                    candidateSource.set(f, source);
                    candidateIndex.set(f, index);
                  }
                }
              }
            }
          }
        }
      }
    }

    // Local first so it wins on overlap.
    gatherCandidates(localIndex, "local");
    gatherCandidates(communityIndex, "community");

    // Broaden if nothing found yet.
    if (!candidateSource.size) {
      for (const idx of [localIndex, communityIndex]) {
        if (!idx || !idx.posting) continue;
        const src = idx === localIndex ? "local" : "community";
        for (const qTerm of queryTerms) {
          for (const iTerm of Object.keys(idx.posting)) {
            if (iTerm.includes(qTerm)) {
              for (const f of idx.posting[iTerm]) {
                if (!candidateSource.has(f) || src === "local") {
                  candidateSource.set(f, src);
                  candidateIndex.set(f, idx);
                }
              }
            }
          }
        }
      }
    }

    if (!candidateSource.size) {
      return {
        query,
        index_used: true,
        sources: {
          local: !!localIndex,
          community: !!communityIndex,
        },
        total_results: "0",
        results: [],
      };
    }

    // Score candidates: sum of normalized TF-IDF weights for query terms.
    // Local results get a 15% boost — tailors results toward the workspace's own
    // knowledge while still surfacing highly-relevant community articles.
    const LOCAL_BOOST = 1.15;
    const scored = [];
    for (const [filename, source] of candidateSource) {
      const idx = candidateIndex.get(filename);
      const fileInfo = idx.files[filename];
      if (!fileInfo) continue;
      const { norm_vec: vec, title } = fileInfo;
      let score = 0;
      for (const term of queryTerms) {
        if (vec[term] !== undefined) score += vec[term];
        if (title.toLowerCase().includes(term)) score += 0.4;
      }
      if (source === "local") score *= LOCAL_BOOST;
      if (score > 0) scored.push({ filename, score, fileInfo, source });
    }
    // Sort by score desc; at equal scores, local wins; then alphabetical.
    scored.sort(
      (a, b) =>
        b.score - a.score ||
        (a.source === "local" ? -1 : 0) - (b.source === "local" ? -1 : 0) ||
        a.filename.localeCompare(b.filename),
    );

    const topScored = scored.slice(0, maxResults);
    const results = await Promise.all(
      topScored.map(async ({ filename, score, fileInfo, source }) => {
        let snippet = "";
        try {
          const body = await readKnowledgeFileContent(filename);
          snippet = buildKnowledgeSnippet(body, queryTerms);
        } catch {
          /* skip */
        }
        return {
          path: `knowledge/${filename}`,
          title: fileInfo.title,
          score: Math.round(score * 1000) / 1000,
          top_terms: fileInfo.top_terms.slice(0, 8),
          related: fileInfo.related,
          source,
          snippet,
        };
      }),
    );

    const builtAt = localIndex
      ? localIndex.built_at
      : communityIndex
        ? communityIndex.built_at
        : null;

    return {
      query,
      index_used: true,
      sources: {
        local: !!localIndex,
        community: !!communityIndex,
      },
      built_at: builtAt,
      total_results: String(scored.length),
      results,
    };
  }

  function formatKnowledgeIndexSearchResult(result) {
    const src = result.sources || {};
    const srcLabel = [src.local && "local", src.community && "community"]
      .filter(Boolean)
      .join(" + ");
    const lines = [
      `Query: ${result.query}`,
      `Index: ${result.built_at || "no index"} · ${result.total_results} candidates · sources: ${srcLabel || "none"}`,
      "",
      "Results:",
    ];
    for (let i = 0; i < result.results.length; i++) {
      const r = result.results[i];
      lines.push(`${i + 1}. ${r.title}`);
      lines.push(`   Path: ${r.path}`);
      lines.push(
        `   Score: ${r.score}  |  Source: ${r.source || "unknown"}  |  Terms: ${r.top_terms.join(", ")}`,
      );
      if (r.related && r.related.length) {
        lines.push(`   Related: ${r.related.join(", ")}`);
      }
      if (r.snippet) lines.push(`   Snippet: ${r.snippet}`);
      lines.push("");
    }
    if (!result.results.length) {
      lines.push("No matching knowledge notes found.");
    }
    return lines.join("\n");
  }

  function formatBuildIndexResult(result) {
    return [
      `Action: ${result.action}`,
      `Files indexed: ${result.file_count}`,
      `Unique terms: ${result.term_count}`,
      `Index path: ${result.path}`,
    ].join("\n");
  }

  function scoreKnowledgeMatch(relativePath, title, body, terms) {
    const pathText = relativePath.toLowerCase();
    const titleText = title.toLowerCase();
    const bodyText = body.toLowerCase();
    let score = 0;

    for (const term of terms) {
      const regex = new RegExp(escapeRegExp(term), "g");
      score += (titleText.match(regex) || []).length * 10;
      score += (pathText.match(regex) || []).length * 6;
      score += (bodyText.match(regex) || []).length * 2;
    }

    return score;
  }

  return {
    collectMarkdownFiles,
    buildKnowledgeIndex,
    searchKnowledgeIndex,
    formatKnowledgeIndexSearchResult,
    formatBuildIndexResult,
    scoreKnowledgeMatch,
    buildKnowledgeSnippet,
    tokenizeDocText,
    computeDocTF,
    computeCorpusIDF,
  };
};
