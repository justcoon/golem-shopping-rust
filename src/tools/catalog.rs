use crate::product_search::{ProductSearchAgentClient, ProductWithPrice};
use golem_rust::{Schema, tool_definition, tool_implementation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Search product catalog for items by name, brand, or query keywords with active prices
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct SearchProductsInput {
    /// Search query string, keywords, or brand
    pub query: String,
    /// Optional currency code, e.g. USD, EUR
    pub currency: Option<String>,
}

/// Get detailed product specifications, price, and inventory by product ID
#[derive(Schema, Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct GetProductDetailsInput {
    /// Unique product identifier from catalog search
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    /// Optional currency code, e.g. USD, EUR
    pub currency: Option<String>,
}

/// Store catalog tools for searching products and viewing item details
#[tool_definition(version = "1.0.0")]
pub trait CatalogTools {
    /// Search product catalog for items by name, brand, or query keywords with active prices
    async fn search_products(&self, input: SearchProductsInput) -> Vec<ProductWithPrice>;

    /// Get detailed product specifications, price, and inventory by product ID
    async fn get_product_details(&self, input: GetProductDetailsInput) -> Option<ProductWithPrice>;
}

pub struct CatalogToolsImpl;

#[tool_implementation]
impl CatalogTools for CatalogToolsImpl {
    async fn search_products(&self, input: SearchProductsInput) -> Vec<ProductWithPrice> {
        let client = ProductSearchAgentClient::new_phantom();
        client
            .search_with_prices(input.query, input.currency)
            .await
            .value
            .unwrap_or_default()
    }

    async fn get_product_details(&self, input: GetProductDetailsInput) -> Option<ProductWithPrice> {
        let client = ProductSearchAgentClient::new_phantom();
        match client
            .get_by_ids_with_prices(input.product_id, input.currency)
            .await
            .value
        {
            Ok(mut products) => products.pop(),
            Err(_) => None,
        }
    }
}
