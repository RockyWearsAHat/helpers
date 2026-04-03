#!/usr/bin/env node
"use strict";

const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawn, spawnSync } = require("child_process");

const REPO_ROOT = path.resolve(__dirname, "..");
const DEFAULT_BASE_URL =
  "https://rockywearsahat.github.io/github-shell-helpers/";
const DEFAULT_OUTPUT_ROOT = path.join(REPO_ROOT, "build", "visual-captures");
const DEFAULT_VIEWPORT = { width: 1440, height: 900 };
const DEFAULT_SCROLL_OVERLAP = 96;
const DEFAULT_JPEG_QUALITY = 60;
const DEFAULT_MIN_JPEG_QUALITY = 40;
const DEFAULT_CAPTURE_SCALE = 0.8;
const DEFAULT_MIN_CAPTURE_SCALE = 0.55;
const DEFAULT_CAPTURE_SCALE_STEP = 0.1;
const DEFAULT_MAX_IMAGE_BYTES = 350 * 1024;
const DEFAULT_SCROLL_SETTLE_MS = 300;
const DEFAULT_PAGE_READY_TIMEOUT_MS = 15000;
const DEFAULT_CDP_TIMEOUT_MS = 15000;
const DEFAULT_CHROME_STARTUP_TIMEOUT_MS = 15000;
const DEFAULT_CHROME_PATH =
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const LIVE_SITE_FLOWS = [
  { name: "browse", path: "", maxScrolls: 3, settleMs: 12000 },
  { name: "search", path: "?q=graph+algorithms", maxScrolls: 8, settleMs: 12000 },
  { name: "about", path: "?page=about", maxScrolls: 8, settleMs: 5000 },
  {
    name: "reader",
    path: "?doc=knowledge/algorithms-graph.md",
    maxScrolls: 10,
    settleMs: 12000,
  },
  {
    name: "practice",
    path: "?doc=knowledge/algorithms-graph.md&practice=1",
    maxScrolls: 8,
    settleMs: 14000,
    preScrollJs: `
      // Ensure practice panel is visible
      var panel = document.getElementById('practice-panel');
      if (panel) panel.classList.remove('hidden');
      var btn = document.getElementById('practice-toggle');
      if (btn) { btn.classList.add('active'); btn.setAttribute('aria-pressed','true'); }
    `,
  },
  {
    name: "practice-solving",
    path: "?doc=knowledge/algorithms-graph.md&practice=1",
    maxScrolls: 6,
    settleMs: 16000,
    preScrollJs: `
      // Open panel
      var panel = document.getElementById('practice-panel');
      if (panel) panel.classList.remove('hidden');
      var btn = document.getElementById('practice-toggle');
      if (btn) btn.classList.add('active');
      // Switch to problem tab
      var tabs = document.querySelectorAll('.practice-tab');
      if (tabs[0]) tabs[0].click();
      // Set some code in the editor to show the IDE in action
      setTimeout(function() {
        var cm = document.querySelector('.CodeMirror');
        if (cm && cm.CodeMirror) {
          cm.CodeMirror.setValue('def bfs(graph, start):\\n    visited = set()\\n    queue = [start]\\n    result = []\\n    while queue:\\n        node = queue.pop(0)\\n        if node not in visited:\\n            visited.add(node)\\n            result.append(node)\\n            queue.extend(graph.get(node, []))\\n    return result\\n');
        }
      }, 1000);
    `,
  },
];


function printUsage() {
  const lines = [
    "Capture browser flow screenshots from the published Atlas site.",
    "",
    "Usage:",
    "  node scripts/capture-live-site-browser-flows.js [options]",
    "",
    "Options:",
    `  --base-url <url>       Base site URL (default: ${DEFAULT_BASE_URL})`,
    "  --flows <names>        Comma-separated flow names (browse,search,about,reader,practice,practice-solving)",
    "  --output-dir <path>    Output directory (default: build/visual-captures/live-site-<timestamp>)",
    "  --viewport <WxH>       Capture viewport (default: 1440x900)",
    `  --quality <1-100>      JPEG quality (default: ${DEFAULT_JPEG_QUALITY})`,
    `  --scale <0.1-1.0>      Capture scale (default: ${DEFAULT_CAPTURE_SCALE})`,
    `  --max-image-kb <num>   Per-image size cap in KB (default: ${DEFAULT_MAX_IMAGE_BYTES / 1024})`,
    "  --chrome <path>        Chrome executable path",
    "  --list-flows           Print available flows and exit",
    "  --help                 Show this help",
  ];

  console.log(lines.join("\n"));
}

function timestampToken(date = new Date()) {
  const pad = (value) => String(value).padStart(2, "0");
  return [
    date.getFullYear(),
    pad(date.getMonth() + 1),
    pad(date.getDate()),
    "-",
    pad(date.getHours()),
    pad(date.getMinutes()),
    pad(date.getSeconds()),
  ].join("");
}

function readFlagValue(argv, index, inlineValue, flagName) {
  if (inlineValue !== undefined) {
    return { value: inlineValue, nextIndex: index };
  }
  if (index + 1 >= argv.length) {
    throw new Error(`Missing value for ${flagName}.`);
  }
  return { value: argv[index + 1], nextIndex: index + 1 };
}

function parsePositiveNumber(value, flagName) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`${flagName} must be a positive number.`);
  }
  return parsed;
}

function parseViewport(value) {
  const match = /^(\d+)x(\d+)$/i.exec(value || "");
  if (!match) {
    throw new Error(`Invalid viewport \"${value}\". Expected <width>x<height>.`);
  }
  const width = Number(match[1]);
  const height = Number(match[2]);
  if (width < 320 || height < 320) {
    throw new Error("Viewport must be at least 320x320.");
  }
  return { width, height };
}

function parseFlowSelection(value) {
  const names = String(value || "")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);

  if (names.length === 0) {
    throw new Error("--flows must include at least one flow name.");
  }

  const selected = [];
  for (const flowName of names) {
    const flow = LIVE_SITE_FLOWS.find((entry) => entry.name === flowName);
    if (!flow) {
      throw new Error(
        `Unknown flow \"${flowName}\". Available flows: ${LIVE_SITE_FLOWS.map((entry) => entry.name).join(", ")}`,
      );
    }
    selected.push(flow);
  }

  return selected;
}

function parseArgs(argv) {
  const config = {
    baseUrl: DEFAULT_BASE_URL,
    flows: LIVE_SITE_FLOWS,
    outputDir: path.join(
      DEFAULT_OUTPUT_ROOT,
      `live-site-${timestampToken()}`,
    ),
    viewport: { ...DEFAULT_VIEWPORT },
    quality: DEFAULT_JPEG_QUALITY,
    minQuality: DEFAULT_MIN_JPEG_QUALITY,
    scale: DEFAULT_CAPTURE_SCALE,
    minScale: DEFAULT_MIN_CAPTURE_SCALE,
    scaleStep: DEFAULT_CAPTURE_SCALE_STEP,
    maxImageBytes: DEFAULT_MAX_IMAGE_BYTES,
    scrollOverlap: DEFAULT_SCROLL_OVERLAP,
    scrollSettleMs: DEFAULT_SCROLL_SETTLE_MS,
    pageReadyTimeoutMs: DEFAULT_PAGE_READY_TIMEOUT_MS,
    cdpTimeoutMs: DEFAULT_CDP_TIMEOUT_MS,
    chromeStartupTimeoutMs: DEFAULT_CHROME_STARTUP_TIMEOUT_MS,
    chromePath: process.env.CHROME_BIN || DEFAULT_CHROME_PATH,
    help: false,
    listFlows: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const rawArg = argv[index];
    const splitIndex = rawArg.indexOf("=");
    const flag = splitIndex >= 0 ? rawArg.slice(0, splitIndex) : rawArg;
    const inlineValue = splitIndex >= 0 ? rawArg.slice(splitIndex + 1) : undefined;

    if (flag === "--help" || flag === "-h") {
      config.help = true;
      continue;
    }
    if (flag === "--list-flows") {
      config.listFlows = true;
      continue;
    }
    if (flag === "--base-url") {
      const result = readFlagValue(argv, index, inlineValue, flag);
      config.baseUrl = result.value;
      index = result.nextIndex;
      continue;
    }
    if (flag === "--flows") {
      const result = readFlagValue(argv, index, inlineValue, flag);
      config.flows = parseFlowSelection(result.value);
      index = result.nextIndex;
      continue;
    }
    if (flag === "--output-dir") {
      const result = readFlagValue(argv, index, inlineValue, flag);
      config.outputDir = path.resolve(result.value);
      index = result.nextIndex;
      continue;
    }
    if (flag === "--viewport") {
      const result = readFlagValue(argv, index, inlineValue, flag);
      config.viewport = parseViewport(result.value);
      index = result.nextIndex;
      continue;
    }
    if (flag === "--quality") {
      const result = readFlagValue(argv, index, inlineValue, flag);
      config.quality = parsePositiveNumber(result.value, flag);
      index = result.nextIndex;
      continue;
    }
    if (flag === "--scale") {
      const result = readFlagValue(argv, index, inlineValue, flag);
      config.scale = parsePositiveNumber(result.value, flag);
      index = result.nextIndex;
      continue;
    }
    if (flag === "--max-image-kb") {
      const result = readFlagValue(argv, index, inlineValue, flag);
      config.maxImageBytes = Math.round(
        parsePositiveNumber(result.value, flag) * 1024,
      );
      index = result.nextIndex;
      continue;
    }
    if (flag === "--chrome") {
      const result = readFlagValue(argv, index, inlineValue, flag);
      config.chromePath = result.value;
      index = result.nextIndex;
      continue;
    }

    throw new Error(`Unknown flag: ${rawArg}`);
  }

  config.baseUrl = new URL(config.baseUrl).toString();
  if (config.quality > 100) {
    throw new Error("--quality must not exceed 100.");
  }
  if (config.scale > 1) {
    throw new Error("--scale must not exceed 1.0.");
  }
  if (config.scale < config.minScale) {
    config.minScale = config.scale;
  }
  if (config.quality < config.minQuality) {
    config.minQuality = config.quality;
  }

  return config;
}

function listFlowDefinitions() {
  for (const flow of LIVE_SITE_FLOWS) {
    console.log(
      `${flow.name}: ${flow.path || "<base-url>"} (max scrolls: ${flow.maxScrolls}, settle: ${flow.settleMs}ms)`,
    );
  }
}

function resolveChromeExecutable(chromePath) {
  if (chromePath && fs.existsSync(chromePath)) {
    return chromePath;
  }

  for (const commandName of ["google-chrome", "chromium", "chromium-browser"]) {
    const probe = spawnSync("which", [commandName], { encoding: "utf8" });
    if (probe.status === 0) {
      const resolved = probe.stdout.trim();
      if (resolved) {
        return resolved;
      }
    }
  }

  throw new Error(
    `Chrome executable not found. Set CHROME_BIN or pass --chrome. Tried ${chromePath || DEFAULT_CHROME_PATH}.`,
  );
}

function resolveFlowUrl(baseUrl, flowPath) {
  if (!flowPath) {
    return new URL(".", baseUrl).toString();
  }
  return new URL(flowPath, baseUrl).toString();
}

function buildScrollPlan(pageHeight, viewportHeight, scrollOverlap, maxScrolls) {
  const step = Math.max(1, viewportHeight - scrollOverlap);
  const maxScrollY = Math.max(0, pageHeight - viewportHeight);
  const naturalStops = Math.max(1, Math.ceil(maxScrollY / step) + 1);
  const stopCount = Math.max(1, Math.min(maxScrolls, naturalStops));

  if (stopCount === 1 || maxScrollY === 0) {
    return [0];
  }

  const positions = [];
  for (let index = 0; index < stopCount; index += 1) {
    const ratio = index / (stopCount - 1);
    positions.push(Math.round(maxScrollY * ratio));
  }

  return [...new Set(positions)];
}

async function sleep(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchJson(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} for ${url}`);
  }
  return response.json();
}

async function waitForJson(url, timeoutMs) {
  const startedAt = Date.now();
  let lastError = null;
  while (Date.now() - startedAt < timeoutMs) {
    try {
      return await fetchJson(url);
    } catch (error) {
      lastError = error;
      await sleep(200);
    }
  }
  throw new Error(
    `Timed out waiting for ${url}${lastError ? ` (${lastError.message})` : ""}`,
  );
}

function getWebSocketConstructor() {
  if (typeof WebSocket === "function") {
    return WebSocket;
  }
  try {
    return require("ws");
  } catch (_error) {
    return null;
  }
}

function addSocketListener(socket, eventName, handler) {
  if (typeof socket.addEventListener === "function") {
    socket.addEventListener(eventName, handler);
    return;
  }
  if (typeof socket.on === "function") {
    socket.on(eventName, handler);
    return;
  }
  socket[`on${eventName}`] = handler;
}

async function normalizeSocketMessage(event) {
  const payload = event && typeof event === "object" && "data" in event
    ? event.data
    : event;

  if (typeof payload === "string") {
    return payload;
  }
  if (Buffer.isBuffer(payload)) {
    return payload.toString("utf8");
  }
  if (payload instanceof ArrayBuffer) {
    return Buffer.from(payload).toString("utf8");
  }
  if (ArrayBuffer.isView(payload)) {
    return Buffer.from(payload.buffer, payload.byteOffset, payload.byteLength).toString(
      "utf8",
    );
  }
  if (payload && typeof payload.text === "function") {
    return payload.text();
  }
  return String(payload);
}

class CDPSession {
  constructor(socket, timeoutMs) {
    this.socket = socket;
    this.timeoutMs = timeoutMs;
    this.id = 0;
    this.pending = new Map();
  }

  static async connect(wsUrl, timeoutMs) {
    const WebSocketCtor = getWebSocketConstructor();
    if (!WebSocketCtor) {
      throw new Error(
        "No WebSocket client available. Use Node.js with global WebSocket or install the ws package.",
      );
    }

    const socket = new WebSocketCtor(wsUrl);
    await new Promise((resolve, reject) => {
      addSocketListener(socket, "open", () => resolve());
      addSocketListener(socket, "error", (event) => {
        reject(event && event.error ? event.error : new Error("WebSocket connection failed"));
      });
    });

    const session = new CDPSession(socket, timeoutMs);
    addSocketListener(socket, "message", (event) => {
      Promise.resolve(normalizeSocketMessage(event))
        .then((text) => {
          const message = JSON.parse(text);
          if (!message.id || !session.pending.has(message.id)) {
            return;
          }

          const pending = session.pending.get(message.id);
          session.pending.delete(message.id);
          clearTimeout(pending.timeout);

          if (message.error) {
            pending.reject(
              new Error(`${message.error.message} (${message.error.code})`),
            );
            return;
          }

          pending.resolve(message);
        })
        .catch((error) => {
          for (const pending of session.pending.values()) {
            clearTimeout(pending.timeout);
            pending.reject(error);
          }
          session.pending.clear();
        });
    });

    addSocketListener(socket, "close", () => {
      for (const pending of session.pending.values()) {
        clearTimeout(pending.timeout);
        pending.reject(new Error("Chrome DevTools connection closed"));
      }
      session.pending.clear();
    });

    addSocketListener(socket, "error", (event) => {
      const error = event && event.error ? event.error : new Error("WebSocket error");
      for (const pending of session.pending.values()) {
        clearTimeout(pending.timeout);
        pending.reject(error);
      }
      session.pending.clear();
    });

    return session;
  }

  send(method, params = {}, timeoutMs = this.timeoutMs) {
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Timed out waiting for ${method}`));
      }, timeoutMs);

      this.pending.set(id, { resolve, reject, timeout });

      try {
        this.socket.send(JSON.stringify({ id, method, params }));
      } catch (error) {
        clearTimeout(timeout);
        this.pending.delete(id);
        reject(error);
      }
    });
  }

  close() {
    if (this.socket && typeof this.socket.close === "function") {
      this.socket.close();
    }
  }
}

function runtimeValue(response) {
  return response && response.result && response.result.result
    ? response.result.result.value
    : undefined;
}

async function waitForDocumentReady(cdp, timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const response = await cdp.send("Runtime.evaluate", {
      expression:
        "document.readyState === 'complete' && !!document.body && document.body.scrollHeight > 0",
      returnByValue: true,
    });
    if (runtimeValue(response)) {
      return;
    }
    await sleep(250);
  }
  throw new Error("Timed out waiting for document readiness.");
}

async function captureViewport(cdp, config, scrollY) {
  let quality = config.quality;
  let scale = config.scale;

  while (true) {
    const response = await cdp.send("Page.captureScreenshot", {
      format: "jpeg",
      quality,
      fromSurface: true,
      optimizeForSpeed: true,
      captureBeyondViewport: false,
      clip: {
        x: 0,
        y: scrollY,
        width: config.viewport.width,
        height: config.viewport.height,
        scale,
      },
    });

    const imageData = response.result ? response.result.data : undefined;
    if (!imageData) {
      throw new Error("Chrome returned no screenshot data.");
    }

    const buffer = Buffer.from(imageData, "base64");
    if (
      buffer.length <= config.maxImageBytes ||
      (quality === config.minQuality && scale === config.minScale)
    ) {
      return { buffer, quality, scale };
    }

    if (quality > config.minQuality) {
      quality = Math.max(config.minQuality, quality - 10);
      continue;
    }

    scale = Math.max(
      config.minScale,
      Number((scale - config.scaleStep).toFixed(2)),
    );
  }
}

async function captureFlow(cdp, config, flow, outputDir) {
  const flowUrl = resolveFlowUrl(config.baseUrl, flow.path);
  console.log(`Capturing: ${flow.name} (${flowUrl})`);

  await cdp.send("Page.navigate", { url: flowUrl });
  await waitForDocumentReady(cdp, config.pageReadyTimeoutMs);
  await sleep(flow.settleMs);

  if (flow.preScrollJs) {
    await cdp.send("Runtime.evaluate", { expression: flow.preScrollJs });
    await sleep(2000);
  }

  const pageHeightResponse = await cdp.send("Runtime.evaluate", {
    expression: `Math.max(
      document.body ? document.body.scrollHeight : 0,
      document.documentElement ? document.documentElement.scrollHeight : 0,
      window.innerHeight
    )`,
    returnByValue: true,
  });
  const pageHeight = runtimeValue(pageHeightResponse) || config.viewport.height;
  const scrollPlan = buildScrollPlan(
    pageHeight,
    config.viewport.height,
    config.scrollOverlap,
    flow.maxScrolls,
  );

  console.log(
    `  ${flow.name}: page height = ${pageHeight}px, taking ${scrollPlan.length} compressed captures`,
  );

  const captures = [];
  for (const [index, scrollY] of scrollPlan.entries()) {
    console.log(`    capture ${index + 1}/${scrollPlan.length} at scrollY=${scrollY}`);
    await cdp.send("Runtime.evaluate", {
      expression: `window.scrollTo({ top: ${scrollY}, behavior: 'instant' })`,
    });
    await sleep(config.scrollSettleMs);

    const capture = await captureViewport(cdp, config, scrollY);
    const fileName = `${flow.name}-${String(index + 1).padStart(2, "0")}.jpg`;
    const filePath = path.join(outputDir, fileName);
    fs.writeFileSync(filePath, capture.buffer);
    console.log(
      `    -> ${filePath} (${Math.round(capture.buffer.length / 1024)} KB, q=${capture.quality}, scale=${capture.scale}, y=${scrollY})`,
    );

    captures.push({
      fileName,
      filePath,
      bytes: capture.buffer.length,
      scrollY,
      quality: capture.quality,
      scale: capture.scale,
    });
  }

  return {
    name: flow.name,
    url: flowUrl,
    pageHeight,
    captures,
  };
}

async function main() {
  const config = parseArgs(process.argv.slice(2));
  if (config.help) {
    printUsage();
    return;
  }
  if (config.listFlows) {
    listFlowDefinitions();
    return;
  }

  config.chromePath = resolveChromeExecutable(config.chromePath);
  fs.mkdirSync(config.outputDir, { recursive: true });

  const port = 9222 + Math.floor(Math.random() * 400);
  const userDataDir = fs.mkdtempSync(
    path.join(os.tmpdir(), "atlas-live-site-browser-flows-"),
  );
  const chromeArgs = [
    "--headless=new",
    `--remote-debugging-port=${port}`,
    `--window-size=${config.viewport.width},${config.viewport.height}`,
    `--user-data-dir=${userDataDir}`,
    "--disable-gpu",
    "--disable-extensions",
    "--disable-background-networking",
    "--hide-scrollbars",
    "--mute-audio",
    "--no-default-browser-check",
    "--no-first-run",
    "about:blank",
  ];

  if (process.platform === "linux") {
    chromeArgs.splice(chromeArgs.length - 1, 0, "--no-sandbox");
  }

  const chrome = spawn(config.chromePath, chromeArgs, { stdio: "ignore" });

  let cdp = null;
  try {
    const targets = await waitForJson(
      `http://127.0.0.1:${port}/json`,
      config.chromeStartupTimeoutMs,
    );
    const pageTarget = targets.find((target) => target.type === "page") || targets[0];
    if (!pageTarget || !pageTarget.webSocketDebuggerUrl) {
      throw new Error("Chrome DevTools did not expose a page target.");
    }

    cdp = await CDPSession.connect(pageTarget.webSocketDebuggerUrl, config.cdpTimeoutMs);
    await cdp.send("Page.enable");
    await cdp.send("Runtime.enable");
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: config.viewport.width,
      height: config.viewport.height,
      deviceScaleFactor: 1,
      mobile: false,
    });

    const manifest = {
      capturedAt: new Date().toISOString(),
      baseUrl: config.baseUrl,
      outputDir: config.outputDir,
      viewport: config.viewport,
      quality: config.quality,
      scale: config.scale,
      maxImageBytes: config.maxImageBytes,
      flows: [],
    };

    for (const flow of config.flows) {
      manifest.flows.push(await captureFlow(cdp, config, flow, config.outputDir));
    }

    const manifestPath = path.join(config.outputDir, "manifest.json");
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

    console.log("\nAll live-site captures complete!");
    console.log(`Output: ${config.outputDir}`);
    console.log(`Manifest: ${manifestPath}`);
  } finally {
    if (cdp) {
      cdp.close();
    }
    if (chrome.exitCode === null) {
      chrome.kill();
      await Promise.race([
        new Promise((resolve) => chrome.once("exit", resolve)),
        sleep(1000),
      ]);
    }
    fs.rmSync(userDataDir, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error.stack || error.message || String(error));
  process.exitCode = 1;
});