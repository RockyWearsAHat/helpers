#!/usr/bin/env bash
set -euo pipefail
SRC="/Users/alexwaldmann/bin/git-research-mcp"
OUT="/Users/alexwaldmann/bin-refactor-modular/git-research-mcp.new"

{
  # Lines 1-122: shebang, imports, env loading, all constants
  sed -n '1,122p' "$SRC"

  # Retry constants (lines 232-234 in original)
  printf '\n'
  sed -n '232,234p' "$SRC"

  # FETCH_TIMEOUT_MS
  printf 'const FETCH_TIMEOUT_MS = 120_000;\n'

  # Google search constants (lines ~1517-1540 in original)
  printf '\n'
  sed -n '1517,1544p' "$SRC"

  # Module loading block
  cat << 'MODULES'

// ── Module loading ──────────────────────────────────────────────────────────
const createUtils = require("./lib/mcp-utils");
const createKnowledgeIndex = require("./lib/mcp-knowledge-index");
const createKnowledgeRW = require("./lib/mcp-knowledge-rw");
const createGoogleHeadless = require("./lib/mcp-google-headless");
const createWebSearch = require("./lib/mcp-web-search");

// Shared utilities
const utils = createUtils({
  DEFAULT_USER_AGENT,
  RETRY_MAX_ATTEMPTS,
  RETRY_BASE_DELAY_MS,
  RETRY_MAX_DELAY_MS,
  FETCH_TIMEOUT_MS,
});

// Google headless Chrome
const google = createGoogleHeadless({
  ...utils,
  GOOGLE_MIN_DELAY_MS,
  GOOGLE_EMPTY_RETRY_DELAY_MS,
  GOOGLE_EMPTY_RETRY_MAX,
  GOOGLE_429_BASE_DELAY_MS,
  GOOGLE_RESULTS_PER_PAGE,
  GOOGLE_DEFAULT_PAGE_COUNT,
  GOOGLE_CAPTCHA_POLL_DELAY_SECONDS,
  GOOGLE_CAPTCHA_POLL_ATTEMPTS,
  GOOGLE_CONSENT_COOKIES,
  GOOGLE_DEFAULT_ACCEPT_LANGUAGE,
  HEADLESS_CHROME_EXECUTABLE,
  CHROME_EXECUTABLE_PATH,
  GOOGLE_BROWSER_PROFILE_DIR,
  DEFAULT_USER_AGENT,
  RETRY_MAX_ATTEMPTS,
  RETRY_BASE_DELAY_MS,
  RETRY_MAX_DELAY_MS,
});

// Knowledge R/W (needs forward reference resolved below)
let knowledgeRW;

// Knowledge index
const knowledgeIndex = createKnowledgeIndex({
  KNOWLEDGE_ROOT,
  REPO_KNOWLEDGE_ROOT,
  LOCAL_INDEX_PATH,
  WORKSPACE_ROOT,
  ...utils,
  get fetchCommunityIndex() { return knowledgeRW.fetchCommunityIndex; },
  get readKnowledgeFileContent() { return knowledgeRW.readKnowledgeFileContent; },
  get searchKnowledgeCache() { return knowledgeRW.searchKnowledgeCache; },
});

// Debounced index rebuild — coalesces rapid-fire writes into a single rebuild.
let _indexRebuildTimer = null;
let _indexRebuildRunning = false;
function scheduleIndexRebuild() {
  if (_indexRebuildTimer) clearTimeout(_indexRebuildTimer);
  _indexRebuildTimer = setTimeout(async () => {
    _indexRebuildTimer = null;
    if (_indexRebuildRunning) return;
    _indexRebuildRunning = true;
    try {
      await knowledgeIndex.buildKnowledgeIndex({});
    } catch (err) {
      process.stderr.write(
        `[git-research-mcp] Index rebuild failed: ${err.message}\n`,
      );
    } finally {
      _indexRebuildRunning = false;
    }
  }, 2000);
}

// Knowledge R/W (now resolved)
knowledgeRW = createKnowledgeRW({
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
  ...utils,
  buildKnowledgeIndex: (...a) => knowledgeIndex.buildKnowledgeIndex(...a),
  buildKnowledgeSnippet: knowledgeIndex.buildKnowledgeSnippet,
  scoreKnowledgeMatch: knowledgeIndex.scoreKnowledgeMatch,
  collectMarkdownFiles: knowledgeIndex.collectMarkdownFiles,
  scheduleIndexRebuild,
});

// Web search
const webSearch = createWebSearch({
  ...utils,
  ...google,
  WORKSPACE_ROOT,
  DEFAULT_USER_AGENT,
  GOOGLE_RESULTS_PER_PAGE,
  GOOGLE_DEFAULT_PAGE_COUNT,
  GOOGLE_DEFAULT_ACCEPT_LANGUAGE,
  GOOGLE_EMPTY_RETRY_MAX,
  GOOGLE_EMPTY_RETRY_DELAY_MS,
});

// Destructure for convenience in handler
const { searchKnowledgeCache, readKnowledgeNote, writeKnowledgeNote,
        updateKnowledgeNote, appendToKnowledgeNote, submitCommunityResearch,
        formatKnowledgeSearchResult, formatKnowledgeNoteResult, formatKnowledgeWriteResult } = knowledgeRW;
const { buildKnowledgeIndex, searchKnowledgeIndex,
        formatKnowledgeIndexSearchResult, formatBuildIndexResult } = knowledgeIndex;
const { searchWeb, fetchPages, formatSearchResult, formatFetchPagesResult } = webSearch;

MODULES

  # send / sendError (lines 257-268)
  sed -n '257,268p' "$SRC"
  printf '\n'

  # RESEARCH_TOOLS + handler + main loop (lines 2672-3029)
  sed -n '2672,3029p' "$SRC"

} > "$OUT"

echo "Done. New file: $OUT ($(wc -l < "$OUT") lines)"
