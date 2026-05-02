#!/usr/bin/env node
"use strict";

const assert = require("assert");
const path = require("path");

const screenshot = require("../vision-tool/screenshot");
const visionMcp = require("../vision-tool/mcp-server");

async function main() {
  const originalTakeScreenshot = screenshot.takeScreenshot;
  process.env.GSH_VISION_IPC_INFO_PATH = path.join(
    process.cwd(),
    ".tmp-does-not-exist-vision-ipc.json",
  );

  try {
    screenshot.takeScreenshot = async (input) => ({
      path: "/tmp/fake-shot.png",
      size: 321,
      mode: input.mode || "fullscreen",
    });

    const content = await visionMcp.handleToolCall("take_screenshot", {
      mode: "window",
      app_name: "Code",
    });

    assert.strictEqual(content.length, 1);
    assert.strictEqual(
      content[0].text,
      "Screenshot saved to /tmp/fake-shot.png (321 bytes, mode: window)",
    );
  } finally {
    screenshot.takeScreenshot = originalTakeScreenshot;
    delete process.env.GSH_VISION_IPC_INFO_PATH;
  }

  console.log("ok");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});