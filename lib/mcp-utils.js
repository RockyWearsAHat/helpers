// lib/mcp-utils.js — Shared utilities for git-research-mcp
"use strict";

const { execFile } = require("child_process");

module.exports = function createUtils(config) {
  const {
    DEFAULT_USER_AGENT,
    RETRY_MAX_ATTEMPTS,
    RETRY_BASE_DELAY_MS,
    RETRY_MAX_DELAY_MS,
    FETCH_TIMEOUT_MS,
  } = config;

  // -------------------------------------------------------------------------
  // Environment helpers
  // -------------------------------------------------------------------------

  function requireEnv(name) {
    const value = process.env[name];
    if (!value) {
      throw new Error(`Missing required environment variable: ${name}`);
    }
    return value;
  }

  function toPositiveInt(value, fallback, min, max) {
    const parsed = Number.parseInt(String(value ?? fallback), 10);
    if (!Number.isFinite(parsed)) {
      return fallback;
    }
    return Math.min(max, Math.max(min, parsed));
  }

  // -------------------------------------------------------------------------
  // HTML / text utilities
  // -------------------------------------------------------------------------

  function decodeHtmlEntities(text) {
    return text
      .replace(/&nbsp;/gi, " ")
      .replace(/&amp;/gi, "&")
      .replace(/&quot;/gi, '"')
      .replace(/&#39;/gi, "'")
      .replace(/&lt;/gi, "<")
      .replace(/&gt;/gi, ">")
      .replace(/&#(\d+);/g, (_, code) =>
        String.fromCharCode(Number.parseInt(code, 10)),
      )
      .replace(/&#x([0-9a-f]+);/gi, (_, code) =>
        String.fromCharCode(Number.parseInt(code, 16)),
      );
  }

  /**
   * Extract the content between the LAST opening <tag> and LAST closing </tag>
   * for a given tag name.  Greedy — captures everything including nested tags.
   */
  function extractTagContent(html, tag) {
    // Greedy: find the last </tag> and pair it with the first <tag>
    const openRe = new RegExp(`<${tag}\\b[^>]*>`, "i");
    const closeRe = new RegExp(`</${tag}>`, "gi");
    const openMatch = html.match(openRe);
    if (!openMatch) return null;

    const startIdx = openMatch.index + openMatch[0].length;

    // Find the last closing tag (greedy match)
    let lastClose = -1;
    let m;
    while ((m = closeRe.exec(html)) !== null) {
      lastClose = m.index;
    }
    if (lastClose <= startIdx) return null;

    return html.slice(startIdx, lastClose);
  }

  /**
   * Extract the main content region from an HTML document.
   * Tries <article>, <main>, [role="main"], content-classed divs, then <body>.
   * Strips nav/header/footer/aside/form chrome before returning.
   */
  function extractMainContent(html) {
    // Remove non-visible and non-content blocks first (global)
    let cleaned = html
      .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gis, " ")
      .replace(/<style\b[^<]*(?:(?!<\/style>)<[^<]*)*<\/style>/gis, " ")
      .replace(
        /<noscript\b[^<]*(?:(?!<\/noscript>)<[^<]*)*<\/noscript>/gis,
        " ",
      )
      .replace(/<svg\b[^<]*(?:(?!<\/svg>)<[^<]*)*<\/svg>/gis, " ")
      .replace(/<iframe\b[^<]*(?:(?!<\/iframe>)<[^<]*)*<\/iframe>/gis, " ");

    // Try semantic content tags in priority order (greedy extraction)
    const tagCandidates = ["article", "main"];
    let content = "";

    for (const tag of tagCandidates) {
      const extracted = extractTagContent(cleaned, tag);
      if (extracted && extracted.replace(/<[^>]+>/g, "").trim().length > 200) {
        content = extracted;
        break;
      }
    }

    // Try role="main" div
    if (!content) {
      const roleMainMatch = cleaned.match(
        /<div[^>]*role\s*=\s*["']main["'][^>]*>/i,
      );
      if (roleMainMatch) {
        const start = roleMainMatch.index + roleMainMatch[0].length;
        const rest = cleaned.slice(start);
        // Grab a large chunk — divs nest, so take everything until a late </div>
        const divClosePositions = [];
        const closeRe = /<\/div>/gi;
        let m;
        while ((m = closeRe.exec(rest)) !== null) {
          divClosePositions.push(m.index);
        }
        if (divClosePositions.length > 0) {
          const candidate = rest.slice(
            0,
            divClosePositions[divClosePositions.length - 1],
          );
          if (candidate.replace(/<[^>]+>/g, "").trim().length > 200) {
            content = candidate;
          }
        }
      }
    }

    // Try content-classed divs (id or class containing article/post/entry/content)
    if (!content) {
      const contentDivRe =
        /<div[^>]*(?:class|id)\s*=\s*["'][^"']*\b(?:article|post|entry|content|main-content|page-content)\b[^"']*["'][^>]*>/gi;
      let bestCandidate = "";
      let bestLen = 0;
      let divMatch;
      while ((divMatch = contentDivRe.exec(cleaned)) !== null) {
        const start = divMatch.index + divMatch[0].length;
        const rest = cleaned.slice(start);
        // Find matching depth — take everything up to the last </div> in a
        // reasonable window (first 500KB) to avoid runaway on huge pages
        const window = rest.slice(0, 500000);
        const lastDiv = window.lastIndexOf("</div>");
        if (lastDiv > 0) {
          const text = window
            .slice(0, lastDiv)
            .replace(/<[^>]+>/g, "")
            .trim();
          if (text.length > bestLen) {
            bestLen = text.length;
            bestCandidate = window.slice(0, lastDiv);
          }
        }
      }
      if (bestLen > 200) {
        content = bestCandidate;
      }
    }

    // Fallback: use the body, or the whole document
    if (!content) {
      const bodyContent = extractTagContent(cleaned, "body");
      content = bodyContent || cleaned;
    }

    // Strip navigation/chrome elements from the content region
    content = content
      .replace(/<nav\b[\s\S]*?<\/nav>/gis, " ")
      .replace(/<header\b[\s\S]*?<\/header>/gis, " ")
      .replace(/<footer\b[\s\S]*?<\/footer>/gis, " ")
      .replace(/<aside\b[\s\S]*?<\/aside>/gis, " ")
      .replace(/<form\b[\s\S]*?<\/form>/gis, " ")
      .replace(/<button\b[\s\S]*?<\/button>/gis, " ")
      .replace(/<select\b[\s\S]*?<\/select>/gis, " ")
      .replace(/<dialog\b[\s\S]*?<\/dialog>/gis, " ");

    return content;
  }

  function stripHtml(html) {
    const content = extractMainContent(html);
    return decodeHtmlEntities(
      content
        .replace(/<br\s*\/?>/gi, "\n")
        .replace(
          /<\/(p|div|section|article|main|li|tr|table|h1|h2|h3|h4|h5|h6|blockquote|pre)>/gi,
          "$&\n",
        )
        .replace(/<[^>]+>/g, " ")
        .replace(/[ \t]+/g, " ")
        .replace(/\n[ \t]+/g, "\n")
        .replace(/\n{3,}/g, "\n\n")
        .trim(),
    );
  }

  function getTitle(html) {
    const match = html.match(/<title[^>]*>([\s\S]*?)<\/title>/i);
    return match ? decodeHtmlEntities(match[1].trim()) : "Untitled";
  }

  function summarizeText(text, maxChars) {
    if (text.length <= maxChars) {
      return text;
    }

    const clipped = text.slice(0, maxChars);
    const lastBreak = Math.max(
      clipped.lastIndexOf("\n\n"),
      clipped.lastIndexOf(". "),
    );
    if (lastBreak > maxChars * 0.6) {
      return `${clipped.slice(0, lastBreak).trim()}...`;
    }
    return `${clipped.trim()}...`;
  }

  function summarizeInline(text, maxChars) {
    return summarizeText(text.replace(/\s+/g, " ").trim(), maxChars);
  }

  function getMarkdownTitle(text, fallback) {
    const headingMatch = text.match(/^#\s+(.+)$/m);
    return headingMatch ? headingMatch[1].trim() : fallback;
  }

  function escapeRegExp(text) {
    return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  function tokenizeQuery(query) {
    return String(query)
      .toLowerCase()
      .split(/[^a-z0-9_.-]+/)
      .map((term) => term.trim())
      .filter((term) => term.length >= 2);
  }

  // -------------------------------------------------------------------------
  // Async / HTTP utilities
  // -------------------------------------------------------------------------

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  function execFileAsync(command, args, options = {}) {
    return new Promise((resolve, reject) => {
      execFile(command, args, options, (err, stdout, stderr) => {
        if (err) {
          err.stdout = stdout;
          err.stderr = stderr;
          reject(err);
          return;
        }
        resolve({ stdout, stderr });
      });
    });
  }

  function isRetryable(status) {
    return status === 429 || (status >= 500 && status < 600);
  }

  async function fetchWithRetry(url, options) {
    let lastError;
    for (let attempt = 0; attempt < RETRY_MAX_ATTEMPTS; attempt++) {
      // Compose caller-supplied signal with a per-attempt timeout so fetches
      // never hang forever, even under heavy concurrent load.
      const timeoutSignal = AbortSignal.timeout(FETCH_TIMEOUT_MS);
      const signal = options?.signal
        ? AbortSignal.any([options.signal, timeoutSignal])
        : timeoutSignal;
      let response;
      try {
        response = await fetch(url, { ...options, signal });
      } catch (fetchErr) {
        // Network-level error (timeout, DNS, refused) — treat as retryable
        if (attempt < RETRY_MAX_ATTEMPTS - 1) {
          const delayMs = Math.min(
            RETRY_BASE_DELAY_MS * Math.pow(2, attempt),
            RETRY_MAX_DELAY_MS,
          );
          process.stderr.write(
            `[git-research-mcp] Fetch error from ${url}: ${fetchErr.message} — retrying in ${(delayMs / 1000).toFixed(1)}s (attempt ${attempt + 1}/${RETRY_MAX_ATTEMPTS})\n`,
          );
          lastError = fetchErr;
          await sleep(delayMs);
          continue;
        }
        throw fetchErr;
      }
      if (response.ok) {
        return response;
      }
      if (!isRetryable(response.status)) {
        const body = await response.text();
        throw new Error(
          `HTTP ${response.status} from ${url}: ${body.slice(0, 400)}`,
        );
      }
      const retryAfterHeader = response.headers.get("retry-after");
      let delayMs;
      if (retryAfterHeader && /^\d+$/.test(retryAfterHeader.trim())) {
        delayMs = Math.min(
          parseInt(retryAfterHeader, 10) * 1000,
          RETRY_MAX_DELAY_MS,
        );
      } else {
        delayMs = Math.min(
          RETRY_BASE_DELAY_MS * Math.pow(2, attempt),
          RETRY_MAX_DELAY_MS,
        );
      }
      process.stderr.write(
        `[git-research-mcp] HTTP ${response.status} from ${url} — retrying in ${(delayMs / 1000).toFixed(1)}s (attempt ${attempt + 1}/${RETRY_MAX_ATTEMPTS})\n`,
      );
      lastError = new Error(`HTTP ${response.status} from ${url}`);
      await sleep(delayMs);
    }
    throw lastError;
  }

  async function fetchJson(url) {
    const response = await fetchWithRetry(url, {
      headers: {
        "user-agent": DEFAULT_USER_AGENT,
        accept: "application/json",
      },
    });
    return response.json();
  }

  async function fetchText(url) {
    const response = await fetchWithRetry(url, {
      headers: {
        "user-agent": DEFAULT_USER_AGENT,
        accept:
          "text/html,application/xhtml+xml;q=0.9,text/plain;q=0.8,*/*;q=0.5",
      },
      redirect: "follow",
    });
    // Reject binary content types — reading them as text produces garbage.
    const ct = (response.headers.get("content-type") || "").toLowerCase();
    if (
      ct.includes("application/pdf") ||
      ct.includes("application/octet-stream") ||
      ct.includes("image/") ||
      ct.includes("audio/") ||
      ct.includes("video/") ||
      ct.includes("application/zip") ||
      ct.includes("application/gzip")
    ) {
      throw new Error(
        `Binary content type (${ct.split(";")[0].trim()}) — cannot extract text`,
      );
    }
    return response.text();
  }

  return {
    decodeHtmlEntities,
    extractTagContent,
    extractMainContent,
    stripHtml,
    getTitle,
    summarizeText,
    summarizeInline,
    getMarkdownTitle,
    escapeRegExp,
    tokenizeQuery,
    requireEnv,
    toPositiveInt,
    sleep,
    execFileAsync,
    isRetryable,
    fetchWithRetry,
    fetchJson,
    fetchText,
  };
};
