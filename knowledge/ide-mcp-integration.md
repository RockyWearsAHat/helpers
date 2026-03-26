# MCP Integration in VS Code — Protocol, Server Architecture & Tool Extension

The **Model Context Protocol (MCP)** is an open standard for extending LLM capabilities with specialized tools and resources. VS Code agents integrate MCP servers to access capabilities beyond built-in tools: database queries, API integrations, custom domain knowledge, and any computation the server implements.

## MCP Fundamentals

MCP establishes a **client-server relationship**:

- **Client:** VS Code agent (requests tools, invokes them)
- **Server:** External process that provides tools and resources

The server is independent; VS Code launches it, communicates via a **transport protocol**, and discovers available tools. Multiple MCP servers can run simultaneously, each contributing tools to the agent's toolkit.

### Why MCP?

Without MCP, extending agent capabilities requires building extension code inside VS Code (complex, tied to IDE release cycles). MCP decouples capability provision from the IDE:

- Build a server in any language (Node.js, Python, Go, Rust)
- Deploy it independently
- Multiple clients can use the same server (VS Code, Claude API, other LLM platforms)
- Update the server without updating the IDE

Example: A company's internal MCP server provides tools to:
- Query the team's database
- Access internal APIs
- Look up documentation
- Run compliance checks

The same server works for all Copilot-powered tools in the company, not just VS Code.

## Transport Mechanisms

MCP messages use **JSON-RPC 2.0** (structured RPC format). The protocol layer specifies *how* JSON-RPC messages are delivered between client and server. Two standard transports exist:

### STDIO Transport

Client and server communicate via standard input/output streams.

**How it works:**
1. VS Code launches the server process as a subprocess
2. VS Code sends JSON-RPC messages to the server's stdin (newline-delimited)
3. Server reads from stdin, processes requests, writes JSON-RPC responses to stdout
4. VS Code reads from the server's stdout
5. Server MAY write logging to stderr (ignored by VS Code)

**Example command:**
```bash
node mcp-server.js
```

VS Code pipes JSON-RPC messages (one per line) to the server's stdin; the server writes responses to stdout.

**Characteristics:**
- Simple (no networking complexity)
- Process-scoped (server lifecycle tied to parent; if VS Code closes, server dies)
- Requires the server executable to be available locally
- Default transport for most MCP servers

### Streamable HTTP Transport

Client and server communicate via HTTP (POST/GET) with optional Server-Sent Events (SSE) streaming.

**How it works:**
1. Server runs as an independent HTTP service (e.g., listening on `http://localhost:3000/mcp`)
2. VS Code sends JSON-RPC messages as HTTP POST requests to the MCP endpoint
3. Server responds with JSON or streams responses via SSE
4. VS Code can open SSE streams to receive server-initiated messages

**Characteristics:**
- Decoupled (server runs independently; multiple clients can connect)
- Network-based (allows remote servers)
- Stateful (server can maintain session state across requests)
- More complex (session management, authentication, DNS rebinding protection required)

**Example flow:**
```
POST https://example.com/mcp
{"jsonrpc": "2.0", "method": "tools/list", "id": 1}
→ Response: {"jsonrpc": "2.0", "result": {"tools": [...]}, "id": 1}
```

For long-running requests, the server responds with `Content-Type: text/event-stream` and streams JSON-RPC messages as SSE events.

## Server Registration in VS Code

Servers are configured in `.vscode/mcp.json` (workspace-scoped) or `~/.copilot/mcp.json` (user-scoped):

```json
{
  "mcpServers": {
    "weather": {
      "command": "node",
      "args": ["./servers/weather-server.js"]
    },
    "github": {
      "url": "http://localhost:3001/mcp",
      "env": {
        "GITHUB_TOKEN": "${env:GITHUB_TOKEN}"
      }
    }
  }
}
```

- **Subprocess entry:** Command and args to run a STDIO server
- **HTTP entry:** URL pointing to a Streamable HTTP server
- **env:** Environment variables injected into the server process

VS Code reads the configuration, starts/connects to servers, and discovers their tools.

## MCP Server Lifecycle

### Initialization Phase

When VS Code connects to an MCP server:

1. **Initialize request:** VS Code sends `initialize` request with protocol version, client info
2. **Initialize response:** Server responds with protocol version, supported features, capabilities
3. **Tools discovery:** VS Code requests `tools/list` to see available tools
4. **Server ready:** Tools are available to agents

### Request/Response Cycle

For each tool invocation:

1. Agent (in chat) invokes a tool: `#weather/getForecast` with parameters
2. VS Code sends `tools/call` JSON-RPC request to the server
3. Server processes the request (queries API, runs computation, returns data)
4. Server responds with tool result
5. VS Code returns result to agent

The agent sees the result asynchronously; if the tool takes 5 seconds, the chat shows "waiting for tool..." until the response arrives.

### Streaming & Long Operations

Servers can stream responses via SSE:

1. Agent invokes tool
2. VS Code sends HTTP POST to server with tool request
3. Server responds with `Content-Type: text/event-stream`
4. Server streams multiple JSON-RPC messages as SSE events
5. VS Code collects events and returns final result when done

This enables long-running operations: a tool that processes large datasets, searches a database, or trains a model can stream progress back to the agent.

## Tool Definition

MCP servers *declare* their tools via a tool schema. Example:

```json
{
  "name": "get_forecast",
  "description": "Get weather forecast for a location",
  "inputSchema": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "City name or coordinates"
      },
      "days": {
        "type": "integer",
        "description": "Number of days (1-7)",
        "default": 3
      }
    },
    "required": ["location"]
  }
}
```

The **inputSchema** is a JSON Schema describing expected parameters. VS Code uses this to:
- Validate agent-provided parameters before invoking the tool
- Show users exactly what the tool needs (for approval dialogs)
- Help agents understand what parameters to pass

## Resources vs. Tools

MCP also defines **resources**: static or dynamic data the server exposes. Example:

- Tools: `get_forecast`, `get_alerts` (computations)
- Resources: `weather://forecast/nyc.json` (static data from the weather API)

Agents can request resources directly without running a computation. Less relevant for most agent workflows but supported by the protocol.

## Authentication & Authorization

MCP servers can require authentication. Common patterns:

- **API keys:** Server expects header with API token
- **Environment variables:** VS Code passes tokens via `.vscode/mcp.json` env config
- **Interactive auth:** Server returns "please authenticate" challenge; IDE opens browser; user logs in

The authorization model is server-dependent; VS Code provides the plumbing (env injection, request forwarding).

## Building Custom MCP Servers

### Python Example (FastMCP Framework)

```python
from mcp.server.fastmcp import FastMCP

mcp = FastMCP("my-server")

@mcp.tool()
async def search_docs(query: str) -> str:
    """Search internal documentation"""
    # Your logic here
    return f"Found: {query}"

async def start():
    await mcp.run()
```

VS Code launches this server; it auto-discovers the `search_docs` tool.

### Node.js Example

```typescript
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

const server = new Server({
  name: "my-server",
  version: "1.0.0",
});

server.setRequestHandler(ListToolsRequestSchema, () => ({
  tools: [{
    name: "search_docs",
    description: "Search internal documentation",
    inputSchema: {
      type: "object",
      properties: { query: { type: "string" } },
      required: ["query"]
    }
  }]
}));

server.setRequestHandler(CallToolRequestSchema, (request) => {
  const { name, arguments: args } = request.params;
  if (name === "search_docs") {
    // Run tool logic
    return { content: [{ type: "text", text: "Result" }] };
  }
});

// Start server
const transport = new StdioServerTransport();
server.connect(transport);
```

### Deployment

- **Local STDIO server:** Ship a binary or script; add to `.vscode/mcp.json`
- **Remote HTTP server:** Deploy to a web service; point VS Code to the URL
- **Composite:** VS Code can run multiple servers of both types simultaneously

## Security Considerations

MCP servers run with IDE permissions; they can:
- Read/write files
- Execute terminal commands
- Make network requests
- Access environment variables

Threat model:
- **Malicious MCP server:** A compromised server can exfiltrate data or perform attacks
- **Prompt injection via tool output:** Tool returns content that tricks the LLM

Mitigation:
- Only use MCP servers from trusted sources
- Review server code before using
- Use network sandboxing (firewall, proxy) to limit where servers connect
- VS Code shows approval dialogs for tool calls; review carefully
- Post-approval for tool output (prevents prompt injection)

## MCP vs. VS Code Extensions

Both MCP servers and VS Code extensions extend capabilities:

| Aspect | MCP | Extension |
|--------|-----|-----------|
| Language | Any | TypeScript/JavaScript |
| Protocol | JSON-RPC | VS Code API |
| Lifecycle | Independent process | IDE-scoped |
| Deployment | Portable (share across platforms) | IDE-specific |
| Complexity | Simpler (single tool) | Complex (full IDE control) |
| Tool types | Tools mostly | Chat participants, diagnostics, UI |

For extending agent tools: MCP is preferred (portable, language-agnostic, decoupled). For IDE features (diagnostics, keybindings, UI modifications): extensions are needed.

## Advanced Patterns

### Chaining Servers

An MCP server can invoke another tool server:

```
VS Code Agent
  ↓ (invokes tool)
Server A
  ↓ (needs data from)
Server B
  ↓ (returns data)
Server A (transforms data)
  ↓ (returns to agent)
VS Code Agent
```

Enables modular composition of complex workflows.

### Caching & Performance

MCP servers should cache expensive computations:
- Database query results (TTL cache)
- API responses (rate limiting)
- Parsed files (watch for changes)

Without caching, repeated agent tool calls become bottlenecks.

## See Also

- [ide-vscode-agent-tools.md](ide-vscode-agent-tools.md) — How agents invoke MCP tools
- [web-sse-streaming.md](web-sse-streaming.md) — SSE transport details
- [api-design.md](api-design.md) — Tool schema design patterns