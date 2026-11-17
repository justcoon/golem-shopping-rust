# Implementation Walkthrough: Fix Assistant Cart Product Resolution & History Retention

- **Feature ID / Slug**: `fix-assistant-cart-product-resolution`
- **Date**: 2026-11-16
- **Status**: Completed

---

## 1. Executive Summary

When customers asked the Shopping Assistant to find a product (such as *"i want to buy airpods"*), the assistant successfully searched the catalog and returned the AirPods Pro (`ap003`). However, upon the customer's subsequent message *"add it to cart"*, an **iPhone 15 Pro** (`ap001`) was added instead.

This issue has been resolved by:
1. Reconstructing tool call records and their raw results in the conversational history sent to the LLM on subsequent turns.
2. Eliminating the hardcoded example product ID (`ap001`) from tool schema doc comments.
3. Adding explicit fallback search and ground-truth product ID resolution rules to the shopping assistant system prompt.

---

## 2. Key Changes by Component

### 1. Agent Loop & History Reconstruction (`src/ai/agent_loop.rs`)
- Added `build_initial_messages` to unify message reconstruction across synchronous (`execute_agent_chat_turn`) and streaming (`execute_agent_chat_stream`) chat loops.
- When an assistant turn has `tool_calls: Some(records)`:
  - Generates an `OpenAiMessage` with `role: "assistant"`, `content: None`, and `tool_calls: Some(tcs)`. Ensures Gemini `thought_signature` validator is satisfied when targeting Gemini models.
  - Generates `OpenAiMessage`s with `role: "tool"`, `tool_call_id: Some(record.id)`, and `content: Some(record.result)`.
  - Appends the assistant's final text reply as an `OpenAiMessage` with `role: "assistant"` if non-empty.
- When an assistant turn does not contain tool calls, emits a standard assistant text message.
- Added unit tests:
  - `test_build_initial_messages_preserves_tool_calls_and_results`: Validates that search outputs with product IDs (like `ap003`) and Gemini thought signatures are preserved in multi-turn sequences.
  - `test_build_initial_messages_plain_history_without_tools`: Validates standard text-only chat history.

### 2. Catalog Tool Schema (`src/tools/catalog.rs`)
- Replaced `/// Unique product identifier, e.g. 'ap001'` with `/// Unique product identifier from catalog search` on `GetProductDetailsInput`.
- This removes the misleading `ap001` prompt priming that caused models to default to the iPhone 15 Pro.

### 3. Shopping Assistant System Prompt & Tool Args (`src/shopping_assistant/tools.rs`)
- Streamlined `SHOPPING_ASSISTANT_SYSTEM_PROMPT` into a concise, structured markdown format:
  - Removed the redundant inline listing of tools (since tool signatures are already declared directly via the OpenAI/Gemini tools API).
  - Explicitly stated **Core Rules**: concise styling, product mentions with name/brand/price, strict IDs (never guess or invent IDs), and locked carts (`cancel_checkout` required before modifications).
  - Structured the two-phase checkout workflow (Prepare & Review with 10-minute hold vs. explicit Confirm).
- Updated `AddToCartArgs.product_id` doc comment to clarify that it must be obtained from catalog search.

---

## 3. Verification Results

### Automated Validation

1. **Code Formatting (`cargo fmt`)**:
   - Status: **PASSED** (compliant with Rust formatting guidelines).

2. **WASM Compilation & Type Checking (`cargo check --target wasm32-wasip2`)**:
   - Status: **PASSED** (0 errors).

3. **Linter (`cargo clippy --target wasm32-wasip2`)**:
   - Status: **PASSED** (0 warnings on modified code).

4. **Component Release Build (`cargo build --target wasm32-wasip2 --release`)**:
   - Status: **PASSED** (compiled `shopping_rust_shopping.wasm`).

5. **Unit Tests (`cargo test`)**:
   - Status: **PASSED** (all 36 tests passed, including new multi-turn tool history reconstruction tests):
     - `test_build_initial_messages_preserves_tool_calls_and_results ... ok`
     - `test_build_initial_messages_plain_history_without_tools ... ok`

---

## 4. User Review & Manual Verification

To test the assistant behavior:
1. Start a chat session with the assistant:
   - Message 1: *"I want to buy airpods"*
   - The assistant calls `search_products` and mentions Apple AirPods Pro (2nd Gen).
2. Follow up in the same session:
   - Message 2: *"Add it to cart"*
   - The assistant now retains the `ap003` product ID from the search result in context and calls `add_to_cart(productId: "ap003")`.
3. Check the cart items:
   - Only the AirPods Pro (`ap003`) is added; no unintended products (`ap001`) are added.
