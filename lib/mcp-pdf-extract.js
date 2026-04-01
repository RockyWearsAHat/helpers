"use strict";
// lib/mcp-pdf-extract.js — PDF text extraction + page rendering for visual analysis
//
// Uses pdf-parse v2 (PDFParse class) for both text extraction and page
// rendering to PNG. No Puppeteer dependency needed.
//
// Key design:
//   - getText() returns per-page text via r.pages array [{text, num}, ...]
//   - Large PDFs: full text for first/last pages, summaries for middle
//   - Page images: rendered for requested pages, sparse pages, or first N
//   - Agent can request specific pages via pdf_pages parameter for follow-up

const fs = require("fs");
const os = require("os");
const path = require("path");

// Max pages to render as images (controls vision API cost)
const MAX_RENDER_PAGES = 5;
// Max total text chars before switching to structured per-page output
const MAX_TEXT_CHARS = 80000;
// Pages with fewer chars than this are considered sparse (likely graphical)
const SPARSE_PAGE_THRESHOLD = 50;
// When structured output is used, show full text for first/last N pages
const FULL_TEXT_WINDOW = 8;

let _pdfParseModule = null;

function getPDFParse() {
  if (_pdfParseModule) return _pdfParseModule;
  try {
    _pdfParseModule = require("pdf-parse");
  } catch {
    const globalRoot = path.join(
      process.env.HOME || "",
      ".nvm/versions/node",
      process.version,
      "lib/node_modules",
    );
    _pdfParseModule = require(path.join(globalRoot, "pdf-parse"));
  }
  return _pdfParseModule;
}

/**
 * Download a PDF from a URL and return its Buffer.
 */
async function downloadPdf(url, fetchWithRetry, userAgent) {
  const response = await fetchWithRetry(url, {
    headers: {
      "user-agent": userAgent,
      accept: "application/pdf,*/*",
    },
    redirect: "follow",
  });

  if (!response.ok) {
    const status =
      typeof response.status === "number" ? response.status : "unknown";
    const statusText = response.statusText || "Unknown status";
    throw new Error(
      `Failed to download PDF from ${url}: HTTP ${status} ${statusText}`,
    );
  }

  let contentType;
  if (response.headers) {
    if (typeof response.headers.get === "function") {
      contentType =
        response.headers.get("content-type") ||
        response.headers.get("Content-Type");
    } else if (typeof response.headers === "object") {
      contentType =
        response.headers["content-type"] || response.headers["Content-Type"];
    }
  }

  if (
    contentType &&
    !/application\/pdf/i.test(contentType) &&
    !/application\/octet-stream/i.test(contentType)
  ) {
    throw new Error(
      `Unexpected content-type when downloading PDF from ${url}: ${contentType}`,
    );
  }

  const arrayBuf = await response.arrayBuffer();
  return Buffer.from(arrayBuf);
}

/**
 * Build structured per-page text output for large PDFs.
 * Full text for first/last pages and requested pages; summaries for the rest.
 */
function buildStructuredText(pageArray, requestedPages) {
  const total = pageArray.length;
  if (total === 0) return { text: "", pageSummary: [] };

  const requestedSet = new Set(requestedPages || []);
  const lines = [];
  const pageSummary = [];

  for (const pg of pageArray) {
    const trimmed = pg.text.trim();
    const firstLine = trimmed.split("\n")[0] || "";
    const isSparse = trimmed.length < SPARSE_PAGE_THRESHOLD;

    pageSummary.push({
      page: pg.num,
      chars: trimmed.length,
      sparse: isSparse,
      heading: firstLine.slice(0, 120),
    });

    // Show full text for: first window, last window, sparse pages, requested pages
    const inFirstWindow = pg.num <= FULL_TEXT_WINDOW;
    const inLastWindow = pg.num > total - FULL_TEXT_WINDOW;
    const isRequested = requestedSet.has(pg.num);
    const showFull = inFirstWindow || inLastWindow || isRequested || isSparse;

    if (showFull) {
      lines.push(`\n--- Page ${pg.num} of ${total} ---\n`);
      lines.push(trimmed || "[empty page]");
    }
  }

  return { text: lines.join("\n"), pageSummary };
}

/**
 * Determine which pages to render as images.
 * Priority: explicitly requested pages > sparse pages > first N.
 */
function selectPagesToRender(pageSummary, maxRender, requestedPages) {
  const limit = maxRender || MAX_RENDER_PAGES;
  const toRender = new Set();

  // 1. Always include explicitly requested pages
  if (requestedPages && requestedPages.length > 0) {
    for (const p of requestedPages) toRender.add(p);
  }

  // 2. Add sparse pages (likely diagrams/schematics/tables)
  if (toRender.size < limit) {
    for (const pg of pageSummary) {
      if (toRender.size >= limit) break;
      if (pg.sparse || pg.chars < SPARSE_PAGE_THRESHOLD) toRender.add(pg.page);
    }
  }

  // 3. Fill remaining slots with first pages (cover, TOC, etc.)
  if (toRender.size < limit) {
    for (let i = 1; i <= pageSummary.length && toRender.size < limit; i++) {
      toRender.add(i);
    }
  }

  return Array.from(toRender).sort((a, b) => a - b);
}

/**
 * Full PDF processing pipeline.
 *
 * @param {string} url - The PDF URL
 * @param {object} opts
 * @param {Function} opts.fetchWithRetry   - Retry-capable fetch
 * @param {string}   opts.userAgent        - User-Agent header
 * @param {number}   [opts.maxRenderPages] - Max pages to render as images (default: 5)
 * @param {boolean}  [opts.renderImages]   - Whether to render page images (default: true)
 * @param {number[]} [opts.pages]          - Specific pages to extract text + render images for
 * @returns {Promise<{text: string, numPages: number, info: object, pageImages: Array, pageSummary: Array}>}
 */
async function processPdf(url, opts) {
  const {
    fetchWithRetry,
    userAgent,
    maxRenderPages,
    renderImages = true,
    pages: requestedPages,
  } = opts;

  const { PDFParse } = getPDFParse();

  process.stderr.write(`[git-research-mcp] Downloading PDF: ${url}\n`);
  const pdfBuffer = await downloadPdf(url, fetchWithRetry, userAgent);

  if (pdfBuffer.length === 0) {
    throw new Error("PDF download returned empty content");
  }

  process.stderr.write(
    `[git-research-mcp] PDF downloaded (${(pdfBuffer.length / 1024).toFixed(0)} KB) — extracting text\n`,
  );

  const parser = new PDFParse({ data: pdfBuffer });

  // Extract per-page text (v2 returns pages as [{text, num}, ...])
  const textResult = await parser.getText();
  const pageArray = Array.isArray(textResult.pages) ? textResult.pages : [];
  const numPages = pageArray.length || textResult.total || 0;
  const totalChars = pageArray.reduce((s, p) => s + (p.text || "").length, 0);

  // Build text output — structured for large PDFs, flat for small ones
  let text;
  let pageSummary;

  if (requestedPages && requestedPages.length > 0) {
    // Targeted extraction: only return requested pages (full text)
    const requestedSet = new Set(requestedPages);
    const lines = [];
    pageSummary = [];
    for (const pg of pageArray) {
      const trimmed = (pg.text || "").trim();
      pageSummary.push({
        page: pg.num,
        chars: trimmed.length,
        sparse: trimmed.length < SPARSE_PAGE_THRESHOLD,
        heading: trimmed.split("\n")[0].slice(0, 120),
      });
      if (requestedSet.has(pg.num)) {
        lines.push(`\n--- Page ${pg.num} of ${numPages} ---\n`);
        lines.push(trimmed || "[empty page]");
      }
    }
    text = lines.join("\n");
  } else if (totalChars <= MAX_TEXT_CHARS) {
    // Small PDF — return everything with page markers
    const lines = [];
    pageSummary = [];
    for (const pg of pageArray) {
      const trimmed = (pg.text || "").trim();
      pageSummary.push({
        page: pg.num,
        chars: trimmed.length,
        sparse: trimmed.length < SPARSE_PAGE_THRESHOLD,
        heading: trimmed.split("\n")[0].slice(0, 120),
      });
      lines.push(`\n--- Page ${pg.num} of ${numPages} ---\n`);
      lines.push(trimmed || "[empty page]");
    }
    text = lines.join("\n");
  } else {
    // Large PDF — structured: full text for first/last pages, summaries for middle
    process.stderr.write(
      `[git-research-mcp] Large PDF (${totalChars} chars / ${numPages} pages) — using structured output\n`,
    );
    const structured = buildStructuredText(pageArray, requestedPages);
    text = structured.text;
    pageSummary = structured.pageSummary;
  }

  // Extract metadata
  let info = {};
  try {
    const infoResult = await parser.getInfo();
    info = infoResult.info || {};
  } catch {
    // Metadata extraction is non-critical
  }

  // Render pages as PNG images for visual analysis
  let pageImages = [];
  if (renderImages) {
    const renderList = selectPagesToRender(
      pageSummary,
      maxRenderPages,
      requestedPages,
    );
    if (renderList.length > 0) {
      try {
        process.stderr.write(
          `[git-research-mcp] Rendering PDF pages [${renderList.join(", ")}] as PNG\n`,
        );
        const ssResult = await parser.getScreenshot({
          scale: 1.5,
          partial: renderList,
          imageBuffer: true,
          imageDataUrl: false,
        });

        const tempDir = path.join(os.tmpdir(), `gsh-pdf-${Date.now()}`);
        fs.mkdirSync(tempDir, { recursive: true });

        for (let i = 0; i < ssResult.pages.length; i++) {
          const page = ssResult.pages[i];
          if (page && page.data) {
            const pageNum =
              typeof page.num === "number" && Number.isFinite(page.num)
                ? page.num
                : renderList[i] || i + 1;
            const imgPath = path.join(tempDir, `page-${pageNum}.png`);
            fs.writeFileSync(imgPath, page.data);
            pageImages.push({ page: pageNum, path: imgPath });
          }
        }
      } catch (err) {
        process.stderr.write(
          `[git-research-mcp] PDF page rendering failed: ${err.message}\n`,
        );
      }
    }
  }

  await parser.destroy();

  return { text, numPages, info, pageImages, pageSummary, totalChars };
}

/**
 * Format PDF extraction results for MCP text output.
 */
function formatPdfResult(url, result) {
  const urlPath = (() => {
    try {
      return path.basename(new URL(url).pathname);
    } catch {
      return url;
    }
  })();
  const lines = [
    `# PDF: ${result.info.Title || urlPath}`,
    `Source: ${url}`,
    `Pages: ${result.numPages} | Total text: ${result.totalChars} chars`,
  ];

  if (result.info.Author) lines.push(`Author: ${result.info.Author}`);
  if (result.info.Subject) lines.push(`Subject: ${result.info.Subject}`);

  // Page map for large PDFs — shows agent what each page contains
  if (result.pageSummary && result.numPages > FULL_TEXT_WINDOW * 2) {
    lines.push("", "## Page Map");
    // Group into sections for readability
    const skippedPages = [];
    for (const pg of result.pageSummary) {
      const inWindow =
        pg.page <= FULL_TEXT_WINDOW ||
        pg.page > result.numPages - FULL_TEXT_WINDOW;
      if (!inWindow && !pg.sparse) {
        skippedPages.push(pg);
      }
    }
    if (skippedPages.length > 0) {
      lines.push(
        `Pages ${FULL_TEXT_WINDOW + 1}-${result.numPages - FULL_TEXT_WINDOW} (${skippedPages.length} pages) summarized — full text for first/last ${FULL_TEXT_WINDOW}:`,
      );
      for (const pg of skippedPages) {
        lines.push(`  p${pg.page}: ${pg.chars} chars — ${pg.heading}`);
      }
      lines.push(
        "",
        `To get full text for specific pages, call scrape_webpage again with the same URL and pdf_pages: [page_numbers].`,
      );
    }
  }

  lines.push("");

  if (result.text && result.text.trim()) {
    lines.push("## Extracted Text", "", result.text);
  } else {
    lines.push(
      "## Text Extraction",
      "",
      "No text could be extracted — this PDF may be image-only (scanned document).",
    );
  }

  if (result.pageImages.length > 0) {
    lines.push(
      "",
      "## Page Images (for visual analysis)",
      "",
      `${result.pageImages.length} page(s) rendered as PNG images.`,
      "Use analyze_images to inspect diagrams, charts, schematics, or visual content:",
      "",
    );
    for (const img of result.pageImages) {
      lines.push(`  Page ${img.page}: ${img.path}`);
    }
  }

  return lines.join("\n");
}

module.exports = {
  processPdf,
  formatPdfResult,
  downloadPdf,
  MAX_RENDER_PAGES,
  MAX_TEXT_CHARS,
};
