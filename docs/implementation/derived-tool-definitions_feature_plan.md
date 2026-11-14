# Feature Plan: First-Class Golem Tools with Platform MCP & Type-Derived Internal Dispatch

- **Feature ID / Slug**: `derived-tool-definitions`
- **Date**: 2026-11-03
- **Status**: Completed & Verified

---

## 1. Overview & Goals

Currently, tools in `golem-shopping-rust` are implemented as private, ad-hoc string-matching logic:
1. **Manual JSON Schemas**: Over 200 lines of raw JSON objects constructed via `serde_json::json!({...})` in `src/shopping_assistant/tools.rs` and `src/manager/tools.rs`.
2. **Monolithic String Parsing**: Over 700 lines of manual string extraction and `serde_json::from_str` matching inside giant `match tool_name` blocks (`execute_shopping_assistant_tool` and `execute_manager_tool`).
3. **Redundant Manual Dispatchers**: Custom `ToolDispatcher` structs (`ShoppingAssistantDispatcher` and `ManagerDispatcher`) written solely to bridge string calls into the monolithic `match` function.
4. **Internal-Only Silo**: These tools are invisible to the Golem platform, cannot be called by other components, and cannot be discovered or invoked by external AI clients (Claude Desktop, Cursor, or external LLMs).

### Goals

1. **First-Class Golem Platform Tools (`#[tool_definition]`)**:
   - Define reusable, typed Golem Tools using `#[tool_definition(version = "1.0.0")]` and `#[tool_implementation]` in `src/tools/`.
   - Tools cover the primary functional capabilities of the store:
     - `CartTool`: Cart management, item additions/removals, and checkout with domain return types (`Option<Cart>`, `Result<CartUpdated, CartToolError>`, `Result<OrderConfirmation, CartToolError>`, etc.).
     - `CatalogTool`: Product search, product details, and inventory checks with domain return types (`Vec<ProductWithPrice>`, `Option<ProductWithPrice>`, `Availability`, `Option<Product>`).
     - `OrderTool`: Order history, order tracking, and order status updates with domain return types (`Option<Order>`, `Result<Vec<Order>, OrderToolError>`, `Result<OrderUpdated, OrderToolError>`).
   - Implement tools by delegating directly to the underlying durable agents (`CartAgentClient`, `ProductSearchAgentClient`, `OrderAgentClient`, `InventoryAgentClient`).
   - All tool methods return **strongly typed domain structs and typed `ToolError` enums**, eliminating raw string returns across all tools.
2. **Dual-Use Automatic Schema Derivation (`schemars`) for Internal LLMs**:
   - Tool input structs derive both Golem schemas (`IntoSchema`, `FromSchema`) and `schemars::JsonSchema`.
   - Standard Rust doc comments (`///`) serve as the single source of truth for parameter and tool descriptions.
   - Automatically generate `ToolSpec`s for OpenAI/Gemini without writing raw JSON.
3. **Automatic Generic `ToolDispatcher` (Zero Manual Dispatch Boilerplate)**:
   - Implement `ToolDispatcher` **once, generically** on `ToolRegistry`.
   - Completely delete `execute_shopping_assistant_tool`, `execute_manager_tool`, and custom dispatcher structs (`ShoppingAssistantDispatcher`, `ManagerDispatcher`).
   - The registry automatically routes tool calls, deserializes JSON arguments into typed structs, and returns formatted responses.
4. **Expose Capabilities via Golem MCP Server (`golem.yaml`)**:
   - Add an `mcp.deployments` section to `golem.yaml` for `local` and `cloud` environments.
   - External AI agents and developer tools (Claude Desktop, Cursor, MCP Inspector) can interact with the store over standard Streamable HTTP MCP (`http://shopping-mcp.localhost:9007/mcp`) with zero extra code.
5. **Preserve Full Backward Compatibility**:
   - Keep tool names, arguments, and behavior identical so active chat histories, system prompts, and web UI streaming features continue to function seamlessly.

---

## 2. Architecture & Technical Design

```
                                 ┌────────────────────────────────────────────────────────────┐
                                 │              Golem Platform Tools (`src/tools/`)           │
                                 │                                                            │
                                 │  #[tool_definition(version = "1.0.0")]                     │
                                 │  - CartTool       (CartToolImpl -> CartAgentClient)        │
                                 │  - CatalogTool    (CatalogToolImpl -> Product/Inventory)   │
                                 │  - OrderTool      (OrderToolImpl -> OrderAgentClient)      │
                                 └─────────────────────────────┬──────────────────────────────┘
                                                               │
                       ┌───────────────────────────────────────┴───────────────────────────────────────┐
                       ▼                                                                               ▼
       ┌───────────────────────────────┐                                               ┌───────────────────────────────┐
       │   1. External Exposure (MCP)  │                                               │    2. Internal LLM Loop       │
       │                               │                                               │                               │
       │  `golem.yaml`:                │                                               │  `ShoppingAssistant` & `Manager`:
       │  mcp.deployments:             │                                               │  - Args derive `JsonSchema`   │
       │    subdomain: shopping-mcp    │                                               │  - Auto-generate `ToolSpec`s  │
       │  (Claude, Cursor, MCP clients)│                                               │  - Generic `ToolDispatcher`   │
       └───────────────────────────────┘                                               │    (No manual match blocks!)  │
                                                                                       └───────────────────────────────┘
```

### 2.1 Tool Definitions (`src/tools/`)

Each tool is defined using Golem's native macros:

```rust
use golem_rust::{tool_definition, tool_implementation, FromSchema, IntoSchema, ToolError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Typed arguments derive both Golem Schema and schemars JSON Schema
#[derive(Debug, Clone, Serialize, Deserialize, IntoSchema, FromSchema, JsonSchema)]
pub struct AddItemInput {
    /// User ID owning the cart
    pub user_id: String,
    /// Product SKU / ID, e.g. 'ap001'
    pub product_id: String,
    /// Number of units to add
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoSchema, FromSchema, ToolError)]
pub enum CartToolError {
    #[tool_error("Item '{0}' not found in catalog")]
    ProductNotFound(String),
    #[tool_error("Failed to update cart: {message}")]
    OperationFailed { message: String },
}

#[tool_definition(version = "1.0.0")]
pub trait CartTool {
    /// Add an item to the shopping cart
    async fn add_item(&self, input: AddItemInput) -> Result<String, CartToolError>;
    
    /// Retrieve the current cart details
    async fn get_cart(&self, user_id: String) -> Result<String, CartToolError>;
    
    /// Place an order from the user's cart
    async fn checkout(&self, user_id: String) -> Result<String, CartToolError>;
}
```

### 2.2 Tool Implementation (`#[tool_implementation]`)

Each tool implementation forwards calls to the appropriate Golem agent client:
```rust
pub struct CartToolImpl;

#[tool_implementation]
impl CartTool for CartToolImpl {
    async fn add_item(&self, input: AddItemInput) -> Result<String, CartToolError> {
        let mut cart = CartAgentClient::get(input.user_id.clone());
        match cart.add_item(input.product_id.clone(), input.quantity).await {
            Ok(_) => Ok(format!("Added {} of {} to cart", input.quantity, input.product_id)),
            Err(e) => Err(CartToolError::OperationFailed { message: format!("{e:?}") }),
        }
    }
    // ...
}
```

### 2.3 Automatic Schema Generation for Internal LLMs

In `src/ai/tools.rs`, a helper bridges the tool input structs into the `ToolSpec` needed by OpenAI/Gemini:

```rust
pub fn spec_from_schema<T: schemars::JsonSchema>(name: &str, description: &str) -> ToolSpec {
    let raw_schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    // Normalize into OpenAI-compatible parameters object
    ToolSpec {
        name: name.to_string(),
        description: description.to_string(),
        parameters: clean_openai_parameters(raw_schema),
    }
}
```

### 2.4 Generic `ToolDispatcher` Elimination (No More Manual Dispatchers!)

In current code, each agent defines a manual `ToolDispatcher` struct that calls a huge `match` block:
```rust
// ❌ OLD: Required for every agent
struct ShoppingAssistantDispatcher<'a> { user_id: &'a str }
impl ToolDispatcher for ShoppingAssistantDispatcher<'_> {
    async fn execute(&self, name: &str, args: &str) -> (String, String) {
        execute_shopping_assistant_tool(self.user_id, name, args).await // 350-line match block
    }
}
```

In the new architecture, **`ToolDispatcher` is implemented generically on `ToolRegistry`**:
```rust
// ✅ NEW: Generic implementation on `ToolRegistry` in `src/ai/tools.rs`
impl ToolDispatcher for ToolRegistry {
    async fn execute(&self, name: &str, arguments: &str) -> (String, String) {
        if let Some(tool) = self.get(name) {
            tool.execute(arguments).await
        } else {
            let err = format!("Unknown tool: {name}");
            (err.clone(), err)
        }
    }
}
```

Inside `ShoppingAssistantAgentImpl` and `ManagerAgentImpl`:
- Pass `&tools` (the `ToolRegistry`) directly to `process_chat_turn` or `stream_chat_turn`.
- **Zero** dispatcher structs needed.
- `execute_shopping_assistant_tool` and `execute_manager_tool` are completely removed.

### 2.5 Golem Manifest Integration (`golem.yaml`)

Declare the tools and the MCP deployment:
```yaml
tools:
  cart:
    component: shopping-rust:shopping
  catalog:
    component: shopping-rust:shopping
  order:
    component: shopping-rust:shopping

mcp:
  deployments:
    local:
      - subdomain: shopping-mcp
        agents:
          CartAgent: {}
          ProductSearchAgent: {}
          InventoryAgent: {}
          OrderAgent: {}
```

---

## 3. File-by-File Changes

| File | Status | Description |
|---|---|---|
| `src/tools/mod.rs` | `[NEW]` | Module entry point for tools. |
| `src/tools/cart.rs` | `[NEW]` | `CartTool` definition (`#[tool_definition]`), implementation, error types, and argument schemas. |
| `src/tools/catalog.rs` | `[NEW]` | `CatalogTool` definition (product search, details, inventory). |
| `src/tools/order.rs` | `[NEW]` | `OrderTool` definition (order history, details, tracking). |
| `src/ai/tools.rs` | `[NEW]` | Type-safe `ToolSpec` generator, `ToolRegistry`, and generic `ToolDispatcher` implementation. |
| `src/ai/mod.rs` | `[MODIFY]` | Re-export `tools` module. |
| `src/shopping_assistant/tools.rs` | `[MODIFY]` | Replace manual JSON specs and 350-line `match` dispatcher with tool registration on `ToolRegistry`. |
| `src/shopping_assistant/mod.rs` | `[MODIFY]` | Remove `ShoppingAssistantDispatcher` struct; pass `&tools` (`ToolRegistry`) directly to chat methods. |
| `src/manager/tools.rs` | `[MODIFY]` | Replace manual JSON specs and 350-line `match` dispatcher with tool registration on `ToolRegistry`. |
| `src/manager/mod.rs` | `[MODIFY]` | Remove `ManagerDispatcher` struct; pass `&tools` (`ToolRegistry`) directly to chat methods. |
| `src/lib.rs` | `[MODIFY]` | Re-export `tools` module. |
| `golem.yaml` | `[MODIFY]` | Add `tools` definitions and `mcp.deployments` for local and cloud environments. |

---

## 4. Verification Plan

### Automated Checks
1. **Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Type Checking (WASM target)**:
   ```bash
   cargo check --target wasm32-wasip2
   ```
3. **Clippy Linting**:
   ```bash
   cargo clippy --target wasm32-wasip2
   ```
4. **Unit Tests**:
   ```bash
   cargo test --lib
   ```
   - Test JSON schema generation from `JsonSchema` derives.
   - Test argument deserialization, tool routing, and error formatting in `ToolRegistry`.
5. **Golem Build**:
   ```bash
   golem build --yes
   ```
   - Confirms Golem macros (`#[tool_definition]`, `#[tool_implementation]`) generate valid component bindings and exports.

### Functional / Smoke Verification
- Verify that `ShoppingAssistantAgent` and `ManagerAgent` continue to answer shopping queries and invoke tools (search, cart, checkout) accurately.
- Verify that the generated JSON schemas match OpenAI and Gemini function calling format specifications.
- Verify that `golem deploy --yes` properly exposes the local MCP endpoint and tool bindings.

---

## 5. Risks & Open Questions

- **Model Schema Sensitivity**: Some LLM providers reject top-level `$schema` or require an explicit `"type": "object"` in tool parameters.
  - *Mitigation*: The `clean_openai_parameters` utility strips top-level metadata and ensures root object shape.
- **Component Packaging**: Golem allows tools and agents to coexist in the same component crate.
  - *Mitigation*: Validated by building targeting `shopping-rust:shopping` with `golem build --yes`.
