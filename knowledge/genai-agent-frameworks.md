# AI Agent Frameworks — LangChain, LlamaIndex, AutoGen, and Beyond

## The Framework Landscape

Agent frameworks provide orchestration, memory management, tool integration, and planning for autonomous LLM systems. They abstract the agent loop (observe → think → act) into higher-level primitives. Major players:

- **LangChain** (Python, JS): Agent execution, chains, memory, tool binding
- **LangGraph** (Python): State machine-based agents, full control flow; sister project to LangChain
- **LlamaIndex**: Document indexing + agent query engine; RAG-first
- **AutoGen** (Microsoft): Multi-agent orchestration, teacher-student patterns, goal-driven agents
- **CrewAI**: High-level multi-agent framework; emphasizes agent personas and hierarchies
- **Semantic Kernel** (Microsoft): C#/.NET first; plugin architecture; integrates LLMs into existing applications
- **Rivet**: Low-code visual agent IDE; web-based

## Core Patterns

### Tool Use and Function Calling

Agents invoke external tools (APIs, code, databases) to ground outputs.

**Framework approach:**
- Define tool schemas (name, description, input/output types)
- LLM receives tool descriptions in system prompt or function signatures
- LLM outputs structured requests (tool name + arguments)
- Framework executes and returns results to LLM

**LangChain example**:
```python
from langchain.agents import Tool, initialize_agent
tools = [Tool(name="search", func=web_search, ...)]
agent = initialize_agent(tools, llm, agent="zero-shot-react-agent")
```

**LlamaIndex example**:
```python
agent = ReActAgent.from_tools([search_tool, code_tool], llm=llm)
response = agent.chat("What is X?")
```

**Key consideration**: Tool descriptions must be precise; verbose descriptions → more tokens; terse descriptions → poor tool selection.

### Memory and Context Management

Agents must maintain conversation history and long-term state. Different scopes:

#### Short-Term Memory (Conversation History)
Store the last N messages or within a token budget.

**LangChain**: `ConversationBufferMemory` (all), `ConversationBufferWindowMemory` (last N)
**LlamaIndex**: `ChatMemory` with sliding window
**AutoGen**: Built-in conversation history in `ConversableAgent.chat_history`

**Trade-off**: Short history loses context; long history consumes tokens.

#### Long-Term Memory (Semantic/Episodic)
Store summaries, embeddings, or structured facts for retrieval across sessions.

**Patterns**:
- **Vector database memory**: Embed conversation turns, retrieve similar past interactions
- **Summarization**: Periodically summarize old messages; keep summary in context
- **Explicit fact store**: Structured key-value or graph; agent queries relevant facts

**Example (LangChain)**:
```python
from langchain.memory import VectorStoreMemory
memory = VectorStoreMemory(vectorstore=faiss_db, memory_key="history")
```

#### Context Pruning
When context exceeds token limits, frameworks must decide what to keep.

**Strategies**:
- **Recency bias**: Keep recent messages
- **Salience**: Keep high-relevance messages (LLM-scored)
- **Compression**: Summarize unimportant context

### Planning and Reasoning Strategies

Different agent strategies for decomposing problems:

#### ReAct (Reasoning + Acting)
Interleave LLM reasoning and tool execution. The loop:
1. LLM generates "Thought" (reasoning) + "Action" (tool call)
2. Observe tool result
3. Update "Thought", decide on next "Action", or final "Answer"

**Pros**: Interpretable; can correct course mid-task.
**Cons**: More tokens; slower due to sequential steps.
**Frameworks**: LangChain ("react"), LlamaIndex (built-in), AutoGen

#### Chain-of-Thought (CoT)
Prompt LLM to explain reasoning before acting; single-shot or few-shot.

**Prompt template**:
```
Q: [User query]
Let's think step-by-step:
1. ...
2. ...
Given the above, I will call [tool] with [args].
```

#### Tree-of-Thought
Explore multiple reasoning paths; backtrack if one fails.

**Requires**: Branching logic; keeping multiple agent states; scoring/selecting best branch.

**Frameworks**: Custom implementations; AutoGen via nested agents.

#### Hierarchical / Multi-Agent Planning
Decompose tasks via agent specialization. Example:

- **Manager agent**: Decides high-level steps
- **Specialist agents**: Domain-specific sub-tasks
- **Synthesizer agent**: Combines results

**Frameworks**:
- **AutoGen**: `GroupChat` orchestrates multiple agents via LLM-driven turn-taking
- **CrewAI**: Agent "manager" oversees team of specialized agents
- **LangGraph**: Manual orchestration via graph nodes = agents


## Framework Comparison

### LangChain

**Strengths**:
- Largest ecosystem; extensive docs and community examples
- Chains for complex workflows
- Integrations: 200+ LLMs, retrievers, databases, tools
- Good for prototyping

**Weaknesses**:
- Complexity grows fast; multiple patterns to solve same problem
- Agent execution fragile if not carefully debugged
- Performance overhead (many abstraction layers)

**Best for**: Rapid prototyping, diverse tool integration, learning agents.

### LangGraph

**Strengths**:
- Explicit state machine; full control over agent flow
- Deterministic (vs. LLM-driven turn-taking in other frameworks)
- Works with any LLM backend
- Better debugging (state snapshots)

**Weaknesses**:
- Requires more boilerplate than high-level agent APIs
- Newer; smaller community
- Steeper learning curve

**Best for**: Complex agent workflows, fine-grained control, production systems.

### LlamaIndex

**Strengths**:
- RAG-first design; excellent document indexing and querying
- Agent + retriever co-designed
- Clean API for simple cases

**Weaknesses**:
- Less flexible for non-retrieval agents
- Smaller tool ecosystem than LangChain
- Memory management less mature

**Best for**: RAG agents, Q&A over documents, semantic search.

### AutoGen

**Strengths**:
- Multi-agent orchestration out-of-the-box
- Teacher-student patterns; group chat
- Built-in cost tracking and code execution

**Weaknesses**:
- Opinionated; less flexible for custom patterns
- Requires careful prompt engineering for agent personas
- Agent coordination can be unpredictable

**Best for**: Multi-agent systems, collaborative planning, team simulation.

### CrewAI

**Strengths**:
- Developer experience; intuitive agent definitions
- Agent roles, goals, backstories (easy personas)
- Built-in manager agent for delegation

**Weaknesses**:
- Less mature; fewer integrations
- Limited benchmarks
- Tightly coupled opinion on agent design

**Best for**: Hierarchical teams, role-based tasks, business workflows.

## Tool Integration Best Practices

### Schema Definition

Provide clear, concise schemas:

```
Tool: calculate
Description: Perform mathematical operations
Inputs:
  operation: string (add, subtract, multiply, divide)
  a: number
  b: number
Returns: number (result)
```

**Anti-patterns**: "Do stuff", vague parameter names, missing type info.

### Tool Execution Safety

- **Timeouts**: Prevent hanging tools (API calls, code execution)
- **Sandboxing**: Restrict code execution to safe environments (containers, jails)
- **Rate limiting**: Prevent tool exhaustion
- **Authorization**: Check agent has permission for tool

**Example (AutoGen code execution)**:
```python
agent = UserProxyAgent(
    name="user",
    code_execution_config={
        "work_dir": "/tmp/agent_work",
        "use_docker": True,  # sandbox
    }
)
```

### Fallback and Error Handling

If a tool fails, agent should retry with different args or different tool.

**LangChain pattern**:
```python
agent_executor = AgentExecutor.from_agent_and_tools(
    agent=agent,
    tools=tools,
    handle_parsing_errors=True,  # retry if output parsing fails
    max_iterations=10
)
```

## Memory Patterns in Practice

### Summarization on Context Overflow

When reaching token limit, summarize conversation history:

```
Summarize this conversation into 3-4 bullet points:
[full conversation]

---
Summary:
- Point 1
- Point 2
```

Then drop old messages; keep summary + recent messages.

### Retrieval-Augmented Memory

Embed conversation turns; on each new query, retrieve semantically similar past interactions:

```
Current query: "What was the integration timeline?"

Retrieved similar past turns:
1. "When did we plan the integration?"
2. "What was the rollout schedule?"

Context: [retrieved snippets] + [recent messages]
```

### Episodic Memory Graph

Store structured facts (entity + relation + entity):

```
{
  "alice": {
    "works_at": "company_x",
    "role": "engineer",
    "skills": ["Python", "Go"]
  }
}
```

Agent queries: "What does alice do?" → look up entity "alice" → return facts.

## Common Gotchas

1. **Token accumulation**: Conversation history grows unbounded; fix via summarization or windowing
2. **Tool reliability**: APIs fail; ensure fallbacks and graceful degradation
3. **Prompt sensitivity**: Small prompt changes → different agent behavior; need careful tuning
4. **Reasoning vs. acting trade-off**: More reasoning → more tokens but better decisions; too little reasoning → poor planning
5. **Agent communication**: In multi-agent systems, ensuring agents understand each other is hard; needs explicit schemas/protocols
6. **Cost explosion**: Agents can make many API calls; implement spending limits and tracking

## See Also

- [AI Agent Architectures — Reasoning, Planning & Tool Use](genai-agents.md)
- [LLM Function Calling — Tool Use, Structured Output & Execution Patterns](genai-function-calling.md)
- [Retrieval-Augmented Generation — Grounding LLMs in External Knowledge](genai-rag-patterns.md)