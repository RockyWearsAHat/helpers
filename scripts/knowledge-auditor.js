#!/usr/bin/env node
"use strict";

/**
 * Knowledge Base Auditor — Continuous accuracy & freshness verification.
 *
 * Runs incrementally: each invocation audits BATCH_SIZE files, prioritizing
 * never-audited files first, then oldest-audited. After a full pass, the
 * cycle restarts.
 *
 * When issues are found, writes an audit-wave file that KnowledgeBuilder
 * agents can pick up to research and fix.
 *
 * Environment:
 *   AUDIT_API_KEY      — API key for the LLM provider (required)
 *   AUDIT_API_BASE     — Base URL (default: https://api.openai.com/v1)
 *   AUDIT_MODEL        — Model name (default: gpt-4.1-mini)
 *   AUDIT_BATCH_SIZE   — Files per run (default: 15)
 *   AUDIT_WAVE_DIR     — Where to write wave files (default: knowledge/waves)
 *   KNOWLEDGE_DIR      — Knowledge directory (default: auto-detect)
 */

const fs = require("fs");
const fsp = require("fs/promises");
const path = require("path");
const https = require("https");
const http = require("http");

// ─── Configuration ──────────────────────────────────────────────────────────
const API_KEY = process.env.AUDIT_API_KEY;
const API_BASE = (process.env.AUDIT_API_BASE || "https://api.openai.com/v1").replace(/\/+$/, "");
const MODEL = process.env.AUDIT_MODEL || "gpt-4.1-mini";
const BATCH_SIZE = parseInt(process.env.AUDIT_BATCH_SIZE || "15", 10);

const REPO_ROOT = path.resolve(__dirname, "..");
const KNOWLEDGE_ROOT = (() => {
  const explicit = process.env.KNOWLEDGE_DIR;
  if (explicit) return path.resolve(explicit);
  const atRoot = path.join(REPO_ROOT, "knowledge");
  const atGithub = path.join(REPO_ROOT, ".github", "knowledge");
  try {
    if (fs.readdirSync(atRoot).some((f) => f.endsWith(".md"))) return atRoot;
  } catch { /* fallthrough */ }
  return atGithub;
})();

const WAVE_DIR = process.env.AUDIT_WAVE_DIR || path.join(KNOWLEDGE_ROOT, "waves");
const STATE_PATH = path.join(WAVE_DIR, "_audit-state.json");

// ─── State management ───────────────────────────────────────────────────────
async function loadState() {
  try {
    return JSON.parse(await fsp.readFile(STATE_PATH, "utf8"));
  } catch {
    return { files: {}, lastFullPass: null, runCount: 0 };
  }
}

async function saveState(state) {
  await fsp.mkdir(WAVE_DIR, { recursive: true });
  await fsp.writeFile(STATE_PATH, JSON.stringify(state, null, 2), "utf8");
}

// ─── File selection ─────────────────────────────────────────────────────────
async function selectFiles(state) {
  const entries = await fsp.readdir(KNOWLEDGE_ROOT);
  const mdFiles = entries.filter(
    (f) => f.endsWith(".md") && !f.startsWith("_"),
  );

  // Partition: never-audited first, then oldest-audited
  const neverAudited = [];
  const audited = [];
  for (const f of mdFiles) {
    if (!state.files[f]) {
      neverAudited.push({ filename: f, lastAudited: 0 });
    } else {
      audited.push({
        filename: f,
        lastAudited: new Date(state.files[f].lastAudited).getTime(),
      });
    }
  }
  audited.sort((a, b) => a.lastAudited - b.lastAudited);

  const candidates = [...neverAudited, ...audited];
  return candidates.slice(0, BATCH_SIZE).map((c) => c.filename);
}

// ─── LLM API ───────────────────────────────────────────────────────────────
function chatCompletion(messages, retries = 3) {
  return new Promise((resolve, reject) => {
    const url = new URL(`${API_BASE}/chat/completions`);
    const transport = url.protocol === "https:" ? https : http;
    const body = JSON.stringify({
      model: MODEL,
      messages,
      temperature: 0.1,
      response_format: { type: "json_object" },
      max_tokens: 2000,
    });

    const reqOptions = {
      hostname: url.hostname,
      port: url.port || (url.protocol === "https:" ? 443 : 80),
      path: url.pathname + url.search,
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${API_KEY}`,
        "Content-Length": Buffer.byteLength(body),
      },
    };

    const req = transport.request(reqOptions, (res) => {
      let data = "";
      res.on("data", (chunk) => (data += chunk));
      res.on("end", () => {
        if (res.statusCode === 429 && retries > 0) {
          const delay = parseInt(res.headers["retry-after"] || "5", 10) * 1000;
          setTimeout(() => chatCompletion(messages, retries - 1).then(resolve, reject), delay);
          return;
        }
        if (res.statusCode >= 400) {
          reject(new Error(`API ${res.statusCode}: ${data.slice(0, 500)}`));
          return;
        }
        try {
          const parsed = JSON.parse(data);
          const content = parsed.choices?.[0]?.message?.content;
          resolve(content ? JSON.parse(content) : null);
        } catch (err) {
          reject(new Error(`Failed to parse API response: ${err.message}`));
        }
      });
    });

    req.on("error", (err) => {
      if (retries > 0) {
        setTimeout(() => chatCompletion(messages, retries - 1).then(resolve, reject), 3000);
      } else {
        reject(err);
      }
    });

    req.write(body);
    req.end();
  });
}

// ─── Audit prompt ───────────────────────────────────────────────────────────
const SYSTEM_PROMPT = `You are a technical encyclopedia fact-checker with expertise across software engineering. Your job is to review knowledge base articles for accuracy. These articles are used by AI coding assistants — if they contain errors, millions of developers may receive bad advice.

Be CONSERVATIVE. Only flag issues you are confident about. Do not flag stylistic preferences or subjective matters. Focus on:

1. FACTUAL ERRORS: Wrong algorithms, wrong complexity claims, incorrect attributions, wrong behavior descriptions.
2. OUTDATED INFORMATION: Version numbers that have changed, APIs that were deprecated, tools that no longer exist, ecosystem shifts.
3. MISSING CRITICAL CONTEXT: Important caveats omitted, security implications not mentioned, major alternatives ignored.
4. UNVERIFIABLE CLAIMS: Statements that sound authoritative but cannot be traced to a standard reference.

Do NOT flag:
- Stylistic choices or writing quality (unless genuinely misleading)
- Minor omissions that don't affect correctness
- Topics that could be deeper (every article has scope limits)
- Subjective trade-off assessments that are clearly presented as opinions

Return JSON with this exact schema:
{
  "status": "pass" | "needs-update" | "needs-rewrite",
  "confidence": 0.0 to 1.0,
  "issues": [
    {
      "type": "factual" | "outdated" | "incomplete" | "unverifiable",
      "severity": "critical" | "major" | "minor",
      "location": "approximate heading or section where the issue is",
      "description": "what is wrong — be specific",
      "suggestion": "what the correct information should be"
    }
  ],
  "summary": "one-line assessment of the article"
}

If the article is accurate, return status "pass" with an empty issues array.
Err on the side of "pass" — only flag what you are genuinely confident is wrong or outdated.`;

async function auditFile(filename) {
  const filepath = path.join(KNOWLEDGE_ROOT, filename);
  const content = await fsp.readFile(filepath, "utf8");

  // Truncate very long files to stay within token limits
  const truncated = content.length > 15000 ? content.slice(0, 15000) + "\n\n[...truncated for review]" : content;

  const messages = [
    { role: "system", content: SYSTEM_PROMPT },
    {
      role: "user",
      content: `Review this knowledge base article for factual accuracy and freshness.\n\nFilename: ${filename}\n\n---\n${truncated}\n---\n\nReturn your assessment as JSON.`,
    },
  ];

  return chatCompletion(messages);
}

// ─── Wave file generation ───────────────────────────────────────────────────
function buildWaveFile(findings, date) {
  // Group findings by severity for assignment ordering
  const critical = findings.filter((f) =>
    f.issues.some((i) => i.severity === "critical"),
  );
  const major = findings.filter(
    (f) =>
      !f.issues.some((i) => i.severity === "critical") &&
      f.issues.some((i) => i.severity === "major"),
  );
  const minor = findings.filter(
    (f) =>
      f.issues.every((i) => i.severity === "minor"),
  );

  const assignments = [];
  let id = 1;

  function addGroup(label, items) {
    if (!items.length) return;
    const topics = items.map((item) => ({
      filename: item.filename,
      description: item.issues
        .map((i) => `[${i.severity}/${i.type}] ${i.description} → ${i.suggestion}`)
        .join(" | "),
    }));
    assignments.push({ id: id++, label, topics });
  }

  addGroup("Critical Fixes — Factual Errors", critical);
  addGroup("Major Updates — Outdated or Incomplete", major);
  addGroup("Minor Improvements", minor);

  if (!assignments.length) return null;

  return {
    wave: `audit-${date}`,
    description: `Automated audit findings from ${date}. ${critical.length} critical, ${major.length} major, ${minor.length} minor issues across ${findings.length} files.`,
    agent: "KnowledgeBuilder",
    prefix:
      "AUDIT REMEDIATION — These are corrections and updates identified by automated fact-checking. " +
      "For each topic, the existing file needs to be UPDATED (not rewritten from scratch unless marked needs-rewrite). " +
      "Research the specific issues flagged, verify against current authoritative sources, and fix only what is wrong. " +
      "Do not rewrite correct sections. Use update_knowledge_note to patch specific sections.\n\n" +
      "ACCURACY IS PARAMOUNT. These articles are used by AI assistants serving millions of developers. " +
      "Every claim must be verifiable against official docs, RFCs, or widely-cited references.",
    suffix:
      "Fix only the flagged issues. Verify each fix against current authoritative sources. " +
      "Use update_knowledge_note for targeted section updates rather than rewriting entire files.",
    assignments,
  };
}

// ─── Main ───────────────────────────────────────────────────────────────────
async function main() {
  if (!API_KEY) {
    console.error("AUDIT_API_KEY is required. Set it as an environment variable or GitHub secret.");
    process.exit(1);
  }

  const state = await loadState();
  state.runCount = (state.runCount || 0) + 1;

  const files = await selectFiles(state);
  if (!files.length) {
    console.log("No files to audit.");
    process.exit(0);
  }

  console.log(`Audit run #${state.runCount} — reviewing ${files.length} files with ${MODEL}`);

  const now = new Date().toISOString();
  const date = now.slice(0, 10);
  const findings = [];
  let passCount = 0;
  let errorCount = 0;

  for (const filename of files) {
    process.stdout.write(`  ${filename} ... `);
    try {
      const result = await auditFile(filename);
      if (!result) {
        console.log("no response");
        errorCount++;
        continue;
      }

      // Update state regardless of result
      state.files[filename] = {
        lastAudited: now,
        status: result.status,
        issueCount: (result.issues || []).length,
        confidence: result.confidence,
        summary: result.summary,
      };

      if (result.status === "pass") {
        console.log("PASS");
        passCount++;
      } else {
        const issues = result.issues || [];
        const critCount = issues.filter((i) => i.severity === "critical").length;
        const majCount = issues.filter((i) => i.severity === "major").length;
        console.log(
          `${result.status.toUpperCase()} — ${issues.length} issues (${critCount} critical, ${majCount} major)`,
        );
        if (issues.length > 0) {
          findings.push({ filename, status: result.status, issues });
        }
      }
    } catch (err) {
      console.log(`ERROR: ${err.message}`);
      errorCount++;
    }

    // Small delay to respect rate limits
    await new Promise((r) => setTimeout(r, 1000));
  }

  // Check if we've audited every file (full pass complete)
  const allFiles = (await fsp.readdir(KNOWLEDGE_ROOT)).filter(
    (f) => f.endsWith(".md") && !f.startsWith("_"),
  );
  const auditedCount = Object.keys(state.files).filter((f) =>
    allFiles.includes(f),
  ).length;
  if (auditedCount >= allFiles.length) {
    state.lastFullPass = now;
    console.log(`\nFull pass complete — all ${allFiles.length} files audited at least once.`);
  }

  // Write wave file if there are findings
  if (findings.length > 0) {
    const wave = buildWaveFile(findings, date);
    if (wave) {
      await fsp.mkdir(WAVE_DIR, { recursive: true });
      const wavePath = path.join(WAVE_DIR, `audit-${date}.json`);
      // Append run count if file already exists for today
      const finalPath = fs.existsSync(wavePath)
        ? path.join(WAVE_DIR, `audit-${date}-run${state.runCount}.json`)
        : wavePath;
      await fsp.writeFile(finalPath, JSON.stringify(wave, null, 2), "utf8");
      console.log(`\nWave file written: ${path.relative(REPO_ROOT, finalPath)}`);
      console.log(
        `  ${wave.assignments.reduce((n, a) => n + a.topics.length, 0)} remediation topics across ${wave.assignments.length} priority groups`,
      );
    }
  }

  await saveState(state);

  // Summary
  console.log(
    `\nSummary: ${passCount} pass, ${findings.length} need work, ${errorCount} errors`,
  );
  console.log(
    `Progress: ${auditedCount}/${allFiles.length} files audited (${Math.round((auditedCount / allFiles.length) * 100)}%)`,
  );
}

main().catch((err) => {
  console.error("Auditor failed:", err.message);
  process.exit(1);
});
