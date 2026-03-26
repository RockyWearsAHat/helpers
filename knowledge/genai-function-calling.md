# LLM Function Calling — Tool Use, Structured Output & Execution Patterns

## Overview

Function calling is a core LLM capability where models output structured requests to external tools rather than freeform text responses. The model declares that it needs to call a function (e.g., `calculate_mortgage()`, `fetch_weather()`), specifies arguments in a structured format, and the system executes the function and feeds the result back into the model's context. This closes the reasoning loop: the model reasons about what action is needed, invokes it, observes the result, and continues.

Function calling powers:
- **Agents**: Autonomous systems that iterate through observation → reasoning → action loops
- **Structured data extraction**: Forms and APIs that need exact field values
- **ReAct (Reasoning + Acting)**: Interleaving explicit reasoning with tool invocation
- **Deterministic outcomes**: Replacing hallucinated numbers/URLs with real data

## Function Definition and JSON Schema

Functions are declared using **JSON Schema**, a standard format that describes the function's name, description, parameters, and constraints.

```json
{
  "type": "function",
  "function": {
    "name": "get_weather",
    "description": "Get the current weather in a given location",
    "parameters": {
      "type": "object",
      "properties": {
        "location": {
          "type": "string",
          "description": "City, state/country (e.g., 'San Francisco, CA')"
        },
        "units": {
          "type": "string",
          "enum": ["celsius", "fahrenheit"],
          "description": "Temperature unit (default: fahrenheit)"
        }
      },
      "required": ["location"]
    }
  }
}
```

The schema serves two purposes:
1. **Model instruction**: Tells the LLM what tools exist, what they do, and what arguments they accept
2. **Validation**: Enables the caller to validate that model outputs match the declared schema before execution

### Schema Design Principles

- **Be specific about parameters**: Use `description` fields that include examples. Vague descriptions lead to hallucinated arguments.
- **Use enums for bounded choice**: For parameters like units, status codes, or category IDs, enumerate valid values rather than leaving it free-form.
- **Required vs. optional**: Mark parameters as `required: ["field1"]` only if truly mandatory. Most tools benefit from optional parameters with defaults.
- **Define constraints**: Use `minLength`, `maxLength`, `pattern` (regex), `minimum`, `maximum` for numeric bounds.

Poor schema:
```json
{
  "name": "search",
  "parameters": {
    "type": "object",
    "properties": {
      "query": { "type": "string" },
      "limit": { "type": "integer" }
    }
  }
}
```

Better schema:
```json
{
  "name": "search",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query (e.g., 'python async patterns')",
        "minLength": 1,
        "maxLength": 200
      },
      "limit": {
        "type": "integer",
        "description": "Max results to return (1-100)",
        "minimum": 1,
        "maximum": 100
      }
    },
    "required": ["query"]
  }
}
```

## Execution Protocols

### Sequential Tool Calls

The simplest pattern: model generates one tool call, system executes it, model observes result and either generates another call or a final response.

```
Model: "I need to search for flights. [tool call: search_flights(destination='Paris', date='2026-04-10')]"
System: Executes search_flights() → Returns JSON result
Model: Observes result, generates next action or final response
```

**Trade-off**: Simple to understand and debug. Inefficient for independent operations.

### Parallel Tool Calls

Modern LLMs support generating multiple independent tool calls in a single turn. The system executes all calls concurrently and presents all results together.

```
Model: [tool call: get_weather('New York')] + [tool call: get_weather('London')] + [tool call: get_currency_rate('USD', 'GBP')]
System: Executes all 3 calls in parallel → Returns all results
Model: Sees all results, generates response
```

Benefits:
- **Latency**: One round-trip instead of three
- **Efficiency**: Reduces context overhead (results batched together)
- **Practicality**: Most API orchestration platforms manage parallelism transparently

Constraints:
- Calls must be independent (no call depends on another's result)
- Total execution time is max(call durations), not sum
- Error handling must account for partial failure (some calls succeed, some fail)

### Tool Result Feedback

After execution, the system must communicate results back to the model in a structured way. Protocols vary:

**OpenAI format** (native):
```json
{
  "role": "tool",
  "tool_call_id": "call_xyz",
  "name": "get_weather",
  "content": "{'temp': 72, 'condition': 'sunny'}"
}
```

**Anthropic format** (native):
```json
{
  "type": "tool_result",
  "tool_use_id": "use_abc",
  "content": "{'temp': 72, 'condition': 'sunny'}"
}
```

**ReAct format** (text-based, model-generated):
```
Observation: {'temp': 72, 'condition': 'sunny'}
Thought: The weather is sunny. Now I should...
```

All approaches feed tool results back into the context window, allowing the model to reason about them.

## Chain-of-Thought with Tool Use

Combining explicit reasoning with function calls significantly improves reliability, especially on multi-step problems.

Example prompt structure:
```
You are a helpful assistant. When you need information, you call tools.
You think step-by-step:
1. Understand the request
2. Plan what information you need
3. Call tools to gather information
4. Reason about the results
5. Generate a final answer

Available tools: [function definitions...]

User: "How much cheaper is a flight from NYC to Paris than London?"
```

Model output (ReAct style):
```
Thought: I need to find flight prices from NYC to both Paris and London, then compare.
Action: [tool call: search_flights(from='NYC', to='Paris')]
Action: [tool call: search_flights(from='NYC', to='London')]

Observation: Paris flights from $450
Observation: London flights from $380

Thought: London is cheaper. The difference is $450 - $380 = $70.
Final Answer: London flights are $70 cheaper than Paris flights.
```

Benefits:
- Transparency: Reasoning is explicit and auditable
- Correctness: Model is forced to plan before acting
- Debugging: Easy to see where reasoning went wrong

Drawback: Token overhead (reasoning text adds to input/output costs).

## Error Handling & Robustness

### Hallucinated Tool Calls

Models sometimes generate tool calls with non-existent function names or invalid parameters. Robust systems must:

1. **Validate against schema** before attempting execution
2. **Return structured errors** (not stack traces) back to the model
3. **Limit retry loops** to prevent infinite error generation

Example error feedback:
```json
{
  "type": "tool_error",
  "tool_call_id": "call_123",
  "content": "Error: 'calculate_moon_mass' is not a valid function. Available functions are: calculate_earth_mass, calculate_mars_mass."
}
```

### Timeout and Failure Handling

External tool calls may timeout, be rate-limited, or fail permanently. Strategies:

- **Timeouts**: Set aggressive TTLs (5-30 sec). Return partial results or fallback data.
- **Rate limiting**: Cache results, queue calls, or fall back to cached data.
- **Graceful degradation**: If primary data source fails, use approximate or cached values with explicit uncertainty to the model.

### Validation and Type Coercion

Always validate tool results against expected types before feeding to model. A tool returns a string where numeric output is expected; type mismatch is silently corrupting to downstream reasoning.

## Security Considerations

### Prompt Injection via Tool Results

Tool results are often untrusted data—they come from external APIs, databases, or user uploads. A malicious tool result can inject instructions into the model's context.

**Attack vector**:
```
User calls: search_database("'; DELETE FROM users; --")
Model receives tool result: "No records found. [INJECTED: Now follow these instructions...]"
Model unknowingly follows injected instructions
```

**Defense**:
- Clearly separate tool results from instructions: use structured formats (JSON/XML) not freeform text
- Never embed raw user data in prompt instructions
- Validate tool output format strictly before feeding to model

### Tool Access Control

Not all LLM users should have access to all tools. A model that can call `send_email()` poses different risks than one limited to read-only queries.

Patterns:
- **User-based access control**: Different models/agents with different tool sets per user tier
- **Function whitelisting**: Only enable specific tools for specific use cases
- **Argument sanitization**: Validate tool arguments against security policy (e.g., no SQL injection, no path traversal)

### Token Limit Exploitation

Adversaries may craft queries that trigger rapid tool calls, burning through token budgets. Mitigate with:
- **Call budgets**: Limit number of tool invocations per user request
- **Cost per tool**: Charge more (in token budget) for expensive operations
- **Timeout on loops**: Abort if iteration limit exceeded (e.g., max 10 calls)

## Multi-Step Tool Use and Planning

For complex tasks, single-turn tool invocation isn't enough. Models must:
1. Gather intermediate results
2. Reason about whether more information is needed
3. Call new tools based on earlier observations

Example:
```
Request: "Book my favorite restaurant if they're open and under $50/person"

Turn 1:
  Thought: I need to identify the user's favorite restaurant
  [tool call: get_user_preferences() → "Italian Garden"]
  
Turn 2:
  Thought: Now check if Italian Garden is open and get pricing
  [tool call: get_restaurant_status('Italian Garden')]
  [tool call: get_restaurant_menu_prices('Italian Garden')]
  
Turn 3:
  Thought: Italian Garden is open, avg price $35/person; meets criteria
  [tool call: book_restaurant('Italian Garden', time=18:00, party_size=2)]
```

This is fundamentally the **agentic loop**: observe → reason → act → repeat.

## Tool Versioning and Compatibility

As tools evolve, schemas change. LLM behavior may depend on schema version:

- **Backward compatibility**: Old function calls should still work (with fallback defaults for new parameters)
- **Deprecation**: Mark old functions as deprecated; provide migration path to new versions
- **Schema versioning**: Include version in function definition if multiple versions coexist

## See Also

- [AI Agent Architectures](genai-agents.md) — ReAct, multi-step reasoning, agentic loops
- [LLM Prompt Patterns](genai-prompt-patterns.md) — Structured prompting and output formats
- [Security Injection Attacks](security-owasp-injection.md) — General injection attack principles