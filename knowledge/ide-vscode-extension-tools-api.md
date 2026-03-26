# VS Code Extension Tools API — Contributing Tools to Copilot

Extensions can contribute **Language Model Tools** to VS Code Copilot, enabling agents to invoke extension-provided capabilities. This is distinct from chat participants: tools perform focused actions within agent workflows, while participants handle entire requests.

## The Language Model Tools API

The `lm.registerTool` API allows extensions to register tools accessible to agents. When registered, tools appear in the agent's tool picker and can be invoked during agent operations.

### Tool Registration (TypeScript/JavaScript)

```typescript
import * as vscode from "vscode";

const tool = vscode.lm.registerTool("analyze", {
  name: "analyze-code",
  description: "Analyze code for complexity and performance issues",
  inputSchema: {
    type: "object",
    properties: {
      filePath: {
        type: "string",
        description: "Path to the file to analyze"
      },
      depth: {
        type: "integer",
        description: "Analysis depth (1-5)"
      }
    },
    required: ["filePath"]
  }
});
```

### Tool Callback Handler

```typescript
tool.onInvoked(async (input: Record<string, unknown>, token) => {
  const filePath = input.filePath as string;
  const depth = (input.depth as number) ?? 2;
  
  // Perform analysis
  const results = await analyzeFile(filePath, depth);
  
  // Return ToolResult
  return [
    {
      kind: "text",
      value: JSON.stringify(results, null, 2)
    }
  ];
});
```

The callback receives:
- **input:** Parameters from the agent (matches inputSchema)
- **token:** Cancellation token (when agent cancels the operation)

Returns a **ToolResult**: array of content items (text, markdown, images, file references).

## Tool Metadata

### inputSchema

A **JSON Schema** describing the tool's input parameters. Key properties:

- **type:** Always `"object"`
- **properties:** Object mapping parameter names to schemas
- **required:** Array of required parameter names
- **description:** Human-readable descriptions for each prop

Example:

```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "Search query",
      "minLength": 1,
      "maxLength": 100
    },
    "limit": {
      "type": "integer",
      "description": "Max results",
      "minimum": 1,
      "maximum": 100,
      "default": 10
    },
    "filters": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Optional filters"
    }
  },
  "required": ["query"]
}
```

These descriptions are shown to agents when they decide whether to invoke the tool. Good descriptions → better agent decisions.

### Tool Visibility

Tools are visible to agents if:
- Registered in the active extension
- Not marked as hidden (advanced configuration)
- Appropriate for current context (workspace has necessary packages, etc.)

Extensions can conditionally register tools based on workspace state.

## Tool Invocation Flow

1. **Agent evaluates task:** "I need to analyze this code for performance issues"
2. **Agent scans available tools:** Sees `analyze-code` from your extension
3. **Agent invokes:** Sends JSON object matching inputSchema
4. **VS Code calls handler:** Passes input to your tool callback
5. **Handler executes:** Does work (file analysis, API query, etc.)
6. **Handler returns result:** Text, markdown, or structured output
7. **Agent receives result:** Integrates into reasoning loop

Result types (ToolResult content items):

- **text:** Plain text
- **markdown:** Markdown-formatted text (agents render with formatting)
- **references:** File references (`{kind: "reference", uri, ...}`)
- **images:** Base64-encoded images for vision agents

## Extension Contribution Point (package.json)

To register tools, declare them in `package.json`:

```json
{
  "contributes": {
    "languageModelTools": [
      {
        "id": "analyze-code",
        "label": "Code Analyzer",
        "description": "Analyze code structure and complexity",
        "when": "workspaceHasFile:**.ts"
      }
    ]
  }
}
```

Properties:

- **id:** Unique identifier for the tool
- **label:** Short human-readable name
- **description:** What the tool does
- **when:** Conditional activation (optional). Tool only appears if expression is true (e.g., `workspaceHasFile` checks if workspace contains file matching pattern)

The tool is activated when the extension activates. Register the handler in your extension's `activate()` function.

## Practical Tool Patterns

### Pattern 1: File Analysis Tool

```typescript
async function handleAnalyzeFile(input: Record<string, unknown>) {
  const uri = vscode.Uri.file(input.filePath as string);
  const document = await vscode.workspace.openTextDocument(uri);
  
  // Analyze
  const complexity = calculateComplexity(document.getText());
  const issues = findIssues(document.getText());
  
  return [{
    kind: "markdown",
    value: `# Analysis Results
- Cyclomatic Complexity: ${complexity}
- Issues Found: ${issues.length}
${issues.map(issue => `- ${issue}`).join("\n")}`
  }];
}
```

### Pattern 2: API Integration Tool

```typescript
async function handleQueryAPI(input: Record<string, unknown>) {
  const query = input.query as string;
  const apiKey = vscode.workspace.getConfiguration("myExtension").apiKey;
  
  try {
    const response = await fetch(`https://api.example.com/search`, {
      method: "POST",
      headers: { "Authorization": `Bearer ${apiKey}` },
      body: JSON.stringify({ query })
    });
    
    const data = await response.json();
    
    return [{
      kind: "text",
      value: JSON.stringify(data, null, 2)
    }];
  } catch (error) {
    return [{
      kind: "text",
      value: `Error: ${error.message}`
    }];
  }
}
```

### Pattern 3: Database Query Tool

```typescript
async function handleQueryDB(input: Record<string, unknown>) {
  const table = input.table as string;
  const filters = input.filters as Record<string, unknown>;
  
  const connection = await getDBConnection();
  const results = await connection.query(table, filters);
  
  return [{
    kind: "markdown",
    value: `
# Query Results
\`\`\`json
${JSON.stringify(results, null, 2)}
\`\`\`
`
  }];
}
```

### Pattern 4: Vision Tool (Screenshots + Analysis)

```typescript
async function handleScreenshotAnalysis(input: Record<string, unknown>) {
  const imageUri = input.imageUri as string;
  
  // Get screenshot
  const screenshot = await takeScreenshot();
  
  // Analyze image (via separate model or tool)
  const analysis = await analyzeImage(screenshot);
  
  return [{
    kind: "text",
    value: analysis
  }];
}
```

Tools can return binary data (images) for agents with vision capabilities.

## Tool vs. Chat Participant

**Chat Participants** (via `chat.createChatParticipant`) handle full requests within the chat context. Example: `@codeai explain this function`.

**Tools** are invoked *by* agents as part of autonomous workflows. Example: agent decides to call the `analyze-code` tool to understand a codebase before making changes.

| Aspect | Tool | Participant |
|--------|------|-------------|
| Invocation | Agent-driven (autonomous) | User-driven (chat command) |
| Scope | Single focused action | Full request handling |
| Input | Structured parameters (JSON) | Free-form user text |
| Used for | Analysis, queries, transformations | Building domain-specific interfaces |

Most extensions that want agent integration use **tools**. Chat participants are for building specialized chat experiences (e.g., `@github` for GitHub-specific queries).

## Tool Approval & Permissions

When an agent invokes a tool:

1. **Approval dialog** appears showing tool name and input parameters
2. User approves/denies
3. If approved, tool handler executes
4. Result is reviewed (user can approve/deny result before it reaches agent)

Advanced users can pre-approve tools (skip the dialog) or enable auto-approval for specific tools via settings.

## Error Handling & Timeouts

Tools should handle errors gracefully:

```typescript
try {
  // Tool logic
  return [{ kind: "text", value: "Success" }];
} catch (error) {
  return [{
    kind: "text",
    value: `Tool failed: ${error.message}`
  }];
}
```

Errors in tools are caught and returned to agents as failed results. Agents can retry or use alternative approaches.

Timeouts: If a tool takes too long (seconds), VS Code may cancel it (via cancellation token). Handlers should check `token.isCancellationRequested`.

## Dynamic Tool Registration

Tools can be registered/unregistered dynamically based on workspace or extension state:

```typescript
export async function activate(context: vscode.ExtensionContext) {
  // Register tool if prerequisites are met
  if (workspaceHasPythonProject()) {
    registerPythonAnalysisTool(context);
  }
}
```

This allows extensions to offer tools only when relevant.

## Testing Tools

Tools can be tested via the Extension Test Suite:

```typescript
const mockInput = { filePath: "src/test.ts", depth: 2 };
const result = await toolHandler(mockInput, CancellationToken.None);
assert(result.length > 0);
```

## Performance Considerations

Tool execution happens synchronously from the agent's perspective; the agent waits for results. Keep tools fast:

- Cache expensive computations
- Offload to background processes if needed
- Return partial results if full analysis is slow
- Set reasonable timeouts

Slow tools slow down agent reasoning loops.

## Security & Trust Model

Tool handlers run with extension privileges:

- Can read/write any file
- Can execute terminal commands
- Can access workspace secrets
- Can make network requests

Trust the extension: if it's malicious, no tool-level protection helps. Use extensions from trusted sources.

Agents should not blindly invoke tools. The approval dialog is a human checkpoint: the user reviews what parameters the tool will receive. This prevents prompt injection attacks that try to trick agents into misusing tools.

## See Also

- [ide-vscode-agent-tools.md](ide-vscode-agent-tools.md) — How agents use tools
- [api-design.md](api-design.md) — Tool parameter design best practices
- [documentation-systems.md](documentation-systems.md) — Documenting tool behavior for agents