#!/usr/bin/env node
"use strict";
// npm postinstall: download the prebuilt native `helpers-native` binary for this
// platform into the package directory. The npm package is a thin shim — the
// runtime is the single Node-free binary. (npm is the one channel where Node
// already exists, so this small downloader is fine here.)

const fs = require("fs");
const path = require("path");
const https = require("https");
const { spawnSync } = require("child_process");

const REPO = "RockyWearsAHat/helpers";
const PKG_DIR = path.resolve(__dirname, "..");
const VERSION = (() => {
  try {
    return fs.readFileSync(path.join(PKG_DIR, "VERSION"), "utf8").trim();
  } catch {
    return "";
  }
})();

// Compile-independent host → release target tag.
function hostTag() {
  const { platform, arch } = process;
  const a = arch === "x64" ? "x86_64" : arch === "arm64" ? "aarch64" : arch;
  if (platform === "darwin") return "macos-universal";
  if (platform === "win32") return `windows-${a}`;
  if (platform === "linux") {
    let musl = false;
    try { musl = !process.report.getReport().header.glibcVersionRuntime; } catch {}
    return musl ? `linux-${a}-musl` : `linux-${a}`;
  }
  return null;
}

function download(url, dest, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 6) return reject(new Error("too many redirects"));
    https.get(url, { headers: { "User-Agent": "helpers-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        res.resume();
        return resolve(download(res.headers.location, dest, redirects + 1));
      }
      if (res.statusCode !== 200) {
        res.resume();
        return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
      }
      const out = fs.createWriteStream(dest);
      res.pipe(out);
      out.on("finish", () => out.close(resolve));
      out.on("error", reject);
    }).on("error", reject);
  });
}

async function main() {
  const tag = hostTag();
  if (!tag) {
    console.warn(`[helpers] no prebuilt for ${process.platform}/${process.arch}; build from source: helpers build --from-source`);
    return;
  }
  const exe = process.platform === "win32" ? ".exe" : "";
  const asset = `helpers-native-${tag}.tar.gz`;
  const urls = [];
  if (VERSION) urls.push(`https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`);
  urls.push(`https://github.com/${REPO}/releases/latest/download/${asset}`);

  const tarball = path.join(PKG_DIR, asset);
  let ok = false;
  for (const url of urls) {
    try { await download(url, tarball); ok = true; break; } catch { /* try next */ }
  }
  if (!ok) {
    console.warn(`[helpers] could not download ${asset}; run 'helpers build' later or build from source.`);
    return;
  }
  const ex = spawnSync("tar", ["-xf", asset], { cwd: PKG_DIR, stdio: "inherit" });
  fs.rmSync(tarball, { force: true });
  if (ex.status !== 0 || !fs.existsSync(path.join(PKG_DIR, `helpers-native${exe}`))) {
    console.warn("[helpers] could not extract the prebuilt binary.");
    return;
  }
  try { fs.chmodSync(path.join(PKG_DIR, `helpers-native${exe}`), 0o755); } catch {}
  console.log(`[helpers] installed helpers-native (${tag}). Run 'helpers install' to register with your agent.`);
}

main().catch((e) => console.warn(`[helpers] postinstall: ${e.message}`));
