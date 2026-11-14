use crate::cart::{Cart, CartAgentClient, CartUpdated, OrderConfirmation, SavedContactInfo};
use crate::common::Address;
use golem_rust::{Schema, ToolError, tool_definition, tool_implementation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// User cart identifier input
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug, Default)]
pub struct UserCartInput {
    /// Customer user ID
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
}

/// Add a product item to the user's shopping cart
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct AddToCartInput {
    /// Customer user ID
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
    /// Product ID to add
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// Quantity to add (default 1)
    pub quantity: Option<u32>,
}

/// Update the absolute quantity of an item in the shopping cart
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateCartQuantityInput {
    /// Customer user ID
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
    /// Product ID in cart
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// New target quantity
    pub quantity: u32,
}

/// Remove a product completely from the shopping cart
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct RemoveFromCartInput {
    /// Customer user ID
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
    /// Product ID to remove
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
}

/// Set address details on the shopping cart
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct SetAddressInput {
    /// Customer user ID
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
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

impl From<SetAddressInput> for Address {
    fn from(input: SetAddressInput) -> Self {
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
    /// Customer user ID
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
    /// Customer email address
    pub email: String,
}

/// Order ID parameter for checkout confirmation or cancellation
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct CheckoutOrderInput {
    /// Customer user ID
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
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

/// Customer shopping cart management tools for viewing and modifying items
#[allow(clippy::too_many_arguments)]
#[tool_definition(version = "1.0.0")]
pub trait CartTools {
    /// View current shopping cart items, quantities, totals, and addresses
    async fn get_cart(&self, input: UserCartInput) -> Option<Cart>;

    /// Add a product item to the user's shopping cart
    async fn add_to_cart(&self, input: AddToCartInput) -> Result<CartUpdated, CartToolError>;

    /// Update the absolute quantity of an item in the shopping cart
    async fn update_cart_quantity(
        &self,
        input: UpdateCartQuantityInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Remove a product completely from the shopping cart
    async fn remove_from_cart(
        &self,
        input: RemoveFromCartInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Clear all items, addresses, and state from the shopping cart
    async fn clear_cart(&self, input: UserCartInput);

    /// Check saved shipping/billing addresses and email from previous orders
    async fn get_saved_contact_info(&self, input: UserCartInput) -> Option<SavedContactInfo>;

    /// Pre-fill the cart with saved contact details for 1-click checkout
    async fn apply_saved_contact_info(
        &self,
        input: UserCartInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Set recipient shipping destination address on the shopping cart
    async fn set_shipping_address(
        &self,
        input: SetAddressInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Set billing address on the shopping cart
    async fn set_billing_address(
        &self,
        input: SetAddressInput,
    ) -> Result<CartUpdated, CartToolError>;

    /// Set customer contact email on the shopping cart
    async fn set_email(&self, input: SetEmailInput) -> Result<CartUpdated, CartToolError>;

    /// Prepare checkout by reserving inventory for 10 minutes and generating an order draft
    async fn prepare_checkout(&self, input: UserCartInput) -> Result<Cart, CartToolError>;

    /// Confirm and place the order after explicit user confirmation
    async fn confirm_checkout(
        &self,
        input: CheckoutOrderInput,
    ) -> Result<OrderConfirmation, CartToolError>;

    /// Cancel an active checkout reservation to make changes to the cart
    async fn cancel_checkout(&self, input: CheckoutOrderInput) -> Result<(), CartToolError>;
}

pub struct CartToolsImpl;

#[tool_implementation]
impl CartTools for CartToolsImpl {
    async fn get_cart(&self, input: UserCartInput) -> Option<Cart> {
        let mut cart_client = CartAgentClient::get(input.user_id);
        cart_client.get_cart().await
    }

    async fn add_to_cart(&self, input: AddToCartInput) -> Result<CartUpdated, CartToolError> {
        let quantity = input.quantity.unwrap_or(1);
        let mut cart_client = CartAgentClient::get(input.user_id);
        cart_client
            .add_item(input.product_id, quantity)
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
        input: UpdateCartQuantityInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
        cart_client
            .update_item_quantity(input.product_id, input.quantity)
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
        input: RemoveFromCartInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
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

    async fn clear_cart(&self, input: UserCartInput) {
        let mut cart_client = CartAgentClient::get(input.user_id);
        cart_client.clear().await;
    }

    async fn get_saved_contact_info(&self, input: UserCartInput) -> Option<SavedContactInfo> {
        let cart_client = CartAgentClient::get(input.user_id);
        cart_client.get_saved_contact_info().await
    }

    async fn apply_saved_contact_info(
        &self,
        input: UserCartInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
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
        input: SetAddressInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
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
        input: SetAddressInput,
    ) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
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

    async fn set_email(&self, input: SetEmailInput) -> Result<CartUpdated, CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
        cart_client.update_email(input.email).await.map_err(|e| {
            let msg = match e {
                crate::cart::UpdateEmailError::EmailNotValid(err) => err.message,
                crate::cart::UpdateEmailError::ActionNotAllowed(err) => err.message,
            };
            CartToolError::OperationFailed { message: msg }
        })
    }

    async fn prepare_checkout(&self, input: UserCartInput) -> Result<Cart, CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
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
        input: CheckoutOrderInput,
    ) -> Result<OrderConfirmation, CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
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

    async fn cancel_checkout(&self, input: CheckoutOrderInput) -> Result<(), CartToolError> {
        let mut cart_client = CartAgentClient::get(input.user_id);
        cart_client
            .cancel_checkout(input.order_id)
            .await
            .map_err(|msg| CartToolError::OperationFailed { message: msg })
    }
}
