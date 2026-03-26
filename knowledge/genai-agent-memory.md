# Agent Memory — Short-Term, Medium-Term, and Long-Term Persistence

## The Memory Problem: Context Windows Aren't Memory

Every agent interaction starts from zero. Customer service agents forget yesterday's conversation. Coding assistants have no recollection of architectural decisions from the previous sprint. Each session exists in isolation, with no mechanism to learn, adapt, or carry forward context.

This isn't a limitation that bigger context windows solve. The industry calls it **the illusion of memory**: stuffing 200K tokens into a prompt isn't memory. It's a bigger Post-it note. Close the session, and it's all gone. Memory means the information survives the session boundary.

### Why Context Windows Fail for Memory

- **Degradation before limits.** A model claiming 200K tokens typically becomes unreliable around 130K, with sudden performance drops. You can't safely use the full window.
- **No sense of importance.** All tokens weight equally. Your customer's name gets treated the same as a throwaway comment from three weeks ago.
- **Linear cost scaling.** Every token in the context window costs money. If you're reloading full conversation history, you're paying 10-100x more per interaction as sessions get longer.
- **No temporal awareness.** Context windows can't distinguish "what we knew at time T" from "what we know now." They're snapshots, not time-aware stores.

The distinction: **RAG is not agent memory.** RAG brings external knowledge into the prompt at inference time (useful for factual grounding). Memory brings continuity—it knows about previous interactions and adapts behavior based on accumulated experience over days, weeks, or months.

## The Four Memory Types: A Cognitive Framework

Research at Princeton (CoALA framework, 2023) and the broader cognitive science literature define four memory types that map directly to how human memory works. Every major agent framework builds on this taxonomy.

| Memory Type | Human Equivalent | What It Stores | Example in Agents |
|------------|------------------|----------------|------------------|
| **Working Memory** | Your brain's scratchpad; what you're actively thinking about | Current conversation context, retrieved data, intermediate reasoning | Conversation buffers, sliding windows, current session state |
| **Procedural Memory** | Muscle memory; knowing how to ride a bike without thinking | System prompts, agent code, decision logic, heuristics | Agent instructions, tool definitions, configuration |
| **Semantic Memory** | General knowledge; facts and concepts accumulated over time | User preferences, extracted facts, domain knowledge | Vector stores, knowledge bases, preference databases |
| **Episodic Memory** | Autobiographical memory; recalling specific experiences from your past | Past action sequences, conversation logs, decision audit trails | Timestamped logs, conversation transcripts, few-shot examples |

### Mapping to Lilian Weng's Formula

A foundational principle: **Agent = LLM + Memory + Planning + Tool Use**

- Short-term memory ≈ working memory (context window)
- Long-term memory ≈ procedural + semantic + episodic (persistent storage)

An agent with only working memory is like trying to do your job with a whiteboard that gets erased at the end of each day. You can reason through today's problem, but you can't learn from yesterday's mistakes or apply patterns from past successes.

## Memory Architecture: Three Paradigms Working Together

Production systems don't rely on a single storage mechanism. They converge on three paradigms working in concert, often backed by a unified database.

### Vector Stores for Semantic Memory

**What it does**: Text is converted to embeddings (128–2,048 dimensions), stored in a vector database, and retrieved through similarity search (typically HNSW—hierarchical navigable small world indexing).

**What it's good for**: Fast semantic lookup. "Has this customer mentioned coffee before?" → Vector search finds the coffee-related memory in ~10ms.

**What it's bad at**: Relationships and temporal context. Vector search finds that the customer mentioned coffee but can't tell you they prefer a specific shop, ordered last Tuesday, and always get oat milk.

**Common systems**: Pinecone, Weaviate, Milvus, Qdrant, native AI vector search in PostgreSQL, Oracle, Azure Cosmos DB.

```
Example: Customer service memory
Input: "Customer Sarah is still upset about the shipping delay from last week"
  ↓ (embed)
Vector: [0.82, -0.15, 0.43, ...]
  ↓ (store)
Vector DB
  ↓ (on next interaction)
Query: "What's Sarah's mood?"
Retrieve: "upset about shipping delay" + "last week"
```

### Knowledge Graphs for Relationship Memory

**What it does**: Facts are stored as entities and relationships, with edges capturing how they connect. Add bi-temporal modeling (tracking when events happened and when the system learned about them), and you can ask not just "what do we know?" but "what did we know at any point in time?"

**What it's good for**: Structural reasoning. "Is Sarah still upset?" requires understanding entities (Sarah), properties (upset), causation (shipping delay), and time (still relevant?).

**Example systems**: Neo4j, Amazon Neptune, Oracle Database's native property graph support (SQL/PGQ).

```
Graph structure:
  Customer[Sarah] --ordered--> Order[#12345]
  Order[#12345] --shipped-with--> Carrier[FedEx]
  Carrier --delayed-delivery--> Status[Late 5 days]
  Status --caused-feeling-on-[2025-03-15]--> Feeling[Upset]
```

Query: "What upset Sarah and when?" → Graph traversal gives the full chain and timestamps.

### Relational Databases for Factual Memory

Structured data goes in relational tables: user profiles, access controls, session metadata, audit logs.

**Why converged storage matters**: Most teams stitch this together with separate databases (Pinecone for vectors, Neo4j for graphs, Postgres for relational). This means:
- Three security models
- Three failure modes
- No shared transaction boundaries (memory inconsistency if one write fails)

Oracle's converged database (and similar unified systems) run all three paradigms natively:
- AI Vector Search for embeddings + similarity retrieval
- SQL/PGQ for property graph queries
- Relational tables for structured data
- JSON Document Store for flexible, schema-free objects

All four share the same ACID transaction boundary and security model. A single failed write either rolls back everything or commits everything, keeping agent memory consistent.

## Memory Operations: The Core Cycle

Every memory system runs on four core operations. Modern systems often delegate these decisions to the LLM itself rather than brittle if/then logic:

1. **ADD**: Store a completely new fact
   - Condition: Fact is novel (doesn't exist in vector store and isn't a duplicate)
   - Example: "Customer prefers meetings on Tuesdays" (never mentioned before)

2. **UPDATE**: Modify an existing memory when new information complements or corrects it
   - Condition: Fact contradicts or refines an old memory
   - Example: "Customer prefers Tuesday meetings" → "Customer prefers Tuesday morning meetings after recent feedback"

3. **DELETE**: Remove a memory when new information negates it
   - Condition: Fact is explicitly superseded
   - Example: "Customer is upset about shipping delay" → "Customer shipping delay resolved and satisfied"

4. **SKIP**: Do nothing when information is a repeat or irrelevant
   - Condition: Fact is already stored or trivial
   - Example: "Customer's email is sarah@example.com" (already in profile)

### The Extraction-Update Cycle

```
1. [Extraction] New conversation → LLM extracts candidate memories
   Input: "Actually, I was only upset because I needed it for my daughter's birthday party this Saturday. It arrived in time. Thanks for fixing that!"
   
2. [Update decision] Convert to memory operations
   ADD: "Daughter's birthday party this Saturday"
   DELETE: "Upset about shipping delay"
   UPDATE: "Customer satisfaction: high"
   
3. [Conflict detection] Compare against existing store
   Vector search: "Is there a similar memory?"
   Result: "Yes, 'customer upset about shipping delay' exists. This contradicts it."
   
4. [Execute] Run the UPDATE/DELETE
   Relational DB: Update customer_satisfaction = "high"
   Vector DB: Mark old memory as superseded, add new fact
```

## Hot-Path vs. Background Memory Updates

Two strategies for keeping memory current:

### Hot-Path Memory (Synchronous)

The agent explicitly decides to remember something **before responding**.

```
Agent reasoning:
  Observation: Customer said "I have a cat named Whiskers"
  Thought: "I should remember this for personalization"
  Action: Save memory → Add(pet: cat, name: Whiskers)
  ---delay added to response time---
```

**Pros**: Memory is immediately available for the next turn
**Cons**: Adds latency to every response; agent has to decide what's worth remembering

### Background Memory (Asynchronous)

A separate process extracts and stores memories **during or after** the conversation.

```
Response sent → [latency-free]
[Background process]
  Read full conversation
  Extract memories
  Update vector store, graph, relational DB
Next turn: Memory available (almost always)
```

**Pros**: No response latency; automatic extraction means nothing is missed
**Cons**: Slight delay before memory is available; requires robust error handling if background process fails

**Production pattern**: Start with background memory (less latency-sensitive for customer-facing agents). Use hot-path memory for critical decisions (e.g., immediately after a customer opt-out).

## Retrieval Strategies for Agent Context

When an agent runs, it needs to decide which memories to load. Loading all memories is expensive; loading too few breaks context.

### Time-Decay Retrieval

**Principle**: Recent events matter more than distant ones.

```
Memory score = base_relevance_score × decay_factor^(days_old)

Example:
"Customer upset (today)" → score = 1.0 × 1.0 = 1.0
"Customer upset (30 days ago, resolved)" → score = 0.8 × 0.5^(30/30) = 0.4

Retrieve top-K by score
```

### Semantic + Temporal Hybrid Retrieval

**Principle**: Combine semantic similarity with time relevance.

```
Query: "What's the customer's shipping preference?"

Candidates from vector search:
  1. "Prefers FedEx overnight" (1.0 similarity, 5 days old)
  2. "Mentioned shipping once" (0.6 similarity, 60 days old)
  3. "Likes UPS" (0.95 similarity, 2 years old)

Rank by: similarity × recency_weight
  1: 1.0 × 0.9 = 0.90
  2: 0.6 × 0.5 = 0.30
  3: 0.95 × 0.02 = 0.019

Retrieved: "Prefers FedEx overnight" (top candidate)
```

### Graph-Aware Retrieval

**Principle**: When no direct memory matches, traverse relationships to infer context.

```
Query: "What would frustrate this customer?"

Direct match: ∅ (no "frustration" memories)

Graph traversal:
  Customer → issued_complaint → Slow response times
  Customer → issued_complaint → Hard to reach support

Result: Memories about frustration inferred from complaint history
```

## Examples: Memory in Production

### Customer Service Agent

- **Working**: Current ticket, customer history in sliding window
- **Procedural**: Escalation rules, refund policies, response templates
- **Semantic**: Customer preferences, account status, known issues with customer
- **Episodic**: Ticket history, past interactions, notes from previous support reps

When customer returns, the agent loads recent episodic memories + relevant semantic preferences, giving context without stuffing the full conversation history.

### Coding Agent Over Days

- **Working**: Current file, test output, recent error messages
- **Procedural**: Language-specific conventions, project build commands, deployment steps
- **Semantic**: Architectural decisions made in sprint ("Use Redis for caching, not memcached"), team coding standards
- **Episodic**: Files modified, tests written, previous attempts at similar features

Over a multi-day implementation, the agent builds a growing episodic record. When returning to a module days later, it doesn't start from zero—it remembers what was tried, what failed, and why decisions were made.

### Continuously Learning Research Agent

- **Working**: Current paper being analyzed
- **Procedural**: Query formulation rules, citation management format
- **Semantic**: Knowledge graph of concepts, relationships, and claims from hundreds of papers
- **Episodic**: Papers read, notes on key findings, contradictions discovered between sources

The agent's semantic memory grows as a knowledge graph, eliminating the need to re-read related papers. Episodic memory prevents the same claim from being "discovered" twice.

## The Programmatic vs. Agentic Decision

Two design choices:

### Programmatic Memory (Developer-Controlled)

The developer decides what to store, when, and how.

```python
# Human architect decides: we store these fields
stored_memory_schema = {
    "customer_id": "...",
    "satisfaction_level": "...",
    "shipping_preference": "...",
}
```

**Pros**: Predictable, auditable, easy to debug
**Cons**: Requires rebuilding for each new use case; misses unexpected valuable information

### Agentic Memory (Agent-Controlled)

The agent itself decides what to remember, using structured tools.

```
Agent runs memory_add(fact="customer has cat named Whiskers", retention_days=365)
Agent runs memory_delete(fact="customer frustrated about shipping")
```

**Pros**: Adaptable, captures what agents naturally consider important
**Cons**: Less predictable; agents may hallucinate unnecessary memories; harder to audit

**Trend**: The field is moving toward agentic memory, with constraints. Agents that manage their own memory adapt to individual contexts without developer intervention for each new use case.

## See Also

- **genai-rag-architecture.md** — retrieval-augmented generation (related but stateless)
- **database-vector.md** — vector database fundamentals and indexing
- **genai-agents.md** — agent architectures and planning strategies
- **genai-agentic-coding.md** — memory patterns in multi-stage coding pipelines