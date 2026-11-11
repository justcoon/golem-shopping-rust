use crate::ai::agent_loop::{ToolDispatcher, ToolSpec};
use crate::ai::tools::ToolRegistry;
use crate::tools::catalog::CatalogToolsImpl;
use crate::tools::inventory::InventoryToolsImpl;
use crate::tools::manager_order::ManagerOrderToolsImpl;

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
    CatalogToolsImpl::register(&mut registry);
    InventoryToolsImpl::register(&mut registry);
    ManagerOrderToolsImpl::register(&mut registry);
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
