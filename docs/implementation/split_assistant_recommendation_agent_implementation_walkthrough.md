# Implementation Walkthrough: Split Shopping Assistant & Recommendation Agents

## 1. Executive Summary & Objective
- **Objective**: Decouple recommendation generation and serving from conversational chat by splitting `ShoppingAssistantAgent` into:
  - `ShoppingAssistantAgent`: Dedicated strictly to multi-turn conversational chat, SSE streaming, and tool execution (`/v1/assistant/{id}`).
  - `ShoppingRecommendationAgent`: Dedicated to computing and persisting personalized product/brand recommendations (`/v1/recommendation/{id}`).
- **Key Design Decisions**:
  - `recommend_items(&mut self) -> bool` is an internal agent method (Golem RPC, no `#[endpoint]` attribute). It is invoked asynchronously via `ShoppingRecommendationAgentClient::get(user_id).trigger_recommend_items()` when `CartAgent` confirms checkout.
  - Public HTTP read endpoint: `GET /v1/recommendation/{id}/recommended-items` returning `RecommendedItems`.
  - Full snapshot backward compatibility preserved in `ShoppingAssistantAgent`.

---

## 2. Architecture & Design Changes

### Agent Separation
```
+-------------------------------------------------------------+
|                         API Gateway                         |
+------------------------------+------------------------------+
                               |
            +------------------+------------------+
            |                                     |
            v                                     v
+-----------------------+             +-----------------------+
| ShoppingAssistantAgent|             |ShoppingRecommendation |
| /v1/assistant/{id}    |             | Agent                 |
|                       |             | /v1/recommendation/   |
| - Chat & SSE stream   |             |   {id}                |
| - Message history     |             | - GET /recommended-  |
| - Multi-turn tools    |             |   items               |
+-----------------------+             | - recommend_items()   |
                                      |   (internal RPC)      |
                                      +-----------------------+
                                                  ^
                                                  | (trigger_recommend_items)
                                      +-----------+-----------+
                                      |       CartAgent       |
                                      | (on confirm_checkout) |
                                      +-----------------------+
```

---

## 3. Detailed Code Modifications

### 1. `src/recommendation/mod.rs` (New Component)
- Defined `ShoppingRecommendationAgent` trait mounted at `/v1/recommendation/{id}`.
- Added `get_recommended_items(&self) -> RecommendedItems` annotated with `#[read_only]` and `#[endpoint(get = "/recommended-items")]`.
- Added internal method `async fn recommend_items(&mut self) -> bool` without `#[endpoint]` (internal Golem RPC only).
- Implemented state persistence via `ShoppingRecommendationSnapshot` with `save_snapshot` and `load_snapshot`.
- Implemented `reduce_order_items`, `get_order_items`, and `get_llm_recommendations`.
- Included unit tests `test_reduce_order_items` and `test_llm_recommended_items_deserialization`.

### 2. `src/lib.rs`
- Exported `pub mod recommendation;`.

### 3. `src/shopping_assistant/mod.rs`
- Stripped all recommendation logic, data models (`RecommendedItems`, `LlmOrderItem`, etc.), and recommendation endpoints.
- Maintained backward compatibility in `load_snapshot` using `LegacyShoppingAssistantSnapshot` to migrate legacy snapshots containing `recommended_items`.

### 4. `src/cart/mod.rs`
- Replaced `ShoppingAssistantAgentClient` import with `use crate::recommendation::ShoppingRecommendationAgentClient;`.
- Updated line 767 in `confirm_checkout` to trigger recommendations on `ShoppingRecommendationAgentClient`:
  ```rust
  ShoppingRecommendationAgentClient::get(user_id.clone()).trigger_recommend_items();
  ```

### 5. `golem.yaml`
- Added `ShoppingRecommendationAgent: {}` under `httpApi.deployments.local.agents`.

### 6. Frontend & Web Assets
- `web/assets/assistant.js`: Updated recommendation fetch to `/v1/recommendation/${encodeURIComponent(currentUserId)}/recommended-items`.
- `frontend/src/api/services/assistantService.ts`: Updated `getRecommendedProducts` to fetch from `/v1/recommendation/${userId}/recommended-items`.

### 7. Documentation & Architecture
- `architecture.puml`: Added `Shopping Recommendation Agent` component, relationships (`cart --> recommendation`, `recommendation --> order/product_search/llm`, `gateway --> recommendation`), and architectural notes.
- `README.md`: Added `Shopping Recommendation Agent` to the architecture component list.

---

## 4. Verification & Validation Evidence

### Cargo Check (`wasm32-wasip2`)
```bash
cargo check --target wasm32-wasip2
```
```
    Checking shopping_rust_shopping v0.0.1 (/Users/coon/workspace-zv/git/golem-shopping-rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.98s
```

### Cargo Test
```bash
cargo test
```
```
running 28 tests
test ai::agent_loop::tests::test_extract_json_str ... ok
test ai::tests::test_resolve_endpoint_url ... ok
test ai::agent_loop::tests::test_streamed_tool_call_builder ... ok
test ai::agent_loop::tests::test_sse_decoder ... ok
test cart::tests::test_saved_contact_info_construction ... ok
test cart::tests::test_cart_draft_expiration ... ok
test ai::tests::test_missing_api_key_returns_graceful_error ... ok
test ai::tests::test_tool_call_record_and_chunk_serialization ... ok
test ai::agent_loop::tests::test_noop_dispatcher ... ok
test cart::tests::test_validate_checkout_draft ... ok
test cart::tests::test_cart_draft_lifecycle_and_locking ... ok
test ai::tests::test_chat_schema_serialization ... ok
test ai::agent_loop::tests::test_gemini_thought_signature_fallback_when_missing ... ok
test cart::tests::test_validate_prepare_checkout ... ok
test ai::agent_loop::tests::test_gemini_thought_signature_preservation ... ok
test inventory::tests::test_availability_calculation ... ok
test inventory::tests::test_reservation_expiration ... ok
test inventory::tests::test_release_and_fulfill ... ok
test inventory::tests::test_reserve_over_limit_fails ... ok
test inventory::tests::test_reserve_with_explicit_expiration ... ok
test order_search::tests::test_query_matcher_field_filters ... ok
test order_search::tests::test_query_matcher_quoted_terms ... ok
test order_search::tests::test_query_matcher_terms ... ok
test product_search::tests::test_query_matcher_terms ... ok
test recommendation::tests::test_llm_recommended_items_deserialization ... ok
test recommendation::tests::test_reduce_order_items ... ok
test product_search::tests::test_extract_product_agent_id ... ok
test order_search::tests::test_extract_order_agent_id ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### Cargo Clippy & Fmt
```bash
cargo fmt --check
# Passed with 0 formatting discrepancies

cargo clippy --target wasm32-wasip2
# Finished cleanly with 0 warnings in recommendation and shopping_assistant modules
```

### Benchmark Check
```bash
cargo check --manifest-path benchmark/Cargo.toml
```
```
    Checking benchmark v0.0.1 (/Users/coon/workspace-zv/git/golem-shopping-rust/benchmark)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.41s
```

### Frontend Build
```bash
cd frontend && npx vite build
```
```
vite v5.4.19 building for production...
✓ 765 modules transformed.
dist/index.html                   0.75 kB │ gzip:  0.42 kB
dist/assets/index-DUmwNJ6k.css   60.97 kB │ gzip: 10.61 kB
dist/assets/index-BT7ovE40.js   215.07 kB │ gzip: 76.17 kB
✓ built in 2.05s
```

---

## 5. Backward Compatibility & Migration Considerations
- **Snapshots**: Existing instances of `ShoppingAssistantAgent` with legacy snapshots containing `recommended_items` deserialize smoothly via `LegacyShoppingAssistantSnapshot`.
- **Cart Checkout Trigger**: Cart agent triggers `ShoppingRecommendationAgentClient` directly; existing carts proceed without disturbance.
- **Client Endpoints**: Web UI and Frontend services now query `/v1/recommendation/{id}/recommended-items`.

