#![allow(clippy::too_many_arguments)]

use crate::ai::tools::ToolRegistry;
use crate::inventory::{Availability, InventoryAgentClient};
use crate::product::{Product, ProductAgentClient};
use crate::product_search::{ProductSearchAgentClient, ProductWithPrice};
use golem_rust::{Schema, tool_definition, tool_implementation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Search product catalog for items by name, brand, or query keywords with active prices
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct SearchProductsInput {
    /// Search keyword, brand, or category
    pub query: String,
    /// Currency code, e.g. 'USD'
    pub currency: Option<String>,
}

/// Get detailed product specifications, price, and inventory by product ID
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct GetProductDetailsInput {
    /// Product ID, e.g. 'ap001'
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// Currency code, e.g. 'USD'
    pub currency: Option<String>,
}

/// Check available and total stock quantity for a product in a region
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct CheckInventoryInput {
    /// Product ID to check inventory for
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// Region name (defaults to 'global')
    pub region: Option<String>,
}

/// Check basic product details (name, brand, tags) by product ID
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct CheckProductDetailsInput {
    /// Product ID to inspect
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
}

#[allow(clippy::too_many_arguments)]
#[tool_definition(version = "1.0.0")]
pub trait CatalogTool {
    /// Search product catalog for items by name, brand, or query keywords with active prices
    async fn search_products(&self, input: SearchProductsInput) -> Vec<ProductWithPrice>;

    /// Get detailed product specifications, price, and inventory by product ID
    async fn get_product_details(&self, input: GetProductDetailsInput) -> Option<ProductWithPrice>;

    /// Check available stock quantity for a product in a region
    async fn check_inventory(&self, input: CheckInventoryInput) -> Availability;

    /// Check basic product details (name, brand, tags)
    async fn check_product_details(&self, input: CheckProductDetailsInput) -> Option<Product>;
}

pub struct CatalogToolImpl;

#[tool_implementation]
impl CatalogTool for CatalogToolImpl {
    async fn search_products(&self, input: SearchProductsInput) -> Vec<ProductWithPrice> {
        let client = ProductSearchAgentClient::new_phantom();
        client
            .search_with_prices(input.query.clone(), input.currency)
            .await
            .value
            .unwrap_or_default()
    }

    async fn get_product_details(&self, input: GetProductDetailsInput) -> Option<ProductWithPrice> {
        let client = ProductSearchAgentClient::new_phantom();
        match client
            .get_by_ids_with_prices(input.product_id.clone(), input.currency)
            .await
            .value
        {
            Ok(mut products) => products.pop(),
            Err(_) => None,
        }
    }

    async fn check_inventory(&self, input: CheckInventoryInput) -> Availability {
        let region = input.region.unwrap_or_else(|| "global".to_string());
        let client = InventoryAgentClient::get(input.product_id.clone());
        client.get_available_quantity(region).await
    }

    async fn check_product_details(&self, input: CheckProductDetailsInput) -> Option<Product> {
        let client = ProductAgentClient::get(input.product_id);
        client.get_product().await
    }
}

impl CatalogToolImpl {
    /// Registers all catalog tools directly into a `ToolRegistry` for LLM consumption.
    pub fn register(registry: &mut ToolRegistry) {
        registry.register_tool_call(
            "search_products",
            |args| async move { CatalogToolClient::new().search_products(args).await },
            |products| format!("Found {} product(s)", products.len()),
        );

        registry.register_tool_call(
            "get_product_details",
            |args| async move { CatalogToolClient::new().get_product_details(args).await },
            |p| match p {
                Some(p) => format!("Details for {} ({})", p.name, p.product_id),
                None => "Product not found".to_string(),
            },
        );

        registry.register_tool_call(
            "check_inventory",
            |args| async move { CatalogToolClient::new().check_inventory(args).await },
            |avail| {
                format!(
                    "Product {} has {} available in region {} (total: {})",
                    avail.product_id, avail.available_quantity, avail.region, avail.total_quantity
                )
            },
        );

        registry.register_tool_call(
            "check_product_details",
            |args| async move { CatalogToolClient::new().check_product_details(args).await },
            |p| match p {
                Some(p) => format!("Product {}: {} ({})", p.product_id, p.name, p.brand),
                None => "Product not found".to_string(),
            },
        );
    }
}
