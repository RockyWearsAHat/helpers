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
// Optional local reader-model endpoint (OpenAI-compatible). When set, the model
// is the extractor; otherwise the deterministic keyword extractor runs.
const MODEL_URL = arg("--model-url", null);

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

/** Meta/changelog prose that is not a code rule — dropped to cut noise. */
const NOISE = /\b(internal changes|stabilized|compatibility notes|release notes?|changelog|§ Version|this release|now stable)\b/i;

/**
 * High-confidence sections that documentation generators publish as explicit
 * contracts. A sentence sitting under one of these headers is a rule by
 * construction (e.g. rustdoc's `§ Panics`/`§ Safety`/`§ Errors`), so it is
 * labeled and weighted above loose keyword matches.
 */
const SECTIONS = [
  [/§\s*Safety\b/i, "footgun"],
  [/§\s*Panics\b/i, "bug"],
  [/§\s*Errors\b/i, "error"],
  [/\bdeprecated since\b|§\s*Deprecated\b/i, "deprecation"],
];

/**
 * Split readable text into sentences and keep those that state a rule. A
 * sentence is high-confidence (`strong`) only when it *itself* carries an
 * explicit doc-contract marker (rustdoc's `§ Safety`/`§ Panics`/`§ Errors`,
 * "undefined behavior", "deprecated since"); otherwise it must carry an
 * imperative/prohibitive signal word. Changelog/meta prose is filtered out.
 *
 * This is the extractor seam: it is deliberately conservative and keyword-based.
 * A learned reader (a local model — see `--extractor model`) can replace it for
 * far better recall/precision without touching the crawl or graph machinery.
 */
function extractSignals(text) {
  const sentences = text.split(/(?<=[.!?])\s+/);
  const hits = [];
  for (const s of sentences) {
    if (s.length < 25 || s.length > 320 || NOISE.test(s)) continue;
    const strong = SECTIONS.find(([re]) => re.test(s));
    if (strong) {
      hits.push({ label: strong[1], text: s.trim(), strong: true });
      continue;
    }
    const sig = SIGNALS.find(([re]) => re.test(s));
    if (sig) hits.push({ label: sig[1], text: s.trim(), strong: false });
  }
  return hits;
}

/**
 * The reader-model extractor (route B). When `--model-url` points at a local,
 * OpenAI-compatible chat endpoint (e.g. llama.cpp's server, Ollama, or your own
 * 1-bit runtime), each page is read by the model instead of the keyword
 * heuristic. The model is asked for strict JSON so its output is machine-usable
 * ("force specific output"); any failure falls back to [`extractSignals`] so a
 * crawl never breaks. The model itself is user-supplied/-trained — this is the
 * integration seam, not the model.
 */
const MODEL_PROMPT =
  "You read software documentation and extract durable CODE RULES. " +
  'Return ONLY a JSON array; each item: {"label":"avoid|prefer|deprecation|footgun|bug|error","text":"<the rule in one sentence>","strong":true|false}. ' +
  "Include only genuine, reusable rules / anti-patterns / footguns / bugs — skip prose, navigation, and changelog noise. If none, return [].";

/** Pull the first JSON array out of a model response (tolerates wrapper prose). */
function parseRules(content) {
  const start = content.indexOf("[");
  const end = content.lastIndexOf("]");
  if (start < 0 || end <= start) return null;
  try {
    const arr = JSON.parse(content.slice(start, end + 1));
    if (!Array.isArray(arr)) return null;
    return arr
      .filter((r) => r && typeof r.text === "string" && r.text.length > 10)
      .map((r) => ({ label: String(r.label || "avoid"), text: r.text.trim(), strong: !!r.strong }));
  } catch {
    return null;
  }
}

/** Ask the local model to extract rules from `text`; null on any failure. */
async function modelExtract(text, modelUrl) {
  try {
    const res = await fetch(modelUrl, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        messages: [
          { role: "system", content: MODEL_PROMPT },
          { role: "user", content: text.slice(0, 6000) },
        ],
        temperature: 0,
        stream: false,
      }),
    });
    if (!res.ok) return null;
    const data = await res.json();
    const content = data?.choices?.[0]?.message?.content ?? data?.content ?? "";
    return parseRules(content);
  } catch {
    return null;
  }
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
  const corpus = []; // {url, label, text, code} — signal sentences (extraction set)
  const pages = []; // {url, text} — full page prose (LM pretraining corpus)
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
    // Reader-model when configured (route B), else the deterministic fallback.
    const signals = MODEL_URL ? (await modelExtract(text, MODEL_URL)) ?? extractSignals(text) : extractSignals(text);
    nodes.push({ url, depth, bytes: html.length, signalCount: signals.length, codeBlocks: code.length });
    if (text.length > 200) pages.push({ url, text });
    for (const sig of signals) {
      corpus.push({ url, label: sig.label, text: sig.text, strong: sig.strong, code: code[0] || "" });
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
    const cur = byKey.get(key) || { label: c.label, text: c.text, urls: new Set(), code: c.code, count: 0, strong: false };
    cur.count++;
    cur.urls.add(c.url);
    cur.strong = cur.strong || !!c.strong;
    if (!cur.code && c.code) cur.code = c.code;
    byKey.set(key, cur);
  }
  const rules = [...byKey.values()]
    .map((r) => ({
      label: r.label,
      text: r.text,
      code: r.code,
      strong: r.strong,
      occurrences: r.count,
      sources: [...r.urls].slice(0, 5),
      // Explicit doc-contract sections (Safety/Panics/Errors/Deprecated) are
      // rules by construction, so they outrank loose keyword matches.
      score: (WEIGHT[r.label] || 1) * Math.log2(1 + r.count) * (r.strong ? 2 : 1),
    }))
    .sort((a, b) => b.score - a.score);

  const base = join(OUT, host);
  writeFileSync(`${base}.graph.json`, JSON.stringify({ host, seededFrom: start, pages: nodes.length, links: edges.length, nodes, edges }, null, 2) + "\n");
  writeFileSync(`${base}.rules.json`, JSON.stringify({ host, fetchedAt: new Date().toISOString(), ruleCount: rules.length, rules }, null, 2) + "\n");
  writeFileSync(`${base}.corpus.jsonl`, corpus.map((c) => JSON.stringify(c)).join("\n") + "\n");
  // Full page prose — the predictive-coding (next-token) pretraining corpus.
  writeFileSync(`${base}.text.jsonl`, pages.map((p) => JSON.stringify(p)).join("\n") + "\n");

  const words = pages.reduce((n, p) => n + p.text.split(/\s+/).length, 0);
  console.log(`[crawl] ${host}: ${nodes.length} pages, ${edges.length} links, ${corpus.length} signal chunks, ${rules.length} ranked rules, ~${words} words of prose`);
  console.log(`[crawl] wrote ${base}.{graph.json,rules.json,corpus.jsonl,text.jsonl}`);
}

main().catch((e) => {
  console.error(`[crawl] ${e.stack || e}`);
  process.exit(1);
});
