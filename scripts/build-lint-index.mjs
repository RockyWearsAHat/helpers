#!/usr/bin/env node
// build-lint-index.mjs — the "backrub" lint indexer.
//
// Derives a normalized rule catalog *directly from official, version-matched
// linter documentation* — no hand-authored rules, no running the external tool.
// For each supported tool it locates a structured official source (a published
// `lints.json`, or the canonical rules table/index on the project's docs site),
// extracts a normalized rule (id, category, severity, description, and a direct
// source URL), and emits a packed index file that conforms to lint-index/SCHEMA.md.
//
// The emitted file is byte-stable for a given source: rules are sorted by id and
// the file carries a `checksum` = "sha256:" + sha256(canonical JSON of the rules
// array, no whitespace). The fast-path resolver compares that checksum to decide
// "packed index is current" without refetching.
//
//   usage: node scripts/build-lint-index.mjs [--tool <name>|--all] [--out DIR]
//
// Tools (official, structured sources only):
//   - clippy      rust   rust-lang.github.io/rust-clippy/rust-<version>/lints.json (quick-fetch JSON)
//   - ruff        python docs.astral.sh/ruff/rules                                 (quick-fetch table)
//   - eslint      js     eslint.org/docs/latest/rules                              (quick-fetch index)
//   - staticcheck go     staticcheck.dev/docs/checks                               (quick-fetch table)

import { createHash } from "node:crypto";
import { execSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = outArg() ?? join(ROOT, "lint-index");
const UA = "helpers-lint-indexer/0.2 (+https://github.com/RockyWearsAHat/helpers)";

/** Parse `--out DIR` from argv, or return null for the default. */
function outArg() {
  const i = process.argv.indexOf("--out");
  return i >= 0 ? process.argv[i + 1] : null;
}

/**
 * The set of tools to build, from `--tool <name>` (repeatable) or `--all`.
 * Returns null to mean "default: build every tool whose source is reachable".
 */
function selectedTools() {
  if (process.argv.includes("--all")) return null;
  const tools = [];
  for (let i = 0; i < process.argv.length; i++) {
    if (process.argv[i] === "--tool" && process.argv[i + 1]) tools.push(process.argv[i + 1]);
  }
  return tools.length ? tools : null;
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/** Fetch JSON, throwing with context on a non-200 so failures are never silent. */
async function fetchJson(url) {
  const res = await fetch(url, { headers: { "user-agent": UA } });
  if (!res.ok) throw new Error(`GET ${url} -> ${res.status}`);
  return res.json();
}

/** Fetch text (HTML), throwing with context on a non-200. */
async function fetchText(url) {
  const res = await fetch(url, { headers: { "user-agent": UA } });
  if (!res.ok) throw new Error(`GET ${url} -> ${res.status}`);
  return res.text();
}

/** Strip HTML tags to readable text and decode the common entities. */
function htmlToText(html) {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&#39;|&apos;/g, "'")
    .replace(/&quot;/g, '"')
    .replace(/\s+/g, " ")
    .trim();
}

/** Compare two "X.Y.Z" versions: negative if a<b, 0 if equal, positive if a>b. */
function semverCmp(a, b) {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    if ((pa[i] || 0) !== (pb[i] || 0)) return (pa[i] || 0) - (pb[i] || 0);
  }
  return 0;
}

/**
 * Canonical-serialize a single rule: keys in a fixed order so the JSON is
 * byte-stable across machines regardless of object insertion order. Only the
 * schema's rule fields participate in the checksum.
 */
function canonicalRule(rule) {
  const ordered = {};
  for (const k of ["id", "category", "severity", "description", "exampleBad", "exampleGood", "source"]) {
    if (rule[k] !== undefined) ordered[k] = rule[k];
  }
  return ordered;
}

/**
 * The packed-index `checksum` per SCHEMA.md: "sha256:" + sha256 of the canonical
 * JSON of the rules array, sorted by id, with no whitespace. Deterministic, so
 * committing and pulling the index is reproducible.
 */
function rulesChecksum(rules) {
  const sorted = [...rules].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  const canonical = JSON.stringify(sorted.map(canonicalRule));
  return "sha256:" + createHash("sha256").update(canonical).digest("hex");
}

/**
 * Assemble a schema-conformant index object: rules are sorted by id, the
 * checksum is computed over them, and the standard provenance fields are set.
 */
function packIndex({ tool, language, toolchainVersion, docsVersion, source, docsBase, rules }) {
  const sorted = [...rules].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return {
    tool,
    language,
    toolchainVersion: toolchainVersion ?? null,
    docsVersion,
    source,
    docsBase,
    fetchedAt: new Date().toISOString(),
    checksum: rulesChecksum(sorted),
    ruleCount: sorted.length,
    rules: sorted,
  };
}

// ---------------------------------------------------------------------------
// clippy (rust) — quick-fetch from the published machine-readable lints.json
// ---------------------------------------------------------------------------

/** The Rust toolchain version in use (e.g. "1.95.0"), or null if rustc absent. */
function rustVersion() {
  try {
    const out = execSync("rustc --version", { encoding: "utf8" });
    const m = out.match(/\b(\d+\.\d+\.\d+)\b/);
    return m ? m[1] : null;
  } catch {
    return null;
  }
}

/**
 * Resolve the Clippy docs version whose machine-readable `lints.json` we fetch.
 * We read the official `versions.json` manifest and pick the newest published
 * numbered version that is **≤** the toolchain the project uses, falling back to
 * the newest published overall when the toolchain is newer than anything
 * published. Each candidate's `lints.json` is probed newest-first so a version
 * dir without the data file is skipped rather than failing the build.
 */
async function resolveClippyVersion(toolchainVersion) {
  const manifest = await fetchJson("https://rust-lang.github.io/rust-clippy/versions.json");
  const numbered = manifest
    .filter((v) => /^rust-\d+\.\d+\.\d+$/.test(v))
    .map((v) => v.slice("rust-".length))
    .sort(semverCmp);
  if (numbered.length === 0) return null;
  const le = toolchainVersion
    ? numbered.filter((v) => semverCmp(v, toolchainVersion) <= 0)
    : numbered;
  const ordered = (le.length ? le : numbered).slice().reverse(); // newest first
  for (const v of ordered) {
    const res = await fetch(`https://rust-lang.github.io/rust-clippy/rust-${v}/lints.json`, {
      headers: { "user-agent": UA },
    });
    if (res.ok) return v;
  }
  return null;
}

/** Pull the first fenced ```rust block out of a Clippy `docs` markdown body that
 * follows the given header (e.g. "Example", "Use instead"). "" when absent. */
function codeAfter(docs, header) {
  const at = docs.indexOf(`### ${header}`);
  if (at < 0) return "";
  const fence = docs.indexOf("```", at);
  if (fence < 0) return "";
  const start = docs.indexOf("\n", fence);
  const end = docs.indexOf("```", start + 1);
  if (start < 0 || end < 0) return "";
  return docs.slice(start + 1, end).trim();
}

/** The prose under a `### Header` up to the next header (one-line, trimmed). */
function sectionText(docs, header) {
  const at = docs.indexOf(`### ${header}`);
  if (at < 0) return "";
  const start = docs.indexOf("\n", at);
  const next = docs.indexOf("\n### ", start + 1);
  const body = docs.slice(start + 1, next < 0 ? undefined : next);
  return body.replace(/\s+/g, " ").trim();
}

/** Clippy lint group -> our normalized CS-principle category. */
const CLIPPY_GROUP_CATEGORY = {
  correctness: "correctness",
  suspicious: "correctness",
  complexity: "complexity",
  perf: "performance",
};
/** Clippy level -> our severity (high|medium|low). */
const CLIPPY_LEVEL_SEVERITY = { deny: "high", warn: "medium", allow: "low" };
/** Groups that carry CS-principle substance (skip pedantic/nursery/restriction/cargo/style). */
const CLIPPY_KEEP_GROUPS = new Set(["correctness", "suspicious", "complexity", "perf"]);

/**
 * Build the Rust/Clippy index from the official lints database. `docsVersion`
 * is the published Clippy version we read; `toolchain` is the version the
 * project actually uses (recorded so a later refetch can re-match).
 */
async function buildClippy(toolchain) {
  const docsVersion = await resolveClippyVersion(toolchain);
  if (!docsVersion) throw new Error("no published Clippy lints.json found");
  const base = `https://rust-lang.github.io/rust-clippy/rust-${docsVersion}`;
  const lints = await fetchJson(`${base}/lints.json`);
  const rules = [];
  for (const lint of lints) {
    if (!CLIPPY_KEEP_GROUPS.has(lint.group) || lint.level === "allow") continue;
    const docs = lint.docs || "";
    const whatItDoes = sectionText(docs, "What it does");
    const whyBad = sectionText(docs, "Why is this bad?") || sectionText(docs, "Why is this bad");
    rules.push({
      id: lint.id,
      category: CLIPPY_GROUP_CATEGORY[lint.group] ?? "correctness",
      severity: CLIPPY_LEVEL_SEVERITY[lint.level] ?? "medium",
      description: [whatItDoes, whyBad].filter(Boolean).join(" — ") || lint.id,
      exampleBad: codeAfter(docs, "Example"),
      exampleGood: codeAfter(docs, "Use instead"),
      source: `https://rust-lang.github.io/rust-clippy/master/index.html#${lint.id}`,
    });
  }
  return packIndex({
    tool: "clippy",
    language: "rust",
    toolchainVersion: toolchain,
    docsVersion,
    source: "rust-clippy",
    docsBase: base,
    rules,
  });
}

// ---------------------------------------------------------------------------
// ruff (python) — quick-fetch from the official rules table
// ---------------------------------------------------------------------------

/**
 * Map a ruff rule code prefix (e.g. "S", "B", "E", "F") to a normalized
 * category. ruff groups rules by linter; we collapse those into CS categories.
 */
function ruffCategory(code) {
  const prefix = code.match(/^[A-Z]+/)?.[0] ?? "";
  if (/^(S|BLE|ASYNC|DTZ|G)$/.test(prefix)) return "security";
  if (/^(F|B|PLE|RUF|TRY|PYI|SLOT|PT|RSE|RET)$/.test(prefix)) return "correctness";
  if (/^(C4|C90|SIM|PIE|PERF|FURB)$/.test(prefix)) return "complexity";
  if (/^(N|D|Q|ERA|COM|ISC|ICN|UP|E|W|I|A|ANN)$/.test(prefix)) return "style";
  return "correctness";
}

/** ruff prefix -> best-effort severity; security and likely-bug codes rank high. */
function ruffSeverity(code) {
  const prefix = code.match(/^[A-Z]+/)?.[0] ?? "";
  if (/^(S|BLE|F8|E9|PLE)$/.test(prefix) || /^F/.test(prefix)) return "high";
  if (/^(E|W|D|N|Q|ERA|COM|I|UP)$/.test(prefix)) return "low";
  return "medium";
}

/**
 * Build the Python/Ruff index from the official rules table. The page is a
 * sequence of `<table>` blocks, each preceded by an `<h2>` naming the linter
 * group. We parse the `Code | Name | Message` rows directly; a "🧪 preview"
 * marker in the status column is recorded but does not exclude the rule.
 */
async function buildRuff() {
  const html = await fetchText("https://docs.astral.sh/ruff/rules/");
  const docsVersion = await ruffDocsVersion(html);
  // Each rule row: <td id="CODE">CODE</td><td><a href="name/">name</a></td><td>message</td>...
  const rowRe =
    /<tr>\s*<td id="([^"]+)">[^<]*<\/td>\s*<td><a href="([^"]+)">([^<]+)<\/a><\/td>\s*<td>([\s\S]*?)<\/td>/g;
  const rules = [];
  const seen = new Set();
  let m;
  while ((m = rowRe.exec(html)) !== null) {
    const [, code, href, name, messageHtml] = m;
    if (seen.has(code)) continue; // a code can appear under multiple group tables
    seen.add(code);
    const description = htmlToText(messageHtml);
    rules.push({
      id: code,
      name,
      category: ruffCategory(code),
      severity: ruffSeverity(code),
      description: description || name.replace(/-/g, " "),
      source: `https://docs.astral.sh/ruff/rules/${href.replace(/\/$/, "")}/`,
    });
  }
  if (rules.length === 0) throw new Error("ruff: parsed 0 rules (table layout changed?)");
  return packIndex({
    tool: "ruff",
    language: "python",
    toolchainVersion: null,
    docsVersion,
    source: "astral-ruff",
    docsBase: "https://docs.astral.sh/ruff/rules/",
    rules,
  });
}

/** Best-effort ruff docs version from the docs landing page; "latest" on miss. */
async function ruffDocsVersion(rulesHtml) {
  const m = rulesHtml.match(/ruff[ \/-]v?(\d+\.\d+\.\d+)/i);
  if (m) return m[1];
  try {
    const home = await fetchText("https://docs.astral.sh/ruff/");
    const h = home.match(/(\d+\.\d+\.\d+)/);
    return h ? h[1] : "latest";
  } catch {
    return "latest";
  }
}

// ---------------------------------------------------------------------------
// eslint (js) — quick-fetch from the official rules index
// ---------------------------------------------------------------------------

/** ESLint section heading id -> normalized category. */
const ESLINT_SECTION_CATEGORY = {
  "possible-problems": "correctness",
  suggestions: "style",
  "layout--formatting": "style",
  deprecated: "deprecation",
  removed: "deprecation",
};

/**
 * Build the JS/ESLint index from the official rules index page. Rules are
 * `<article class="rule">` blocks; the governing category is the nearest
 * preceding `<h2 id="...">` section. "deprecated"/"removed" sections are kept
 * but flagged (severity low, category deprecation) since they still inform
 * config. Each article carries id, a short description, and category emoji.
 */
async function buildEslint() {
  const html = await fetchText("https://eslint.org/docs/latest/rules/");
  const docsVersion = await eslintDocsVersion();
  // Walk the document, tracking the current <h2 id> section as we hit articles.
  const tokenRe =
    /<h2[^>]*id="([^"]+)"[^>]*>|<article class="rule[^"]*">([\s\S]*?)<\/article>/g;
  let section = "possible-problems";
  const rules = [];
  const seen = new Set();
  let m;
  while ((m = tokenRe.exec(html)) !== null) {
    if (m[1]) {
      if (ESLINT_SECTION_CATEGORY[m[1]] !== undefined) section = m[1];
      continue;
    }
    const article = m[2];
    // Active rules link the name (<a class="rule__name">name</a>); deprecated/
    // removed rules use a plain <p class="rule__name">name</p>. Match both.
    const nameM = article.match(/class="rule__name">\s*([a-z0-9-]+)\s*</i);
    if (!nameM) continue;
    const id = nameM[1].trim();
    if (seen.has(id)) continue;
    seen.add(id);
    const descM = article.match(/class="rule__description">([\s\S]*?)<\/p>/);
    const description = descM ? htmlToText(descM[1]) : id.replace(/-/g, " ");
    const deprecated = section === "deprecated" || section === "removed";
    const recommended = article.includes("✅");
    rules.push({
      id,
      category: ESLINT_SECTION_CATEGORY[section] ?? "correctness",
      severity: deprecated ? "low" : section === "possible-problems" ? "high" : recommended ? "medium" : "low",
      description,
      source: `https://eslint.org/docs/latest/rules/${id}`,
    });
  }
  if (rules.length === 0) throw new Error("eslint: parsed 0 rules (page layout changed?)");
  return packIndex({
    tool: "eslint",
    language: "javascript",
    toolchainVersion: null,
    docsVersion,
    source: "eslint-org",
    docsBase: "https://eslint.org/docs/latest/rules/",
    rules,
  });
}

/** Best-effort latest ESLint version from npm; "latest" on miss. */
async function eslintDocsVersion() {
  try {
    const data = await fetchJson("https://registry.npmjs.org/eslint/latest");
    return data.version ?? "latest";
  } catch {
    return "latest";
  }
}

// ---------------------------------------------------------------------------
// staticcheck (go) — quick-fetch from the official checks table
// ---------------------------------------------------------------------------

/** Staticcheck check-code prefix -> normalized category. */
const STATICCHECK_GROUP = {
  SA: { category: "correctness", severity: "high" }, // staticcheck: real bugs
  S: { category: "complexity", severity: "low" }, // simple: simplifications
  ST: { category: "style", severity: "low" }, // stylecheck
  QF: { category: "style", severity: "low" }, // quickfix
};

/**
 * Build the Go/Staticcheck index from the official checks table. Rows whose
 * cells are `<td>` are individual checks (`CODE | short description`); `<th>`
 * rows are group headers and are skipped. The leading letter prefix of the code
 * (SA/S/ST/QF) selects the category and best-effort severity.
 */
async function buildStaticcheck() {
  const html = await fetchText("https://staticcheck.dev/docs/checks/");
  const docsVersion = await staticcheckDocsVersion();
  const rowRe = /<td><a href=#([A-Z]+\d+)>\1<\/a><\/td><td>([\s\S]*?)<\/td>/g;
  const rules = [];
  const seen = new Set();
  let m;
  while ((m = rowRe.exec(html)) !== null) {
    const code = m[1];
    if (seen.has(code)) continue;
    seen.add(code);
    const prefix = code.match(/^[A-Z]+/)[0];
    const group = STATICCHECK_GROUP[prefix] ?? { category: "correctness", severity: "medium" };
    rules.push({
      id: code,
      category: group.category,
      severity: group.severity,
      description: htmlToText(m[2]),
      source: `https://staticcheck.dev/docs/checks/#${code}`,
    });
  }
  if (rules.length === 0) throw new Error("staticcheck: parsed 0 checks (table layout changed?)");
  return packIndex({
    tool: "staticcheck",
    language: "go",
    toolchainVersion: null,
    docsVersion,
    source: "staticcheck-dev",
    docsBase: "https://staticcheck.dev/docs/checks/",
    rules,
  });
}

/** Best-effort latest staticcheck release tag from GitHub; "latest" on miss. */
async function staticcheckDocsVersion() {
  try {
    const data = await fetchJson("https://api.github.com/repos/dominikh/go-tools/releases/latest");
    return (data.tag_name ?? "latest").replace(/^v/, "");
  } catch {
    return "latest";
  }
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

/** Structured adapters (`kind: "builtin"`): highest-fidelity, version-matched
 * extraction from each tool's machine-readable source. */
const ADAPTERS = {
  clippy: () => buildClippy(rustVersion()),
  ruff: () => buildRuff(),
  eslint: () => buildEslint(),
  staticcheck: () => buildStaticcheck(),
};

/**
 * The universal adapter (`kind: "crawl"`): fetch a linter's official rules-index
 * page and extract each linked rule (anchor slug = id, text = description). Lower
 * fidelity than a structured adapter, but it makes adding ANY linter a data entry
 * in sources.json — no parser code. Output is the same Rust-parsable index.
 */
async function buildFromCrawl({ tool, language, docsVersion, docsBase, seed }) {
  const html = await fetchText(seed);
  const skip = /^(home|edit|history|login|logout|search|wiki|index|next|previous|prev|back|top|home page|table of contents|contents)$/i;
  const seen = new Set();
  const rules = [];
  const re = /<a[^>]+href="([^"]+)"[^>]*>([^<]{2,90})<\/a>/gi;
  let m;
  while ((m = re.exec(html)) !== null) {
    const text = m[2].replace(/\s+/g, " ").trim();
    const id = text.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
    if (!id || id.length < 2 || seen.has(id) || skip.test(text)) continue;
    seen.add(id);
    let url;
    try {
      url = new URL(m[1], seed).toString();
    } catch {
      url = seed;
    }
    rules.push({ id, category: "correctness", severity: "medium", description: text, source: url });
  }
  if (rules.length === 0) throw new Error(`generic crawl extracted 0 rules from ${seed}`);
  return packIndex({ tool, language, toolchainVersion: null, docsVersion: docsVersion ?? "latest", source: `crawl:${seed}`, docsBase, rules });
}

/** Read the source registry (lint-index/sources.json) — the data-driven tool list. */
function loadSources() {
  const raw = readFileSync(join(ROOT, "lint-index", "sources.json"), "utf8");
  return JSON.parse(raw).sources || [];
}

async function main() {
  mkdirSync(OUT, { recursive: true });
  const want = selectedTools();
  const sources = loadSources().filter((s) => !want || want.includes(s.tool));
  if (want && sources.length === 0) {
    console.error(`[lint-index] no source in sources.json matches: ${want.join(", ")}`);
    process.exit(2);
  }

  const built = [];
  for (const s of sources) {
    try {
      const index = s.kind === "crawl" ? await buildFromCrawl(s) : await ADAPTERS[s.tool]();
      const file = join(OUT, `${index.tool}.json`);
      writeFileSync(file, JSON.stringify(index, null, 2) + "\n");
      const ver = index.toolchainVersion
        ? `toolchain ${index.toolchainVersion} -> docs ${index.docsVersion}`
        : `docs ${index.docsVersion}`;
      built.push(`${s.tool} [${s.kind}] (${ver}): ${index.ruleCount} rules -> ${file}`);
    } catch (e) {
      console.error(`[lint-index] ${s.tool} failed: ${e.message}`);
    }
  }

  if (built.length === 0) {
    console.error("[lint-index] no indexes built.");
    process.exit(1);
  }
  for (const line of built) console.log(`[lint-index] ${line}`);
}

main().catch((e) => {
  console.error(`[lint-index] ${e.stack || e}`);
  process.exit(1);
});
