# Feature Plan: Order Search Agent

- **Feature ID / Slug**: `order-search`
- **Date**: 2026-11-02
- **Status**: Completed <!-- Draft | Approved | In Progress | Completed -->

---

## 1. Overview & Goals

This feature introduces an `OrderSearchAgent` to the Golem Shopping Rust application, mirroring the capabilities in [`golem-shopping-effect`](https://github.com/golemcloud/golem-shopping-effect/src/agents/order-search/order-search-agent.ts) while strictly adhering to the Rust idiomatic patterns demonstrated by [`ProductSearchAgent`](../../src/product_search/mod.rs).

The `OrderSearchAgent` enables customers, store administrators, and AI assistants (such as `ShoppingAssistantAgent`) to discover, query, filter, and inspect orders placed across the system.

### Goals

1. **Dedicated Ephemeral Order Search Agent (`OrderSearchAgent`)**:
   - Defined with `#[agent_definition(ephemeral, mount = "/v1/order-search")]`.
   - Resolves the local component ID via `resolve_component_id("shopping-rust:shopping")` as in `ProductSearchAgent`.
   - Discovers active `OrderAgent` instances using Golem's component agent discovery API (`GetAgents` with `AgentNameFilter` matching `"OrderAgent("`).
   - Fetches and inspects orders concurrently via `OrderAgentClient::get(id).get_order()` using `futures::future::join_all`.
2. **Search & Filter Capabilities**:
   - **General Search (`search_orders`)**: `GET /search?query={query}&status={status}&userId={user_id}&limit={limit}` — filters by free-text queries, field selectors (`orderid:`, `id:`, `userid:`, `status:`, `email:`, `productid:`), optional lifecycle `status`, and customer `userId`.
   - **User Orders Query (`get_orders_by_user_id`)**: `GET /user/{user_id}?limit={limit}` — fast retrieval of all orders placed by a specific customer.
   - **Status Filter Query (`get_orders_by_status`)**: `GET /status/{status}?limit={limit}` — retrieval of orders by lifecycle status (`New`, `Shipped`, `Cancelled`).
   - **Recent Orders Query (`get_recent_orders`)**: `GET /recent?limit={limit}` — global timeline of recent orders across the store.
3. **Sorting & Pagination**:
   - Results sorted descending by `created_at` (`b.created_at.cmp(&a.created_at)`) so newer orders appear first.
   - Configurable `limit` parameter (defaults to 20).
4. **Manifest, Architecture & Documentation**:
   - Register `OrderSearchAgent` in `golem.yaml` under the local HTTP API deployment.
   - Update `architecture.puml` and `README.md` to reflect `OrderSearchAgent` and its routing/communication.

### Non-Goals

- Storing a separate external database index (the agent uses Golem's native agent enumeration and RPC fetch).
- Mutation of orders (all search operations are read-only).

---

## 2. Architecture & Technical Design

### 2.1 Pattern Alignment with `ProductSearchAgent`

In [`src/product_search/mod.rs`](../../src/product_search/mod.rs), the agent leverages:
1. `resolve_component_id("shopping-rust:shopping")` stored on the agent struct.
2. `get_agent_filter() -> AgentAnyFilter` with `StartsWith` filter on `"OrderAgent("`.
3. `get_order_agent_id(agent_name: &str) -> Option<String>` extracting the ID via regex `r#"OrderAgent\("([^)]+)"\)"#`.
4. Streaming pagination with `GetAgents::new(component_id, Some(&filter), false)` and tracking `processed_agent_ids: HashSet<String>`.
5. Concurrent batch RPC fetches using `futures::future::join_all`.
6. Custom tokenized query matcher (`OrderQueryMatcher`) supporting quoted strings, field filters, and general term matching.

```mermaid
flowchart TD
    Client["Client / HTTP Gateway\nGET /v1/order-search/search?..."] --> OSA["OrderSearchAgent\n(Ephemeral Agent mounted at /v1/order-search)"]
    OSA -->|1. Resolve & Filter Agents| Discovery["Golem Agent Discovery\n(GetAgents prefix 'OrderAgent(')"]
    Discovery -->|Order Agent IDs| OSA
    OSA -->|2. Concurrent RPC join_all(get_order())| Orders["OrderAgent(order-1)\nOrderAgent(order-2)\n..."]
    Orders -->|Order States| OSA
    OSA -->|3. Match, Sort Desc, Limit| Filter["OrderQueryMatcher & Chrono Sort"]
    Filter -->|Filtered Orders| Client
```

### 2.2 Query Matcher (`OrderQueryMatcher`)

Matches the tokenizer and query evaluator pattern in `ProductSearchAgent` adapted for [`Order`](../../src/order/mod.rs):
- **Field Filters**:
  - `orderid:<id>` or `id:<id>`: exact matching against `order.order_id` (case-insensitive).
  - `userid:<id>`: exact matching against `order.user_id` (case-insensitive).
  - `status:<val>` or `orderstatus:<val>`: exact matching against `order.order_status` (`new`, `shipped`, `cancelled`).
  - `email:<text>`: substring match against `order.email`.
  - `productid:<id>`: checks whether any line item in `order.items` matches `item.product_id`.
- **Free-Text Search Terms**:
  - Matched against `order.order_id`, `order.user_id`, string representation of `order.order_status`, `order.email`, shipping address name, billing address name, and line item `product_name`, `product_brand`, or `product_id`.
  - All terms must match (logical AND). Wildcard `*` matches all.

### 2.3 Agent Trait Definition

```rust
#[agent_definition(ephemeral, mount = "/v1/order-search")]
pub trait OrderSearchAgent {
    fn new() -> Self;

    #[endpoint(get = "/search?query={query}&status={status}&userId={user_id}&limit={limit}")]
    async fn search_orders(
        &self,
        query: Option<String>,
        status: Option<OrderStatus>,
        user_id: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String>;

    #[endpoint(get = "/user/{user_id}?limit={limit}")]
    async fn get_orders_by_user_id(
        &self,
        user_id: String,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String>;

    #[endpoint(get = "/status/{status}?limit={limit}")]
    async fn get_orders_by_status(
        &self,
        status: OrderStatus,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String>;

    #[endpoint(get = "/recent?limit={limit}")]
    async fn get_recent_orders(&self, limit: Option<u32>) -> Result<Vec<Order>, String>;
}
```

---

## 3. Proposed Changes & File Impact

| Action | File Path | Description |
| :--- | :--- | :--- |
| `[NEW]` | [`src/order_search/mod.rs`](../../src/order_search/mod.rs) | `OrderSearchAgent` definition, implementation, `OrderQueryMatcher`, discovery logic, sorting, and unit tests. |
| `[MODIFY]` | [`src/order/mod.rs`](../../src/order/mod.rs) | Add `Debug` derive to `OrderStatus` for logging and diagnostic printing. |
| `[MODIFY]` | [`src/lib.rs`](../../src/lib.rs) | Register and export `pub mod order_search;`. |
| `[MODIFY]` | [`golem.yaml`](../../golem.yaml) | Register `OrderSearchAgent: {}` in `httpApi.deployments.local.agents`. |
| `[MODIFY]` | [`architecture.puml`](../../architecture.puml) | Add `Order Search Agent` component, endpoint notes, and interaction arrows. |
| `[MODIFY]` | [`README.md`](../../README.md) | Document `Order Search Agent` in application overview and components list. |

---

## 4. Verification Plan

### Automated Checks

1. **Code Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Compilation & Type Check**:
   ```bash
   cargo check --target wasm32-wasip2
   ```
3. **Linter (Clippy)**:
   ```bash
   cargo clippy --target wasm32-wasip2
   ```
4. **Release Build**:
   ```bash
   cargo build --target wasm32-wasip2 --release
   ```
5. **Unit Tests**:
   - Matcher unit tests inside `src/order_search/mod.rs` verifying tokenization, field filters (`orderid:`, `userid:`, `status:`, `email:`, `productid:`), free-text matching, and quoted terms.

---

## 5. User Review & Approval Gate

Please review this implementation plan. Once approved, we will proceed with the implementation, verification, and walkthrough documentation.
