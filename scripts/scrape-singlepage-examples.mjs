#!/usr/bin/env node
// scrape-singlepage-examples.mjs — close the corpus gap for tools whose docs put
// every rule on ONE page with anchors (staticcheck, markdownlint), instead of one
// page per rule. Fetch the page once, slice each rule's section, pull its first
// code example into exampleBad, then recompute the index checksum so it stays valid.
//
//   node scripts/scrape-singlepage-examples.mjs

import { readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const UA = "helpers-lint-indexer/0.2";

async function fetchText(url) {
  const res = await fetch(url, { headers: { "user-agent": UA } });
  if (!res.ok) throw new Error(`GET ${url} -> ${res.status}`);
  return res.text();
}

function decodeEntities(s) {
  return s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, "&");
}

// canonical checksum — must match build-lint-index.mjs / the Rust verifier.
function canonicalRule(r) {
  const o = {};
  for (const k of ["id", "category", "severity", "description", "exampleBad", "exampleGood", "source"]) {
    if (r[k] !== undefined) o[k] = r[k];
  }
  return o;
}
function rulesChecksum(rules) {
  const sorted = [...rules].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return "sha256:" + createHash("sha256").update(JSON.stringify(sorted.map(canonicalRule))).digest("hex");
}

/** Markdown: split Rules.md by `## ` headers; first fenced block after the header
 * that names the rule is its example. */
async function markdownlint(idx) {
  const md = await fetchText("https://raw.githubusercontent.com/DavidAnson/markdownlint/main/doc/Rules.md");
  const byId = new Map(idx.rules.map((r) => [r.id.toLowerCase(), r]));
  let got = 0;
  const sections = md.split(/\n## /);
  for (const sec of sections) {
    const m = sec.match(/^[`*]*([A-Z]+\d+)\b/i);
    if (!m) continue;
    const rule = byId.get(m[1].toLowerCase());
    if (!rule) continue;
    const fence = sec.match(/```[a-z]*\n([\s\S]*?)```/i);
    if (fence) {
      const code = fence[1].trim();
      if (code.length >= 3) {
        rule.exampleBad = code;
        got++;
      }
    }
  }
  return got;
}

/** staticcheck: one HTML page; each check is an `id="CODE"` anchor. Pull the first
 * <pre>/<code> block within the check's section. */
async function staticcheck(idx) {
  const html = await fetchText("https://staticcheck.dev/docs/checks/");
  const byId = new Map(idx.rules.map((r) => [r.id, r]));
  let got = 0;
  // split on anchors like id="SA1000"
  const parts = html.split(/id="([A-Z]+\d+)"/);
  for (let i = 1; i < parts.length; i += 2) {
    const id = parts[i];
    const body = parts[i + 1] || "";
    const rule = byId.get(id);
    if (!rule) continue;
    const m = body.match(/<pre[^>]*>([\s\S]*?)<\/pre>/) || body.match(/<code[^>]*>([\s\S]*?)<\/code>/);
    if (m) {
      const code = decodeEntities(m[1].replace(/<[^>]+>/g, "")).trim();
      if (code.length >= 5 && /[A-Za-z]/.test(code)) {
        rule.exampleBad = code;
        got++;
      }
    }
  }
  return got;
}

for (const [tool, fn] of [["markdownlint", markdownlint], ["staticcheck", staticcheck]]) {
  const path = join(ROOT, "lint-index", `${tool}.json`);
  const idx = JSON.parse(readFileSync(path, "utf8"));
  try {
    const got = await fn(idx);
    idx.checksum = rulesChecksum(idx.rules);
    idx.ruleCount = idx.rules.length;
    writeFileSync(path, JSON.stringify(idx, null, 0));
    console.log(`${tool}: attached ${got} examples; checksum recomputed`);
  } catch (e) {
    console.error(`${tool}: ${e.message}`);
  }
}
