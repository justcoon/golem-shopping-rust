#![allow(clippy::too_many_arguments)]

use crate::ai::tools::ToolRegistry;
use crate::cart::{Cart, CartAgentClient, CartUpdated, OrderConfirmation, SavedContactInfo};
use crate::common::Address;
use crate::order::Order;
use golem_rust::{Schema, ToolError, tool_definition, tool_implementation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Empty input parameters for cart operations
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug, Default)]
pub struct EmptyInput {}

/// Add a product item to the user's shopping cart
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct AddToCartInput {
    /// Product ID to add
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// Quantity to add (default 1)
    pub quantity: Option<u32>,
}

/// Update the absolute quantity of an item in the shopping cart
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateCartQuantityInput {
    /// Product ID in cart
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// New target quantity
    pub quantity: u32,
}

/// Remove a product completely from the shopping cart
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct RemoveFromCartInput {
    /// Product ID to remove
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
}

/// Address details for shipping or billing
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
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

impl From<AddressToolInput> for Address {
    fn from(input: AddressToolInput) -> Self {
        Address {
            street: input.street,
            city: input.city,
            state_or_region: input.state_or_region,
            country: input.country,
            postal_code: input.postal_code,
            name: input.name,
            phone_number: input.phone_number,
        }
    }
}

/// Set customer contact email on the shopping cart
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct SetEmailInput {
    /// Customer email address
    pub email: String,
}

/// Retrieve past orders placed by this customer
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct GetOrderHistoryInput {
    /// Maximum number of past orders to return
    pub limit: Option<u32>,
}

/// Order ID parameter for checkout confirmation or cancellation
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct CheckoutOrderInput {
    /// Order ID associated with the checkout draft
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToolError)]
pub enum CartToolError {
    #[tool_error(kind = "runtime-error", exit_code = 1)]
    OperationFailed { message: String },
}

impl std::fmt::Display for CartToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CartToolError::OperationFailed { message } => write!(f, "{message}"),
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[tool_definition(version = "1.0.0")]
pub trait CartTool {
    /// View current shopping cart items, quantities, totals, and addresses
    async fn get_cart(&self, user_id: String) -> Option<Cart>;

    /// Add a product item to the user's shopping cart
    async fn add_to_cart(
        &self,
        user_id: String,
        input: AddToCartInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Update the absolute quantity of an item in the shopping cart
    async fn update_cart_quantity(
        &self,
        user_id: String,
        input: UpdateCartQuantityInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Remove a product completely from the shopping cart
    async fn remove_from_cart(
        &self,
        user_id: String,
        input: RemoveFromCartInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Clear all items, addresses, and state from the shopping cart
    async fn clear_cart(&self, user_id: String);

    /// Check saved shipping/billing addresses and email from previous orders
    async fn get_saved_contact_info(&self, user_id: String) -> Option<SavedContactInfo>;

    /// Pre-fill the cart with saved contact details for 1-click checkout
    async fn apply_saved_contact_info(&self, user_id: String)
    -> Result<CartUpdated, CartToolError>;

    /// Set recipient shipping destination address on the shopping cart
    async fn set_shipping_address(
        &self,
        user_id: String,
        input: AddressToolInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Set billing address on the shopping cart
    async fn set_billing_address(
        &self,
        user_id: String,
        input: AddressToolInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Set customer contact email on the shopping cart
    async fn set_email(
        &self,
        user_id: String,
        input: SetEmailInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Prepare checkout by reserving inventory for 10 minutes and generating an order draft
    async fn prepare_checkout(&self, user_id: String) -> Result<Cart, CartToolError>;

    /// Confirm and place the order after explicit user confirmation
    async fn confirm_checkout(
        &self,
        user_id: String,
        input: CheckoutOrderInput,
    ) -> Result<OrderConfirmation, CartToolError>;

    /// Cancel an active checkout reservation to make changes to the cart
    async fn cancel_checkout(
        &self,
        user_id: String,
        input: CheckoutOrderInput,
    ) -> Result<(), CartToolError>;

    /// Retrieve past orders placed by this customer
    async fn get_order_history(&self, user_id: String, input: GetOrderHistoryInput) -> Vec<Order>;
}

pub struct CartToolImpl;

#[tool_implementation]
impl CartTool for CartToolImpl {
    async fn get_cart(&self, user_id: String) -> Option<Cart> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client.get_cart().await
    }

    async fn add_to_cart(
        &self,
        user_id: String,
        input: AddToCartInput,
    ) -> Result<CartUpdated, CartToolError> {
        let quantity = input.quantity.unwrap_or(1);
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client
            .add_item(input.product_id.clone(), quantity)
            .await
            .map_err(|e| {
                let msg = match e {
                    crate::cart::AddItemError::ProductNotFound(err) => err.message,
                    crate::cart::AddItemError::PricingNotFound(err) => err.message,
                    crate::cart::AddItemError::InsufficientStock(err) => {
                        format!(
                            "Insufficient stock (requested {}, available {})",
                            err.requested, err.available
                        )
                    }
                    crate::cart::AddItemError::ActionNotAllowed(err) => err.message,
                };
                CartToolError::OperationFailed { message: msg }
            })
    }

    async fn update_cart_quantity(
        &self,
        user_id: String,
        input: UpdateCartQuantityInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client
            .update_item_quantity(input.product_id.clone(), input.quantity)
            .await
            .map_err(|e| {
                let msg = match e {
                    crate::cart::UpdateItemQuantityError::ItemNotFound(err) => err.message,
                    crate::cart::UpdateItemQuantityError::InsufficientStock(err) => {
                        format!(
                            "Insufficient stock (requested {}, available {})",
                            err.requested, err.available
                        )
                    }
                    crate::cart::UpdateItemQuantityError::ActionNotAllowed(err) => err.message,
                };
                CartToolError::OperationFailed { message: msg }
            })
    }

    async fn remove_from_cart(
        &self,
        user_id: String,
        input: RemoveFromCartInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client
            .remove_item(input.product_id)
            .await
            .map_err(|e| {
                let msg = match e {
                    crate::cart::RemoveItemError::ItemNotFound(err) => err.message,
                    crate::cart::RemoveItemError::ActionNotAllowed(err) => err.message,
                };
                CartToolError::OperationFailed { message: msg }
            })
    }

    async fn clear_cart(&self, user_id: String) {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client.clear().await;
    }

    async fn get_saved_contact_info(&self, user_id: String) -> Option<SavedContactInfo> {
        let cart_client = CartAgentClient::get(user_id);
        cart_client.get_saved_contact_info().await
    }

    async fn apply_saved_contact_info(
        &self,
        user_id: String,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client.apply_saved_contact_info().await.map_err(|e| {
            let msg = match e {
                crate::cart::UpdateAddressError::AddressNotValid(err) => err.message,
                crate::cart::UpdateAddressError::ActionNotAllowed(err) => err.message,
            };
            CartToolError::OperationFailed { message: msg }
        })
    }

    async fn set_shipping_address(
        &self,
        user_id: String,
        input: AddressToolInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client
            .update_shipping_address(
                input.street,
                input.city,
                input.state_or_region,
                input.country,
                input.postal_code,
                input.name,
                input.phone_number,
            )
            .await
            .map_err(|e| {
                let msg = match e {
                    crate::cart::UpdateAddressError::AddressNotValid(err) => err.message,
                    crate::cart::UpdateAddressError::ActionNotAllowed(err) => err.message,
                };
                CartToolError::OperationFailed { message: msg }
            })
    }

    async fn set_billing_address(
        &self,
        user_id: String,
        input: AddressToolInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client
            .update_billing_address(
                input.street,
                input.city,
                input.state_or_region,
                input.country,
                input.postal_code,
                input.name,
                input.phone_number,
            )
            .await
            .map_err(|e| {
                let msg = match e {
                    crate::cart::UpdateAddressError::AddressNotValid(err) => err.message,
                    crate::cart::UpdateAddressError::ActionNotAllowed(err) => err.message,
                };
                CartToolError::OperationFailed { message: msg }
            })
    }

    async fn set_email(
        &self,
        user_id: String,
        input: SetEmailInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client.update_email(input.email).await.map_err(|e| {
            let msg = match e {
                crate::cart::UpdateEmailError::EmailNotValid(err) => err.message,
                crate::cart::UpdateEmailError::ActionNotAllowed(err) => err.message,
            };
            CartToolError::OperationFailed { message: msg }
        })
    }

    async fn prepare_checkout(&self, user_id: String) -> Result<Cart, CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client.prepare_checkout().await.map_err(|e| {
            let msg = match e {
                crate::cart::PrepareCheckoutError::EmptyItems(err) => err.message,
                crate::cart::PrepareCheckoutError::EmptyEmail(err) => err.message,
                crate::cart::PrepareCheckoutError::BillingAddressNotSet(err) => err.message,
                crate::cart::PrepareCheckoutError::InsufficientStock(err) => err.message,
                crate::cart::PrepareCheckoutError::ActionNotAllowed(err) => err.message,
            };
            CartToolError::OperationFailed { message: msg }
        })
    }

    async fn confirm_checkout(
        &self,
        user_id: String,
        input: CheckoutOrderInput,
    ) -> Result<OrderConfirmation, CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client
            .confirm_checkout(input.order_id)
            .await
            .map_err(|e| {
                let msg = match e {
                    crate::cart::ConfirmCheckoutError::NoActiveDraft(msg) => msg,
                    crate::cart::ConfirmCheckoutError::OrderMismatch(msg) => msg,
                    crate::cart::ConfirmCheckoutError::ReservationExpired(msg) => msg,
                    crate::cart::ConfirmCheckoutError::OrderCreate(err) => err.message,
                };
                CartToolError::OperationFailed { message: msg }
            })
    }

    async fn cancel_checkout(
        &self,
        user_id: String,
        input: CheckoutOrderInput,
    ) -> Result<(), CartToolError> {
        let mut cart_client = CartAgentClient::get(user_id);
        cart_client
            .cancel_checkout(input.order_id)
            .await
            .map_err(|msg| CartToolError::OperationFailed { message: msg })
    }

    async fn get_order_history(&self, user_id: String, input: GetOrderHistoryInput) -> Vec<Order> {
        let cart_client = CartAgentClient::get(user_id);
        cart_client.get_order_history(input.limit).await
    }
}

impl CartToolImpl {
    /// Registers all cart tools for a specific user into a `ToolRegistry` for LLM consumption.
    pub fn register(user_id: String, registry: &mut ToolRegistry) {
        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "get_cart",
            "View current shopping cart items, quantities, totals, and addresses",
            move |_: EmptyInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().get_cart(uid).await }
            },
            |cart| match cart {
                Some(c) if c.items.is_empty() => "Cart is currently empty.".to_string(),
                Some(c) => format!(
                    "Cart has {} item(s) (total: ${:.2})",
                    c.items.len(),
                    c.total
                ),
                None => "Cart is currently empty.".to_string(),
            },
        );

        let uid = user_id.clone();
        registry.register_tool_call(
            "add_to_cart",
            move |args: AddToCartInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().add_to_cart(uid, args).await }
            },
            |_| "Cart updated successfully".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call(
            "update_cart_quantity",
            move |args: UpdateCartQuantityInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().update_cart_quantity(uid, args).await }
            },
            |_| "Cart quantity updated successfully".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call(
            "remove_from_cart",
            move |args: RemoveFromCartInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().remove_from_cart(uid, args).await }
            },
            |_| "Item removed from cart".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "clear_cart",
            "Clear all items, addresses, and state from the shopping cart",
            move |_: EmptyInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().clear_cart(uid).await }
            },
            |_| "Cleared all items from cart".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "get_saved_contact_info",
            "Check saved shipping/billing addresses and email from past orders for 1-click checkout",
            move |_: EmptyInput| {
                let uid = uid.clone();
                async move {
                    CartToolClient::new()
                        .get_saved_contact_info(uid)
                        .await
                }
            },
            |info| match info {
                Some(_) => "Retrieved saved contact info".to_string(),
                None => "No saved contact info found".to_string(),
            },
        );

        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "apply_saved_contact_info",
            "Pre-fill current cart with saved shipping/billing addresses and email from previous orders",
            move |_: EmptyInput| {
                let uid = uid.clone();
                async move {
                    CartToolClient::new()
                        .apply_saved_contact_info(uid)
                        .await
                }
            },
            |_| "Applied saved contact info to cart".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "set_shipping_address",
            "Set recipient shipping destination address on the shopping cart",
            move |args: AddressToolInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().set_shipping_address(uid, args).await }
            },
            |_| "Updated shipping address".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "set_billing_address",
            "Set billing address on the shopping cart",
            move |args: AddressToolInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().set_billing_address(uid, args).await }
            },
            |_| "Updated billing address".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call(
            "set_email",
            move |args: SetEmailInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().set_email(uid, args).await }
            },
            |_| "Updated contact email on cart".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "prepare_checkout",
            "Prepare checkout by reserving inventory for 10 minutes and generating an order draft",
            move |_: EmptyInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().prepare_checkout(uid).await }
            },
            |cart| {
                let draft = cart.draft.as_ref();
                let order_id = draft.map(|d| d.order_id.as_str()).unwrap_or("");
                let expires_at = draft.map(|d| d.expires_at.to_rfc3339()).unwrap_or_default();
                format!(
                    "Prepared checkout for order #{order_id}! Total: {}{:.2}, items reserved until {expires_at}. Please ask the user to confirm.",
                    cart.currency, cart.total
                )
            },
        );

        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "confirm_checkout",
            "Confirm and place the order after explicit user confirmation",
            move |args: CheckoutOrderInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().confirm_checkout(uid, args).await }
            },
            |conf| format!("Order confirmed! Order ID: {}", conf.order_id),
        );

        let uid = user_id.clone();
        registry.register_tool_call_with_desc(
            "cancel_checkout",
            "Cancel an active checkout reservation to make changes to the cart",
            move |args: CheckoutOrderInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().cancel_checkout(uid, args).await }
            },
            |_| "Cancelled checkout reservation".to_string(),
        );

        let uid = user_id.clone();
        registry.register_tool_call(
            "get_order_history",
            move |args: GetOrderHistoryInput| {
                let uid = uid.clone();
                async move { CartToolClient::new().get_order_history(uid, args).await }
            },
            |orders| format!("Retrieved {} past order(s)", orders.len()),
        );
    }
}
