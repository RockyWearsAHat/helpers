/**
 * Research tool schemas and dispatch handler. Defines the MCP tool
 * definitions for knowledge, web search, and session memory tools,
 * plus a factory that creates the dispatch handler from instantiated
 * module functions.
 */

// ─── Tool schemas ───────────────────────────────────────────────────────────

const RESEARCH_TOOLS = [
  {
    name: "search_knowledge_cache",
    description:
      "Search the durable knowledge cache and return matching note paths with snippets. Searches both the local workspace (.github/knowledge/) and the community knowledge base (knowledge/ in the repo or fetched from GitHub).",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "Search query.",
        },
        max_results: {
          type: "integer",
          description: "Number of note matches to return (1-20).",
        },
      },
      required: ["query"],
    },
  },
  {
    name: "read_knowledge_note",
    description:
      "Read a specific knowledge note. Pass a bare filename (e.g. networking-dns.md) or a workspace-relative path. Resolves from workspace knowledge root, then repo bundled, then GitHub community.",
    inputSchema: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description:
            "Filename (e.g. networking-dns.md) or workspace-relative path. Bare filenames resolve to the detected knowledge root.",
        },
        max_chars: {
          type: "integer",
          description:
            "Optional. Maximum characters to return (500-100000). Default: no limit (full content).",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "search_web",
    description:
      "Search the web via Google. Returns up to max_results deduplicated results (default 20, max 100). Page 1 is fetched first; additional pages are fetched in parallel. Set auto_scrape to fetch full page content for the top N results inline, eliminating the need for a separate scrape_webpage call.",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "Search query.",
        },
        site_filter: {
          type: "string",
          description:
            "Optional domain to scope the search, for example gbatek.com or doc.qt.io.",
        },
        exact_terms: {
          type: "string",
          description: "Terms that must appear in results.",
        },
        exclude_terms: {
          type: "string",
          description: "Terms that should be excluded from results.",
        },
        file_type: {
          type: "string",
          description: "Optional file type filter, for example pdf.",
        },
        language: {
          type: "string",
          description:
            "Optional language code, for example en. Defaults to en.",
        },
        time_range: {
          type: "string",
          description: "Optional time range filter: day, week, month, or year.",
        },
        max_results: {
          type: "number",
          description:
            "Maximum results to return. Default 20, max 100. Each Google page yields ~10 results, so 50 means 5 pages.",
        },
        auto_scrape: {
          type: "number",
          description:
            "Automatically scrape and return full page content for the top N results (0-10). Default 0, which returns only snippets. Set to 3-5 when you need page content, avoiding a separate scrape_webpage call.",
        },
      },
      required: ["query"],
    },
  },
  {
    name: "write_knowledge_note",
    description:
      "Create or overwrite a knowledge note. Writes to the workspace's knowledge directory (auto-detected: knowledge/ in source repo, .github/knowledge/ elsewhere), rebuilds the local index before returning, and can optionally publish to the shared knowledge base when publish=true and sharing is enabled.",
    inputSchema: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description:
            "Filename for the note, e.g. networking-dns.md. Can also be a workspace-relative path. The tool places bare filenames in the detected knowledge root automatically.",
        },
        content: {
          type: "string",
          description: "Full markdown content to write.",
        },
        overwrite: {
          type: "boolean",
          description:
            "Set to true to replace an existing file. Default false (fails if file exists).",
        },
        publish: {
          type: "boolean",
          description:
            "When true, submit the note to the shared knowledge base after the local index rebuild succeeds. Requires shareKnowledge (or legacy shareResearch) to be enabled in community settings.",
        },
      },
      required: ["path", "content"],
    },
  },
  {
    name: "update_knowledge_note",
    description:
      "Replace a specific section (identified by heading) in an existing knowledge note. Preserves all other sections, rebuilds the local index before returning, and can optionally publish the updated note.",
    inputSchema: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description:
            "Filename (e.g. networking-dns.md) or workspace-relative path to the knowledge note.",
        },
        heading: {
          type: "string",
          description:
            "Exact text of the heading to replace (without the # prefix).",
        },
        content: {
          type: "string",
          description:
            "New content to place under the heading. The heading line is preserved; only the body below it is replaced.",
        },
        publish: {
          type: "boolean",
          description:
            "When true, submit the updated note to the shared knowledge base after the local index rebuild succeeds.",
        },
      },
      required: ["path", "heading", "content"],
    },
  },
  {
    name: "append_to_knowledge_note",
    description:
      "Append content to the end of an existing knowledge note. Rebuilds the local index before returning and can optionally publish the updated note when the appended content is shareable.",
    inputSchema: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description:
            "Filename (e.g. networking-dns.md) or workspace-relative path to the knowledge note.",
        },
        content: {
          type: "string",
          description: "Markdown content to append at the end of the file.",
        },
        publish: {
          type: "boolean",
          description:
            "When true, submit the updated note to the shared knowledge base after the local index rebuild succeeds.",
        },
      },
      required: ["path", "content"],
    },
  },
  {
    name: "scrape_webpage",
    description:
      "Fetch web pages, strip HTML chrome (nav, header, footer, sidebar), and return cleaned article text. For PDFs: extracts per-page text and renders page images for visual analysis. Use pdf_pages to request specific pages from a large PDF (e.g. after seeing the page map from an initial scrape).",
    inputSchema: {
      type: "object",
      properties: {
        urls: {
          type: "array",
          items: { type: "string" },
          minItems: 1,
          description: "Absolute URLs to fetch.",
        },
        pdf_pages: {
          type: "array",
          items: { type: "integer" },
          description:
            "Specific page numbers to extract from PDF URLs. When set, returns full text and rendered images only for these pages. Use after initial scrape to get deep content from large PDFs.",
        },
        output_file: {
          type: "string",
          description:
            "Optional workspace-relative path to write scraped content as markdown. Must include an explicit subdirectory (for example knowledge/note.md or .github/knowledge/note.md). When set, content is also returned in the response.",
        },
      },
      required: ["urls"],
    },
  },
  {
    name: "submit_community_research",
    description:
      "Submit a knowledge note to the shared knowledge base as a pull request. Requires knowledge sharing to be enabled in settings. The note content is validated for privacy and the submission rebuilds knowledge/_index.json so the published cache stays searchable.",
    inputSchema: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "Path to the knowledge note to submit.",
        },
      },
      required: ["path"],
    },
  },
  {
    name: "build_knowledge_index",
    description:
      "Build or rebuild the local workspace TF-IDF search index (_index.json) from knowledge files. The community knowledge index is pre-built on GitHub and fetched automatically — this tool only rebuilds the local workspace index. Run manually after bulk additions; also rebuilt automatically after write/update/append operations.",
    inputSchema: {
      type: "object",
      properties: {},
      required: [],
    },
  },
  {
    name: "search_knowledge_index",
    description:
      "Search the knowledge base using TF-IDF indexes. Merges results from the local workspace index (if built) and the community knowledge index (pre-built on GitHub, fetched with ETag caching). Returns ranked results with relevance scores, source tags (local/community), related files, and text snippets. Falls back to keyword search if no index is available.",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "Search query.",
        },
        max_results: {
          type: "integer",
          description: "Number of results to return (1-20). Default: 5.",
        },
      },
      required: ["query"],
    },
  },

  // ─── Session Memory Tools (Engram-inspired learning system) ──────────────
  {
    name: "log_session_event",
    description:
      "Log an action and its outcome to the per-workspace session memory. The index is rebuilt automatically after every write. Use this after any non-trivial action to record what happened, especially when the outcome was unexpected (set surprise > 0.5). This builds a searchable history that prevents repeating mistakes.",
    inputSchema: {
      type: "object",
      properties: {
        action: {
          type: "string",
          description:
            "What was attempted. Be specific: 'refactored git-upload test detection into lib/upload-test-detection.sh', not 'made changes'.",
        },
        outcome: {
          type: "string",
          description:
            "What happened. 'success', 'failed — type error in line 42', 'partial — tests pass but lint warnings remain'.",
        },
        surprise: {
          type: "number",
          description:
            "How unexpected was this outcome? 0.0 = completely expected, 1.0 = totally unexpected. High-surprise events get preferential retrieval weight (Engram dopamine-learning analog). Default: 0.",
        },
        model: {
          type: "string",
          description:
            "The model that performed this action, e.g. 'claude-sonnet-4-6', 'gpt-4o'. Used for model-tier retrieval gating.",
        },
        tags: {
          type: "array",
          items: { type: "string" },
          description:
            "Tags for categorization: ['refactor', 'bash', 'git-upload']. Aids search and summary.",
        },
        context: {
          type: "string",
          description:
            "Optional extra context: the file(s) involved, the error message, the approach that worked or failed.",
        },
      },
      required: ["action"],
    },
  },
  {
    name: "search_session_log",
    description:
      "Search the session memory for past actions and outcomes similar to a query. Uses TF-IDF with surprise-weighted scoring (high-surprise events surface first) and model-tier gating (same-model matches get boosted). Call this BEFORE attempting non-trivial actions to learn from past experience.",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description:
            "What you're about to do or what went wrong. The search finds similar past situations.",
        },
        max_results: {
          type: "integer",
          description: "Number of results to return (1-20). Default: 5.",
        },
        current_model: {
          type: "string",
          description:
            "The model currently running, e.g. 'claude-sonnet-4-6'. Same-model matches get a 1.3x relevance boost.",
        },
      },
      required: ["query"],
    },
  },
  {
    name: "get_session_summary",
    description:
      "Get a summary of the session memory: total entries, average surprise, model usage breakdown, top tags, outcome distribution, and the most recent N events. Use this to understand the project's learning history.",
    inputSchema: {
      type: "object",
      properties: {
        limit: {
          type: "integer",
          description:
            "Number of recent entries to include (1-100). Default: 20.",
        },
      },
      required: [],
    },
  },
  {
    name: "rebuild_session_index",
    description:
      "Manually rebuild the session memory TF-IDF index. Normally this happens automatically after every log_session_event call. Use this only if the index appears corrupted or out of sync.",
    inputSchema: {
      type: "object",
      properties: {},
      required: [],
    },
  },
];

// ─── Handler factory ────────────────────────────────────────────────────────

/**
 * Create the dispatch handler for research tools.
 * @param {object} fns — functions returned by createResearch()
 * @returns {function} async (toolName, toolArguments) => content[] | null
 */
function createHandler(fns) {
  const {
    searchKnowledgeCache,
    readKnowledgeNote,
    writeKnowledgeNote,
    updateKnowledgeNote,
    appendToKnowledgeNote,
    submitCommunityResearch,
    formatKnowledgeSearchResult,
    formatKnowledgeNoteResult,
    formatKnowledgeWriteResult,
    buildKnowledgeIndex,
    searchKnowledgeIndex,
    formatKnowledgeIndexSearchResult,
    formatBuildIndexResult,
    searchWeb,
    fetchPages,
    formatSearchResult,
    formatFetchPagesResult,
    logSessionEvent,
    searchSessionLog,
    getSessionSummary,
    buildSessionIndex,
    formatSessionLogResult,
    formatSessionSearchResults,
    formatSessionSummaryResult,
  } = fns;

  return async function handleResearchToolCall(toolName, toolArguments) {
    if (toolName === "search_knowledge_cache") {
      const result = await searchKnowledgeCache(toolArguments);
      return [{ type: "text", text: formatKnowledgeSearchResult(result) }];
    }
    if (toolName === "read_knowledge_note") {
      const result = await readKnowledgeNote(toolArguments);
      return [{ type: "text", text: formatKnowledgeNoteResult(result) }];
    }
    if (toolName === "search_web") {
      const result = await searchWeb(toolArguments);
      return [{ type: "text", text: formatSearchResult(result) }];
    }
    if (toolName === "write_knowledge_note") {
      const result = await writeKnowledgeNote(toolArguments);
      return [{ type: "text", text: formatKnowledgeWriteResult(result) }];
    }
    if (toolName === "update_knowledge_note") {
      const result = await updateKnowledgeNote(toolArguments);
      return [{ type: "text", text: formatKnowledgeWriteResult(result) }];
    }
    if (toolName === "append_to_knowledge_note") {
      const result = await appendToKnowledgeNote(toolArguments);
      return [{ type: "text", text: formatKnowledgeWriteResult(result) }];
    }
    if (toolName === "scrape_webpage") {
      const result = await fetchPages(toolArguments);
      return [{ type: "text", text: formatFetchPagesResult(result) }];
    }
    if (toolName === "submit_community_research") {
      const result = await submitCommunityResearch(toolArguments);
      return [{ type: "text", text: formatKnowledgeWriteResult(result) }];
    }
    if (toolName === "build_knowledge_index") {
      const result = await buildKnowledgeIndex(toolArguments);
      return [{ type: "text", text: formatBuildIndexResult(result) }];
    }
    if (toolName === "search_knowledge_index") {
      const result = await searchKnowledgeIndex(toolArguments);
      return [{ type: "text", text: formatKnowledgeIndexSearchResult(result) }];
    }
    // ─── Session memory handlers ────────────────────────────────────────
    if (toolName === "log_session_event") {
      const result = await logSessionEvent(toolArguments);
      return [{ type: "text", text: formatSessionLogResult(result) }];
    }
    if (toolName === "search_session_log") {
      const result = await searchSessionLog(toolArguments);
      return [{ type: "text", text: formatSessionSearchResults(result) }];
    }
    if (toolName === "get_session_summary") {
      const result = await getSessionSummary(toolArguments);
      return [{ type: "text", text: formatSessionSummaryResult(result) }];
    }
    if (toolName === "rebuild_session_index") {
      const result = await buildSessionIndex();
      return [
        {
          type: "text",
          text: `Session index rebuilt: ${result.entry_count} entries, ${result.term_count} terms.`,
        },
      ];
    }
    return null;
  };
}

module.exports = { RESEARCH_TOOLS, createHandler };
