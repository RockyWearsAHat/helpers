// vision-tool/lib/video-asr.js
//
// Lightweight ASR (Automatic Speech Recognition) adapter.
// Extracts audio from a video file using ffmpeg, runs a locally-installed
// speech-to-text tool, and parses the output into transcript segments
// compatible with the video analysis pipeline.
//
// Supported backends (checked in priority order):
//   1. whisper  (OpenAI whisper CLI — `pip install openai-whisper`)
//   2. mlx_whisper (Apple Silicon optimized — `pip install mlx-whisper`)
//   3. whisper-cpp (C++ port — `brew install whisper-cpp`)
//
// If no backend is found, returns a clear error with install instructions.

const { execFile } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

function execPromise(cmd, args, options = {}) {
  return new Promise((resolve, reject) => {
    execFile(
      cmd,
      args,
      { timeout: options.timeout || 600000, ...options },
      (err, stdout, stderr) => {
        if (err) reject(new Error((stderr || err.message || "").trim()));
        else
          resolve({
            stdout: (stdout || "").trim(),
            stderr: (stderr || "").trim(),
          });
      },
    );
  });
}

function checkCommand(name) {
  return new Promise((resolve) => {
    execFile("which", [name], (err, stdout) => {
      resolve(err ? null : (stdout || "").trim());
    });
  });
}

function checkPythonModule(moduleName) {
  return new Promise((resolve) => {
    execFile(
      "python3",
      ["-c", `import ${moduleName}; print("ok")`],
      { timeout: 10000 },
      (err) => resolve(!err),
    );
  });
}

const BACKENDS = [
  {
    name: "whisper",
    check: () => checkCommand("whisper"),
    install: "pip install openai-whisper",
  },
  {
    name: "mlx_whisper",
    check: async () => {
      const cmd = await checkCommand("mlx_whisper");
      if (cmd) return cmd;
      const hasMod = await checkPythonModule("mlx_whisper");
      return hasMod ? "python3 -m mlx_whisper" : null;
    },
    install: "pip install mlx-whisper",
  },
  {
    name: "whisper-cpp",
    check: () => checkCommand("whisper-cpp"),
    install: "brew install whisper-cpp",
  },
];

async function detectBackend() {
  for (const backend of BACKENDS) {
    const result = await backend.check();
    if (result) {
      return { name: backend.name, path: result };
    }
  }
  return null;
}

async function extractAudio(videoPath, tempDir) {
  const audioPath = path.join(tempDir, "audio.wav");

  await execPromise("ffmpeg", [
    "-i",
    videoPath,
    "-vn",
    "-acodec",
    "pcm_s16le",
    "-ar",
    "16000",
    "-ac",
    "1",
    "-y",
    audioPath,
  ]);

  if (!fs.existsSync(audioPath)) {
    throw new Error("ffmpeg did not produce an audio file.");
  }

  return audioPath;
}

function parseWhisperJson(jsonPath) {
  const data = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
  const segments = (data.segments || []).map((seg) => ({
    start: seg.start,
    end: seg.end,
    text: (seg.text || "").trim(),
  }));
  return segments;
}

function parseWhisperSrt(srtPath) {
  const raw = fs.readFileSync(srtPath, "utf8");
  const blocks = raw.split(/\n\n+/).filter((b) => b.trim());
  const segments = [];

  for (const block of blocks) {
    const lines = block.trim().split("\n");
    if (lines.length < 3) continue;

    const timeMatch = lines[1].match(
      /(\d{2}):(\d{2}):(\d{2})[,.](\d{3})\s*-->\s*(\d{2}):(\d{2}):(\d{2})[,.](\d{3})/,
    );
    if (!timeMatch) continue;

    const start =
      parseInt(timeMatch[1]) * 3600 +
      parseInt(timeMatch[2]) * 60 +
      parseInt(timeMatch[3]) +
      parseInt(timeMatch[4]) / 1000;
    const end =
      parseInt(timeMatch[5]) * 3600 +
      parseInt(timeMatch[6]) * 60 +
      parseInt(timeMatch[7]) +
      parseInt(timeMatch[8]) / 1000;
    const text = lines.slice(2).join(" ").trim();

    if (text) {
      segments.push({ start, end, text });
    }
  }

  return segments;
}

function parseWhisperVtt(vttPath) {
  const raw = fs.readFileSync(vttPath, "utf8");
  const blocks = raw.split(/\n\n+/).filter((b) => b.trim());
  const segments = [];

  for (const block of blocks) {
    const lines = block.trim().split("\n");
    const timeLine = lines.find((l) => l.includes("-->"));
    if (!timeLine) continue;

    const timeMatch = timeLine.match(
      /(\d{2}):(\d{2}):(\d{2})[.](\d{3})\s*-->\s*(\d{2}):(\d{2}):(\d{2})[.](\d{3})/,
    );
    if (!timeMatch) continue;

    const start =
      parseInt(timeMatch[1]) * 3600 +
      parseInt(timeMatch[2]) * 60 +
      parseInt(timeMatch[3]) +
      parseInt(timeMatch[4]) / 1000;
    const end =
      parseInt(timeMatch[5]) * 3600 +
      parseInt(timeMatch[6]) * 60 +
      parseInt(timeMatch[7]) +
      parseInt(timeMatch[8]) / 1000;
    const idx = lines.indexOf(timeLine);
    const text = lines
      .slice(idx + 1)
      .join(" ")
      .trim();

    if (text) {
      segments.push({ start, end, text });
    }
  }

  return segments;
}

async function runWhisper(audioPath, tempDir, model) {
  const outputDir = tempDir;
  const whisperModel = model || "base";

  await execPromise(
    "whisper",
    [
      audioPath,
      "--model",
      whisperModel,
      "--output_format",
      "json",
      "--output_dir",
      outputDir,
    ],
    { timeout: 600000 },
  );

  const jsonPath = path.join(outputDir, "audio.json");
  if (fs.existsSync(jsonPath)) {
    return parseWhisperJson(jsonPath);
  }

  const srtPath = path.join(outputDir, "audio.srt");
  if (fs.existsSync(srtPath)) {
    return parseWhisperSrt(srtPath);
  }

  throw new Error("Whisper did not produce expected output files.");
}

async function runMlxWhisper(audioPath, tempDir, cmdPath, model) {
  const outputDir = tempDir;
  const whisperModel = model || "mlx-community/whisper-base-mlx";

  const args = [audioPath, "--model", whisperModel, "--output-dir", outputDir];

  if (cmdPath.startsWith("python3")) {
    await execPromise("python3", ["-m", "mlx_whisper", ...args], {
      timeout: 600000,
    });
  } else {
    await execPromise(cmdPath, args, { timeout: 600000 });
  }

  const jsonPath = path.join(outputDir, "audio.json");
  if (fs.existsSync(jsonPath)) {
    return parseWhisperJson(jsonPath);
  }

  for (const ext of [".srt", ".vtt"]) {
    const outPath = path.join(outputDir, `audio${ext}`);
    if (fs.existsSync(outPath)) {
      return ext === ".srt"
        ? parseWhisperSrt(outPath)
        : parseWhisperVtt(outPath);
    }
  }

  throw new Error("mlx_whisper did not produce expected output files.");
}

async function runWhisperCpp(audioPath, tempDir) {
  const { stdout } = await execPromise(
    "whisper-cpp",
    [
      "-f",
      audioPath,
      "--output-srt",
      "--output-file",
      path.join(tempDir, "audio"),
    ],
    { timeout: 600000 },
  );

  const srtPath = path.join(tempDir, "audio.srt");
  if (fs.existsSync(srtPath)) {
    return parseWhisperSrt(srtPath);
  }

  if (stdout) {
    const lines = stdout.split("\n").filter((l) => l.trim());
    const segments = [];
    for (const line of lines) {
      const match = line.match(
        /\[(\d{2}:\d{2}:\d{2}\.\d{3})\s*-->\s*(\d{2}:\d{2}:\d{2}\.\d{3})\]\s*(.*)/,
      );
      if (match) {
        const parseTs = (ts) => {
          const [h, m, rest] = ts.split(":");
          const [s, ms] = rest.split(".");
          return (
            parseInt(h) * 3600 +
            parseInt(m) * 60 +
            parseInt(s) +
            parseInt(ms) / 1000
          );
        };
        segments.push({
          start: parseTs(match[1]),
          end: parseTs(match[2]),
          text: match[3].trim(),
        });
      }
    }
    if (segments.length) return segments;
  }

  throw new Error("whisper-cpp did not produce parseable output.");
}

async function transcribeVideo(videoPath, options = {}) {
  const backend = await detectBackend();
  if (!backend) {
    const installs = BACKENDS.map((b) => `  - ${b.name}: ${b.install}`).join(
      "\n",
    );
    throw new Error(
      `No ASR backend found. Install one of:\n${installs}`,
    );
  }

  const tempDir =
    options.tempDir ||
    path.join(
      os.tmpdir(),
      `gsh-asr-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    );
  fs.mkdirSync(tempDir, { recursive: true });

  try {
    const audioPath = await extractAudio(videoPath, tempDir);

    let segments;
    switch (backend.name) {
      case "whisper":
        segments = await runWhisper(audioPath, tempDir, options.whisperModel);
        break;
      case "mlx_whisper":
        segments = await runMlxWhisper(
          audioPath,
          tempDir,
          backend.path,
          options.whisperModel,
        );
        break;
      case "whisper-cpp":
        segments = await runWhisperCpp(audioPath, tempDir);
        break;
      default:
        throw new Error(`Unknown ASR backend: ${backend.name}`);
    }

    return {
      backend: backend.name,
      segmentCount: segments.length,
      segments,
    };
  } finally {
    if (!options.keepTempDir) {
      try {
        fs.rmSync(tempDir, { recursive: true, force: true });
      } catch {
        // Best-effort cleanup
      }
    }
  }
}

module.exports = {
  transcribeVideo,
  detectBackend,
  extractAudio,
  parseWhisperJson,
  parseWhisperSrt,
  parseWhisperVtt,
};
