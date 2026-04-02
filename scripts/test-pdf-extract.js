#!/usr/bin/env node
"use strict";

const assert = require("assert");

const {
  MAX_RENDER_PAGES,
  createPdfParser,
  selectPagesToRender,
} = require("../lib/mcp-pdf-extract");

async function main() {
  const pageSummary = Array.from({ length: 8 }, (_, index) => ({
    page: index + 1,
    chars: index === 4 ? 10 : 100,
    sparse: index === 4,
  }));

  assert.deepStrictEqual(
    selectPagesToRender(pageSummary, 3, [7, 2, 5, 1]),
    [2, 5, 7],
    "Requested pages are capped to the render limit",
  );

  class ModernPDFParse {
    constructor(options) {
      this.options = options;
    }

    getText() {
      return { pages: [] };
    }
  }

  const classParser = createPdfParser(Buffer.from("pdf"), {
    PDFParse: ModernPDFParse,
  });
  assert.strictEqual(classParser.mode, "class");
  assert.ok(classParser.parser instanceof ModernPDFParse);
  assert.strictEqual(classParser.parser.options.data.toString(), "pdf");

  const legacyCalls = [];
  const legacyParser = createPdfParser(
    Buffer.from("legacy"),
    Object.assign(
      (buffer) => {
        legacyCalls.push(buffer.toString());
        return { text: "ok" };
      },
      { PDFParse: () => ({}) },
    ),
  );
  assert.strictEqual(legacyParser.mode, "legacy");
  await legacyParser.parse(Buffer.from("legacy-run"));
  assert.deepStrictEqual(legacyCalls, ["legacy-run"]);

  assert.strictEqual(MAX_RENDER_PAGES, 5);
  console.log("ok");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
