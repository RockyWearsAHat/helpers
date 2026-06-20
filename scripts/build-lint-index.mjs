#!/usr/bin/env node
// build-lint-index.mjs — the "backrub" lint indexer.
//
// Crawls official, version-matched linter documentation and *derives the rules
// of code directly from it* — no hand-authored rules, no running the external
// tool. For each supported toolchain it detects the version the project uses,
// fetches that version's official machine-readable rule database, and extracts a
// normalized rule (id, severity, category, description, and the canonical
// bad/good code examples straight from the docs) into a versioned index.
//
// The index is the snapshot embedded into the binary; it can be refetched when a
// network connection is available so the rules always match the toolchain in
// use. Because the rules come from the official docs for that exact version,
// they are always modern and correct.
//
//   usage: node scripts/build-lint-index.mjs [--out DIR]
//
// Sources (curated allowlist — official docs only):
//   - Rust / Clippy: rust-lang.github.io/rust-clippy/rust-<version>/lints.json

import { execSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = outArg() ?? join(ROOT, "lint-index");

/** Parse `--out DIR` from argv, or return null for the default. */
function outArg() {
  const i = process.argv.indexOf("--out");
  return i >= 0 ? process.argv[i + 1] : null;
}

/** The Rust toolchain version in use (e.g. "1.83.0"), or null if rustc absent. */
function rustVersion() {
  try {
    const out = execSync("rustc --version", { encoding: "utf8" });
    const m = out.match(/\b(\d+\.\d+\.\d+)\b/);
    return m ? m[1] : null;
  } catch {
    return null;
  }
}

/** Fetch JSON, throwing with context on a non-200 so failures are never silent. */
async function fetchJson(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`GET ${url} -> ${res.status}`);
  return res.json();
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
 * Resolve the Clippy docs version whose machine-readable `lints.json` we fetch.
 * Clippy only publishes that database for numbered versions (the `master`/
 * `stable` docs moved to a different format), so we read the official
 * `versions.json` manifest and pick the newest published version that is **≤**
 * the toolchain the project uses — i.e. the closest official match to the
 * version actually in play — falling back to the newest published overall when
 * the toolchain is newer than anything published yet. Each candidate's
 * `lints.json` is probed newest-first so a version dir without the data file is
 * skipped rather than failing the build.
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
    const res = await fetch(`https://rust-lang.github.io/rust-clippy/rust-${v}/lints.json`);
    if (res.ok) return v;
  }
  return null;
}

/**
 * Pull the first fenced ```rust block out of a Clippy `docs` markdown body that
 * follows the given header (e.g. "Example", "Use instead"). Returns the trimmed
 * code, or "" when the section/block is absent.
 */
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
const GROUP_CATEGORY = {
  correctness: "correctness",
  suspicious: "correctness",
  complexity: "complexity",
  perf: "data-structures",
  style: "naming",
};
/** Clippy level -> our severity. */
const LEVEL_SEVERITY = { deny: "high", warn: "medium", allow: "low" };
/** Groups that carry CS-principle substance (skip pedantic/nursery/restriction/cargo). */
const KEEP_GROUPS = new Set(["correctness", "suspicious", "complexity", "perf"]);

/** Build the Rust/Clippy index from the official lints database. `docsVersion`
 * is the published Clippy version we read; `toolchain` is the version the
 * project actually uses (recorded so a later refetch can re-match). */
async function buildClippy(docsVersion, toolchain) {
  const base = `https://rust-lang.github.io/rust-clippy/rust-${docsVersion}`;
  const lints = await fetchJson(`${base}/lints.json`);
  const rules = [];
  for (const lint of lints) {
    if (!KEEP_GROUPS.has(lint.group) || lint.level === "allow") continue;
    const docs = lint.docs || "";
    rules.push({
      id: `clippy-${lint.id}`,
      lintId: lint.id,
      language: "rust",
      category: GROUP_CATEGORY[lint.group] ?? "correctness",
      severity: LEVEL_SEVERITY[lint.level] ?? "medium",
      group: lint.group,
      applicability: lint.applicability?.applicability ?? null,
      whatItDoes: sectionText(docs, "What it does"),
      whyBad: sectionText(docs, "Why is this bad?") || sectionText(docs, "Why is this bad"),
      exampleBad: codeAfter(docs, "Example"),
      exampleGood: codeAfter(docs, "Use instead"),
      source: `https://rust-lang.github.io/rust-clippy/master/index.html#${lint.id}`,
    });
  }
  return {
    source: "rust-clippy",
    language: "rust",
    tool: "clippy",
    toolchainVersion: toolchain ?? null,
    docsVersion,
    fetchedAt: new Date().toISOString(),
    docsBase: base,
    ruleCount: rules.length,
    rules,
  };
}

async function main() {
  mkdirSync(OUT, { recursive: true });
  const built = [];

  const rust = rustVersion();
  try {
    const docsVersion = await resolveClippyVersion(rust);
    if (!docsVersion) throw new Error("no published Clippy lints.json found");
    const index = await buildClippy(docsVersion, rust);
    const file = join(OUT, "rust-clippy.json");
    writeFileSync(file, JSON.stringify(index, null, 2) + "\n");
    const match = rust ? `toolchain ${rust} -> docs ${docsVersion}` : `docs ${docsVersion}`;
    built.push(`rust-clippy (${match}): ${index.ruleCount} rules -> ${file}`);
  } catch (e) {
    console.error(`[lint-index] clippy failed: ${e.message}`);
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
