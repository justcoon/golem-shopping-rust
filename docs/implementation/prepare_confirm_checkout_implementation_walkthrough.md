# Implementation Walkthrough: Two-Phase Checkout with Inventory Reservation & Expiration

- **Feature ID / Slug**: `prepare-confirm-checkout`
- **Date**: 2026-11-05
- **Status**: Completed

---

## 1. Overview of Changes

We successfully refactored the checkout architecture from a single-turn `checkout` operation into an enterprise-grade **Two-Phase Checkout (`prepare_checkout` $\to$ `confirm_checkout`)** with time-bounded inventory reservation, cart locking invariants, and frontend transparency.

### Key Highlights:
1. **Complete Removal of Legacy `checkout()` & `CheckoutError`**:
   - The legacy single-turn `checkout` function, `CheckoutError`, and `checkoutCart` APIs have been completely removed across the backend, assistant tools, and frontend.
   - Enforces that all checkout interactions strictly follow the two-phase flow (`prepare_checkout` $\to$ `confirm_checkout`).
2. **Explicit Expiration Timestamp Across Distributed Inventory (`src/inventory/mod.rs`)**:
   - `reserve_quantity` accepts `expires_at: Option<chrono::DateTime<chrono::Utc>>` instead of relative TTL seconds.
   - Allows `CartAgent` to compute a single, unified `expires_at` timestamp for the entire order draft and pass the exact same timestamp to every product's `InventoryAgent`, eliminating clock skew and ensuring all items expire concurrently.
   - Lazy purge `purge_expired_allocations()` and active availability calculation automatically reclaim expired reservations.
3. **Lean Cart State & Invariant Locking (`src/cart/mod.rs`)**:
   - Lightweight `CheckoutDraft { order_id, expires_at, created_at }` embedded in `Cart` without duplicating items or addresses.
   - `cart.items` are retained during the reservation and only emptied upon successful `confirm_checkout`.
   - `get_cart()` freezes item and price re-fetching during active checkout.
   - Any mutating operations (`add_item`, `remove_item`, `update_quantity`, `update_address`, `update_email`) while checkout is active return `ActionNotAllowedError`.
4. **AI Assistant HITL Tooling (`src/shopping_assistant/tools.rs`)**:
   - Replaced single `checkout_cart` with `prepare_checkout`, `confirm_checkout(orderId)`, and `cancel_checkout(orderId)`.
   - Updated `SHOPPING_ASSISTANT_SYSTEM_PROMPT` to guide the assistant through the two-phase review and confirmation flow.
5. **Frontend Reactive Experience (`frontend/`)**:
   - `cartStore` exposes `isCheckoutActive` and `activeDraft`.
   - `CheckoutView.vue` features a live 10-minute countdown timer (`MM:SS`), "Confirm & Place Order" and "Cancel Reservation" actions.
   - `CartView.vue` displays an active hold banner and locks item editing while checkout is active.

---

## 2. File-by-File Changes

| File | Purpose |
| :--- | :--- |
| `src/inventory/mod.rs` | Added `expires_at` to `OrderAllocation`, `purge_expired_allocations()`, and TTL support in `reserve_quantity`. |
| `src/order/mod.rs` | Updated `reserve_quantity` calls in order initialization and modifications to pass `None` (unbounded allocation). |
| `src/cart/mod.rs` | Added `CheckoutDraft`, `ActionNotAllowedError`, `prepare_checkout`, `confirm_checkout`, `cancel_checkout`, and frozen `get_cart`. |
| `src/shopping_assistant/tools.rs` | Replaced `checkout_cart` with `prepare_checkout`, `confirm_checkout`, `cancel_checkout`, and updated system prompt. |
| `frontend/src/api/services/cartService.ts` | Added `CheckoutDraft` interface, `prepareCheckout`, `confirmCheckout`, and `cancelCheckout` APIs. |
| `frontend/src/stores/cartStore.ts` | Added `isCheckoutActive`, `activeDraft`, and actions for prepare, confirm, and cancel checkout. |
| `frontend/src/views/CheckoutView.vue` | 2-step checkout flow with active reservation banner, countdown timer, and confirm/cancel buttons. |
| `frontend/src/views/CartView.vue` | Added active reservation banner and mutation guards when checkout is in progress. |
| `frontend/src/stores/assistantStore.ts` | Added `prepare_checkout`, `confirm_checkout`, `cancel_checkout` to cart-mutating tool watchers. |

---

## 3. Code Highlights

### Cart Locking Invariant (`src/cart/mod.rs`)

```rust
pub fn ensure_modifiable(&mut self) -> Result<(), ActionNotAllowedError> {
    self.purge_expired_draft();
    if let Some(ref draft) = self.draft {
        return Err(ActionNotAllowedError {
            message: format!(
                "Cannot modify cart while checkout is in progress for order #{}. Please confirm or cancel the active checkout first.",
                draft.order_id
            ),
        });
    }
    Ok(())
}
```

### Frozen `get_cart()` During Active Draft (`src/cart/mod.rs`)

```rust
async fn get_cart(&mut self) -> Option<Cart> {
    info!("Getting cart");
    if let Some(cart) = self.state.as_mut() {
        cart.purge_expired_draft();
        if cart.is_in_checkout() {
            // Freezes cart state during active checkout draft (no dynamic price re-fetching)
            return Some(cart.clone());
        }

        let mut items = Vec::new();
        for item in cart.items.clone() {
            // ... join product & pricing ...
        }
        cart.set_items(items);
        Some(cart.clone())
    } else {
        None
    }
}
```

### Fire-and-Forget Trigger Calls (`trigger_release_reservation` / `trigger_fulfill_reservation`)

```rust
// In prepare_checkout compensation rollback and cancel_checkout:
for item in &state.items {
    let mut inv_client = InventoryAgentClient::get(item.product_id.clone());
    inv_client.trigger_release_reservation(order_id.clone());
}

// In confirm_checkout fulfillment:
for item in &state.items {
    let mut inv_client = InventoryAgentClient::get(item.product_id.clone());
    inv_client.trigger_fulfill_reservation(order_id.clone());
}
```

### Two-Phase Assistant Tool Specification (`src/shopping_assistant/tools.rs`)

```rust
ToolSpec {
    name: "prepare_checkout".to_string(),
    description: "Validate cart, reserve items in inventory for 10 minutes, lock order ID, and prepare checkout summary. MUST be called first when user wants to checkout.".to_string(),
    parameters: json!({ "type": "object", "properties": {} }),
},
ToolSpec {
    name: "confirm_checkout".to_string(),
    description: "Place the order using the pregenerated order ID from prepare_checkout. ONLY call this tool after the user has explicitly confirmed the prepared order summary.".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "orderId": { "type": "string", "description": "The order ID generated by prepare_checkout" }
        },
        "required": ["orderId"]
    }),
},
```

---

## 4. Verification Results

### 1. Code Formatting (`cargo fmt --check`)
```text
$ cargo fmt --check
(exited with 0)
```

### 2. Compilation Check (`cargo check --target wasm32-wasip2`)
```text
$ cargo check --target wasm32-wasip2
    Checking shopping_rust_shopping v0.0.1 (/Users/coon/workspace-zv/git/golem-shopping-rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.28s
(0 errors, 0 warnings)
```

### 3. Unit Tests (`cargo test`)
```text
$ cargo test
running 23 tests
test ai::agent_loop::tests::test_extract_json_str ... ok
test ai::agent_loop::tests::test_sse_decoder ... ok
test ai::agent_loop::tests::test_streamed_tool_call_builder ... ok
test ai::agent_loop::tests::test_noop_dispatcher ... ok
test ai::tests::test_resolve_endpoint_url ... ok
test cart::tests::test_saved_contact_info_construction ... ok
test ai::tests::test_tool_call_record_and_chunk_serialization ... ok
test cart::tests::test_cart_draft_expiration ... ok
test cart::tests::test_cart_draft_lifecycle_and_locking ... ok
test ai::tests::test_missing_api_key_returns_graceful_error ... ok
test inventory::tests::test_availability_calculation ... ok
test inventory::tests::test_reservation_expiration ... ok
test ai::agent_loop::tests::test_gemini_thought_signature_preservation ... ok
test ai::agent_loop::tests::test_gemini_thought_signature_fallback_when_missing ... ok
test ai::tests::test_chat_schema_serialization ... ok
test inventory::tests::test_reserve_over_limit_fails ... ok
test inventory::tests::test_release_and_fulfill ... ok
test order_search::tests::test_query_matcher_quoted_terms ... ok
test order_search::tests::test_query_matcher_field_filters ... ok
test order_search::tests::test_query_matcher_terms ... ok
test product_search::tests::test_query_matcher_terms ... ok
test product_search::tests::test_extract_product_agent_id ... ok
test order_search::tests::test_extract_order_agent_id ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 4. Frontend Build (`npx vite build`)
```text
$ cd frontend && npx vite build
✓ 765 modules transformed.
dist/index.html                   0.75 kB │ gzip:  0.42 kB
dist/assets/index-DUmwNJ6k.css   60.97 kB │ gzip: 10.61 kB
dist/assets/index-aWmaUvXK.js   215.33 kB │ gzip: 76.19 kB
✓ built in 1.94s
```

