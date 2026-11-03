use crate::common::{CURRENCY_DEFAULT, INVENTORY_REGION_DEFAULT, PRICING_REGION_DEFAULT};
use crate::inventory::InventoryAgentClient;
use crate::pricing::{PricingAgentClient, PricingItem};
use crate::product::{Product, ProductAgentClient};
use futures::future::join_all;
use golem_rust::{
    AgentAllFilter, AgentAnyFilter, AgentNameFilter, AgentPropertyFilter, ComponentId, GetAgents,
    Schema, StringFilterComparator, agent_definition, agent_implementation, description, endpoint,
    prompt, resolve_component_id,
};
use log::info;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ProductWithPrice {
    pub product_id: String,
    pub name: String,
    pub brand: String,
    pub description: String,
    pub tags: Vec<String>,
    pub price: Option<PricingItem>,
    pub available_quantity: Option<u32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
struct ProductQueryMatcher {
    terms: Vec<String>,
    field_filters: Vec<(String, String)>,
}

impl ProductQueryMatcher {
    // Parse a simple query string into terms and field filters
    fn new(query: &str) -> Self {
        let mut terms = Vec::new();
        let mut field_filters = Vec::new();

        let tokens = Self::tokenize(query);

        for part in tokens {
            if let Some((field, value)) = part.split_once(':') {
                field_filters.push((field.to_string(), value.to_string()));
            } else {
                terms.push(part.to_string());
            }
        }

        Self {
            terms,
            field_filters,
        }
    }

    // Tokenize the query string, handling quoted strings
    fn tokenize(query: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for c in query.chars() {
            match c {
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(current.trim().to_string());
                        current.clear();
                    }
                }
                '"' => {
                    in_quotes = !in_quotes;
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(current.trim().to_string());
        }

        tokens
    }

    // Check if a product matches the query
    fn matches(&self, product: Product) -> bool {
        fn text_matches(text: &str, query: &str) -> bool {
            query == "*" || text.to_lowercase().contains(&query.to_lowercase())
        }

        fn text_exact_matches(text: &str, query: &str) -> bool {
            query == "*" || text == query
        }

        // Check field filters first
        for (field, value) in self.field_filters.iter() {
            let matches = match field.to_lowercase().as_str() {
                "product-id" | "productid" => text_exact_matches(&product.product_id, value),
                "name" => text_matches(&product.name, value),
                "brand" => text_matches(&product.brand, value),
                "description" => text_matches(&product.description, value),
                "tag" | "tags" => product.tags.iter().any(|tag| text_matches(tag, value)),
                _ => false, // Unknown field
            };

            if !matches {
                return false;
            }
        }

        // If no terms to match, just check if field filters passed
        if self.terms.is_empty() {
            return true;
        }

        // Check search terms against all searchable fields
        for term in self.terms.iter() {
            let matches = text_matches(&product.name, term)
                || text_matches(&product.brand, term)
                || text_matches(&product.description, term)
                || product.tags.iter().any(|tag| text_matches(tag, term));

            if !matches {
                return false;
            }
        }

        true
    }
}

fn get_agent_filter() -> AgentAnyFilter {
    AgentAnyFilter {
        filters: vec![AgentAllFilter {
            filters: vec![AgentPropertyFilter::Name(AgentNameFilter {
                comparator: StringFilterComparator::StartsWith,
                value: "ProductAgent(".to_string(),
            })],
        }],
    }
}

fn get_product_agent_id(agent_name: &str) -> Option<String> {
    Regex::new(r#"ProductAgent\("([^)]+)"\)"#)
        .ok()?
        .captures(agent_name)
        .filter(|caps| caps.len() > 0)
        .map(|caps| caps[1].to_string())
}

async fn get_products(
    agent_ids: HashSet<String>,
    matcher: ProductQueryMatcher,
) -> Result<Vec<Product>, String> {
    let clients: Vec<ProductAgentClient> =
        agent_ids.into_iter().map(ProductAgentClient::get).collect();

    let tasks: Vec<_> = clients.iter().map(|client| client.get_product()).collect();

    let responses = join_all(tasks).await;

    let result: Vec<Product> = responses
        .into_iter()
        .flatten()
        .filter(|p| matcher.matches(p.clone()))
        .collect();

    Ok(result)
}

async fn fetch_products_by_ids(ids: &str) -> Vec<Product> {
    let product_ids: Vec<String> = ids
        .split(',')
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();

    info!("Get products - ids: ({})", product_ids.join(","));

    let tasks: Vec<_> = product_ids
        .into_iter()
        .map(|id| async move {
            let client = ProductAgentClient::get(id);
            client.get_product().await
        })
        .collect();

    let responses = join_all(tasks).await;
    responses.into_iter().flatten().collect()
}

async fn attach_pricing_to_products(
    products: Vec<Product>,
    currency: Option<String>,
) -> Vec<ProductWithPrice> {
    if products.is_empty() {
        return Vec::new();
    }

    let target_currency = currency.unwrap_or_else(|| CURRENCY_DEFAULT.to_string());
    let target_region = PRICING_REGION_DEFAULT.to_string();
    let inventory_region = INVENTORY_REGION_DEFAULT.to_string();

    let tasks: Vec<_> = products
        .into_iter()
        .map(|product| {
            let pid = product.product_id.clone();
            let curr = target_currency.clone();
            let reg = target_region.clone();
            let inv_reg = inventory_region.clone();

            async move {
                let pricing_client = PricingAgentClient::get(pid.clone());
                let inv_client = InventoryAgentClient::get(pid);

                let (price, avail) = futures::future::join(
                    pricing_client.get_price(curr, reg),
                    inv_client.get_available_quantity(inv_reg),
                )
                .await;

                ProductWithPrice {
                    product_id: product.product_id,
                    name: product.name,
                    brand: product.brand,
                    description: product.description,
                    tags: product.tags,
                    price,
                    available_quantity: Some(avail.available_quantity),
                    created_at: product.created_at,
                    updated_at: product.updated_at,
                }
            }
        })
        .collect();

    join_all(tasks).await
}

#[agent_definition(ephemeral, mount = "/v1/product")]
pub trait ProductSearchAgent {
    fn new() -> Self;

    #[endpoint(get = "/?ids={ids}")]
    #[description("Get products by comma-separated product IDs")]
    async fn get_by_ids(&self, ids: String) -> Result<Vec<Product>, String>;

    #[endpoint(get = "/search?query={query}")]
    #[description("Search products across the catalog by query keyword or field filter")]
    async fn search(&self, query: String) -> Result<Vec<Product>, String>;

    #[endpoint(get = "/with-prices?ids={ids}&currency={currency}")]
    #[prompt(
        "Call this to fetch specifications, descriptions, brands, tags, and prices for specific products."
    )]
    #[description("Get detailed product information with current pricing by product ID(s).")]
    async fn get_by_ids_with_prices(
        &self,
        ids: String,
        currency: Option<String>,
    ) -> Result<Vec<ProductWithPrice>, String>;

    #[endpoint(get = "/search-with-prices?query={query}&currency={currency}")]
    #[prompt(
        "Call this when the user is searching for products, asking recommendations, or looking for item options."
    )]
    #[description(
        "Search product catalog with keyword, brand, or tag filters including active prices."
    )]
    async fn search_with_prices(
        &self,
        query: String,
        currency: Option<String>,
    ) -> Result<Vec<ProductWithPrice>, String>;
}

struct ProductSearchAgentImpl {
    component_id: Option<ComponentId>,
}

impl ProductSearchAgentImpl {
    async fn search_products_by_query(&self, query: &str) -> Result<Vec<Product>, String> {
        let component_id = self
            .component_id
            .clone()
            .ok_or_else(|| "Component not found".to_string())?;

        info!("searching for products - query: {}", query);

        let mut values: Vec<Product> = Vec::new();
        let matcher = ProductQueryMatcher::new(query);

        let filter = get_agent_filter();

        let get_agents = GetAgents::new(component_id, Some(&filter), false);

        let mut processed_agent_ids: HashSet<String> = HashSet::new();

        while let Some(agents) = get_agents.get_next().map_err(|e| e.to_string())? {
            let agent_ids = agents
                .iter()
                .filter_map(|a| get_product_agent_id(a.agent_id.agent_id.as_str()))
                .filter(|n| !processed_agent_ids.contains(n))
                .collect::<HashSet<_>>();

            let products = get_products(agent_ids.clone(), matcher.clone()).await?;
            processed_agent_ids.extend(agent_ids);
            values.extend(products);
        }

        Ok(values)
    }
}

#[agent_implementation]
impl ProductSearchAgent for ProductSearchAgentImpl {
    fn new() -> Self {
        let component_id = resolve_component_id("shopping-rust:shopping");
        ProductSearchAgentImpl { component_id }
    }

    async fn get_by_ids(&self, ids: String) -> Result<Vec<Product>, String> {
        Ok(fetch_products_by_ids(&ids).await)
    }

    async fn search(&self, query: String) -> Result<Vec<Product>, String> {
        self.search_products_by_query(&query).await
    }

    async fn get_by_ids_with_prices(
        &self,
        ids: String,
        currency: Option<String>,
    ) -> Result<Vec<ProductWithPrice>, String> {
        let products = fetch_products_by_ids(&ids).await;
        Ok(attach_pricing_to_products(products, currency).await)
    }

    async fn search_with_prices(
        &self,
        query: String,
        currency: Option<String>,
    ) -> Result<Vec<ProductWithPrice>, String> {
        let products = self.search_products_by_query(&query).await?;
        Ok(attach_pricing_to_products(products, currency).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_product_agent_id() {
        assert_eq!(
            get_product_agent_id(r#"ProductAgent("ap001")"#),
            Some("ap001".to_string())
        );
        assert_eq!(get_product_agent_id(r#"OrderAgent("ord-1")"#), None);
    }

    #[test]
    fn test_query_matcher_terms() {
        let now = chrono::Utc::now();
        let product = Product {
            product_id: "ap001".to_string(),
            name: "iPhone 15 Pro".to_string(),
            brand: "Apple".to_string(),
            description: "Titanium design, A17 Pro chip".to_string(),
            tags: vec!["smartphone".to_string(), "apple".to_string()],
            created_at: now,
            updated_at: now,
        };

        let matcher = ProductQueryMatcher::new("iphone");
        assert!(matcher.matches(product.clone()));

        let matcher_brand = ProductQueryMatcher::new("brand:Apple");
        assert!(matcher_brand.matches(product.clone()));

        let matcher_wrong = ProductQueryMatcher::new("Samsung");
        assert!(!matcher_wrong.matches(product));
    }
}
