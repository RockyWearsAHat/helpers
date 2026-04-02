#!/usr/bin/env node
"use strict";

const fs = require("fs/promises");
const path = require("path");

const REPO_ROOT = path.resolve(__dirname, "..");
const SITE_SOURCE_ROOT = path.join(REPO_ROOT, "pages");
const OUTPUT_ROOT = path.join(REPO_ROOT, "build", "pages-search");
const OUTPUT_DATA_PATH = path.join(OUTPUT_ROOT, "data", "notes-search.json");
const COMMUNITY_MANIFEST_PATH = path.join(
  REPO_ROOT,
  "community-cache",
  "manifest.json",
);
const REPO_WEB_BASE =
  process.env.GSH_PAGES_REPO_URL ||
  "https://github.com/RockyWearsAHat/github-shell-helpers";
const REPO_RAW_BASE =
  process.env.GSH_PAGES_RAW_BASE ||
  "https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers";
const REPO_BRANCH = process.env.GSH_PAGES_BRANCH || "main";

const STOPWORDS = new Set([
  "a",
  "about",
  "after",
  "all",
  "also",
  "an",
  "and",
  "any",
  "are",
  "as",
  "at",
  "be",
  "because",
  "been",
  "before",
  "being",
  "between",
  "both",
  "but",
  "by",
  "can",
  "could",
  "did",
  "do",
  "does",
  "doing",
  "down",
  "during",
  "each",
  "few",
  "for",
  "from",
  "further",
  "get",
  "got",
  "had",
  "has",
  "have",
  "having",
  "how",
  "if",
  "in",
  "into",
  "is",
  "it",
  "its",
  "itself",
  "just",
  "like",
  "may",
  "more",
  "most",
  "must",
  "need",
  "no",
  "nor",
  "not",
  "now",
  "of",
  "off",
  "on",
  "once",
  "only",
  "or",
  "other",
  "our",
  "out",
  "over",
  "own",
  "same",
  "should",
  "so",
  "some",
  "such",
  "than",
  "that",
  "the",
  "their",
  "them",
  "then",
  "there",
  "these",
  "they",
  "this",
  "those",
  "through",
  "to",
  "too",
  "under",
  "until",
  "up",
  "use",
  "used",
  "using",
  "very",
  "via",
  "was",
  "we",
  "were",
  "what",
  "when",
  "where",
  "which",
  "while",
  "who",
  "will",
  "with",
  "would",
  "you",
  "your",
]);

const CATEGORY_LABELS = {
  ai: "AI",
  api: "API",
  auth: "Auth",
  ci: "CI",
  cli: "CLI",
  css: "CSS",
  ddd: "DDD",
  genai: "GenAI",
  ide: "IDE",
  iot: "IoT",
  lsp: "LSP",
  ml: "ML",
  os: "OS",
  sre: "SRE",
  svg: "SVG",
  web3: "Web3",
};

const MARKDOWN_SOURCES = [
  {
    rootType: "directory",
    root: path.join(REPO_ROOT, "knowledge"),
    pathPrefix: "knowledge",
    scopeKey: "knowledge",
    scopeLabel: "Knowledge atlas",
  },
  {
    rootType: "directory",
    root: path.join(REPO_ROOT, "research-sources", "legacy-root-dumps"),
    pathPrefix: "research-sources/legacy-root-dumps",
    scopeKey: "archive",
    scopeLabel: "Source archive",
    categoryLabel: "Source Archive",
  },
  {
    rootType: "file",
    root: path.join(
      REPO_ROOT,
      "copilot-config",
      "skills",
      "copilot-research",
      "studybase.md",
    ),
    pathPrefix: "copilot-config/skills/copilot-research",
    scopeKey: "copilot",
    scopeLabel: "Copilot guide",
    categoryLabel: "Copilot Studybase",
  },
  {
    rootType: "file",
    root: path.join(
      REPO_ROOT,
      "copilot-config",
      "devops-audit-community-cache.md",
    ),
    pathPrefix: "copilot-config",
    scopeKey: "copilot",
    scopeLabel: "Copilot guide",
    categoryLabel: "Copilot Cache Contract",
  },
];

const COMMUNITY_PACK_DEFINITIONS = [
  {
    manifestKey: "promptingPrinciples",
    arrayKey: "principles",
    category: "Copilot Principle",
    kindLabel: "Principle",
    title: (item) => `Principle ${item.id} — ${summarize(item.statement, 88)}`,
    previewText: (item) => item.statement,
    highlights: (item) => [
      item.applicability
        ? `Applicability: ${titleCase(item.applicability)}`
        : "",
      item.recommendationStrength
        ? `Strength: ${titleCase(item.recommendationStrength)}`
        : "",
      item.authoritativeSupport
        ? `Support: ${titleCase(item.authoritativeSupport)}`
        : "",
    ],
    topics: (item) => item.topics || [],
    resourceLinks: (item) => normalizeEvidenceLinks(item.evidenceRefs),
  },
  {
    manifestKey: "applicationPractices",
    arrayKey: "practices",
    category: "Copilot Practice",
    kindLabel: "Practice",
    title: (item) => `Practice ${item.id} — ${summarize(item.statement, 88)}`,
    previewText: (item) => item.statement,
    highlights: (item) => [
      item.applicability
        ? `Applicability: ${titleCase(item.applicability)}`
        : "",
      item.recommendationStrength
        ? `Strength: ${titleCase(item.recommendationStrength)}`
        : "",
      item.authoritativeSupport
        ? `Support: ${titleCase(item.authoritativeSupport)}`
        : "",
    ],
    topics: (item) => item.topics || [],
    resourceLinks: (item) => normalizeEvidenceLinks(item.evidenceRefs),
  },
  {
    manifestKey: "antiPatterns",
    arrayKey: "antiPatterns",
    category: "Copilot Anti-pattern",
    kindLabel: "Anti-pattern",
    title: (item) =>
      `Anti-pattern ${item.id} — ${summarize(item.statement, 88)}`,
    previewText: (item) => item.statement,
    highlights: (item) => [
      item.applicability
        ? `Applicability: ${titleCase(item.applicability)}`
        : "",
      item.recommendationStrength
        ? `Severity: ${titleCase(item.recommendationStrength)}`
        : "",
      item.authoritativeSupport
        ? `Support: ${titleCase(item.authoritativeSupport)}`
        : "",
    ],
    topics: (item) => item.topics || [],
    resourceLinks: (item) => normalizeEvidenceLinks(item.evidenceRefs),
  },
  {
    manifestKey: "workflowPatterns",
    arrayKey: "patterns",
    category: "Copilot Workflow Pattern",
    kindLabel: "Workflow pattern",
    title: (item) => item.name,
    previewText: (item) => item.description,
    highlights: (item) => [
      item.applicability
        ? `Applicability: ${titleCase(item.applicability)}`
        : "",
      ...(item.exemplars || []).map((exemplar) => `Example: ${exemplar}`),
    ],
    topics: (item) => item.topics || [],
    resourceLinks: (item) => normalizeEvidenceLinks(item.evidenceRefs),
  },
  {
    manifestKey: "deprecations",
    arrayKey: "deprecations",
    category: "Copilot Deprecation",
    kindLabel: "Deprecation",
    title: (item) => `Deprecation ${item.id} — ${summarize(item.what, 88)}`,
    previewText: (item) =>
      [
        item.what,
        item.replacedBy ? `Use ${item.replacedBy} instead.` : "",
        item.impact || "",
      ]
        .filter(Boolean)
        .join(" "),
    highlights: (item) => [
      item.severity ? `Severity: ${titleCase(item.severity)}` : "",
      item.detectedIn ? `Detected in: ${item.detectedIn}` : "",
      item.replacedBy ? `Replacement: ${item.replacedBy}` : "",
    ],
    topics: () => ["deprecations", "copilot"],
    resourceLinks: (item) => normalizeEvidenceLinks(item.evidenceRefs),
  },
  {
    manifestKey: "officialSources",
    arrayKey: "sources",
    category: "Copilot Official Source",
    kindLabel: "Official source",
    title: (item) => item.title,
    previewText: (item) =>
      item.whyItMatters || item.covers || item.description || "",
    highlights: (item) => [
      item.type ? `Type: ${titleCase(item.type)}` : "",
      item.confidence ? `Confidence: ${titleCase(item.confidence)}` : "",
      item.freshness ? `Freshness: ${item.freshness}` : "",
    ],
    topics: (item) => item.topics || [],
    resourceLinks: (item) =>
      dedupeLinks([
        item.url ? { label: item.title, url: item.url } : null,
        ...normalizeEvidenceLinks(item.evidenceRefs),
      ]),
  },
];

async function pathExists(targetPath) {
  try {
    await fs.access(targetPath);
    return true;
  } catch {
    return false;
  }
}

async function directoryExists(dirPath) {
  try {
    const stat = await fs.stat(dirPath);
    return stat.isDirectory();
  } catch {
    return false;
  }
}

async function readJson(filePath) {
  return JSON.parse(await fs.readFile(filePath, "utf8"));
}

async function collectMarkdownFiles(rootDir) {
  const files = [];

  async function walk(currentDir) {
    const entries = await fs.readdir(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        await walk(fullPath);
        continue;
      }
      if (entry.isFile() && entry.name.endsWith(".md")) {
        files.push(fullPath);
      }
    }
  }

  await walk(rootDir);
  files.sort((left, right) => left.localeCompare(right));
  return files;
}

function stripMarkdown(text) {
  return text
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/`([^`]+)`/g, " $1 ")
    .replace(/!\[[^\]]*\]\([^)]*\)/g, " ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, " $1 ")
    .replace(/^>\s?/gm, "")
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/^\s*[-*+]\s+/gm, "")
    .replace(/^\s*\d+\.\s+/gm, "")
    .replace(/\|/g, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/[~*_]/g, " ")
    .replace(/\r/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function summarize(text, maxLength) {
  if (!text) return "";
  if (text.length <= maxLength) return text;
  const truncated = text.slice(0, maxLength);
  const lastSpace = truncated.lastIndexOf(" ");
  return `${truncated.slice(0, lastSpace > 0 ? lastSpace : maxLength).trim()}...`;
}

function extractTitle(text, fallback) {
  const match = text.match(/^#\s+(.+)$/m);
  return match ? match[1].replace(/[*_`]/g, "").trim() : fallback;
}

function extractHeadings(text) {
  const headings = [];
  const regex = /^#{2,4}\s+(.+)$/gm;
  let match = regex.exec(text);
  while (match) {
    headings.push(match[1].replace(/[*_`]/g, "").trim());
    if (headings.length >= 8) break;
    match = regex.exec(text);
  }
  return headings;
}

function extractParagraphs(text) {
  const withoutCodeBlocks = text.replace(/```[\s\S]*?```/g, "\n\n");
  return withoutCodeBlocks
    .split(/\n\s*\n/)
    .map((block) => stripMarkdown(block))
    .filter((block) => block.length >= 48);
}

function tokenize(text) {
  return text
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((token) => token.length >= 3 && !STOPWORDS.has(token));
}

function extractKeywords(text) {
  const frequencies = new Map();
  for (const token of tokenize(text)) {
    frequencies.set(token, (frequencies.get(token) || 0) + 1);
  }

  return [...frequencies.entries()]
    .sort((left, right) => {
      if (right[1] !== left[1]) return right[1] - left[1];
      return left[0].localeCompare(right[0]);
    })
    .slice(0, 18)
    .map(([token]) => token);
}

function titleCase(slug) {
  const lower = String(slug || "").toLowerCase();
  if (CATEGORY_LABELS[lower]) return CATEGORY_LABELS[lower];
  return lower
    .split(/[-_]/)
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join(" ");
}

function deriveCategory(relativePath, source) {
  if (source.categoryLabel) return source.categoryLabel;
  if (source.scopeKey === "archive") return "Source Archive";
  const fileName = path.basename(relativePath, ".md");
  return titleCase(fileName.split("-")[0] || "notes");
}

function buildGitHubUrls(relativePath) {
  const normalizedPath = relativePath.replace(/\\/g, "/");
  return {
    githubUrl: `${REPO_WEB_BASE}/blob/${REPO_BRANCH}/${normalizedPath}`,
    rawUrl: `${REPO_RAW_BASE}/${REPO_BRANCH}/${normalizedPath}`,
  };
}

function dedupeLinks(links) {
  const seen = new Set();
  return (links || [])
    .filter(Boolean)
    .filter((link) => link.url)
    .filter((link) => {
      if (seen.has(link.url)) return false;
      seen.add(link.url);
      return true;
    })
    .slice(0, 8);
}

function normalizeEvidenceLinks(evidenceRefs) {
  return dedupeLinks(
    (evidenceRefs || []).map((ref) => ({
      label: ref.title || ref.url,
      url: ref.url,
    })),
  );
}

function makeMarkdownDocument(source, relativePath, raw) {
  const title = extractTitle(raw, path.basename(relativePath, ".md"));
  const headings = extractHeadings(raw);
  const paragraphs = extractParagraphs(raw);
  const cleanText = stripMarkdown(raw);
  const keywords = extractKeywords(raw);
  const category = deriveCategory(relativePath, source);
  const previewText = summarize(
    paragraphs.slice(0, 3).join(" ") || cleanText,
    980,
  );
  const snippet = summarize(paragraphs[0] || cleanText, 260);
  const searchText = summarize(
    [title, ...headings, ...keywords, cleanText].join(" "),
    1800,
  );
  const { githubUrl, rawUrl } = buildGitHubUrls(relativePath);

  return {
    id: relativePath,
    title,
    path: relativePath,
    scopeKey: source.scopeKey,
    scopeLabel: source.scopeLabel,
    category,
    documentType: "note",
    snippet,
    previewText,
    headings,
    highlights: headings,
    keywords,
    topics: [],
    metaPills: [source.scopeLabel, category],
    resultPills: [source.scopeLabel, category, ...keywords.slice(0, 2)],
    searchText,
    githubUrl,
    rawUrl,
    resourceLinks: dedupeLinks([
      { label: "Open on GitHub", url: githubUrl },
      { label: "Open raw markdown", url: rawUrl },
    ]),
    relatedIds: [],
  };
}

async function buildMarkdownCorpus() {
  const documents = [];

  for (const source of MARKDOWN_SOURCES) {
    const exists =
      source.rootType === "directory"
        ? await directoryExists(source.root)
        : await pathExists(source.root);
    if (!exists) continue;

    const markdownFiles =
      source.rootType === "file"
        ? [source.root]
        : await collectMarkdownFiles(source.root);

    for (const absolutePath of markdownFiles) {
      const relativeWithinSource =
        source.rootType === "file"
          ? path.basename(absolutePath)
          : path.relative(source.root, absolutePath);
      const relativePath = path
        .join(source.pathPrefix, relativeWithinSource)
        .replace(/\\/g, "/");
      const raw = await fs.readFile(absolutePath, "utf8");
      documents.push(makeMarkdownDocument(source, relativePath, raw));
    }
  }

  return documents;
}

function makeCommunityDocument({
  entryId,
  title,
  previewText,
  relativePath,
  category,
  kindLabel,
  topics = [],
  highlights = [],
  metaPills = [],
  resourceLinks = [],
  communityContent = null,
}) {
  const { githubUrl, rawUrl } = buildGitHubUrls(relativePath);
  const cleanHighlights = highlights.filter(Boolean).slice(0, 8);
  const cleanTopics = topics.filter(Boolean).map((topic) => String(topic));
  const defaultMeta = [
    "Copilot guide",
    category,
    kindLabel,
    ...metaPills,
  ].filter(Boolean);
  const searchText = summarize(
    [
      title,
      previewText,
      ...cleanTopics,
      ...cleanHighlights,
      ...defaultMeta,
    ].join(" "),
    1800,
  );

  return {
    id: `copilot:${entryId}`,
    title,
    path: `${relativePath} · ${entryId}`,
    scopeKey: "copilot",
    scopeLabel: "Copilot guide",
    category,
    documentType: "community",
    snippet: summarize(previewText, 260),
    previewText: summarize(previewText, 980),
    headings: [],
    highlights: cleanHighlights,
    keywords: extractKeywords(searchText),
    topics: cleanTopics,
    metaPills: defaultMeta,
    resultPills: [category, kindLabel, ...cleanTopics.slice(0, 2)].filter(
      Boolean,
    ),
    searchText,
    githubUrl,
    rawUrl,
    resourceLinks: dedupeLinks([
      ...resourceLinks,
      { label: "Open source pack", url: githubUrl },
      { label: "Open raw JSON", url: rawUrl },
    ]),
    relatedIds: [],
    communityContent,
  };
}

function normalizeCommunityPack(pack, relativePath, definition) {
  return (pack[definition.arrayKey] || []).map((item) =>
    makeCommunityDocument({
      entryId: item.id || item.name || item.title,
      title: definition.title(item),
      previewText: definition.previewText(item),
      relativePath,
      category: definition.category,
      kindLabel: definition.kindLabel,
      topics: definition.topics(item),
      highlights: definition.highlights(item),
      metaPills: [
        item.applicability ? titleCase(item.applicability) : "",
        item.recommendationStrength
          ? titleCase(item.recommendationStrength)
          : "",
        item.authoritativeSupport
          ? `${titleCase(item.authoritativeSupport)} support`
          : "",
      ],
      resourceLinks: definition.resourceLinks(item),
      communityContent: {
        kind: definition.kindLabel,
        category: definition.category,
        item,
      },
    }),
  );
}

function normalizeCommunityResources(pack, relativePath) {
  const documents = [];

  for (const repository of pack.repositories || []) {
    documents.push(
      makeCommunityDocument({
        entryId: `repo-${repository.name}`,
        title: repository.name,
        previewText: repository.description,
        relativePath,
        category: "Copilot Resource",
        kindLabel: titleCase(repository.type || "repository"),
        topics: ["copilot", "resources"],
        highlights: [
          ...(repository.whatToUse || []).map((item) => `Use: ${item}`),
          ...(repository.whatToAvoid || []).map((item) => `Avoid: ${item}`),
        ],
        metaPills: [repository.maintainer || ""],
        resourceLinks: dedupeLinks([
          repository.url
            ? { label: repository.name, url: repository.url }
            : null,
        ]),
        communityContent: {
          kind: titleCase(repository.type || "repository"),
          category: "Copilot Resource",
          item: repository,
        },
      }),
    );
  }

  for (const doc of pack.officialDocumentation || []) {
    documents.push(
      makeCommunityDocument({
        entryId: `doc-${doc.title}`,
        title: doc.title,
        previewText: doc.covers,
        relativePath,
        category: "Copilot Resource",
        kindLabel: "Official docs",
        topics: ["copilot", "official docs"],
        highlights: [doc.freshness ? `Freshness: ${doc.freshness}` : ""],
        resourceLinks: dedupeLinks([
          doc.url ? { label: doc.title, url: doc.url } : null,
        ]),
        communityContent: {
          kind: "Official docs",
          category: "Copilot Resource",
          item: doc,
        },
      }),
    );
  }

  for (const command of pack.vsCodeCommands || []) {
    documents.push(
      makeCommunityDocument({
        entryId: `command-${command.command}`,
        title: command.command,
        previewText: command.description,
        relativePath,
        category: "Copilot Resource",
        kindLabel: "VS Code command",
        topics: ["copilot", "vscode", "commands"],
        communityContent: {
          kind: "VS Code command",
          category: "Copilot Resource",
          item: command,
        },
      }),
    );
  }

  for (const [key, steps] of Object.entries(pack.usageGuidelines || {})) {
    documents.push(
      makeCommunityDocument({
        entryId: `guide-${key}`,
        title: `Copilot resource guide — ${titleCase(key)}`,
        previewText: (steps || []).join(" "),
        relativePath,
        category: "Copilot Resource Guide",
        kindLabel: "Guide",
        topics: ["copilot", "workflow", "guide"],
        highlights: steps || [],
        communityContent: {
          kind: "Guide",
          category: "Copilot Resource Guide",
          item: { title: `Copilot resource guide — ${titleCase(key)}`, steps },
        },
      }),
    );
  }

  return documents;
}

async function buildCommunityCacheCorpus() {
  if (!(await pathExists(COMMUNITY_MANIFEST_PATH))) return [];

  const communityManifest = await readJson(COMMUNITY_MANIFEST_PATH);
  const snapshotManifestPath = path.join(
    REPO_ROOT,
    communityManifest.snapshotManifest || "",
  );
  if (!(await pathExists(snapshotManifestPath))) return [];

  const snapshotManifest = await readJson(snapshotManifestPath);
  const documents = [];

  for (const definition of COMMUNITY_PACK_DEFINITIONS) {
    const relativePath = snapshotManifest.files?.[definition.manifestKey];
    if (!relativePath) continue;
    const absolutePath = path.join(REPO_ROOT, relativePath);
    if (!(await pathExists(absolutePath))) continue;
    const pack = await readJson(absolutePath);
    documents.push(...normalizeCommunityPack(pack, relativePath, definition));
  }

  const communityResourcesPath = snapshotManifest.files?.communityResources;
  if (communityResourcesPath) {
    const absolutePath = path.join(REPO_ROOT, communityResourcesPath);
    if (await pathExists(absolutePath)) {
      const pack = await readJson(absolutePath);
      documents.push(
        ...normalizeCommunityResources(pack, communityResourcesPath),
      );
    }
  }

  return documents;
}

function buildSignalSet(document) {
  return new Set(
    [
      ...document.keywords.slice(0, 12),
      ...(document.topics || []).map((topic) => String(topic).toLowerCase()),
      ...String(document.category || "")
        .toLowerCase()
        .split(/[^a-z0-9]+/)
        .filter((token) => token.length >= 3 && !STOPWORDS.has(token)),
    ].filter(Boolean),
  );
}

function countOverlap(left, right) {
  const [smaller, larger] =
    left.size <= right.size ? [left, right] : [right, left];
  let overlap = 0;
  for (const token of smaller) {
    if (larger.has(token)) overlap += 1;
  }
  return overlap;
}

function attachRelatedDocuments(documents) {
  const signalSets = new Map(
    documents.map((document) => [document.id, buildSignalSet(document)]),
  );

  for (const document of documents) {
    const currentSignals = signalSets.get(document.id);
    const scored = [];

    for (const candidate of documents) {
      if (candidate.id === document.id) continue;
      const overlap = countOverlap(
        currentSignals,
        signalSets.get(candidate.id),
      );
      let score = overlap * 1.1;
      if (document.scopeKey === candidate.scopeKey) score += 0.9;
      if (document.category === candidate.category) score += 1.4;
      if (document.documentType === candidate.documentType) score += 0.35;
      if (score > 1.45) {
        scored.push({ id: candidate.id, score });
      }
    }

    scored.sort(
      (left, right) =>
        right.score - left.score || left.id.localeCompare(right.id),
    );
    document.relatedIds = scored.slice(0, 4).map((entry) => entry.id);
  }

  return documents;
}

function buildMetadata(documents) {
  const scopeCounts = {};
  const categoryCounts = {};

  for (const document of documents) {
    scopeCounts[document.scopeKey] = (scopeCounts[document.scopeKey] || 0) + 1;
    categoryCounts[document.category] =
      (categoryCounts[document.category] || 0) + 1;
  }

  const featuredCategories = Object.entries(categoryCounts)
    .sort((left, right) => {
      if (right[1] !== left[1]) return right[1] - left[1];
      return left[0].localeCompare(right[0]);
    })
    .slice(0, 12)
    .map(([label, count]) => ({ label, count }));

  return {
    builtAt: new Date().toISOString(),
    totalDocuments: documents.length,
    scopeCounts,
    featuredCategories,
    repositoryUrl: REPO_WEB_BASE,
    branch: REPO_BRANCH,
  };
}

async function copySiteSource() {
  await fs.rm(OUTPUT_ROOT, { recursive: true, force: true });
  await fs.mkdir(OUTPUT_ROOT, { recursive: true });
  await fs.cp(SITE_SOURCE_ROOT, OUTPUT_ROOT, { recursive: true });
  const indexHtml = await fs.readFile(
    path.join(SITE_SOURCE_ROOT, "index.html"),
    "utf8",
  );
  await fs.writeFile(path.join(OUTPUT_ROOT, "404.html"), indexHtml, "utf8");
  await fs.writeFile(path.join(OUTPUT_ROOT, ".nojekyll"), "", "utf8");
}

async function main() {
  if (!(await directoryExists(SITE_SOURCE_ROOT))) {
    throw new Error(`Missing site source directory: ${SITE_SOURCE_ROOT}`);
  }

  const documents = attachRelatedDocuments(
    [
      ...(await buildMarkdownCorpus()),
      ...(await buildCommunityCacheCorpus()),
    ].sort((left, right) => left.title.localeCompare(right.title)),
  );

  if (!documents.length) {
    throw new Error("No documents found for the public search corpus.");
  }

  const scopeKeys = new Set(documents.map((document) => document.scopeKey));
  if (!scopeKeys.has("knowledge")) {
    throw new Error(
      "Pages search corpus is missing the knowledge atlas scope.",
    );
  }
  if (!scopeKeys.has("copilot")) {
    throw new Error(
      "Pages search corpus is missing the Copilot guidance scope.",
    );
  }

  await copySiteSource();
  await fs.mkdir(path.dirname(OUTPUT_DATA_PATH), { recursive: true });

  const payload = {
    metadata: buildMetadata(documents),
    documents,
  };

  await fs.writeFile(OUTPUT_DATA_PATH, JSON.stringify(payload), "utf8");
  process.stdout.write(
    `Built GitHub Pages search site: ${documents.length} documents -> ${OUTPUT_ROOT}\n`,
  );
}

main().catch((error) => {
  process.stderr.write(`build-pages-search-site failed: ${error.message}\n`);
  process.exit(1);
});
