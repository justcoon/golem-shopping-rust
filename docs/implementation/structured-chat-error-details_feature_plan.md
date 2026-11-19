# Feature Implementation Plan: Clean Chat Error Handling

## 1. Overview & Goals

Currently, when any failure occurs during AI chat execution (e.g. HTTP 503 high demand, 429 rate limit, network timeout, JSON serialization failure), the backend:
1. Concatenates raw HTTP payloads (often raw JSON bodies like `[{"error":{"code":503,...}}]`) into the chat error string:
   `AI service error (HTTP 503 Service Unavailable): [{ "error": ... }]`
2. Emits this message twice: first as a `ChatChunk::TextDelta`, and second as a `ChatChunk::Error`.
3. Forces the frontend and chat UI to receive raw JSON strings, displaying raw technical dumps directly to users.

### Goals
- **Zero Schema Pollution**: Keep `ChatMessage`, `ChatResponse`, and `ChatChunk` clean and standard. Do not leak transport/debug fields (`detail: Option<String>`) into conversation history.
- **Log Technical Details on Backend**: Raw HTTP status codes, stack traces, and upstream JSON response bodies belong in server logs via `log::error!`, where developers can inspect them with `golem agent stream` or server logs.
- **Polite User-Facing Error Messages**: The backend will return a clean, friendly user message (e.g. 503 $\rightarrow$ *"The AI assistant is currently experiencing high demand. Please try again in a few moments."*).
- **Eliminate Fake `TextDelta` on Error**: Stop emitting `ChatChunk::TextDelta` on failure. An error is an operational event, not conversational speech. Emit only `ChatChunk::Error` and `ChatChunk::Done`.
- **Zero String Parsing on Frontend**: The frontend receives a clean, ready-to-display string. All regex matching, heuristic JSON extraction, and string parsing are removed from the frontend.

---

## 2. Architecture & Design

### 2.1 Backend Schema (`src/ai/schema.rs`)
**No schema changes.**
`ChatMessage`, `ChatResponse`, and `ChatChunk` remain unchanged:
```rust
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCallRecord>>,
    pub timestamp: String,
    pub r#type: Option<ChatMessageType>,
}

pub struct ChatResponse {
    pub reply: String,
    pub tool_calls_used: Vec<ToolCallRecord>,
    pub r#type: Option<ChatMessageType>,
}

pub enum ChatChunk {
    TextDelta { content: String },
    ToolStart { content: String, tool_call: ToolCallRecord },
    ToolResult { content: String, tool_call: ToolCallRecord },
    Done { content: String },
    Error { content: String },
}
```

### 2.2 Backend Agent Execution (`src/ai/agent_loop.rs`)

#### 1. Friendly Error Message Helper
```rust
fn map_http_error_to_user_message(status: http::StatusCode, body_bytes: &[u8]) -> String {
    let body_str = String::from_utf8_lossy(body_bytes);
    log::error!("AI service HTTP error {status}: {body_str}");

    match status.as_u16() {
        503 => "The AI assistant is currently experiencing high demand. Please try again in a few moments.".to_string(),
        429 => "The AI assistant is temporarily busy due to rate limits. Please try again shortly.".to_string(),
        _   => "The AI assistant is temporarily unavailable. Please try again later.".to_string(),
    }
}
```

#### 2. Clean Stream & Turn Emission (All 9 Error Sites)
In both `execute_agent_chat_turn` (synchronous) and `execute_agent_chat_stream` (streaming):
- **Remove all fake `ChatChunk::TextDelta` emissions on error.**
- Emit **only**:
  ```rust
  let _ = writer.write_one(ChatChunk::Error { content: user_msg }).await;
  let _ = writer.write_one(ChatChunk::Done { content: String::new() }).await;
  ```
- All error cases (missing key, serialization failure, network failure, HTTP error, body read error, JSON parse error, SSE decode error, turn limit) log technical details via `log::error!` and emit a concise user-friendly message.

### 2.3 State Persistence (`src/ai/chat_state.rs`)
When a turn ends in error:
- If no conversational speech occurred, the turn records the error message with `ChatMessageType::Error` and the clean user-facing string in `content`.
- The conversation context fed to future LLM turns is not poisoned by raw JSON dumps.

### 2.4 Frontend Layer

#### Pinia Store (`frontend/src/stores/assistantStore.ts`)
- In streaming handler:
  ```ts
  case "error":
    streamHadError = true;
    error.value = new Error(chunk.content);
    break;
  ```
- When stream completes with error and `streamingText` is empty:
  Push an assistant error message:
  ```ts
  messages.value.push({
    role: "assistant",
    content: chunk.content, // Clean human string directly from backend
    timestamp: new Date().toISOString(),
    type: "error",
  });
  ```

#### Vue View (`frontend/src/views/ShoppingAssistantView.vue`)
- Remove all regex matching, JSON parsing, and string heuristic functions (`formatErrorMessage`).
- Render `msg.content` directly in chat bubble.
- Render `assistantStore.error.message` directly in the error banner.

---

## 3. File-by-File Changes

| File | Status | Description |
|---|---|---|
| `src/ai/agent_loop.rs` | `[MODIFY]` | Add `map_http_error_to_user_message`. Log raw technical status/body to `log::error!`. Remove fake `TextDelta` emissions across all 9 error sites in sync & streaming paths. Emit clean user messages. |
| `frontend/src/stores/assistantStore.ts` | `[MODIFY]` | On stream error where `streamingText` is empty, record the error message directly from `chunk.content`. |
| `frontend/src/views/ShoppingAssistantView.vue` | `[MODIFY]` | Remove regex/JSON parsing heuristics. Render `msg.content` and `assistantStore.error.message` directly. |

---

## 4. Verification Plan

### Automated Checks
1. **Rust Code Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Rust Compilation**:
   ```bash
   cargo check --target wasm32-wasip2
   ```
3. **Clippy Linter**:
   ```bash
   cargo clippy --target wasm32-wasip2
   ```
4. **Rust Tests**:
   ```bash
   cargo test
   ```
5. **Golem WASM Build**:
   ```bash
   golem build --yes
   ```
6. **Frontend Build**:
   ```bash
   cd frontend && npx vite build
   ```

### Functional / Smoke Verification
- Verify that on an error (e.g. 503 high demand):
  1. The assistant speech bubble does not show "generating..." typing raw JSON.
  2. The primary text in the chat bubble is a clean, polite message: *"The AI assistant is currently experiencing high demand. Please try again in a few moments."*
  3. No raw JSON strings or HTTP status numbers are rendered in the UI.
  4. The bottom alert banner shows the exact same clean message with the `🔄 Retry` button.
  5. The server logs capture the full raw HTTP status and JSON response for developer debugging.
