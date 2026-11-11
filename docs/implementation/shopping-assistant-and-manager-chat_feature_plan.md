# Feature Plan: Shopping Assistant Chat & Manager Agent (Custom LLM Loop with Streaming)

- **Feature ID / Slug**: `shopping-assistant-and-manager-chat`
- **Date**: 2026-11-03
- **Status**: Draft <!-- Draft | Approved | In Progress | Completed -->

---

## 1. Overview & Goals

In [`golem-shopping-effect`](), the application features two AI-powered agents:
1. **`ShoppingAssistantAgent`**: A customer-facing shopping assistant that provides conversational product discovery, cart operations, saved contact info retrieval, 1-click checkout, and order status tracking.
2. **`ManagerAgent`**: An operations-facing store manager agent that provides order management (search, list, ship, cancel, item modification, address/email correction) and catalog/inventory auditing.

In the Effect implementation, both agents utilize `@earendil-works/pi-agent-core` to run an agent loop with tool calling, supporting both synchronous `chat` and streaming `chatStream`. In this Rust repository (`golem-shopping-rust`), there is no `pi-agent` library available. Instead, we implement a lightweight, robust, durable **custom agent loop** in pure Rust targeting `wasm32-wasip2` using `golem_rust::http` and `golem_rust::agentic::AgentStream`.

### Goals

1. **AI Core Module (`src/ai/`)**:
   - Provide shared Golem `Schema` types: `ChatRole`, `ToolCallRecord`, `ChatMessageType`, `ChatMessage`, `ChatResponse`, and streaming `ChatChunk` (compatible with the Effect schemas and HTTP API).
   - Implement OpenAI-compatible HTTP chat completion with function/tool definitions.
   - Implement a custom multi-turn tool-calling loop (`execute_agent_chat_turn`) supporting model providers (OpenRouter, Gemini OpenAI endpoint, and custom `apiUrl`).
   - Emit live `ChatChunk` events (tool start, tool result, text delta, done, error) to support streaming.
   - Gracefully handle missing or invalid API keys with informative messages instead of crashing.
2. **`ShoppingAssistantAgent` Enhancements (`src/shopping_assistant/mod.rs`)**:
   - Add conversational methods to the `ShoppingAssistantAgent` trait:
     - `chat(&mut self, message: String) -> ChatResponse` mounted at `POST /v1/assistant/{id}/chat`
     - `chat_stream(&mut self, message: String) -> AgentStream<ChatChunk>` mounted at `POST /v1/assistant/{id}/chat-stream`
     - `get_history(&self) -> Vec<ChatMessage>` mounted at `GET /v1/assistant/{id}/history` (`#[read_only]`)
     - `clear_history(&mut self)` mounted at `POST /v1/assistant/{id}/clear-history`
   - Retain existing recommendation methods: `get_recommended_items` and `recommend_items`.
   - Wire all 16 shopping assistant tools (product search, details, inventory, cart management, address setting, checkout, order history/details) into the custom loop.
   - Maintain durable conversation history across turns in agent state and snapshots.
3. **New `ManagerAgent` (`src/manager/mod.rs`)**:
   - Create a durable `ManagerAgent` mounted at `/v1/manager/{id}`.
   - Expose endpoints:
     - `chat(&mut self, message: String) -> ChatResponse` mounted at `POST /v1/manager/{id}/chat`
     - `chat_stream(&mut self, message: String) -> AgentStream<ChatChunk>` mounted at `POST /v1/manager/{id}/chat-stream`
     - `get_history(&self) -> Vec<ChatMessage>` mounted at `GET /v1/manager/{id}/history` (`#[read_only]`)
     - `clear_history(&mut self)` mounted at `POST /v1/manager/{id}/clear-history`
   - Wire all 14 store manager tools (order search, recent orders, order details, ship, cancel, modify items/quantities/addresses/email, product details, inventory check, product search) into the custom loop.
   - Maintain durable conversation history in agent state and snapshots.
4. **App Manifest & Exports (`golem.yaml`, `src/lib.rs`)**:
   - Register `ManagerAgent` in `golem.yaml` under `httpApi.deployments.local[0].agents`.
   - Re-export modules and clients in `src/lib.rs`.

### Non-Goals

- Modifying underlying business logic in `CartAgent`, `OrderAgent`, `OrderSearchAgent`, `ProductAgent`, `ProductSearchAgent`, or `InventoryAgent`.

---

## 2. Technical Design & Architecture

### 2.1 Communication Flow & Agent Loop

```mermaid
sequenceDiagram
    autonumber
    actor User as User / Client
    participant Agent as ShoppingAssistantAgent / ManagerAgent
    participant Loop as Custom Agent Loop
    participant LLM as LLM Provider (OpenRouter / Gemini)
    participant SubAgents as CartAgent / OrderAgent / ProductSearch / Inventory

    alt Synchronous Chat
        User->>Agent: POST /v1/assistant/{id}/chat {"message": "..."}
        Agent->>Loop: execute_agent_chat_turn(history, message, tools)
        Loop->>LLM: POST /chat/completions (messages + tool schemas)
        alt LLM returns Tool Calls
            LLM-->>Loop: tool_calls: [search_products, check_inventory]
            Loop->>SubAgents: RPC invocation (e.g. search_with_prices)
            SubAgents-->>Loop: Tool results
            Loop->>LLM: POST /chat/completions (tool results)
            LLM-->>Loop: Final assistant reply text
        else LLM returns Direct Answer
            LLM-->>Loop: Assistant reply text
        end
        Loop-->>Agent: ChatTurnResult (reply, tool_calls_used, chunks)
        Agent->>Agent: Append user & assistant messages to persistent state
        Agent-->>User: 200 OK ChatResponse
    else Streaming Chat
        User->>Agent: POST /v1/assistant/{id}/chat-stream {"message": "..."}
        Agent->>Loop: execute_agent_chat_turn(...)
        Agent-->>User: AgentStream<ChatChunk> (ToolStart, ToolResult, TextDelta, Done)
    end
```

### 2.2 Schema Definitions (`src/ai/schema.rs`)

```rust
use golem_rust::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ChatRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "tool")]
    Tool,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ToolCallRecord {
    pub id: String,
    pub tool_name: String,
    pub arguments: String,
    pub result: String,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ChatMessageType {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "error")]
    Error,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCallRecord>>,
    pub timestamp: String,
    pub r#type: Option<ChatMessageType>,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChatResponse {
    pub reply: String,
    pub tool_calls_used: Vec<ToolCallRecord>,
    pub r#type: Option<ChatMessageType>,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ChatChunk {
    TextDelta {
        content: String,
    },
    ToolStart {
        content: String,
        tool_call: ToolCallRecord,
    },
    ToolResult {
        content: String,
        tool_call: ToolCallRecord,
    },
    Done {
        content: String,
    },
    Error {
        content: String,
    },
}
```

### 2.3 Custom Loop Engine & Streaming (`src/ai/agent_loop.rs`)

The custom loop:
1. **Model Resolution**:
   - If `api_url` is provided: normalizes URL (e.g. `https://generativelanguage.googleapis.com/v1beta/openai` or custom proxy).
   - If no `api_url` is provided:
     - If `model` starts with `gemini-`, default to Google Generative AI OpenAI-compatible endpoint `https://generativelanguage.googleapis.com/v1beta/openai/chat/completions`.
     - Otherwise, default to `https://openrouter.ai/api/v1/chat/completions`.
2. **Message Assembly**:
   - Start with `system` prompt message.
   - Replay previous conversation messages (roles `user` and `assistant`).
   - Append current user turn `user` message.
3. **Execution Loop & Chunk Generation**:
   - Loop up to `MAX_TURNS` (10 turns).
   - Send `POST` via `golem_rust::http`.
   - If missing API key or HTTP error:
     - Record `ChatChunk::Error` and `ChatChunk::Done`.
     - Return error response cleanly.
   - If response contains `tool_calls`:
     - For each tool call:
       - Emit `ChatChunk::ToolStart { content: format!("Executing {}...", name), tool_call }`.
       - Execute tool via the agent's dispatcher.
       - Emit `ChatChunk::ToolResult { content: summary, tool_call }`.
       - Add assistant message with tool calls and tool result messages (`role: "tool"`, `tool_call_id`) to the active loop messages.
       - Continue to next loop turn.
   - If response has no tool calls:
     - Extract text content.
     - Emit `ChatChunk::TextDelta { content: reply }`.
     - Emit `ChatChunk::Done { content: String::new() }`.
     - Return `ChatTurnResult { reply, tool_calls_used, chunks, is_error }`.
4. **Streaming Integration (`AgentStream<ChatChunk>`)**:
   - `chat_stream` uses `golem_rust::agentic::{AgentStream, spawn_local}`:
     ```rust
     let (mut writer, stream) = AgentStream::new();
     spawn_local(async move {
         for chunk in chunks {
             if writer.write_one(chunk).await.is_err() {
                 break; // Consumer dropped / closed connection
             }
         }
     });
     stream
     ```

### 2.4 Tools Specification

#### Shopping Assistant Tools (16 tools):
- `search_products`: `query: String, currency: Option<String>` -> `ProductSearchAgentClient::get().search_with_prices`
- `get_product_details`: `product_id: String, currency: Option<String>` -> `ProductSearchAgentClient::get().get_by_ids_with_prices`
- `check_inventory`: `product_id: String, region: Option<String>` -> `InventoryAgentClient::get(product_id).get_available_quantity`
- `get_cart`: none -> `CartAgentClient::get(user_id).get_cart`
- `add_to_cart`: `product_id: String, quantity: u32` -> `CartAgentClient::get(user_id).add_item`
- `update_cart_quantity`: `product_id: String, quantity: u32` -> `CartAgentClient::get(user_id).update_item_quantity`
- `remove_from_cart`: `product_id: String` -> `CartAgentClient::get(user_id).remove_item`
- `clear_cart`: none -> `CartAgentClient::get(user_id).clear`
- `get_saved_contact_info`: none -> `CartAgentClient::get(user_id).get_saved_contact_info`
- `apply_saved_contact_info`: none -> `CartAgentClient::get(user_id).apply_saved_contact_info`
- `set_shipping_address`: `street, city, state_or_region, country, postal_code, name, phone_number` -> `CartAgentClient::get(user_id).update_shipping_address`
- `set_billing_address`: `street, city, state_or_region, country, postal_code, name, phone_number` -> `CartAgentClient::get(user_id).update_billing_address`
- `set_email`: `email: String` -> `CartAgentClient::get(user_id).update_email`
- `checkout_cart`: none -> `CartAgentClient::get(user_id).checkout`
- `get_order_history`: `limit: Option<u32>` -> `CartAgentClient::get(user_id).get_order_history`
- `get_order_details`: `order_id: String` -> `OrderAgentClient::get(order_id).get_order`

#### Manager Agent Tools (14 tools):
- `search_orders`: `query: Option<String>, status: Option<String>, user_id: Option<String>, limit: Option<u32>` -> `OrderSearchAgentClient::get().search_orders`
- `list_recent_orders`: `limit: Option<u32>` -> `OrderSearchAgentClient::get().get_recent_orders`
- `get_order_details`: `order_id: String` -> `OrderAgentClient::get(order_id).get_order`
- `ship_order`: `order_id: String` -> `OrderAgentClient::get(order_id).ship_order`
- `cancel_order`: `order_id: String` -> `OrderAgentClient::get(order_id).cancel_order`
- `add_order_item`: `order_id: String, product_id: String, quantity: u32` -> `OrderAgentClient::get(order_id).add_item`
- `remove_order_item`: `order_id: String, product_id: String` -> `OrderAgentClient::get(order_id).remove_item`
- `update_order_item_quantity`: `order_id: String, product_id: String, quantity: u32` -> `OrderAgentClient::get(order_id).update_item_quantity`
- `update_order_shipping_address`: `order_id: String, address fields` -> `OrderAgentClient::get(order_id).update_shipping_address`
- `update_order_billing_address`: `order_id: String, address fields` -> `OrderAgentClient::get(order_id).update_billing_address`
- `update_order_email`: `order_id: String, email: String` -> `OrderAgentClient::get(order_id).update_email`
- `check_product_details`: `product_id: String` -> `ProductAgentClient::get(product_id).get_product`
- `check_inventory`: `product_id: String, region: Option<String>` -> `InventoryAgentClient::get(product_id).get_available_quantity`
- `search_products`: `query: String, currency: Option<String>` -> `ProductSearchAgentClient::get().search_with_prices`

---

## 3. File-by-File Changes

| Status | File Path | Description |
|---|---|---|
| `[NEW]` | `src/ai/mod.rs` | AI module exports |
| `[NEW]` | `src/ai/schema.rs` | Shared Schema types: `ChatMessage`, `ChatResponse`, `ChatChunk`, `ToolCallRecord`, `ChatRole`, `ChatMessageType` |
| `[NEW]` | `src/ai/agent_loop.rs` | Custom LLM loop, HTTP request formatting, tool-calling execution cycle, chunk emission |
| `[MODIFY]` | `src/shopping_assistant/mod.rs` | Add `chat`, `chat_stream`, `get_history`, `clear_history` to `ShoppingAssistantAgent`; add state message storage, tool executor, and snapshot updates |
| `[NEW]` | `src/manager/mod.rs` | New `ManagerAgent` definition and implementation with store management tools, `chat`, and `chat_stream` |
| `[MODIFY]` | `src/lib.rs` | Declare and re-export `ai` and `manager` modules |
| `[MODIFY]` | `golem.yaml` | Add `ManagerAgent` under `httpApi.deployments.local.agents` |

---

## 4. Verification Plan

1. **Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **WASM Compilation**:
   ```bash
   cargo check --target wasm32-wasip2
   ```
3. **Clippy Linting**:
   ```bash
   cargo clippy --target wasm32-wasip2
   ```
4. **Golem Application Build**:
   ```bash
   golem build --yes
   ```
5. **Unit / Component Tests**:
   - Test JSON serialization of `ChatMessage`, `ChatResponse`, `ChatChunk`, and `ToolCallRecord`.
   - Test missing API key handling returns graceful error message and error chunk.
   - Test tool schema generation and argument parsing.

---

## 5. Risks & Trade-Offs

- **Model Compatibility**: Different providers handle tool calling slightly differently (e.g. Gemini OpenAI compatibility vs OpenRouter vs pure OpenAI). By supporting standard OpenAI function calling schema and headers (`Authorization: Bearer <key>`), both OpenRouter and Google Gemini OpenAI endpoints work cleanly.
- **Snapshot Evolution**: `ShoppingAssistantAgent` already has saved snapshots in running instances with `RecommendedItems`. We handle backwards compatibility by deserializing `ShoppingAssistantSnapshot` and falling back to `RecommendedItems` with empty history if the new format is not found.
- **Streaming Backpressure**: `AgentStream::write_one` waits for downstream consumer acceptance; wrapping production inside `spawn_local` ensures cooperative scheduling without blocking the initial invocation return.
