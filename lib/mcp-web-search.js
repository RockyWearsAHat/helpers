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
    const requestedMax = args.max_results
      ? Math.max(1, Math.min(100, Math.round(Number(args.max_results))))
      : GOOGLE_DEFAULT_PAGE_COUNT * GOOGLE_RESULTS_PER_PAGE;
    const targetPages = Math.max(
      1,
      Math.min(10, Math.ceil(requestedMax / GOOGLE_RESULTS_PER_PAGE)),
    );
    const searchUrls = [];

    for (let pageIndex = 0; pageIndex < targetPages; pageIndex++) {
      const params = new URLSearchParams({
        q: fullQuery,
        hl: args.language || "en",
        filter: "0",
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

    // Fetch Google results: page 1 sequentially (CAPTCHA gate), then
    // remaining pages in parallel for speed.  Google killed &num= in Sept 2025
    // so each page returns exactly 10 results — parallelism is the only way
    // to get 100 results fast.
    let results = [];
    let provider = "google";
    let lastZeroOutcome = null;
    const maxResults = requestedMax;

    for (let attempt = 0; attempt < GOOGLE_EMPTY_RETRY_MAX; attempt++) {
      // --- Phase 1: fetch page 1 to check for CAPTCHA ---
      await googleRateLimit();
      let firstOutcome;
      try {
        firstOutcome = await searchGoogleHeadless(searchUrls[0]);
      } catch (err) {
        process.stderr.write(
          `[git-research-mcp] Google search failed: ${err.message}\n`,
        );
        break;
      }

      if (firstOutcome.challenge) {
        if (!canUseLiveChromeFallback()) {
          throw new Error(
            "Google presented a CAPTCHA and live Chrome fallback is unavailable on this platform.",
          );
        }

        const challengeUrl = firstOutcome.pageUrl || searchUrls[0];
        process.stderr.write(
          `[git-research-mcp] Google CAPTCHA encountered — opening interactive Chrome on ${challengeUrl}\n`,
        );
        await resetHeadlessBrowser();
        const challengeOutcome =
          await resolveGoogleChallengeViaLiveChrome(challengeUrl);
        if (challengeOutcome.challenge) {
          throw new Error(
            "Google presented a CAPTCHA in the interactive browser. Solve it, then retry the search.",
          );
        }

        if (
          Array.isArray(challengeOutcome.results) &&
          challengeOutcome.results.length > 0
        ) {
          results = mergeGoogleResults(
            results,
            challengeOutcome.results,
            maxResults,
          );
        }

        // After CAPTCHA solve, retry from the top with a fresh headless browser.
        if (results.length === 0) {
          continue;
        }
        // Got results from interactive Chrome — skip to parallel phase for
        // remaining pages.
      }

      // Merge page 1 results if they came from headless.
      if (!firstOutcome.challenge && firstOutcome.results.length > 0) {
        results = mergeGoogleResults(results, firstOutcome.results, maxResults);
      } else if (!firstOutcome.challenge && firstOutcome.results.length === 0) {
        lastZeroOutcome = {
          requestedUrl: searchUrls[0],
          pageUrl: firstOutcome.pageUrl || "",
          pageTitle: firstOutcome.pageTitle || "",
          bodyText: firstOutcome.bodyText || "",
        };
      }

      if (results.length === 0) {
        // Page 1 was empty — retry with backoff.
        if (attempt < GOOGLE_EMPTY_RETRY_MAX - 1) {
          process.stderr.write(
            `[git-research-mcp] Google returned 0 results — retrying in ${GOOGLE_EMPTY_RETRY_DELAY_MS * (attempt + 1)}ms (attempt ${attempt + 1}/${GOOGLE_EMPTY_RETRY_MAX})\n`,
          );
          await sleep(GOOGLE_EMPTY_RETRY_DELAY_MS * (attempt + 1));
          continue;
        }
        break;
      }

      // --- Phase 2: fetch remaining pages in parallel ---
      if (searchUrls.length > 1) {
        const remainingUrls = searchUrls.slice(1);
        const PARALLEL_BATCH_SIZE = 3;
        process.stderr.write(
          `[git-research-mcp] Fetching ${remainingUrls.length} additional pages in parallel (batches of ${PARALLEL_BATCH_SIZE})\n`,
        );

        for (
          let batchStart = 0;
          batchStart < remainingUrls.length;
          batchStart += PARALLEL_BATCH_SIZE
        ) {
          const batch = remainingUrls.slice(
            batchStart,
            batchStart + PARALLEL_BATCH_SIZE,
          );
          const batchResults = await Promise.allSettled(
            batch.map((url) => searchGoogleHeadless(url)),
          );

          let batchEmpty = true;
          for (const settled of batchResults) {
            if (settled.status !== "fulfilled") continue;
            const outcome = settled.value;
            if (outcome.challenge) {
              process.stderr.write(
                `[git-research-mcp] CAPTCHA hit on parallel page — stopping pagination\n`,
              );
              batchEmpty = false;
              break;
            }
            if (outcome.results.length > 0) {
              results = mergeGoogleResults(
                results,
                outcome.results,
                maxResults,
              );
              batchEmpty = false;
            }
          }

          // If an entire batch returned nothing, later pages won't either.
          if (batchEmpty) break;
        }
      }

      break; // Got results — done.
    }

    const deduped = results.map((item, i) => ({
      rank: i + 1,
      title: item.title || "Untitled",
      url: item.url,
      display_url: item.url,
      snippet: item.snippet || "",
      engines: provider,
    }));

    // Auto-scrape: fetch full page content for the top N results inline.
    const autoScrape = Math.max(
      0,
      Math.min(10, Math.round(Number(args.auto_scrape) || 0)),
    );
    if (autoScrape > 0 && deduped.length > 0) {
      const toScrape = deduped.slice(0, autoScrape);
      const settled = await Promise.allSettled(
        toScrape.map(async (item) => {
          const html = await fetchText(item.url);
          return { url: item.url, title: getTitle(html), text: stripHtml(html) };
        }),
      );
      for (let i = 0; i < settled.length; i++) {
        if (settled[i].status === "fulfilled" && settled[i].value) {
          deduped[i].page_content = settled[i].value.text;
          if (settled[i].value.title) {
            deduped[i].page_title = settled[i].value.title;
          }
        } else if (settled[i].status === "rejected") {
          deduped[i].scrape_error = settled[i].reason?.message || "fetch failed";
        }
      }
    }

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
      if (item.page_content) {
        lines.push(`   --- Page Content ---`);
        lines.push(`   ${item.page_content}`);
      }
      if (item.scrape_error) {
        lines.push(`   Scrape error: ${item.scrape_error}`);
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
