# Long-Running Agent Harness Design — Initialization, Incremental Execution & Sprint Contracts

## Overview

Long-running autonomous agents (hours-long coding sessions, multi-day research projects) succeed or fail based on **harness design**—the scaffolding that decomposes work, tracks progress, manages context, and enables agents to resume coherently. An agent with a good harness easily builds a full-stack application across 6+ hour sessions; the same agent without one produces half-stubbed code, gets lost mid-implementation, and declares victory prematurely.

Anthropic's 2026 research identified two critical harness components: an **initializer agent** (run once) that sets up the environment, and **coding agents** (run repeatedly) that make incremental progress while leaving clean state for the next session.

## The Core Problem: Why Long Tasks Fail

When asked to build a "Claude.ai clone" or "music production app," agents exhibit two predictable failures:

### Failure 1: One-Shotting

The agent tries to implement everything at once. It generates code for auth, database, frontend, and API endpoints in a single pass—often running out of context mid-feature. The next session starts with half-implemented features and no documentation of what was intended.

**Result**: Subsequent agents spend time reverse-engineering what the previous agent was trying to do, rather than making forward progress.

### Failure 2: Premature Completion

After several features are built, the agent looks around, sees progress, and declares the job done—even though the spec calls for 200 features, and only 20 are finished.

**Result**: Users expect a full app; they get a demo.

### Root Cause

Both failures stem from agents lacking **external context** about what done looks like. Without an explicit feature list, success criteria, and progress tracking, agents use pattern matching: "I built some features; I'm done." This is catastrophic for complex work.

## The Two-Part Solution

### Part 1: Initializer Agent (One-Time Setup)

The initializer's job is to set up the environment for all subsequent agents. It runs once at the start and creates three key artifacts.

#### 1.1 Feature List File (JSON)

The initializer expands the user's high-level prompt into a comprehensive feature list, marking all as initially failing:

```json
{
  "project": "claude.ai Clone",
  "total_features": 247,
  "features": [
    {
      "id": "auth-login",
      "category": "authentication",
      "description": "User can log in with email and password",
      "steps": [
        "Navigate to login page",
        "Enter email and password",
        "Click login button",
        "Verify authenticated session created"
      ],
      "passes": false,
      "last_tested_by": null,
      "notes": ""
    },
    {
      "id": "chat-new",
      "category": "core_functionality",
      "description": "User can create a new chat conversation",
      "steps": [
        "Click 'New Chat' button",
        "Verify new conversation created in database",
        "Verify conversation appears in sidebar",
        "Verify user can type in chat area"
      ],
      "passes": false,
      "last_tested_by": null,
      "notes": ""
    },
    {
      "id": "chat-message-send",
      "category": "core_functionality",
      "description": "User can send a message in a conversation",
      "steps": [
        "Type message in chat input",
        "Press Enter key",
        "Verify message appears in chat history",
        "Verify message saved to database"
      ],
      "passes": false,
      "last_tested_by": null,
      "notes": ""
    }
    // ... 244 more features
  ]
}
```

**Key rules for coding agents**:
- Update `passes: true` only after end-to-end testing
- Never delete or edit feature descriptions (only `passes` and `notes`)
- If a feature is unfeasible, note it in `notes` and mark `passes: false`

JSON is chosen deliberately: models are less likely to accidentally corrupt JSON than Markdown. If Python code were tracking state, they'd reformat it haphazardly.

#### 1.2 Progress Tracking File (claude-progress.txt)

Narrative summary of what's been done, bugs, and next steps:

```
=== Initializer Session (Setup Only) ===
Time: 15 minutes
Task: Set up initial environment

Environment created:
- React + TypeScript frontend in /client
- FastAPI backend in /server
- SQLite database in /data/app.db
- Tailwind CSS configured
- Authentication schema designed (users table, JWT tokens)

Initial git commit:
  commit a7f2e1: Initial project scaffold (react, fastapi, database)
  
Feature list created: 247 features marked as failing

Database schema initialized:
  - users (id, email, password_hash, created_at)
  - conversations (id, user_id, title, created_at, updated_at)
  - messages (id, conversation_id, role, content, created_at)
  - api_keys (for future Claude integration)

What the first coding agent should do:
1. Implement login endpoint and auth middleware
2. Implement conversation CRUD (create, read, update, delete)
3. Mark the first 5-10 features as passing after testing

Known constraints:
- Database is SQLite (good for dev, consider PostgreSQL for scale)
- Auth uses JWT (stateless; expires in 7 days)
- Frontend uses Vite (fast rebuilds)

Do not modify these files:
- Feature list (only update `passes` field)
- This progress file (append new sessions, don't edit past ones)
```

#### 1.3 Init Script (init.sh)

A shell script that bootstraps the development environment:

```bash
#!/bin/bash
set -euo pipefail

echo "Starting Claude.ai clone development environment..."

# Install dependencies
echo "Installing Python dependencies..."
cd server && pip install -r requirements.txt && cd ..

echo "Installing Node dependencies..."
cd client && npm install && cd ..

# Start services in background
echo "Starting development servers..."
cd server && python -m uvicorn main:app --reload --port 8000 &
SERVER_PID=$!

cd client && npm run dev &
CLIENT_PID=$!

echo "✓ Backend running on http://localhost:8000"
echo "✓ Frontend running on http://localhost:5173"
echo ""
echo "To stop servers: kill $SERVER_PID $CLIENT_PID"

# Wait for servers to stabilize
sleep 2

# Run basic health check
echo "Running basic health check..."
if curl -s http://localhost:8000/health | grep -q '"status":"ok"'; then
  echo "✓ Backend is healthy"
else
  echo "✗ Backend health check failed"
fi

# Keep process alive
wait
```

When a coding agent starts, it runs: `source init.sh`. Within seconds, the dev environment is running and the agent can test end-to-end without figuring out how to start services.

### Part 2: Coding Agents (Repeated Sessions)

Each subsequent session follows a structured pattern:

#### 2.1 Session Startup (First 5 Minutes)

```
I'll start by getting my bearings.
[Tool: bash] pwd
→ /home/agent/claude-ai-clone

[Tool: read] cat ./claude-progress.txt
→ [reads last session's summary, bugs, next steps]

[Tool: bash] git log --oneline -20
→ [sees recent commits: "Implement login", "Add conversation CRUD", ...]

[Tool: read] cat ./feature_list.json | head -20
→ [scans for next failing feature]

[Tool: bash] source init.sh
→ [starts dev servers]

[Tool: bash] # Basic health check
curl http://localhost:8000/health
curl http://localhost:5173

Based on progress, I can see:
- Sessions 1-2 completed login and conversation CRUD
- 15 features are now passing
- Known bug: message streaming is stubbed
- Next task: Implement streaming responses from Claude API
```

**Key**: Agents read context from artifacts, not by reasoning. This saves hundreds of tokens per session.

#### 2.2 One Feature at a Time

Agents are explicitly instructed to pick one failing feature and implement it completely:

```
Next failing feature (feature_list.json):
{
  "id": "chat-streaming",
  "description": "User can stream chat responses from Claude",
  "steps": [
    "Send a message",
    "Verify response appears character-by-character",
    "Verify full response is saved to database"
  ],
  "passes": false
}

I'll implement this now. Steps:
1. Add Server-Sent Events (SSE) endpoint to FastAPI backend
2. Wire frontend chat component to consume SSE stream
3. Test end-to-end: send message → see streaming response
4. Mark feature as passing
5. Commit to git
6. Update claude-progress.txt
7. Pick next feature (or reset context if token budget exhausted)
```

**Why one feature per pass?** 
- Keeps work inherently incremental
- Prevents context bloat (agent isn't juggling five unfinished features)
- Each session produces a shippable state (all committed features work)

#### 2.3 End-of-Session Cleanup

Before the session ends or context resets, the agent:

```
=== End of Session ===

Git commits made:
- ✓ Implement streaming responses with Server-Sent Events
- ✓ Fix client-side stream parsing
- ✓ Add streaming tests

Features now passing: 18 (was 15)

Updated feature_list.json:
- chat-streaming: passes=true

End-of-session update to claude-progress.txt:
```

Then appends to progress file:
```
=== Session 3 (Feature Implementation) ===
Time: 1 hour 32 minutes
Model: Claude Opus 4.5
Features completed: 3 (18 total passing)

Features implemented:
- chat-streaming: SSE endpoint for streaming responses
- chat-history: Retrieve conversation history on page load
- settings-theme: Dark/light mode toggle

Bugs fixed:
- Message order was reversed in database queries (ORDER BY fixed)
- Race condition when two messages sent rapidly (added message queue)

Known issues (not yet fixed):
- Streaming sometimes drops last few characters (buffer flush issue)
  Priority: MEDIUM — workaround exists (reload page)

Architecture changes:
- Added /api/v1/stream endpoint (SSE)
- Added Message validation schema with Pydantic

Next session should:
1. Fix streaming buffer issue (HIGH PRIORITY)
2. Implement conversation sharing (spec calls for it)
3. Add admin dashboard for user management

All tests passing: 24/24
```

### Code Quality Standard

Agents are instructed to commit work in a state suitable for merging to main:
- No half-stubs ("I'll implement this later")
- Tests pass
- New features documented
- Descriptive commit messages

This contrasts with "I'll just push anything and deal with it later." Each session's work must be production-ready or the next session inherits a mess.

## Sprint Contracts (Optional, Advanced)

For very complex apps, agents can negotiate **sprint contracts** before implementing:

**Generator proposes**:
```
Sprint 5 Contract: Implement user authentication flow

What I will build:
- Login endpoint (/api/login) accepting email/password
- Password hashing with bcrypt
- JWT token generation (7-day expiry)
- /api/me endpoint to retrieve current user

How success will be verified (acceptance criteria):
1. User can log in with valid credentials
2. Invalid credentials return 401
3. JWT token is included in response
4. Token expires after 7 days
5. /api/me returns user data when token is valid
6. /api/me returns 401 when token is invalid or expired
```

**Evaluator reviews and agrees**:
```
This looks good. I'd add:
- Password must be 8+ characters
- Test: Account lockout after 3 failed attempts (future feature, not this sprint)
Done. Let's proceed.
```

**Then generator builds against the contract.** Enables tight feedback loops without over-specifying implementation.

## Common Failure Modes & Solutions

| Problem | Cause | Solution |
|---------|-------|----------|
| Agent one-shots and runs out of context | No feature decomposition | Initializer creates feature list; agents pick one feature per session |
| Agent declares victory early | No external success criteria | Feature list marks 200 features; agent can't declare done until all pass |
| Agent leaves bugs undocumented | No testing discipline | Require end-to-end testing before marking feature complete; include browser automation tools (Playwright MCP) |
| Agent forgets what happened | No progress tracking | claude-progress.txt provides explicit handoff at session start |
| Agent re-implements existing features | No feature visibility | Feature list shows what's done; agent reads it first thing |
| Dev environment takes 30 min to set up | No automation | init.sh brings everything up in < 1 minute |
| Agent can't find bugs in UI without manual testing | Static code review only | Provide Playwright/Puppeteer MCP for real browser interaction |

## Model Version Impact

As models improve, harness requirements change:

| Aspect | Sonnet 4.5 | Opus 4.6 | Future |
|--------|-----------|---------|--------|
| Needs explicit feature decomposition | YES | YES | Likely yes |
| Context resets necessary | YES (context anxiety) | NOT STRICTLY (handles longer sessions) | Probably less critical |
| Needs progress tracking file | YES (cohesion) | YES (efficiency) | Likely yes |
| Needs testing tools (Playwright) | YES (unreliable self-eval) | YES | Likely yes |
| Needs evaluator agent | YES (catches bugs) | YES (still valuable) | Depends on task |

Translation: Core harness patterns remain valuable even as models improve; the overhead decreases but doesn't disappear.

## Checklist for Effective Harness Design

- [ ] **Initializer agent** sets up feature list, progress file, init.sh, and initial git commit
- [ ] **Feature list is JSON** (not Markdown), with `passes` field agents update
- [ ] **Progress file documents** what happened, bugs, and next steps in prose
- [ ] **init.sh brings dev environment up in <1 min** with all services running
- [ ] **Coding agents read context first**: progress file → git log → feature list
- [ ] **One feature per session**: agents pick one failing feature, implement completely, test, commit
- [ ] **Testing is mandatory**: agents use browser automation (Playwright) to verify end-to-end
- [ ] **Git commits are descriptive** and reference feature IDs
- [ ] **Code is production-ready before session end** (no stubs, no "I'll fix later")
- [ ] **Context token budget is respected**: estimate min tokens needed and reset before overflow

## See Also

- [Agent Architecture](genai-agent-architecture.md) — Planner-generator-evaluator, multi-agent patterns
- [Context Engineering](genai-context-engineering.md) — Resets, compaction, handoff artifacts
- [Agent Evaluation Patterns](genai-agent-evaluation-patterns.md) — Testing tools, QA agents, grading criteria

---

**Sources**: Anthropic Engineering research (Justin Young, 2026); "Effective Harnesses for Long-Running Agents," https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents; Claude Agent SDK documentation