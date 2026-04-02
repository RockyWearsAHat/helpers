# GSH Vision Tool

This workspace-local VS Code extension contributes image analysis and video analysis tools:

**Image analysis:**

- `gsh-analyze-images` / `gsh_analyze_images`

**Video analysis:**

- `gsh-analyze-video` / `gsh_analyze_video`

It also contributes one chat participant:

- `@gsh-vision` (commands: `/analyze`, `/analyze-video`)

The tool accepts 1–10 image paths and a freeform goal. It reads the image bytes from disk, attaches them all as real image data to a vision-capable Copilot model (Claude Sonnet 4.6 by default), and returns the model's analysis. If the host already exposes a built-in image tool such as `view_image`, prefer that for simple single-image inspection. Keep this tool for multi-image comparisons, goal-driven evaluation, and environments without a built-in image tool.

It also provides a manual command:

- `GSH: Inspect Screenshot With Copilot Vision` (supports multi-select)

Environment variables:

- `GSH_VISION_MODEL_IDS`: comma-separated model id/name preferences used when choosing a model. Default: `claude-sonnet-4.6`.
- `GSH_VISION_ALLOW_UNDECLARED_IMAGE_MODEL`: optional override for undeclared-image fallback. The extension now defaults to allowing the preferred-model fallback because current Copilot model metadata may report `imageInput: false` even when image requests still work. Set this to `0` or `false` to disable that fallback explicitly.

## Why this exists

The `.agent.md` and skill files cannot directly force binary screenshots to be attached as image context during execution. That requires an extension tool or participant using the VS Code Language Model API.

This extension is the bridge between:

- screenshots captured during visual development
- Copilot models with `imageInput` capability
- agent workflows that need real image analysis

## Expected usage

Once the extension is installed in the normal VS Code profile, Copilot Chat can use `@gsh-vision` for direct image analysis, and the extension exposes the same image-aware analysis through the language model tools above. In environments where the host runtime already exposes a built-in image-view tool, use that first for straightforward single-image viewing and keep `analyze_images` as the richer fallback.

## Activation

1. Build and install the extension locally (see `scripts/build-vsix.sh`).
2. Reload the current VS Code window once so the newly installed extension is activated in the running editor.
3. After that one-time reload, the extension activates at editor startup and the tool should be available to new Copilot sessions without a manual warm-up command.
4. Use `@gsh-vision /analyze /path/to/image.png :: What to evaluate?` in Copilot Chat for manual analysis.
5. For multi-image: `@gsh-vision /analyze /path/to/img1.png :: /path/to/img2.png :: Compare these designs`

The separate Extension Development Host path in `.vscode/launch.json` is still available for extension development, but it is no longer required for normal use.

Use `make reinstall-vision-tool` after editing the extension code so the current profile gets the rebuilt VSIX.

## Architecture: two layers, one pipeline

There are **two distinct config files** in `.vscode/` that are easy to confuse. They are not redundant — they serve different layers of the same pipeline.

```
Copilot / Agent
     │
     │  calls tool via MCP protocol
     ▼
.vscode/mcp.json
  → launches: node ./vision-tool/mcp-server.js
                │
                │  reads at runtime (every call)
                ▼
        .vscode/gsh-vision-ipc.json
          { "socketPath": "/tmp/gsh-vision-*.sock" }
                │
                │  Unix domain socket request
                ▼
        VS Code extension (extension.js, running inside the editor)
          → selects a vision-capable Copilot model (claude-sonnet-4.6 by default)
          → reads all image bytes from disk (up to 10)
          → attaches images to the model via LanguageModelDataPart.image(...)
          → returns the model's response back up the chain
```

### What each file does

| File                          | Role                                                                                | Written by                       |
| ----------------------------- | ----------------------------------------------------------------------------------- | -------------------------------- |
| `.vscode/mcp.json`            | Registers the MCP server so Copilot/agents can call the vision tool                 | You (static config)              |
| `.vscode/gsh-vision-ipc.json` | Runtime service-discovery: holds the Unix socket path to the live extension backend | The VS Code extension at startup |
| `vision-tool/mcp-server.js`   | MCP protocol layer — receives tool calls, forwards them over the socket             | This repo                        |
| `vision-tool/extension.js`    | Vision backend — runs inside the editor, owns the Copilot model API access          | This repo                        |

### Why the split?

Copilot agents running in the chat panel cannot directly call VS Code's `LanguageModelDataPart.image(...)` API — only a running VS Code extension can. The MCP server provides a stable JSON-RPC endpoint that any agent or model can call, and it proxies the request to the extension that actually owns the image-attachment capability.

### Prerequisites for tool calls to succeed

All three of the following must be true at call time:

1. **Extension is installed and active** — build and install the extension, then reload the window. The extension writes `gsh-vision-ipc.json` when it activates.
2. **`gsh-vision-ipc.json` exists and has a live socket path** — if this file is missing or stale (e.g. after a window reload without re-activation), the MCP server will fail with `IPC metadata missing socketPath` or a connection error. The fix is to reload the VS Code window so the extension re-activates and rewrites the file.
3. **A vision-capable Copilot model is available** — `claude-sonnet-4.6` by default. Configure via `GSH_VISION_MODEL_IDS` if needed.

### How agents should call the tool

Agents and models interact with this pipeline through one tool:

**`mcp_gsh-vis_analyze_images`** — analyze 1–10 images with a freeform goal

```
image_paths: ["/absolute/path/to/screenshot1.png", "/absolute/path/to/screenshot2.png"]
goal:        "Compare these two designs for layout and color consistency"
```

The `mcp_` prefix and the `gsh-vis` server name come from how VS Code exposes MCP tools to the language model. The underlying tool name registered in `mcp-server.js` is `analyze_images`.

### Common failure modes

| Symptom                                       | Cause                                                | Fix                                                       |
| --------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------- |
| `IPC metadata missing socketPath`             | `gsh-vision-ipc.json` is missing                     | Reload VS Code window                                     |
| `Extension IPC timeout` or `ENOENT` on socket | Extension not active, socket stale                   | Reload VS Code window                                     |
| `No chat model with image input capability`   | No vision model available in current Copilot session | Check `GSH_VISION_MODEL_IDS`; ensure Copilot is signed in |
| Tool not visible to agent                     | `mcp.json` not picked up                             | Restart VS Code; confirm `.vscode/mcp.json` exists        |

## Video Analysis

The `analyze_video` tool extracts frames from a video at regular intervals, analyzes each batch through the existing vision model pipeline, and automatically generates a transcript via local ASR (whisper/mlx_whisper/whisper-cpp) or yt-dlp subtitle download for URLs. Returns a structured timeline merging visual and audio evidence.

### Dependencies

- **ffmpeg** and **ffprobe** — required for frame extraction and metadata inspection. Install with `brew install ffmpeg`.
- **yt-dlp** — optional, needed for URL ingestion (YouTube, Shorts). Also auto-downloads subtitles when available. Install with `brew install yt-dlp`.
- **whisper** / **mlx_whisper** / **whisper-cpp** — optional, for local ASR transcription. Install with `pip install openai-whisper` or `pip install mlx_whisper`.

### MCP tool input

```json
{
  "video_path": "/absolute/path/to/video.mp4",
  "goal": "Describe the visual content and any on-screen text over the course of this video.",
  "start_sec": 0,
  "end_sec": 60,
  "sample_every_sec": 2,
  "max_frames": 30,
  "include_report": true,
  "include_timeline": true,
  "auto_transcribe": true,
  "whisper_model": "base"
}
```

### VS Code language model tool input (camelCase)

```json
{
  "videoPath": "/absolute/path/to/video.mp4",
  "goal": "Describe the visual content over the course of this video.",
  "sampleEverySec": 2,
  "maxFrames": 30,
  "includeReport": true,
  "includeTimeline": true,
  "autoTranscribe": true,
  "whisperModel": "base"
}
```

### Chat participant usage

```
@gsh-vision /analyze-video /path/to/video.mp4 :: What happens in this video?
```

### Output shape

```json
{
  "metadata": {
    "sourceType": "local-video",
    "videoPath": "/path/to/video.mp4",
    "durationSec": 120.5,
    "fps": 30,
    "width": 1920,
    "height": 1080
  },
  "sampling": {
    "strategy": "auto-interval",
    "interval": 5,
    "framesAnalyzed": 24,
    "batchCount": 3
  },
  "transcriptSource": "whisper",
  "asr": {
    "backend": "whisper",
    "segmentCount": 42
  },
  "segments": [
    {
      "start": 0,
      "end": 1,
      "transcript": "Welcome to the demo.",
      "visual": "A person standing at a podium in a conference room...",
      "ocrLikeText": "TechConf 2026 — Opening Keynote",
      "confidence": 0.9
    }
  ],
  "globalSummary": "...",
  "report": "# Video Analysis Report\n..."
}
```

### Transcript acquisition

Transcripts are acquired automatically — there are no user-facing transcript parameters.

1. **URL videos (yt-dlp)**: Subtitles are downloaded alongside the video (`--write-subs --write-auto-subs`). If available, parsed into timestamped segments. This is the fastest path and produces high-quality captions for YouTube/Shorts content.
2. **Local ASR (whisper/mlx_whisper/whisper-cpp)**: If no subtitles were obtained (local files, or URLs without captions), the audio is extracted and transcribed locally. Backend is auto-detected at runtime in priority order: whisper → mlx_whisper → whisper-cpp.
3. **Visual-only**: If ASR is disabled (`auto_transcribe: false`) or no backend is available, the pipeline produces visual-only analysis without transcript alignment.

### Sampling behavior

| Video duration | Default interval | Expected frames |
| -------------- | ---------------- | --------------- |
| ≤ 10s          | 1s               | up to 10        |
| 10–60s         | 2s               | up to 30        |
| 60–300s        | 5s               | up to 30        |
| > 300s         | 10s              | up to 30        |

Hard cap: 60 frames maximum. Frames are sent to the vision model in batches of 8.
