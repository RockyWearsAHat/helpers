"use strict";
// lib/mcp-session-memory.js — Engram-inspired session learning system
//
// Provides per-workspace append-only session logs with TF-IDF indexing and
// surprise-weighted retrieval. Agents log actions and outcomes; the index
// is rebuilt after every write so subsequent searches are always current.
//
// Key concepts adapted from DeepSeek Engram (Jan 2026):
//   - O(1) lookup via TF-IDF posting lists (analogous to N-gram hash tables)
//   - Surprise-weighted scoring (high-surprise entries surface preferentially)
//   - Model-tier gating (same-model matches get boosted relevance)
//   - Post-write auto-rebuild (index always current)

const fs = require("fs/promises");
const fsSync = require("fs");
const path = require("path");

module.exports = function createSessionMemory(deps) {
  const {
    WORKSPACE_ROOT,
    escapeRegExp,
    tokenizeQuery,
    summarizeInline,
    toPositiveInt,
  } = deps;

  // ─── Paths ────────────────────────────────────────────────────────────────

  const SESSION_DIR = path.join(WORKSPACE_ROOT, ".github", "session-memory");
  const LOG_PATH = path.join(SESSION_DIR, "session-log.jsonl");
  const INDEX_PATH = path.join(SESSION_DIR, "_session-index.json");

  // ─── Model tier classification ────────────────────────────────────────────

  const MODEL_TIERS = {
    quick: [
      "haiku",
      "gpt-4o-mini",
      "gpt-4.1-mini",
      "gpt-4.1-nano",
      "gemini-flash",
    ],
    capable: ["sonnet", "gpt-4o", "gpt-4.1", "gpt-5", "gpt-5.2", "gemini-pro"],
    thorough: ["opus", "o3", "o4-mini", "gpt-5-turbo", "gemini-ultra"],
  };

  function classifyModelTier(model) {
    if (!model) return "unknown";
    const lower = model.toLowerCase();
    for (const [tier, patterns] of Object.entries(MODEL_TIERS)) {
      for (const pat of patterns) {
        if (lower.includes(pat)) return tier;
      }
    }
    return "unknown";
  }

  // ─── TF-IDF primitives (lean version of mcp-knowledge-index) ─────────────

  const STOPWORDS = new Set([
    "a",
    "about",
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
    "been",
    "but",
    "by",
    "can",
    "did",
    "do",
    "does",
    "for",
    "from",
    "get",
    "had",
    "has",
    "have",
    "he",
    "her",
    "him",
    "his",
    "how",
    "if",
    "in",
    "into",
    "is",
    "it",
    "its",
    "just",
    "me",
    "more",
    "must",
    "my",
    "no",
    "nor",
    "not",
    "now",
    "of",
    "on",
    "or",
    "our",
    "out",
    "own",
    "she",
    "so",
    "some",
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
    "to",
    "too",
    "up",
    "use",
    "used",
    "using",
    "very",
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

  function tokenizeText(text) {
    return text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, " ")
      .split(/\s+/)
      .filter((t) => t.length >= 3 && !STOPWORDS.has(t));
  }

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
    for (const tf of allTFs) {
      for (const term of Object.keys(tf)) df[term] = (df[term] || 0) + 1;
    }
    const idf = {};
    for (const [term, count] of Object.entries(df)) {
      idf[term] = Math.log((docCount + 1) / (count + 1)) + 1;
    }
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
    for (const [term, val] of Object.entries(smaller)) {
      if (larger[term] !== undefined) sum += val * larger[term];
    }
    return sum;
  }

  // ─── Ensure directory exists ──────────────────────────────────────────────

  async function ensureDir() {
    await fs.mkdir(SESSION_DIR, { recursive: true });
  }

  // ─── Read all log entries ─────────────────────────────────────────────────

  async function readLogEntries() {
    try {
      const raw = await fs.readFile(LOG_PATH, "utf8");
      return raw
        .split("\n")
        .filter((line) => line.trim())
        .map((line, idx) => {
          try {
            return JSON.parse(line);
          } catch {
            return null;
          }
        })
        .filter(Boolean);
    } catch {
      return [];
    }
  }

  // ─── Build index from log entries ─────────────────────────────────────────

  async function buildSessionIndex() {
    const entries = await readLogEntries();
    if (!entries.length) {
      const emptyIndex = {
        version: 1,
        built_at: new Date().toISOString(),
        entry_count: 0,
        idf: {},
        entries: [],
        posting: {},
      };
      await fs.writeFile(
        INDEX_PATH,
        JSON.stringify(emptyIndex, null, 2),
        "utf8",
      );
      return { action: "built", entry_count: 0, term_count: 0 };
    }

    // Tokenize each entry's combined text fields.
    const docs = entries.map((entry, idx) => {
      const text = [
        entry.action || "",
        entry.outcome || "",
        entry.context || "",
        (entry.tags || []).join(" "),
      ].join(" ");
      const tokens = tokenizeText(text);
      return { idx, tf: computeTF(tokens), entry };
    });

    const idf = computeIDF(
      docs.map((d) => d.tf),
      docs.length,
    );

    const indexEntries = [];
    const posting = {};

    for (const doc of docs) {
      const tfidf = {};
      for (const [term, tfVal] of Object.entries(doc.tf)) {
        if (idf[term]) tfidf[term] = tfVal * idf[term];
      }
      // Keep top 60 terms per entry (entries are short).
      const sorted = Object.entries(tfidf).sort((a, b) => b[1] - a[1]);
      const sparseRaw = {};
      for (const [term, val] of sorted.slice(0, 60)) sparseRaw[term] = val;
      const normVec = l2Normalize(sparseRaw);

      for (const term of Object.keys(normVec)) {
        if (!posting[term]) posting[term] = [];
        posting[term].push(doc.idx);
      }

      indexEntries.push({
        ts: doc.entry.ts,
        model: doc.entry.model || null,
        tier: doc.entry.tier || null,
        surprise: doc.entry.surprise || 0,
        tags: doc.entry.tags || [],
        norm_vec: normVec,
      });
    }

    const index = {
      version: 1,
      built_at: new Date().toISOString(),
      entry_count: entries.length,
      idf,
      entries: indexEntries,
      posting,
    };

    await fs.writeFile(INDEX_PATH, JSON.stringify(index, null, 2), "utf8");
    return {
      action: "built",
      entry_count: entries.length,
      term_count: Object.keys(idf).length,
    };
  }

  // ─── Log a session event ──────────────────────────────────────────────────

  async function logSessionEvent(args) {
    const action = String(args.action || "").trim();
    if (!action)
      throw new Error("log_session_event requires a non-empty action.");

    const outcome = String(args.outcome || "").trim();
    const surprise = Math.max(0, Math.min(1, parseFloat(args.surprise) || 0));
    const model = String(args.model || "").trim() || null;
    const tier = model ? classifyModelTier(model) : args.tier || "unknown";
    const context = String(args.context || "").trim() || null;
    const tags = Array.isArray(args.tags)
      ? args.tags.map((t) => String(t).trim()).filter(Boolean)
      : typeof args.tags === "string"
        ? args.tags
            .split(",")
            .map((t) => t.trim())
            .filter(Boolean)
        : [];

    const entry = {
      ts: new Date().toISOString(),
      model,
      tier,
      action,
      outcome,
      surprise,
      tags,
      context,
    };

    await ensureDir();
    await fs.appendFile(LOG_PATH, JSON.stringify(entry) + "\n", "utf8");

    // Auto-rebuild index after every write (Engram principle: index always current).
    await buildSessionIndex();

    return {
      action: "logged",
      entry,
      index_rebuilt: true,
    };
  }

  // ─── Search session log ───────────────────────────────────────────────────

  async function searchSessionLog(args) {
    const query = String(args.query || "").trim();
    if (!query)
      throw new Error("search_session_log requires a non-empty query.");

    const maxResults = toPositiveInt(args.max_results, 5, 1, 20);
    const currentModel = String(args.current_model || "").trim() || null;
    const currentTier = currentModel ? classifyModelTier(currentModel) : null;

    // Load index.
    let index;
    try {
      const raw = await fs.readFile(INDEX_PATH, "utf8");
      index = JSON.parse(raw);
    } catch {
      // No index — try building one.
      await buildSessionIndex();
      try {
        const raw = await fs.readFile(INDEX_PATH, "utf8");
        index = JSON.parse(raw);
      } catch {
        return { results: [], message: "No session memory yet." };
      }
    }

    if (!index.entry_count) {
      return { results: [], message: "Session memory is empty." };
    }

    // Tokenize query.
    const queryTerms = [
      ...new Set([
        ...tokenizeText(query),
        ...query
          .toLowerCase()
          .split(/[^a-z0-9]+/)
          .filter((t) => t.length >= 2 && !STOPWORDS.has(t)),
      ]),
    ];

    if (!queryTerms.length) {
      return { results: [], message: "Query produced no searchable terms." };
    }

    // Build query TF-IDF vector.
    const queryTF = computeTF(queryTerms);
    const queryTFIDF = {};
    for (const [term, tfVal] of Object.entries(queryTF)) {
      if (index.idf[term]) queryTFIDF[term] = tfVal * index.idf[term];
    }
    const queryVec = l2Normalize(queryTFIDF);

    // Collect candidate entries from posting list.
    const candidateIndices = new Set();
    for (const qTerm of queryTerms) {
      // Exact match.
      if (index.posting[qTerm]) {
        for (const idx of index.posting[qTerm]) candidateIndices.add(idx);
      }
      // Prefix match for broader recall.
      for (const pTerm of Object.keys(index.posting)) {
        if (pTerm.startsWith(qTerm) || qTerm.startsWith(pTerm)) {
          for (const idx of index.posting[pTerm]) candidateIndices.add(idx);
        }
      }
    }

    if (!candidateIndices.size) {
      return { results: [], message: "No matching session events found." };
    }

    // Load raw entries for result formatting.
    const rawEntries = await readLogEntries();

    // Score candidates with surprise weighting and model-tier gating.
    const scored = [];
    for (const idx of candidateIndices) {
      const indexEntry = index.entries[idx];
      if (!indexEntry) continue;

      // Base score: cosine similarity.
      let score = cosineSim(queryVec, indexEntry.norm_vec);

      // Surprise weighting (Engram dopamine-learning analog):
      // Entries with surprise > 0.5 get up to 2x boost.
      const surpriseBoost = 1 + (indexEntry.surprise || 0);
      score *= surpriseBoost;

      // Model-tier gating:
      if (currentTier && indexEntry.tier) {
        if (
          currentModel &&
          indexEntry.model &&
          indexEntry.model === currentModel
        ) {
          score *= 1.3; // Exact model match.
        } else if (currentTier === indexEntry.tier) {
          score *= 1.0; // Same tier, neutral.
        } else {
          score *= 0.8; // Different tier, slight penalty.
        }
      }

      // Recency boost: newer entries get a small advantage.
      // Max 10% boost for entries from the last hour, decaying over 7 days.
      if (indexEntry.ts) {
        const ageMs = Date.now() - new Date(indexEntry.ts).getTime();
        const ageDays = ageMs / (1000 * 60 * 60 * 24);
        const recencyBoost = Math.max(0, 1.1 - ageDays * (0.1 / 7));
        score *= recencyBoost;
      }

      scored.push({ idx, score, indexEntry });
    }

    scored.sort((a, b) => b.score - a.score);

    const results = scored.slice(0, maxResults).map((s) => {
      const raw = rawEntries[s.idx] || {};
      return {
        score: Math.round(s.score * 1000) / 1000,
        ts: raw.ts,
        model: raw.model,
        tier: raw.tier,
        action: raw.action,
        outcome: raw.outcome,
        surprise: raw.surprise,
        tags: raw.tags,
        context: raw.context,
      };
    });

    return { results };
  }

  // ─── Get session summary ──────────────────────────────────────────────────

  async function getSessionSummary(args) {
    const limit = toPositiveInt(args.limit, 20, 1, 100);
    const entries = await readLogEntries();

    if (!entries.length) {
      return { summary: "No session events recorded yet.", entries: [] };
    }

    // Return the most recent N entries.
    const recent = entries.slice(-limit);

    // Compute aggregate stats.
    const totalEntries = entries.length;
    const avgSurprise =
      entries.reduce((sum, e) => sum + (e.surprise || 0), 0) / totalEntries;
    const highSurpriseCount = entries.filter(
      (e) => (e.surprise || 0) > 0.5,
    ).length;
    const modelCounts = {};
    for (const e of entries) {
      const m = e.model || "unknown";
      modelCounts[m] = (modelCounts[m] || 0) + 1;
    }
    const tagCounts = {};
    for (const e of entries) {
      for (const t of e.tags || []) {
        tagCounts[t] = (tagCounts[t] || 0) + 1;
      }
    }
    const topTags = Object.entries(tagCounts)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10)
      .map(([tag, count]) => ({ tag, count }));

    const outcomeBreakdown = {};
    for (const e of entries) {
      const o = e.outcome || "unspecified";
      outcomeBreakdown[o] = (outcomeBreakdown[o] || 0) + 1;
    }

    return {
      total_entries: totalEntries,
      avg_surprise: Math.round(avgSurprise * 100) / 100,
      high_surprise_count: highSurpriseCount,
      model_usage: modelCounts,
      top_tags: topTags,
      outcome_breakdown: outcomeBreakdown,
      recent_entries: recent,
    };
  }

  // ─── Format helpers ───────────────────────────────────────────────────────

  function formatLogResult(result) {
    const e = result.entry;
    const lines = [
      `Session event logged (index rebuilt).`,
      "",
      `  Action: ${e.action}`,
      `  Outcome: ${e.outcome || "(none)"}`,
      `  Surprise: ${e.surprise}`,
      `  Model: ${e.model || "unknown"} (${e.tier})`,
      `  Tags: ${(e.tags || []).join(", ") || "(none)"}`,
    ];
    if (e.context) lines.push(`  Context: ${e.context}`);
    return lines.join("\n");
  }

  function formatSearchResults(result) {
    if (result.message) return result.message;
    if (!result.results.length) return "No matching session events found.";

    const lines = [
      `Found ${result.results.length} matching session event(s):`,
      "",
    ];
    for (const r of result.results) {
      lines.push(`--- [score: ${r.score}] ${r.ts || "?"} ---`);
      lines.push(`  Model: ${r.model || "unknown"} (${r.tier || "?"})`);
      lines.push(`  Action: ${r.action}`);
      lines.push(`  Outcome: ${r.outcome || "(none)"}`);
      lines.push(`  Surprise: ${r.surprise || 0}`);
      if (r.tags && r.tags.length) lines.push(`  Tags: ${r.tags.join(", ")}`);
      if (r.context) lines.push(`  Context: ${r.context}`);
      lines.push("");
    }
    return lines.join("\n");
  }

  function formatSummaryResult(result) {
    if (result.summary) return result.summary;

    const lines = [
      `Session Memory Summary`,
      `  Total entries: ${result.total_entries}`,
      `  Avg surprise: ${result.avg_surprise}`,
      `  High-surprise events (>0.5): ${result.high_surprise_count}`,
      "",
      "Model usage:",
      ...Object.entries(result.model_usage).map(([m, c]) => `  ${m}: ${c}`),
      "",
      "Top tags:",
      ...result.top_tags.map((t) => `  ${t.tag}: ${t.count}`),
      "",
      "Outcome breakdown:",
      ...Object.entries(result.outcome_breakdown).map(
        ([o, c]) => `  ${o}: ${c}`,
      ),
      "",
      `Recent entries (last ${result.recent_entries.length}):`,
      "",
    ];
    for (const e of result.recent_entries) {
      lines.push(
        `  [${e.ts}] ${e.action} → ${e.outcome || "?"} (surprise: ${e.surprise || 0}, model: ${e.model || "?"})`,
      );
    }
    return lines.join("\n");
  }

  return {
    logSessionEvent,
    searchSessionLog,
    getSessionSummary,
    buildSessionIndex,
    formatLogResult,
    formatSearchResults,
    formatSummaryResult,
    SESSION_DIR,
    LOG_PATH,
    INDEX_PATH,
  };
};
