# Copilot Ecosystem & AI Coding Tools

## GitHub Copilot Architecture (2025-2026)

### Core Components
- **Copilot Chat**: Conversational AI in VS Code, JetBrains, CLI. Supports agent mode for multi-step tasks.
- **Copilot Edits**: Multi-file editing with working set. Apply AI-suggested changes across files.
- **Copilot Agent Mode**: Autonomous multi-step execution — reads files, runs terminal commands, iterates on errors. The "agentic" coding workflow.
- **Copilot Code Review**: AI-powered PR reviews on GitHub.com. Catches bugs, suggests improvements.
- **Copilot Workspace**: Full-feature planning from issue to PR (GitHub.com).

### Customization System (VS Code)
```
.github/
├── copilot-instructions.md    # Repo-wide instructions (always loaded)
├── instructions/              # Conditional instructions (.instructions.md)
│   └── shell-scripts.instructions.md  # applyTo: **/*.sh
├── agents/                    # Custom agent modes (.agent.md)
│   └── reviewer.agent.md
├── prompts/                   # Slash commands (.prompt.md)
│   └── explain.prompt.md
├── skills/                    # Packaged domain workflows
│   └── testing/SKILL.md
└── knowledge/                 # Reference docs (this directory)
```

**Key concepts:**
- `copilot-instructions.md`: Always injected into context. Keep concise, factual.
- `.instructions.md`: Conditional on `applyTo` glob patterns or `description` semantic matching.
- `.agent.md`: Custom agent modes with tool restrictions and role definitions.
- `.prompt.md`: Reusable slash commands. Support `agent:` frontmatter (not `mode:`).
- `SKILL.md`: Step-by-step domain expertise. Referenced from agents/prompts.

### MCP (Model Context Protocol) Integration
VS Code supports MCP servers for extending Copilot's capabilities:
```jsonc
// .vscode/mcp.json
{
  "servers": {
    "my-server": {
      "type": "stdio",
      "command": "node",
      "args": ["path/to/server.js"]
    }
  }
}
```
MCP servers provide tools that Copilot can invoke during agent mode — database queries, API calls, web search, custom analysis.

## AI Coding Tool Landscape

### Code Completion
| Tool | Model | Approach |
|------|-------|----------|
| GitHub Copilot | GPT-4o / Claude / custom | Inline completions + chat |
| Cursor | Claude / GPT-4 / custom | Fork of VS Code, AI-native |
| Windsurf (Codeium) | Custom models | AI-native IDE, Cascade agent |
| Supermaven | Custom (300K context) | Fast completions, large context |
| Continue | Any model (local/cloud) | Open-source, bring your own model |
| Amazon Q Developer | Amazon models | AWS ecosystem integration |
| Tabnine | Custom + local | Privacy-focused, on-premise option |

### Autonomous Coding Agents
| Tool | What it does |
|------|-------------|
| Copilot Agent Mode | Multi-step in VS Code — reads, edits, runs, iterates |
| Claude Code | Terminal-based agent, full codebase awareness |
| Devin (Cognition) | Fully autonomous agent with its own IDE/browser |
| SWE-agent (Princeton) | Open-source agent for GitHub issues |
| OpenHands (ex-OpenDevin) | Open-source autonomous coding platform |
| Cline | VS Code extension, fully autonomous agent mode |
| Aider | Terminal-based pair programming with git integration |

### Code Review AI
| Tool | Scope |
|------|-------|
| Copilot Code Review | PR-level review on GitHub |
| CodeRabbit | Automated PR review + chat |
| Sourcery | Python-focused refactoring suggestions |
| Qodo (ex-CodiumAI) | Test generation + PR review |

### Specialized Tools
| Tool | Purpose |
|------|---------|
| GitHub Copilot for CLI | Terminal command suggestions |
| Warp | AI-native terminal |
| Pieces | AI-powered snippet manager, context across IDEs |
| Cody (Sourcegraph) | Codebase-aware chat with code search |
| Bloop | Semantic code search |

## Effective AI-Assisted Development Patterns

### Good Prompting for Code
```
❌ "Fix this code"
✅ "This function should return the sum of even numbers in the list,
    but it's including odd numbers. The bug is in the filter condition."

❌ "Write a server"
✅ "Write an Express.js REST API with these endpoints:
    GET /users - list all users with pagination (limit/offset)
    POST /users - create user (validate name, email required)
    Use TypeScript, return JSON, include error handling."
```

### Copilot Customization Best Practices
1. **Keep `copilot-instructions.md` factual and concise**: Framework versions, conventions, project structure. Not aspirational prose.
2. **Use `applyTo` patterns**: Different instructions for different file types. Shell scripts get shell conventions.
3. **Agents restrict tools**: `tools: [read, search]` for read-only agents. Don't give edit tools to research agents.
4. **Skills contain tested workflows**: Step-by-step methodology that agents invoke. The "how" lives in skills.
5. **Knowledge files are reference material**: Language guides, API docs, patterns. Agents can search and read them.

### AI Coding Anti-Patterns
1. **Vibe coding without review**: Accepting AI-generated code without understanding it. You're responsible for what ships.
2. **Prompt-and-pray**: Giving vague instructions then hoping the AI guesses right. Be specific.
3. **Ignoring context window limits**: Dumping entire codebases into prompts. Be selective about context.
4. **Fighting the model**: If the AI keeps generating the wrong pattern, your instructions are ambiguous, not wrong. Clarify.
5. **Over-relying on AI for architecture**: AI is great at implementation. Architecture requires understanding tradeoffs the AI can't fully grasp from context alone.

## MCP Server Ecosystem

### Popular MCP Servers
- **filesystem**: Read/write local files with sandbox controls
- **github**: GitHub API — issues, PRs, repos, search
- **postgres/sqlite**: Database query and schema inspection
- **puppeteer/playwright**: Browser automation and testing
- **fetch**: HTTP requests with content extraction
- **memory**: Persistent key-value store for agent memory
- **brave-search/searxng**: Web search
- **sequential-thinking**: Structured reasoning tool

### Building MCP Servers
```javascript
// Minimal stdio MCP server (Node.js)
// Reads JSON-RPC from stdin, writes to stdout
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

const server = new Server({ name: "my-tool", version: "1.0.0" }, {
  capabilities: { tools: {} }
});

server.setRequestHandler("tools/list", async () => ({
  tools: [{
    name: "my_tool",
    description: "Does something useful",
    inputSchema: { type: "object", properties: { query: { type: "string" } } }
  }]
}));

server.setRequestHandler("tools/call", async (request) => {
  if (request.params.name === "my_tool") {
    return { content: [{ type: "text", text: "Result" }] };
  }
});

const transport = new StdioServerTransport();
await server.connect(transport);
```

---

*Sources: GitHub Copilot documentation, VS Code Copilot extensibility docs, Model Context Protocol specification, Cursor docs, various AI coding tool documentation*
