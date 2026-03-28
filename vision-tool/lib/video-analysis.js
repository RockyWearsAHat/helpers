// vision-tool/lib/video-analysis.js
//
// Orchestrates the video analysis pipeline: extract frames, analyze via
// vision model in batches, merge with transcript, produce structured output.
// No vscode dependencies — receives analyzeImagesFn as a callback.

const {
  ensureDependencies,
  getVideoMetadata,
  computeSamplingPlan,
  createTempDir,
  cleanupTempDir,
  extractFrames,
  checkDependency,
} = require("./video-frames");
const { buildReport, buildTimeline } = require("./video-report");
const { transcribeVideo, detectBackend } = require("./video-asr");
const fs = require("fs");
const os = require("os");
const path = require("path");

const BATCH_SIZE = 8;

const VIDEO_EXTENSIONS = [
  ".mp4",
  ".mov",
  ".avi",
  ".mkv",
  ".webm",
  ".m4v",
  ".flv",
  ".wmv",
  ".ts",
  ".mts",
];

function validateInput(input) {
  const videoPath = input.videoPath || input.video_path;
  if (!videoPath) throw new Error("videoPath is required.");

  const resolved = path.resolve(videoPath);
  if (!fs.existsSync(resolved)) {
    throw new Error(`Video file not found: ${resolved}`);
  }

  const ext = path.extname(resolved).toLowerCase();
  if (!VIDEO_EXTENSIONS.includes(ext)) {
    throw new Error(
      `Unsupported video format: ${ext}. Supported: ${VIDEO_EXTENSIONS.join(", ")}`,
    );
  }

  return resolved;
}

function buildFramePrompt(frames, batchIndex, totalBatches, goal) {
  const lines = [
    "You are analyzing video frames extracted at specific timestamps.",
    "Describe what you observe with evidence-grounded, timestamp-oriented detail.",
    "",
    `Batch ${batchIndex + 1} of ${totalBatches}.`,
    `Analysis goal: ${goal}`,
    "",
    "For EACH frame, describe:",
    "- People present and their actions/expressions",
    "- Objects and their arrangement",
    "- Scene/setting and lighting",
    "- On-screen UI elements, text, overlays, or captions",
    "- Camera angle and any transitions from previous frame",
    "- Any text visible in the frame (extract it verbatim)",
    "- What changed compared to adjacent frames",
    "",
    "Frame timestamps in this batch:",
  ];

  for (const frame of frames) {
    lines.push(`  - ${frame.filename}: ${frame.timestamp.toFixed(2)}s`);
  }

  lines.push("");
  lines.push(
    "Respond with a structured description for EACH frame, clearly labeled by timestamp.",
  );
  lines.push(
    "Be specific and factual. Do not speculate about what might be happening off-screen.",
  );

  return lines.join("\n");
}

function batchFrames(frames) {
  const batches = [];
  for (let i = 0; i < frames.length; i += BATCH_SIZE) {
    batches.push(frames.slice(i, i + BATCH_SIZE));
  }
  return batches;
}

function buildGlobalSummary(batchResults) {
  const allAnalysis = batchResults.map((b) => b.analysis).join("\n\n");
  const lineCount = allAnalysis.split("\n").length;

  if (lineCount <= 30) return allAnalysis;

  return batchResults
    .map((b, i) => {
      const firstLines = b.analysis
        .split("\n")
        .filter((l) => l.trim())
        .slice(0, 3)
        .join(" ");
      return `[Batch ${i + 1}: ${b.frames[0].timestamp.toFixed(1)}s–${b.frames[b.frames.length - 1].timestamp.toFixed(1)}s] ${firstLines.slice(0, 300)}`;
    })
    .join("\n\n");
}

// ---------------------------------------------------------------------------
// Phase 2: URL ingestion — yt-dlp preferred, direct HTTP fallback
// ---------------------------------------------------------------------------

const STREAMING_PATTERNS = [
  /youtube\.com/i,
  /youtu\.be/i,
  /vimeo\.com/i,
  /dailymotion\.com/i,
  /twitch\.tv/i,
  /tiktok\.com/i,
  /facebook\.com.*video/i,
  /instagram\.com/i,
  /x\.com/i,
  /twitter\.com/i,
  /reddit\.com/i,
];

function isStreamingSite(url) {
  return STREAMING_PATTERNS.some((p) => p.test(url));
}

async function downloadVideo(url) {
  // Prefer yt-dlp when available (handles streaming sites + subtitle download)
  const ytdlp = await checkDependency("yt-dlp");
  if (ytdlp) {
    return downloadWithYtdlp(url);
  }

  // No yt-dlp — streaming sites require it, direct URLs can use HTTP fetch
  if (isStreamingSite(url)) {
    throw new Error(
      `Streaming site detected (${new URL(url).hostname}) but yt-dlp is not installed. ` +
        "Install with: brew install yt-dlp",
    );
  }

  return directHttpDownload(url);
}

async function directHttpDownload(url) {
  const tempDir = path.join(os.tmpdir(), `gsh-video-dl-${Date.now()}`);
  fs.mkdirSync(tempDir, { recursive: true });

  const urlPath = new URL(url).pathname;
  let ext = path.extname(urlPath).toLowerCase();
  if (!VIDEO_EXTENSIONS.includes(ext)) ext = ".mp4";
  const videoFile = path.join(tempDir, `video${ext}`);

  const response = await fetch(url, { redirect: "follow" });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} downloading video from ${url}`);
  }
  const buffer = Buffer.from(await response.arrayBuffer());
  fs.writeFileSync(videoFile, buffer);

  if (fs.statSync(videoFile).size === 0) {
    throw new Error("Direct download produced an empty file.");
  }

  return {
    videoPath: videoFile,
    tempDownloadDir: tempDir,
    sourceUrl: url,
    subtitleSegments: null,
  };
}

async function downloadWithYtdlp(url) {
  const tempDir = path.join(os.tmpdir(), `gsh-video-dl-${Date.now()}`);
  fs.mkdirSync(tempDir, { recursive: true });
  const outputTemplate = path.join(tempDir, "video.%(ext)s");

  const { execFile: execFileCb } = require("child_process");
  await new Promise((resolve, reject) => {
    execFileCb(
      "yt-dlp",
      [
        "-f",
        "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best",
        "--merge-output-format",
        "mp4",
        "--write-subs",
        "--write-auto-subs",
        "--sub-langs",
        "en.*,en",
        "--sub-format",
        "vtt/srt/best",
        "--convert-subs",
        "vtt",
        "--ignore-errors",
        "-o",
        outputTemplate,
        "--no-playlist",
        url,
      ],
      { timeout: 300000, cwd: tempDir },
      (err, stdout, stderr) => {
        if (err) {
          // yt-dlp may exit non-zero for subtitle-only failures while
          // the video was downloaded successfully. Check if a video file
          // exists before treating the error as fatal.
          const outputFiles = fs
            .readdirSync(tempDir)
            .filter((f) => !f.startsWith("."));
          const hasVideo = outputFiles.some((f) =>
            VIDEO_EXTENSIONS.includes(path.extname(f).toLowerCase()),
          );
          if (hasVideo) {
            resolve(stdout || "");
          } else {
            reject(
              new Error(`yt-dlp failed: ${(stderr || err.message).trim()}`),
            );
          }
        } else {
          resolve(stdout);
        }
      },
    );
  });

  const files = fs.readdirSync(tempDir).filter((f) => !f.startsWith("."));
  const videoFile = files.find((f) =>
    VIDEO_EXTENSIONS.includes(path.extname(f).toLowerCase()),
  );
  if (!videoFile) throw new Error("yt-dlp did not produce an output file.");

  // Look for downloaded subtitle files (.vtt)
  const subtitleFile = files.find((f) => f.endsWith(".vtt"));
  let subtitleSegments = null;
  if (subtitleFile) {
    try {
      const vttContent = fs.readFileSync(
        path.join(tempDir, subtitleFile),
        "utf-8",
      );
      subtitleSegments = parseVttSubtitles(vttContent);
    } catch (_) {
      // Subtitle parsing failed — continue without
    }
  }

  return {
    videoPath: path.join(tempDir, videoFile),
    tempDownloadDir: tempDir,
    sourceUrl: url,
    subtitleSegments,
  };
}

/**
 * Parse a WebVTT file into transcript segments [{start, end, text}].
 */
function parseVttSubtitles(vttContent) {
  const segments = [];
  const lines = vttContent.split("\n");
  let i = 0;

  // Skip header
  while (i < lines.length && !lines[i].includes("-->")) i++;

  while (i < lines.length) {
    const line = lines[i].trim();
    const match = line.match(
      /(\d{2}):(\d{2}):(\d{2})\.(\d{3})\s*-->\s*(\d{2}):(\d{2}):(\d{2})\.(\d{3})/,
    );
    if (match) {
      const start =
        parseInt(match[1]) * 3600 +
        parseInt(match[2]) * 60 +
        parseInt(match[3]) +
        parseInt(match[4]) / 1000;
      const end =
        parseInt(match[5]) * 3600 +
        parseInt(match[6]) * 60 +
        parseInt(match[7]) +
        parseInt(match[8]) / 1000;
      i++;
      const textLines = [];
      while (
        i < lines.length &&
        lines[i].trim() !== "" &&
        !lines[i].includes("-->")
      ) {
        // Strip VTT formatting tags like <c>, </c>, <00:00:01.234>
        const cleaned = lines[i].trim().replace(/<[^>]+>/g, "");
        if (cleaned) textLines.push(cleaned);
        i++;
      }
      const text = textLines.join(" ").trim();
      if (text) {
        // Deduplicate consecutive identical segments (common in auto-subs)
        const prev = segments[segments.length - 1];
        if (!prev || prev.text !== text) {
          segments.push({ start, end, text });
        } else {
          prev.end = end; // Extend previous segment
        }
      }
    } else {
      i++;
    }
  }

  return segments.length > 0 ? segments : null;
}

// ---------------------------------------------------------------------------
// Main pipeline
// ---------------------------------------------------------------------------

async function analyzeVideo(input, analyzeImagesFn) {
  await ensureDependencies();

  let videoPath;
  let sourceType = "local-video";
  let tempDownloadDir = null;
  let sourceUrl = null;
  let transcript = { type: "none" };
  let transcriptSource = null;

  const rawPath = input.videoPath || input.video_path || "";

  if (rawPath.startsWith("http://") || rawPath.startsWith("https://")) {
    const download = await downloadVideo(rawPath);
    videoPath = download.videoPath;
    tempDownloadDir = download.tempDownloadDir;
    sourceType = "url-video";
    sourceUrl = download.sourceUrl;

    // Use yt-dlp subtitles if available
    if (download.subtitleSegments) {
      transcript = { type: "segmented", segments: download.subtitleSegments };
      transcriptSource = "yt-dlp-subtitles";
    }
  } else {
    videoPath = validateInput(input);
  }

  const goal =
    input.goal || "Describe what happens visually over time in this video.";
  const includeReport = input.includeReport !== false;
  const includeTimeline = input.includeTimeline !== false;
  const keepTempDir =
    input.keepTempDir === true || input.keep_temp_dir === true;

  const metadata = await getVideoMetadata(videoPath);

  // Auto-transcribe via ASR if no transcript yet and not explicitly disabled
  const autoTranscribe =
    (input.autoTranscribe ?? input.auto_transcribe) !== false;
  let asrInfo = null;

  if (transcript.type === "none" && autoTranscribe) {
    const backend = await detectBackend();
    if (backend) {
      try {
        const asrResult = await transcribeVideo(videoPath, {
          whisperModel: input.whisperModel || input.whisper_model,
          keepTempDir,
        });
        transcript = { type: "segmented", segments: asrResult.segments };
        transcriptSource = asrResult.backend;
        asrInfo = {
          backend: asrResult.backend,
          segmentCount: asrResult.segmentCount,
        };
      } catch (asrErr) {
        asrInfo = { backend: backend.name, error: asrErr.message };
      }
    }
  }

  const plan = computeSamplingPlan(metadata.durationSec, {
    startSec: input.startSec || input.start_sec,
    endSec: input.endSec || input.end_sec,
    sampleEverySec: input.sampleEverySec || input.sample_every_sec,
    maxFrames: input.maxFrames || input.max_frames,
  });

  const tempDir = createTempDir();
  let frames;

  try {
    frames = await extractFrames(videoPath, plan.timestamps, tempDir);
    if (!frames.length) {
      throw new Error("No frames could be extracted from the video.");
    }

    const batches = batchFrames(frames);
    const batchResults = [];

    for (let i = 0; i < batches.length; i++) {
      const batch = batches[i];
      const prompt = buildFramePrompt(batch, i, batches.length, goal);
      const imagePaths = batch.map((f) => f.path);

      const result = await analyzeImagesFn({
        imagePaths,
        goal: prompt,
        context:
          `Video analysis batch ${i + 1}/${batches.length}. ` +
          `Frames from ${batch[0].timestamp.toFixed(1)}s to ` +
          `${batch[batch.length - 1].timestamp.toFixed(1)}s.`,
      });

      batchResults.push({
        batchIndex: i,
        frames: batch.map((f) => ({
          timestamp: f.timestamp,
          filename: f.filename,
        })),
        analysis: result.response,
        model: result.model,
      });
    }

    const segments = buildTimeline(batchResults, transcript, metadata);
    const globalSummary = buildGlobalSummary(batchResults);
    const displayPath = sourceUrl || videoPath;

    const report = includeReport
      ? buildReport(
          displayPath,
          metadata,
          plan,
          segments,
          globalSummary,
          transcript,
        )
      : undefined;

    const output = {
      metadata: {
        sourceType,
        videoPath: displayPath,
        durationSec: metadata.durationSec,
        fps: metadata.fps,
        width: metadata.width,
        height: metadata.height,
      },
      sampling: {
        strategy: plan.strategy,
        interval: plan.interval,
        framesAnalyzed: frames.length,
        batchCount: batches.length,
        tempDir: keepTempDir ? tempDir : undefined,
      },
    };

    if (asrInfo) {
      output.asr = asrInfo;
    }

    if (transcriptSource) {
      output.transcriptSource = transcriptSource;
    }

    if (includeTimeline) {
      output.segments = segments;
    }

    output.globalSummary = globalSummary;

    if (report) {
      output.report = report;
    }

    return output;
  } finally {
    if (!keepTempDir) {
      cleanupTempDir(tempDir);
    }
    if (tempDownloadDir && !keepTempDir) {
      cleanupTempDir(tempDownloadDir);
    }
  }
}

module.exports = {
  analyzeVideo,
  downloadVideo,
  validateInput,
  parseVttSubtitles,
};
