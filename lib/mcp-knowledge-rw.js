"use strict";
// lib/mcp-knowledge-rw.js — Knowledge read/write, cache, and community research
const fs = require("fs/promises");
const fsSync = require("fs");
const path = require("path");
const { execFile: defaultExecFile } = require("child_process");

module.exports = function createKnowledgeRW(deps) {
  const {
    WORKSPACE_ROOT,
    KNOWLEDGE_ROOT,
    REPO_KNOWLEDGE_ROOT,
    LOCAL_INDEX_PATH,
    GITHUB_RAW_BASE,
    GITHUB_API_BASE,
    GITHUB_CACHE_DIR,
    CACHE_META_PATH,
    INDEX_MAX_AGE_MS,
    DEFAULT_USER_AGENT,
    escapeRegExp,
    tokenizeQuery,
    getMarkdownTitle,
    summarizeInline,
    summarizeText,
    sleep,
    fetchJson,
    fetchText,
    toPositiveInt,
    buildKnowledgeIndex,
    buildKnowledgeSnippet,
    scoreKnowledgeMatch,
    collectMarkdownFiles,
    execFileImpl = defaultExecFile,
    homeDir = process.env.HOME || process.env.USERPROFILE || "",
  } = deps;

  let _cacheMeta = null;

  function loadCacheMeta() {
    if (_cacheMeta) return _cacheMeta;
    try {
      _cacheMeta = JSON.parse(fsSync.readFileSync(CACHE_META_PATH, "utf8"));
    } catch {
      _cacheMeta = { etags: {}, fetched_at: {} };
    }
    return _cacheMeta;
  }

  function saveCacheMeta() {
    if (!_cacheMeta) return;
    try {
      fsSync.writeFileSync(CACHE_META_PATH, JSON.stringify(_cacheMeta), "utf8");
    } catch {
      /* best effort */
    }
  }

  function isCacheFresh(cacheKey) {
    const meta = loadCacheMeta();
    const ts = meta.fetched_at[cacheKey];
    return ts && Date.now() - ts < INDEX_MAX_AGE_MS;
  }

  function getCommunitySettingsPaths() {
    return {
      global: path.join(
        homeDir,
        ".copilot",
        "devops-audit-community-settings.json",
      ),
      workspace: path.join(
        WORKSPACE_ROOT,
        ".github",
        "devops-audit-community-settings.json",
      ),
    };
  }

  async function readJsonFileIfExists(filePath) {
    try {
      return JSON.parse(await fs.readFile(filePath, "utf8"));
    } catch (error) {
      if (error && error.code === "ENOENT") {
        return null;
      }
      throw new Error(`Failed to read ${filePath}: ${error.message}`);
    }
  }

  async function loadKnowledgeSharingSettings() {
    const { global, workspace } = getCommunitySettingsPaths();
    const globalSettings = await readJsonFileIfExists(global);
    const workspaceSettings = await readJsonFileIfExists(workspace);

    return {
      ...(globalSettings || {}),
      ...(workspaceSettings || {}),
    };
  }

  function isKnowledgeSharingEnabled(settings) {
    if (typeof settings.shareKnowledge === "boolean") {
      return settings.shareKnowledge;
    }
    if (typeof settings.shareResearch === "boolean") {
      return settings.shareResearch;
    }
    return false;
  }

  async function rebuildLocalKnowledgeIndex() {
    try {
      const result = await buildKnowledgeIndex({});
      return {
        status: "rebuilt",
        path: result.path,
        file_count: result.file_count,
        term_count: result.term_count,
      };
    } catch (error) {
      return {
        status: "failed",
        message: error.message,
      };
    }
  }

  async function runCommunityResearchSubmit(resolvedPath) {
    const scriptPath = path.join(
      __dirname,
      "..",
      "scripts",
      "community-research-submit.sh",
    );

    return new Promise((resolve, reject) => {
      execFileImpl(
        "bash",
        [scriptPath, resolvedPath],
        { cwd: WORKSPACE_ROOT, timeout: 120000 },
        (err, stdout, stderr) => {
          if (err) {
            const msg = (stderr || err.message || "").trim();
            reject(new Error(`Community research submit failed: ${msg}`));
            return;
          }

          resolve({
            action: "submitted",
            path: path.relative(WORKSPACE_ROOT, resolvedPath),
            output: (stderr || stdout || "").trim(),
          });
        },
      );
    });
  }

  async function maybePublishKnowledgeNote(args, resolvedPath, index) {
    if (!Object.prototype.hasOwnProperty.call(args, "publish")) {
      return {
        requested: false,
        status: "local-only",
        message:
          "Note kept local. Set publish=true to submit it to the shared knowledge base.",
      };
    }

    if (!args.publish) {
      return {
        requested: false,
        status: "local-only",
        message: "Note kept local by request.",
      };
    }

    if (!index || index.status !== "rebuilt") {
      return {
        requested: true,
        status: "blocked",
        message:
          "Local knowledge index rebuild failed, so the note was not published.",
      };
    }

    let settings;
    try {
      settings = await loadKnowledgeSharingSettings();
    } catch (error) {
      return {
        requested: true,
        status: "failed",
        message: error.message,
      };
    }

    if (!isKnowledgeSharingEnabled(settings)) {
      return {
        requested: true,
        status: "blocked",
        message:
          "Knowledge sharing is not enabled. Set shareKnowledge: true (or legacy shareResearch: true) in community settings.",
      };
    }

    try {
      const result = await runCommunityResearchSubmit(resolvedPath);
      return {
        requested: true,
        status: "submitted",
        path: result.path,
        output: result.output,
      };
    } catch (error) {
      return {
        requested: true,
        status: "failed",
        message: error.message,
      };
    }
  }

  async function finalizeKnowledgeWrite(result, resolvedPath, args) {
    const index = await rebuildLocalKnowledgeIndex();
    const publish = await maybePublishKnowledgeNote(args, resolvedPath, index);
    return {
      ...result,
      index,
      publish,
    };
  }

  /**
   * Fetch a file from GitHub raw content with ETag caching to ~/.cache/gsh/.
   * Returns { text, fromCache }.
   */
  async function fetchGitHubFile(repoPath) {
    const meta = loadCacheMeta();
    const cacheFile = path.join(
      GITHUB_CACHE_DIR,
      repoPath.replace(/\//g, "__"),
    );
    const url = `${GITHUB_RAW_BASE}/${repoPath}`;

    const headers = { "User-Agent": "gsh-mcp/1.0" };
    if (meta.etags[repoPath]) headers["If-None-Match"] = meta.etags[repoPath];

    try {
      const resp = await fetch(url, {
        headers,
        signal: AbortSignal.timeout(15000),
      });
      if (resp.status === 304) {
        meta.fetched_at[repoPath] = Date.now();
        saveCacheMeta();
        const cached = await fs.readFile(cacheFile, "utf8");
        return { text: cached, fromCache: true };
      }
      if (!resp.ok) throw new Error(`HTTP ${resp.status} fetching ${repoPath}`);
      const text = await resp.text();
      const etag = resp.headers.get("etag");
      if (etag) meta.etags[repoPath] = etag;
      meta.fetched_at[repoPath] = Date.now();
      saveCacheMeta();
      await fs.mkdir(path.dirname(cacheFile), { recursive: true });
      await fs.writeFile(cacheFile, text, "utf8");
      return { text, fromCache: false };
    } catch (err) {
      // Network failure — try local cache fallback
      try {
        const cached = await fs.readFile(cacheFile, "utf8");
        return { text: cached, fromCache: true };
      } catch {
        throw err;
      }
    }
  }

  /**
   * Fetch the community index (pre-built _index.json) from GitHub.
   * Uses ETag caching so subsequent calls within 10 min are instant.
   */
  async function fetchCommunityIndex() {
    const cacheKey = "knowledge/_index.json";
    // Fast path: if we checked recently, use local cache.
    if (isCacheFresh(cacheKey)) {
      const cacheFile = path.join(
        GITHUB_CACHE_DIR,
        cacheKey.replace(/\//g, "__"),
      );
      try {
        return JSON.parse(await fs.readFile(cacheFile, "utf8"));
      } catch {
        /* fall through to network */
      }
    }
    const { text } = await fetchGitHubFile(cacheKey);
    return JSON.parse(text);
  }

  /**
   * Read a knowledge file content. Resolution order:
   * 1. Local workspace (.github/knowledge/)
   * 2. GitHub raw content (community knowledge, cached)
   */
  async function readKnowledgeFileContent(filename) {
    // Local first (fast — no network)
    for (const root of [KNOWLEDGE_ROOT, REPO_KNOWLEDGE_ROOT]) {
      try {
        return await fs.readFile(path.join(root, filename), "utf8");
      } catch {
        /* not found locally */
      }
    }
    // Fetch from GitHub community knowledge
    const { text } = await fetchGitHubFile(`knowledge/${filename}`);
    return text;
  }

  async function searchKnowledgeCache(args) {
    const query = String(args.query || "").trim();
    if (!query) {
      throw new Error("search_knowledge_cache requires a non-empty query.");
    }

    const maxResults = toPositiveInt(args.max_results, 5, 1, 20);
    const terms = tokenizeQuery(query);
    if (!terms.length) {
      throw new Error(
        "search_knowledge_cache query must include searchable terms.",
      );
    }

    // Collect files from both workspace and repo knowledge roots.
    // Deduplicate when both resolve to the same directory (e.g. running
    // inside the git-shell-helpers repo itself).
    const roots = [KNOWLEDGE_ROOT];
    const resolvedWorkspace = path.resolve(KNOWLEDGE_ROOT);
    const resolvedRepo = path.resolve(REPO_KNOWLEDGE_ROOT);
    if (resolvedRepo !== resolvedWorkspace) {
      roots.push(REPO_KNOWLEDGE_ROOT);
    }

    const seenPaths = new Set();
    const files = [];
    for (const root of roots) {
      try {
        const found = await collectMarkdownFiles(root);
        for (const f of found) {
          const real = path.resolve(f);
          if (!seenPaths.has(real)) {
            seenPaths.add(real);
            files.push({ filePath: f, root });
          }
        }
      } catch (error) {
        if (!error || error.code !== "ENOENT") throw error;
        // root doesn't exist — skip
      }
    }

    if (!files.length) {
      return {
        query,
        root: "knowledge",
        total_results: "0",
        results: [],
      };
    }

    const results = [];
    for (const { filePath: fp, root } of files) {
      const body = await fs.readFile(fp, "utf8");
      // Show workspace-relative path for workspace files, repo-relative for repo files
      const relativePath =
        root === KNOWLEDGE_ROOT
          ? path.relative(WORKSPACE_ROOT, fp)
          : path.relative(path.resolve(__dirname), fp);
      const title = getMarkdownTitle(body, path.basename(fp, path.extname(fp)));
      const score = scoreKnowledgeMatch(relativePath, title, body, terms);
      if (score <= 0) {
        continue;
      }

      results.push({
        path: relativePath,
        title,
        score,
        source: root === KNOWLEDGE_ROOT ? "workspace" : "repo",
        snippet: buildKnowledgeSnippet(body, terms),
      });
    }

    results.sort(
      (left, right) =>
        right.score - left.score || left.path.localeCompare(right.path),
    );

    return {
      query,
      root: "knowledge",
      total_results: String(results.length),
      results: results.slice(0, maxResults).map((result, index) => ({
        rank: index + 1,
        path: result.path,
        title: result.title,
        source: result.source,
        snippet: result.snippet,
      })),
    };
  }

  async function readKnowledgeNote(args) {
    const notePath = String(args.path || "").trim();
    if (!notePath) {
      throw new Error("read_knowledge_note requires a non-empty path.");
    }

    // Accept bare filenames — resolve to KNOWLEDGE_ROOT.
    let resolvedPath;
    if (!notePath.includes(path.sep) && !notePath.includes("/")) {
      resolvedPath = path.join(KNOWLEDGE_ROOT, notePath);
    } else {
      resolvedPath = path.resolve(WORKSPACE_ROOT, notePath);
    }
    const relativeToRoot = path.relative(KNOWLEDGE_ROOT, resolvedPath);
    const relativeToRepo = path.relative(REPO_KNOWLEDGE_ROOT, resolvedPath);

    const inWorkspace =
      !relativeToRoot.startsWith("..") && !path.isAbsolute(relativeToRoot);
    const inRepo =
      !relativeToRepo.startsWith("..") && !path.isAbsolute(relativeToRepo);

    if (!inWorkspace && !inRepo) {
      throw new Error(
        "read_knowledge_note only allows files under the knowledge directory.",
      );
    }

    // Determine the filename relative to the knowledge root
    const filename = inWorkspace ? relativeToRoot : relativeToRepo;

    // Resolution order: local workspace → repo knowledge root → GitHub community
    let text;
    let source;

    // 1. Local workspace
    try {
      text = await fs.readFile(path.join(KNOWLEDGE_ROOT, filename), "utf8");
      source = "workspace";
    } catch {
      /* not found locally */
    }

    // 2. Repo knowledge root (when running inside the repo)
    if (!text) {
      try {
        text = await fs.readFile(
          path.join(REPO_KNOWLEDGE_ROOT, filename),
          "utf8",
        );
        source = "repo";
      } catch {
        /* not found in repo */
      }
    }

    // 3. GitHub community knowledge (fetched with ETag cache)
    if (!text) {
      try {
        const { text: fetched } = await fetchGitHubFile(
          `knowledge/${filename}`,
        );
        text = fetched;
        source = "community";
      } catch {
        throw new Error(
          `Knowledge note not found locally or in community: ${filename}`,
        );
      }
    }

    const maxChars = args.max_chars
      ? toPositiveInt(args.max_chars, 0, 500, 100000)
      : 0;
    const trimmed = text.trim();

    return {
      path: `knowledge/${filename}`,
      source,
      title: getMarkdownTitle(
        trimmed,
        path.basename(filename, path.extname(filename)),
      ),
      text: maxChars > 0 ? summarizeText(trimmed, maxChars) : trimmed,
    };
  }

  function resolveKnowledgePath(notePath) {
    // Accept bare filenames (e.g. "networking-dns.md") — prepend KNOWLEDGE_ROOT.
    // Also accept full workspace-relative paths for backward compat.
    let resolvedPath;
    if (!notePath.includes(path.sep) && !notePath.includes("/")) {
      // Bare filename — place in KNOWLEDGE_ROOT.
      resolvedPath = path.join(KNOWLEDGE_ROOT, notePath);
    } else {
      resolvedPath = path.resolve(WORKSPACE_ROOT, notePath);
    }
    const relativeToRoot = path.relative(KNOWLEDGE_ROOT, resolvedPath);
    if (relativeToRoot.startsWith("..") || path.isAbsolute(relativeToRoot)) {
      throw new Error(
        `Knowledge note path must be under the knowledge directory (${path.relative(WORKSPACE_ROOT, KNOWLEDGE_ROOT)}/).`,
      );
    }
    if (!resolvedPath.endsWith(".md")) {
      throw new Error("Knowledge notes must be .md files.");
    }
    return resolvedPath;
  }

  async function writeKnowledgeNote(args) {
    const notePath = String(args.path || "").trim();
    if (!notePath) {
      throw new Error("write_knowledge_note requires a non-empty path.");
    }

    const content = String(args.content || "").trim();
    if (!content) {
      throw new Error("write_knowledge_note requires non-empty content.");
    }

    const resolvedPath = resolveKnowledgePath(notePath);

    let exists = false;
    try {
      await fs.access(resolvedPath);
      exists = true;
    } catch {
      // file doesn't exist — will create
    }

    if (exists && !args.overwrite) {
      throw new Error(
        `File already exists: ${path.relative(WORKSPACE_ROOT, resolvedPath)}. Set overwrite=true to replace it.`,
      );
    }

    await fs.mkdir(path.dirname(resolvedPath), { recursive: true });
    await fs.writeFile(resolvedPath, content, "utf8");

    return finalizeKnowledgeWrite(
      {
        action: exists ? "overwritten" : "created",
        path: path.relative(WORKSPACE_ROOT, resolvedPath),
      },
      resolvedPath,
      args,
    );
  }

  async function updateKnowledgeNote(args) {
    const notePath = String(args.path || "").trim();
    if (!notePath) {
      throw new Error("update_knowledge_note requires a non-empty path.");
    }

    const heading = String(args.heading || "").trim();
    if (!heading) {
      throw new Error(
        "update_knowledge_note requires a heading to locate the section.",
      );
    }

    const content = String(args.content || "").trim();
    if (!content) {
      throw new Error("update_knowledge_note requires non-empty content.");
    }

    const resolvedPath = resolveKnowledgePath(notePath);
    const text = await fs.readFile(resolvedPath, "utf8");

    // Match the heading line (any heading level)
    const escapedHeading = escapeRegExp(heading);
    const headingRegex = new RegExp(`^(#{1,6})\\s+${escapedHeading}\\s*$`, "m");
    const headingMatch = headingRegex.exec(text);
    if (!headingMatch) {
      throw new Error(
        `Heading "${heading}" not found in ${path.relative(WORKSPACE_ROOT, resolvedPath)}.`,
      );
    }

    const headingLevel = headingMatch[1].length;
    const sectionStart = headingMatch.index + headingMatch[0].length;

    // Find the next heading at the same or higher level
    const nextHeadingRegex = new RegExp(`^#{1,${headingLevel}}\\s+`, "m");
    const rest = text.slice(sectionStart);
    const nextMatch = nextHeadingRegex.exec(rest);
    const sectionEnd = nextMatch ? sectionStart + nextMatch.index : text.length;

    const updated =
      text.slice(0, sectionStart) +
      "\n" +
      content +
      "\n\n" +
      text.slice(sectionEnd);
    await fs.writeFile(resolvedPath, updated, "utf8");

    return finalizeKnowledgeWrite(
      {
        action: "updated",
        path: path.relative(WORKSPACE_ROOT, resolvedPath),
        heading,
      },
      resolvedPath,
      args,
    );
  }

  async function appendToKnowledgeNote(args) {
    const notePath = String(args.path || "").trim();
    if (!notePath) {
      throw new Error("append_to_knowledge_note requires a non-empty path.");
    }

    const content = String(args.content || "").trim();
    if (!content) {
      throw new Error("append_to_knowledge_note requires non-empty content.");
    }

    const resolvedPath = resolveKnowledgePath(notePath);
    const existing = await fs.readFile(resolvedPath, "utf8");
    const separator = existing.endsWith("\n") ? "\n" : "\n\n";
    await fs.writeFile(
      resolvedPath,
      existing + separator + content + "\n",
      "utf8",
    );

    return finalizeKnowledgeWrite(
      {
        action: "appended",
        path: path.relative(WORKSPACE_ROOT, resolvedPath),
      },
      resolvedPath,
      args,
    );
  }

  async function submitCommunityResearch(args) {
    const notePath = String(args.path || "").trim();
    if (!notePath) {
      throw new Error("submit_community_research requires a non-empty path.");
    }

    const resolvedPath = resolveKnowledgePath(notePath);

    // Verify the file exists
    try {
      await fs.access(resolvedPath);
    } catch {
      throw new Error(
        `Knowledge note not found: ${path.relative(WORKSPACE_ROOT, resolvedPath)}`,
      );
    }

    return runCommunityResearchSubmit(resolvedPath);
  }

  function formatKnowledgeSearchResult(result) {
    const lines = [
      `Query: ${result.query}`,
      `Knowledge root: ${result.root}`,
      `Total results: ${result.total_results}`,
      "",
      "Results:",
    ];

    for (const item of result.results) {
      lines.push(`${item.rank}. ${item.title}`);
      lines.push(`   Path: ${item.path}`);
      if (item.source) {
        lines.push(`   Source: ${item.source}`);
      }
      if (item.snippet) {
        lines.push(`   Snippet: ${item.snippet}`);
      }
    }

    if (!result.results.length) {
      lines.push("No cached knowledge notes matched.");
    }

    return lines.join("\n");
  }

  function formatKnowledgeNoteResult(result) {
    return [
      `Title: ${result.title}`,
      `Path: ${result.path}`,
      "",
      result.text || "No text available.",
    ].join("\n");
  }

  function formatKnowledgeWriteResult(result) {
    const lines = [`Action: ${result.action}`, `Path: ${result.path}`];
    if (result.heading) {
      lines.push(`Heading: ${result.heading}`);
    }
    if (result.index) {
      if (result.index.status === "rebuilt") {
        lines.push(
          `Index: rebuilt (${result.index.file_count} files, ${result.index.term_count} terms)`,
        );
        if (result.index.path) {
          lines.push(`Index path: ${result.index.path}`);
        }
      } else {
        lines.push("Index: failed");
        if (result.index.message) {
          lines.push(`Index detail: ${result.index.message}`);
        }
      }
    }
    if (result.publish) {
      lines.push(`Publish: ${result.publish.status}`);
      if (result.publish.message) {
        lines.push(`Publish detail: ${result.publish.message}`);
      }
      if (result.publish.output) {
        lines.push(`Publish detail: ${result.publish.output}`);
      }
    }
    if (result.output) {
      lines.push(`Output: ${result.output}`);
    }
    return lines.join("\n");
  }

  return {
    loadCacheMeta,
    saveCacheMeta,
    isCacheFresh,
    fetchGitHubFile,
    fetchCommunityIndex,
    readKnowledgeFileContent,
    searchKnowledgeCache,
    readKnowledgeNote,
    resolveKnowledgePath,
    writeKnowledgeNote,
    updateKnowledgeNote,
    appendToKnowledgeNote,
    submitCommunityResearch,
    formatKnowledgeSearchResult,
    formatKnowledgeNoteResult,
    formatKnowledgeWriteResult,
  };
};
