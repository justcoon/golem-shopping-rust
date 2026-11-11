use crate::ai::tools::ToolRegistry;
use crate::common::Address;
use crate::order::{Order, OrderAgentClient, OrderStatus, OrderUpdated};
use crate::order_search::OrderSearchAgentClient;
use golem_rust::{Schema, ToolError, tool_definition, tool_implementation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Search orders across system by query text, status, or customer userId
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct SearchOrdersInput {
    /// Free text query matching products, customer, or address
    pub query: Option<String>,
    /// Order status filter: 'new', 'shipped', or 'cancelled'
    pub status: Option<String>,
    /// Customer user ID to filter by
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: Option<String>,
    /// Max number of orders to return
    pub limit: Option<u32>,
}

/// List recent orders across the store
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct ListRecentOrdersInput {
    /// Max number of recent orders (default 10)
    pub limit: Option<u32>,
}

/// Get full details of a specific order including items, total, status, and shipping address
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct GetOrderDetailsInput {
    /// Order ID to look up
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
}

/// Order ID for status updates or tracking
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct OrderIdInput {
    /// Order ID
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
}

/// Add a product item to an existing new order. Reserves required inventory.
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct AddOrderItemInput {
    /// Order ID to modify
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
    /// Product ID to add
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// Quantity to add
    pub quantity: u32,
}

/// Remove an item from an existing new order. Releases reserved inventory.
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct RemoveOrderItemInput {
    /// Order ID to modify
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
    /// Product ID to remove
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
}

/// Update quantity of an item in an existing new order. Adjusts inventory reservations.
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateOrderItemQuantityInput {
    /// Order ID to modify
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
    /// Product ID in order
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// New target quantity
    pub quantity: u32,
}

/// Address details for updating shipping or billing address on an order
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateOrderAddressInput {
    /// Order ID to modify
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
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

impl From<UpdateOrderAddressInput> for Address {
    fn from(input: UpdateOrderAddressInput) -> Self {
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

/// Update customer contact email on an existing new order
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateOrderEmailInput {
    /// Order ID to modify
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
    /// Updated email address
    pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToolError)]
pub enum OrderToolError {
    #[tool_error(kind = "runtime-error", exit_code = 1)]
    OperationFailed { message: String },
}

impl std::fmt::Display for OrderToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderToolError::OperationFailed { message } => write!(f, "{message}"),
        }
    }
}

/// Store manager tools for order search, fulfillment, and lifecycle management
#[allow(clippy::too_many_arguments)]
#[tool_definition(version = "1.0.0")]
pub trait ManagerOrderTools {
    /// Search orders across system by query text, status, or customer userId
    async fn search_orders(&self, input: SearchOrdersInput) -> Result<Vec<Order>, OrderToolError>;

    /// List the most recent orders across the store
    async fn list_recent_orders(
        &self,
        input: ListRecentOrdersInput,
    ) -> Result<Vec<Order>, OrderToolError>;

    /// Get full details of a specific order by order ID
    async fn get_order_details(&self, input: GetOrderDetailsInput) -> Option<Order>;

    /// Ship an existing new order
    async fn ship_order(&self, input: OrderIdInput) -> Result<OrderUpdated, OrderToolError>;

    /// Cancel an existing new order
    async fn cancel_order(&self, input: OrderIdInput) -> Result<OrderUpdated, OrderToolError>;

    /// Add a product item to an existing order
    async fn add_order_item(
        &self,
        input: AddOrderItemInput,
    ) -> Result<OrderUpdated, OrderToolError>;

    /// Remove a product item from an existing order
    async fn remove_order_item(
        &self,
        input: RemoveOrderItemInput,
    ) -> Result<OrderUpdated, OrderToolError>;

    /// Update quantity of an item on an existing order
    async fn update_order_item_quantity(
        &self,
        input: UpdateOrderItemQuantityInput,
    ) -> Result<OrderUpdated, OrderToolError>;

    /// Update recipient shipping address on an existing order
    async fn update_order_shipping_address(
        &self,
        input: UpdateOrderAddressInput,
    ) -> Result<OrderUpdated, OrderToolError>;

    /// Update billing address on an existing order
    async fn update_order_billing_address(
        &self,
        input: UpdateOrderAddressInput,
    ) -> Result<OrderUpdated, OrderToolError>;

    /// Update customer email on an existing order
    async fn update_order_email(
        &self,
        input: UpdateOrderEmailInput,
    ) -> Result<OrderUpdated, OrderToolError>;
}

pub struct ManagerOrderToolsImpl;

#[tool_implementation]
impl ManagerOrderTools for ManagerOrderToolsImpl {
    async fn search_orders(&self, input: SearchOrdersInput) -> Result<Vec<Order>, OrderToolError> {
        let status = input.status.and_then(|s| match s.to_lowercase().as_str() {
            "new" => Some(OrderStatus::New),
            "shipped" => Some(OrderStatus::Shipped),
            "cancelled" | "canceled" => Some(OrderStatus::Cancelled),
            _ => None,
        });

        let client = OrderSearchAgentClient::new_phantom();
        client
            .search_orders(input.query, status, input.user_id, input.limit)
            .await
            .value
            .map_err(|e| OrderToolError::OperationFailed { message: e })
    }

    async fn list_recent_orders(
        &self,
        input: ListRecentOrdersInput,
    ) -> Result<Vec<Order>, OrderToolError> {
        let client = OrderSearchAgentClient::new_phantom();
        client
            .get_recent_orders(input.limit)
            .await
            .value
            .map_err(|e| OrderToolError::OperationFailed { message: e })
    }

    async fn get_order_details(&self, input: GetOrderDetailsInput) -> Option<Order> {
        let client = OrderAgentClient::get(input.order_id);
        client.get_order().await
    }

    async fn ship_order(&self, input: OrderIdInput) -> Result<OrderUpdated, OrderToolError> {
        let mut order_client = OrderAgentClient::get(input.order_id);
        order_client.ship_order().await.map_err(|e| {
            let msg = match e {
                crate::order::ShipOrderError::ActionNotAllowed(err) => err.message,
                crate::order::ShipOrderError::EmptyItems(err) => err.message,
                crate::order::ShipOrderError::EmptyEmail(err) => err.message,
                crate::order::ShipOrderError::BillingAddressNotSet(err) => err.message,
            };
            OrderToolError::OperationFailed { message: msg }
        })
    }

    async fn cancel_order(&self, input: OrderIdInput) -> Result<OrderUpdated, OrderToolError> {
        let mut order_client = OrderAgentClient::get(input.order_id);
        order_client.cancel_order().await.map_err(
            |crate::order::CancelOrderError::ActionNotAllowed(err)| {
                OrderToolError::OperationFailed {
                    message: err.message,
                }
            },
        )
    }

    async fn add_order_item(
        &self,
        input: AddOrderItemInput,
    ) -> Result<OrderUpdated, OrderToolError> {
        let mut client = OrderAgentClient::get(input.order_id);
        client
            .add_item(input.product_id, input.quantity)
            .await
            .map_err(|e| {
                let msg = match e {
                    crate::order::AddItemError::ProductNotFound(err) => err.message,
                    crate::order::AddItemError::PricingNotFound(err) => err.message,
                    crate::order::AddItemError::ActionNotAllowed(err) => err.message,
                    crate::order::AddItemError::InsufficientInventory(err) => err.message,
                };
                OrderToolError::OperationFailed { message: msg }
            })
    }

    async fn remove_order_item(
        &self,
        input: RemoveOrderItemInput,
    ) -> Result<OrderUpdated, OrderToolError> {
        let mut client = OrderAgentClient::get(input.order_id);
        client.remove_item(input.product_id).await.map_err(|e| {
            let msg = match e {
                crate::order::RemoveItemError::ActionNotAllowed(err) => err.message,
                crate::order::RemoveItemError::ItemNotFound(err) => err.message,
            };
            OrderToolError::OperationFailed { message: msg }
        })
    }

    async fn update_order_item_quantity(
        &self,
        input: UpdateOrderItemQuantityInput,
    ) -> Result<OrderUpdated, OrderToolError> {
        let mut client = OrderAgentClient::get(input.order_id);
        client
            .update_item_quantity(input.product_id, input.quantity)
            .await
            .map_err(|e| {
                let msg = match e {
                    crate::order::UpdateItemQuantityError::ActionNotAllowed(err) => err.message,
                    crate::order::UpdateItemQuantityError::ItemNotFound(err) => err.message,
                    crate::order::UpdateItemQuantityError::InsufficientInventory(err) => {
                        err.message
                    }
                };
                OrderToolError::OperationFailed { message: msg }
            })
    }

    async fn update_order_shipping_address(
        &self,
        input: UpdateOrderAddressInput,
    ) -> Result<OrderUpdated, OrderToolError> {
        let mut client = OrderAgentClient::get(input.order_id.clone());
        client
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
                    crate::order::UpdateAddressError::AddressNotValid(err) => err.message,
                    crate::order::UpdateAddressError::ActionNotAllowed(err) => err.message,
                };
                OrderToolError::OperationFailed { message: msg }
            })
    }

    async fn update_order_billing_address(
        &self,
        input: UpdateOrderAddressInput,
    ) -> Result<OrderUpdated, OrderToolError> {
        let mut client = OrderAgentClient::get(input.order_id.clone());
        client
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
                    crate::order::UpdateAddressError::AddressNotValid(err) => err.message,
                    crate::order::UpdateAddressError::ActionNotAllowed(err) => err.message,
                };
                OrderToolError::OperationFailed { message: msg }
            })
    }

    async fn update_order_email(
        &self,
        input: UpdateOrderEmailInput,
    ) -> Result<OrderUpdated, OrderToolError> {
        let mut client = OrderAgentClient::get(input.order_id);
        client.update_email(input.email).await.map_err(|e| {
            let msg = match e {
                crate::order::UpdateEmailError::ActionNotAllowed(err) => err.message,
                crate::order::UpdateEmailError::EmailNotValid(err) => err.message,
            };
            OrderToolError::OperationFailed { message: msg }
        })
    }
}

impl ManagerOrderToolsImpl {
    /// Registers all manager order tools into a `ToolRegistry` for LLM consumption.
    pub fn register(registry: &mut ToolRegistry) {
        registry.register_tool_call(
            "search_orders",
            |args| async move { ManagerOrderToolsClient::new().search_orders(args).await },
            |orders| format!("Found {} matching order(s)", orders.len()),
        );

        registry.register_tool_call(
            "list_recent_orders",
            |args| async move {
                ManagerOrderToolsClient::new()
                    .list_recent_orders(args)
                    .await
            },
            |orders| format!("Retrieved {} recent order(s)", orders.len()),
        );

        registry.register_tool_call(
            "get_order_details",
            |args| async move { ManagerOrderToolsClient::new().get_order_details(args).await },
            |o| match o {
                Some(o) => format!(
                    "Retrieved order {} (Status: {:?}, Total: ${:.2}, Items: {})",
                    o.order_id,
                    o.order_status,
                    o.total,
                    o.items.len()
                ),
                None => "Order not found".to_string(),
            },
        );

        registry.register_tool_call_with_desc(
            "ship_order",
            "Ship an existing new order. Fulfills inventory allocations and marks order status as 'shipped'.",
            |args| async move {
                ManagerOrderToolsClient::new()
                    .ship_order(args)
                    .await
            },
            |u| format!("Order {} has been successfully shipped", u.order_id),
        );

        registry.register_tool_call_with_desc(
            "cancel_order",
            "Cancel an existing new order. Releases reserved inventory allocations and marks order status as 'cancelled'.",
            |args| async move {
                ManagerOrderToolsClient::new()
                    .cancel_order(args)
                    .await
            },
            |u| format!("Order {} has been successfully cancelled", u.order_id),
        );

        registry.register_tool_call(
            "add_order_item",
            |args| async move { ManagerOrderToolsClient::new().add_order_item(args).await },
            |u| format!("Added item to order {}", u.order_id),
        );

        registry.register_tool_call(
            "remove_order_item",
            |args| async move { ManagerOrderToolsClient::new().remove_order_item(args).await },
            |u| format!("Removed item from order {}", u.order_id),
        );

        registry.register_tool_call(
            "update_order_item_quantity",
            |args| async move {
                ManagerOrderToolsClient::new()
                    .update_order_item_quantity(args)
                    .await
            },
            |u| format!("Updated item quantity on order {}", u.order_id),
        );

        registry.register_tool_call_with_desc(
            "update_order_shipping_address",
            "Update recipient shipping address on an existing new order",
            |args| async move {
                ManagerOrderToolsClient::new()
                    .update_order_shipping_address(args)
                    .await
            },
            |u| format!("Updated shipping address on order {}", u.order_id),
        );

        registry.register_tool_call_with_desc(
            "update_order_billing_address",
            "Update billing address on an existing new order",
            |args| async move {
                ManagerOrderToolsClient::new()
                    .update_order_billing_address(args)
                    .await
            },
            |u| format!("Updated billing address on order {}", u.order_id),
        );

        registry.register_tool_call(
            "update_order_email",
            |args| async move {
                ManagerOrderToolsClient::new()
                    .update_order_email(args)
                    .await
            },
            |u| format!("Updated customer email on order {}", u.order_id),
        );
    }
}
