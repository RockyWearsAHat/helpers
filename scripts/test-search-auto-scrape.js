#!/usr/bin/env node

// scripts/test-search-auto-scrape.js
// Tests the auto_scrape parameter on search_web.
// Uses dependency injection to mock all network calls.

"use strict";

const fs = require("fs/promises");
const path = require("path");

let passed = 0;
let failed = 0;

const TEMP_WORKSPACE = path.join(
  "/tmp",
  "test-search-auto-scrape-" + Date.now(),
);

function assert(condition, label) {
  if (condition) {
    passed++;
    process.stderr.write(`  PASS: ${label}\n`);
  } else {
    failed++;
    process.stderr.write(`  FAIL: ${label}\n`);
  }
}

async function fileExists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

// --- Mock deps ---

function makeMockDeps(opts = {}) {
  const searchResults = opts.searchResults || [
    {
      title: "Page One",
      url: "https://example.com/1",
      snippet: "First result",
    },
    {
      title: "Page Two",
      url: "https://example.com/2",
      snippet: "Second result",
    },
    { title: "Page Three", url: "https://example.com/3", snippet: "Third" },
  ];

  const pageHtml = {
    "https://example.com/1":
      "<html><head><title>Full Page One</title></head><body><p>Content of page one.</p></body></html>",
    "https://example.com/2":
      "<html><head><title>Full Page Two</title></head><body><p>Content of page two.</p></body></html>",
    "https://example.com/3":
      "<html><head><title>Full Page Three</title></head><body><p>Content of page three.</p></body></html>",
  };

  const searchGoogleHeadless =
    opts.searchGoogleHeadless ||
    (async () => ({
      challenge: false,
      noResults: false,
      results: searchResults,
      pageUrl: "",
      pageTitle: "",
      bodyText: "",
    }));
  const sleep = opts.sleep || (async () => {});
  const resolveGoogleChallengeViaLiveChrome =
    opts.resolveGoogleChallengeViaLiveChrome ||
    (async () => ({ challenge: true, noResults: false, results: [] }));

  return {
    fetchText: async (url) => {
      if (opts.fetchTextFail && opts.fetchTextFail.includes(url)) {
        throw new Error(`Simulated fetch failure for ${url}`);
      }
      return pageHtml[url] || "<html><body>Unknown</body></html>";
    },
    fetchJson: async () => ({}),
    fetchWithRetry: async () => "",
    getTitle: (html) => {
      const m = html.match(/<title>([^<]*)<\/title>/i);
      return m ? m[1] : "";
    },
    stripHtml: (html) => {
      return html
        .replace(/<[^>]+>/g, " ")
        .replace(/\s+/g, " ")
        .trim();
    },
    decodeHtmlEntities: (s) => s,
    sleep,
    toPositiveInt: (v, d) => Math.max(1, Math.round(Number(v) || d)),
    summarizeInline: (t) => t,
    canUseLiveChromeFallback: opts.canUseLiveChromeFallback || (() => false),
    collectGoogleResultsViaLiveChrome: async () => ({ results: [] }),
    searchGoogleHeadless,
    parseGoogleResults: () => [],
    postProcessGoogleResults: (r) => r,
    mergeGoogleResults: (existing, incoming, max) => {
      const seen = new Set(existing.map((r) => r.url));
      const merged = [...existing];
      for (const r of incoming) {
        if (!seen.has(r.url) && merged.length < max) {
          seen.add(r.url);
          merged.push(r);
        }
      }
      return merged;
    },
    resetHeadlessBrowser: async () => {},
    runInteractiveGoogleBrowser: async () => ({ results: [] }),
    googleRateLimit: async () => {},
    resolveGoogleChallengeViaLiveChrome,
    WORKSPACE_ROOT: opts.workspaceRoot || TEMP_WORKSPACE,
    DEFAULT_USER_AGENT: "test-agent",
    GOOGLE_RESULTS_PER_PAGE: 10,
    GOOGLE_DEFAULT_PAGE_COUNT: 2,
    GOOGLE_DEFAULT_ACCEPT_LANGUAGE: "en",
    GOOGLE_EMPTY_RETRY_MAX: opts.googleEmptyRetryMax || 1,
    GOOGLE_EMPTY_RETRY_DELAY_MS: opts.googleEmptyRetryDelayMs || 10,
  };
}

// --- Tests ---

const createWebSearch = require("../lib/mcp-web-search");

async function main() {
  await fs.mkdir(TEMP_WORKSPACE, { recursive: true });

  // 1. auto_scrape=0 (default) — no page_content on results
  {
    const { searchWeb } = createWebSearch(makeMockDeps());
    const result = await searchWeb({ query: "test" });
    assert(result.results.length === 3, "Returns 3 results");
    assert(
      !result.results[0].page_content,
      "auto_scrape=0: no page_content on result 1",
    );
    assert(
      !result.results[1].page_content,
      "auto_scrape=0: no page_content on result 2",
    );
  }

  // 2. auto_scrape=2 — first 2 results get scraped content
  {
    const { searchWeb } = createWebSearch(makeMockDeps());
    const result = await searchWeb({ query: "test", auto_scrape: 2 });
    assert(
      result.results[0].page_content &&
        result.results[0].page_content.includes("Content of page one"),
      "auto_scrape=2: result 1 has page content",
    );
    assert(
      result.results[0].page_title === "Full Page One",
      "auto_scrape=2: result 1 has page title",
    );
    assert(
      result.results[1].page_content &&
        result.results[1].page_content.includes("Content of page two"),
      "auto_scrape=2: result 2 has page content",
    );
    assert(
      !result.results[2].page_content,
      "auto_scrape=2: result 3 NOT scraped",
    );
  }

  // 3. auto_scrape with fetch failure — sets scrape_error
  {
    const deps = makeMockDeps({
      fetchTextFail: ["https://example.com/1"],
    });
    const { searchWeb } = createWebSearch(deps);
    const result = await searchWeb({ query: "test", auto_scrape: 2 });
    assert(
      result.results[0].scrape_error &&
        result.results[0].scrape_error.includes("Simulated fetch failure"),
      "Fetch failure sets scrape_error on result 1",
    );
    assert(
      result.results[1].page_content &&
        result.results[1].page_content.includes("Content of page two"),
      "Result 2 still scraped despite result 1 failure",
    );
  }

  // 4. auto_scrape clamped to 10 max
  {
    const { searchWeb } = createWebSearch(makeMockDeps());
    const result = await searchWeb({ query: "test", auto_scrape: 50 });
    // Only 3 results exist, so only 3 should be scraped (clamped by actual count)
    assert(
      result.results[2].page_content &&
        result.results[2].page_content.includes("Content of page three"),
      "auto_scrape=50 clamped: all 3 results scraped",
    );
  }

  // 5. auto_scrape negative — treated as 0
  {
    const { searchWeb } = createWebSearch(makeMockDeps());
    const result = await searchWeb({ query: "test", auto_scrape: -5 });
    assert(
      !result.results[0].page_content,
      "auto_scrape=-5: no page_content (treated as 0)",
    );
  }

  // 6. formatSearchResult includes page content when present
  {
    const { searchWeb, formatSearchResult } = createWebSearch(makeMockDeps());
    const result = await searchWeb({ query: "test", auto_scrape: 1 });
    const formatted = formatSearchResult(result);
    assert(
      formatted.includes("--- Page Content ---"),
      "formatSearchResult includes page content section",
    );
    assert(
      formatted.includes("Content of page one"),
      "formatSearchResult includes actual page text",
    );
  }

  // 7. formatSearchResult shows scrape_error
  {
    const deps = makeMockDeps({
      fetchTextFail: ["https://example.com/1"],
    });
    const { searchWeb, formatSearchResult } = createWebSearch(deps);
    const result = await searchWeb({ query: "test", auto_scrape: 1 });
    const formatted = formatSearchResult(result);
    assert(
      formatted.includes("Scrape error:"),
      "formatSearchResult includes scrape error",
    );
  }

  // 8. fetchPages writes only when output_file targets an explicit subdirectory
  {
    const { fetchPages } = createWebSearch(makeMockDeps());
    const result = await fetchPages({
      urls: ["https://example.com/1"],
      output_file: "knowledge/scraped-page.md",
    });
    const writtenPath = path.join(
      TEMP_WORKSPACE,
      "knowledge",
      "scraped-page.md",
    );
    const content = await fs.readFile(writtenPath, "utf8");
    assert(
      result.output_file === "knowledge/scraped-page.md",
      "fetchPages returns normalized explicit subdirectory output path",
    );
    assert(
      content.includes("Content of page one"),
      "fetchPages writes scraped content to the requested subdirectory",
    );
  }

  // 9. fetchPages rejects bare filenames so they cannot land at workspace root
  {
    const { fetchPages } = createWebSearch(makeMockDeps());
    let error = null;
    try {
      await fetchPages({
        urls: ["https://example.com/1"],
        output_file: "research-root-spill.md",
      });
    } catch (err) {
      error = err;
    }
    assert(
      error && error.message.includes("must include an explicit subdirectory"),
      "fetchPages rejects bare output_file names",
    );
    assert(
      !(await fileExists(path.join(TEMP_WORKSPACE, "research-root-spill.md"))),
      "fetchPages does not create a workspace-root file when output_file is bare",
    );
  }

  // 10. confirmed no-results pages stop immediately without retries or interactive fallback
  {
    let searchCalls = 0;
    let sleepCalls = 0;
    let fallbackCalls = 0;
    const { searchWeb, formatSearchResult } = createWebSearch(
      makeMockDeps({
        googleEmptyRetryMax: 4,
        canUseLiveChromeFallback: () => true,
        sleep: async () => {
          sleepCalls++;
        },
        searchGoogleHeadless: async () => {
          searchCalls++;
          return {
            challenge: false,
            noResults: true,
            results: [],
            pageUrl: "https://www.google.com/search?q=definitely-no-match",
            pageTitle: "No results",
            bodyText:
              "Your search - definitely no match - did not match any documents. Suggestions: try different keywords.",
          };
        },
        resolveGoogleChallengeViaLiveChrome: async () => {
          fallbackCalls++;
          return { challenge: false, noResults: false, results: [] };
        },
      }),
    );
    const result = await searchWeb({ query: "definitely no match" });
    const formatted = formatSearchResult(result);
    assert(result.results.length === 0, "Confirmed no-results returns empty list");
    assert(searchCalls === 1, "Confirmed no-results skips retries");
    assert(sleepCalls === 0, "Confirmed no-results skips retry backoff sleeps");
    assert(fallbackCalls === 0, "Confirmed no-results skips interactive fallback");
    assert(
      formatted.includes("Google reported no matching results"),
      "formatSearchResult explains confirmed no-results pages",
    );
  }

  // 11. challenge resolution retries the caller query instead of merging interactive results
  {
    let searchCalls = 0;
    let fallbackCalls = 0;
    const retriedHeadlessResults = [
      {
        title: "Page One",
        url: "https://example.com/1",
        snippet: "First result",
      },
      {
        title: "Page Two",
        url: "https://example.com/2",
        snippet: "Second result",
      },
    ];
    const { searchWeb } = createWebSearch(
      makeMockDeps({
        googleEmptyRetryMax: 2,
        canUseLiveChromeFallback: () => true,
        searchGoogleHeadless: async () => {
          searchCalls++;
          if (searchCalls === 1) {
            return {
              challenge: true,
              noResults: false,
              results: [],
              pageUrl: "https://www.google.com/sorry/index",
              pageTitle: "About this page",
              bodyText: "detected unusual traffic",
            };
          }
          return {
            challenge: false,
            noResults: false,
            results: retriedHeadlessResults,
            pageUrl: "https://www.google.com/search?q=test",
            pageTitle: "Google Search",
            bodyText: "",
          };
        },
        resolveGoogleChallengeViaLiveChrome: async () => {
          fallbackCalls++;
          return {
            challenge: false,
            noResults: false,
            results: [
              {
                title: "Interactive Result",
                url: "https://example.com/interactive",
                snippet: "Should not be merged into the caller results",
              },
            ],
          };
        },
      }),
    );
    const result = await searchWeb({ query: "test" });
    assert(fallbackCalls === 1, "Challenge path invokes interactive resolution once");
    assert(
      searchCalls >= 2,
      "Challenge path retries headless search after resolution",
    );
    assert(
      result.results.every((item) => item.url !== "https://example.com/interactive"),
      "Interactive results are not merged into the caller response",
    );
  }

  // Summary
  process.stderr.write(`\nauto_scrape: ${passed} passed, ${failed} failed\n`);
  await fs.rm(TEMP_WORKSPACE, { recursive: true, force: true });
  if (failed > 0) {
    process.stdout.write(`TEST_SUMMARY: fail ${failed}/${passed + failed}\n`);
    process.exit(1);
  }
  process.stdout.write(`TEST_SUMMARY: pass ${passed}/${passed + failed}\n`);
}

main().catch(async (err) => {
  await fs.rm(TEMP_WORKSPACE, { recursive: true, force: true });
  process.stderr.write(`Unexpected error: ${err.message}\n${err.stack}\n`);
  process.stdout.write("TEST_SUMMARY: fail 1/1\n");
  process.exit(1);
});
