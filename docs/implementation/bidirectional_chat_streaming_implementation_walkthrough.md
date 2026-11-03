# Feature Implementation Walkthrough: Bi-directional Chat Streaming with Input Stream & Live Token Streaming

- **Feature ID / Slug**: `bidirectional-chat-streaming`
- **Date**: 2026-11-03
- **Status**: Ready for Review <!-- Ready for Review | Approved -->

---

## 1. Executive Summary

This walkthrough documents the successful implementation of the **bi-directional chat streaming architecture** across the Rust backend, Vue 3 frontend, and static web client.

Key capabilities delivered:
1. **Bi-directional Streaming (`chat_session`) via Golem Durable Streams**:
   - Both `ShoppingAssistantAgent` and `ManagerAgent` expose `chat_session(&mut self, input: AgentStream<ClientMessage>) -> AgentStream<ChatChunk>`.
   - The method is configured with `durable_streams(input("input", name = "inbox"), output("$result", name = "outbox"), ...)`.
   - External clients establish a single long-lived session (`PUT /chat-session`), listen to live SSE events from `outbox`, and continuously stream incoming user prompts into `inbox` via HTTP `POST`.
2. **Real-Time Chunk Emission in Agent Loop (`src/ai/agent_loop.rs`)**:
   - Introduced `ChunkSink` trait with `AgentStreamChunkSink` and `NoopChunkSink`.
   - Tool start (`ChatChunk::ToolStart`) and tool completion (`ChatChunk::ToolResult`) are emitted in real time as tools execute, and `ChatChunk::TextDelta` is emitted immediately upon response generation.
3. **Frontend Integration (`frontend/`)**:
   - Added `createChatSession` to `assistantService.ts`.
   - Updated `assistantStore.ts` with persistent session management, sending prompts to `inbox` and streaming deltas smoothly into Vue reactive state with graceful fallback to single-turn streaming and synchronous chat.
   - Tied session lifecycle cleanup to `onUnmounted` in `ShoppingAssistantView.vue` and `clearMessages()`.
4. **Static Web Client Integration (`web/`)**:
   - Updated `web/assets/assistant.js` and `web/assets/manager.js` to reuse active Durable Streams sessions across consecutive user messages.

---

## 2. Changes Implemented

### File Modifications

| Action     | File Path                                                                                            | Summary of Changes                                                                                                                   |
| :--------- | :--------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------- |
| `[MODIFY]` | [`src/ai/schema.rs`](../../src/ai/schema.rs)                                                         | Added `ClientMessage` struct (`pub message: String`) with `#[derive(Schema, ...)]`.                                                  |
| `[MODIFY]` | [`src/ai/agent_loop.rs`](../../src/ai/agent_loop.rs)                                                 | Added `ChunkSink` trait, `AgentStreamChunkSink`, `NoopChunkSink`, and real-time chunk emissions in `execute_agent_chat_turn_internal`. |
| `[MODIFY]` | [`src/shopping_assistant/mod.rs`](../../src/shopping_assistant/mod.rs)                               | Added `chat_session` endpoint with Durable Streams (`inbox`/`outbox`) and wrapped `messages` in `Rc<RefCell<Vec<ChatMessage>>>`.   |
| `[MODIFY]` | [`src/manager/mod.rs`](../../src/manager/mod.rs)                                                     | Added `chat_session` endpoint with Durable Streams (`inbox`/`outbox`) and wrapped `messages` in `Rc<RefCell<Vec<ChatMessage>>>`.   |
| `[MODIFY]` | [`frontend/src/api/services/assistantService.ts`](../../frontend/src/api/services/assistantService.ts) | Implemented `ChatSession` interface and `createChatSession` with persistent SSE and `sendMessage` via `inbox`.                       |
| `[MODIFY]` | [`frontend/src/stores/assistantStore.ts`](../../frontend/src/stores/assistantStore.ts)               | Implemented persistent session lifecycle management, chunk handling, turn promises, and fallback paths.                             |
| `[MODIFY]` | [`frontend/src/views/ShoppingAssistantView.vue`](../../frontend/src/views/ShoppingAssistantView.vue) | Added session cleanup in `onUnmounted`.                                                                                              |
| `[MODIFY]` | [`web/assets/assistant.js`](../../web/assets/assistant.js)                                           | Reused active Durable Streams session for consecutive messages with cleanup on user switch and history clear.                       |
| `[MODIFY]` | [`web/assets/manager.js`](../../web/assets/manager.js)                                               | Reused active Durable Streams session for consecutive manager chat messages.                                                         |

---

## 3. Verification & Validation Results

### 3.1 Code Formatting

```bash
cargo fmt --check
```

**Result**: Passed cleanly with zero diffs.

### 3.2 Compilation & Type Checking

```bash
cargo check --target wasm32-wasip2
```

**Result**:
```text
    Checking shopping_rust_shopping v0.0.1 (/Users/coon/workspace-zv/git/golem-shopping-rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.49s
```

### 3.3 Linter Verification (Clippy)

```bash
cargo clippy --target wasm32-wasip2
```

**Result**: Passed with zero clippy errors (all RefCell borrow-across-await issues resolved).

### 3.4 Automated Unit & Integration Tests

```bash
cargo test
```

**Result**:
```text
running 16 tests
test cart::tests::test_saved_contact_info_construction ... ok
test ai::tests::test_resolve_endpoint_url ... ok
test inventory::tests::test_reserve_over_limit_fails ... ok
test inventory::tests::test_availability_calculation ... ok
test ai::tests::test_tool_call_record_and_chunk_serialization ... ok
test order_search::tests::test_query_matcher_quoted_terms ... ok
test order_search::tests::test_query_matcher_field_filters ... ok
test ai::tests::test_chat_schema_serialization ... ok
test order_search::tests::test_query_matcher_terms ... ok
test inventory::tests::test_release_and_fulfill ... ok
test ai::tests::test_missing_api_key_returns_graceful_error ... ok
test ai::agent_loop::tests::test_gemini_thought_signature_preservation ... ok
test ai::agent_loop::tests::test_gemini_thought_signature_fallback_when_missing ... ok
test product_search::tests::test_query_matcher_terms ... ok
test product_search::tests::test_extract_product_agent_id ... ok
test order_search::tests::test_extract_order_agent_id ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 3.5 Frontend Build Verification

```bash
cd frontend && npx vite build
```

**Result**:
```text
vite v5.4.19 building for production...
✓ 765 modules transformed.
dist/index.html                   0.75 kB │ gzip:  0.42 kB
dist/assets/index-JqOUyIdI.css   59.13 kB │ gzip: 10.30 kB
dist/assets/index-BlDCGhon.js   213.12 kB │ gzip: 75.66 kB
✓ built in 1.67s
```

---

## 4. How to Run & Verify

1. **Start the Golem local dev server**:
   ```bash
   golem server -v
   ```
2. **Deploy the application**:
   ```bash
   golem deploy --yes
   ```
3. **Verify bi-directional streaming via Durable Streams CLI/cURL**:
   - Initialize session:
     ```bash
     curl -i -X PUT "http://localhost:9006/v1/assistant/user-1/chat-session" \
          -H "Stream-TTL: 600"
     ```
     Take the `Location` header, e.g. `/v1/assistant/user-1/chat-session/invocations/<session-id>`.
   - Listen to SSE output on `outbox`:
     ```bash
     curl -N "http://localhost:9006<session-path>/streams/outbox?live=sse&offset=-1"
     ```
   - In another terminal, send prompt 1 to `inbox`:
     ```bash
     curl -X POST "http://localhost:9006<session-path>/streams/inbox" \
          -H "Content-Type: application/json" \
          -d '{"message": "What products do you have?"}'
     ```
   - Send prompt 2 in the same session without reconnecting:
     ```bash
     curl -X POST "http://localhost:9006<session-path>/streams/inbox" \
          -H "Content-Type: application/json" \
          -d '{"message": "Tell me more about the first product"}'
     ```
   - Observe events (`tool_start`, `tool_result`, `text_delta`, `done`) arriving continuously over the single SSE stream.

