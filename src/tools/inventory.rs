use crate::ai::tools::ToolRegistry;
use crate::inventory::{Availability, InventoryAgentClient};
use golem_rust::{Schema, tool_definition, tool_implementation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Check available and total stock quantity for a product in a region
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct CheckInventoryInput {
    /// Product ID to check inventory for
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// Optional fulfillment region (defaults to 'global')
    pub region: Option<String>,
}

/// Inventory management tools for checking stock and regional availability
#[tool_definition(version = "1.0.0")]
pub trait InventoryTools {
    /// Check available stock quantity for a product in a region
    async fn check_inventory(&self, input: CheckInventoryInput) -> Availability;
}

pub struct InventoryToolsImpl;

#[tool_implementation]
impl InventoryTools for InventoryToolsImpl {
    async fn check_inventory(&self, input: CheckInventoryInput) -> Availability {
        let region = input.region.as_deref().unwrap_or("global");
        let inventory_client = InventoryAgentClient::get(input.product_id);
        inventory_client
            .get_available_quantity(region.to_string())
            .await
    }
}

impl InventoryToolsImpl {
    /// Registers inventory tools into a `ToolRegistry` for LLM consumption.
    pub fn register(registry: &mut ToolRegistry) {
        registry.register_tool_call(
            "check_inventory",
            |args| async move { InventoryToolsClient::new().check_inventory(args).await },
            |avail| {
                format!(
                    "Product {} has {} available in region {} (total: {})",
                    avail.product_id, avail.available_quantity, avail.region, avail.total_quantity
                )
            },
        );
    }
}
