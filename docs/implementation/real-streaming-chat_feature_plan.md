# Feature Plan: Real-time Streaming for Chat and Assistant Invocations

- **Feature ID / Slug**: `real-streaming-chat`
- **Date**: 2026-11-03
- **Status**: Approved

---

## 1. Overview & Goals

Currently, `chat_stream` in both `ShoppingAssistantAgent` and `ManagerAgent` implements pseudo-streaming:
```rust
async fn chat_stream(&mut self, message: String) -> AgentStream<ChatChunk> {
    self.execute_turn(message).await.to_stream()
}
```
In this implementation:
1. `execute_turn(message).await` runs to 100% completion before `AgentStream` is even created or returned to the client.
2. The LLM request sends standard `stream: false`, buffering the entire response payload into memory with `http::response_bytes`.
3. `ToolStart` and `ToolResult` chunks are only recorded into a local `Vec<ChatChunk>` and never sent to the client while tools are actually running.
4. `.to_stream()` merely opens an `AgentStreamWriter` and flushes all pre-buffered chunks in a burst at the end.

### Goals

- Implement **true real-time streaming**:
  1. Return `AgentStream<ChatChunk>` immediately to the caller so the HTTP streaming response starts right away.
  2. Stream tool lifecycle events (`ChatChunk::ToolStart`, `ChatChunk::ToolResult`) in real time as each tool starts and finishes executing.
  3. Stream token deltas (`ChatChunk::TextDelta`) live token-by-token from the LLM provider using Server-Sent Events (`stream: true`) and WASI HTTP p3 incremental response body consumption (`http-body-util`).
  4. Ensure agent conversation history (`messages`) is properly updated in durable state even when streamed asynchronously.
  5. Apply the pattern cleanly to both `ShoppingAssistantAgent` and `ManagerAgent` while preserving backward compatibility for non-streaming `chat` and `get_llm_recommendations`.

### Non-Goals

- Changing frontend UI components or API contracts (the frontend already expects `ChatChunk` variants: `tool_start`, `tool_result`, `text_delta`, `error`, `done`).
- Adding bidirectional streaming (input remains standard string messages; output is an `AgentStream<ChatChunk>`).

---

## 2. Architecture & Technical Design

### 2.1 Asynchronous Stream Producer & State Management

In Golem Rust agents, methods are executed sequentially on a single thread. When a method returns `AgentStream<T>`, the Golem runtime keeps the invocation active until the `AgentStreamWriter<T>` is dropped.

Because `spawn_local` in Rust requires `'static` bounds on the spawned future, a background task cannot borrow `&mut self.messages` directly across `spawn_local`. To resolve this safely:
- Wrap `messages` in `Arc<std::sync::Mutex<Vec<ChatMessage>>>` inside `ShoppingAssistantAgentImpl` and `ManagerAgentImpl`.
- In `chat_stream`:
  1. Create `let (writer, stream) = AgentStream::new()`.
  2. Clone the `Arc` handle to messages and necessary config values.
  3. Return `stream` immediately.
  4. In `spawn_local`, drive the streaming execution loop, writing each `ChatChunk` to `writer.write_one(chunk).await` live.
  5. Upon turn completion (or error), lock the mutex to push the user message and final assistant message (with all `ToolCallRecord`s) to history, and close the stream.

### 2.2 LLM Server-Sent Events (SSE) Streaming

When invoking the LLM provider (OpenAI / OpenRouter / Gemini):
- Set `"stream": true` in the JSON request payload.
- Use `http-body-util::BodyExt::frame` to read data chunks from `response.into_body()` as `bytes::Bytes`.
- Maintain a line buffer to parse Server-Sent Event frames:
  - Lines starting with `data: [DONE]`: signals completion of current LLM response stream.
  - Lines starting with `data: { ... }`: deserialized as `OpenAiStreamChunk`.
- Handle two response types from the stream:
  - **Text tokens**: when `delta.content` is present, immediately write `ChatChunk::TextDelta { content }`.
  - **Tool call fragments**: when `delta.tool_calls` is present, accumulate `id`, `name`, and chunked `arguments` fragments by index until `finish_reason == Some("tool_calls")` or stream finishes.

### 2.3 Streaming Multi-turn Tool Loop

Once a stream turn ends with tool calls accumulated:
1. For each tool call:
   - Immediately write `ChatChunk::ToolStart { content: format!("Executing {name}..."), tool_call }`.
   - Dispatch the tool: `dispatcher.execute(&name, &arguments).await`.
   - Immediately write `ChatChunk::ToolResult { content: summary, tool_call }`.
   - Append tool execution result into the LLM conversation context (`role: "tool"`).
2. Begin the next LLM streaming turn (up to `MAX_LOOP_TURNS`).
3. When the final text stream finishes:
   - Write `ChatChunk::Done { content: String::new() }`.
   - Update `messages` history.

---

## 3. Proposed Changes & File Impact

| Action     | File Path                                                                                     | Description                                                                                                   |
| :--------- | :-------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------ |
| `[MODIFY]` | [`Cargo.toml`](../../Cargo.toml)                                                             | Add `http-body-util = "0.1"` for WASI HTTP p3 incremental response stream reading.                           |
| `[MODIFY]` | [`src/ai/agent_loop.rs`](../../src/ai/agent_loop.rs)                                         | Add SSE streaming parser, streaming tool accumulator, and `execute_agent_chat_stream` function.               |
| `[MODIFY]` | [`src/shopping_assistant/mod.rs`](../../src/shopping_assistant/mod.rs)                       | Use `Arc<Mutex<Vec<ChatMessage>>>` for agent history and wire `chat_stream` to live background stream task. |
| `[MODIFY]` | [`src/manager/mod.rs`](../../src/manager/mod.rs)                                             | Use `Arc<Mutex<Vec<ChatMessage>>>` for agent history and wire `chat_stream` to live background stream task. |

### Detailed Changes

#### 1. `Cargo.toml`
- Add dependency:
  ```toml
  http-body-util = "0.1"
  ```
  Matches `http-body 1.0.1` already pulled by `golem-rust`.

#### 2. `src/ai/agent_loop.rs`
- Define streaming data structures:
  ```rust
  #[derive(Deserialize, Debug)]
  struct OpenAiStreamChunk {
      choices: Option<Vec<OpenAiStreamChoice>>,
      error: Option<OpenAiErrorPayload>,
  }

  #[derive(Deserialize, Debug)]
  struct OpenAiStreamChoice {
      delta: Option<OpenAiStreamDelta>,
      finish_reason: Option<String>,
  }

  #[derive(Deserialize, Debug, Default)]
  struct OpenAiStreamDelta {
      content: Option<String>,
      tool_calls: Option<Vec<OpenAiStreamToolCall>>,
  }

  #[derive(Deserialize, Debug, Clone)]
  struct OpenAiStreamToolCall {
      index: Option<usize>,
      id: Option<String>,
      function: Option<OpenAiStreamFunction>,
  }

  #[derive(Deserialize, Debug, Clone)]
  struct OpenAiStreamFunction {
      name: Option<String>,
      arguments: Option<String>,
  }
  ```
- Implement `SseLineDecoder` helper for buffering and yielding complete `data: ...` lines from raw byte frames.
- Implement `execute_agent_chat_stream`:
  - Takes `&Config<AgentConfig>`, `Arc<Mutex<Vec<ChatMessage>>>`, `&[ToolSpec]`, `dispatcher`, and `mut writer: AgentStreamWriter<ChatChunk>`.
  - Connects with `stream: true` in `OpenAiChatRequest`.
  - Streams text chunks directly to `writer.write_one(ChatChunk::TextDelta)`.
  - Accumulates streamed tool calls, executes dispatcher, and streams `ToolStart` and `ToolResult` live.
  - Finalizes with `ChatChunk::Done` and appends messages to history.
- Unit tests:
  - Test SSE line decoding across fragmented TCP/HTTP chunks.
  - Test tool call delta accumulation from multiple streamed frames.

#### 3. `src/shopping_assistant/mod.rs` & `src/manager/mod.rs`
- Update `messages` field in `ShoppingAssistantAgentImpl` and `ManagerAgentImpl` to:
  ```rust
  messages: Arc<std::sync::Mutex<Vec<ChatMessage>>>
  ```
- Update `chat_stream`:
  ```rust
  async fn chat_stream(&mut self, message: String) -> AgentStream<ChatChunk> {
      let (writer, stream) = AgentStream::new();
      let messages = self.messages.clone();
      let config = self.config.clone();
      let user_id = self._id.clone();
      spawn_local(async move {
          let dispatcher = ShoppingAssistantDispatcher { user_id: &user_id };
          let tools = get_shopping_assistant_tools();
          execute_agent_chat_stream(
              &config,
              messages,
              message,
              SHOPPING_ASSISTANT_SYSTEM_PROMPT,
              &tools,
              &dispatcher,
              writer,
          ).await;
      });
      stream
  }
  ```
- Keep `chat` method using existing non-streaming logic or awaiting the turn.
- Update `get_history` (`self.messages.lock().unwrap().clone()`) and `clear_history` (`self.messages.lock().unwrap().clear()`).

---

## 4. Verification Plan

### Automated Checks

1. **Code Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Compilation & Type Check**:
   ```bash
   cargo check --target wasm32-wasip2
   ```
3. **Linter Verification**:
   ```bash
   cargo clippy --target wasm32-wasip2
   ```
4. **Automated Unit Tests**:
   ```bash
   cargo test
   ```
   (Verify SSE decoder, delta accumulator, and all 18 existing tests pass).
5. **Golem Component Build**:
   ```bash
   golem build --yes
   ```

---

## 5. Risks & Open Questions

- **Risk: HTTP Stream Disconnect / Client Drop**:
  - *Mitigation*: If the client cancels the stream, `writer.write_one(...).await` returns `Err`. The streaming loop checks for this error and gracefully terminates early.
- **Risk: LLM Provider HTTP Error during Streaming**:
  - *Mitigation*: If the initial HTTP response status is not 200, the body contains an error JSON, not SSE. The error handler reads the body, writes `ChatChunk::Error` and `ChatChunk::Done`, and safely updates state.

