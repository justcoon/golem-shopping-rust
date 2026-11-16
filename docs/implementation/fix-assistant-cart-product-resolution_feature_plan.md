# Feature Plan: Fix Assistant Cart Product Resolution & History Retention

- **Feature ID / Slug**: `fix-assistant-cart-product-resolution`
- **Date**: 2026-11-16
- **Status**: Completed

---

## 1. Overview & Goals

When a customer uses the Shopping Assistant to find an item (for example, saying *"i want to buy airpods"*), the assistant correctly executes `search_products(query="airpods")` and identifies Apple AirPods Pro (2nd Gen) (`ap003`). The assistant replies with the product description and price. However, when the user subsequently says *"add it to cart"*, the assistant frequently adds a completely different product—specifically an **iPhone 15 Pro** (`ap001`)—to the shopping cart.

### Root Cause
1. **Tool History Stripping in Agent Loop**:
   In `src/ai/agent_loop.rs` (`execute_agent_chat_turn` and `execute_agent_chat_stream`), when converting stored conversation `history: &[ChatMessage]` to OpenAI/Gemini message payloads for a new turn, all assistant `tool_calls` are wiped out (`tool_calls: None`) and tool response messages (`role: "tool"`) are omitted. Consequently, in Turn 2, the LLM has zero knowledge of the previous search output or the product ID `ap003`.
2. **Schema Example Priming & Hallucination**:
   Because the LLM does not have `ap003` in context and the `add_to_cart` tool requires `productId: String`, the LLM looks at the available tools. In `src/tools/catalog.rs`, the schema documentation for `GetProductDetailsInput` specifies `/// Unique product identifier, e.g. 'ap001'`. Primed by this example or guessing the first product identifier, the LLM passes `"ap001"`, which in our store database is the **iPhone 15 Pro**.
3. **Missing System Prompt Guardrails**:
   The `SHOPPING_ASSISTANT_SYSTEM_PROMPT` in `src/shopping_assistant/tools.rs` does not explicitly instruct the model to look up product IDs via `search_products` if they are not already known with certainty, allowing the model to make ungrounded guesses.

### Goals
- Retain tool call records and their tool responses across conversation turns in `src/ai/agent_loop.rs` while strictly adhering to OpenAI and Gemini chat message formatting rules.
- Eliminate hardcoded example IDs (`ap001`) from tool schema doc comments in `src/tools/catalog.rs`.
- Enhance `SHOPPING_ASSISTANT_SYSTEM_PROMPT` in `src/shopping_assistant/tools.rs` with explicit rules for resolving product IDs before invoking `add_to_cart`.
- Add unit tests verifying multi-turn tool message reconstruction in `src/ai/agent_loop.rs`.

### Non-Goals
- Changing the cart agent state machine or checkout flow.
- Modifying the frontend Vue UI (which already supports displaying tool calls when present).

---

## 2. Architecture & Technical Design

### Message History Reconstruction
In `src/ai/agent_loop.rs`, extract a shared helper:
```rust
fn build_initial_messages(
    system_prompt: &str,
    history: &[ChatMessage],
    user_message: &str,
    is_gemini: bool,
) -> Vec<OpenAiMessage>
```
When iterating `history`:
- **`ChatRole::User`**: pushed as an `OpenAiMessage` with `role: "user"`.
- **`ChatRole::Assistant`**:
  - If `h.tool_calls` contains records (`ToolCallRecord`):
    1. Push an `OpenAiMessage` with `role: "assistant"`, `content: None`, and `tool_calls: Some(open_ai_tool_calls)`. (If `is_gemini` is true, invoke `ensure_gemini_thought_signature`).
    2. For each tool call record, push an `OpenAiMessage` with `role: "tool"`, `tool_call_id: Some(record.id)`, and `content: Some(record.result)`.
    3. If `!h.content.is_empty()`, push an `OpenAiMessage` with `role: "assistant"`, `content: Some(h.content)`, and `tool_calls: None`.
  - If `h.tool_calls` is empty or `None`:
    Push a single `OpenAiMessage` with `role: "assistant"`, `content: Some(h.content)`, and `tool_calls: None`.
- **`ChatRole::System`**: pushed as an `OpenAiMessage` with `role: "system"`.
- Finally, append the new `user_message` as `role: "user"`.

### Consistency Between Turn Execution and Streaming
Both `execute_agent_chat_turn` and `execute_agent_chat_stream` will utilize this unified helper, ensuring identical history behavior whether the client calls `/chat` or `/chat-stream`.

### Schema & Prompt Refinements
- In `src/tools/catalog.rs`:
  Change `/// Unique product identifier, e.g. 'ap001'` to `/// Unique product identifier from catalog search`.
- In `src/shopping_assistant/tools.rs`:
  Add clear instructions under Guidelines:
  - When adding items to cart or updating quantities, always use the exact `productId` from previous search results or product details in the conversation.
  - If the user refers to an item (e.g. *"add it to cart"*) and its `productId` is not known from previous turns, call `search_products` first to resolve the item. Never hallucinate or guess product IDs.

---

## 3. Proposed Changes & File Impact

| Action     | File Path                                                                                   | Description                                                                                                     |
| :--------- | :------------------------------------------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------------- |
| `[MODIFY]` | [`src/ai/agent_loop.rs`](../../src/ai/agent_loop.rs)                                         | Add `build_initial_messages` helper to reconstruct tool calls/results in history; add comprehensive unit tests. |
| `[MODIFY]` | [`src/shopping_assistant/tools.rs`](../../src/shopping_assistant/tools.rs)                   | Update system prompt guidelines and `AddToCartArgs` description regarding `productId` resolution.               |
| `[MODIFY]` | [`src/tools/catalog.rs`](../../src/tools/catalog.rs)                                       | Remove hardcoded `'ap001'` example from `GetProductDetailsInput` documentation.                                 |

---

## 4. Verification Plan

### Automated Checks
1. **Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Type Checking & WASM Build**:
   ```bash
   cargo check --target wasm32-wasip2
   cargo clippy --target wasm32-wasip2
   ```
3. **Golem Component Compilation**:
   ```bash
   golem build --yes
   ```
4. **Unit Tests**:
   ```bash
   cargo test
   ```
   Specifically verifies new and existing unit tests in `src/ai/agent_loop.rs` for history reconstruction with tool calls.

---

## 5. Risks & Open Questions

- **Token Length in Long Conversations**: Re-including tool outputs in context consumes additional tokens. Because `MAX_LOOP_TURNS = 10` is bounded and typical shopping search outputs are compact JSON lists, this is standard practice and well within model context limits (128k+ tokens for Gemini/GPT-4o).
- **Gemini Thought Signature Requirement**: Handled by ensuring `ensure_gemini_thought_signature` is applied to reconstructed tool calls when running against Gemini endpoints.
