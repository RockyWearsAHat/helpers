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
// Screenshot capture (cross-platform)
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

function escapeForJavaScriptLiteral(value) {
  return JSON.stringify(String(value));
}

function escapeForPowerShellLiteral(value) {
  return `'${String(value).replace(/'/g, "''")}'`;
}

async function commandExists(cmd) {
  const checker = process.platform === "win32" ? "where.exe" : "which";
  try {
    await execPromise(checker, [cmd]);
    return true;
  } catch {
    return false;
  }
}

async function findFirstAvailableCommand(commands) {
  for (const cmd of commands) {
    if (await commandExists(cmd)) {
      return cmd;
    }
  }
  return null;
}

async function captureWindowMacOSWithQuartz(appName, outputPath) {
  const script = [
    "import json",
    "import sys",
    "",
    "from Quartz import (",
    "    CGWindowListCopyWindowInfo,",
    "    CGWindowListCreateImage,",
    "    CGRectNull,",
    "    kCGNullWindowID,",
    "    kCGWindowImageDefault,",
    "    kCGWindowListOptionAll,",
    "    kCGWindowListOptionIncludingWindow,",
    "    CGImageDestinationCreateWithURL,",
    "    CGImageDestinationAddImage,",
    "    CGImageDestinationFinalize,",
    ")",
    "from Foundation import NSURL",
    "",
    "app_name = sys.argv[1].strip().lower()",
    "output_path = sys.argv[2]",
    "",
    "windows = CGWindowListCopyWindowInfo(kCGWindowListOptionAll, kCGNullWindowID) or []",
    "candidates = []",
    "",
    "for window in windows:",
    "    owner = str(window.get('kCGWindowOwnerName') or '')",
    "    if owner.lower() != app_name:",
    "        continue",
    "",
    "    layer = int(window.get('kCGWindowLayer') or 0)",
    "    alpha = float(window.get('kCGWindowAlpha') or 1)",
    "    bounds = window.get('kCGWindowBounds') or {}",
    "    width = int(bounds.get('Width') or 0)",
    "    height = int(bounds.get('Height') or 0)",
    "    if layer != 0 or alpha <= 0 or width <= 0 or height <= 0:",
    "        continue",
    "",
    "    title = str(window.get('kCGWindowName') or '')",
    "    window_id = int(window.get('kCGWindowNumber') or 0)",
    "    if window_id <= 0:",
    "        continue",
    "",
    "    score = width * height",
    "    if title:",
    "        score += 1_000_000",
    "    if width >= 300 and height >= 200:",
    "        score += 250_000",
    "",
    "    candidates.append({",
    "        'window_id': window_id,",
    "        'title': title,",
    "        'width': width,",
    "        'height': height,",
    "        'score': score,",
    "    })",
    "",
    "if not candidates:",
    "    raise RuntimeError(f\"No capturable window found for {sys.argv[1]}.\")",
    "",
    "best = None",
    "image = None",
    "for candidate in sorted(candidates, key=lambda item: item['score'], reverse=True):",
    "    image = CGWindowListCreateImage(",
    "        CGRectNull,",
    "        kCGWindowListOptionIncludingWindow,",
    "        candidate['window_id'],",
    "        kCGWindowImageDefault,",
    "    )",
    "    if image:",
    "        best = candidate",
    "        break",
    "",
    "if not image or not best:",
    "    tried = ', '.join(str(item['window_id']) for item in sorted(candidates, key=lambda item: item['score'], reverse=True)[:5])",
    "    raise RuntimeError(f\"Quartz could not create an image for any candidate window. Tried: {tried}.\")",
    "",
    "url = NSURL.fileURLWithPath_(output_path)",
    "dest = CGImageDestinationCreateWithURL(url, 'public.png', 1, None)",
    "if not dest:",
    "    raise RuntimeError('Failed to create image destination for screenshot output.')",
    "",
    "CGImageDestinationAddImage(dest, image, None)",
    "if not CGImageDestinationFinalize(dest):",
    "    raise RuntimeError('Quartz failed to write the screenshot PNG.')",
    "",
    "print(json.dumps({'windowId': best['window_id'], 'title': best['title']}))",
  ].join("\n");

  const result = await execPromise("python3", ["-c", script, appName, outputPath], {
    timeout: 30000,
  });
  return JSON.parse(result);
}

async function findVisibleWindowIdByAppNameMacOS(appName) {
  const script = `
ObjC.import("CoreGraphics");
ObjC.import("CoreFoundation");

const targetName = ${escapeForJavaScriptLiteral(appName)}.toLowerCase();
const options =
  $.kCGWindowListOptionOnScreenOnly |
  $.kCGWindowListExcludeDesktopElements;
const list = $.CGWindowListCopyWindowInfo(options, $.kCGNullWindowID);
const count = Number($.CFArrayGetCount(list));
let foundWindow = null;

function unwrap(value) {
  try {
    return ObjC.unwrap(value);
  } catch (_error) {
    return value;
  }
}

for (let index = 0; index < count; index += 1) {
  const entry = ObjC.castRefToObject($.CFArrayGetValueAtIndex(list, index));
  const ownerName = String(unwrap(entry.objectForKey("kCGWindowOwnerName")) || "");
  if (!ownerName || ownerName.toLowerCase() !== targetName) {
    continue;
  }

  const layer = Number(unwrap(entry.objectForKey("kCGWindowLayer")) || 0);
  const alpha = Number(unwrap(entry.objectForKey("kCGWindowAlpha")) || 1);
  const windowId = Number(unwrap(entry.objectForKey("kCGWindowNumber")) || 0);
  if (layer !== 0 || alpha <= 0 || windowId <= 0) {
    continue;
  }

  const title = String(unwrap(entry.objectForKey("kCGWindowName")) || "");
  foundWindow = { windowId, ownerName, title };
  break;
}

if (!foundWindow) {
  throw new Error("No visible on-screen window found for " + ${escapeForJavaScriptLiteral(appName)} + ".");
}

console.log(JSON.stringify(foundWindow));
  `;
  const result = await execPromise("osascript", ["-l", "JavaScript", "-e", script]);
  return JSON.parse(result).windowId;
}

async function findWindowIdByAppNameLinux(appName) {
  if (!(await commandExists("xdotool"))) {
    throw new Error(
      "Window screenshots on Linux require xdotool to find a visible window.",
    );
  }

  const result = await execPromise("xdotool", [
    "search",
    "--onlyvisible",
    "--name",
    appName,
  ]);
  const windowId = result.split(/\s+/).find(Boolean);
  if (!windowId) {
    throw new Error(`No visible window found for \"${appName}\".`);
  }
  return windowId;
}

async function takeScreenshotMacOS(input, outputPath, mode) {
  const args = ["-x"];

  if (mode === "window") {
    const appName = input.app_name || input.appName;
    if (!appName) {
      throw new Error("Window capture requires app_name or appName.");
    }
    let quartzError = null;
    if (await commandExists("python3")) {
      try {
        await captureWindowMacOSWithQuartz(appName, outputPath);
        return;
      } catch (error) {
        quartzError = error;
      }
    }

    try {
      const windowId = await findVisibleWindowIdByAppNameMacOS(appName);
      args.push("-l", String(windowId));
    } catch (fallbackError) {
      const detail = quartzError
        ? `${quartzError.message} | ${fallbackError.message}`
        : fallbackError.message;
      throw new Error(`Window capture failed for ${appName}: ${detail}`);
    }
  } else if (mode === "region") {
    const x = input.x ?? 0;
    const y = input.y ?? 0;
    const w = input.width ?? 800;
    const h = input.height ?? 600;
    args.push("-R", `${x},${y},${w},${h}`);
  }

  args.push(outputPath);
  await execPromise("screencapture", args);
}

async function takeScreenshotLinux(input, outputPath, mode) {
  const x = input.x ?? 0;
  const y = input.y ?? 0;
  const w = input.width ?? 800;
  const h = input.height ?? 600;

  if (mode === "fullscreen") {
    const tool = await findFirstAvailableCommand([
      "grim",
      "gnome-screenshot",
      "import",
    ]);
    if (tool === "grim") {
      await execPromise("grim", [outputPath]);
      return;
    }
    if (tool === "gnome-screenshot") {
      await execPromise("gnome-screenshot", ["-f", outputPath]);
      return;
    }
    if (tool === "import") {
      await execPromise("import", ["-window", "root", outputPath]);
      return;
    }
    throw new Error(
      "No supported Linux screenshot backend found. Install grim, gnome-screenshot, or ImageMagick import.",
    );
  }

  if (mode === "region") {
    if (await commandExists("grim")) {
      await execPromise("grim", ["-g", `${x},${y} ${w}x${h}`, outputPath]);
      return;
    }
    if (await commandExists("import")) {
      await execPromise("import", [
        "-window",
        "root",
        "-crop",
        `${w}x${h}+${x}+${y}`,
        outputPath,
      ]);
      return;
    }
    throw new Error(
      "Region screenshots on Linux require grim or ImageMagick import.",
    );
  }

  if (mode === "window") {
    const appName = input.app_name || input.appName;
    if (!appName) {
      throw new Error("Window capture requires app_name or appName.");
    }
    if (!(await commandExists("import")) || !(await commandExists("xdotool"))) {
      throw new Error(
        "Window screenshots on Linux require both xdotool and ImageMagick import.",
      );
    }
    const windowId = await findWindowIdByAppNameLinux(appName);
    await execPromise("import", ["-window", windowId, outputPath]);
    return;
  }

  throw new Error(`Unsupported screenshot mode: ${mode}`);
}

async function takeScreenshotWindows(input, outputPath, mode) {
  const shell = await findFirstAvailableCommand(["powershell.exe", "pwsh"]);
  if (!shell) {
    throw new Error("Windows screenshot capture requires PowerShell.");
  }

  const x = Number(input.x ?? 0);
  const y = Number(input.y ?? 0);
  const w = Number(input.width ?? 800);
  const h = Number(input.height ?? 600);
  const appName = input.app_name || input.appName || "";

  let rectScript = "";
  if (mode === "fullscreen") {
    rectScript = "$rect = [System.Windows.Forms.SystemInformation]::VirtualScreen";
  } else if (mode === "region") {
    rectScript = `$rect = New-Object System.Drawing.Rectangle(${x}, ${y}, ${w}, ${h})`;
  } else if (mode === "window") {
    if (!appName) {
      throw new Error("Window capture requires app_name or appName.");
    }
    rectScript = [
      `$name = ${escapeForPowerShellLiteral(appName)}`,
      "$proc = Get-Process | Where-Object {",
      "  $_.MainWindowHandle -ne 0 -and (",
      '    $_.ProcessName -like ("*" + $name + "*") -or',
      '    $_.MainWindowTitle -like ("*" + $name + "*")',
      "  )",
      "} | Select-Object -First 1",
      'if (-not $proc) { throw ("No visible window found for " + $name + ".") }',
      "$nativeRect = New-Object GshNative+RECT",
      "[void][GshNative]::GetWindowRect($proc.MainWindowHandle, [ref]$nativeRect)",
      "$rect = New-Object System.Drawing.Rectangle($nativeRect.Left, $nativeRect.Top, $nativeRect.Right - $nativeRect.Left, $nativeRect.Bottom - $nativeRect.Top)",
    ].join(";\n");
  } else {
    throw new Error(`Unsupported screenshot mode: ${mode}`);
  }

  const script = [
    "$ErrorActionPreference = 'Stop'",
    "Add-Type -AssemblyName System.Drawing",
    "Add-Type -AssemblyName System.Windows.Forms",
    'Add-Type @"\nusing System;\nusing System.Runtime.InteropServices;\npublic static class GshNative {\n  [StructLayout(LayoutKind.Sequential)]\n  public struct RECT {\n    public int Left;\n    public int Top;\n    public int Right;\n    public int Bottom;\n  }\n  [DllImport(\"user32.dll\")]\n  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);\n}\n"@',
    rectScript,
    'if ($rect.Width -le 0 -or $rect.Height -le 0) { throw "Resolved screenshot bounds were empty." }',
    "$bitmap = New-Object System.Drawing.Bitmap($rect.Width, $rect.Height)",
    "$graphics = [System.Drawing.Graphics]::FromImage($bitmap)",
    "$graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)",
    `$bitmap.Save(${escapeForPowerShellLiteral(outputPath)}, [System.Drawing.Imaging.ImageFormat]::Png)`,
    "$graphics.Dispose()",
    "$bitmap.Dispose()",
  ].join(";\n");

  await execPromise(shell, [
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    script,
  ]);
}

async function takeScreenshot(input) {
  const outputPath =
    input.output_path || input.outputPath || defaultScreenshotPath();
  const mode = input.mode || "fullscreen";

  const dir = path.dirname(outputPath);
  fs.mkdirSync(dir, { recursive: true });

  if (process.platform === "darwin") {
    await takeScreenshotMacOS(input, outputPath, mode);
  } else if (process.platform === "linux") {
    await takeScreenshotLinux(input, outputPath, mode);
  } else if (process.platform === "win32") {
    await takeScreenshotWindows(input, outputPath, mode);
  } else {
    throw new Error(
      `Unsupported screenshot platform: ${process.platform}. Supported platforms are macOS, Linux, and Windows.`,
    );
  }

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
