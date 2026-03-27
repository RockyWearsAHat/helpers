// vision-tool/lib/video-frames.js
//
// ffmpeg / ffprobe operations for video frame extraction.
// Used by the video analysis pipeline. No vscode dependencies.

const { execFile } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

function execPromise(cmd, args, options = {}) {
  return new Promise((resolve, reject) => {
    execFile(
      cmd,
      args,
      { timeout: 120000, ...options },
      (err, stdout, stderr) => {
        if (err) reject(new Error((stderr || err.message || "").trim()));
        else resolve((stdout || "").trim());
      },
    );
  });
}

function checkDependency(name) {
  return new Promise((resolve) => {
    execFile("which", [name], (err, stdout) => {
      resolve(err ? null : (stdout || "").trim());
    });
  });
}

async function ensureDependencies() {
  const ffmpeg = await checkDependency("ffmpeg");
  const ffprobe = await checkDependency("ffprobe");
  const missing = [];
  if (!ffmpeg) missing.push("ffmpeg");
  if (!ffprobe) missing.push("ffprobe");
  if (missing.length) {
    throw new Error(
      `Required dependencies not found: ${missing.join(", ")}. ` +
        "Install with: brew install ffmpeg",
    );
  }
  return { ffmpeg, ffprobe };
}

function parseFps(fpsStr) {
  if (!fpsStr) return 0;
  const parts = String(fpsStr).split("/");
  if (parts.length === 2) {
    const num = parseFloat(parts[0]);
    const den = parseFloat(parts[1]);
    return den > 0 ? num / den : 0;
  }
  return parseFloat(fpsStr) || 0;
}

async function getVideoMetadata(videoPath) {
  const result = await execPromise("ffprobe", [
    "-v",
    "quiet",
    "-print_format",
    "json",
    "-show_format",
    "-show_streams",
    videoPath,
  ]);
  const data = JSON.parse(result);
  const videoStream = (data.streams || []).find(
    (s) => s.codec_type === "video",
  );
  const format = data.format || {};

  return {
    durationSec: parseFloat(format.duration) || 0,
    fps: videoStream ? parseFps(videoStream.r_frame_rate) : 0,
    width: videoStream ? parseInt(videoStream.width, 10) : 0,
    height: videoStream ? parseInt(videoStream.height, 10) : 0,
    codec: videoStream ? videoStream.codec_name : "unknown",
    bitRate: parseInt(format.bit_rate, 10) || 0,
    fileSize: parseInt(format.size, 10) || 0,
  };
}

function computeSamplingPlan(durationSec, options = {}) {
  const startSec = Math.max(0, options.startSec || 0);
  const endSec = options.endSec
    ? Math.min(options.endSec, durationSec)
    : durationSec;
  const effectiveDuration = endSec - startSec;

  if (effectiveDuration <= 0) {
    throw new Error(
      `Invalid time window: ${startSec}s to ${endSec}s (video is ${durationSec}s)`,
    );
  }

  let interval = options.sampleEverySec;
  if (!interval) {
    if (effectiveDuration <= 10) interval = 1;
    else if (effectiveDuration <= 60) interval = 2;
    else if (effectiveDuration <= 300) interval = 5;
    else interval = 10;
  }

  const maxFrames = Math.min(options.maxFrames || 30, 60);

  const timestamps = [];
  for (
    let t = startSec;
    t < endSec && timestamps.length < maxFrames;
    t += interval
  ) {
    timestamps.push(Math.round(t * 100) / 100);
  }

  // Include the last moment if we haven't reached it
  if (
    timestamps.length < maxFrames &&
    timestamps.length > 0 &&
    timestamps[timestamps.length - 1] < endSec - 0.5
  ) {
    timestamps.push(Math.round((endSec - 0.1) * 100) / 100);
  }

  return {
    strategy: options.sampleEverySec ? "fixed-interval" : "auto-interval",
    interval,
    startSec,
    endSec,
    frameCount: timestamps.length,
    timestamps,
  };
}

function createTempDir() {
  const id = `gsh-video-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const dir = path.join(os.tmpdir(), id);
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

function cleanupTempDir(dir) {
  try {
    fs.rmSync(dir, { recursive: true, force: true });
  } catch {
    // Best-effort cleanup
  }
}

async function extractFrames(videoPath, timestamps, tempDir) {
  const frames = [];

  for (const ts of timestamps) {
    const filename = `frame_${ts.toFixed(2).replace(".", "s")}.jpg`;
    const outputPath = path.join(tempDir, filename);

    await execPromise("ffmpeg", [
      "-ss",
      String(ts),
      "-i",
      videoPath,
      "-frames:v",
      "1",
      "-q:v",
      "2",
      "-y",
      outputPath,
    ]);

    if (fs.existsSync(outputPath)) {
      frames.push({ timestamp: ts, path: outputPath, filename });
    }
  }

  return frames;
}

module.exports = {
  ensureDependencies,
  getVideoMetadata,
  computeSamplingPlan,
  createTempDir,
  cleanupTempDir,
  extractFrames,
  checkDependency,
};
