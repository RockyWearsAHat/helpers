#!/usr/bin/env node
"use strict";

/*
 * git-cs-grade — objective structural rubric for CS2420 / CS3500 projects.
 *
 * Scans a (primarily Java) project and scores it against a course rubric, then
 * writes GRADE.md: a numeric+letter grade, per-category scores with the exact
 * evidence behind them, and a prioritized, concrete "Path to A+" checklist.
 *
 * This grades the things an agent can actually restructure: design, abstraction,
 * tests, documentation, style, and cleanliness. It is a structural rubric, not a
 * substitute for the course autograder's correctness suite — it says so in the
 * report. The intended loop: `git-cs-grade` -> agent fixes the checklist ->
 * `git-cs-grade` again, until the grade is A+.
 *
 *   git-cs-grade [path] [--course cs2420|cs3500|auto] [--json]
 */

const fs = require("fs");
const path = require("path");

// ---------------------------------------------------------------------------
// args
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
let root = ".";
let course = "auto";
let asJson = false;
for (let i = 0; i < args.length; i++) {
  const a = args[i];
  if (a === "--course") course = (args[++i] || "auto").toLowerCase();
  else if (a === "--json") asJson = true;
  else if (a === "-h" || a === "--help") {
    console.log("usage: git-cs-grade [path] [--course cs2420|cs3500|auto] [--json]");
    process.exit(0);
  } else if (!a.startsWith("-")) root = a;
}
root = path.resolve(root);
if (!fs.existsSync(root)) {
  console.error(`git-cs-grade: path not found: ${root}`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// walk the tree (skip noise)
// ---------------------------------------------------------------------------
const IGNORE = new Set([
  ".git", "node_modules", "target", "build", "out", "bin", "dist",
  ".idea", ".vscode", ".gradle", ".settings", "__pycache__",
]);
function walk(dir, acc = []) {
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return acc;
  }
  for (const e of entries) {
    if (e.name.startsWith(".") && e.name !== ".") {
      if (IGNORE.has(e.name)) continue;
    }
    if (IGNORE.has(e.name)) continue;
    const full = path.join(dir, e.name);
    if (e.isDirectory()) walk(full, acc);
    else acc.push(full);
  }
  return acc;
}
const rel = (f) => path.relative(root, f) || path.basename(f);
// Sort by relative path so corpora — and therefore scores and evidence
// listings — are deterministic across platforms and match the Rust build.
const allFiles = walk(root).sort((a, b) => {
  const ra = rel(a), rb = rel(b);
  return ra < rb ? -1 : ra > rb ? 1 : 0;
});

const javaFiles = allFiles.filter((f) => f.endsWith(".java"));
const readText = (f) => {
  try { return fs.readFileSync(f, "utf8"); } catch { return ""; }
};

// Source vs test partition.
const isTestFile = (f) =>
  /(^|\/)(test|tests)\//i.test(rel(f)) || /Test[s]?\.java$/.test(f) || /Tests?\b/.test(path.basename(f));
const testFiles = javaFiles.filter(isTestFile);
const srcFiles = javaFiles.filter((f) => !isTestFile(f));

// ---------------------------------------------------------------------------
// signals
// ---------------------------------------------------------------------------
const corpus = srcFiles.map(readText);
const joined = corpus.join("\n");
const testCorpus = testFiles.map(readText).join("\n");

const count = (re, hay = joined) => (hay.match(re) || []).length;

// public methods / classes vs javadoc
const publicDecls = count(/\bpublic\s+(?:static\s+)?(?:final\s+)?(?:abstract\s+)?(?:class|interface|enum|[\w<>\[\]]+\s+\w+\s*\()/g);
const javadocBlocks = count(/\/\*\*[\s\S]*?\*\//g);
const javadocRatio = publicDecls ? Math.min(1, javadocBlocks / publicDecls) : 0;

const interfaceCount = count(/\binterface\s+\w+/g);
const classCount = count(/\bclass\s+\w+/g) || 1;
const abstractCount = count(/\babstract\s+class\s+\w+/g);

// design pattern hints (by name / structure)
const patternHits = [];
for (const p of ["Strategy", "Command", "Factory", "Builder", "Observer", "Adapter", "Decorator", "Visitor", "Composite", "Iterator", "Singleton", "Facade"]) {
  if (new RegExp(`\\b\\w*${p}\\b`).test(joined)) patternHits.push(p);
}

// MVC separation (CS3500)
const hasModel = /(^|\/)model(s)?(\/|\.|$)|class\s+\w*Model\b|interface\s+\w*Model\b/i.test(joined + javaFiles.map(rel).join("\n"));
const hasView = /(^|\/)view(s)?(\/|\.|$)|class\s+\w*View\b|interface\s+\w*View\b/i.test(joined + javaFiles.map(rel).join("\n"));
const hasController = /(^|\/)controller(s)?(\/|\.|$)|class\s+\w*Controller\b|interface\s+\w*Controller\b/i.test(joined + javaFiles.map(rel).join("\n"));
const mvcScore = (hasModel ? 1 : 0) + (hasView ? 1 : 0) + (hasController ? 1 : 0);

// tests
const junitUsage = /org\.junit|@Test/.test(testCorpus + joined);
const testRatio = srcFiles.length ? Math.min(1, testFiles.length / srcFiles.length) : 0;
const assertionCount = count(/\bassert\w*\s*\(/g, testCorpus);

// build & structure
const buildFiles = allFiles.filter((f) => /(^|\/)(pom\.xml|build\.gradle(\.kts)?|build\.xml|Makefile)$/.test(rel(f)));
const usesPackages = count(/^\s*package\s+[\w.]+;/gm);
const usesSrcLayout = javaFiles.some((f) => /(^|\/)src\//.test(rel(f)));

// docs
const readmes = allFiles.filter((f) => /readme(\.md|\.txt)?$/i.test(path.basename(f)));
const readmeBytes = readmes.reduce((n, f) => n + (fs.statSync(f).size || 0), 0);
const designDocs = allFiles.filter((f) => /(design|architecture|analysis|writeup|report)\.(md|txt|pdf)$/i.test(path.basename(f)));

// complexity / analysis writeup (CS2420)
const bigOMentions = count(/\bO\([^)]+\)|big-?o|asymptotic|time complexity/gi, joined + readmes.map(readText).join("\n") + designDocs.map(readText).join("\n"));
const usesGoodStructures = /\b(HashMap|HashSet|TreeMap|TreeSet|PriorityQueue|ArrayDeque|LinkedList|ArrayList)\b/.test(joined);

// cleanliness
const godClasses = srcFiles.filter((f) => readText(f).split("\n").length > 400).map(rel);
const longMethodHits = count(/\{[^{}]{1600,}\}/g); // rough: very large brace bodies
const debugPrints = count(/System\.out\.print|printStackTrace\(/g);
const todoMarkers = count(/\b(TODO|FIXME|XXX|HACK)\b/g);
const commentedCode = count(/^\s*\/\/\s*(if|for|while|return|System\.|int |String |public |private )/gm);

// ---------------------------------------------------------------------------
// course auto-detection
// ---------------------------------------------------------------------------
if (course === "auto") {
  const oodScore = mvcScore * 2 + interfaceCount + patternHits.length;
  const dsaScore = (usesGoodStructures ? 2 : 0) + Math.min(4, bigOMentions);
  course = oodScore >= dsaScore ? "cs3500" : "cs2420";
}

// ---------------------------------------------------------------------------
// scoring helpers — every sub-score is 0..1 with attached evidence
// ---------------------------------------------------------------------------
const clamp01 = (x) => Math.max(0, Math.min(1, x));

function categoryTests() {
  const sub = clamp01(0.3 * (junitUsage ? 1 : 0) + 0.4 * testRatio + 0.3 * clamp01(assertionCount / Math.max(8, srcFiles.length * 3)));
  return {
    score: sub,
    evidence: `${testFiles.length} test file(s) for ${srcFiles.length} source file(s) (ratio ${(testRatio).toFixed(2)}), ${assertionCount} assertion(s), JUnit ${junitUsage ? "detected" : "NOT detected"}.`,
    fixes: [
      !junitUsage && "Add JUnit tests (`@Test`, assertions) — no test framework usage detected.",
      testRatio < 0.5 && "Raise test coverage: aim for a test class per non-trivial source class.",
      assertionCount < srcFiles.length * 3 && "Add more assertions per test, including edge cases and failure paths.",
    ].filter(Boolean),
  };
}

function categoryDocs() {
  const readmeScore = clamp01(readmeBytes / 1200);
  const sub = clamp01(0.55 * javadocRatio + 0.3 * readmeScore + 0.15 * clamp01(designDocs.length));
  return {
    score: sub,
    evidence: `Javadoc coverage ~${(javadocRatio * 100).toFixed(0)}% (${javadocBlocks} blocks / ${publicDecls} public decls), README ${readmeBytes} bytes, ${designDocs.length} design/analysis doc(s).`,
    fixes: [
      javadocRatio < 0.9 && "Add Javadoc to every public class, interface, and method (purpose, @param, @return, @throws).",
      readmeBytes < 1200 && "Expand the README: overview, how to build/run, and a design overview.",
      designDocs.length === 0 && "Add a design/analysis document describing key decisions.",
    ].filter(Boolean),
  };
}

function categoryStyle() {
  const cleanliness =
    1 -
    clamp01(godClasses.length * 0.25) -
    clamp01(longMethodHits * 0.1) -
    clamp01(debugPrints * 0.05) -
    clamp01(todoMarkers * 0.05) -
    clamp01(commentedCode * 0.05);
  const sub = clamp01(cleanliness);
  return {
    score: sub,
    evidence: `${godClasses.length} file(s) >400 lines${godClasses.length ? " (" + godClasses.slice(0, 3).join(", ") + ")" : ""}, ${longMethodHits} very-long method body(ies), ${debugPrints} debug print(s), ${todoMarkers} TODO/FIXME, ${commentedCode} commented-out code line(s).`,
    fixes: [
      godClasses.length && `Split god classes (>400 lines): ${godClasses.slice(0, 5).join(", ")}.`,
      longMethodHits && "Extract long methods into small, single-responsibility helpers.",
      debugPrints && "Remove debug prints / printStackTrace; use proper error handling or logging.",
      todoMarkers && "Resolve or remove TODO/FIXME/HACK markers before submission.",
      commentedCode && "Delete commented-out code — version control is the history.",
    ].filter(Boolean),
  };
}

function categoryBuild() {
  const sub = clamp01(0.5 * (buildFiles.length ? 1 : usesSrcLayout ? 0.6 : 0.3) + 0.5 * (usesPackages ? 1 : 0.4));
  return {
    score: sub,
    evidence: `Build file: ${buildFiles.length ? buildFiles.map(rel).join(", ") : "none"}; ${usesPackages} package declaration(s); src/ layout ${usesSrcLayout ? "yes" : "no"}.`,
    fixes: [
      !buildFiles.length && "Add a build file (pom.xml / build.gradle / Makefile) so the project builds reproducibly.",
      !usesPackages && "Organize classes into Java packages rather than the default package.",
      !usesSrcLayout && "Adopt a standard src/ (and src/test) source layout.",
    ].filter(Boolean),
  };
}

function categoryDesignOOD() {
  const interfaceUse = clamp01(interfaceCount / Math.max(2, classCount * 0.3));
  const sub = clamp01(0.4 * (mvcScore / 3) + 0.35 * interfaceUse + 0.25 * clamp01(patternHits.length / 2));
  return {
    score: sub,
    evidence: `MVC: model=${hasModel}, view=${hasView}, controller=${hasController}; ${interfaceCount} interface(s) / ${classCount} class(es); patterns seen: ${patternHits.join(", ") || "none"}.`,
    fixes: [
      mvcScore < 3 && `Establish clear MVC separation (missing: ${[!hasModel && "model", !hasView && "view", !hasController && "controller"].filter(Boolean).join(", ")}).`,
      interfaceUse < 0.8 && "Program to interfaces: expose behavior through interfaces, keep concrete classes behind them.",
      patternHits.length < 2 && "Apply appropriate design patterns (Strategy/Command/Factory/Builder/Observer) where they reduce coupling.",
    ].filter(Boolean),
  };
}

function categoryAbstraction() {
  const sub = clamp01(0.5 * clamp01(interfaceCount / Math.max(1, classCount * 0.25)) + 0.25 * (abstractCount ? 1 : 0.5) + 0.25 * (debugPrints === 0 ? 1 : 0.5));
  return {
    score: sub,
    evidence: `${interfaceCount} interface(s), ${abstractCount} abstract class(es); implementation details ${debugPrints ? "leak via prints" : "appear encapsulated"}.`,
    fixes: [
      interfaceCount < Math.ceil(classCount * 0.25) && "Increase abstraction: define interfaces for major roles; avoid depending on concrete types.",
      "Ensure no field is public; expose state through methods and keep representation private.",
    ].filter(Boolean),
  };
}

function categoryDataStructures() {
  const sub = clamp01(0.5 * (usesGoodStructures ? 1 : 0.3) + 0.5 * clamp01(bigOMentions / 4));
  return {
    score: sub,
    evidence: `Standard collections ${usesGoodStructures ? "used" : "not detected"}; ${bigOMentions} complexity/Big-O mention(s) in code+docs.`,
    fixes: [
      !usesGoodStructures && "Use appropriate data structures (HashMap/TreeMap/PriorityQueue/…) for each access pattern.",
      bigOMentions < 4 && "Document the asymptotic complexity of key operations and include a timing/analysis writeup.",
    ].filter(Boolean),
  };
}

// ---------------------------------------------------------------------------
// rubrics (category -> weight). Weights sum to 100.
// ---------------------------------------------------------------------------
const RUBRICS = {
  cs2420: [
    ["Correctness & build", 20, categoryBuild],
    ["Tests & coverage", 20, categoryTests],
    ["Data structures & complexity", 15, categoryDataStructures],
    ["Documentation & Javadoc", 15, categoryDocs],
    ["Code style & cleanliness", 15, categoryStyle],
    ["Design & structure", 15, categoryDesignOOD],
  ],
  cs3500: [
    ["Object-oriented design", 25, categoryDesignOOD],
    ["Tests & coverage", 20, categoryTests],
    ["Documentation & Javadoc", 15, categoryDocs],
    ["Code style & cleanliness", 15, categoryStyle],
    ["Correctness & build", 15, categoryBuild],
    ["Abstraction & encapsulation", 10, categoryAbstraction],
  ],
};

const rubric = RUBRICS[course] || RUBRICS.cs2420;

// ---------------------------------------------------------------------------
// evaluate
// ---------------------------------------------------------------------------
const results = rubric.map(([name, weight, fn]) => {
  const r = fn();
  return { name, weight, earned: r.score * weight, ...r };
});
const total = results.reduce((s, r) => s + r.earned, 0);
const pct = Math.round(total * 10) / 10;

function letter(p) {
  if (p >= 97) return "A+";
  if (p >= 93) return "A";
  if (p >= 90) return "A-";
  if (p >= 87) return "B+";
  if (p >= 83) return "B";
  if (p >= 80) return "B-";
  if (p >= 77) return "C+";
  if (p >= 73) return "C";
  if (p >= 70) return "C-";
  if (p >= 60) return "D";
  return "F";
}
const grade = letter(pct);

// Path to A+: every category not already at A+ level (>= 0.97 of its weight),
// ordered by the points it would recover.
const gaps = results
  .filter((r) => r.earned < r.weight * 0.97)
  .map((r) => ({ ...r, recoverable: r.weight - r.earned }))
  .sort((a, b) => b.recoverable - a.recoverable);

// ---------------------------------------------------------------------------
// emit
// ---------------------------------------------------------------------------
if (asJson) {
  console.log(JSON.stringify({ course, pct, grade, total, results, gaps }, null, 2));
  process.exit(pct >= 97 ? 0 : 2);
}

const lines = [];
lines.push(`# Grade Report — ${course.toUpperCase()}`);
lines.push("");
lines.push(`**Grade: ${grade}  (${pct.toFixed(1)} / 100)**`);
lines.push("");
lines.push(`Project: \`${rel(root) || root}\` · ${srcFiles.length} source file(s), ${testFiles.length} test file(s) · generated by \`git-cs-grade\`.`);
lines.push("");
lines.push("> Structural rubric: grades design, abstraction, tests, documentation, style, and");
lines.push("> cleanliness — the things you can restructure. It does **not** run the course");
lines.push("> autograder's correctness suite; pair it with the official tests.");
lines.push("");
lines.push("## Scorecard");
lines.push("");
lines.push("| Category | Score | Weight | Evidence |");
lines.push("| -------- | ----- | ------ | -------- |");
for (const r of results) {
  lines.push(`| ${r.name} | ${r.earned.toFixed(1)} | ${r.weight} | ${r.evidence} |`);
}
lines.push(`| **Total** | **${pct.toFixed(1)}** | **100** | **${grade}** |`);
lines.push("");

if (gaps.length === 0) {
  lines.push("## Path to A+");
  lines.push("");
  lines.push("🎉 Already at A+ on this structural rubric. Confirm correctness with the course autograder.");
} else {
  lines.push("## Path to A+ (highest-impact first)");
  lines.push("");
  for (const g of gaps) {
    lines.push(`### ${g.name}  — recover up to ${g.recoverable.toFixed(1)} pts`);
    for (const fix of g.fixes) lines.push(`- [ ] ${fix}`);
    if (g.fixes.length === 0) lines.push(`- [ ] Polish "${g.name}" — close the remaining gap to full marks.`);
    lines.push("");
  }
  lines.push("After applying fixes, re-run `git-cs-grade` (or `gsh grade`) and repeat until the grade is A+.");
}
lines.push("");

const report = lines.join("\n");
const outPath = path.join(root, "GRADE.md");
fs.writeFileSync(outPath, report + "\n");

// console summary
console.log(report);
console.log(`\nWrote ${path.relative(process.cwd(), outPath) || outPath}`);
process.exit(pct >= 97 ? 0 : 2);
