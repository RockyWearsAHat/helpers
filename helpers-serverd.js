#!/usr/bin/env node
"use strict";

/*
 * helpers-serverd — warm MCP daemon.
 *
 * Loads the (heavy) MCP server modules ONCE and keeps them resident, then
 * serves each incoming connection as an independent MCP session. The C client
 * shim (helpers-mcp) connects here, so after the first launch every new agent
 * session starts in ~1ms instead of paying cold Node+compile startup.
 *
 * Lifecycle:
 *   - Socket path comes from $HELPERS_MCPD_SOCK (the shim derives it per workspace
 *     so different projects/env get isolated, correctly-scoped daemons).
 *   - Exits after IDLE_MS with no connections, so edits to the server code are
 *     picked up on the next launch and stale daemons don't linger.
 *   - Cleans up its socket on exit; refuses to double-bind a live socket.
 */

const net = require("net");
const fs = require("fs");
const path = require("path");
const os = require("os");

const { serveConnection } = require("./helpers-server.js");

const HOME = process.env.HOME || process.env.USERPROFILE || os.homedir();
const SOCK =
  process.env.HELPERS_MCPD_SOCK ||
  path.join(HOME, ".cache", "helpers", "mcpd-default.sock");
const IDLE_MS = Number(process.env.HELPERS_MCPD_IDLE_MS || 15 * 60 * 1000);

let connections = 0;
let idleTimer = null;

function armIdleExit() {
  clearTimeout(idleTimer);
  if (connections > 0) return;
  idleTimer = setTimeout(() => {
    cleanup();
    process.exit(0);
  }, IDLE_MS);
  // Don't let the idle timer itself keep the loop alive once everything else
  // is gone (belt-and-suspenders; the listening server keeps us alive anyway).
  if (idleTimer.unref) idleTimer.unref();
}

function cleanup() {
  try {
    fs.unlinkSync(SOCK);
  } catch {
    /* already gone */
  }
}

function start() {
  fs.mkdirSync(path.dirname(SOCK), { recursive: true });

  // If a live daemon already owns this socket, defer to it and exit cleanly.
  if (fs.existsSync(SOCK)) {
    const probe = net.connect(SOCK);
    probe.on("connect", () => {
      probe.destroy();
      process.exit(0); // someone beat us to it
    });
    probe.on("error", () => {
      // Stale socket from a dead daemon — remove and bind.
      cleanup();
      listen();
    });
    return;
  }
  listen();
}

function listen() {
  const server = net.createServer((sock) => {
    connections++;
    clearTimeout(idleTimer);
    sock.setNoDelay(true);
    serveConnection(sock);
    const onGone = () => {
      connections = Math.max(0, connections - 1);
      armIdleExit();
    };
    sock.on("close", onGone);
    sock.on("error", () => {});
  });

  server.on("error", (err) => {
    process.stderr.write(`[helpers-mcpd] listen error: ${err.message}\n`);
    process.exit(1);
  });

  server.listen(SOCK, () => {
    // Tighten perms: only the owner may talk to the daemon.
    try {
      fs.chmodSync(SOCK, 0o600);
    } catch {}
    process.stderr.write(`[helpers-mcpd] ready on ${SOCK}\n`);
    armIdleExit();
  });

  for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
    process.on(sig, () => {
      cleanup();
      process.exit(0);
    });
  }
  process.on("exit", cleanup);
}

start();
