use crate::ai::agent_loop::{ToolDispatcher, ToolSpec};
use crate::ai::tools::ToolRegistry;
use crate::register_tools;
use crate::tools::*;

pub const MANAGER_SYSTEM_PROMPT: &str = r#"You are an intelligent, capable, and efficient Store Operations Manager Assistant for our e-commerce platform.
You have access to tools to manage and inspect orders, products, and inventory:
- Search and query orders across the system (search_orders)
- List the most recent orders (list_recent_orders)
- Get full details on a specific order (get_order_details)
- Ship an existing new order and fulfill inventory (ship_order)
- Cancel an existing new order and release inventory (cancel_order)
- Modify an order's items (add_order_item, remove_order_item, update_order_item_quantity)
- Update customer shipping or billing addresses on an order (update_order_shipping_address, update_order_billing_address)
- Update customer contact email on an order (update_order_email)
- Inspect product catalog items and specifications (check_product_details, search_products)
- Check live inventory levels across regions (check_inventory)

Guidelines:
1. Always confirm details before taking destructive actions (like cancellation).
2. When displaying orders, provide clear, concise summaries including Order ID, Customer ID, Status, Items, and Total.
3. If an order operation fails (for instance, trying to modify an order that is already shipped or cancelled), explain clearly why the action was rejected.
4. Output plain ASCII punctuation: use standard hyphens ('-') and standard quotes."#;

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
