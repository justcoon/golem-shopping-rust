# Feature Plan: Cart Saved Contact Info & Order History

- **Feature ID / Slug**: `cart-saved-contact-and-history`
- **Date**: 2026-11-02
- **Status**: Completed <!-- Draft | Approved | In Progress | Completed -->

---

## 1. Overview & Goals

In [`golem-shopping-effect`](file:///Users/coon/workspace-zv/git/golem-shopping-effect/src/agents/cart/cart-agent.ts), `CartAgent` provides convenience methods to simplify customer checkout and order tracking:
1. **`getSavedContactInfo`**: Inspects previous order history to extract the last-used shipping address, billing address, and email for 1-click checkout preview.
2. **`applySavedContactInfo`**: Applies the saved addresses and contact email from the customer's last order directly onto the active cart.
3. **`getOrderHistory`**: Retrieves the customer's previous `Order` instances (sorted newest first, with configurable limit).

This feature brings these capabilities to the Rust implementation in [`src/cart/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/cart/mod.rs).

### Goals

1. **`SavedContactInfo` Schema**:
   - Define a schema struct containing `shipping_address: Option<Address>`, `billing_address: Option<Address>`, `email: Option<String>`, and `source_order_id: Option<String>`.
2. **Get Saved Contact Info (`get_saved_contact_info`)**:
   - `GET /v1/cart/{id}/saved-contact`: Inspects `previous_order_ids`, loads the most recent order via `OrderAgentClient`, and returns the addresses/email.
3. **Apply Saved Contact Info (`apply_saved_contact_info`)**:
   - `POST /v1/cart/{id}/use-saved-contact`: Loads contact info from the most recent order and applies it to the active cart's `shipping_address`, `billing_address`, and `email`.
4. **Get Order History (`get_order_history`)**:
   - `GET /v1/cart/{id}/orders?limit={limit}`: Fetches the customer's past orders in reverse chronological order (newest first) up to `limit` (default 10).
5. **AI / LLM Discovery Annotations**:
   - Annotate these methods with `#[prompt(...)]` and `#[description(...)]` to facilitate 1-click checkout and past-order queries by the `ShoppingAssistantAgent` and external AI tools.
6. **Architecture & Documentation**:
   - Update `architecture.puml` and `README.md` to document the new endpoints.

### Non-Goals

- Storing a separate contact book database table (derives contact info directly from past orders recorded in `previous_order_ids`).
- Changing existing cart item management, pricing refresh, or checkout logic.

---

## 2. Technical Design & Architecture

### 2.1 Communication Flow

```mermaid
flowchart TD
    Client["Client / HTTP Gateway / Shopping Assistant\nGET /v1/cart/{user_id}/saved-contact"] --> CartAgent["CartAgent(user_id)"]
    CartAgent -->|1. Inspect previous_order_ids| State["Cart State\n(previous_order_ids: [ord-1, ord-2])"]
    CartAgent -->|2. RPC get_order() on last order| OrderAgent["OrderAgent(ord-2)"]
    OrderAgent -->|Order State (addresses, email)| CartAgent
    CartAgent -->|3. Return SavedContactInfo or apply to cart| Client
```

### 2.2 Schema: `SavedContactInfo`

```rust
use crate::common::Address;

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SavedContactInfo {
    pub shipping_address: Option<Address>,
    pub billing_address: Option<Address>,
    pub email: Option<String>,
    pub source_order_id: Option<String>,
}
```

### 2.3 Agent Trait Updates

In [`src/cart/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/cart/mod.rs):

```rust
#[agent_definition(mount = "/v1/cart/{id}")]
pub trait CartAgent {
    fn new(id: String) -> Self;

    // Existing methods...
    #[endpoint(get = "/")]
    async fn get_cart(&mut self) -> Option<Cart>;

    #[endpoint(put = "/items/{product_id}")]
    async fn add_item(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<CartUpdated, AddItemError>;

    #[endpoint(post = "/checkout")]
    async fn checkout(&mut self) -> Result<OrderConfirmation, CheckoutError>;

    #[endpoint(put = "/email")]
    fn update_email(&mut self, email: String) -> Result<CartUpdated, UpdateEmailError>;

    fn clear(&mut self);

    #[endpoint(delete = "/items/{product_id}")]
    fn remove_item(&mut self, product_id: String) -> Result<CartUpdated, RemoveItemError>;

    #[endpoint(put = "/billing-address")]
    fn update_billing_address(
        &mut self,
        street: String,
        city: String,
        state_or_region: String,
        country: String,
        postal_code: String,
        name: Option<String>,
        phone_number: Option<String>,
    ) -> Result<CartUpdated, UpdateAddressError>;

    async fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<CartUpdated, UpdateItemQuantityError>;

    #[endpoint(put = "/shipping-address")]
    fn update_shipping_address(
        &mut self,
        street: String,
        city: String,
        state_or_region: String,
        country: String,
        postal_code: String,
        name: Option<String>,
        phone_number: Option<String>,
    ) -> Result<CartUpdated, UpdateAddressError>;

    // NEW METHODS:

    #[endpoint(get = "/saved-contact")]
    #[prompt("Call this before checkout to check if the user has saved shipping/billing details from previous orders.")]
    #[description("Retrieves previously saved contact information and addresses from the user's past orders.")]
    async fn get_saved_contact_info(&self) -> Option<SavedContactInfo>;

    #[endpoint(post = "/use-saved-contact")]
    #[prompt("Call this when the user confirms they want to use their saved address and contact info for 1-click checkout.")]
    #[description("Applies saved contact details and addresses from previous orders to the current cart.")]
    async fn apply_saved_contact_info(&mut self) -> Result<CartUpdated, UpdateAddressError>;

    #[endpoint(get = "/orders?limit={limit}")]
    #[prompt("Call this when the user asks about their previous orders, order history, or recent purchases.")]
    #[description("Retrieves past orders placed by this customer.")]
    async fn get_order_history(&self, limit: Option<u32>) -> Vec<crate::order::Order>;
}
```

### 2.4 Method Implementation Details

1. **`get_saved_contact_info`**:
   - Inspects `self.state.as_ref()?.previous_order_ids`.
   - Takes the last order ID (`last_order_id`).
   - Fetches `OrderAgentClient::get(last_order_id.clone()).get_order().await`.
   - Returns `Some(SavedContactInfo { shipping_address: order.shipping_address, billing_address: order.billing_address, email: order.email, source_order_id: Some(last_order_id) })`.

2. **`apply_saved_contact_info`**:
   - Checks `self.get_state().previous_order_ids`.
   - If empty, returns `Err(UpdateAddressError::AddressNotValid(AddressNotValidError { message: "No previous orders found to apply saved contact info".to_string() }))`.
   - Takes the last order ID and calls `OrderAgentClient::get(last_order_id.clone()).get_order().await`.
   - If order not found, returns `Err(UpdateAddressError::AddressNotValid(...))`.
   - Copies `shipping_address`, `billing_address`, and `email` to current cart.
   - Updates `updated_at = chrono::Utc::now()`.
   - Returns `Ok(CartUpdated { user_id })`.

3. **`get_order_history`**:
   - Takes `previous_order_ids` in reverse order (newest first).
   - Limits to `limit.unwrap_or(10) as usize`.
   - Concurrently fetches order details using `futures::future::join_all` calling `OrderAgentClient::get(id).get_order()`.
   - Flattens and returns `Vec<Order>`.

---

## 3. Proposed Changes & File Impact

| Action | File Path | Description |
| :--- | :--- | :--- |
| `[MODIFY]` | [`src/cart/mod.rs`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/src/cart/mod.rs) | Add `SavedContactInfo`, prompt/description imports, and implementations of `get_saved_contact_info`, `apply_saved_contact_info`, and `get_order_history`. Add unit tests. |
| `[MODIFY]` | [`architecture.puml`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/architecture.puml) | Document `/v1/cart/{id}/saved-contact`, `/use-saved-contact`, `/orders` in gateway notes. |
| `[MODIFY]` | [`README.md`](file:///Users/coon/workspace-zv/git/golem-shopping-rust/README.md) | Update `Cart Agent` description with saved contact and order history features. |

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
   - Test `SavedContactInfo` construction and serialization.
   - Test cart state address application logic.

---

## 5. User Review & Approval Gate

Please review this implementation plan. Once approved, we will proceed with the implementation, verification, and walkthrough documentation.

