# Implementation Walkthrough: First-Class Golem Tools with Direct Mounting & Platform MCP

- **Feature ID / Slug**: `derived-tool-definitions`
- **Date**: 2026-11-03
- **Status**: Complete & Verified

---

## 1. Summary of Changes

We completely unified the tool architecture so that **Golem Platform Tools (`#[tool_definition]`) are the single source of truth** across the application:

1. **Categorized Golem Tools with Strongly Typed Domain Returns (`src/tools/`)**:
   - `CatalogTools` & `CatalogToolsImpl` ([src/tools/catalog.rs](../../src/tools/catalog.rs)): `search_products`, `get_product_details`.
   - `InventoryTools` & `InventoryToolsImpl` ([src/tools/inventory.rs](../../src/tools/inventory.rs)): `check_inventory`.
   - `CartTools` & `CartToolsImpl` ([src/tools/cart.rs](../../src/tools/cart.rs)): Cart management, addresses, and two-phase checkout (`prepare_checkout`, `confirm_checkout`, `cancel_checkout`).
   - `CustomerOrderTools` & `CustomerOrderToolsImpl` ([src/tools/customer_order.rs](../../src/tools/customer_order.rs)): Customer order viewing (`get_order_history`, `get_order_details`).
   - `ManagerOrderTools` & `ManagerOrderToolsImpl` ([src/tools/manager_order.rs](../../src/tools/manager_order.rs)): Operations & fulfillment (`search_orders`, `list_recent_orders`, `ship_order`, `cancel_order`, item & address updates).
   - All tool responses are automatically serialized to JSON by `TypedTool` for the internal LLM conversation context, while remaining clean, strongly-typed Rust APIs for internal Golem RPC and Golem MCP.
2. **Generated Tool Clients & Modular Tool Mounting**:
   - Uses definition-owned tool clients: `CatalogToolsClient::new()`, `InventoryToolsClient::new()`, `CartToolsClient::new()`, `CustomerOrderToolsClient::new()`, and `ManagerOrderToolsClient::new()`.
   - Tool descriptions are **automatically extracted from Rust doc comments (`///`)** via `description_from_schema::<Args>()`.
   - Introduced `registry.register_tool_call(...)` and `registry.register_tool_call_with_desc(...)`:
     - Completely eliminates boilerplate: no manual `match` blocks, no manual `serde_json::to_string` calls, and no repetitive error string formatting.
     - Each tool registration is reduced to **a single concise call** passing the client method and a summary badge closure.
   - Configured runtime tool bindings under `components.shopping-rust:shopping.tools` in `golem.yaml` (`catalog-tools`, `inventory-tools`, `cart-tools`, `customer-order-tools`, `manager-order-tools`).
3. **Generic `ToolDispatcher` on `ToolRegistry` (`src/ai/tools.rs`)**:
   - Dispatches incoming tool calls directly to the tool implementations.
   - Completely deleted manual dispatcher structs (`ShoppingAssistantDispatcher` and `ManagerDispatcher`).
4. **Manifest Deployment Integration (`golem.yaml`)**:
   - Declared top-level tools (`catalog-tools`, `inventory-tools`, `cart-tools`, `customer-order-tools`, `manager-order-tools`) implemented by `shopping-rust:shopping`.
   - Configured `mcp.deployments` for local development with `ownerComponent: shopping-rust:shopping` and agent deployments, allowing external AI clients (Claude Desktop, Cursor, MCP clients) to interact with the store via Streamable HTTP MCP at `http://localhost:9007/mcp`.

---

## 2. Key Code Comparison

### Before (Manual Monolith in `shopping_assistant/tools.rs`)
- Over 200 lines of manual JSON strings (`serde_json::json!({ "type": "object", ... })`).
- Over 350 lines of manual string parsing inside a massive `match tool_name` block.
- Duplicated again in `manager/tools.rs`.

### After (Direct Tool Definition & Mounting)
- All capability logic lives in `src/tools/`:
  - `catalog.rs`
  - `inventory.rs`
  - `cart.rs`
  - `customer_order.rs`
  - `manager_order.rs`
- Assistants mount their relevant tools cleanly:
  ```rust
  let mut registry = ToolRegistry::new();
  CatalogToolsImpl::register(&mut registry);
  InventoryToolsImpl::register(&mut registry);
  CartToolsImpl::register(user_id.clone(), &mut registry);
  CustomerOrderToolsImpl::register(user_id, &mut registry);
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
