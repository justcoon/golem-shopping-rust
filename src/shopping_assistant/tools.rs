use crate::ai::agent_loop::{ToolDispatcher, ToolSpec};
use crate::ai::tools::ToolRegistry;
use crate::register_tools;
use crate::tools::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SHOPPING_ASSISTANT_SYSTEM_PROMPT: &str = r#"You are the Shopping Assistant for our e-commerce store. Help users find products, manage their cart, place orders, and track order history. Politely decline requests unrelated to shopping at our store.

### Core Rules
1. **Style & Formatting:** Be concise and direct. Use plain ASCII punctuation only (standard hyphens '-' and straight quotes '"').
2. **Product Mentions:** Always include the product's **name**, **brand**, and **price**.
3. **Strict IDs:** Never guess or invent a `productId` or `orderId`. Always retrieve them from tool outputs (`search_products`, `get_product_details`, `prepare_checkout`, etc.) first.
4. **Locked Carts:** If an active checkout reservation exists (`prepare_checkout` was called) and the user wants to change items, quantities, or addresses, you MUST call `cancel_checkout` before modifying the cart.

### Checkout Workflow
**Phase 1 - Prepare & Review:**
- Call `get_cart`. If empty, tell the user and stop.
- Call `get_saved_contact_info`.
  - If saved details exist: ask to use them (or call `apply_saved_contact_info` if already approved).
  - If none exist: ask for shipping address, billing address, and email, then save them with `set_shipping_address`, `set_billing_address`, and `set_email`.
- Once cart and contact info are set, call `prepare_checkout` to reserve inventory for 10 minutes.
- Show the user the **Order Summary** containing:
  - Order ID
  - Itemized list (product name, quantity, unit price)
  - Order total
  - Recipient name, shipping address, and email
- Inform the user that items are reserved for 10 minutes and explicitly ask for confirmation to place the order.

**Phase 2 - Confirm:**
- NEVER call `confirm_checkout` in the same turn you present the Order Summary.
- Only call `confirm_checkout` after the user explicitly replies to confirm (e.g., "yes", "place order", "confirm").
- Once confirmed, share the final Order ID."#;

/// Empty input parameters for parameter-free tool operations
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug, Default)]
pub struct EmptyInput {}

/// Add a product item to the user's shopping cart
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct AddToCartArgs {
    /// Product ID to add from product search
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// Quantity to add (default 1)
    pub quantity: Option<u32>,
}

/// Update the absolute quantity of an item in the shopping cart
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateCartQuantityArgs {
    /// Product ID in cart
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// New target quantity
    pub quantity: u32,
}

/// Remove a product completely from the shopping cart
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct RemoveFromCartArgs {
    /// Product ID to remove
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
}

/// Address details for shipping or billing
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct AddressToolInput {
    pub street: String,
    pub city: String,
    #[serde(rename = "stateOrRegion", alias = "state_or_region")]
    pub state_or_region: String,
    pub country: String,
    #[serde(rename = "postalCode", alias = "postal_code")]
    pub postal_code: String,
    pub name: Option<String>,
    #[serde(rename = "phoneNumber", alias = "phone_number")]
    pub phone_number: Option<String>,
}

/// Set customer contact email on the shopping cart
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct SetEmailArgs {
    /// Customer email address
    pub email: String,
}

/// Order ID parameter for checkout confirmation or cancellation
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct CheckoutOrderArgs {
    /// Order ID associated with the checkout draft
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
}

/// Retrieve past orders placed by this customer
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug, Default)]
pub struct GetCustomerOrderHistoryArgs {
    /// Maximum number of past orders to return
    pub limit: Option<u32>,
}

/// Builds the shopping assistant tool registry by mounting the Golem tool implementations.
pub fn build_shopping_assistant_tools(user_id: String) -> ToolRegistry {
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

    register_tools!(registry, CustomerOrderTools, context: user_id, use_impl: true, [
        get_order_details => |o| match o {
            Some(o) => format!(
                "Order {} status: {:?} (total: ${:.2})",
                o.order_id, o.order_status, o.total
            ),
            None => "Order not found".to_string(),
        },
        get_order_history(args: GetCustomerOrderHistoryArgs, uid) => GetCustomerOrderHistoryInput {
            user_id: uid.clone(),
            limit: args.limit,
        } => |orders| format!("Retrieved {} past order(s)", orders.len()),
    ]);

    register_tools!(registry, CartTools, context: user_id, use_impl: true, [
        get_cart(_: EmptyInput, uid) => UserCartInput {
            user_id: uid.clone(),
        } => |cart| match cart {
            Some(c) if c.items.is_empty() => "Cart is currently empty.".to_string(),
            Some(c) => format!(
                "Cart has {} item(s) (total: ${:.2})",
                c.items.len(),
                c.total
            ),
            None => "Cart is currently empty.".to_string(),
        },
        add_to_cart(args: AddToCartArgs, uid) => AddToCartInput {
            user_id: uid.clone(),
            product_id: args.product_id,
            quantity: args.quantity,
        } => |_| "Cart updated successfully".to_string(),
        update_cart_quantity(args: UpdateCartQuantityArgs, uid) => UpdateCartQuantityInput {
            user_id: uid.clone(),
            product_id: args.product_id,
            quantity: args.quantity,
        } => |_| "Cart quantity updated successfully".to_string(),
        remove_from_cart(args: RemoveFromCartArgs, uid) => RemoveFromCartInput {
            user_id: uid.clone(),
            product_id: args.product_id,
        } => |_| "Item removed from cart".to_string(),
        clear_cart(_: EmptyInput, uid) => UserCartInput {
            user_id: uid.clone(),
        } => |_| "Cleared all items from cart".to_string(),
        get_saved_contact_info(_: EmptyInput, uid) => UserCartInput {
            user_id: uid.clone(),
        } => |info| match info {
            Some(_) => "Retrieved saved contact info".to_string(),
            None => "No saved contact info found".to_string(),
        },
        apply_saved_contact_info(_: EmptyInput, uid) => UserCartInput {
            user_id: uid.clone(),
        } => |_| "Applied saved contact info to cart".to_string(),
        set_shipping_address(args: AddressToolInput, uid) => SetAddressInput {
            user_id: uid.clone(),
            street: args.street,
            city: args.city,
            state_or_region: args.state_or_region,
            country: args.country,
            postal_code: args.postal_code,
            name: args.name,
            phone_number: args.phone_number,
        } => |_| "Updated shipping address".to_string(),
        set_billing_address(args: AddressToolInput, uid) => SetAddressInput {
            user_id: uid.clone(),
            street: args.street,
            city: args.city,
            state_or_region: args.state_or_region,
            country: args.country,
            postal_code: args.postal_code,
            name: args.name,
            phone_number: args.phone_number,
        } => |_| "Updated billing address".to_string(),
        set_email(args: SetEmailArgs, uid) => SetEmailInput {
            user_id: uid.clone(),
            email: args.email,
        } => |_| "Updated contact email on cart".to_string(),
        prepare_checkout(_: EmptyInput, uid) => UserCartInput {
            user_id: uid.clone(),
        } => |cart| {
            let draft = cart.draft.as_ref();
            let order_id = draft.map(|d| d.order_id.as_str()).unwrap_or("");
            let expires_at = draft.map(|d| d.expires_at.to_rfc3339()).unwrap_or_default();
            format!(
                "Prepared checkout for order #{order_id}! Total: {}{:.2}, items reserved until {expires_at}. Please ask the user to confirm.",
                cart.currency, cart.total
            )
        },
        confirm_checkout(args: CheckoutOrderArgs, uid) => CheckoutOrderInput {
            user_id: uid.clone(),
            order_id: args.order_id,
        } => |conf| format!("Order confirmed! Order ID: {}", conf.order_id),
        cancel_checkout(args: CheckoutOrderArgs, uid) => CheckoutOrderInput {
            user_id: uid.clone(),
            order_id: args.order_id,
        } => |_| "Cancelled checkout reservation".to_string(),
    ]);

    registry
}

/// Returns the tool specifications for the shopping assistant.
pub fn get_shopping_assistant_tools() -> Vec<ToolSpec> {
    build_shopping_assistant_tools("dummy".to_string()).specs()
}

/// Executes a tool call by delegating to the generic `ToolRegistry` dispatcher.
pub async fn execute_shopping_assistant_tool(
    user_id: &str,
    tool_name: &str,
    args_json: &str,
) -> (String, String) {
    let registry = build_shopping_assistant_tools(user_id.to_string());
    registry.execute(tool_name, args_json).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shopping_assistant_tools_spec_generation() {
        let specs = get_shopping_assistant_tools();
        assert!(!specs.is_empty());

        let names: Vec<&str> = specs.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"search_products"));
        assert!(names.contains(&"get_product_details"));
        assert!(names.contains(&"check_inventory"));
        assert!(names.contains(&"get_order_details"));
        assert!(names.contains(&"get_order_history"));
        assert!(names.contains(&"get_cart"));
        assert!(names.contains(&"add_to_cart"));
        assert!(names.contains(&"update_cart_quantity"));
        assert!(names.contains(&"remove_from_cart"));
        assert!(names.contains(&"prepare_checkout"));
        assert!(names.contains(&"confirm_checkout"));
        assert!(names.contains(&"cancel_checkout"));

        // Verify descriptions were extracted from Golem tool definitions
        let search_spec = specs.iter().find(|s| s.name == "search_products").unwrap();
        assert_eq!(
            search_spec.description,
            "Search product catalog for items by name, brand, or query keywords with active prices"
        );

        let update_cart_spec = specs
            .iter()
            .find(|s| s.name == "update_cart_quantity")
            .unwrap();
        assert_eq!(
            update_cart_spec.description,
            "Update the absolute quantity of an item in the shopping cart"
        );
    }
}
