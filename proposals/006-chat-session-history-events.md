# Proposal: Add Stable Chat Session History Events and Read API

**Target**: `microsoft/vscode`
**Component**: Chat / Extension API
**Type**: Feature Addition (Read-only Session Feed)

## Summary

Add a stable, read-only chat session history API that lets extensions observe committed chat turns and read session history without scraping internal JSONL files from `workspaceStorage`.

This complements Proposal 003 (active chat session focus events). Proposal 003 answers “which chat is active?” This proposal answers “what committed turns were added to that chat?”

## Motivation

Extensions increasingly need durable, per-conversation state:

- Long-term chat archives
- Searchable chat history caches
- Session-scoped memory and context files
- Automation that reacts when a conversation reaches a milestone
- Project tooling that wants committed user/assistant turns without reading private storage

Today there is no supported API for this. The only practical workaround is to watch VS Code’s internal JSONL session files under `workspaceStorage`, parse private records, and maintain a separate cache.

That workaround is brittle for three reasons:

1. **Private file format**: the JSONL shape is internal and can change without notice.
2. **Pathological record size**: a single committed response can be hundreds of kilobytes on one line. In real usage we observed a session file with only 22 records where one line was about 247 KB. Generic file-open/read flows become unreliable even when total file size is moderate.
3. **Wrong abstraction boundary**: extensions should not need to know where the chat service persists its backing store.

### Real-world use case: lossless archived chat cache

Our extension now maintains a lossless, chunked archive of Copilot chat history because direct reads of the raw JSONL storage are too fragile. The archive stores exact compressed JSONL chunks plus a compact search index and reconstructs readable text on demand.

That is a reasonable local workaround, but it is solving the wrong layer. The extension should be able to subscribe to committed chat-session events directly and build its cache from API events rather than by tailing internal files.

## Proposed API

```typescript
export namespace chat {
  /**
   * Fired when a new chat session is created.
   */
  export const onDidCreateSession: Event<ChatSessionMetadata>;

  /**
   * Fired when session metadata changes, such as title updates.
   */
  export const onDidChangeSessionMetadata: Event<ChatSessionMetadata>;

  /**
   * Fired when committed turns are appended to a chat session.
   */
  export const onDidAppendSessionTurns: Event<ChatSessionTurnsAppendedEvent>;

  /**
   * Fired when a chat session is deleted or disposed permanently.
   */
  export const onDidDeleteSession: Event<Uri>;

  /**
   * Read committed turns for a session.
   */
  export function getSessionTurns(
    session: Uri,
    options?: ChatSessionReadOptions,
  ): Thenable<readonly ChatSessionTurn[]>;
}

export interface ChatSessionMetadata {
  readonly session: Uri;
  readonly createdAt?: number;
  readonly title?: string;
  readonly location?: ChatLocation;
}

export interface ChatSessionTurnsAppendedEvent {
  readonly session: Uri;
  readonly startIndex: number;
  readonly turns: readonly ChatSessionTurn[];
}

export interface ChatSessionReadOptions {
  readonly startIndex?: number;
  readonly limit?: number;
}

export type ChatSessionTurn = ChatRequestTurn | ChatResponseTurn;
```

## Type Reuse

VS Code already has private session-history turn shapes in the proposed API surface (`ChatRequestTurn2`, `ChatResponseTurn2`). This proposal should promote equivalent stable read-only types instead of inventing a second representation.

The event feed and read API should expose only **committed** turns:

- User prompts that were actually sent
- Assistant response parts that were actually surfaced
- Tool invocation parts already represented in response content

The API should not expose:

- Draft input state
- Cursor-selection edits while the user is typing
- Hidden chain-of-thought or internal planner state
- Private persistence details such as JSONL offsets or storage paths

## Why Events and Read Both Matter

Extensions need both surfaces:

- **Events** for incremental caches, triggers, analytics, and session-scoped automations
- **Read API** for cold start, restoration after reload, backfill, and pagination

Either one alone is insufficient. Events without read force extensions to miss history after reload. Read without events forces polling.

## Implementation Notes

The core infrastructure already exists internally:

- Chat sessions already have stable resource URIs
- The renderer and session service already maintain committed history entries
- Session history is already persisted to disk

What is missing is a supported extension-host bridge.

Implementation should:

1. Raise append events from the internal session model when committed turns are finalized
2. Surface metadata-change events when titles or other session metadata change
3. Expose a read API backed by the in-memory session model, with lazy hydration from persisted storage when needed
4. Keep the storage format private; extensions should never see JSONL persistence details

## Safety and Privacy

- **Read-only**: extensions can observe and read committed history only
- **No draft leakage**: input-in-progress is out of scope
- **No hidden reasoning leakage**: only surfaced response parts are included
- **Opaque session identity**: session identity remains a URI, not a filesystem path

## Current Workaround

Current workaround in the field:

1. Subscribe to Proposal 003’s focus event to know the active session
2. Watch `workspaceStorage/*/chatSessions/*.jsonl`
3. Parse private JSONL records
4. Chunk/compress the file externally to avoid giant-line crashes
5. Maintain a private search index

This works, but it is the same class of workaround that Proposals 001–005 are trying to eliminate.

## Backward Compatibility

- Purely additive API
- No existing extension breaks
- Can coexist with Proposal 003 cleanly
- Extensions already using private storage can gradually migrate from file-watching to event subscriptions

## Relationship to Proposal 003

- **Proposal 003**: identify the active chat session
- **Proposal 006**: observe and read committed history within any chat session

Both should ship. Focus events are necessary for per-chat state. History events are necessary for durable chat-aware extensions.

## References

- [Proposal 003](003-chat-session-focus-stable.md) — stable active-session focus events
- [Issue #305853](https://github.com/microsoft/vscode/issues/305853) — chat session resource exposure momentum
- [PR #304532](https://github.com/microsoft/vscode/pull/304532) — chat session customizations
- [PR #305297](https://github.com/microsoft/vscode/pull/305297) — multi-chat support
- [PR #305730](https://github.com/microsoft/vscode/pull/305730) — session mapping and session-type changes