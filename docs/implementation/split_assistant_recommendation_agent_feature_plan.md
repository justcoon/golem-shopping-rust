# Feature Plan: Split `ShoppingAssistantAgent` into Chat and Recommendation Agents

- **Feature ID / Slug**: `split-assistant-recommendation-agent`
- **Date**: 2026-11-06
- **Status**: Draft <!-- Draft | Approved | In Progress | Completed -->

---

## 1. Overview & Goals

Currently, [`ShoppingAssistantAgent`](../../src/shopping_assistant/mod.rs) handles two distinct responsibilities:
1. **Interactive Conversational AI Assistant**: Multi-turn dialogue, real-time SSE streaming (`/chat-stream`), tool execution (`search_products`, `add_to_cart`, `prepare_checkout`, `confirm_checkout`, etc.), and message history management.
2. **Personalized Product & Brand Recommendations**: Asynchronous background computation of recommendations (`recommend_items`) based on user purchase history via LLM, and caching of `RecommendedItems` in agent state (`get_recommended_items`).

### Problems Solved by the Split
- **Separation of Concerns**: Chat session state (`messages`) and recommendation state (`recommended_items`) have completely different access patterns, update frequencies, and lifecycles.
- **Uncoupled Latency & Concurrency**: Chat requires low-latency, interactive response times with SSE streaming. Recomputing recommendations involves pulling order history across multiple agents and executing structured LLM inference, which should execute asynchronously without contention on the chat agent's locks or state.
- **Direct Domain Integration**: `CartAgent` triggers recommendation recalculation upon checkout confirmation (`trigger_recommend_items`). It should talk directly to a dedicated recommendation agent rather than the chat assistant.
- **Cleaner Snapshotting & Storage**: Each agent only persists state relevant to its domain (`messages` for chat, `recommended_items` for recommendations).

### Goals
- Split `ShoppingAssistantAgent` into:
  - **`ShoppingAssistantAgent`**: Dedicated to interactive conversational chat, streaming, history, and tool execution (`/v1/assistant/{id}`).
  - **`ShoppingRecommendationAgent`**: Dedicated to personalized recommendations (`/v1/recommendation/{id}`).
- Update `CartAgent` to trigger `ShoppingRecommendationAgentClient::get(user_id).trigger_recommend_items()` upon successful checkout confirmation.
- Add `ShoppingRecommendationAgent` to `golem.yaml` under `httpApi.deployments.local.agents`.
- Optionally provide a `get_recommendations` tool in the chat assistant so the LLM can recommend items directly during chat conversations.
- Update the web client (`web/assets/assistant.js`) and storefront frontend (`frontend/src/api/services/assistantService.ts`) to fetch recommendations from `/v1/recommendation/{id}`.
- Update documentation and `architecture.puml`.

---

## 2. Architecture & Technical Design

```mermaid
flowchart TD
    subgraph Client Surfaces
        WebClient["Web UI (/assistant)"]
        FrontendStore["Frontend Storefront (Vue 3)"]
        User["Customer / Shopper"]
    end

    subgraph Golem API Gateway
        Gateway["API Gateway (localhost:9006)"]
    end

    subgraph Domain Agents
        Cart["CartAgent (/v1/cart/{id})"]
        Order["OrderAgent (/v1/order/{id})"]
        Product["ProductAgent & Search"]
    end

    subgraph Split AI Agents
        Assistant["ShoppingAssistantAgent<br/>/v1/assistant/{id}<br/>• /chat, /chat-stream<br/>• /history, /clear-history<br/>• Tool Dispatcher"]
        Recommendation["ShoppingRecommendationAgent<br/>/v1/recommendation/{id}<br/>• /recommended-items (GET)<br/>• Persisted RecommendedItems"]
    end

    subgraph External LLM
        LLM["Google Gemini / OpenRouter API"]
    end

    User --> Gateway
    WebClient --> Gateway
    FrontendStore --> Gateway

    Gateway -->|/v1/assistant/*| Assistant
    Gateway -->|/v1/recommendation/*| Recommendation
    Gateway -->|/v1/cart/*| Cart

    Cart -->|on confirm_checkout: trigger_recommend_items()| Recommendation

    Assistant -->|Tool Calling| Cart
    Assistant -->|Tool Calling| Product
    Assistant -->|Tool Calling (optional)| Recommendation
    Assistant -->|Chat Completion| LLM

    Recommendation -->|Get Order History| Cart
    Recommendation -->|Get Order Items| Order
    Recommendation -->|Generate Recommendations| LLM
```

### Agent 1: `ShoppingAssistantAgent` (Chat Agent)
- **Mount Path**: `/v1/assistant/{id}`
- **Trait Definition**:
  ```rust
  #[agent_definition(mount = "/v1/assistant/{id}")]
  pub trait ShoppingAssistantAgent {
      fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self;

      #[endpoint(post = "/chat")]
      async fn chat(&mut self, message: String) -> ChatResponse;

      #[endpoint(post = "/chat-stream")]
      async fn chat_stream(&mut self, message: String) -> AgentStream<ChatChunk>;

      #[read_only]
      #[endpoint(get = "/history")]
      fn get_history(&self) -> Vec<ChatMessage>;

      #[endpoint(post = "/clear-history")]
      fn clear_history(&mut self);
  }
  ```
- **State**:
  ```rust
  pub struct ShoppingAssistantAgentImpl {
      _id: String,
      config: Config<AgentConfig>,
      messages: Arc<Mutex<Vec<ChatMessage>>>,
  }
  ```
- **Snapshotting**:
  Saves and restores `Vec<ChatMessage>`, with backward-compatible deserialization support for historical snapshots.

### Agent 2: `ShoppingRecommendationAgent` (Recommendation Agent)
- **Mount Path**: `/v1/recommendation/{id}`
- **Trait Definition**:
  ```rust
  #[agent_definition(mount = "/v1/recommendation/{id}")]
  pub trait ShoppingRecommendationAgent {
      fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self;

      #[read_only]
      #[endpoint(get = "/recommended-items")]
      fn get_recommended_items(&self) -> RecommendedItems;

      // Internal RPC only (triggered by CartAgent on order confirmation)
      async fn recommend_items(&mut self) -> bool;
  }
  ```
- **State**:
  ```rust
  pub struct ShoppingRecommendationAgentImpl {
      _id: String,
      config: Config<AgentConfig>,
      recommended_items: RecommendedItems,
  }
  ```
- **Data Models**:
  - `RecommendedItems`: `{ product_ids: Vec<String>, product_brands: Vec<String>, updated_at: DateTime<Utc> }`
  - `LlmOrderItem` and `LlmRecommendedItems` for structured LLM prompting and JSON extraction.
- **Snapshotting**:
  Saves and restores `RecommendedItems`.

---

## 3. Proposed Changes & File Impact

| Action | File Path | Description |
| :--- | :--- | :--- |
| `[NEW]` | [`src/recommendation/mod.rs`](../../src/recommendation/mod.rs) | Implement `ShoppingRecommendationAgent`, recommendation models, and LLM synthesis logic |
| `[MODIFY]` | [`src/lib.rs`](../../src/lib.rs) | Export `pub mod recommendation;` |
| `[MODIFY]` | [`src/shopping_assistant/mod.rs`](../../src/shopping_assistant/mod.rs) | Remove recommendation methods & state; focus strictly on chat & history |
| `[MODIFY]` | [`src/shopping_assistant/tools.rs`](../../src/shopping_assistant/tools.rs) | Optional: add `get_recommendations` tool so the assistant can query recommendations |
| `[MODIFY]` | [`src/cart/mod.rs`](../../src/cart/mod.rs) | Update post-checkout trigger to `ShoppingRecommendationAgentClient::get(user_id).trigger_recommend_items()` |
| `[MODIFY]` | [`golem.yaml`](../../golem.yaml) | Add `ShoppingRecommendationAgent: {}` under `httpApi.deployments.local.agents` |
| `[MODIFY]` | [`web/assets/assistant.js`](../../web/assets/assistant.js) | Update `loadRecommendations()` to fetch from `/v1/recommendation/${encodeURIComponent(currentUserId)}` |
| `[MODIFY]` | [`frontend/src/api/services/assistantService.ts`](../../frontend/src/api/services/assistantService.ts) | Update recommendation API call to `/v1/recommendation/${userId}` |
| `[MODIFY]` | [`architecture.puml`](../../architecture.puml) | Add `Shopping Recommendation Agent` component and RPC connections |
| `[MODIFY]` | [`README.md`](../../README.md) | Document the new `Shopping Recommendation Agent` |

---

## 4. Step-by-Step Implementation Plan

### Step 1: Create `src/recommendation/mod.rs`
1. Define `ShoppingRecommendationAgent` trait with:
   - `fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self;`
   - `#[read_only] #[endpoint(get = "/recommended-items")] fn get_recommended_items(&self) -> RecommendedItems;`
   - `async fn recommend_items(&mut self) -> bool;` (Internal RPC method, no HTTP exposure)
2. Move helper functions from `shopping_assistant/mod.rs`:
   - `get_order_items(id: String) -> Vec<OrderItem>`
   - `reduce_order_items(items: Vec<OrderItem>) -> Vec<OrderItem>`
   - `get_llm_recommendations(items: Vec<OrderItem>, config: LlmConfig) -> Result<LlmRecommendedItems, String>`
3. Move structs `LlmOrderItem`, `LlmRecommendedItems`, and `RecommendedItems`.
4. Implement snapshot save/load for `RecommendedItems`.
5. Export `pub mod recommendation;` in `src/lib.rs`.

### Step 2: Clean Up `src/shopping_assistant/mod.rs`
1. Remove `RECOMMENDATION_*` constants, `get_order_items`, `reduce_order_items`, and `get_llm_recommendations`.
2. Remove `recommended_items` from `ShoppingAssistantAgent` trait and `ShoppingAssistantAgentImpl`.
3. Simplify `ShoppingAssistantSnapshot` to persist only `messages: Vec<ChatMessage>`. Provide backward-compatible snapshot deserialization.

### Step 3: Wire Cart Confirmation to Recommendation Agent
In `src/cart/mod.rs`:
```rust
// Replace:
// ShoppingAssistantAgentClient::get(user_id.clone()).trigger_recommend_items();
// With:
crate::recommendation::ShoppingRecommendationAgentClient::get(user_id.clone()).trigger_recommend_items();
```

### Step 4: Update `golem.yaml`
Add `ShoppingRecommendationAgent` to the HTTP API routing:
```yaml
httpApi:
  deployments:
    local:
    - domain: localhost:9006
      agents:
        ShoppingAssistantAgent: {}
        ShoppingRecommendationAgent: {}
```

### Step 5: Update Web Client and Frontend Service
1. In `web/assets/assistant.js`:
   Update `/v1/assistant/${encodeURIComponent(currentUserId)}/recommended-items` to `/v1/recommendation/${encodeURIComponent(currentUserId)}/recommended-items`.
2. In `frontend/src/api/services/assistantService.ts`:
   Update `/v1/assistant/${userId}/recommended-items` to `/v1/recommendation/${userId}/recommended-items`.

### Step 6: Update Documentation & Architecture
1. In `architecture.puml`:
   Add `[Shopping Recommendation Agent] as recommendation`.
   Add arrows:
   - `cart --> recommendation : "RPC\nTrigger recommendations on order"`
   - `recommendation --> cart : "RPC\nGet order history"`
   - `recommendation --> order : "RPC\nGet order items"`
   - `recommendation --> llm : "API Call"`
2. In `README.md`:
   Add `Shopping Recommendation Agent` to the architecture agent list.

---

## 5. Verification Plan

### Automated Checks
1. **Compilation / Type Check**:
   ```bash
   cargo check --target wasm32-wasip2
   ```
2. **Unit Tests**:
   ```bash
   cargo test
   ```
   (Verify all 26+ tests pass, including order reduction and LLM JSON parsing tests).
3. **Clippy Linting**:
   ```bash
   cargo clippy --target wasm32-wasip2
   ```
4. **Code Formatting**:
   ```bash
   cargo fmt --check
   ```
5. **Benchmark Check**:
   ```bash
   cargo check --manifest-path benchmark/Cargo.toml
   ```
6. **Frontend Build**:
   ```bash
   cd frontend && npx vite build
   ```

---

## 6. Open Questions / Confirmations for the User

1. **Naming**:
   The plan proposes the standard Rust identifier `ShoppingRecommendationAgent` with module name `src/recommendation/mod.rs` and mount `/v1/recommendation/{id}`. (Your prompt typed `ShoppingRecommendetationAgent`). Is `ShoppingRecommendationAgent` acceptable?
2. **Endpoint Path for Recommendations**:
   Proposed endpoint: `GET /v1/recommendation/{id}` (and `POST /v1/recommendation/{id}/recommend`). Would you also like an alias `GET /v1/recommendation/{id}/recommended-items`?
3. **Chat Assistant Tool**:
   Would you like the `ShoppingAssistantAgent` to have a `get_recommendations` tool so the user can ask the chat assistant for recommendations in dialogue (e.g. *"What do you recommend for me?"*)?

