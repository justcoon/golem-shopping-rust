# Feature Implementation Walkthrough: Per-Product Regional Inventory Management

- **Feature ID / Slug**: `product-inventory`
- **Date**: 2026-11-02
- **Status**: Ready for Review <!-- Ready for Review | Approved -->

---

## 1. Executive Summary

This feature brings durable, region-aware inventory management to the Golem Shopping Rust application, completing parity with `golem-shopping-effect`. 

Key outcomes:
1. Created `InventoryAgent` ([`src/inventory/mod.rs`](../../src/inventory/mod.rs)), a durable Golem agent mounted at `/v1/inventory/{id}` managing regional stock quantities, order reservations, and stock fulfillment with snapshot durability.
2. Updated `CartAgent` ([`src/cart/mod.rs`](../../src/cart/mod.rs)) to perform soft regional stock validation in `add_item` and `update_item_quantity`, protecting customers from over-ordering with informative `InsufficientStockError` responses.
3. Updated `OrderAgent` ([`src/order/mod.rs`](../../src/order/mod.rs)) with hard reservations during `initialize_order`, automatic non-blocking Saga rollback compensation via `trigger_release_reservation` upon partial reservation failures, dynamic reservation updates during order modification (`add_item`, `update_item_quantity`, `remove_item` via `trigger_release_reservation`), fire-and-forget reservation releases upon cancellation (`cancel_order` via `trigger_release_reservation`), and asynchronous regional stock deductions upon shipping (`ship_order` via `trigger_fulfill_reservation`).
4. Updated manifest configuration in [`golem.yaml`](../../golem.yaml), data fixtures and import configs in [`data/import.yaml`](../../data/import.yaml) and [`data/README.md`](../../data/README.md), and architecture documentation in [`architecture.puml`](../../architecture.puml) and [`README.md`](../../README.md).

---

## 2. Changes Implemented

### File Modifications

| Action | File Path | Summary of Changes |
| :--- | :--- | :--- |
| `[NEW]` | [`src/inventory/mod.rs`](../../src/inventory/mod.rs) | `InventoryAgent` definition, schemas (`InventoryItem`, `OrderAllocation`, `Inventory`, `Availability`, `ReserveQuantityError`), quantity math, snapshot persistence, and unit tests. |
| `[MODIFY]` | [`src/lib.rs`](../../src/lib.rs) | Registered and exported `pub mod inventory;`. |
| `[MODIFY]` | [`src/common/mod.rs`](../../src/common/mod.rs) | Added and exported `pub const INVENTORY_REGION_DEFAULT: &str = "global";`. |
| `[MODIFY]` | [`src/cart/mod.rs`](../../src/cart/mod.rs) | Added `InsufficientStockError`, integrated soft stock validation using `INVENTORY_REGION_DEFAULT` into `add_item` and `update_item_quantity`, and mapped inventory failures in `create_order`. |
| `[MODIFY]` | [`src/order/mod.rs`](../../src/order/mod.rs) | Added `InsufficientInventoryError`, integrated hard reservations using `INVENTORY_REGION_DEFAULT`, Saga rollback compensation, dynamic adjustments, fulfillment, and release via trigger methods. |
| `[MODIFY]` | [`golem.yaml`](../../golem.yaml) | Registered `InventoryAgent: {}` in `httpApi.deployments.local.agents`. |
| `[MODIFY]` | [`data/import.yaml`](../../data/import.yaml) | Added Drill plan step importing `./fixtures/tech_inventory.csv` to `/v1/inventory/{{ item.product_id }}`. |
| `[MODIFY]` | [`data/README.md`](../../data/README.md) | Documented `tech_inventory.csv` data structure and import step. |
| `[MODIFY]` | [`architecture.puml`](../../architecture.puml) | Added `Inventory Agent` component, gateway route `/v1/inventory/**`, and RPC communication edges. |
| `[MODIFY]` | [`README.md`](../../README.md) | Documented `Inventory Agent` in application overview and state management list. |

---

## 3. Verification & Validation Results

### 3.1 Code Formatting

Command executed:
```bash
cargo fmt --check
```

**Result**:
```text
(Clean exit with code 0 - all modified and newly created files adhere strictly to rustfmt standards)
```

### 3.2 Compilation & Type Checking

Command executed:
```bash
cargo check --target wasm32-wasip2
```

**Result**:
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.90s
(Clean exit with code 0 - zero compilation or type errors)
```

### 3.3 Linter Verification (Clippy)

Command executed:
```bash
cargo clippy --target wasm32-wasip2
```

**Result**:
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.92s
(Clean exit with code 0 - zero clippy errors or warnings on any new or modified inventory code)
```

### 3.4 Release Build

Command executed:
```bash
cargo build --target wasm32-wasip2 --release
```

**Result**:
```text
Compiling shopping_rust_shopping v0.0.1 (/Users/coon/workspace-zv/git/golem-shopping-rust)
Finished `release` profile [optimized] target(s) in 48.28s
(Clean exit with code 0 - optimized WASM component created)
```

---

## 4. How to Run & Verify

1. **Build and Deploy**:
   ```bash
   golem build --yes
   golem deploy --yes
   ```

2. **Seed Initial Inventory**:
   ```bash
   HOST=http://localhost:9006 API_HOST=localhost:9006 drill --benchmark data/import.yaml --stats
   ```

3. **Verify Inventory Endpoint**:
   ```bash
   # Check initial stock of product ap001
   curl http://localhost:9006/v1/inventory/ap001/availability?region=global
   # Response: {"product_id":"ap001","region":"global","available_quantity":10,"total_quantity":10,"allocated_quantity":0}
   ```

4. **Verify Cart Soft Stock Check**:
   ```bash
   # Attempt to add 15 items when only 10 available
   curl -X PUT http://localhost:9006/v1/cart/user-1/items/ap001 \
     -H "Content-Type: application/json" \
     -d '{"quantity": 15}'
   # Returns InsufficientStock error: Requested 15, available 10

   # Add valid quantity (4)
   curl -X PUT http://localhost:9006/v1/cart/user-1/items/ap001 \
     -H "Content-Type: application/json" \
     -d '{"quantity": 4}'
   # Returns CartUpdated
   ```

5. **Verify Checkout & Order Lifecycle**:
   ```bash
   # Checkout cart
   curl -X POST http://localhost:9006/v1/cart/user-1/checkout
   # Order created, inventory shows:
   # allocated_quantity: 4, available_quantity: 6, total_quantity: 10

   # Ship order -> total deducted, allocation cleared:
   # allocated_quantity: 0, available_quantity: 6, total_quantity: 6
   ```

---

## 5. Sign-off / Next Steps

- All implementation and validation criteria from Stage 3 are satisfied.
- Ready for final user sign-off (Stage 5).

