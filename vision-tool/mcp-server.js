#!/usr/bin/env node

const fs = require("fs");
const net = require("net");
const path = require("path");
const readline = require("readline");

const MCP_VERSION = "2024-11-05";
const IPC_INFO_PATH =
  process.env.GSH_VISION_IPC_INFO_PATH ||
  path.join(require("os").homedir(), ".cache", "gsh", "vision-ipc.json");

function send(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

function sendError(id, code, message) {
  send({
    jsonrpc: "2.0",
    id,
    error: { code, message },
  });
}

function loadIpcInfo() {
  const raw = fs.readFileSync(IPC_INFO_PATH, "utf8");
  const parsed = JSON.parse(raw);
  if (!parsed.socketPath) {
    throw new Error("IPC metadata missing socketPath");
  }
  return parsed;
}

async function connectAndSend(request) {
  let lastError;
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      const ipcInfo = loadIpcInfo();
      return await new Promise((resolve, reject) => {
        const socket = net.createConnection(ipcInfo.socketPath, () => {
          socket.write(`${JSON.stringify(request)}\n`);
        });

        let buffer = "";

        socket.setEncoding("utf8");
        socket.on("data", (chunk) => {
          buffer += chunk;
          const lines = buffer.split("\n");
          buffer = lines.pop() || "";

          for (const line of lines) {
            if (!line.trim()) {
              continue;
            }

            try {
              const response = JSON.parse(line);
              socket.destroy();
              resolve(response);
              return;
            } catch {
              // Ignore malformed partial lines and wait for the next chunk.
            }
          }
        });

        socket.on("error", reject);
        socket.setTimeout(120000, () => {
          socket.destroy(new Error("Extension IPC timeout (120s)"));
        });
      });
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
  }

  throw lastError || new Error("Extension IPC not configured");
}

const VISION_TOOLS = [
  {
    name: "take_screenshot",
    description:
      "Capture a screenshot on macOS. Returns the absolute path to the saved PNG. Use this to get images for analyze_images.",
    inputSchema: {
      type: "object",
      properties: {
        output_path: {
          type: "string",
          description:
            "Optional absolute path for the output PNG. Defaults to a timestamped file in the system temp directory.",
        },
        mode: {
          type: "string",
          enum: ["fullscreen", "window", "region"],
          description:
            "Capture mode. 'fullscreen' (default): entire screen. 'window': a specific window by owner name. 'region': a rectangle defined by x, y, width, height.",
        },
        app_name: {
          type: "string",
          description:
            "Application name whose frontmost window to capture (used with mode 'window'). Example: 'AIO Server'.",
        },
        x: {
          type: "number",
          description: "X origin for region capture.",
        },
        y: {
          type: "number",
          description: "Y origin for region capture.",
        },
        width: {
          type: "number",
          description: "Width for region capture.",
        },
        height: {
          type: "number",
          description: "Height for region capture.",
        },
      },
      required: [],
    },
  },
  {
    name: "analyze_images",
    description:
      "Analyze up to 10 images with a vision model. Supports single inspection, comparisons, batch evaluation, and any custom analysis goal.",
    inputSchema: {
      type: "object",
      properties: {
        image_paths: {
          type: "array",
          items: { type: "string" },
          minItems: 1,
          maxItems: 10,
          description:
            "Absolute paths to image files (1–10). Order matters: reference images first when comparing.",
        },
        goal: {
          type: "string",
          description:
            "What to determine from the images. Be specific: inspect quality, compare differences, evaluate design, verify a fix, etc.",
        },
        context: {
          type: "string",
          description: "Optional extra context about the images.",
        },
        style_context: {
          type: "string",
          description:
            "Optional QSS/CSS stylesheet content for the page being viewed. Helps the model suggest concrete selector-level fixes.",
        },
      },
      required: ["image_paths", "goal"],
    },
  },
  {
    name: "analyze_video",
    description:
      "Analyze a video by extracting frames, running them through a vision model with transcript context, and producing a synthesized visual+audio timeline and report. The vision model sees both the frames AND what is being said, so it can detect visual gags, comedic timing, editing choices, and how speech relates to what's shown. Requires ffmpeg. Accepts local paths or URLs (YouTube etc via yt-dlp).",
    inputSchema: {
      type: "object",
      properties: {
        video_path: {
          type: "string",
          description:
            "Absolute path to a local video file, or a URL (YouTube/Shorts) if yt-dlp is installed.",
        },
        goal: {
          type: "string",
          description:
            "What to analyze or determine from the video. Be specific about what visual evidence to look for.",
        },
        start_sec: {
          type: "number",
          description:
            "Optional start time in seconds. Limits analysis to a time window.",
        },
        end_sec: {
          type: "number",
          description: "Optional end time in seconds.",
        },
        sample_every_sec: {
          type: "number",
          description:
            "Interval between frame samples in seconds. Auto-calculated if omitted.",
        },
        max_frames: {
          type: "number",
          description:
            "Maximum number of frames to extract and analyze. Default: 30, max: 60.",
        },
        include_report: {
          type: "boolean",
          description:
            "Include a human-readable report in the output. Default: true.",
        },
        include_timeline: {
          type: "boolean",
          description:
            "Include the structured timeline segments in the output. Default: true.",
        },
        auto_transcribe: {
          type: "boolean",
          description:
            "Automatically transcribe the video audio using local Whisper ASR. Default: true.",
        },
        whisper_model: {
          type: "string",
          description:
            "Whisper model name for ASR. Default: onnx-community/whisper-tiny.en. Larger models are more accurate but slower.",
        },
      },
      required: ["video_path", "goal"],
    },
  },
  {
    name: "transcribe_video",
    description:
      "Transcribe a video's audio to text using local Whisper ASR. No vision model needed — fast and lightweight. Returns timestamped segments and full text. Accepts local paths or URLs (YouTube etc via yt-dlp). Use this when you only need what was said, not what was shown.",
    inputSchema: {
      type: "object",
      properties: {
        video_path: {
          type: "string",
          description:
            "Absolute path to a local video file, or a URL (YouTube etc) if yt-dlp is installed.",
        },
        whisper_model: {
          type: "string",
          description:
            "Whisper model name. Default: onnx-community/whisper-tiny.en. Use whisper-small.en or whisper-medium.en for better accuracy on complex audio.",
        },
      },
      required: ["video_path"],
    },
  },
];

async function handleVisionToolCall(toolName, toolArguments) {
  if (toolName === "take_screenshot") {
    const response = await connectAndSend({
      method: toolName,
      arguments: toolArguments,
    });
    if (!response.ok) {
      throw new Error(response.error || "Extension IPC failed");
    }
    return [{ type: "text", text: response.result }];
  }

  if (toolName === "analyze_images") {
    const response = await connectAndSend({
      method: toolName,
      arguments: toolArguments,
    });
    if (!response.ok) {
      throw new Error(response.error || "Extension IPC failed");
    }
    return [
      { type: "text", text: `Model: ${response.model}\n\n${response.result}` },
    ];
  }

  if (toolName === "analyze_video") {
    const response = await connectAndSend({
      method: toolName,
      arguments: toolArguments,
    });
    if (!response.ok) {
      throw new Error(response.error || "Extension IPC failed");
    }
    return [{ type: "text", text: response.result }];
  }

  if (toolName === "transcribe_video") {
    // Transcription runs locally — no vision model needed, no IPC required
    const { transcribeOnly } = require("./lib/video-analysis");
    const result = await transcribeOnly(toolArguments);
    return [{ type: "text", text: JSON.stringify(result, null, 2) }];
  }

  return null;
}

async function handleRequest(request) {
  const { id, method } = request;

  if (method === "initialize") {
    send({
      jsonrpc: "2.0",
      id,
      result: {
        protocolVersion: MCP_VERSION,
        capabilities: { tools: {} },
        serverInfo: { name: "gsh-vision", version: "1.0.0" },
      },
    });
    return;
  }

  if (method === "notifications/initialized") {
    return;
  }

  if (method === "tools/list") {
    send({
      jsonrpc: "2.0",
      id,
      result: { tools: VISION_TOOLS },
    });
    return;
  }

  if (method === "tools/call") {
    const toolName = request.params?.name;
    const toolArguments = request.params?.arguments || {};

    try {
      const content = await handleVisionToolCall(toolName, toolArguments);
      if (content) {
        send({ jsonrpc: "2.0", id, result: { content } });
        return;
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      sendError(id, -32603, message);
      return;
    }

    sendError(id, -32601, `Unknown tool: ${toolName}`);
    return;
  }

  sendError(id, -32601, `Unknown method: ${method}`);
}

module.exports = { tools: VISION_TOOLS, handleToolCall: handleVisionToolCall };

if (require.main === module) {
  const lineReader = readline.createInterface({
    input: process.stdin,
    crlfDelay: Infinity,
  });

  lineReader.on("line", (line) => {
    if (!line.trim()) {
      return;
    }

    let request;
    try {
      request = JSON.parse(line);
    } catch {
      sendError(null, -32700, "Parse error");
      return;
    }
    handleRequest(request).catch((err) => {
      process.stderr.write(`[vision-tool] Unhandled error: ${err.message}\n`);
      if (request.id != null) {
        sendError(request.id, -32603, err.message);
      }
    });
  });
}
