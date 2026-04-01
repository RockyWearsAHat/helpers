/**
 * Research module factory — instantiates all knowledge, web search,
 * and session memory modules with shared configuration. Used by the
 * combined MCP server (git-shell-helpers-mcp) and the standalone
 * research server (git-research-mcp).
 *
 * @param {object} [options]
 * @param {string} [options.workspaceRoot] — override auto-detected workspace root
 * @returns {object} all instantiated functions needed by the tool handler
 */

const fsSync = require("fs");
const path = require("path");

module.exports = function createResearch(options = {}) {
  // ─── .env loading ─────────────────────────────────────────────────────
  const CONFIG_ENV_PATH = path.join(
    process.env.HOME || process.env.USERPROFILE || "",
    ".config",
    "git-research-mcp",
    ".env",
  );
  const LOCAL_ENV_PATH = path.join(__dirname, "..", ".env");
  const envPath = fsSync.existsSync(CONFIG_ENV_PATH)
    ? CONFIG_ENV_PATH
    : fsSync.existsSync(LOCAL_ENV_PATH)
      ? LOCAL_ENV_PATH
      : null;

  if (envPath) {
    for (const line of fsSync.readFileSync(envPath, "utf8").split("\n")) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const eqIdx = trimmed.indexOf("=");
      if (eqIdx < 1) continue;
      const key = trimmed.slice(0, eqIdx).trim();
      const val = trimmed
        .slice(eqIdx + 1)
        .trim()
        .replace(/^["']|["']$/g, "");
      if (!process.env[key]) process.env[key] = val;
    }
  }

  // ─── Constants ────────────────────────────────────────────────────────
  const REPO_ROOT = path.join(__dirname, "..");

  const DEFAULT_USER_AGENT =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";
  const GOOGLE_DEFAULT_ACCEPT_LANGUAGE = "en-US,en;q=0.9";
  const CHROME_EXECUTABLE_PATH =
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
  const HEADLESS_CHROME_EXECUTABLE =
    process.env.GSH_HEADLESS_CHROME_EXECUTABLE || "";
  const GOOGLE_CONSENT_COOKIES = [
    {
      name: "CONSENT",
      value: "PENDING+987",
      domain: ".google.com",
      path: "/",
      secure: true,
    },
    {
      name: "SOCS",
      value:
        "CAISNQgDEitib3FfaWRlbnRpdHlmcm9udGVuZHVpc2VydmVyXzIwMjUwMzI0LjA1X3AwGgJlbiADGgYIgN_JvAY",
      domain: ".google.com",
      path: "/",
      secure: true,
    },
  ];

  // ─── Workspace detection ──────────────────────────────────────────────
  const WORKSPACE_ROOT =
    options.workspaceRoot ||
    (() => {
      if (process.env.GSH_WORKSPACE_ROOTS) {
        try {
          const roots = JSON.parse(process.env.GSH_WORKSPACE_ROOTS);
          if (Array.isArray(roots) && roots.length > 0) return roots[0];
        } catch {
          /* malformed — fall through */
        }
      }
      const cwd = process.cwd();
      if (
        fsSync.existsSync(path.join(cwd, ".git")) ||
        fsSync.existsSync(path.join(cwd, ".github"))
      )
        return cwd;
      return REPO_ROOT;
    })();

  const REPO_KNOWLEDGE_ROOT = path.join(REPO_ROOT, "knowledge");

  const KNOWLEDGE_ROOT = (() => {
    const atRoot = path.join(WORKSPACE_ROOT, "knowledge");
    const atGithub = path.join(WORKSPACE_ROOT, ".github", "knowledge");
    try {
      const entries = fsSync.readdirSync(atRoot);
      if (entries.some((f) => f.endsWith(".md"))) return atRoot;
    } catch {
      /* dir may not exist */
    }
    return atGithub;
  })();

  const LOCAL_INDEX_PATH = path.join(KNOWLEDGE_ROOT, "_index.json");

  // ─── GitHub-backed community knowledge ────────────────────────────────
  const GITHUB_REPO = "RockyWearsAHat/github-shell-helpers";
  const GITHUB_BRANCH = "dev";
  const GITHUB_RAW_BASE = `https://raw.githubusercontent.com/${GITHUB_REPO}/${GITHUB_BRANCH}`;
  const GITHUB_API_BASE = `https://api.github.com/repos/${GITHUB_REPO}`;
  const GITHUB_CACHE_DIR = path.join(
    process.env.HOME || process.env.USERPROFILE || "/tmp",
    ".cache",
    "gsh",
  );
  const GOOGLE_BROWSER_PROFILE_DIR = path.join(
    GITHUB_CACHE_DIR,
    "google-browser-profile",
  );
  const INDEX_MAX_AGE_MS = 10 * 60 * 1000;

  try {
    fsSync.mkdirSync(GITHUB_CACHE_DIR, { recursive: true });
  } catch {
    /* ignore */
  }

  // ─── Retry / fetch constants ──────────────────────────────────────────
  const RETRY_MAX_ATTEMPTS = 4;
  const RETRY_BASE_DELAY_MS = 2000;
  const RETRY_MAX_DELAY_MS = 30000;
  const FETCH_TIMEOUT_MS = 120_000;

  // ─── Google search constants ──────────────────────────────────────────
  const GOOGLE_MIN_DELAY_MS = parseInt(
    process.env.GOOGLE_MIN_DELAY_MS || "3000",
    10,
  );
  const GOOGLE_EMPTY_RETRY_DELAY_MS = 10000;
  const GOOGLE_EMPTY_RETRY_MAX = 4;
  const GOOGLE_429_BASE_DELAY_MS = 30000;
  const GOOGLE_RESULTS_PER_PAGE = 10;
  const GOOGLE_DEFAULT_PAGE_COUNT = parseInt(
    process.env.GOOGLE_DEFAULT_PAGE_COUNT || "2",
    10,
  );
  const GOOGLE_CAPTCHA_POLL_DELAY_SECONDS = 1;
  const GOOGLE_CAPTCHA_POLL_ATTEMPTS = parseInt(
    process.env.GOOGLE_CAPTCHA_POLL_ATTEMPTS || "300",
    10,
  );

  // Cache metadata path
  const CACHE_META_PATH = path.join(GITHUB_CACHE_DIR, "_cache_meta.json");

  // ─── Module instantiation ─────────────────────────────────────────────
  const createUtils = require("./mcp-utils");
  const createKnowledgeIndex = require("./mcp-knowledge-index");
  const createKnowledgeRW = require("./mcp-knowledge-rw");
  const createGoogleHeadless = require("./mcp-google-headless");
  const createWebSearch = require("./mcp-web-search");
  const createSessionMemory = require("./mcp-session-memory");

  const utils = createUtils({
    DEFAULT_USER_AGENT,
    RETRY_MAX_ATTEMPTS,
    RETRY_BASE_DELAY_MS,
    RETRY_MAX_DELAY_MS,
    FETCH_TIMEOUT_MS,
  });

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

  const knowledgeIndex = createKnowledgeIndex({
    KNOWLEDGE_ROOT,
    REPO_KNOWLEDGE_ROOT,
    LOCAL_INDEX_PATH,
    WORKSPACE_ROOT,
    ...utils,
    fetchCommunityIndex: (...args) => knowledgeRW.fetchCommunityIndex(...args),
    readKnowledgeFileContent: (...args) =>
      knowledgeRW.readKnowledgeFileContent(...args),
    searchKnowledgeCache: (...args) =>
      knowledgeRW.searchKnowledgeCache(...args),
  });

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
  });

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

  const sessionMemory = createSessionMemory({
    WORKSPACE_ROOT,
    ...utils,
  });

  // ─── Destructure and return all functions ─────────────────────────────
  const {
    searchKnowledgeCache,
    readKnowledgeNote,
    writeKnowledgeNote,
    updateKnowledgeNote,
    appendToKnowledgeNote,
    submitCommunityResearch,
    formatKnowledgeSearchResult,
    formatKnowledgeNoteResult,
    formatKnowledgeWriteResult,
  } = knowledgeRW;

  const {
    buildKnowledgeIndex,
    searchKnowledgeIndex,
    formatKnowledgeIndexSearchResult,
    formatBuildIndexResult,
  } = knowledgeIndex;

  const { searchWeb, fetchPages, formatSearchResult, formatFetchPagesResult } =
    webSearch;

  const {
    logSessionEvent,
    searchSessionLog,
    getSessionSummary,
    buildSessionIndex,
    formatLogResult: formatSessionLogResult,
    formatSearchResults: formatSessionSearchResults,
    formatSummaryResult: formatSessionSummaryResult,
  } = sessionMemory;

  return {
    // Knowledge R/W
    searchKnowledgeCache,
    readKnowledgeNote,
    writeKnowledgeNote,
    updateKnowledgeNote,
    appendToKnowledgeNote,
    submitCommunityResearch,
    formatKnowledgeSearchResult,
    formatKnowledgeNoteResult,
    formatKnowledgeWriteResult,
    // Knowledge Index
    buildKnowledgeIndex,
    searchKnowledgeIndex,
    formatKnowledgeIndexSearchResult,
    formatBuildIndexResult,
    // Web Search
    searchWeb,
    fetchPages,
    formatSearchResult,
    formatFetchPagesResult,
    // Session Memory
    logSessionEvent,
    searchSessionLog,
    getSessionSummary,
    buildSessionIndex,
    formatSessionLogResult,
    formatSessionSearchResults,
    formatSessionSummaryResult,
  };
};
