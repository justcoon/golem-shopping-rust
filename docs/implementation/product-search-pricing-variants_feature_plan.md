# Feature Plan: Product Search Pricing & Inventory Variants (+ Read-Only Queries)

- **Feature ID / Slug**: `product-search-pricing-variants`
- **Date**: 2026-11-02
- **Status**: Completed <!-- Draft | Approved | In Progress | Completed -->

---

## 1. Overview & Goals

In [`golem-shopping-effect`](file:///Users/coon/workspace-zv/git/golem-shopping-effect/src/agents/product-search/product-search-agent.ts), `ProductSearchAgent` provides enriched endpoints that combine catalog product details with active regional prices (from `PricingAgent`) and real-time inventory availability (from `InventoryAgent`). It also provides batch product retrieval by comma-separated product IDs (`getByIds`).

Furthermore, to maximize performance, cache efficiency, and throughput of both batch lookups and search sweeps across many products, we will mark the underlying query methods in `ProductAgent`, `PricingAgent`, and `InventoryAgent` as **`#[read_only]`**.

### Goals

1. **`ProductWithPrice` Schema**:
   - Define a schema struct containing core `Product` fields plus `price: Option<PricingItem>` and `available_quantity: Option<u32>`.
2. **Batch Lookup (`get_by_ids`)**:
   - `GET /v1/product/?ids={ids}`: Retrieve multiple products concurrently given a comma-separated list of product IDs (e.g. `ap001,sm002`).
3. **Batch Lookup with Prices & Inventory (`get_by_ids_with_prices`)**:
   - `GET /v1/product/with-prices?ids={ids}&currency={currency}`: Retrieve multiple products with active pricing and available quantity.
4. **Search with Prices & Inventory (`search_with_prices`)**:
   - `GET /v1/product/search-with-prices?query={query}&currency={currency}`: Search products matching free-text and field queries, returning them enriched with active price and available stock.
5. **AI / LLM Discovery Annotations**:
   - Annotate endpoints with `#[prompt(...)]` and `#[description(...)]` for AI / Shopping Assistant tool discovery, matching Effect's prompt hints.
6. **Mount Path Alignment**:
   - Align `ProductSearchAgent` mount to `/v1/product`:
     - `GET /v1/product/?ids={ids}`
     - `GET /v1/product/search?query={query}`
     - `GET /v1/product/with-prices?ids={ids}&currency={currency}`
     - `GET /v1/product/search-with-prices?query={query}&currency={currency}`
   - Full compatibility with existing search callers.
7. **Read-Only Caching on Underlying Query Methods**:
   - **`ProductAgent::get_product`**: Mark `#[read_only]` for pure state read.
   - **`PricingAgent::get_price` & `get_pricing`**: Mark `#[read_only]` for worker-level price caching per `(currency, region)` until price updates.
   - **`InventoryAgent::get_available_quantity` & `get_inventory`**: Mark `#[read_only]` for worker-level stock availability caching per region until allocations/stock changes.
8. **Documentation & Architecture**:
   - Update `architecture.puml` and `README.md` to document the new endpoints and edges to `PricingAgent` and `InventoryAgent`.

### Non-Goals

- Changing the underlying business logic of price calculation or inventory reservations.
- Changing `ShoppingAssistantAgent` recommendation logic.

---

## 2. Technical Design & Architecture

### 2.1 Communication & Caching Architecture

```mermaid
flowchart TD
    Client["Client / HTTP Gateway / Shopping Assistant\nGET /v1/product/search-with-prices?query=phone&currency=USD"] --> PSA["ProductSearchAgent\n(Ephemeral Agent mounted at /v1/product)"]
    PSA -->|1. Discover & Filter Products| Discovery["Product Catalog Discovery\n(GetAgents prefix 'ProductAgent(')"]
    Discovery -->|Matching Products| PSA
    PSA -->|2. Concurrent RPC get_price()| Pricing["PricingAgent(product_id)\n[read_only cached until write]"]
    PSA -->|3. Concurrent RPC get_available_quantity()| Inventory["InventoryAgent(product_id)\n[read_only cached until write]"]
    Pricing -->|PricingItem| PSA
    Inventory -->|Available Quantity| PSA
    PSA -->|4. Return Enriched Structs| Client["Vec<ProductWithPrice>"]
```

### 2.2 Schema: `ProductWithPrice`

```rust
use crate::pricing::PricingItem;

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ProductWithPrice {
    pub product_id: String,
    pub name: String,
    pub brand: String,
    pub description: String,
    pub tags: Vec<String>,
    pub price: Option<PricingItem>,
    pub available_quantity: Option<u32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

### 2.3 Agent Definition: `ProductSearchAgent`

```rust
#[agent_definition(ephemeral, mount = "/v1/product")]
pub trait ProductSearchAgent {
    fn new() -> Self;

    #[endpoint(get = "/?ids={ids}")]
    #[description("Get products by comma-separated product IDs")]
    async fn get_by_ids(&self, ids: String) -> Result<Vec<Product>, String>;

    #[endpoint(get = "/search?query={query}")]
    #[description("Search products across the catalog by query keyword or field filter")]
    async fn search(&self, query: String) -> Result<Vec<Product>, String>;

    #[endpoint(get = "/with-prices?ids={ids}&currency={currency}")]
    #[prompt("Call this to fetch specifications, descriptions, brands, tags, and prices for specific products.")]
    #[description("Get detailed product information with current pricing by product ID(s).")]
    async fn get_by_ids_with_prices(
        &self,
        ids: String,
        currency: Option<String>,
    ) -> Result<Vec<ProductWithPrice>, String>;

    #[endpoint(get = "/search-with-prices?query={query}&currency={currency}")]
    #[prompt("Call this when the user is searching for products, asking recommendations, or looking for item options.")]
    #[description("Search product catalog with keyword, brand, or tag filters including active prices.")]
    async fn search_with_prices(
        &self,
        query: String,
        currency: Option<String>,
    ) -> Result<Vec<ProductWithPrice>, String>;
}
```

### 2.4 Concurrent Enrichment Helper

```rust
async fn attach_pricing_to_products(
    products: Vec<Product>,
    currency: Option<String>,
) -> Vec<ProductWithPrice> {
    if products.is_empty() {
        return Vec::new();
    }

    let target_currency = currency.unwrap_or_else(|| CURRENCY_DEFAULT.to_string());
    let target_region = PRICING_REGION_DEFAULT.to_string();
    let inventory_region = INVENTORY_REGION_DEFAULT.to_string();

    let tasks: Vec<_> = products
        .into_iter()
        .map(|product| {
            let pid = product.product_id.clone();
            let curr = target_currency.clone();
            let reg = target_region.clone();
            let inv_reg = inventory_region.clone();

            async move {
                let pricing_client = PricingAgentClient::get(pid.clone());
                let inv_client = InventoryAgentClient::get(pid);

                let (price, avail) = futures::future::join(
                    pricing_client.get_price(curr, reg),
                    async { inv_client.get_available_quantity(inv_reg) },
                )
                .await;

                ProductWithPrice {
                    product_id: product.product_id,
                    name: product.name,
                    brand: product.brand,
                    description: product.description,
                    tags: product.tags,
                    price,
                    available_quantity: Some(avail.available_quantity),
                    created_at: product.created_at,
                    updated_at: product.updated_at,
                }
            }
        })
        .collect();

    join_all(tasks).await
}
```

### 2.5 Read-Only Annotations on Dependent Agents

1. **`src/product/mod.rs`**:
   ```rust
   use golem_rust::{Schema, agent_definition, agent_implementation, endpoint, read_only};

   #[agent_definition(mount = "/v1/product/{id}")]
   pub trait ProductAgent {
       fn new(id: String) -> Self;

       #[read_only]
       #[endpoint(get = "/")]
       fn get_product(&self) -> Option<Product>;
       ...
   ```
2. **`src/pricing/mod.rs`**:
   ```rust
   use golem_rust::{Schema, agent_definition, agent_implementation, endpoint, read_only};

   #[agent_definition(mount = "/v1/pricing/{id}")]
   pub trait PricingAgent {
       fn new(id: String) -> Self;

       #[read_only]
       #[endpoint(get = "/")]
       fn get_pricing(&self) -> Option<Pricing>;

       #[read_only]
       fn get_price(&self, currency: String, region: String) -> Option<PricingItem>;
       ...
   ```
3. **`src/inventory/mod.rs`**:
   ```rust
   use golem_rust::{Schema, agent_definition, agent_implementation, endpoint, read_only};

   #[agent_definition(mount = "/v1/inventory/{id}")]
   pub trait InventoryAgent {
       fn new(id: String) -> Self;

       #[read_only]
       #[endpoint(get = "/")]
       fn get_inventory(&self) -> Option<Inventory>;

       #[read_only]
       #[endpoint(get = "/availability?region={region}")]
       fn get_available_quantity(&self, region: String) -> Availability;
       ...
   ```

---

## 3. Proposed Changes & File Impact

| Action | File Path | Description |
| :--- | :--- | :--- |
| `[MODIFY]` | [`src/product_search/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/product_search/mod.rs) | Add `ProductWithPrice`, `fetch_products_by_ids`, `attach_pricing_to_products`, endpoints `get_by_ids`, `get_by_ids_with_prices`, `search_with_prices`, prompt annotations, and unit tests. |
| `[MODIFY]` | [`src/product/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/product/mod.rs) | Add `#[read_only]` to `get_product`. |
| `[MODIFY]` | [`src/pricing/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/pricing/mod.rs) | Add `#[read_only]` to `get_price` and `get_pricing`. |
| `[MODIFY]` | [`src/inventory/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/inventory/mod.rs) | Add `#[read_only]` to `get_available_quantity` and `get_inventory`. |
| `[MODIFY]` | [`architecture.puml`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/architecture.puml) | Add RPC arrows `product_search --> pricing` and `product_search --> inventory`; update gateway note. |
| `[MODIFY]` | [`README.md`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/README.md) | Update `Product Search Agent` description to reflect pricing and inventory search capabilities. |

---

## 4. Verification Plan

### Automated Checks
1. **Formatting**:
   ```bash
   cargo fmt --check
   ```
2. **Type Checking**:
   ```bash
   cargo check --target wasm32-wasip2
   ```
3. **Clippy Linter**:
   ```bash
   cargo clippy --target wasm32-wasip2
   ```
4. **Release Compilation**:
   ```bash
   cargo build --target wasm32-wasip2 --release
   ```
5. **Unit Tests**:
   - Test comma-separated ID parsing and filtering.
   - Test `ProductWithPrice` construction and serialization.

---

## 5. User Review & Approval Gate

Please review this expanded implementation plan. Once approved, we will proceed with the implementation, verification, and walkthrough documentation.
