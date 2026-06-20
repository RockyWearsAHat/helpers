#!/usr/bin/env node
// crawl-docs.mjs — the "backrub" documentation crawler.
//
// Seed it with a HOSTNAME (e.g. doc.rust-lang.org). It crawls in-domain links
// breadth-first, building a link graph and, from each page, deterministically
// extracting rule-candidates: code examples plus the prose that carries an
// imperative/prohibitive/deprecation signal ("avoid", "prefer", "never",
// "undefined behavior", "deprecated", "unsound", …). Output is three artifacts:
//
//   crawl-index/<host>.graph.json   nodes (pages) + edges (links) + per-page signals
//   crawl-index/<host>.rules.json   synthesized, deduped rule-candidates (ranked)
//   crawl-index/<host>.corpus.jsonl one chunk per line: text + weak label (training set)
//
// It is bounded (page budget, depth, polite delay) so a demo run is quick; raise
// --max for the deep pass. Deterministic: same crawl -> same artifacts. The
// extractor is a clean seam — a smarter reader (a local model) can replace
// `extractSignals` later without touching the crawl/graph machinery.
//
//   usage: node scripts/crawl-docs.mjs --seed <hostname> [--max N] [--depth D] [--delay MS]

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = join(ROOT, "crawl-index");
const UA = "helpers-doc-crawler/0.1 (+https://github.com/RockyWearsAHat/helpers)";

/** Read a `--flag value` pair from argv, or a default. */
function arg(flag, def) {
  const i = process.argv.indexOf(flag);
  return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : def;
}

const SEED = arg("--seed", null);
const MAX_PAGES = Number(arg("--max", "25"));
const MAX_DEPTH = Number(arg("--depth", "3"));
const DELAY_MS = Number(arg("--delay", "300"));

/** Prose markers that flag a sentence as a rule / anti-pattern / footgun. */
const SIGNALS = [
  // [regex, weak label]
  [/\bdeprecat(ed|ion)\b/i, "deprecation"],
  [/\bunsound\b|\bundefined behavior\b|\bdata race\b/i, "footgun"],
  [/\b(avoid|never|do not|don't|must not|should not)\b/i, "avoid"],
  [/\b(prefer|always|should|recommended|instead of|use\s+\w+\s+instead)\b/i, "prefer"],
  [/\b(common (mistake|bug|pitfall)|gotcha|footgun|easy to (forget|misuse))\b/i, "bug"],
  [/\b(panic|crash|leak|overflow|out of bounds)\b/i, "bug"],
];

/** Sleep `ms` between fetches to stay polite to the host. */
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Strip tags to readable text (crude, dependency-free). */
function htmlToText(html) {
  return html
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/\s+/g, " ")
    .trim();
}

/** Extract the text of each `<pre>`/`<code>` block (the canonical examples). */
function codeBlocks(html) {
  const out = [];
  const re = /<pre[\s\S]*?>([\s\S]*?)<\/pre>/gi;
  let m;
  while ((m = re.exec(html)) !== null) {
    const code = htmlToText(m[1]);
    if (code.length > 8) out.push(code.slice(0, 400));
  }
  return out.slice(0, 12);
}

/** In-domain absolute URLs linked from `html` (fragments/queries dropped). */
function links(html, pageUrl, host) {
  const out = new Set();
  const re = /href\s*=\s*["']([^"']+)["']/gi;
  let m;
  while ((m = re.exec(html)) !== null) {
    let href = m[1];
    if (href.startsWith("#") || href.startsWith("mailto:") || href.startsWith("javascript:")) continue;
    let url;
    try {
      url = new URL(href, pageUrl);
    } catch {
      continue;
    }
    if (url.hostname !== host) continue;
    url.hash = "";
    url.search = "";
    if (!/\.(html?)?$/i.test(url.pathname) && !url.pathname.endsWith("/") && url.pathname.includes(".")) {
      // skip obvious non-page assets (.png/.css/.js/…)
      if (/\.(png|jpe?g|gif|svg|css|js|json|woff2?|ico|pdf|zip)$/i.test(url.pathname)) continue;
    }
    out.add(url.toString());
  }
  return [...out];
}

/** Split readable text into sentences and keep those carrying a rule signal. */
function extractSignals(text) {
  const sentences = text.split(/(?<=[.!?])\s+/);
  const hits = [];
  for (const s of sentences) {
    if (s.length < 25 || s.length > 320) continue;
    for (const [re, label] of SIGNALS) {
      if (re.test(s)) {
        hits.push({ label, text: s.trim() });
        break;
      }
    }
  }
  return hits;
}

async function main() {
  if (!SEED) {
    console.error("usage: node scripts/crawl-docs.mjs --seed <hostname> [--max N] [--depth D]");
    process.exit(2);
  }
  const host = SEED.replace(/^https?:\/\//, "").replace(/\/.*$/, "");
  mkdirSync(OUT, { recursive: true });

  const start = `https://${host}/`;
  const visited = new Set();
  const queue = [{ url: start, depth: 0 }];
  const nodes = [];
  const edges = [];
  const corpus = []; // {url, label, text, code}
  let fetched = 0;

  while (queue.length && fetched < MAX_PAGES) {
    const { url, depth } = queue.shift();
    if (visited.has(url) || depth > MAX_DEPTH) continue;
    visited.add(url);

    let html;
    try {
      const res = await fetch(url, { headers: { "user-agent": UA } });
      if (!res.ok || !/text\/html/i.test(res.headers.get("content-type") || "")) continue;
      html = await res.text();
    } catch {
      continue;
    }
    fetched++;

    const text = htmlToText(html);
    const code = codeBlocks(html);
    const signals = extractSignals(text);
    nodes.push({ url, depth, bytes: html.length, signalCount: signals.length, codeBlocks: code.length });
    for (const sig of signals) {
      corpus.push({ url, label: sig.label, text: sig.text, code: code[0] || "" });
    }

    const outLinks = links(html, url, host);
    for (const l of outLinks) {
      edges.push([url, l]);
      if (!visited.has(l)) queue.push({ url: l, depth: depth + 1 });
    }
    await sleep(DELAY_MS);
  }

  // Synthesize ranked rule-candidates: dedupe by normalized text, rank by signal
  // weight (deprecation/footgun highest) and how often the idea recurs.
  const WEIGHT = { deprecation: 5, footgun: 5, bug: 4, avoid: 3, prefer: 2 };
  const byKey = new Map();
  for (const c of corpus) {
    const key = c.text.toLowerCase().replace(/[^a-z0-9 ]/g, "").slice(0, 80);
    const cur = byKey.get(key) || { label: c.label, text: c.text, urls: new Set(), code: c.code, count: 0 };
    cur.count++;
    cur.urls.add(c.url);
    if (!cur.code && c.code) cur.code = c.code;
    byKey.set(key, cur);
  }
  const rules = [...byKey.values()]
    .map((r) => ({
      label: r.label,
      text: r.text,
      code: r.code,
      occurrences: r.count,
      sources: [...r.urls].slice(0, 5),
      score: (WEIGHT[r.label] || 1) * Math.log2(1 + r.count),
    }))
    .sort((a, b) => b.score - a.score);

  const base = join(OUT, host);
  writeFileSync(`${base}.graph.json`, JSON.stringify({ host, seededFrom: start, pages: nodes.length, links: edges.length, nodes, edges }, null, 2) + "\n");
  writeFileSync(`${base}.rules.json`, JSON.stringify({ host, fetchedAt: new Date().toISOString(), ruleCount: rules.length, rules }, null, 2) + "\n");
  writeFileSync(`${base}.corpus.jsonl`, corpus.map((c) => JSON.stringify(c)).join("\n") + "\n");

  console.log(`[crawl] ${host}: ${nodes.length} pages, ${edges.length} links, ${corpus.length} signal chunks, ${rules.length} ranked rules`);
  console.log(`[crawl] wrote ${base}.{graph.json,rules.json,corpus.jsonl}`);
}

main().catch((e) => {
  console.error(`[crawl] ${e.stack || e}`);
  process.exit(1);
});
