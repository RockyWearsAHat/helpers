/**
 * Web research tool schemas + dispatch handler. Knowledge, session memory, and
 * the project index are implemented natively in the gsh-native Rust binary (see
 * lib/mcp-native.js); only the puppeteer-backed web tools remain here.
 */

// ─── Tool schemas ───────────────────────────────────────────────────────────

const RESEARCH_TOOLS = [
  // search_web — Google search with optional inline scraping of top results.
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
  // scrape_webpage — fetch + clean article text (and PDF pages/images).
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
];

// ─── Handler factory ────────────────────────────────────────────────────────

/**
 * Create the dispatch handler for web research tools.
 * @param {object} fns — functions returned by createResearch()
 * @returns {function} async (toolName, toolArguments) => content[] | null
 */
function createHandler(fns) {
  const { searchWeb, fetchPages, formatSearchResult, formatFetchPagesResult } =
    fns;

  return async function handleResearchToolCall(toolName, toolArguments) {
    if (toolName === "search_web") {
      const result = await searchWeb(toolArguments);
      return [{ type: "text", text: formatSearchResult(result) }];
    }
    if (toolName === "scrape_webpage") {
      const result = await fetchPages(toolArguments);
      return [{ type: "text", text: formatFetchPagesResult(result) }];
    }
    // Knowledge + session memory + project index are handled natively
    // (gsh-native) — see lib/mcp-native.js.
    return null;
  };
}

module.exports = { RESEARCH_TOOLS, createHandler };
