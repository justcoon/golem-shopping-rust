# Feature Implementation Walkthrough: Shopping Assistant Chat & Manager Agent (Custom LLM Loop with Streaming)

- **Feature ID / Slug**: `shopping-assistant-and-manager-chat`
- **Date**: 2026-11-03
- **Status**: Ready for Review <!-- Ready for Review | Approved -->

---

## 1. Executive Summary

Based on the TypeScript/Effect implementation in [`golem-shopping-effect`](file:///Users/coon/workspace-zv/git/golem-shopping-effect), we implemented conversational chat capabilities for `ShoppingAssistantAgent` and created the new `ManagerAgent`. Because `@earendil-works/pi-agent-core` is specific to TypeScript, we implemented a custom multi-turn tool-calling loop using WASI HTTP p3 (`golem_rust::http`) and native streaming (`golem_rust::agentic::AgentStream`).

Both agents now support:
1. Synchronous conversational `chat(message: String) -> ChatResponse`
2. Streaming `chat_stream(message: String) -> AgentStream<ChatChunk>` with real-time tool start, tool result, text delta, and completion events
3. Durable conversation history queries via `get_history() -> Vec<ChatMessage>` (`#[read_only]`)
4. Conversation history reset via `clear_history()`
5. Execution of all corresponding agent tools (16 shopping assistant tools and 14 store manager tools) with full state persistence across turns and backward-compatible snapshot recovery

---

## 2. Changes Implemented

### File Modifications

| Action | File Path | Summary of Changes |
| :--- | :--- | :--- |
| `[NEW]` | [`src/ai/schema.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/ai/schema.rs) | Golem `Schema` models: `ChatRole`, `ToolCallRecord`, `ChatMessageType`, `ChatMessage`, `ChatResponse`, `ChatChunk` |
| `[NEW]` | [`src/ai/agent_loop.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/ai/agent_loop.rs) | Custom LLM loop, OpenAI/Gemini/OpenRouter endpoint resolution, multi-turn tool execution, and chunk emission |
| `[NEW]` | [`src/ai/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/ai/mod.rs) | AI module exports and unit test suite |
| `[NEW]` | [`src/shopping_assistant/tools.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/shopping_assistant/tools.rs) | 16 shopping assistant tool definitions, JSON schemas, system prompt, and async execution dispatcher |
| `[MODIFY]` | [`src/shopping_assistant/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/shopping_assistant/mod.rs) | Added `chat`, `chat_stream`, `get_history`, `clear_history` to `ShoppingAssistantAgent`; wired custom loop; updated snapshots with backward compatibility |
| `[NEW]` | [`src/manager/tools.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/manager/tools.rs) | 14 store manager tool definitions, JSON schemas, manager system prompt, and async execution dispatcher |
| `[NEW]` | [`src/manager/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/manager/mod.rs) | `ManagerAgent` trait and implementation mounted at `/v1/manager/{id}` with `chat`, `chat_stream`, `get_history`, `clear_history`, and snapshots |
| `[MODIFY]` | [`src/lib.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/lib.rs) | Re-exported `ai`, `manager`, and `web` modules |
| `[NEW]` | [`src/web/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/web/mod.rs) | `WebAssistant` (mounted at `/assistant`) and `WebManager` (mounted at `/manager`) serving HTML and static assets via WASI filesystem and embedded fallbacks |
| `[MODIFY]` | [`web/assets/chat-common.js`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/assets/chat-common.js) | Added `normalizeChunk` and `normalizeToolCall` to seamlessly handle both Effect and Rust SSE stream formats and tool records |
| `[MODIFY]` | [`web/assets/assistant.js`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/assets/assistant.js) | Integrated normalized chunk and tool parsing; support for `toolCallsUsed`/`tool_calls_used` and `productIds`/`product_ids` |
| `[MODIFY]` | [`web/assets/manager.js`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/assets/manager.js) | Integrated normalized chunk and tool parsing; support for `toolCallsUsed`/`tool_calls_used` |
| `[MODIFY]` | [`web/assistant.html`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/assistant.html) | Updated header subtitle |
| `[MODIFY]` | [`web/manager.html`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/manager.html) | Updated header subtitle |
| `[MODIFY]` | [`golem.yaml`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/golem.yaml) | Mounted `/web` files to component and registered `ManagerAgent`, `WebAssistant`, and `WebManager` under `httpApi.deployments.local[0].agents` |

### Key Logic & API Changes

1. **Custom Agent Loop & Session Helper (`src/ai/agent_loop.rs`)**:
   - Handles multi-turn tool calling up to `MAX_LOOP_TURNS = 10`.
   - Normalizes provider URLs for OpenRouter, Gemini, and custom OpenAI-compatible endpoints.
   - Automatically handles missing API keys without panicking, returning clear user-facing guidance.
   - Provides consolidated `process_chat_turn` helper and `ChatTurnResult::to_response()` / `ChatTurnResult::to_stream()` adapters, eliminating duplicate message-recording, config-extraction, and stream-spawning logic across `ShoppingAssistantAgent` and `ManagerAgent`.
   - Centralized `AgentConfig` and `LlmConfig` in `src/ai/schema.rs`.
2. **Streaming Execution (`AgentStream<ChatChunk>`)**:
   - `ChatChunk` variants: `TextDelta`, `ToolStart`, `ToolResult`, `Done`, `Error`.
   - Uses `golem_rust::agentic::{AgentStream, spawn_local}` for asynchronous backpressure-aware event streaming.
3. **Shopping Assistant Tools**:
   - `search_products`, `get_product_details`, `check_inventory`, `get_cart`, `add_to_cart`, `update_cart_quantity`, `remove_from_cart`, `clear_cart`, `get_saved_contact_info`, `apply_saved_contact_info`, `set_shipping_address`, `set_billing_address`, `set_email`, `checkout_cart`, `get_order_history`, `get_order_details`.
4. **Manager Agent Tools**:
   - `search_orders`, `list_recent_orders`, `get_order_details`, `ship_order`, `cancel_order`, `add_order_item`, `remove_order_item`, `update_order_item_quantity`, `update_order_shipping_address`, `update_order_billing_address`, `update_order_email`, `check_product_details`, `check_inventory`, `search_products`.
5. **Web Static Serving & UI Compatibility**:
   - `WebAssistant` ephemeral agent mounted at `/assistant`, exposing `index` (`/`) and `asset` (`/assets/{*path}`).
   - `WebManager` ephemeral agent mounted at `/manager`, exposing `index` (`/`) and `asset` (`/assets/{*path}`).
   - Files are loaded dynamically from `/web/` via Golem IFS mount in `golem.yaml`, with automatic fallback to compile-time embedded assets (`include_bytes!`).
   - `web/assets/chat-common.js` normalizes stream chunks (`normalizeChunk`) across both TypeScript/Effect formats (`{ _tag: "text_delta", content }`) and Rust enum serde representation (`{ TextDelta: { content } }`), as well as normalizing camelCase vs snake_case fields (`toolCallsUsed` vs `tool_calls_used`, `productIds` vs `product_ids`).

---

## 3. Verification & Validation Results

### 3.1 Code Formatting

```bash
cargo fmt --check
```

**Result**: Passed (exited with code 0).

### 3.2 Compilation & Type Checking

```bash
cargo check --target wasm32-wasip2
```

**Result**: Passed (exited with code 0, 0 compilation errors).

### 3.3 Linter Verification (Clippy)

```bash
cargo clippy --target wasm32-wasip2
```

**Result**: Passed (exited with code 0, 0 errors).

### 3.4 Automated Unit Tests

```bash
cargo test
```

**Result**:
```text
running 14 tests
test cart::tests::test_saved_contact_info_construction ... ok
test ai::tests::test_resolve_endpoint_url ... ok
test ai::tests::test_missing_api_key_returns_graceful_error ... ok
test inventory::tests::test_reserve_over_limit_fails ... ok
test inventory::tests::test_availability_calculation ... ok
test ai::tests::test_chat_schema_serialization ... ok
test ai::tests::test_tool_call_record_and_chunk_serialization ... ok
test order_search::tests::test_query_matcher_quoted_terms ... ok
test order_search::tests::test_query_matcher_field_filters ... ok
test order_search::tests::test_query_matcher_terms ... ok
test product_search::tests::test_query_matcher_terms ... ok
test inventory::tests::test_release_and_fulfill ... ok
test product_search::tests::test_extract_product_agent_id ... ok
test order_search::tests::test_extract_order_agent_id ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 3.5 Golem Build

```bash
golem build --yes
```

**Result**:
```text
Selecting components
  Found components: shopping-rust:shopping
  Selected components and layers:
    shopping-rust:shopping: rust, rust[release], shopping-rust:shopping
Building components
  Building shopping-rust:shopping
    Executing external command 'cargo build --target wasm32-wasip2 --release'
         in directory /Users/coon/workspace-zv/git/golem-shopping-rust
 cargo │     Finished `release` profile [optimized] target(s) in 52.51s
  Adding metadata to shopping-rust:shopping

Finished building [OK]
```

---

## 4. How to Run & Verify

1. **Build and Deploy Application**:
   ```bash
   golem build --yes
   ./deploy.sh # or golem deploy --yes
   ```
2. **Shopping Assistant Chat**:
   ```bash
   # Synchronous chat:
   curl -X POST http://localhost:9006/v1/assistant/user123/chat \
     -H "Content-Type: application/json" \
     -d '{"message": "Can you recommend some running shoes?"}'

   # Conversation history:
   curl http://localhost:9006/v1/assistant/user123/history

   # Clear history:
   curl -X POST http://localhost:9006/v1/assistant/user123/clear-history
   ```
3. **Manager Agent Chat**:
   ```bash
   # Synchronous manager chat:
   curl -X POST http://localhost:9006/v1/manager/mgr1/chat \
     -H "Content-Type: application/json" \
     -d '{"message": "List the most recent orders across the store"}'

   # Conversation history:
   curl http://localhost:9006/v1/manager/mgr1/history

   # Clear history:
   curl -X POST http://localhost:9006/v1/manager/mgr1/clear-history
   ```

---

## 5. Sign-off / Next Steps

Implementation and validation are complete and verified against `wasm32-wasip2` and Golem build targets.
Awaiting user review and approval.

