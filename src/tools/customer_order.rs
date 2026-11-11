use crate::ai::tools::ToolRegistry;
use crate::order::{Order, OrderAgentClient};
use crate::order_search::OrderSearchAgentClient;
use golem_rust::{Schema, tool_definition, tool_implementation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Retrieve past orders placed by this customer
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug, Default)]
pub struct GetCustomerOrderHistoryInput {
    /// Customer user ID
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
    /// Maximum number of past orders to return
    pub limit: Option<u32>,
}

/// Internal arguments for assistant get_order_history tool call
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug, Default)]
pub struct GetCustomerOrderHistoryArgs {
    /// Maximum number of past orders to return
    pub limit: Option<u32>,
}

/// Get full details of a specific order by order ID
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct GetCustomerOrderDetailsInput {
    /// Order ID (e.g. 'ord-123')
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: String,
}

/// Customer order tools for viewing order history and specific order status
#[tool_definition(version = "1.0.0")]
pub trait CustomerOrderTools {
    /// Retrieve past orders placed by this customer
    async fn get_order_history(&self, input: GetCustomerOrderHistoryInput) -> Vec<Order>;

    /// Get full details of a specific order by order ID
    async fn get_order_details(&self, input: GetCustomerOrderDetailsInput) -> Option<Order>;
}

pub struct CustomerOrderToolsImpl;

#[tool_implementation]
impl CustomerOrderTools for CustomerOrderToolsImpl {
    async fn get_order_history(&self, input: GetCustomerOrderHistoryInput) -> Vec<Order> {
        let client = OrderSearchAgentClient::new_phantom();
        client
            .get_orders_by_user_id(input.user_id, input.limit)
            .await
            .value
            .unwrap_or_default()
    }

    async fn get_order_details(&self, input: GetCustomerOrderDetailsInput) -> Option<Order> {
        let client = OrderAgentClient::get(input.order_id);
        client.get_order().await
    }
}

impl CustomerOrderToolsImpl {
    /// Registers customer order tools for a user into a `ToolRegistry` for LLM consumption.
    pub fn register(user_id: String, registry: &mut ToolRegistry) {
        let uid = user_id.clone();
        registry.register_tool_call(
            "get_order_history",
            move |args: GetCustomerOrderHistoryArgs| {
                let uid = uid.clone();
                async move {
                    CustomerOrderToolsClient::new()
                        .get_order_history(GetCustomerOrderHistoryInput {
                            user_id: uid,
                            limit: args.limit,
                        })
                        .await
                }
            },
            |orders| format!("Retrieved {} past order(s)", orders.len()),
        );

        registry.register_tool_call(
            "get_order_details",
            |args: GetCustomerOrderDetailsInput| async move {
                CustomerOrderToolsClient::new()
                    .get_order_details(args)
                    .await
            },
            |o| match o {
                Some(o) => format!(
                    "Order {} status: {:?} (total: ${:.2})",
                    o.order_id, o.order_status, o.total
                ),
                None => "Order not found".to_string(),
            },
        );
    }
}
