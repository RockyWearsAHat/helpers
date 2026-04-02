#!/usr/bin/env node
"use strict";

const readline = require("readline");
const createResearch = require("./lib/mcp-research");
const { RESEARCH_TOOLS, createHandler } = require("./lib/mcp-research-tools");

const MCP_VERSION = "2024-11-05";
const handleToolCall = createHandler(createResearch());

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

async function handleRequest(request) {
	const { id, method } = request;

	if (method === "initialize") {
		send({
			jsonrpc: "2.0",
			id,
			result: {
				protocolVersion: MCP_VERSION,
				capabilities: { tools: {} },
				serverInfo: {
					name: "GitHub Shell Helpers (Research)",
					version: "1.1.0",
				},
			},
		});
		return;
	}

	if (method === "notifications/initialized") {
		return;
	}

	if (method === "tools/list") {
		send({ jsonrpc: "2.0", id, result: { tools: RESEARCH_TOOLS } });
		return;
	}

	if (method === "tools/call") {
		const toolName = request.params?.name;
		const toolArguments = request.params?.arguments || {};
		try {
			const content = await handleToolCall(toolName, toolArguments);
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

function startServer() {
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
			process.stderr.write(`[git-research-mcp] Unhandled error: ${err.message}\n`);
			if (request.id != null) {
				sendError(request.id, -32603, err.message);
			}
		});
	});
}

module.exports = { startServer, handleRequest, handleToolCall, tools: RESEARCH_TOOLS };

if (require.main === module) {
	startServer();
}