# Feature Implementation Plan: Saga Pattern Transactions for Multi-Item Inventory Reservations

Migrate manual inventory reservation and compensation loops in `CartAgent` and `OrderAgent` to a shared, reusable Golem native saga pattern (`golem_rust::fallible_transaction` and `golem_rust::operation`) in `src/inventory/saga.rs`.

---

## User Review Required

> [!IMPORTANT]
> **Scope & Architecture**:
> - We are introducing a shared `reserve_inventory_saga` function in `src/inventory/saga.rs` (re-exported by `src/inventory/mod.rs`).
> - `CartAgent::prepare_checkout` (in `src/cart/mod.rs`) and `OrderAgent::initialize_order` (in `src/order/mod.rs`) will both consume this shared saga helper.
> - This replaces custom manual tracking vectors (`reserved_products: Vec<String>`) and manual compensation loops with durable, crash-resilient saga transactions managed directly by Golem's runtime oplog.

---

## Proposed Changes

### 1. `src/inventory/saga.rs` [NEW]
- Define `reserve_inventory_saga`:
  ```rust
  pub async fn reserve_inventory_saga(
      items: &[(String, u32)],
      order_id: &str,
      region: &str,
      expires_at: Option<chrono::DateTime<chrono::Utc>>,
  ) -> Result<(), ReserveQuantityError>;
  ```
- Uses `golem_rust::operation` with forward execution (`reserve_quantity`) and inverse compensation (`trigger_release_reservation`).
- Uses `golem_rust::fallible_transaction` to coordinate execution across multiple products with automatic reverse-order compensation upon any failure.
- Handles `golem_rust::TransactionFailure` variants and returns the underlying `ReserveQuantityError`.
- Add unit tests for transaction failure matching and unwrapping.

### 2. `src/inventory/mod.rs` [MODIFY]
- Declare `pub mod saga;` and `pub use saga::reserve_inventory_saga;`.

### 3. `src/cart/mod.rs` [MODIFY]
- Replace `reserve_inventory_for_cart` manual tracking and loop with a call to `crate::inventory::reserve_inventory_saga`.
- Map `ReserveQuantityError` to `PrepareCheckoutError`.
- Remove manual `release_inventory_reservations` helper if no longer needed.

### 4. `src/order/mod.rs` [MODIFY]
- Replace `initialize_order` manual reservation loop and rollback with a call to `crate::inventory::reserve_inventory_saga`.
- Map `ReserveQuantityError` to `InitOrderError`.

---

## File-by-File Changes

| File | Change Type | Description |
|---|---|---|
| `src/inventory/saga.rs` | `[NEW]` | Reusable multi-item inventory reservation saga using `fallible_transaction` and `operation`. |
| `src/inventory/mod.rs` | `[MODIFY]` | Re-export `reserve_inventory_saga`. |
| `src/cart/mod.rs` | `[MODIFY]` | Consume `reserve_inventory_saga` in `prepare_checkout`. |
| `src/order/mod.rs` | `[MODIFY]` | Consume `reserve_inventory_saga` in `initialize_order`. |
| `docs/implementation/saga-transactions_feature_plan.md` | `[MODIFY]` | Implementation plan (Status: Completed). |

---

## Verification Plan

### Automated Checks
1. Code formatting check:
   ```bash
   cargo fmt --check
   ```
2. WebAssembly compilation check:
   ```bash
   cargo check --target wasm32-wasip2
   ```
3. Clippy lints with zero warnings:
   ```bash
   cargo clippy --target wasm32-wasip2
   ```
4. Full test suite:
   ```bash
   cargo test
   ```
