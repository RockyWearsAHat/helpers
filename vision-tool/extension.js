const vscode = require("vscode");
const fs = require("fs");
const net = require("net");
const os = require("os");
const path = require("path");
const { execFile } = require("child_process");

function mimeTypeForFile(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  if (ext === ".png") {
    return "image/png";
  }
  if (ext === ".jpg" || ext === ".jpeg") {
    return "image/jpeg";
  }
  if (ext === ".webp") {
    return "image/webp";
  }
  if (ext === ".bmp") {
    return "image/bmp";
  }
  if (ext === ".gif") {
    return "image/gif";
  }
  throw new Error(`Unsupported image extension: ${ext || "(none)"}`);
}

function scoreModel(model) {
  const haystack =
    `${model.vendor || ""} ${model.family || ""} ${model.id || ""} ${model.name || ""}`.toLowerCase();
  let score = 0;
  if (model.capabilities?.imageInput) {
    score += 1000;
  }
  if (haystack.includes("claude") && haystack.includes("sonnet")) {
    score += 200;
  }
  if (haystack.includes("gpt-4o")) {
    score += 150;
  }
  if (haystack.includes("mini")) {
    score -= 10;
  }
  return score;
}

function normalizedModelFields(model) {
  return {
    id: (model.id || "").toLowerCase(),
    name: (model.name || "").toLowerCase(),
    family: (model.family || "").toLowerCase(),
  };
}

function parsePreferredModelIds() {
  const raw = process.env.GSH_VISION_MODEL_IDS || "claude-sonnet-4.6";
  return raw
    .split(",")
    .map((item) => item.trim().toLowerCase())
    .filter(Boolean);
}

function modelMatchesPreference(model, preferredIds) {
  const fields = normalizedModelFields(model);
  return preferredIds.some((pref) => {
    return (
      fields.id.includes(pref) ||
      fields.name.includes(pref) ||
      fields.family.includes(pref)
    );
  });
}

function allowsUndeclaredImageModel() {
  const raw = process.env.GSH_VISION_ALLOW_UNDECLARED_IMAGE_MODEL || "";
  if (!raw) {
    return true;
  }
  const normalized = raw.toLowerCase();
  if (["0", "false", "no", "off"].includes(normalized)) {
    return false;
  }
  return ["1", "true", "yes", "on"].includes(normalized);
}

async function selectVisionModel() {
  let models = await vscode.lm.selectChatModels({ vendor: "copilot" });
  if (!models.length) {
    models = await vscode.lm.selectChatModels({});
  }

  const preferredIds = parsePreferredModelIds();
  const visionModels = models.filter((model) => model.capabilities?.imageInput);
  const preferredVisionModels = visionModels.filter((model) =>
    modelMatchesPreference(model, preferredIds),
  );

  if (preferredVisionModels.length) {
    preferredVisionModels.sort(
      (left, right) => scoreModel(right) - scoreModel(left),
    );
    return preferredVisionModels[0];
  }

  if (visionModels.length) {
    visionModels.sort((left, right) => scoreModel(right) - scoreModel(left));
    return visionModels[0];
  }

  if (allowsUndeclaredImageModel()) {
    const preferredModels = models.filter((model) =>
      modelMatchesPreference(model, preferredIds),
    );
    if (preferredModels.length) {
      preferredModels.sort(
        (left, right) => scoreModel(right) - scoreModel(left),
      );
      return preferredModels[0];
    }
  }

  const candidates = models
    .map((model) => ({
      id: model.id || "unknown-id",
      name: model.name || "unknown-name",
      vendor: model.vendor || "unknown-vendor",
      imageInput: Boolean(model.capabilities?.imageInput),
    }))
    .slice(0, 20);

  const guidance = allowsUndeclaredImageModel()
    ? "No preferred fallback model was found."
    : "Set GSH_VISION_ALLOW_UNDECLARED_IMAGE_MODEL=1 to try a preferred model anyway.";

  throw new Error(
    `No chat model with image input capability is currently available. ${guidance} Candidates: ${JSON.stringify(candidates)}`,
  );
}

// ---------------------------------------------------------------------------
// Screenshot capture (macOS)
// ---------------------------------------------------------------------------

function defaultScreenshotPath() {
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  return path.join(os.tmpdir(), `screenshot-${ts}.png`);
}

function execPromise(cmd, args, options = {}) {
  return new Promise((resolve, reject) => {
    execFile(
      cmd,
      args,
      { timeout: 15000, ...options },
      (err, stdout, stderr) => {
        if (err) reject(new Error(stderr || err.message));
        else resolve(stdout.trim());
      },
    );
  });
}

async function findWindowIdByAppName(appName) {
  // Use AppleScript to get the window ID of the frontmost window of the named app
  const script = `
    tell application "System Events"
      set targetProc to first process whose name is "${appName.replace(/"/g, '\\"')}"
      set targetWin to first window of targetProc
      return id of targetWin
    end tell
  `;
  const result = await execPromise("osascript", ["-e", script]);
  return result.trim();
}

async function takeScreenshot(input) {
  const outputPath =
    input.output_path || input.outputPath || defaultScreenshotPath();
  const mode = input.mode || "fullscreen";

  const dir = path.dirname(outputPath);
  fs.mkdirSync(dir, { recursive: true });

  const args = ["-x"]; // silent — no camera shutter sound

  if (mode === "window" && (input.app_name || input.appName)) {
    const appName = input.app_name || input.appName;
    const windowId = await findWindowIdByAppName(appName);
    args.push("-l", windowId);
  } else if (mode === "region") {
    const x = input.x ?? 0;
    const y = input.y ?? 0;
    const w = input.width ?? 800;
    const h = input.height ?? 600;
    args.push("-R", `${x},${y},${w},${h}`);
  }

  args.push(outputPath);

  await execPromise("screencapture", args);

  if (!fs.existsSync(outputPath)) {
    throw new Error("screencapture did not produce an output file.");
  }

  const stats = fs.statSync(outputPath);
  return {
    path: outputPath,
    size: stats.size,
    mode,
  };
}

async function readOptionalSidecar(imageUri) {
  const sidecarUri = imageUri.with({ path: `${imageUri.path}.json` });
  try {
    const raw = await vscode.workspace.fs.readFile(sidecarUri);
    return new TextDecoder().decode(raw);
  } catch {
    return undefined;
  }
}

async function analyzeImages(input, token) {
  const paths = input.imagePaths || input.image_paths || [];
  const goal = input.goal || input.question || "";

  if (!paths.length || !goal) {
    throw new Error("analyzeImages requires image_paths (1–10) and goal.");
  }
  if (paths.length > 10) {
    throw new Error("Maximum 10 images per analysis call.");
  }

  const model = await selectVisionModel();

  // Build message parts: text prompt first, then all images in order
  const parts = [];

  const promptLines = [
    "You are analyzing screenshots during visual development and testing.",
    "Use the images themselves as the primary evidence. Answer directly and concisely.",
    `Number of images: ${paths.length}`,
    "",
    `Goal: ${goal}`,
  ];

  if (input.context) {
    promptLines.push("", `Additional context: ${input.context}`);
  }

  if (input.styleContext || input.style_context) {
    const qss = input.styleContext || input.style_context;
    promptLines.push(
      "",
      "## Active QSS/CSS Stylesheet",
      "The following stylesheet is applied to the page being viewed.",
      "When suggesting fixes, reference specific selectors and properties from this stylesheet.",
      "",
      "```css",
      qss,
      "```",
    );
  }

  // Load all images and their sidecars
  for (let i = 0; i < paths.length; i++) {
    const imageUri = vscode.Uri.file(paths[i]);
    const sidecar = await readOptionalSidecar(imageUri);
    if (sidecar) {
      promptLines.push(
        "",
        `Image ${i + 1} (${path.basename(paths[i])}) metadata:`,
        sidecar,
      );
    }
  }

  parts.push(new vscode.LanguageModelTextPart(promptLines.join("\n")));

  for (const imagePath of paths) {
    const imageUri = vscode.Uri.file(imagePath);
    const imageBytes = await vscode.workspace.fs.readFile(imageUri);
    const mime = mimeTypeForFile(imagePath);
    parts.push(vscode.LanguageModelDataPart.image(imageBytes, mime));
  }

  const messages = [vscode.LanguageModelChatMessage.User(parts)];

  const response = await model.sendRequest(messages, {}, token);
  let text = "";
  for await (const chunk of response.text) {
    text += chunk;
  }

  if (!text.trim()) {
    throw new Error("Vision model returned an empty response.");
  }

  return {
    model: model.name || model.id || model.family || "unknown",
    imageCount: paths.length,
    response: text.trim(),
  };
}

async function handleVisionRequest(method, args, token) {
  if (method === "take_screenshot") {
    const result = await takeScreenshot(args);
    return {
      model: "screencapture",
      response: `Screenshot saved to ${result.path} (${result.size} bytes, mode: ${result.mode})`,
      path: result.path,
    };
  }

  if (method === "analyze_images") {
    return analyzeImages(
      {
        imagePaths: args.imagePaths || args.image_paths,
        goal: args.goal || args.question,
        context: args.context,
        styleContext: args.styleContext || args.style_context,
      },
      token,
    );
  }

  if (method === "analyze_video") {
    const { analyzeVideo: analyzeVideoCore } = require("./lib/video-analysis");
    const output = await analyzeVideoCore(
      {
        videoPath: args.videoPath || args.video_path,
        goal: args.goal,
        startSec: args.startSec || args.start_sec,
        endSec: args.endSec || args.end_sec,
        sampleEverySec: args.sampleEverySec || args.sample_every_sec,
        maxFrames: args.maxFrames || args.max_frames,
        includeReport: args.includeReport ?? args.include_report,
        includeTimeline: args.includeTimeline ?? args.include_timeline,
        keepTempDir: args.keepTempDir || args.keep_temp_dir,
        autoTranscribe: args.autoTranscribe ?? args.auto_transcribe,
        whisperModel: args.whisperModel || args.whisper_model,
      },
      async (imageInput) => analyzeImages(imageInput, token),
    );
    return {
      model: "video-analysis-pipeline",
      response: JSON.stringify(output, null, 2),
    };
  }

  if (method === "transcribe_video") {
    const { transcribeOnly } = require("./lib/video-analysis");
    const result = await transcribeOnly({
      videoPath: args.videoPath || args.video_path,
      whisperModel: args.whisperModel || args.whisper_model,
    });
    return {
      model: "local-asr",
      response: JSON.stringify(result, null, 2),
    };
  }

  // Legacy compat: map old tool names to the unified function
  if (method === "inspect_screenshot") {
    const imagePath = args.imagePath || args.image_path;
    return analyzeImages(
      {
        imagePaths: imagePath ? [imagePath] : [],
        goal: args.question,
        context: args.context,
      },
      token,
    );
  }

  if (method === "compare_screenshots") {
    const paths = [];
    const ref = args.referenceImagePath || args.reference_image_path;
    const test = args.testImagePath || args.test_image_path;
    if (ref) paths.push(ref);
    if (test) paths.push(test);
    return analyzeImages(
      {
        imagePaths: paths,
        goal: args.question,
        context: args.context,
      },
      token,
    );
  }

  throw new Error(`Unsupported IPC method: ${method}`);
}

function makeToolResult(value) {
  return new vscode.LanguageModelToolResult([
    new vscode.LanguageModelTextPart(value),
  ]);
}

function parseAnalyzePrompt(prompt) {
  const trimmed = (prompt || "").trim();
  if (!trimmed) {
    return undefined;
  }

  // Format: /analyze path1 :: path2 :: ... :: goal
  // At minimum: one path :: goal
  const parts = trimmed.split("::").map((part) => part.trim());
  if (parts.length < 2) {
    return undefined;
  }

  const goal = parts[parts.length - 1];
  const imagePaths = parts
    .slice(0, -1)
    .map((p) => p.replace(/^['"]|['"]$/g, ""))
    .filter(Boolean);

  if (!imagePaths.length || !goal) {
    return undefined;
  }

  return { imagePaths, goal };
}

function usageMarkdown() {
  return [
    "Use `@gsh-vision /analyze` with this format:",
    "",
    "`/analyze /path/to/image.png :: What should be evaluated?`",
    "",
    "Multiple images (up to 10):",
    "",
    "`/analyze /path/to/img1.png :: /path/to/img2.png :: /path/to/img3.png :: Compare these and identify differences`",
    "",
    "Use `@gsh-vision /analyze-video` for video analysis:",
    "",
    "`/analyze-video /path/to/video.mp4 :: What should be analyzed?`",
    "",
    "Example:",
    "",
    "`/analyze /Users/alexwaldmann/Desktop/AIO Server/tmp/before.png :: /Users/alexwaldmann/Desktop/AIO Server/tmp/after.png :: How has the homescreen design changed between these two versions?`",
  ].join("\n");
}

function registerChatParticipant(context) {
  const participant = vscode.chat.createChatParticipant(
    "local.gsh-vision",
    async (request, _chatContext, stream, token) => {
      try {
        if (request.command === "analyze") {
          const parsed = parseAnalyzePrompt(request.prompt);
          if (!parsed) {
            stream.markdown(usageMarkdown());
            return;
          }

          const names = parsed.imagePaths.map((p) => path.basename(p));
          stream.progress(`Analyzing ${names.join(", ")}`);
          const result = await analyzeImages(
            { imagePaths: parsed.imagePaths, goal: parsed.goal },
            token,
          );
          stream.markdown(
            `Model: ${result.model} | Images: ${result.imageCount}\n\n${result.response}`,
          );
          return;
        }

        if (request.command === "analyze-video") {
          const parts = (request.prompt || "")
            .trim()
            .split("::")
            .map((p) => p.trim());
          if (parts.length < 2 || !parts[0]) {
            stream.markdown(
              "Usage: `/analyze-video /path/to/video.mp4 :: What to analyze?`",
            );
            return;
          }

          const videoPath = parts[0].replace(/^['"]|['"]$/g, "");
          const goal = parts.slice(1).join(" :: ");

          stream.progress(`Analyzing video: ${path.basename(videoPath)}`);
          const {
            analyzeVideo: analyzeVideoCore,
          } = require("./lib/video-analysis");
          const output = await analyzeVideoCore(
            { videoPath, goal },
            async (imageInput) => analyzeImages(imageInput, token),
          );

          if (output.report) {
            stream.markdown(output.report);
          } else {
            stream.markdown(
              `Video analysis complete.\n\n\`\`\`json\n${JSON.stringify(output, null, 2)}\n\`\`\``,
            );
          }
          return;
        }

        stream.markdown(usageMarkdown());
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        stream.markdown(`Image analysis failed: ${message}`);
      }
    },
  );

  context.subscriptions.push(participant);
}

function registerTool(context) {
  const analyzeToolNames = ["gsh-analyze-images", "gsh_analyze_images"];

  const analyzeTool = {
    async invoke(options, token) {
      const result = await analyzeImages(options.input, token);
      return makeToolResult(
        `Model: ${result.model} | Images: ${result.imageCount}\n\n${result.response}`,
      );
    },
    async prepareInvocation(options) {
      const paths = options.input.imagePaths || options.input.image_paths || [];
      const names = paths.map((p) => path.basename(p));
      return {
        invocationMessage: `Analyzing ${names.join(", ") || "images"}`,
      };
    },
  };

  for (const toolName of analyzeToolNames) {
    context.subscriptions.push(vscode.lm.registerTool(toolName, analyzeTool));
  }

  const screenshotToolNames = ["gsh-take-screenshot", "gsh_take_screenshot"];

  const screenshotTool = {
    async invoke(options) {
      const result = await takeScreenshot(options.input);
      return makeToolResult(
        `Screenshot saved to ${result.path} (${result.size} bytes, mode: ${result.mode})`,
      );
    },
    async prepareInvocation(options) {
      const mode = options.input.mode || "fullscreen";
      const appName = options.input.appName || options.input.app_name || "";
      const label =
        mode === "window" && appName
          ? `Capturing ${appName} window`
          : `Capturing ${mode} screenshot`;
      return { invocationMessage: label };
    },
  };

  for (const toolName of screenshotToolNames) {
    context.subscriptions.push(
      vscode.lm.registerTool(toolName, screenshotTool),
    );
  }

  // Video analysis tool
  const videoToolNames = ["gsh-analyze-video", "gsh_analyze_video"];

  const videoTool = {
    async invoke(options, token) {
      const {
        analyzeVideo: analyzeVideoCore,
      } = require("./lib/video-analysis");
      const output = await analyzeVideoCore(options.input, async (imageInput) =>
        analyzeImages(imageInput, token),
      );
      return makeToolResult(JSON.stringify(output, null, 2));
    },
    async prepareInvocation(options) {
      const videoPath =
        options.input.videoPath || options.input.video_path || "video";
      return {
        invocationMessage: `Analyzing video: ${path.basename(videoPath)}`,
      };
    },
  };

  for (const toolName of videoToolNames) {
    context.subscriptions.push(vscode.lm.registerTool(toolName, videoTool));
  }

  // Transcription-only tool (no vision model needed)
  const transcribeToolNames = ["gsh-transcribe-video", "gsh_transcribe_video"];

  const transcribeTool = {
    async invoke(options) {
      const { transcribeOnly } = require("./lib/video-analysis");
      const result = await transcribeOnly(options.input);
      return makeToolResult(JSON.stringify(result, null, 2));
    },
    async prepareInvocation(options) {
      const videoPath =
        options.input.videoPath || options.input.video_path || "video";
      return {
        invocationMessage: `Transcribing: ${path.basename(videoPath)}`,
      };
    },
  };

  for (const toolName of transcribeToolNames) {
    context.subscriptions.push(
      vscode.lm.registerTool(toolName, transcribeTool),
    );
  }
}

function registerCommand(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("gsh.inspectScreenshotManual", async () => {
      const imageUris = await vscode.window.showOpenDialog({
        canSelectMany: true,
        openLabel: "Analyze Images",
        filters: {
          Images: ["png", "jpg", "jpeg", "webp", "bmp", "gif"],
        },
      });
      if (!imageUris || !imageUris.length) {
        return;
      }

      const goal = await vscode.window.showInputBox({
        prompt: "What should Copilot determine from these images?",
        placeHolder:
          "Example: Evaluate the visual quality and identify any UI issues.",
      });
      if (!goal) {
        return;
      }

      try {
        const imagePaths = imageUris.map((uri) => uri.fsPath);
        const result = await analyzeImages(
          { imagePaths, goal },
          new vscode.CancellationTokenSource().token,
        );
        const pathList = imagePaths.map((p) => `- ${p}`).join("\n");
        const document = await vscode.workspace.openTextDocument({
          language: "markdown",
          content: `# Image Analysis\n\nImages (${result.imageCount}):\n${pathList}\n\nModel: ${result.model}\n\n${result.response}\n`,
        });
        await vscode.window.showTextDocument(document, { preview: false });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        void vscode.window.showErrorMessage(
          `Image analysis failed: ${message}`,
        );
      }
    }),
  );
}

function getIpcState() {
  const cacheDir = path.join(os.homedir(), ".cache", "gsh");
  fs.mkdirSync(cacheDir, { recursive: true });
  return {
    socketPath: path.join(os.tmpdir(), "gsh-vision.sock"),
    infoPath: path.join(cacheDir, "vision-ipc.json"),
  };
}

function safeUnlink(socketPath) {
  try {
    if (fs.existsSync(socketPath)) {
      fs.unlinkSync(socketPath);
    }
  } catch {
    // Ignore cleanup failures.
  }
}

function writeIpcInfo(infoPath, socketPath) {
  fs.mkdirSync(path.dirname(infoPath), { recursive: true });
  fs.writeFileSync(
    infoPath,
    JSON.stringify(
      {
        socketPath,
        updatedAt: new Date().toISOString(),
      },
      null,
      2,
    ),
    "utf8",
  );
}

function startIpcServer(context) {
  const { socketPath, infoPath } = getIpcState();
  safeUnlink(socketPath);
  writeIpcInfo(infoPath, socketPath);

  const server = net.createServer((socket) => {
    let buffer = "";
    socket.setEncoding("utf8");

    socket.on("data", async (chunk) => {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      for (const line of lines) {
        if (!line.trim()) {
          continue;
        }

        let request;
        try {
          request = JSON.parse(line);
        } catch {
          socket.write(
            JSON.stringify({ ok: false, error: "Invalid JSON payload" }) + "\n",
          );
          continue;
        }

        const tokenSource = new vscode.CancellationTokenSource();
        try {
          const result = await handleVisionRequest(
            request.method,
            request.arguments || {},
            tokenSource.token,
          );
          socket.write(
            JSON.stringify({
              ok: true,
              model: result.model,
              result: result.response,
            }) + "\n",
          );
        } catch (error) {
          const message =
            error instanceof Error ? error.message : String(error);
          socket.write(JSON.stringify({ ok: false, error: message }) + "\n");
        } finally {
          tokenSource.dispose();
        }
      }
    });
  });

  server.listen(socketPath);

  context.subscriptions.push({
    dispose() {
      try {
        server.close();
      } catch {
        // Ignore close failures during shutdown.
      }
      safeUnlink(socketPath);
      try {
        fs.unlinkSync(infoPath);
      } catch {
        // Ignore cleanup failures.
      }
    },
  });
}

function activate(context) {
  startIpcServer(context);
  registerTool(context);
  registerCommand(context);
  registerChatParticipant(context);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate,
};
