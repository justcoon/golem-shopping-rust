use crate::ai::ToolSpec;
use crate::cart::CartAgentClient;
use crate::inventory::InventoryAgentClient;
use crate::order::OrderAgentClient;
use crate::product_search::ProductSearchAgentClient;
use serde_json::json;

pub const SHOPPING_ASSISTANT_SYSTEM_PROMPT: &str = r#"You are a helpful, courteous, and efficient personal Shopping Assistant for our e-commerce store.
You have access to tools to help the user:
- Search products and inspect live pricing (search_products)
- Get full details on a specific product (get_product_details)
- View current cart items, totals, and address (get_cart)
- Add items, update quantities, remove items, or clear cart (add_to_cart, update_cart_quantity, remove_from_cart, clear_cart)
- Check saved shipping/billing addresses and email from previous orders (get_saved_contact_info)
- Pre-fill the cart with saved contact details for 1-click checkout (apply_saved_contact_info)
- Manually set shipping address, billing address, or email on the cart (set_shipping_address, set_billing_address, set_email)
- Place the order (checkout_cart)
- Check order history or track a specific order (get_order_history, get_order_details)
- Check product inventory (check_inventory)

Guidelines:
1. Be concise, polite, and directly address the user's intent.
2. When mentioning products, include their name, brand, and price.
3. For checkout:
   - Step 1 (Cart & Contact Info Review):
     * Always inspect the current cart using get_cart. If the cart is empty, politely inform the user and do not proceed with checkout.
     * Check for saved contact details using get_saved_contact_info.
     * If saved contact info exists, propose using those details (or apply them via apply_saved_contact_info if the user already agreed).
     * If no saved contact info exists (first-time customer), ask for their shipping address, billing address, and email, then update the cart using set_shipping_address, set_billing_address, and set_email.
     * Always present an Order Summary for user review before placing the order:
       - Itemized list with product name, quantity, and unit price
       - Order total amount
       - Recipient name, shipping address, and email
     * Explicitly ask the user to confirm placing the order (e.g. "Please review your order summary above. Would you like me to place this order now?").
   - Step 2 (Order Placement):
     * ONLY call checkout_cart AFTER the user has seen the order review and explicitly confirmed (e.g. "yes", "confirm", "proceed", "place order").
     * Never call checkout_cart in the same turn where the order summary is shown for the first time.
     * After checkout succeeds, celebrate the order and provide the order ID.
4. When showing order history, summarize recent orders clearly (status, total, date).
5. Output plain ASCII punctuation: use standard hyphens ('-') instead of special en-dashes ('–') or em-dashes ('—'), and standard quotes."#;

pub fn get_shopping_assistant_tools() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "search_products".to_string(),
            description: "Search product catalog for items by name, brand, or query keywords with active prices".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keyword, brand, or category" },
                    "currency": { "type": "string", "description": "Currency code, e.g. 'USD'" }
                },
                "required": ["query"]
            }),
        },
        ToolSpec {
            name: "get_product_details".to_string(),
            description: "Get detailed product specifications, price, and inventory by product ID".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "productId": { "type": "string", "description": "Product ID, e.g. 'ap001'" },
                    "currency": { "type": "string", "description": "Currency code, e.g. 'USD'" }
                },
                "required": ["productId"]
            }),
        },
        ToolSpec {
            name: "check_inventory".to_string(),
            description: "Check available stock quantity for a product in a region".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "productId": { "type": "string", "description": "Product ID to check inventory for" },
                    "region": { "type": "string", "description": "Region name (defaults to 'global')" }
                },
                "required": ["productId"]
            }),
        },
        ToolSpec {
            name: "get_cart".to_string(),
            description: "View current shopping cart items, quantities, totals, and addresses".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolSpec {
            name: "add_to_cart".to_string(),
            description: "Add a product item to the user's shopping cart".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "productId": { "type": "string", "description": "Product ID to add" },
                    "quantity": { "type": "integer", "description": "Quantity to add (default 1)" }
                },
                "required": ["productId"]
            }),
        },
        ToolSpec {
            name: "update_cart_quantity".to_string(),
            description: "Update the absolute quantity of an item in the shopping cart".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "productId": { "type": "string", "description": "Product ID in cart" },
                    "quantity": { "type": "integer", "description": "New target quantity" }
                },
                "required": ["productId", "quantity"]
            }),
        },
        ToolSpec {
            name: "remove_from_cart".to_string(),
            description: "Remove a product completely from the shopping cart".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "productId": { "type": "string", "description": "Product ID to remove" }
                },
                "required": ["productId"]
            }),
        },
        ToolSpec {
            name: "clear_cart".to_string(),
            description: "Clear all items, addresses, and state from the shopping cart".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolSpec {
            name: "get_saved_contact_info".to_string(),
            description: "Check saved shipping/billing addresses and email from past orders for 1-click checkout".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolSpec {
            name: "apply_saved_contact_info".to_string(),
            description: "Pre-fill current cart with saved shipping/billing addresses and email from previous orders".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolSpec {
            name: "set_shipping_address".to_string(),
            description: "Set recipient shipping destination address on the shopping cart".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "street": { "type": "string" },
                    "city": { "type": "string" },
                    "stateOrRegion": { "type": "string" },
                    "country": { "type": "string" },
                    "postalCode": { "type": "string" },
                    "name": { "type": "string" },
                    "phoneNumber": { "type": "string" }
                },
                "required": ["street", "city", "stateOrRegion", "country", "postalCode"]
            }),
        },
        ToolSpec {
            name: "set_billing_address".to_string(),
            description: "Set billing address on the shopping cart".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "street": { "type": "string" },
                    "city": { "type": "string" },
                    "stateOrRegion": { "type": "string" },
                    "country": { "type": "string" },
                    "postalCode": { "type": "string" },
                    "name": { "type": "string" },
                    "phoneNumber": { "type": "string" }
                },
                "required": ["street", "city", "stateOrRegion", "country", "postalCode"]
            }),
        },
        ToolSpec {
            name: "set_email".to_string(),
            description: "Set customer contact email on the shopping cart".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "email": { "type": "string", "description": "Customer email address" }
                },
                "required": ["email"]
            }),
        },
        ToolSpec {
            name: "checkout_cart".to_string(),
            description: "Place the order from the current shopping cart after user explicit confirmation".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolSpec {
            name: "get_order_history".to_string(),
            description: "Retrieve past orders placed by this customer".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Maximum number of past orders to return" }
                }
            }),
        },
        ToolSpec {
            name: "get_order_details".to_string(),
            description: "Get full details of a specific order by order ID".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "orderId": { "type": "string", "description": "Order ID to look up" }
                },
                "required": ["orderId"]
            }),
        },
    ]
}

pub async fn execute_shopping_assistant_tool(
    user_id: &str,
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
                    let summary = format!("Found {} product(s) for '{query}'", products.len());
                    let res_str = serde_json::to_string(&products).unwrap_or_default();
                    (res_str, summary)
                }
                Err(e) => (
                    format!("Failed to search products: {e}"),
                    format!("Failed to search products: {e}"),
                ),
            }
        }
        "get_product_details" => {
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let currency = get_str("currency", "currency");
            let client = ProductSearchAgentClient::new_phantom();
            match client
                .get_by_ids_with_prices(pid.clone(), currency)
                .await
                .value
            {
                Ok(mut products) => {
                    if let Some(prod) = products.pop() {
                        let summary = format!("Details for {} ({})", prod.name, prod.product_id);
                        let res_str = serde_json::to_string(&prod).unwrap_or_default();
                        (res_str, summary)
                    } else {
                        (
                            format!("Product with ID '{pid}' not found."),
                            format!("Product '{pid}' not found"),
                        )
                    }
                }
                Err(e) => (
                    format!("Failed to get product details: {e}"),
                    format!("Failed to get product details: {e}"),
                ),
            }
        }
        "check_inventory" => {
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let region = get_str("region", "region").unwrap_or_else(|| "global".to_string());
            let client = InventoryAgentClient::get(pid.clone());
            let avail = client.get_available_quantity(region.clone()).await;
            let summary = format!(
                "Product {pid} has {} unit(s) available in region {region}",
                avail.available_quantity
            );
            let res_str = serde_json::to_string(&avail).unwrap_or_default();
            (res_str, summary)
        }
        "get_cart" => {
            let mut client = CartAgentClient::get(user_id.to_string());
            let cart = client.get_cart().await;
            match cart {
                Some(c) => {
                    let summary = if c.items.is_empty() {
                        "Cart is currently empty.".to_string()
                    } else {
                        format!(
                            "Cart has {} item(s) (total: ${:.2})",
                            c.items.len(),
                            c.total
                        )
                    };
                    let res_str = serde_json::to_string(&c).unwrap_or_default();
                    (res_str, summary)
                }
                None => (
                    "Cart is currently empty.".to_string(),
                    "Cart is currently empty.".to_string(),
                ),
            }
        }
        "add_to_cart" => {
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let qty = get_u32("quantity", "quantity").unwrap_or(1);
            let mut client = CartAgentClient::get(user_id.to_string());
            match client.add_item(pid.clone(), qty).await {
                Ok(_) => {
                    let summary = format!("Added item {pid} (qty: {qty}) to cart");
                    (
                        json!({ "success": true, "productId": pid, "quantity": qty }).to_string(),
                        summary,
                    )
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::cart::AddItemError::ProductNotFound(err) => err.message,
                        crate::cart::AddItemError::PricingNotFound(err) => err.message,
                        crate::cart::AddItemError::InsufficientStock(err) => err.message,
                    };
                    (
                        format!("Failed to add item to cart: {err_msg}"),
                        format!("Failed to add item: {err_msg}"),
                    )
                }
            }
        }
        "update_cart_quantity" => {
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let qty = get_u32("quantity", "quantity").unwrap_or(1);
            let mut client = CartAgentClient::get(user_id.to_string());
            match client.update_item_quantity(pid.clone(), qty).await {
                Ok(_) => {
                    let summary = format!("Updated quantity of {pid} to {qty}");
                    (
                        json!({ "success": true, "productId": pid, "quantity": qty }).to_string(),
                        summary,
                    )
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::cart::UpdateItemQuantityError::ItemNotFound(err) => err.message,
                        crate::cart::UpdateItemQuantityError::InsufficientStock(err) => err.message,
                    };
                    (
                        format!("Failed to update quantity: {err_msg}"),
                        format!("Failed to update quantity: {err_msg}"),
                    )
                }
            }
        }
        "remove_from_cart" => {
            let pid = get_str("productId", "product_id").unwrap_or_default();
            let mut client = CartAgentClient::get(user_id.to_string());
            match client.remove_item(pid.clone()).await {
                Ok(_) => {
                    let summary = format!("Removed item {pid} from cart");
                    (
                        json!({ "success": true, "productId": pid }).to_string(),
                        summary,
                    )
                }
                Err(crate::cart::RemoveItemError::ItemNotFound(err)) => (
                    format!("Failed to remove item: {}", err.message),
                    format!("Failed to remove item: {}", err.message),
                ),
            }
        }
        "clear_cart" => {
            let mut client = CartAgentClient::get(user_id.to_string());
            client.clear().await;
            (
                json!({ "success": true }).to_string(),
                "Cleared all items from cart".to_string(),
            )
        }
        "get_saved_contact_info" => {
            let client = CartAgentClient::get(user_id.to_string());
            let info = client.get_saved_contact_info().await;
            match info {
                Some(saved) => {
                    let res_str = serde_json::to_string(&saved).unwrap_or_default();
                    (res_str, "Retrieved saved contact info".to_string())
                }
                None => (
                    "No saved contact info found.".to_string(),
                    "No saved contact info found".to_string(),
                ),
            }
        }
        "apply_saved_contact_info" => {
            let mut client = CartAgentClient::get(user_id.to_string());
            match client.apply_saved_contact_info().await {
                Ok(_) => (
                    json!({ "success": true }).to_string(),
                    "Applied saved contact info to cart".to_string(),
                ),
                Err(crate::cart::UpdateAddressError::AddressNotValid(err)) => (
                    format!("Failed to apply saved contact info: {}", err.message),
                    format!("Failed: {}", err.message),
                ),
            }
        }
        "set_shipping_address" => {
            let street = get_str("street", "street").unwrap_or_default();
            let city = get_str("city", "city").unwrap_or_default();
            let state = get_str("stateOrRegion", "state_or_region").unwrap_or_default();
            let country = get_str("country", "country").unwrap_or_default();
            let postal = get_str("postalCode", "postal_code").unwrap_or_default();
            let name = get_str("name", "name");
            let phone = get_str("phoneNumber", "phone_number");

            let mut client = CartAgentClient::get(user_id.to_string());
            match client
                .update_shipping_address(street, city, state, country, postal, name, phone)
                .await
            {
                Ok(_) => (
                    json!({ "success": true }).to_string(),
                    "Updated shipping address".to_string(),
                ),
                Err(crate::cart::UpdateAddressError::AddressNotValid(err)) => (
                    format!("Failed to update shipping address: {}", err.message),
                    format!("Failed: {}", err.message),
                ),
            }
        }
        "set_billing_address" => {
            let street = get_str("street", "street").unwrap_or_default();
            let city = get_str("city", "city").unwrap_or_default();
            let state = get_str("stateOrRegion", "state_or_region").unwrap_or_default();
            let country = get_str("country", "country").unwrap_or_default();
            let postal = get_str("postalCode", "postal_code").unwrap_or_default();
            let name = get_str("name", "name");
            let phone = get_str("phoneNumber", "phone_number");

            let mut client = CartAgentClient::get(user_id.to_string());
            match client
                .update_billing_address(street, city, state, country, postal, name, phone)
                .await
            {
                Ok(_) => (
                    json!({ "success": true }).to_string(),
                    "Updated billing address".to_string(),
                ),
                Err(crate::cart::UpdateAddressError::AddressNotValid(err)) => (
                    format!("Failed to update billing address: {}", err.message),
                    format!("Failed: {}", err.message),
                ),
            }
        }
        "set_email" => {
            let email = get_str("email", "email").unwrap_or_default();
            let mut client = CartAgentClient::get(user_id.to_string());
            match client.update_email(email.clone()).await {
                Ok(_) => (
                    json!({ "success": true, "email": email }).to_string(),
                    format!("Updated email to {email}"),
                ),
                Err(crate::cart::UpdateEmailError::EmailNotValid(err)) => (
                    format!("Failed to update email: {}", err.message),
                    format!("Failed: {}", err.message),
                ),
            }
        }
        "checkout_cart" => {
            let mut client = CartAgentClient::get(user_id.to_string());
            match client.checkout().await {
                Ok(conf) => {
                    let summary = format!("Checkout complete! Order ID: {}", conf.order_id);
                    let res_str =
                        json!({ "orderId": conf.order_id, "userId": conf.user_id }).to_string();
                    (res_str, summary)
                }
                Err(e) => {
                    let err_msg = match e {
                        crate::cart::CheckoutError::ProductNotFound(err) => err.message,
                        crate::cart::CheckoutError::PricingNotFound(err) => err.message,
                        crate::cart::CheckoutError::EmptyItems(err) => err.message,
                        crate::cart::CheckoutError::EmptyEmail(err) => err.message,
                        crate::cart::CheckoutError::BillingAddressNotSet(err) => err.message,
                        crate::cart::CheckoutError::OrderCreate(err) => err.message,
                        crate::cart::CheckoutError::InsufficientStock(err) => err.message,
                    };
                    (
                        format!("Checkout failed: {err_msg}"),
                        format!("Checkout failed: {err_msg}"),
                    )
                }
            }
        }
        "get_order_history" => {
            let limit = get_u32("limit", "limit");
            let client = CartAgentClient::get(user_id.to_string());
            let orders = client.get_order_history(limit).await;
            let summary = format!("Retrieved {} past order(s)", orders.len());
            let res_str = serde_json::to_string(&orders).unwrap_or_default();
            (res_str, summary)
        }
        "get_order_details" => {
            let order_id = get_str("orderId", "order_id").unwrap_or_default();
            let client = OrderAgentClient::get(order_id.clone());
            let order = client.get_order().await;
            match order {
                Some(o) => {
                    let summary = format!(
                        "Order {} status: {:?} (total: ${:.2})",
                        o.order_id, o.order_status, o.total
                    );
                    let res_str = serde_json::to_string(&o).unwrap_or_default();
                    (res_str, summary)
                }
                None => (
                    format!("Order with ID '{order_id}' not found."),
                    format!("Order '{order_id}' not found"),
                ),
            }
        }
        unknown => {
            let err_msg = format!("Unknown tool: {unknown}");
            (err_msg.clone(), err_msg)
        }
    }
}
