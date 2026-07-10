#!/usr/bin/env node
// build-docs-corpus.mjs — assemble ALL coding documentation into one training +
// grounding corpus for the code-understanding model (the "God Programmer").
//
// The model's recipe is: English understanding (from the dictionary) → read all the
// coding docs → learn what code is and what good/bad/deprecated looks like. This
// builds the second half of that fuel: every rule from every language's official
// docs, as a record the model trains on (and the linter grounds against). Each
// record carries the English lesson (description), the anti-pattern (exampleBad),
// and the fix (exampleGood) — the bad→good pair is the supervised signal.
//
//   node scripts/build-docs-corpus.mjs   ->  corpus/lint-corpus.jsonl  (+ stats)

import { readFileSync, writeFileSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const INDEX_DIR = join(ROOT, "lint-index");
const OUT_DIR = join(ROOT, "corpus");
mkdirSync(OUT_DIR, { recursive: true });

const records = [];
const byLang = {};
let withPair = 0;

for (const file of readdirSync(INDEX_DIR)) {
  if (!file.endsWith(".json") || file === "sources.json") continue;
  const idx = JSON.parse(readFileSync(join(INDEX_DIR, file), "utf8"));
  const lang = idx.language || "?";
  const version = idx.toolchainVersion || idx.docsVersion || "latest";
  byLang[lang] = byLang[lang] || { rules: 0, examples: 0, pairs: 0, version };
  for (const r of idx.rules || []) {
    byLang[lang].rules++;
    const rec = {
      language: lang,
      version,
      rule: r.id,
      category: r.category || "",
      severity: r.severity || "",
      // the English lesson — connects to the dictionary-pretrained understanding
      description: r.description || "",
      // the anti-pattern and its fix — the supervised bad→good signal
      bad: r.exampleBad || "",
      good: r.exampleGood || "",
      source: r.source || "",
    };
    if (rec.bad) byLang[lang].examples++;
    if (rec.bad && rec.good) {
      byLang[lang].pairs++;
      withPair++;
    }
    records.push(rec);
  }
}

// Stable order so the corpus is reproducible (language, then rule id).
records.sort((a, b) => (a.language + a.rule < b.language + b.rule ? -1 : 1));

const jsonl = records.map((r) => JSON.stringify(r)).join("\n") + "\n";
writeFileSync(join(OUT_DIR, "lint-corpus.jsonl"), jsonl);

const manifest = {
  builtAt: new Date().toISOString(),
  totalRules: records.length,
  withBadGoodPair: withPair,
  languages: byLang,
  note: "Training+grounding corpus for the code-understanding model. Each record is one official rule: description (English lesson), bad (anti-pattern), good (fix). bad→good pairs are the supervised signal; description ties to the dictionary-pretrained English understanding.",
};
writeFileSync(join(OUT_DIR, "manifest.json"), JSON.stringify(manifest, null, 2));

console.log(`corpus/lint-corpus.jsonl: ${records.length} rule records, ${withPair} with bad→good training pairs`);
for (const [lang, s] of Object.entries(byLang)) {
  console.log(`  ${lang.padEnd(12)} v${s.version.padEnd(8)} rules=${s.rules} examples=${s.examples} pairs=${s.pairs}`);
}
