# Implementation Walkthrough: Refactor `get_llm_recommendations` to Reuse `execute_agent_chat_turn`

- **Feature ID / Slug**: `fix-get-llm-recommendations`
- **Date**: 2026-11-03
- **Status**: Completed

---

## 1. Overview of Changes

We refactored `get_llm_recommendations` in [`src/shopping_assistant/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/shopping_assistant/mod.rs) to reuse the unified, provider-agnostic agent chat turn executor [`execute_agent_chat_turn`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/ai/agent_loop.rs#L274).

### Key Accomplishments

1. **Provider-Agnostic Execution**:
   - Replaced direct usage of `golem_ai_llm_openrouter::DurableOpenRouter::send` with `execute_agent_chat_turn`.
   - Now supports Google Gemini (`generativelanguage.googleapis.com` OpenAI endpoint), OpenRouter, or any custom `apiUrl` configured in `golem.yaml` or secrets.
2. **Panic Elimination & Error Handling**:
   - Removed `.expect("LLM recommendations")`, preventing unhandled crashes.
   - Failures (missing API keys, network/HTTP errors) are captured cleanly via `ChatTurnResult` and returned as `Err(String)`.
3. **No-Op Dispatcher & Markdown Code Fence Extraction**:
   - Added [`NoopDispatcher`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/ai/agent_loop.rs#L245-L253) in `src/ai/agent_loop.rs` for tool-free chat turns.
   - Added [`extract_json_str`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/ai/agent_loop.rs#L254-L275) to reliably strip markdown code fences (` ```json ... ``` ` or ` ``` ... ``` `) and isolate JSON objects before deserializing with `serde_json::from_str`.
4. **Dependency Cleanup**:
   - Removed unused crates `golem-ai-llm` and `golem-ai-llm-openrouter` from `Cargo.toml`.
   - Removed unused imports and policy definitions in `src/shopping_assistant/mod.rs`.

---

## 2. Changes by File

### 2.1 `src/ai/agent_loop.rs`
- Added `pub struct NoopDispatcher` implementing `ToolDispatcher`.
- Added `pub fn extract_json_str(text: &str) -> &str`.
- Added unit tests: `test_extract_json_str` and `test_noop_dispatcher`.

### 2.2 `src/shopping_assistant/mod.rs`
- Imported `NoopDispatcher`, `execute_agent_chat_turn`, and `extract_json_str` from `crate::ai`.
- Refactored `get_llm_recommendations` to call `execute_agent_chat_turn` with:
  - System prompt with JSON schema definition.
  - User message with current order items string.
  - Empty tools `&[]` and history `&[]`.
  - Credentials extracted from `LlmConfig`.
  - `&NoopDispatcher`.
- Handled `result.is_error` and parsed JSON using `extract_json_str` + `serde_json::from_str`.

### 2.3 `Cargo.toml`
- Removed `golem-ai-llm` and `golem-ai-llm-openrouter` dependencies.

---

## 3. Verification Results

| Check | Command | Result |
|---|---|---|
| **Format Check** | `cargo fmt --check` | Passed |
| **Lint Check** | `cargo clippy --target wasm32-wasip2` | Passed |
| **Unit Tests** | `cargo test` | Passed (18 passed, 0 failed) |
| **WASM Build** | `cargo build --target wasm32-wasip2 --release` | Passed |

