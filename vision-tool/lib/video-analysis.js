// vision-tool/lib/video-analysis.js
//
// Orchestrates the video analysis pipeline: extract frames, analyze via
// vision model in batches, merge with transcript, produce structured output.
// No vscode dependencies — receives analyzeImagesFn as a callback.

const {
  ensureDependencies,
  getVideoMetadata,
  computeSamplingPlan,
  computeSmartSamplingPlan,
  createTempDir,
  cleanupTempDir,
  extractFrames,
  checkDependency,
} = require("./video-frames");
const { buildReport, buildTimeline } = require("./video-report");
const { transcribeVideo } = require("./video-asr");
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

function buildFramePrompt(frames, batchIndex, totalBatches, goal, transcript) {
  const lines = [
    "You are analyzing video frames extracted at specific timestamps.",
    "Your job is to reconstruct what is ACTUALLY HAPPENING in the video —",
    "not just what objects exist, but the story, tone, and intent.",
    "",
    `Batch ${batchIndex + 1} of ${totalBatches}.`,
    `Analysis goal: ${goal}`,
    "",
    "For EACH frame, describe:",
    "- **Who** is on screen: facial expressions, body language, gestures, costumes/outfit",
    "- **What** they are doing: actions, reactions, physical comedy, dramatic pauses",
    "- **Scene type**: is this talking head, skit/sketch, montage, cutaway, B-roll, screen recording?",
    "- **Editing/visual style**: jump cuts, zoom-ins, split screen, color grading shifts",
    "- **On-screen text & graphics**: titles, captions, memes, overlays, lower thirds — extract verbatim",
    "- **Props and visual gags**: anything deliberately placed for humor, irony, or emphasis",
    "- **Tone/energy**: comedic timing, sarcasm cues, deadpan delivery, exaggerated reactions",
    "- **What changed** from the previous frame: new scene? same scene with movement? cut to different angle?",
    "",
  ];

  // Include transcript context so the vision model knows what's being said
  if (transcript && transcript.type === "segmented" && transcript.segments) {
    const batchStart = frames[0].timestamp;
    const batchEnd = frames[frames.length - 1].timestamp;
    const relevantSegs = transcript.segments.filter((s) => {
      const segStart = s.start || 0;
      const segEnd = s.end || segStart + 2;
      return segEnd >= batchStart - 1 && segStart <= batchEnd + 1;
    });
    if (relevantSegs.length > 0) {
      lines.push("**Audio transcript during these frames:**");
      for (const seg of relevantSegs) {
        lines.push(`  [${(seg.start || 0).toFixed(1)}s] "${seg.text}"`);
      }
      lines.push("");
      lines.push(
        "Use the transcript to understand context: what the speaker is reacting to,",
        "what the visual gags are referencing, and how speech and visuals work together.",
      );
      lines.push("");
    }
  }

  lines.push("Frame timestamps in this batch:");

  for (const frame of frames) {
    lines.push(`  - ${frame.filename}: ${frame.timestamp.toFixed(2)}s`);
  }

  lines.push("");
  lines.push(
    "Respond with a structured description for EACH frame, clearly labeled by timestamp.",
  );
  lines.push(
    "Focus on STORYTELLING: what is the video communicating at this moment?",
    "Connect what you see to what is being said. Note visual punchlines, irony, and editing choices.",
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

function buildGlobalSummary(batchResults, transcript) {
  const parts = [];

  for (const b of batchResults) {
    const startTs = b.frames[0].timestamp.toFixed(1);
    const endTs = b.frames[b.frames.length - 1].timestamp.toFixed(1);
    // Take the first meaningful lines (up to 500 chars) from each batch
    const condensed = b.analysis
      .split("\n")
      .filter((l) => l.trim())
      .slice(0, 5)
      .join(" ")
      .slice(0, 500);
    parts.push(`**[${startTs}s–${endTs}s]** ${condensed}`);
  }

  // Append a high-level transcript summary if available
  if (
    transcript &&
    transcript.type === "segmented" &&
    transcript.segments &&
    transcript.segments.length > 0
  ) {
    const totalSegs = transcript.segments.length;
    const firstText = transcript.segments
      .slice(0, 3)
      .map((s) => s.text)
      .join(" ");
    const lastText = transcript.segments
      .slice(-2)
      .map((s) => s.text)
      .join(" ");
    parts.push("");
    parts.push(
      `**Transcript overview** (${totalSegs} segments): Opens with "${firstText.slice(0, 200)}" … closes with "${lastText.slice(0, 200)}"`,
    );
  }

  return parts.join("\n\n");
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
        "-o",
        outputTemplate,
        "--no-playlist",
        url,
      ],
      { timeout: 300000, cwd: tempDir },
      (err, stdout, stderr) => {
        if (err) {
          reject(new Error(`yt-dlp failed: ${(stderr || err.message).trim()}`));
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

  return {
    videoPath: path.join(tempDir, videoFile),
    tempDownloadDir: tempDir,
    sourceUrl: url,
  };
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

  // Auto-transcribe via local ASR (always available — uses bundled JS whisper)
  const autoTranscribe =
    (input.autoTranscribe ?? input.auto_transcribe) !== false;
  let asrInfo = null;

  if (autoTranscribe) {
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
      asrInfo = { error: asrErr.message };
    }
  }

  const plan = await computeSmartSamplingPlan(videoPath, metadata.durationSec, {
    startSec: input.startSec || input.start_sec,
    endSec: input.endSec || input.end_sec,
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
      const prompt = buildFramePrompt(
        batch,
        i,
        batches.length,
        goal,
        transcript,
      );
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
    const globalSummary = buildGlobalSummary(batchResults, transcript);
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

// ---------------------------------------------------------------------------
// Standalone transcription — no vision model needed
// ---------------------------------------------------------------------------

async function transcribeOnly(input) {
  await ensureDependencies();

  let videoPath;
  let tempDownloadDir = null;

  const rawPath = input.videoPath || input.video_path || "";

  if (rawPath.startsWith("http://") || rawPath.startsWith("https://")) {
    const download = await downloadVideo(rawPath);
    videoPath = download.videoPath;
    tempDownloadDir = download.tempDownloadDir;
  } else {
    videoPath = validateInput(input);
  }

  try {
    const metadata = await getVideoMetadata(videoPath);
    const asrResult = await transcribeVideo(videoPath, {
      whisperModel: input.whisperModel || input.whisper_model,
    });

    const fullText = asrResult.segments.map((s) => s.text).join(" ");

    return {
      videoPath: rawPath,
      durationSec: metadata.durationSec,
      backend: asrResult.backend,
      segmentCount: asrResult.segmentCount,
      segments: asrResult.segments,
      fullText,
    };
  } finally {
    if (tempDownloadDir) {
      cleanupTempDir(tempDownloadDir);
    }
  }
}

module.exports = {
  analyzeVideo,
  transcribeOnly,
  downloadVideo,
  validateInput,
};
