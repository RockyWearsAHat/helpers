#!/usr/bin/env node

"use strict";

const fs = require("fs/promises");
const os = require("os");
const path = require("path");
const Module = require("module");

let passed = 0;
let failed = 0;

function assert(condition, label) {
  if (condition) {
    passed++;
    process.stderr.write(`  PASS: ${label}\n`);
  } else {
    failed++;
    process.stderr.write(`  FAIL: ${label}\n`);
  }
}

function makeFakePuppeteer(options = {}) {
  const state = {
    launches: 0,
    interactiveLaunches: 0,
    headlessLaunches: 0,
    browserCloses: 0,
  };

  const cookies = [
    {
      name: "NID",
      value: "captcha-cleared",
      domain: ".google.com",
      path: "/",
      secure: true,
      httpOnly: false,
    },
  ];

  function createPage(browser) {
    const page = {
      _url: "about:blank",
      browser: () => browser,
      goto: async (url) => {
        page._url = url;
        return { status: () => 200 };
      },
      waitForSelector: async () => {},
      waitForNavigation: async () => {},
      url: () => page._url,
      setViewport: async () => {},
      setExtraHTTPHeaders: async () => {},
      evaluateOnNewDocument: async () => {},
      target: () => ({
        createCDPSession: async () => ({
          send: async (method) => {
            if (method === "Browser.getVersion") {
              return {
                userAgent: "Mozilla/5.0 Chrome/123.0.0.0 Safari/537.36",
                product: "Chrome/123.0.0.0",
              };
            }
            if (method === "Network.getAllCookies") {
              return { cookies };
            }
            return {};
          },
        }),
      }),
      evaluate: async (fn) => {
        const source = String(fn);
        if (source.includes("navigator.userAgent")) {
          return {
            userAgent: "Mozilla/5.0 Chrome/123.0.0.0 Safari/537.36",
            language: "en-US",
            languages: ["en-US", "en"],
            platform: "macOS",
            mobile: false,
            brands: [{ brand: "Google Chrome", version: "123" }],
            fullVersionList: [
              { brand: "Google Chrome", version: "123.0.0.0" },
            ],
            uaFullVersion: "123.0.0.0",
            architecture: "x86",
            bitness: "64",
            platformVersion: "16.0.0",
            model: "",
            wow64: false,
          };
        }

        if (options.noResults) {
          return {
            challenge: false,
            noResults: true,
            title: "No results",
            href: page._url,
            bodyText:
              "Your search - impossible query - did not match any documents. Suggestions: try different keywords.",
            results: [],
          };
        }

        return {
          challenge: false,
          noResults: false,
          title: "Google Search",
          href: page._url,
          bodyText: "",
          results: [
            {
              rawHref: "https://example.com/result",
              title: "Example Result",
              snippet: "Example snippet",
              text: "Example Result",
            },
          ],
        };
      },
      close: async () => {},
    };
    return page;
  }

  function createBrowser() {
    const browser = {
      connected: true,
      on: () => {},
      process: () => ({ pid: 43210 }),
      newPage: async () => createPage(browser),
      close: async () => {
        browser.connected = false;
        state.browserCloses++;
      },
    };
    return browser;
  }

  return {
    state,
    puppeteer: {
      executablePath: () => process.execPath,
      launch: async (launchOptions = {}) => {
        state.launches++;
        if (launchOptions.headless === false) {
          state.interactiveLaunches++;
        } else {
          state.headlessLaunches++;
        }
        return createBrowser();
      },
    },
  };
}

function buildDeps(profileDir) {
  return {
    sleep: async () => {},
    execFileAsync: async () => {},
    decodeHtmlEntities: (value) => value,
    fetchWithRetry: async () => "",
    GOOGLE_MIN_DELAY_MS: 0,
    GOOGLE_EMPTY_RETRY_DELAY_MS: 1,
    GOOGLE_EMPTY_RETRY_MAX: 1,
    GOOGLE_429_BASE_DELAY_MS: 1,
    GOOGLE_RESULTS_PER_PAGE: 10,
    GOOGLE_DEFAULT_PAGE_COUNT: 2,
    GOOGLE_CAPTCHA_POLL_DELAY_SECONDS: 0,
    GOOGLE_CAPTCHA_POLL_ATTEMPTS: 3,
    GOOGLE_CONSENT_COOKIES: [],
    GOOGLE_DEFAULT_ACCEPT_LANGUAGE: "en-US,en;q=0.9",
    HEADLESS_CHROME_EXECUTABLE: process.execPath,
    CHROME_EXECUTABLE_PATH: process.execPath,
    GOOGLE_BROWSER_PROFILE_DIR: profileDir,
    DEFAULT_USER_AGENT: "Mozilla/5.0 Chrome/123.0.0.0 Safari/537.36",
    RETRY_MAX_ATTEMPTS: 1,
    RETRY_BASE_DELAY_MS: 1,
    RETRY_MAX_DELAY_MS: 1,
  };
}

async function withMockedPuppeteer(fakePuppeteer, callback) {
  const originalLoad = Module._load;
  Module._load = function patchedLoad(request, parent, isMain) {
    if (request === "puppeteer") {
      return fakePuppeteer;
    }
    return originalLoad.call(this, request, parent, isMain);
  };

  const modulePath = require.resolve("../lib/mcp-google-headless");
  delete require.cache[modulePath];

  try {
    const createGoogleHeadless = require("../lib/mcp-google-headless");
    await callback(createGoogleHeadless);
  } finally {
    Module._load = originalLoad;
    delete require.cache[modulePath];
  }
}

async function main() {
  // 1. Concurrent interactive challenge resolutions share a single browser launch.
  {
    const tempDir = await fs.mkdtemp(
      path.join(os.tmpdir(), "helpers-google-share-"),
    );
    const fake = makeFakePuppeteer();
    await withMockedPuppeteer(fake.puppeteer, async (createGoogleHeadless) => {
      const google = createGoogleHeadless(buildDeps(tempDir));
      const [first, second, third] = await Promise.all([
        google.resolveGoogleChallengeViaLiveChrome(
          "https://www.google.com/search?q=alpha",
        ),
        google.resolveGoogleChallengeViaLiveChrome(
          "https://www.google.com/search?q=beta",
        ),
        google.resolveGoogleChallengeViaLiveChrome(
          "https://www.google.com/search?q=gamma",
        ),
      ]);
      assert(
        fake.state.interactiveLaunches === 1,
        "Concurrent CAPTCHA resolution reuses one interactive browser",
      );
      assert(
        !first.challenge && !second.challenge && !third.challenge,
        "Shared CAPTCHA resolution succeeds for all waiting callers",
      );
      // The query that actually ran (alpha) renders results, and we no longer
      // discard them — that discard was what forced a headless re-fetch that hit
      // the CAPTCHA again. Concurrent waiters (beta/gamma) receive the same
      // resolution tagged with alpha's resolvedUrl, so they can detect the
      // mismatch and re-run their own query through the now-verified browser
      // instead of mistaking alpha's results for their own.
      const alphaUrl = "https://www.google.com/search?q=alpha";
      assert(
        first.results.length > 0 &&
          second.results.length > 0 &&
          third.results.length > 0,
        "Shared CAPTCHA resolution returns the solved query's results (no longer discarded)",
      );
      assert(
        first.resolvedUrl === alphaUrl &&
          second.resolvedUrl === alphaUrl &&
          third.resolvedUrl === alphaUrl,
        "Shared CAPTCHA resolution tags results with resolvedUrl so waiters detect cross-query reuse",
      );
      const cookiePath = path.join(tempDir, "_captcha_cookies.json");
      const cookieText = await fs.readFile(cookiePath, "utf8");
      assert(
        cookieText.includes("captcha-cleared"),
        "Interactive CAPTCHA cookies are persisted for reuse",
      );
    });
    await fs.rm(tempDir, { recursive: true, force: true });
  }

  // 1b. Once a CAPTCHA is solved, the verified browser is reused for later
  //     searches — the user is prompted at most once, not per query.
  {
    const tempDir = await fs.mkdtemp(
      path.join(os.tmpdir(), "helpers-google-reuse-"),
    );
    const fake = makeFakePuppeteer();
    await withMockedPuppeteer(fake.puppeteer, async (createGoogleHeadless) => {
      const google = createGoogleHeadless(buildDeps(tempDir));

      // No verified session yet → reuse path is a no-op.
      const beforeSolve = await google.searchViaVerifiedChrome(
        "https://www.google.com/search?q=early",
      );
      assert(
        beforeSolve === null,
        "searchViaVerifiedChrome is a no-op before any CAPTCHA is solved",
      );

      // Solve once interactively (one visible browser launch).
      await google.resolveGoogleChallengeViaLiveChrome(
        "https://www.google.com/search?q=first",
      );
      const launchesAfterSolve = fake.state.interactiveLaunches;

      // Subsequent searches reuse the verified browser — no new launch, no prompt.
      const reuseUrl = "https://www.google.com/search?q=second";
      const reused = await google.searchViaVerifiedChrome(reuseUrl);
      assert(
        reused && !reused.challenge && reused.results.length > 0,
        "Verified browser reuse returns results without re-prompting",
      );
      assert(
        reused.resolvedUrl === reuseUrl,
        "Verified browser reuse runs the caller's own query",
      );
      assert(
        fake.state.interactiveLaunches === launchesAfterSolve,
        "Verified browser reuse launches no additional interactive browser",
      );
    });
    await fs.rm(tempDir, { recursive: true, force: true });
  }

  // 2. Genuine Google no-results pages are not classified as CAPTCHA challenges.
  {
    const tempDir = await fs.mkdtemp(
      path.join(os.tmpdir(), "helpers-google-no-results-"),
    );
    const fake = makeFakePuppeteer({ noResults: true });
    await withMockedPuppeteer(fake.puppeteer, async (createGoogleHeadless) => {
      const google = createGoogleHeadless(buildDeps(tempDir));
      const outcome = await google.searchGoogleHeadless(
        "https://www.google.com/search?q=impossible-query",
      );
      assert(
        outcome.noResults === true,
        "Headless Google search flags confirmed no-results pages",
      );
      assert(
        outcome.challenge === false,
        "Confirmed no-results pages do not trigger CAPTCHA handling",
      );
    });
    await fs.rm(tempDir, { recursive: true, force: true });
  }

  process.stderr.write(
    `\ngoogle challenge sharing: ${passed} passed, ${failed} failed\n`,
  );
  if (failed > 0) {
    process.stdout.write(`TEST_SUMMARY: fail ${failed}/${passed + failed}\n`);
    process.exit(1);
  }
  process.stdout.write(`TEST_SUMMARY: pass ${passed}/${passed + failed}\n`);
}

main().catch((err) => {
  process.stderr.write(`Unexpected error: ${err.message}\n${err.stack}\n`);
  process.stdout.write("TEST_SUMMARY: fail 1/1\n");
  process.exit(1);
});