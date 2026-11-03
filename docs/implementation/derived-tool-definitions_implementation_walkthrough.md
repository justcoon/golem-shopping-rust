# Implementation Walkthrough: First-Class Golem Tools with Direct Mounting & Platform MCP

- **Feature ID / Slug**: `derived-tool-definitions`
- **Date**: 2026-11-03
- **Status**: Complete & Verified

---

## 1. Summary of Changes

We completely unified the tool architecture so that **Golem Platform Tools (`#[tool_definition]`) are the single source of truth** across the application:

1. **First-Class Golem Tools with Strongly Typed Domain Returns (`src/tools/`)**:
   - `CartTool` & `CartToolImpl` ([src/tools/cart.rs](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/tools/cart.rs)): Defines all cart methods returning domain structs (`Option<Cart>`, `Result<CartUpdated, CartToolError>`, `Result<OrderConfirmation, CartToolError>`, `Option<SavedContactInfo>`, `Vec<Order>`) instead of formatted strings. Typed errors derive Golem's `#[derive(ToolError)]`.
   - `CatalogTool` & `CatalogToolImpl` ([src/tools/catalog.rs](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/tools/catalog.rs)): Defines catalog methods returning typed domain models (`Vec<ProductWithPrice>`, `Option<ProductWithPrice>`, `Availability`, `Option<Product>`).
   - `OrderTool` & `OrderToolImpl` ([src/tools/order.rs](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/tools/order.rs)): Defines order lifecycle methods returning typed domain models (`Option<Order>`, `Result<Vec<Order>, OrderToolError>`, `Result<OrderUpdated, OrderToolError>`) with typed `OrderToolError`.
   - All tool responses are automatically serialized to JSON by `TypedTool` for the internal LLM conversation context, while remaining clean, strongly-typed Rust APIs for internal Golem RPC and Golem MCP.
2. **Generated Tool Clients & Concise Tool Mounting**:
   - Uses definition-owned tool clients: `CatalogToolClient::new()`, `CartToolClient::new()`, and `OrderToolClient::new()`.
   - Tool descriptions are **automatically extracted from Rust doc comments (`///`)** via `description_from_schema::<Args>()`.
   - Introduced `registry.register_tool_call(...)` and `registry.register_tool_call_with_desc(...)`:
     - Completely eliminates boilerplate: no manual `match` blocks, no manual `serde_json::to_string` calls, and no repetitive error string formatting.
     - Each tool registration is reduced to **a single concise call** passing the client method and a summary badge closure.
   - Configured runtime tool bindings under `components.shopping-rust:shopping.tools` in `golem.yaml` (`cart-tool`, `catalog-tool`, `order-tool`).
3. **Generic `ToolDispatcher` on `ToolRegistry` (`src/ai/tools.rs`)**:
   - Dispatches incoming tool calls directly to the tool implementations.
   - Completely deleted manual dispatcher structs (`ShoppingAssistantDispatcher` and `ManagerDispatcher`).
4. **Manifest Deployment Integration (`golem.yaml`)**:
   - Declared top-level tools (`cart-tool`, `catalog-tool`, `order-tool`) implemented by `shopping-rust:shopping`.
   - Configured `mcp.deployments` for local development with `ownerComponent: shopping-rust:shopping` and agent deployments, allowing external AI clients (Claude Desktop, Cursor, MCP clients) to interact with the store via Streamable HTTP MCP at `http://localhost:9007/mcp`.

---

## 2. Key Code Comparison

### Before (Manual Monolith in `shopping_assistant/tools.rs`)
- Over 200 lines of manual JSON strings (`serde_json::json!({ "type": "object", ... })`).
- Over 350 lines of manual string parsing inside a massive `match tool_name` block.
- Duplicated again in `manager/tools.rs`.

### After (Direct Tool Definition & Mounting)
- All capability logic lives once in `src/tools/cart.rs`, `src/tools/catalog.rs`, and `src/tools/order.rs`.
- Assistants simply mount the tools:
  ```rust
  let mut registry = ToolRegistry::new();
  CatalogToolImpl::register(&mut registry);
  CartToolImpl::register(user_id, &mut registry);
  OrderToolImpl::register_for_assistant(&mut registry);
  ```

---

## 3. Verification Results

All strict Rust and Golem verification gates passed successfully:

| Check | Command | Result |
|---|---|---|
| **Code Formatting** | `cargo fmt --check` | Passed |
| **Type Checking (WASM)** | `cargo check --target wasm32-wasip2` | Passed (0 errors, 0 warnings) |
| **Clippy Linting** | `cargo clippy --target wasm32-wasip2` | Passed (0 errors) |
| **Unit Tests** | `cargo test --lib` | 22/22 tests passed |
| **Golem WASM Build** | `golem build --yes` | Passed (`Finished building [OK]`) |
| **Golem Deployment Planning** | `golem deploy --plan` | Passed (`Finished planning [OK]`) |

---

## 4. Deployment Instructions

```bash
# Deploy to local development server:
golem deploy --yes

# The MCP server will be live at:
# http://localhost:9007/mcp
```
