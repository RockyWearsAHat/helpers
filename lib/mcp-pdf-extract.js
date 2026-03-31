"use strict";
// lib/mcp-pdf-extract.js — PDF text extraction + page rendering for visual analysis
//
// Downloads a PDF, extracts text via pdf-parse, and optionally renders pages
// to PNG images via Puppeteer so agents can pipe them to analyze_images.
// Follows the same pattern as vision-tool/lib/video-analysis.js:
//   - text extraction = transcript
//   - page images = frames
//   - agent decides whether to call analyze_images on the images

const fs = require("fs");
const fsPromises = require("fs/promises");
const os = require("os");
const path = require("path");

// Max pages to render as images (controls vision API cost)
const MAX_RENDER_PAGES = 5;
// Max total text chars to return (prevents massive PDFs from blowing context)
const MAX_TEXT_CHARS = 80000;

let _pdfParse = null;

function getPdfParse() {
  if (_pdfParse) return _pdfParse;
  try {
    _pdfParse = require("pdf-parse");
  } catch {
    const globalRoot = path.join(
      process.env.HOME || "",
      ".nvm/versions/node",
      process.version,
      "lib/node_modules",
    );
    _pdfParse = require(path.join(globalRoot, "pdf-parse"));
  }
  return _pdfParse;
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
  const arrayBuf = await response.arrayBuffer();
  return Buffer.from(arrayBuf);
}

/**
 * Extract text from a PDF buffer using pdf-parse.
 * Returns { text, numPages, info }.
 */
async function extractPdfText(pdfBuffer) {
  const pdfParse = getPdfParse();
  const result = await pdfParse(pdfBuffer);
  let text = result.text || "";
  if (text.length > MAX_TEXT_CHARS) {
    text = text.slice(0, MAX_TEXT_CHARS) + "\n\n[… truncated — PDF too large]";
  }
  return {
    text,
    numPages: result.numpages || 0,
    info: result.info || {},
  };
}

/**
 * Render PDF pages to PNG images via Puppeteer's built-in PDF viewer.
 * Returns an array of { page, path } objects for the rendered images.
 *
 * @param {Buffer} pdfBuffer - The PDF content
 * @param {number} numPages  - Total page count
 * @param {number} maxPages  - Max pages to render (cost control)
 * @param {Function} getBrowser - Async function that returns a Puppeteer browser
 * @returns {Promise<Array<{page: number, path: string}>>}
 */
async function renderPdfPages(pdfBuffer, numPages, maxPages, getBrowser) {
  const pagesToRender = Math.min(numPages, maxPages || MAX_RENDER_PAGES);
  if (pagesToRender === 0) return [];

  const tempDir = path.join(os.tmpdir(), `gsh-pdf-${Date.now()}`);
  fs.mkdirSync(tempDir, { recursive: true });

  // Write PDF to temp file for Puppeteer to open
  const pdfPath = path.join(tempDir, "source.pdf");
  fs.writeFileSync(pdfPath, pdfBuffer);

  const browser = await getBrowser();
  const images = [];

  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1200, height: 1600 });

    // Navigate to the PDF via file:// URL
    const fileUrl = `file://${pdfPath}`;
    await page.goto(fileUrl, { waitUntil: "networkidle2", timeout: 30000 });

    // Puppeteer's Chrome viewer renders PDFs inline — take a screenshot of
    // the full page for each PDF page. For multi-page PDFs, we scroll or
    // use the page selector. However, Chrome's PDF viewer is an embedded
    // plugin that's hard to programmatically paginate.
    //
    // Simpler approach: use pdf.js-style rendering by injecting a data URL,
    // or just screenshot the viewport for the first page and note the rest.
    //
    // Most effective: render each page by converting PDF pages to individual
    // canvases. But the simplest cross-platform way is to screenshot the
    // Chrome PDF viewer at different scroll positions.

    // For reliability, we take viewport screenshots. Chrome's PDF viewer
    // renders ~1 page per viewport height.
    for (let i = 0; i < pagesToRender; i++) {
      const imgPath = path.join(tempDir, `page-${i + 1}.png`);

      if (i > 0) {
        // Scroll to next page — Chrome PDF viewer page height ≈ viewport
        await page.evaluate((pageNum) => {
          // Chrome PDF viewer embed exposes scrolling on the main element
          const container =
            document.querySelector("#viewer") ||
            document.querySelector("embed") ||
            document.documentElement;
          if (container.scrollTo) {
            container.scrollTo(0, pageNum * window.innerHeight);
          } else {
            window.scrollTo(0, pageNum * window.innerHeight);
          }
        }, i);
        // Wait for render
        await new Promise((r) => setTimeout(r, 500));
      }

      await page.screenshot({ path: imgPath, type: "png" });
      images.push({ page: i + 1, path: imgPath });
    }

    await page.close();
  } catch (renderErr) {
    process.stderr.write(
      `[git-research-mcp] PDF page rendering failed: ${renderErr.message}\n`,
    );
    // Text extraction still works — images are a bonus
  }

  // Clean up the source PDF but keep the images
  try {
    fs.unlinkSync(pdfPath);
  } catch {
    // Non-critical
  }

  return images;
}

/**
 * Full PDF processing pipeline.
 *
 * @param {string} url - The PDF URL
 * @param {object} opts
 * @param {Function} opts.fetchWithRetry - Retry-capable fetch
 * @param {string}   opts.userAgent      - User-Agent header
 * @param {Function} opts.getBrowser     - Returns Puppeteer browser instance
 * @param {number}   [opts.maxRenderPages] - Max pages to render as images
 * @param {boolean}  [opts.renderImages]   - Whether to render page images (default: true)
 * @returns {Promise<{text: string, numPages: number, info: object, pageImages: Array}>}
 */
async function processPdf(url, opts) {
  const {
    fetchWithRetry,
    userAgent,
    getBrowser,
    maxRenderPages,
    renderImages = true,
  } = opts;

  process.stderr.write(`[git-research-mcp] Downloading PDF: ${url}\n`);
  const pdfBuffer = await downloadPdf(url, fetchWithRetry, userAgent);

  if (pdfBuffer.length === 0) {
    throw new Error("PDF download returned empty content");
  }

  process.stderr.write(
    `[git-research-mcp] PDF downloaded (${(pdfBuffer.length / 1024).toFixed(0)} KB) — extracting text\n`,
  );
  const { text, numPages, info } = await extractPdfText(pdfBuffer);

  let pageImages = [];
  if (renderImages && getBrowser) {
    try {
      process.stderr.write(
        `[git-research-mcp] Rendering up to ${maxRenderPages || MAX_RENDER_PAGES} PDF pages as images\n`,
      );
      pageImages = await renderPdfPages(
        pdfBuffer,
        numPages,
        maxRenderPages || MAX_RENDER_PAGES,
        getBrowser,
      );
    } catch (err) {
      process.stderr.write(
        `[git-research-mcp] PDF image rendering failed: ${err.message}\n`,
      );
    }
  }

  return { text, numPages, info, pageImages };
}

/**
 * Format PDF extraction results for MCP text output.
 */
function formatPdfResult(url, result) {
  const lines = [
    `# PDF: ${result.info.Title || path.basename(new URL(url).pathname)}`,
    `Source: ${url}`,
    `Pages: ${result.numPages}`,
  ];

  if (result.info.Author) lines.push(`Author: ${result.info.Author}`);
  if (result.info.Subject) lines.push(`Subject: ${result.info.Subject}`);

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
  extractPdfText,
  downloadPdf,
  renderPdfPages,
  MAX_RENDER_PAGES,
  MAX_TEXT_CHARS,
};
