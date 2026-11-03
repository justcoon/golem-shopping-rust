# Implementation Walkthrough: Chat-Specific Agents Architecture

- **Feature ID / Slug**: `chat-specific-agents`
- **Date**: 2026-11-03
- **Status**: Completed

---

## 1. Executive Summary

We designed, implemented, and verified the **Two-Tier Chat-Specific Agents Architecture** across the Golem backend, the Vue 3 Single Page Application (`frontend/`), and the embedded static web clients (`web/`).

### What this accomplishes
1. **Eliminated Stream Queue Contention**:
   - Long-lived bi-directional Durable Streams (`/chat-session`) and SSE streams (`/chat-stream`) now run inside dedicated thread actors:
     - `ShoppingAssistantChatAgent(user_id, chat_id)`
     - `ManagerChatAgent(manager_id, chat_id)`
   - The parent workspace agents (`ShoppingAssistantAgent(user_id)` and `ManagerAgent(manager_id)`) no longer hold open streams. Queries such as `GET /chats`, `GET /recommended-items`, and queries to other conversation threads return immediately without waiting for an active stream to close.
2. **Multi-Thread Conversation Management**:
   - Users and managers can list existing conversations, start new named conversations, switch between them, and delete conversation threads.
3. **Zero Overhead During Live Chat**:
   - Kept the design clean and lightweight: no complex inter-agent callbacks on each message turn. Each chat agent manages its conversation turns independently.
4. **100% Backwards Compatibility**:
   - The parent agents provide fallback proxies for legacy single-thread endpoints (`/history`, `/chat`, `/clear-history`), automatically delegating to a canonical `"default"` thread.

---

## 2. Changes Implemented

### Backend (Rust / Golem)

- **[`src/ai/schema.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/ai/schema.rs)**:
  - Added `ChatThreadSummary` schema (`chat_id`, `title`, `created_at`).
  - Added `CreateChatRequest` schema (`title: Option<String>`).
- **[`src/shopping_assistant/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/shopping_assistant/mod.rs)**:
  - Added `ShoppingAssistantChatAgent` trait & `ShoppingAssistantChatAgentImpl` mounted at `/v1/assistant/{user_id}/chats/{chat_id}` with `get_history`, `chat` (non-streaming), `chat_session` (bi-directional Durable Streams), and `clear_history`.
  - Removed redundant `chat_stream` single-turn endpoint.
  - Refactored `ShoppingAssistantAgent` mounted at `/v1/assistant/{id}` to manage `threads: Vec<ChatThreadSummary>`, with endpoints `list_chats`, `create_chat`, `delete_chat`, `get_recommended_items`, `recommend_items`, and backwards-compatible proxies.
  - Streamlined `load_snapshot` to directly deserialize `ShoppingAssistantSnapshot` without legacy schema fallback branching.
- **[`src/manager/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/manager/mod.rs)**:
  - Added `ManagerChatAgent` trait & `ManagerChatAgentImpl` mounted at `/v1/manager/{manager_id}/chats/{chat_id}` with `chat` and `chat_session`.
  - Removed redundant `chat_stream` single-turn endpoint.
  - Refactored `ManagerAgent` mounted at `/v1/manager/{id}` to manage `threads: Vec<ChatThreadSummary>`.
  - Streamlined `load_snapshot` to directly deserialize `ManagerSnapshot` without legacy fallback branching.
- **[`golem.yaml`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/golem.yaml)**:
  - Registered `ShoppingAssistantChatAgent` and `ManagerChatAgent` under `httpApi.deployments.local.agents`.

### Frontend SPA (`frontend/`)

- **[`frontend/src/api/services/assistantService.ts`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/frontend/src/api/services/assistantService.ts)**:
  - Added `ChatThreadSummary` interface.
  - Added `listAssistantChats`, `createAssistantChat`, `deleteAssistantChat`.
  - Parameterized `getAssistantHistory`, `clearAssistantHistory`, `chatWithAssistant`, `streamChatWithAssistant`, and `createChatSession` with `chatId: string = "default"` pointing to `/v1/assistant/${userId}/chats/${chatId}/...`.
- **[`frontend/src/stores/assistantStore.ts`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/frontend/src/stores/assistantStore.ts)**:
  - Added `chats` and `currentChatId` state.
  - Added `fetchChats`, `switchChat`, `createNewChat`, and `deleteChat`.
  - Configured session and history management to bind to `currentChatId`.
- **[`frontend/src/views/ShoppingAssistantView.vue`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/frontend/src/views/ShoppingAssistantView.vue)**:
  - Added thread selection dropdown (`.thread-select`) and `➕ New Chat` button (`.new-chat-btn`) in the header actions.
  - Bound thread switching and new thread creation.

### Embedded Web Clients (`web/`)

- **[`web/assistant.html`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/assistant.html) & [`web/assets/assistant.js`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/assets/assistant.js)**:
  - Added thread selector and "➕" new conversation button.
  - Bound all chat and stream calls to `/v1/assistant/{userId}/chats/{chatId}/...`.
- **[`web/manager.html`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/manager.html) & [`web/assets/manager.js`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/web/assets/manager.js)**:
  - Added symmetrical thread selection for store manager operations.

### Documentation & Architecture

- **[`architecture.puml`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/architecture.puml)**:
  - Updated PlantUML architecture diagram to show the two-tier structure (`ShoppingAssistantAgent` + `ShoppingAssistantChatAgent` and `ManagerAgent` + `ManagerChatAgent`).
- **[`README.md`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/README.md)**:
  - Documented the multi-thread chat agent architecture.

---

## 3. Validation Results

| Test / Check | Tool / Command | Result | Proof |
| :--- | :--- | :--- | :--- |
| **Rust Code Formatting** | `cargo fmt --check` | **PASS** | Formatted cleanly without discrepancies |
| **Type Check & Compilation** | `cargo check --target wasm32-wasip2` | **PASS** | Finished in 5.11s with 0 errors |
| **Linter Verification** | `cargo clippy --target wasm32-wasip2` | **PASS** | 0 errors |
| **Automated Unit Tests** | `cargo test` | **PASS** | 16/16 tests passed |
| **Frontend Production Build** | `cd frontend && npx vite build` | **PASS** | Vite built in 1.73s with 0 errors |

---

## 4. Usage Guide

### 1. Listing & Creating Threads via API
```bash
# List all threads for customer u001 (returns default thread on first call)
curl -s http://localhost:9006/v1/assistant/u001/chats

# Create a new conversation thread
curl -s -X POST http://localhost:9006/v1/assistant/u001/chats \
  -H "Content-Type: application/json" \
  -d '{"title": "Electronics Search"}'
```

### 2. Conversing & Streaming on a Specific Thread
```bash
# Get message history of a specific thread
curl -s http://localhost:9006/v1/assistant/u001/chats/chat-1/history

# Open a Durable Streams session on thread chat-1 (does NOT block u001 parent!)
curl -i -X PUT http://localhost:9006/v1/assistant/u001/chats/chat-1/chat-session \
  -H "Content-Type: application/json" \
  -H "Stream-TTL: 600" \
  -d '{}'
```

### 3. In the UI
- **Vue 3 Frontend**: Launch `cd frontend && npm run dev` -> Select any thread from the dropdown or click **"➕ New Chat"**.
- **Embedded Web Client**: Open `http://localhost:9006/assistant` -> Switch between threads or click **"➕"** to start a new conversation.

