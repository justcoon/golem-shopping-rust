use crate::ai::agent_loop::{ToolDispatcher, ToolSpec};
use crate::ai::tools::ToolRegistry;
use crate::register_tools;
use crate::tools::*;

pub const MANAGER_SYSTEM_PROMPT: &str = r#"You are the Operations Manager Assistant for our e-commerce platform. Help store managers inspect orders, track inventory, update order details, and process shipments. Politely decline requests unrelated to store operations.

### Core Rules
1. **Style & Formatting:** Be concise and direct. Use plain ASCII punctuation only (standard hyphens '-' and straight quotes '"').
2. **Strict IDs & Lookup First:** Never guess or invent an `orderId` or `productId`. Always retrieve and verify them via read tools (`search_orders`, `list_recent_orders`, `get_order_details`, `search_products`) before calling any mutation tool. If a search returns multiple matching orders, list them and ask the manager to specify the exact `orderId`.
3. **Order Lifecycle & Mutability:**
   - Check the order's current status with `get_order_details` before attempting any change.
   - Only orders with status `New` can be shipped, cancelled, or modified (items, quantities, addresses, email).
   - Orders with status `Shipped` or `Cancelled` are immutable. Do not attempt to modify them; explain clearly that their current status prevents changes.
4. **Cancellations (Confirmation Required):** Never call `cancel_order` immediately on a first request. First verify the order is `New`, display its summary (Order ID, Customer ID, Status, Total), and ask for explicit confirmation. Only execute `cancel_order` after the user confirms in a subsequent message.
5. **Order Presentation:** When displaying orders, present a clean summary containing: Order ID, Customer ID, Status, Items list, and Total."#;

/// Builds the store manager tool registry by mounting the Golem tool implementations.
pub fn build_manager_tools() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    register_tools!(registry, CatalogTools, use_impl: true, [
        search_products => |products| format!("Found {} product(s)", products.len()),
        get_product_details => |p| match p {
            Some(p) => format!("Details for {} ({})", p.name, p.product_id),
            None => "Product not found".to_string(),
        },
    ]);

    register_tools!(registry, InventoryTools, use_impl: true, [
        check_inventory => |avail| {
            format!(
                "Product {} has {} available in region {} (total: {})",
                avail.product_id, avail.available_quantity, avail.region, avail.total_quantity
            )
        },
    ]);

    register_tools!(registry, ManagerOrderTools, use_impl: true, [
        search_orders => |orders| format!("Found {} matching order(s)", orders.len()),
        list_recent_orders => |orders| format!("Retrieved {} recent order(s)", orders.len()),
        get_order_details => |o| match o {
            Some(o) => format!(
                "Retrieved order {} (Status: {:?}, Total: ${:.2}, Items: {})",
                o.order_id, o.order_status, o.total, o.items.len()
            ),
            None => "Order not found".to_string(),
        },
        ship_order => |u| format!("Order {} has been successfully shipped", u.order_id),
        cancel_order => |u| format!("Order {} has been successfully cancelled", u.order_id),
        add_order_item => |u| format!("Added item to order {}", u.order_id),
        remove_order_item => |u| format!("Removed item from order {}", u.order_id),
        update_order_item_quantity => |u| format!("Updated item quantity on order {}", u.order_id),
        update_order_shipping_address => |u| format!("Updated shipping address on order {}", u.order_id),
        update_order_billing_address => |u| format!("Updated billing address on order {}", u.order_id),
        update_order_email => |u| format!("Updated customer email on order {}", u.order_id),
    ]);

    registry
}

/// Returns the tool specifications for the manager agent.
pub fn get_manager_tools() -> Vec<ToolSpec> {
    build_manager_tools().specs()
}

/// Executes a tool call by delegating to the generic `ToolRegistry` dispatcher.
pub async fn execute_manager_tool(
    _manager_id: &str,
    tool_name: &str,
    args_json: &str,
) -> (String, String) {
    let registry = build_manager_tools();
    registry.execute(tool_name, args_json).await
}
