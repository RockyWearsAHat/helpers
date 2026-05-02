"use strict";

const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFile } = require("child_process");

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

  const result = await execPromise(
    "python3",
    ["-c", script, appName, outputPath],
    { timeout: 30000 },
  );
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
    const width = input.width ?? 800;
    const height = input.height ?? 600;
    args.push("-R", `${x},${y},${width},${height}`);
  }

  args.push(outputPath);
  await execPromise("screencapture", args);
}

async function takeScreenshotLinux(input, outputPath, mode) {
  const x = input.x ?? 0;
  const y = input.y ?? 0;
  const width = input.width ?? 800;
  const height = input.height ?? 600;

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
      await execPromise("grim", ["-g", `${x},${y} ${width}x${height}`, outputPath]);
      return;
    }
    if (await commandExists("import")) {
      await execPromise("import", [
        "-window",
        "root",
        "-crop",
        `${width}x${height}+${x}+${y}`,
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
  const width = Number(input.width ?? 800);
  const height = Number(input.height ?? 600);
  const appName = input.app_name || input.appName || "";

  let rectScript = "";
  if (mode === "fullscreen") {
    rectScript = "$rect = [System.Windows.Forms.SystemInformation]::VirtualScreen";
  } else if (mode === "region") {
    rectScript = `$rect = New-Object System.Drawing.Rectangle(${x}, ${y}, ${width}, ${height})`;
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
    'Add-Type @"\nusing System;\nusing System.Runtime.InteropServices;\npublic static class GshNative {\n  [StructLayout(LayoutKind.Sequential)]\n  public struct RECT {\n    public int Left;\n    public int Top;\n    public int Right;\n    public int Bottom;\n  }\n  [DllImport("user32.dll")]\n  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);\n}\n"@',
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

async function takeScreenshot(input = {}) {
  const outputPath =
    input.output_path || input.outputPath || defaultScreenshotPath();
  const mode = input.mode || "fullscreen";

  fs.mkdirSync(path.dirname(outputPath), { recursive: true });

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

module.exports = {
  takeScreenshot,
};