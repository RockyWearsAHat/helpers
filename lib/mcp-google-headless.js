"use strict";
// lib/mcp-google-headless.js — Google search via headless Chrome/Puppeteer
const os = require("os");
const path = require("path");
const fs = require("fs/promises");
const fsSync = require("fs");

module.exports = function createGoogleHeadless(deps) {
  const {
    sleep,
    execFileAsync,
    decodeHtmlEntities,
    fetchWithRetry,
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
  } = deps;

  // Module-level state
  let _lastGoogleRequestMs = 0;
  let _puppeteer = null;
  let _browser = null;
  let _browserLaunchPromise = null;
  let _liveChromeSearchQueue = Promise.resolve();
  const _googleNavigationProfiles = new WeakMap();
  let _browserUserDataDir = null;

  /**
   * Enforce a minimum gap between Google requests so we don't burst.
   */
  async function googleRateLimit() {
    const now = Date.now();
    const elapsed = now - _lastGoogleRequestMs;
    if (elapsed < GOOGLE_MIN_DELAY_MS) {
      await sleep(GOOGLE_MIN_DELAY_MS - elapsed);
    }
    _lastGoogleRequestMs = Date.now();
  }

  function canUseLiveChromeFallback() {
    try {
      getPuppeteer();
      return true;
    } catch {
      return false;
    }
  }

  async function getBrowserUserDataDir() {
    if (_browserUserDataDir) return _browserUserDataDir;

    // Use a dedicated headless subdirectory so the headless browser never
    // collides with the user's regular Chrome or the interactive CAPTCHA
    // browser, both of which may hold a SingletonLock on the base profile.
    const headlessDir = `${GOOGLE_BROWSER_PROFILE_DIR}-headless`;
    await fs.mkdir(headlessDir, { recursive: true });

    // Clean up stale profile locks from previous MCP server sessions.
    // If the server was killed, the SingletonLock symlink persists and
    // prevents new headless launches from using the profile.
    try {
      const lockPath = path.join(headlessDir, "SingletonLock");
      const linkTarget = await fs.readlink(lockPath).catch(() => "");
      if (linkTarget) {
        const pidMatch = linkTarget.match(/-(\d+)$/);
        if (pidMatch) {
          const stalePid = parseInt(pidMatch[1], 10);
          try {
            process.kill(stalePid, 0);
          } catch {
            // Process doesn't exist — lock is stale, remove it.
            await fs.unlink(lockPath).catch(() => {});
            process.stderr.write(
              `[git-research-mcp] Removed stale Chrome profile lock (PID ${stalePid} no longer running)\n`,
            );
          }
        }
      }
    } catch {
      // Non-critical — proceed even if lock cleanup fails.
    }

    // Copy saved CAPTCHA cookies into the headless profile so it benefits
    // from interactive CAPTCHA solves done against the base profile.
    try {
      const srcCookies = path.join(
        GOOGLE_BROWSER_PROFILE_DIR,
        "_captcha_cookies.json",
      );
      const dstCookies = path.join(headlessDir, "_captcha_cookies.json");
      await fs.copyFile(srcCookies, dstCookies);
    } catch {
      // No cookies to copy — normal on first run.
    }

    _browserUserDataDir = headlessDir;
    return _browserUserDataDir;
  }

  function resolvePuppeteerBrowserExecutable(puppeteer) {
    if (fsSync.existsSync(CHROME_EXECUTABLE_PATH)) {
      return CHROME_EXECUTABLE_PATH;
    }
    try {
      const bundledExecutable = puppeteer.executablePath();
      if (bundledExecutable && fsSync.existsSync(bundledExecutable)) {
        return bundledExecutable;
      }
    } catch {
      // Fall back to configured executables below.
    }
    if (
      HEADLESS_CHROME_EXECUTABLE &&
      fsSync.existsSync(HEADLESS_CHROME_EXECUTABLE)
    ) {
      return HEADLESS_CHROME_EXECUTABLE;
    }
    if (fsSync.existsSync(CHROME_EXECUTABLE_PATH)) {
      return CHROME_EXECUTABLE_PATH;
    }
    return undefined;
  }

  function resolveInteractiveBrowserExecutable(puppeteer) {
    if (fsSync.existsSync(CHROME_EXECUTABLE_PATH)) {
      return CHROME_EXECUTABLE_PATH;
    }
    return resolvePuppeteerBrowserExecutable(puppeteer);
  }

  async function runInteractiveGoogleBrowser(task) {
    const puppeteer = getPuppeteer();
    let shouldCloseBrowser = true;
    const executablePath = resolveInteractiveBrowserExecutable(puppeteer);

    // Use a persistent directory for the interactive CAPTCHA browser.
    // A temp directory triggers double CAPTCHAs because Google sees a fresh
    // profile with zero history as maximally suspicious. With a persistent
    // profile, cookies and browsing state accumulate, reducing re-challenges.
    const interactiveDataDir = `${GOOGLE_BROWSER_PROFILE_DIR}-interactive`;
    await fs.mkdir(interactiveDataDir, { recursive: true });

    // Clean up stale profile locks (same logic as headless profile).
    try {
      const lockPath = path.join(interactiveDataDir, "SingletonLock");
      const linkTarget = await fs.readlink(lockPath).catch(() => "");
      if (linkTarget) {
        const pidMatch = linkTarget.match(/-(\d+)$/);
        if (pidMatch) {
          const stalePid = parseInt(pidMatch[1], 10);
          try {
            process.kill(stalePid, 0);
          } catch {
            await fs.unlink(lockPath).catch(() => {});
          }
        }
      }
    } catch {
      // Non-critical — proceed even if lock cleanup fails.
    }

    process.stderr.write(
      `[git-research-mcp] Launching interactive Chrome with ${executablePath || "default browser"} (profile: ${interactiveDataDir})\n`,
    );
    const browser = await puppeteer.launch({
      executablePath,
      userDataDir: interactiveDataDir,
      headless: false,
      ignoreDefaultArgs: ["--enable-automation"],
      args: [
        "--disable-dev-shm-usage",
        "--disable-blink-features=AutomationControlled",
        "--window-size=1440,900",
        "--no-first-run",
        "--no-default-browser-check",
      ],
      defaultViewport: null,
    });

    try {
      process.stderr.write(
        "[git-research-mcp] Interactive Chrome launch complete\n",
      );
      const page = await browser.newPage();
      process.stderr.write(
        "[git-research-mcp] Interactive Chrome fresh page created\n",
      );

      // Bring the Chrome window to the foreground on macOS.
      // Puppeteer-launched windows from background processes (like MCP servers)
      // often open behind the IDE and are invisible to the user.
      if (process.platform === "darwin") {
        const pid = browser.process()?.pid;
        if (pid) {
          try {
            await execFileAsync("osascript", [
              "-e",
              `tell application "System Events" to set frontmost of first process whose unix id is ${pid} to true`,
            ]);
          } catch {
            try {
              await execFileAsync("osascript", [
                "-e",
                'tell application "Google Chrome" to activate',
              ]);
            } catch {
              process.stderr.write(
                "[git-research-mcp] Could not bring Chrome to foreground — look for its window behind VS Code\n",
              );
            }
          }
        }
      }

      await applyGoogleNavigationProfile(page);
      process.stderr.write(
        `[git-research-mcp] Interactive Chrome launched with ${executablePath || "default browser"} on a fresh page\n`,
      );
      const result = await task(page, browser);
      if (result && result.keepBrowserOpen) {
        shouldCloseBrowser = false;
        return result.output;
      }
      return result;
    } finally {
      if (shouldCloseBrowser) {
        try {
          await browser.close();
        } catch {
          // Ignore cleanup errors for the interactive challenge browser.
        }
      }
    }
  }

  function normalizeChromeUserAgent(userAgent) {
    const trimmed = String(userAgent || "").trim();
    if (!trimmed) return DEFAULT_USER_AGENT;
    return trimmed.replace(/HeadlessChrome\//g, "Chrome/");
  }

  function extractChromeVersion(...values) {
    for (const value of values) {
      const match = String(value || "").match(
        /(?:Chrome|HeadlessChrome)\/([\d.]+)/,
      );
      if (match) return match[1];
    }
    return "";
  }

  function buildGoogleAcceptLanguage(languages, language) {
    const ordered = [];
    for (const value of [
      language,
      ...(Array.isArray(languages) ? languages : []),
    ]) {
      const normalized = String(value || "").trim();
      if (!normalized || ordered.includes(normalized)) continue;
      ordered.push(normalized);
    }

    return ordered
      .slice(0, 3)
      .map((value, index) => {
        if (index === 0) return value;
        const quality = Math.max(0.1, 1 - index * 0.1).toFixed(1);
        return `${value};q=${quality}`;
      })
      .join(", ");
  }

  function inferGooglePlatform(platform) {
    const normalized = String(platform || "").toLowerCase();
    if (normalized.includes("mac")) return "macOS";
    if (normalized.includes("win")) return "Windows";
    if (normalized.includes("android")) return "Android";
    if (normalized.includes("ios")) return "iOS";
    if (normalized.includes("linux")) return "Linux";
    return "macOS";
  }

  function stripUndefinedFields(object) {
    return Object.fromEntries(
      Object.entries(object).filter(([, value]) => value !== undefined),
    );
  }

  function inferGoogleArchitecture(architecture) {
    const normalized = String(architecture || process.arch || "").toLowerCase();
    if (normalized.includes("arm")) return "arm";
    if (normalized.includes("64") || normalized.includes("x64")) return "x86";
    if (normalized.includes("86")) return "x86";
    return "x86";
  }

  function inferGoogleBitness(bitness) {
    const normalized = String(bitness || "").trim();
    if (normalized === "64" || normalized === "32") return normalized;
    return process.arch.includes("64") ? "64" : "32";
  }

  function inferGooglePlatformVersion(platformVersion) {
    const normalized = String(platformVersion || "").trim();
    if (normalized) return normalized;
    if (process.platform === "darwin") {
      const release = os.release().split(".");
      const darwinMajor = parseInt(release[0] || "0", 10);
      if (darwinMajor >= 24) return `${darwinMajor - 9}.0.0`;
      if (darwinMajor >= 20) return `${darwinMajor - 9}.0.0`;
    }
    return "0.0.0";
  }

  function buildGoogleUaMetadata(runtime, fullVersion, platform) {
    const majorVersion = fullVersion.split(".")[0] || "99";
    const normalizedBrands = Array.isArray(runtime?.brands)
      ? runtime.brands
          .map((entry) => ({
            brand: String(entry?.brand || "").trim(),
            version: String(entry?.version || "").trim(),
          }))
          .filter((entry) => entry.brand && entry.version)
      : [];
    const fallbackBrands = [
      { brand: "Chromium", version: majorVersion },
      { brand: "Google Chrome", version: majorVersion },
      { brand: "Not:A-Brand", version: "99" },
    ];
    const resolvedBrands = normalizedBrands.length
      ? normalizedBrands
      : fallbackBrands;
    const normalizedFullVersionList = Array.isArray(runtime?.fullVersionList)
      ? runtime.fullVersionList
          .map((entry) => ({
            brand: String(entry?.brand || "").trim(),
            version: String(entry?.version || "").trim(),
          }))
          .filter((entry) => entry.brand && entry.version)
      : [];
    const fullVersionList = normalizedFullVersionList.length
      ? normalizedFullVersionList
      : resolvedBrands.map((entry) => ({
          brand: entry.brand,
          version:
            entry.version === majorVersion && fullVersion
              ? fullVersion
              : entry.version.includes(".")
                ? entry.version
                : `${entry.version}.0.0.0`,
        }));

    return {
      brands: resolvedBrands,
      fullVersion: fullVersion || `${majorVersion}.0.0.0`,
      fullVersionList,
      platform,
      platformVersion: inferGooglePlatformVersion(runtime?.platformVersion),
      architecture: inferGoogleArchitecture(runtime?.architecture),
      model: String(runtime?.model || ""),
      mobile: Boolean(runtime?.mobile),
      bitness: inferGoogleBitness(runtime?.bitness),
      wow64: Boolean(runtime?.wow64),
    };
  }

  async function getGoogleNavigationProfile(browser) {
    const cachedProfile = _googleNavigationProfiles.get(browser);
    if (cachedProfile) return cachedProfile;

    const profilePromise = (async () => {
      const page = await browser.newPage();
      try {
        const cdp = await page.target().createCDPSession();
        const versionInfo = await cdp.send("Browser.getVersion");
        const runtime = await page.evaluate(async () => {
          const lowEntropy = {
            userAgent: navigator.userAgent || "",
            language: navigator.language || "",
            languages: Array.isArray(navigator.languages)
              ? navigator.languages
              : [],
            platform:
              navigator.userAgentData?.platform || navigator.platform || "",
            mobile: Boolean(navigator.userAgentData?.mobile),
            brands: Array.isArray(navigator.userAgentData?.brands)
              ? navigator.userAgentData.brands
              : [],
          };

          if (
            navigator.userAgentData &&
            typeof navigator.userAgentData.getHighEntropyValues === "function"
          ) {
            const highEntropy =
              await navigator.userAgentData.getHighEntropyValues([
                "architecture",
                "bitness",
                "fullVersionList",
                "model",
                "platformVersion",
                "uaFullVersion",
                "wow64",
              ]);
            return { ...lowEntropy, ...highEntropy };
          }

          return lowEntropy;
        });

        const userAgent = normalizeChromeUserAgent(
          versionInfo.userAgent || runtime.userAgent,
        );
        const fullVersion = extractChromeVersion(
          runtime.uaFullVersion,
          userAgent,
          versionInfo.product,
          runtime.userAgent,
        );
        const platform = inferGooglePlatform(runtime.platform);
        const acceptLanguage =
          buildGoogleAcceptLanguage(runtime.languages, runtime.language) ||
          GOOGLE_DEFAULT_ACCEPT_LANGUAGE;

        return {
          userAgent,
          acceptLanguage,
          platform,
        };
      } finally {
        try {
          await page.close();
        } catch {
          // Ignore cleanup errors for the temporary profile probe page.
        }
      }
    })().catch((error) => {
      _googleNavigationProfiles.delete(browser);
      throw error;
    });

    _googleNavigationProfiles.set(browser, profilePromise);
    return profilePromise;
  }

  function normalizeGoogleResultUrl(rawHref) {
    const href = String(rawHref || "").trim();
    if (!href) return "";

    try {
      if (href.startsWith("/url?")) {
        const parsed = new URL(href, "https://www.google.com");
        return parsed.searchParams.get("q") || "";
      }
      if (href.startsWith("http://") || href.startsWith("https://")) {
        return href;
      }
      return new URL(href, "https://www.google.com").toString();
    } catch {
      return "";
    }
  }

  function postProcessGoogleResults(results) {
    const deduped = [];
    const seenUrls = new Set();

    for (const item of Array.isArray(results) ? results : []) {
      const url = normalizeGoogleResultUrl(
        item.url || item.href || item.rawHref || "",
      );
      if (!url || seenUrls.has(url)) continue;

      let hostname = "";
      try {
        hostname = new URL(url).hostname;
      } catch {
        continue;
      }
      if (/google\./i.test(hostname)) continue;

      const title = String(item.title || "")
        .replace(/\s+/g, " ")
        .trim();
      if (!title) continue;

      const snippet = String(item.snippet || item.text || "")
        .replace(/\s+/g, " ")
        .replace(
          new RegExp(`^${title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*`),
          "",
        )
        .trim();

      seenUrls.add(url);
      deduped.push({ url, title, snippet: snippet.slice(0, 500) });
    }

    return deduped;
  }

  function mergeGoogleResults(
    existingResults,
    incomingResults,
    maxResults = 100,
  ) {
    const merged = [];
    const seenUrls = new Set();

    for (const item of [
      ...(existingResults || []),
      ...(incomingResults || []),
    ]) {
      const url = String(item.url || "").trim();
      if (!url || seenUrls.has(url)) continue;
      seenUrls.add(url);
      merged.push(item);
      if (merged.length >= maxResults) break;
    }

    return merged;
  }

  async function extractGoogleResultsFromPage(page) {
    const payload = await page.evaluate(() => {
      const bodyText = document.body ? document.body.innerText || "" : "";
      const pageTitle = document.title || "";
      const googleOwnedPage = /(^|\.)google\./i.test(location.hostname || "");
      const interruptionUi = Boolean(
        document.querySelector(
          'form[action*="sorry"], iframe[src*="recaptcha"], #captcha, input[name="captcha"], textarea#g-recaptcha-response, div.g-recaptcha, form#captcha-form',
        ),
      );
      const challengeText = `${pageTitle}\n${bodyText}`
        .replace(/\s+/g, " ")
        .slice(0, 4000);

      const results = [];
      const seen = new Set();
      for (const anchor of document.querySelectorAll("a")) {
        const heading = anchor.querySelector("h3");
        if (!heading) continue;

        const rawHref = anchor.getAttribute("href") || anchor.href || "";
        if (!rawHref || seen.has(rawHref)) continue;
        seen.add(rawHref);

        const title = (heading.textContent || "").replace(/\s+/g, " ").trim();
        if (!title) continue;

        const container =
          anchor.closest("div[data-snc]") ||
          anchor.closest("div.g") ||
          anchor.closest("div[data-ved]") ||
          anchor.closest("div.MjjYud") ||
          anchor.closest("div.N54PNb") ||
          anchor.closest("div");

        let snippet = "";
        if (container) {
          snippet = (container.innerText || "").replace(/\s+/g, " ").trim();
        }

        results.push({
          rawHref,
          title,
          snippet,
          text: (anchor.textContent || "").replace(/\s+/g, " ").trim(),
        });
      }

      const captchaTextMatch =
        /detected unusual traffic|about this page|before you continue|verify you are human|not a robot|press and hold|enable javascript|unusual traffic from your computer/i.test(
          challengeText,
        );
      // Flag as challenge if: /sorry/ URL, CAPTCHA UI elements, CAPTCHA body
      // text, OR a Google-owned page that returned zero parseable results
      // (which almost always means a consent/verification interstitial).
      const challenge =
        location.href.includes("/sorry/") ||
        interruptionUi ||
        captchaTextMatch ||
        (googleOwnedPage && results.length === 0);

      return {
        challenge,
        title: pageTitle,
        href: location.href,
        bodyText: bodyText.slice(0, 1200),
        results,
      };
    });

    return {
      challenge: Boolean(payload.challenge),
      rawResultCount: Array.isArray(payload.results) ? payload.results.length : 0,
      pageTitle: payload.title || "",
      pageUrl: payload.href || "",
      bodyText: payload.bodyText || "",
      results: postProcessGoogleResults(payload.results),
    };
  }

  async function runSerializedLiveChromeSearch(task) {
    const next = _liveChromeSearchQueue.then(task, task);
    _liveChromeSearchQueue = next.catch(() => {});
    return next;
  }

  async function resolveGoogleChallengeViaLiveChrome(searchUrl) {
    if (!canUseLiveChromeFallback()) {
      throw new Error("Live Chrome fallback is unavailable on this platform.");
    }

    // Helper: safely extract results from a page that may be mid-navigation.
    // On context destruction (Google redirect), waits for the page to settle
    // and retries extraction rather than immediately giving up.
    async function safeExtract(page) {
      for (let retries = 0; retries < 3; retries++) {
        try {
          return await extractGoogleResultsFromPage(page);
        } catch (err) {
          if (/context.*destroy|navigation/i.test(err.message || "")) {
            // Page is navigating — wait for it to settle, then retry.
            try {
              await page.waitForNavigation({
                waitUntil: "domcontentloaded",
                timeout: 10000,
              });
            } catch {
              // waitForNavigation may also fail if already settled.
              await sleep(2000);
            }
            continue;
          }
          throw err;
        }
      }
      // After retries, return a challenge stub so the poll loop keeps going.
      return {
        challenge: true,
        pageTitle: "",
        pageUrl: page.url(),
        bodyText: "",
        results: [],
      };
    }

    // After CAPTCHA is solved, copy cookies from the interactive browser back
    // to the headless browser profile so subsequent headless requests benefit
    // from the cleared CAPTCHA tokens.
    async function transferCookiesToHeadless(page) {
      try {
        const cdp = await page.target().createCDPSession();
        const { cookies } = await cdp.send("Network.getAllCookies");
        const googleCookies = cookies.filter(
          (c) => c.domain && /google\./i.test(c.domain),
        );
        if (googleCookies.length > 0) {
          const cookieJson = JSON.stringify(googleCookies);
          // Write to both the base profile (for persistence) and the headless
          // profile (for immediate use without requiring a browser restart).
          const basePath = path.join(
            GOOGLE_BROWSER_PROFILE_DIR,
            "_captcha_cookies.json",
          );
          await fs.writeFile(basePath, cookieJson, "utf8");
          if (
            _browserUserDataDir &&
            _browserUserDataDir !== GOOGLE_BROWSER_PROFILE_DIR
          ) {
            const headlessPath = path.join(
              _browserUserDataDir,
              "_captcha_cookies.json",
            );
            await fs.writeFile(headlessPath, cookieJson, "utf8");
          }
          process.stderr.write(
            `[git-research-mcp] Saved ${googleCookies.length} Google cookies from interactive session for headless reuse\n`,
          );
        }
      } catch (err) {
        process.stderr.write(
          `[git-research-mcp] Cookie transfer failed (non-critical): ${err.message}\n`,
        );
      }
    }

    return runSerializedLiveChromeSearch(async () =>
      runInteractiveGoogleBrowser(async (page) => {
        process.stderr.write(
          `[git-research-mcp] Interactive Chrome navigating to ${searchUrl}\n`,
        );

        // Google may redirect mid-navigation (e.g. to /sorry/ or consent pages),
        // which destroys the page execution context. Handle gracefully.
        try {
          await page.goto(searchUrl, {
            waitUntil: "domcontentloaded",
            timeout: 30000,
          });
        } catch (navErr) {
          if (/context.*destroy|navigation/i.test(navErr.message || "")) {
            process.stderr.write(
              `[git-research-mcp] Interactive Chrome navigation redirect detected — waiting for page to settle\n`,
            );
            try {
              await page.waitForNavigation({
                waitUntil: "domcontentloaded",
                timeout: 15000,
              });
            } catch {
              await sleep(3000);
            }
          } else {
            throw navErr;
          }
        }

        process.stderr.write(
          `[git-research-mcp] Interactive Chrome reached ${page.url()}\n`,
        );

        // Bring Chrome to the foreground on macOS so the user can see/solve CAPTCHA.
        if (process.platform === "darwin") {
          const pid = page.browser().process()?.pid;
          if (pid) {
            try {
              await execFileAsync("osascript", [
                "-e",
                `tell application "System Events" to set frontmost of first process whose unix id is ${pid} to true`,
              ]);
            } catch {
              // Best-effort — window may still be behind VS Code.
            }
          }
        }

        let outcome = await safeExtract(page);

        // Poll: wait for user to solve CAPTCHA, or for results to appear.
        for (
          let attempt = 0;
          attempt < GOOGLE_CAPTCHA_POLL_ATTEMPTS;
          attempt++
        ) {
          if (!outcome.challenge && outcome.results.length > 0) {
            break;
          }
          await sleep(GOOGLE_CAPTCHA_POLL_DELAY_SECONDS * 1000);
          outcome = await safeExtract(page);
        }

        // If we got results, save the session cookies for headless reuse.
        if (!outcome.challenge && outcome.results.length > 0) {
          await transferCookiesToHeadless(page);
        }

        return {
          keepBrowserOpen: Boolean(outcome.challenge),
          output: {
            challenge: Boolean(outcome.challenge),
            pageTitle: outcome.pageTitle || "",
            pageUrl: outcome.pageUrl || "",
            bodyText: outcome.bodyText || "",
            results: Array.isArray(outcome.results) ? outcome.results : [],
            resultsCount: Array.isArray(outcome.results)
              ? outcome.results.length
              : 0,
          },
        };
      }),
    );
  }

  async function collectGoogleResultsViaLiveChrome(searchUrls) {
    throw new Error(
      "Live Chrome scraping fallback is disabled. Result collection must stay in headless Puppeteer.",
    );
  }

  function getPuppeteer() {
    if (_puppeteer) return _puppeteer;
    try {
      _puppeteer = require("puppeteer");
    } catch {
      // Global install — need NODE_PATH
      const globalRoot = path.join(
        process.env.HOME || "",
        ".nvm/versions/node",
        process.version,
        "lib/node_modules",
      );
      _puppeteer = require(path.join(globalRoot, "puppeteer"));
    }
    return _puppeteer;
  }

  async function getBrowser() {
    if (_browser && _browser.connected) return _browser;
    if (_browserLaunchPromise) return _browserLaunchPromise;
    _browserLaunchPromise = (async () => {
      const puppeteer = getPuppeteer();
      _browser = await puppeteer.launch({
        userDataDir: await getBrowserUserDataDir(),
        executablePath: resolvePuppeteerBrowserExecutable(puppeteer),
        headless: true,
        ignoreDefaultArgs: ["--enable-automation"],
        args: [
          "--no-sandbox",
          "--disable-setuid-sandbox",
          "--disable-dev-shm-usage",
          "--disable-gpu",
          "--disable-blink-features=AutomationControlled",
          "--window-size=1440,900",
        ],
      });
      _browser.on("disconnected", () => {
        _browser = null;
        _browserLaunchPromise = null;
      });
      process.stderr.write("[git-research-mcp] Headless Chrome launched\n");
      return _browser;
    })();
    return _browserLaunchPromise;
  }

  async function resetHeadlessBrowser() {
    const browser = _browser;
    _browser = null;
    _browserLaunchPromise = null;
    if (browser) {
      try {
        await browser.close();
      } catch {
        // Ignore browser shutdown errors during forced reset.
      }
    }
  }

  // Shut down browser on process exit
  process.on("exit", () => {
    if (_browser)
      try {
        _browser.close();
      } catch {}
  });
  process.on("SIGTERM", () => {
    if (_browser)
      try {
        _browser.close();
      } catch {}
    process.exit(0);
  });

  async function applyGoogleNavigationProfile(page) {
    const profile = await getGoogleNavigationProfile(page.browser());
    const cdp = await page.target().createCDPSession();
    await cdp.send("Network.enable");
    await cdp.send("Network.setUserAgentOverride", {
      userAgent: profile.userAgent,
      acceptLanguage: profile.acceptLanguage,
      platform: profile.platform,
    });

    // Merge consent cookies with any saved CAPTCHA-session cookies.
    const baseCookies = GOOGLE_CONSENT_COOKIES.map((cookie) => ({
      ...cookie,
      url: "https://www.google.com/",
    }));
    let captchaCookies = [];
    try {
      const cookiePath = path.join(
        GOOGLE_BROWSER_PROFILE_DIR,
        "_captcha_cookies.json",
      );
      const raw = await fs.readFile(cookiePath, "utf8");
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed) && parsed.length > 0) {
        captchaCookies = parsed.map((c) => ({
          name: c.name,
          value: c.value,
          domain: c.domain,
          path: c.path || "/",
          secure: c.secure !== false,
          httpOnly: Boolean(c.httpOnly),
          ...(c.expires && c.expires > 0 ? { expires: c.expires } : {}),
        }));
        process.stderr.write(
          `[git-research-mcp] Injecting ${captchaCookies.length} saved CAPTCHA-session cookies into headless browser\n`,
        );
      }
    } catch {
      // No saved cookies — normal on first run.
    }
    await cdp.send("Network.setCookies", {
      cookies: [...baseCookies, ...captchaCookies],
    });

    await page.setViewport({ width: 1440, height: 900 });
    await page.setExtraHTTPHeaders({
      "accept-language": profile.acceptLanguage,
    });
    await page.evaluateOnNewDocument(() => {
      Object.defineProperty(navigator, "webdriver", {
        get: () => undefined,
      });
    });
  }

  /**
   * Search Google using headless Chrome.
   * Returns rendered results when possible, otherwise signals a challenge.
   */
  async function searchGoogleHeadless(url) {
    let lastError;
    for (let attempt = 0; attempt < RETRY_MAX_ATTEMPTS; attempt++) {
      let page;
      try {
        const browser = await getBrowser();
        page = await browser.newPage();
        await applyGoogleNavigationProfile(page);

        const response = await page.goto(url, {
          waitUntil: "domcontentloaded",
          timeout: 15000,
        });

        try {
          await page.waitForSelector("#search", { timeout: 3000 });
        } catch {
          // #search may not exist on challenge pages.
        }

        let outcome;
        try {
          outcome = await extractGoogleResultsFromPage(page);
        } catch (extractErr) {
          // "Execution context was destroyed" happens when Google redirects
          // mid-evaluate (e.g. to /sorry/ CAPTCHA page). Treat as a challenge.
          if (/context.*destroy|navigation/i.test(extractErr.message || "")) {
            const currentUrl = page.url();
            process.stderr.write(
              `[git-research-mcp] Page context destroyed during extraction (redirected to ${currentUrl}) — treating as challenge\n`,
            );
            await page.close();
            return {
              challenge: true,
              pageTitle: "",
              pageUrl: currentUrl,
              bodyText: "",
              results: [],
            };
          }
          throw extractErr;
        }
        await page.close();

        if (response && response.status() === 429) {
          return {
            ...outcome,
            challenge: true,
          };
        }

        // Challenge detected — return immediately, don't waste retries.
        // Challenges aren't transient; retrying just adds delay.
        if (outcome.challenge) {
          process.stderr.write(
            `[git-research-mcp] Challenge detected on ${outcome.pageUrl || url} — returning immediately\n`,
          );
          return outcome;
        }

        // Detect challenge gap: in-page extraction found DOM elements with
        // <a><h3> (so raw challenge detection said "not a challenge"), but
        // postProcessGoogleResults filtered all of them out. This usually
        // means Google served a non-standard interstitial (consent, cookie
        // wall, new CAPTCHA variant) with h3 elements in nav/chrome that
        // aren't real search results. Upgrade to challenge.
        if (
          outcome.results.length === 0 &&
          outcome.rawResultCount > 0
        ) {
          process.stderr.write(
            `[git-research-mcp] Challenge detection gap: ${outcome.rawResultCount} raw results all filtered to 0 by post-processing. URL=${outcome.pageUrl || url} title=${JSON.stringify(outcome.pageTitle || "")} — upgrading to challenge\n`,
          );
          return {
            ...outcome,
            challenge: true,
          };
        }

        // Log diagnostic info when headless gets 0 results without detecting a challenge.
        if (outcome.results.length === 0) {
          process.stderr.write(
            `[git-research-mcp] Headless got 0 results, no challenge flag. URL=${outcome.pageUrl || url} title=${JSON.stringify(outcome.pageTitle || "")} body=${JSON.stringify((outcome.bodyText || "").slice(0, 300))} status=${response ? response.status() : "unknown"}\n`,
          );
        }

        return outcome;
      } catch (err) {
        if (page)
          try {
            await page.close();
          } catch {}

        // "Execution context was destroyed" during goto/waitForSelector means
        // Google redirected to a CAPTCHA page mid-navigation. Signal challenge
        // immediately instead of wasting retries.
        if (/context.*destroy|navigation/i.test(err.message || "")) {
          process.stderr.write(
            `[git-research-mcp] Navigation context destroyed — treating as CAPTCHA challenge\n`,
          );
          return {
            challenge: true,
            pageTitle: "",
            pageUrl: url,
            bodyText: "",
            results: [],
          };
        }

        if (attempt < RETRY_MAX_ATTEMPTS - 1) {
          const delayMs = Math.min(
            RETRY_BASE_DELAY_MS * Math.pow(2, attempt),
            RETRY_MAX_DELAY_MS,
          );
          process.stderr.write(
            `[git-research-mcp] Google fetch error: ${err.message} — retrying in ${(delayMs / 1000).toFixed(0)}s (attempt ${attempt + 1}/${RETRY_MAX_ATTEMPTS})\n`,
          );
          lastError = err;
          await sleep(delayMs);
          continue;
        }
        throw err;
      }
    }
    throw lastError;
  }

  /**
   * Parse Google search result HTML into structured results.
   * Google's DOM changes occasionally — this targets the data-attribute
   * structure that has been stable since ~2023.
   */
  function parseGoogleResults(html) {
    const results = [];
    const seenUrls = new Set();

    // Strategy 1: Match <a href="/url?q=..." blocks (Google's redirect links)
    // Each search result has a heading inside an <h3> and a snippet in a nearby div.
    // We extract URL, title, and snippet from the raw HTML.

    // Extract all result blocks: Google wraps results in <div class="g"> or
    // data-ved attributes. We look for the /url?q= redirect pattern.
    const linkPattern =
      /<a[^>]+href="\/url\?q=([^"&]+)[^"]*"[^>]*>([\s\S]*?)<\/a>/gi;
    let match;
    while ((match = linkPattern.exec(html)) !== null) {
      let url;
      try {
        url = decodeURIComponent(match[1]);
      } catch {
        continue;
      }
      // Skip Google's own links
      if (
        url.startsWith("https://accounts.google.com") ||
        url.startsWith("https://support.google.com") ||
        url.startsWith("https://policies.google.com") ||
        url.startsWith("https://maps.google.com") ||
        url.startsWith("/")
      )
        continue;
      if (seenUrls.has(url)) continue;
      seenUrls.add(url);

      // Extract title from <h3> inside the link
      const h3Match = match[2].match(/<h3[^>]*>([\s\S]*?)<\/h3>/i);
      const title = h3Match
        ? h3Match[1].replace(/<[^>]+>/g, "").trim()
        : match[2].replace(/<[^>]+>/g, "").trim();

      if (!title) continue;

      results.push({
        url,
        title,
        snippet: "",
        _matchEnd: linkPattern.lastIndex,
      });
    }

    // Strategy 2: If Strategy 1 found URLs, try to extract snippets.
    // Snippets typically live in <div>/<span> elements after each result link.
    // We use the saved match position from Strategy 1 to search a bounded
    // window of HTML for snippet-class elements. The window is capped at the
    // start of the next result (to prevent cross-result bleed) or 3000 chars.
    if (results.length > 0) {
      const snippetClassRe =
        /<(?:span|div)[^>]*class="[^"]*(?:VwiC3b|st|IsZvec|s3v9rd|lEBKkf)[^"]*"[^>]*>([\s\S]*?)<\/(?:span|div)>/gi;
      for (let i = 0; i < results.length; i++) {
        const result = results[i];
        const windowStart = result._matchEnd || 0;
        // Bound the window to the next result's match position (prevent bleed)
        const nextStart =
          i + 1 < results.length
            ? results[i + 1]._matchEnd || windowStart + 3000
            : windowStart + 3000;
        const windowEnd = Math.min(windowStart + 3000, nextStart);
        const window = html.slice(windowStart, windowEnd);

        // Find all snippet-class matches in the window and pick the longest
        snippetClassRe.lastIndex = 0;
        let best = "";
        let m;
        while ((m = snippetClassRe.exec(window)) !== null) {
          const text = m[1]
            .replace(/<[^>]+>/g, " ")
            .replace(/&amp;/g, "&")
            .replace(/&lt;/g, "<")
            .replace(/&gt;/g, ">")
            .replace(/&quot;/g, '"')
            .replace(/&#39;/g, "'")
            .replace(/\s+/g, " ")
            .trim();
          if (text.length > best.length) best = text;
        }
        if (best) result.snippet = best.slice(0, 500);
        delete result._matchEnd;
      }
    }

    // Strategy 3: Fallback — if redirect links didn't work, try direct hrefs
    // to external sites (Google sometimes uses direct links).
    if (results.length === 0) {
      const directPattern =
        /<h3[^>]*>([\s\S]*?)<\/h3>[\s\S]*?<a[^>]+href="(https?:\/\/(?!(?:www\.)?google\.com)[^"]+)"[^>]*>/gi;
      while ((match = directPattern.exec(html)) !== null) {
        const title = match[1].replace(/<[^>]+>/g, "").trim();
        const url = match[2];
        if (!title || !url || seenUrls.has(url)) continue;
        seenUrls.add(url);
        results.push({ url, title, snippet: "" });
      }
    }

    // Also try reversed order: <a href="https://..."><h3>...</h3></a>
    if (results.length === 0) {
      const reversePattern =
        /<a[^>]+href="(https?:\/\/(?!(?:www\.)?google\.com)[^"]+)"[^>]*>[\s\S]*?<h3[^>]*>([\s\S]*?)<\/h3>/gi;
      while ((match = reversePattern.exec(html)) !== null) {
        const url = match[1];
        const title = match[2].replace(/<[^>]+>/g, "").trim();
        if (!title || !url || seenUrls.has(url)) continue;
        seenUrls.add(url);
        results.push({ url, title, snippet: "" });
      }
    }

    return results;
  }

  return {
    googleRateLimit,
    canUseLiveChromeFallback,
    searchGoogleHeadless,
    parseGoogleResults,
    collectGoogleResultsViaLiveChrome,
    resolveGoogleChallengeViaLiveChrome,
    resetHeadlessBrowser,
    getPuppeteer,
    getBrowser,
    postProcessGoogleResults,
    mergeGoogleResults,
    runInteractiveGoogleBrowser,
  };
};
