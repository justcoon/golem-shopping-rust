use crate::ai::ToolSpec;
use crate::inventory::InventoryAgentClient;
use crate::order::{OrderAgentClient, OrderStatus};
use crate::order_search::OrderSearchAgentClient;
use crate::product::ProductAgentClient;
use crate::product_search::ProductSearchAgentClient;
use serde_json::json;

pub const MANAGER_SYSTEM_PROMPT: &str = r#"You are an expert, courteous, and efficient Store Management Assistant for our e-commerce store operations.
You assist store managers and administrators in managing orders, customers, inventory, and fulfillment.

You have access to tools to help:
- Search and list orders: search_orders, list_recent_orders, get_order_details
- Ship orders (fulfilling inventory): ship_order
- Cancel orders (releasing reserved inventory): cancel_order
- Modify orders (for orders with status 'new'):
  * add_order_item: add products to an order (reserves inventory)
  * remove_order_item: remove products from an order (releases inventory)
  * update_order_item_quantity: change item quantity (adjusts inventory)
  * update_order_shipping_address: change delivery destination
  * update_order_billing_address: change billing address
  * update_order_email: update customer contact email
- Check products and inventory:
  * check_product_details: inspect catalog product information
  * check_inventory: inspect available and total regional stock
  * search_products: search catalog

Guidelines:
1. Always confirm critical order actions like cancellation or shipment with the manager if not explicitly commanded.
2. When displaying orders, provide clear, concise summaries including Order ID, Customer ID, Status, Items, and Total.
3. If an order operation fails (for instance, trying to modify an order that is already shipped or cancelled), explain clearly why the action was rejected.
4. Output plain ASCII punctuation: use standard hyphens ('-') and standard quotes."#;

pub fn get_manager_tools() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "search_orders".to_string(),
            description: "Search orders across system by query text, status (new, shipped, cancelled), or customer userId".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Free text query matching products, customer, or address" },
                    "status": { "type": "string", "description": "Order status filter: 'new', 'shipped', or 'cancelled'" },
                    "userId": { "type": "string", "description": "Customer user ID to filter by" },
                    "limit": { "type": "integer", "description": "Max number of orders to return" }
                }
            }),
        },
        ToolSpec {
            name: "list_recent_orders".to_string(),
            description: "List the most recent orders across the store".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max number of recent orders (default 10)" }
                }
            }),
        },
        ToolSpec {
            name: "get_order_details".to_string(),
            description: "Get full details of a specific order including items, total, status, and shipping address".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to look up" }
                },
                "required": ["orderId"]
            }),
        },
        ToolSpec {
            name: "ship_order".to_string(),
            description: "Ship an existing new order. Fulfills inventory allocations and marks order status as 'shipped'.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to ship" }
                },
                "required": ["orderId"]
            }),
        },
        ToolSpec {
            name: "cancel_order".to_string(),
            description: "Cancel an existing new order. Releases reserved inventory allocations and marks order status as 'cancelled'.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to cancel" }
                },
                "required": ["orderId"]
            }),
        },
        ToolSpec {
            name: "add_order_item".to_string(),
            description: "Add a product item to an existing new order. Reserves required inventory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to modify" },
                    "productId": { "type": "string", "description": "Product ID to add" },
                    "quantity": { "type": "integer", "description": "Quantity to add" }
                },
                "required": ["orderId", "productId", "quantity"]
            }),
        },
        ToolSpec {
            name: "remove_order_item".to_string(),
            description: "Remove an item from an existing new order. Releases reserved inventory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to modify" },
                    "productId": { "type": "string", "description": "Product ID to remove" }
                },
                "required": ["orderId", "productId"]
            }),
        },
        ToolSpec {
            name: "update_order_item_quantity".to_string(),
            description: "Update the quantity of an item in an existing new order. Adjusts inventory reservations.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to modify" },
                    "productId": { "type": "string", "description": "Product ID in order" },
                    "quantity": { "type": "integer", "description": "New target quantity" }
                },
                "required": ["orderId", "productId", "quantity"]
            }),
        },
        ToolSpec {
            name: "update_order_shipping_address".to_string(),
            description: "Update recipient shipping address on an existing new order".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to modify" },
                    "street": { "type": "string" },
                    "city": { "type": "string" },
                    "stateOrRegion": { "type": "string" },
                    "country": { "type": "string" },
                    "postalCode": { "type": "string" },
                    "name": { "type": "string" },
                    "phoneNumber": { "type": "string" }
                },
                "required": ["orderId", "street", "city", "stateOrRegion", "country", "postalCode"]
            }),
        },
        ToolSpec {
            name: "update_order_billing_address".to_string(),
            description: "Update billing address on an existing new order".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to modify" },
                    "street": { "type": "string" },
                    "city": { "type": "string" },
                    "stateOrRegion": { "type": "string" },
                    "country": { "type": "string" },
                    "postalCode": { "type": "string" },
                    "name": { "type": "string" },
                    "phoneNumber": { "type": "string" }
                },
                "required": ["orderId", "street", "city", "stateOrRegion", "country", "postalCode"]
            }),
        },
        ToolSpec {
            name: "update_order_email".to_string(),
            description: "Update customer contact email on an existing new order".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to modify" },
                    "email": { "type": "string", "description": "Updated email address" }
                },
                "required": ["orderId", "email"]
            }),
        },
        ToolSpec {
            name: "check_product_details".to_string(),
            description: "Get product details by productId (name, brand, description, tags)".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "productId": { "type": "string", "description": "Product ID to inspect" }
                },
                "required": ["productId"]
            }),
        },
        ToolSpec {
            name: "check_inventory".to_string(),
            description: "Check available and total stock quantity for a product in a region".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "productId": { "type": "string", "description": "Product ID to check" },
                    "region": { "type": "string", "description": "Region name (defaults to 'global')" }
                },
                "required": ["productId"]
            }),
        },
        ToolSpec {
            name: "search_products".to_string(),
            description: "Search product catalog for items by name, brand, or query keywords".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Keyword to search" },
                    "currency": { "type": "string", "description": "Currency code" }
                },
                "required": ["query"]
            }),
        },
    ]
}

pub async fn execute_manager_tool(
    _manager_id: &str,
    tool_name: &str,
    args_json: &str,
) -> (String, String) {
    let args: serde_json::Value = serde_json::from_str(args_json).unwrap_or_else(|_| json!({}));

    let get_str = |field1: &str, field2: &str| -> Option<String> {
        args.get(field1)
            .or_else(|| args.get(field2))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    let get_u32 = |field1: &str, field2: &str| -> Option<u32> {
        args.get(field1)
            .or_else(|| args.get(field2))
            .and_then(|v| v.as_u64())
            .map(|n| n as u32)
    };

    match tool_name {
        "search_orders" => {
            let query = get_str("query", "query");
            let status_str = get_str("status", "status");
            let status = status_str.and_then(|s| match s.to_lowercase().as_str() {
                "new" => Some(OrderStatus::New),
                "shipped" => Some(OrderStatus::Shipped),
                "cancelled" | "canceled" => Some(OrderStatus::Cancelled),
                _ => None,
            });
            let user_id = get_str("userId", "user_id");
            let limit = get_u32("limit", "limit");

            let client = OrderSearchAgentClient::new_phantom();
            match client
                .search_orders(query, status, user_id, limit)
                .await
                .value
            {
                Ok(orders) => {
                    let summary = format!("Found {} matching order(s)", orders.len());
                    let res_str = serde_json::to_string(&orders).unwrap_or_default();
                    (res_str, summary)
                }
                Err(e) => (
                    format!("Failed to search orders: {e}"),
                    format!("Failed to search orders: {e}"),
                ),
            }
        }
        "list_recent_orders" => {
            let limit = get_u32("limit", "limit");
            let client = OrderSearchAgentClient::new_phantom();
            match client.get_recent_orders(limit).await.value {
                Ok(orders) => {
                    let summary = format!("Retrieved {} recent order(s)", orders.len());
                    let res_str = serde_json::to_string(&orders).unwrap_or_default();
                    (res_str, summary)
                }
                Err(e) => (
                    format!("Failed to retrieve recent orders: {e}"),
                    format!("Failed to retrieve recent orders: {e}"),
                ),
            }
        }
        "get_order_details" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let client = OrderAgentClient::get(order_id.clone());
            let order = client.get_order().await;
            match order {
                Some(o) => {
                    let summary = format!(
                        "Retrieved order {} (Status: {:?}, Total: ${:.2}, Items: {})",
                        o.order_id,
                        o.order_status,
                        o.total,
                        o.items.len()
                    );
                    let res_str = serde_json::to_string(&o).unwrap_or_default();
                    (res_str, summary)
                }
                None => (
                    format!("Order {order_id} not found"),
                    format!("Order {order_id} not found"),
                ),
            }
        }
        "ship_order" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let mut client = OrderAgentClient::get(order_id.clone());
            match client.ship_order().await {
                Ok(_) => {
                    let summary = format!(
                        "Order {order_id} has been successfully shipped and inventory fulfilled"
                    );
                    (
                        json!({ "success": true, "orderId": order_id, "status": "shipped" })
                            .to_string(),
                        summary,
                    )
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::order::ShipOrderError::EmptyItems(err) => err.message,
                        crate::order::ShipOrderError::EmptyEmail(err) => err.message,
                        crate::order::ShipOrderError::BillingAddressNotSet(err) => err.message,
                        crate::order::ShipOrderError::ActionNotAllowed(err) => err.message,
                    };
                    (
                        format!("Failed to ship order {order_id}: {err_msg}"),
                        format!("Failed to ship order: {err_msg}"),
                    )
                }
            }
        }
        "cancel_order" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let mut client = OrderAgentClient::get(order_id.clone());
            match client.cancel_order().await {
                Ok(_) => {
                    let summary = format!(
                        "Order {order_id} has been successfully cancelled and inventory released"
                    );
                    (
                        json!({ "success": true, "orderId": order_id, "status": "cancelled" })
                            .to_string(),
                        summary,
                    )
                }
                Err(crate::order::CancelOrderError::ActionNotAllowed(err)) => (
                    format!("Failed to cancel order {order_id}: {}", err.message),
                    format!("Failed to cancel order: {}", err.message),
                ),
            }
        }
        "add_order_item" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let qty = get_u32("quantity", "quantity").unwrap_or(1);
            let mut client = OrderAgentClient::get(order_id.clone());
            match client.add_item(pid.clone(), qty).await {
                Ok(_) => {
                    let summary = format!("Added item {pid} (qty: {qty}) to order {order_id}");
                    (json!({ "success": true, "orderId": order_id, "productId": pid, "quantity": qty }).to_string(), summary)
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::order::AddItemError::ProductNotFound(err) => err.message,
                        crate::order::AddItemError::PricingNotFound(err) => err.message,
                        crate::order::AddItemError::ActionNotAllowed(err) => err.message,
                        crate::order::AddItemError::InsufficientInventory(err) => err.message,
                    };
                    (
                        format!("Failed to add item to order {order_id}: {err_msg}"),
                        format!("Failed: {err_msg}"),
                    )
                }
            }
        }
        "remove_order_item" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let mut client = OrderAgentClient::get(order_id.clone());
            match client.remove_item(pid.clone()).await {
                Ok(_) => {
                    let summary = format!("Removed item {pid} from order {order_id}");
                    (
                        json!({ "success": true, "orderId": order_id, "productId": pid })
                            .to_string(),
                        summary,
                    )
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::order::RemoveItemError::ActionNotAllowed(err) => err.message,
                        crate::order::RemoveItemError::ItemNotFound(err) => err.message,
                    };
                    (
                        format!("Failed to remove item from order {order_id}: {err_msg}"),
                        format!("Failed: {err_msg}"),
                    )
                }
            }
        }
        "update_order_item_quantity" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let qty = get_u32("quantity", "quantity").unwrap_or(1);
            let mut client = OrderAgentClient::get(order_id.clone());
            match client.update_item_quantity(pid.clone(), qty).await {
                Ok(_) => {
                    let summary =
                        format!("Updated quantity for item {pid} in order {order_id} to {qty}");
                    (json!({ "success": true, "orderId": order_id, "productId": pid, "quantity": qty }).to_string(), summary)
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::order::UpdateItemQuantityError::ActionNotAllowed(err) => err.message,
                        crate::order::UpdateItemQuantityError::ItemNotFound(err) => err.message,
                        crate::order::UpdateItemQuantityError::InsufficientInventory(err) => {
                            err.message
                        }
                    };
                    (
                        format!("Failed to update quantity in order {order_id}: {err_msg}"),
                        format!("Failed: {err_msg}"),
                    )
                }
            }
        }
        "update_order_shipping_address" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let street = get_str("street", "street").unwrap_or_default();
            let city = get_str("city", "city").unwrap_or_default();
            let state = get_str("stateOrRegion", "state_or_region").unwrap_or_default();
            let country = get_str("country", "country").unwrap_or_default();
            let postal = get_str("postalCode", "postal_code").unwrap_or_default();
            let name = get_str("name", "name");
            let phone = get_str("phoneNumber", "phone_number");

            let mut client = OrderAgentClient::get(order_id.clone());
            match client
                .update_shipping_address(street, city, state, country, postal, name, phone)
                .await
            {
                Ok(_) => {
                    let summary = format!("Updated shipping address on order {order_id}");
                    (
                        json!({ "success": true, "orderId": order_id }).to_string(),
                        summary,
                    )
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::order::UpdateAddressError::ActionNotAllowed(err) => err.message,
                        crate::order::UpdateAddressError::AddressNotValid(err) => err.message,
                    };
                    (
                        format!("Failed to update shipping address on order {order_id}: {err_msg}"),
                        format!("Failed: {err_msg}"),
                    )
                }
            }
        }
        "update_order_billing_address" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let street = get_str("street", "street").unwrap_or_default();
            let city = get_str("city", "city").unwrap_or_default();
            let state = get_str("stateOrRegion", "state_or_region").unwrap_or_default();
            let country = get_str("country", "country").unwrap_or_default();
            let postal = get_str("postalCode", "postal_code").unwrap_or_default();
            let name = get_str("name", "name");
            let phone = get_str("phoneNumber", "phone_number");

            let mut client = OrderAgentClient::get(order_id.clone());
            match client
                .update_billing_address(street, city, state, country, postal, name, phone)
                .await
            {
                Ok(_) => {
                    let summary = format!("Updated billing address on order {order_id}");
                    (
                        json!({ "success": true, "orderId": order_id }).to_string(),
                        summary,
                    )
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::order::UpdateAddressError::ActionNotAllowed(err) => err.message,
                        crate::order::UpdateAddressError::AddressNotValid(err) => err.message,
                    };
                    (
                        format!("Failed to update billing address on order {order_id}: {err_msg}"),
                        format!("Failed: {err_msg}"),
                    )
                }
            }
        }
        "update_order_email" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let email = get_str("email", "email").unwrap_or_default();

            let mut client = OrderAgentClient::get(order_id.clone());
            match client.update_email(email.clone()).await {
                Ok(_) => {
                    let summary = format!("Updated customer email on order {order_id} to {email}");
                    (
                        json!({ "success": true, "orderId": order_id, "email": email }).to_string(),
                        summary,
                    )
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::order::UpdateEmailError::ActionNotAllowed(err) => err.message,
                        crate::order::UpdateEmailError::EmailNotValid(err) => err.message,
                    };
                    (
                        format!("Failed to update email on order {order_id}: {err_msg}"),
                        format!("Failed: {err_msg}"),
                    )
                }
            }
        }
        "check_product_details" => {
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let client = ProductAgentClient::get(pid.clone());
            let product = client.get_product().await;
            match product {
                Some(p) => {
                    let summary = format!("Product {}: {} ({})", p.product_id, p.name, p.brand);
                    let res_str = serde_json::to_string(&p).unwrap_or_default();
                    (res_str, summary)
                }
                None => (
                    format!("Product {pid} not found"),
                    format!("Product {pid} not found"),
                ),
            }
        }
        "check_inventory" => {
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let region = get_str("region", "region").unwrap_or_else(|| "global".to_string());
            let client = InventoryAgentClient::get(pid.clone());
            let avail = client.get_available_quantity(region.clone()).await;
            let summary = format!(
                "Product {pid} has {} available in region {region} (total: {}, allocated: {})",
                avail.available_quantity, avail.total_quantity, avail.allocated_quantity
            );
            let res_str = serde_json::to_string(&avail).unwrap_or_default();
            (res_str, summary)
        }
        "search_products" => {
            let query = get_str("query", "query").unwrap_or_default();
            let currency = get_str("currency", "currency");
            let client = ProductSearchAgentClient::new_phantom();
            match client
                .search_with_prices(query.clone(), currency)
                .await
                .value
            {
                Ok(products) => {
                    let summary = format!("Found {} matching product(s)", products.len());
                    let res_str = serde_json::to_string(&products).unwrap_or_default();
                    (res_str, summary)
                }
                Err(e) => (
                    format!("Failed to search products: {e}"),
                    format!("Failed to search products: {e}"),
                ),
            }
        }
        unknown => {
            let err_msg = format!("Unknown tool: {unknown}");
            (err_msg.clone(), err_msg)
        }
    }
}
