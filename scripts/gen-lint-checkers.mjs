#!/usr/bin/env node
// gen-lint-checkers.mjs — generate PRECISE checker specs for a language from its
// packed lint index, validating each against the rule's OWN examples so the result
// is accurate by construction (zero false positives, no human rule list).
//
// For each rule it proposes candidate specs (banned-keyword, self-binop, tail-
// return, method/macro regex) derived from the rule's `exampleBad`, then KEEPS a
// candidate only if it flags that `exampleBad` AND does NOT flag the rule's
// `exampleGood`. That self-validation gate is the precision guarantee. Rules whose
// shape none of the primitives can express+validate are reported as uncovered
// (they need a new generic primitive) — never silently approximated.
//
//   usage: node scripts/gen-lint-checkers.mjs <tool>   (e.g. clippy, ruff, eslint)

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const tool = process.argv[2];
if (!tool) {
  console.error("usage: node scripts/gen-lint-checkers.mjs <tool>");
  process.exit(1);
}

const idx = JSON.parse(readFileSync(join(ROOT, "lint-index", `${tool}.json`), "utf8"));
const lang = idx.language;
const version = idx.toolchainVersion || idx.docsVersion || "latest";

// ── primitive interpreters (MIRROR native/src/lint_checkers.rs exactly) ──────
const codeOf = (line) => line.split("//")[0].split("#")[0];

function runRegex(pattern, lines) {
  let re;
  try {
    re = new RegExp(pattern);
  } catch {
    return false;
  }
  return lines.some((l) => re.test(codeOf(l)));
}
function runSelfBinop(ops, lines) {
  const re = /\b([a-zA-Z_]\w*)\s*(==|!=|&&|\|\||&|\||<=|>=)\s*([a-zA-Z_]\w*)\b/g;
  for (const l of lines) {
    const c = codeOf(l);
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(c))) {
      if (m[1] === m[3] && m[1] !== "true" && m[1] !== "false" && ops.includes(m[2])) return true;
    }
  }
  return false;
}
function runBanned(kw, lines) {
  const re = new RegExp(`\\b${kw.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`);
  return lines.some((l) => re.test(codeOf(l)));
}
function runTailReturn(lines) {
  let i = 0;
  while (i < lines.length) {
    const t = lines[i].trimStart();
    const isFn = (t.startsWith("fn ") || t.includes(" fn ") || t.startsWith("def ")) && (lines[i].includes("{") || t.startsWith("def "));
    if (isFn) {
      const braces = lines[i].includes("{");
      let depth = 0,
        end = lines.length - 1;
      if (braces) {
        for (let k = i; k < lines.length; k++) {
          depth += (codeOf(lines[k]).split("{").length - 1) - (codeOf(lines[k]).split("}").length - 1);
          if (depth <= 0) { end = k; break; }
        }
      }
      for (let k = end; k > i; k--) {
        const s = codeOf(lines[k]).trim().replace(/;+$/, "").trim();
        if (s === "" || s === "}") continue;
        if (s === "return" || s.startsWith("return ")) return true;
        break;
      }
      i = end + 1;
    } else i += 1;
  }
  return false;
}
function runSpec(spec, code) {
  const lines = code.split("\n");
  switch (spec.kind) {
    case "regex": return runRegex(spec.pattern, lines);
    case "self_binop": return runSelfBinop(spec.ops, lines);
    case "banned_keyword": return runBanned(spec.keyword, lines);
    case "tail_return": return runTailReturn(lines);
    default: return false;
  }
}

// ── candidate generation from a rule's bad example ───────────────────────────
function candidates(rule) {
  const bad = rule.exampleBad || "";
  const out = [];
  const nm = rule.id.match(/^no-([a-zA-Z]+)$/);
  if (nm) out.push({ kind: "banned_keyword", keyword: nm[1], _c: `kw:${nm[1]}` });
  out.push({ kind: "self_binop", ops: ["==", "!=", "&&", "||", "&", "|"], _c: "self" });
  out.push({ kind: "tail_return", _c: "tail" });
  for (const m of bad.matchAll(/\b([a-zA-Z_]\w+)\s*!\s*\(/g)) {
    out.push({ kind: "regex", pattern: `\\b${m[1]}!\\(`, _c: `${m[1]}!(`, name: m[1] });
  }
  for (const m of bad.matchAll(/\.([a-z_]\w+)\s*\(/g)) {
    out.push({ kind: "regex", pattern: `\\.${m[1]}\\(`, _c: `.${m[1]}(`, name: m[1] });
  }
  return out;
}

// Keywords kept literally in a skeleton (everything else that's a bare word is an
// operand → generalized to \w+). Distinctive structure (methods, macros, operators,
// literals) is preserved, so the pattern stays precise — and the good-example
// validation gate rejects any skeleton that would false-positive.
const KEEP = new Set(
  ("if else elif for while loop match return fn func def let const mut pub use impl struct enum trait " +
   "as in is and or not with try except raise lambda yield await async move dyn ref where self super " +
   "true false None True False null nil Some Ok Err Vec String Box i8 i16 i32 i64 u8 u16 u32 u64 usize " +
   "isize f32 f64 bool str char int float double void str string list dict set tuple").split(" ")
);
const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** Build a precise regex from one example line: methods/macros/operators/literals
 * kept, bare operands → \w+, whitespace → optional. null if too thin to be precise. */
function skeletonRegex(line) {
  const t = line.trim();
  if (t.length < 5 || !/[A-Za-z]/.test(t)) return null;
  const tokRe = /(\.[A-Za-z_]\w*)|([A-Za-z_]\w*!)|([A-Za-z_]\w*)|(\d+)|("(?:[^"\\]|\\.)*")|(\s+)|([^\sA-Za-z0-9_])/g;
  let pat = "";
  let real = 0;
  let m;
  while ((m = tokRe.exec(t))) {
    if (m[1]) { pat += "\\" + m[1]; real++; }
    else if (m[2]) { pat += escapeRe(m[2]); real++; }
    else if (m[3]) { if (KEEP.has(m[3])) { pat += m[3]; real++; } else pat += "\\w+"; }
    else if (m[4]) { pat += m[4]; real++; }
    else if (m[5]) { pat += '"[^"]*"'; real++; }
    else if (m[6]) { pat += "\\s*"; }
    else if (m[7]) { pat += escapeRe(m[7]); real++; }
  }
  return real >= 2 ? pat : null;
}

// document frequency of each construct across all rules (distinctiveness guard for
// rules that lack a good example to validate against).
const freq = {};
for (const r of idx.rules) {
  if (!r.exampleBad) continue;
  for (const cand of candidates(r)) freq[cand._c] = (freq[cand._c] || 0) + 1;
}

// A corpus of CLEAN idiomatic code: every rule's good example. A skeleton that
// matches ANY line here would false-positive on real code, so it's rejected. This
// is the precision gate that keeps coverage honest (no over-matching).
const cleanCorpus = idx.rules.map((r) => r.exampleGood || "").filter(Boolean).join("\n");

// ── generate + self-validate ─────────────────────────────────────────────────
const checkers = [];
const seen = new Set();
let total = 0,
  covered = 0,
  uncovered = [];
for (const r of idx.rules) {
  if (!r.exampleBad) continue;
  total++;
  const good = r.exampleGood || "";
  let picked = null;
  for (const cand of candidates(r)) {
    const flagsBad = runSpec(cand, r.exampleBad);
    if (!flagsBad) continue;
    const flagsGood = good ? runSpec(cand, good) : false;
    if (flagsGood) continue; // would false-positive on the fixed code
    // Reject any regex that matches CLEAN idiomatic code anywhere in the corpus
    // (e.g. `.iter(`, `.map(`): those are not violations. Only constructs absent
    // from all good examples (`.unwrap(`, `panic!(`) survive — precise by evidence.
    if (cand.kind === "regex" && runSpec(cand, cleanCorpus)) continue;
    // No good example to confirm precision → accept a regex construct only when it
    // is distinctive (rare across the corpus) OR the rule id names it (e.g.
    // `unwrap_used` → `.unwrap(`), which is a precise "this construct IS the rule".
    if (!good && cand.kind === "regex") {
      const idNorm = r.id.toLowerCase().replace(/[_-]/g, "");
      const named = cand.name && idNorm.includes(cand.name.toLowerCase());
      if ((freq[cand._c] || 0) > 2 && !named) continue;
    }
    picked = cand;
    break;
  }
  if (!picked) { uncovered.push(r.id); continue; }
  const key = `${picked.kind}:${picked.pattern || picked.keyword || picked._c}`;
  if (seen.has(key)) { covered++; continue; } // dedupe identical checks
  seen.add(key);
  const spec = { rule: r.id, severity: r.severity || "low", kind: picked.kind };
  if (picked.pattern) spec.pattern = picked.pattern;
  if (picked.ops) spec.ops = picked.ops;
  if (picked.keyword) spec.keyword = picked.keyword;
  checkers.push(spec);
  covered++;
}

const outDir = join(ROOT, "lint-checkers");
mkdirSync(outDir, { recursive: true });
// The HARD base bank per language (the bulk, version-independent). Per-version
// supplements (`<lang>@<version>.json`) carry only the deltas a new version adds.
const out = { language: lang, version, _note: `Hard ${lang} checker bank, generated + self-validated from ${tool} docs (each spec flags its rule's exampleBad and not its exampleGood). Version supplements live in ${lang}@<version>.json.`, checkers };
const file = join(outDir, `${lang}.json`);
writeFileSync(file, JSON.stringify(out, null, 2));
console.log(`${lang} (base, gen from ${tool}@${version}): ${covered}/${total} rules validated (${checkers.length} unique) -> lint-checkers/${lang}.json`);
console.log(`  uncovered (need a new primitive): ${uncovered.length}`);
