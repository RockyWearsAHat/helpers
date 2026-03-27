"use strict";
// lib/mcp-web-search.js — Web search and page scraping
const fs = require("fs/promises");
const path = require("path");

module.exports = function createWebSearch(deps) {
  const {
    fetchText,
    fetchJson,
    fetchWithRetry,
    getTitle,
    stripHtml,
    decodeHtmlEntities,
    sleep,
    toPositiveInt,
    summarizeInline,
    canUseLiveChromeFallback,
    collectGoogleResultsViaLiveChrome,
    searchGoogleHeadless,
    parseGoogleResults,
    postProcessGoogleResults,
    mergeGoogleResults,
    resetHeadlessBrowser,
    runInteractiveGoogleBrowser,
    googleRateLimit,
    resolveGoogleChallengeViaLiveChrome,
    WORKSPACE_ROOT,
    DEFAULT_USER_AGENT,
    GOOGLE_RESULTS_PER_PAGE,
    GOOGLE_DEFAULT_PAGE_COUNT,
    GOOGLE_DEFAULT_ACCEPT_LANGUAGE,
    GOOGLE_EMPTY_RETRY_MAX,
    GOOGLE_EMPTY_RETRY_DELAY_MS,
  } = deps;

  async function searchWeb(args) {
    const query = String(args.query || "").trim();
    if (!query) {
      throw new Error("search_web requires a non-empty query.");
    }

    const terms = [query];

    if (args.site_filter) {
      terms.push(`site:${String(args.site_filter).trim()}`);
    }
    if (args.exact_terms) {
      terms.push(`"${String(args.exact_terms).trim()}"`);
    }
    if (args.exclude_terms) {
      for (const term of String(args.exclude_terms)
        .split(/\s+/)
        .filter(Boolean)) {
        terms.push(`-${term}`);
      }
    }
    if (args.file_type) {
      terms.push(`filetype:${String(args.file_type).trim()}`);
    }

    const fullQuery = terms.join(" ");
    const targetPages = Math.max(1, Math.min(10, GOOGLE_DEFAULT_PAGE_COUNT));
    const searchUrls = [];

    for (let pageIndex = 0; pageIndex < targetPages; pageIndex++) {
      const params = new URLSearchParams({
        q: fullQuery,
        hl: args.language || "en",
        num: String(GOOGLE_RESULTS_PER_PAGE),
      });
      if (pageIndex > 0) {
        params.set("start", String(pageIndex * GOOGLE_RESULTS_PER_PAGE));
      }
      if (args.time_range) {
        const tbs = {
          day: "qdr:d",
          week: "qdr:w",
          month: "qdr:m",
          year: "qdr:y",
        };
        if (tbs[args.time_range]) {
          params.set("tbs", tbs[args.time_range]);
        }
      }
      searchUrls.push(`https://www.google.com/search?${params.toString()}`);
    }

    // Try Google directly with empty-result retry logic.
    let results = [];
    let provider = "google";
    let completedWithLiveChrome = false;
    let lastZeroOutcome = null;

    for (let attempt = 0; attempt < GOOGLE_EMPTY_RETRY_MAX; attempt++) {
      const challengeRetryCounts = new Map();

      for (let pageIndex = 0; pageIndex < searchUrls.length; pageIndex++) {
        await googleRateLimit();
        const searchUrl = searchUrls[pageIndex];
        let outcome;
        try {
          outcome = await searchGoogleHeadless(searchUrl);
        } catch (err) {
          process.stderr.write(
            `[git-research-mcp] Google search failed: ${err.message}\n`,
          );
          break;
        }

        if (outcome.challenge) {
          if (!canUseLiveChromeFallback()) {
            throw new Error(
              "Google presented a CAPTCHA and live Chrome fallback is unavailable on this platform.",
            );
          }

          const challengeUrl = outcome.pageUrl || searchUrl;
          process.stderr.write(
            `[git-research-mcp] Google CAPTCHA encountered in headless browser — opening interactive Puppeteer browser on ${challengeUrl} for manual verification\n`,
          );
          await resetHeadlessBrowser();
          const challengeOutcome =
            await resolveGoogleChallengeViaLiveChrome(challengeUrl);
          if (challengeOutcome.challenge) {
            throw new Error(
              "Google presented a CAPTCHA in the interactive Puppeteer browser. The window has been focused and left open so you can solve it. After solving it, retry the search.",
            );
          }

          const challengeRetryCount =
            (challengeRetryCounts.get(searchUrl) || 0) + 1;
          challengeRetryCounts.set(searchUrl, challengeRetryCount);
          if (challengeRetryCount > 1) {
            process.stderr.write(
              `[git-research-mcp] Headless browser is still challenged after manual verification — refusing to scrape results in live Chrome\n`,
            );
            throw new Error(
              "Google is still challenging the headless Puppeteer session after manual Chrome verification. CAPTCHA solving may use regular Chrome, but result scraping remains restricted to headless Puppeteer. Retry the search after the challenge clears.",
            );
          }

          pageIndex -= 1;
          continue;
        }

        challengeRetryCounts.delete(searchUrl);

        if (outcome.results.length > 0) {
          results = mergeGoogleResults(
            results,
            outcome.results,
            targetPages * GOOGLE_RESULTS_PER_PAGE,
          );
        } else {
          lastZeroOutcome = {
            requestedUrl: searchUrl,
            pageUrl: outcome.pageUrl || "",
            pageTitle: outcome.pageTitle || "",
            bodyText: outcome.bodyText || "",
          };
          process.stderr.write(
            `[git-research-mcp] Google returned 0 parsed results for ${searchUrl} -> page=${lastZeroOutcome.pageUrl || "<empty>"} title=${JSON.stringify(lastZeroOutcome.pageTitle || "")} body=${JSON.stringify((lastZeroOutcome.bodyText || "").slice(0, 240))}\n`,
          );
        }

        // Stop early if a later page is empty after we already have results.
        if (pageIndex > 0 && outcome.results.length === 0) {
          break;
        }
      }

      if (completedWithLiveChrome) {
        break;
      }

      if (results.length > 0) {
        break;
      }

      // Empty results — could be transient, wait longer and retry
      if (attempt < GOOGLE_EMPTY_RETRY_MAX - 1) {
        process.stderr.write(
          `[git-research-mcp] Google returned 0 results across ${targetPages} pages — retrying in ${GOOGLE_EMPTY_RETRY_DELAY_MS * (attempt + 1)}ms (attempt ${attempt + 1}/${GOOGLE_EMPTY_RETRY_MAX})\n`,
        );
        await sleep(GOOGLE_EMPTY_RETRY_DELAY_MS * (attempt + 1));
      }
    }

    const deduped = results.map((item, i) => ({
      rank: i + 1,
      title: item.title || "Untitled",
      url: item.url,
      display_url: item.url,
      snippet: item.snippet || "",
      engines: provider,
    }));

    return {
      query: fullQuery,
      provider,
      total_results: String(deduped.length),
      results: deduped,
      debug:
        deduped.length === 0 && lastZeroOutcome
          ? {
              requested_url: lastZeroOutcome.requestedUrl,
              page_url: lastZeroOutcome.pageUrl,
              page_title: lastZeroOutcome.pageTitle,
              body_preview: lastZeroOutcome.bodyText.slice(0, 240),
            }
          : undefined,
    };
  }

  async function fetchPages(args) {
    const urls = Array.isArray(args.urls) ? args.urls : [];
    if (!urls.length) {
      throw new Error("scrape_webpage requires at least one URL.");
    }

    const outputFile = args.output_file ? String(args.output_file).trim() : "";

    // Scrape all URLs concurrently — the fetchWithRetry timeout prevents any
    // single fetch from blocking the rest.
    const settled = await Promise.allSettled(
      urls.map(async (rawUrl) => {
        const url = String(rawUrl).trim();
        if (!url) return null;
        const html = await fetchText(url);
        const title = getTitle(html);
        const text = stripHtml(html);
        return { url, title, text };
      }),
    );

    const pages = [];
    for (const result of settled) {
      if (result.status === "fulfilled" && result.value) {
        pages.push(result.value);
      } else if (result.status === "rejected") {
        process.stderr.write(
          `[git-research-mcp] Scrape failed: ${result.reason?.message || result.reason}\n`,
        );
      }
    }

    if (outputFile) {
      const resolvedPath = path.resolve(WORKSPACE_ROOT, outputFile);
      await fs.mkdir(path.dirname(resolvedPath), { recursive: true });
      const content = pages
        .map((p) => `# ${p.title}\nSource: ${p.url}\n\n${p.text}`)
        .join("\n\n---\n\n");
      await fs.writeFile(resolvedPath, content, "utf8");
      return {
        pages,
        output_file: path.relative(WORKSPACE_ROOT, resolvedPath),
      };
    }

    return { pages };
  }

  function formatSearchResult(result) {
    const lines = [
      `Query: ${result.query}`,
      `Provider: ${result.provider || "searxng"}`,
      `Total results: ${result.total_results}`,
      "",
      "Results:",
    ];

    for (const item of result.results) {
      lines.push(`${item.rank}. ${item.title}`);
      lines.push(`   URL: ${item.url}`);
      if (item.engines) {
        lines.push(`   Engines: ${item.engines}`);
      }
      if (item.snippet) {
        lines.push(`   Snippet: ${item.snippet}`);
      }
    }

    if (!result.results.length) {
      lines.push("No results returned.");
    }

    return lines.join("\n");
  }

  function formatFetchPagesResult(result) {
    const lines = [];

    if (result.output_file) {
      lines.push(`Written to: ${result.output_file}`, "");
    }

    result.pages.forEach((page, index) => {
      if (index > 0) {
        lines.push("", "---", "");
      }
      lines.push(`Title: ${page.title}`);
      lines.push(`URL: ${page.url}`);
      lines.push("", page.text || "No extractable text.");
    });

    return lines.join("\n");
  }

  return {
    searchWeb,
    fetchPages,
    formatSearchResult,
    formatFetchPagesResult,
  };
};
