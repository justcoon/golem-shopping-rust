use crate::ai::{
    AgentConfig, LlmConfig, NoopDispatcher, execute_agent_chat_turn, extract_json_str,
};
use crate::cart::CartAgentClient;
use crate::order::{OrderAgentClient, OrderItem};
use futures::future::join_all;
use golem_rust::agentic::Config;
use golem_rust::{Schema, agent_definition, agent_implementation, endpoint, read_only};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const RECOMMENDATION_INPUT_COUNT: u8 = 100;
pub const RECOMMENDATION_PRODUCT_COUNT: u8 = 4;
pub const RECOMMENDATION_BRAND_COUNT: u8 = 3;

#[derive(Serialize, Deserialize, Clone)]
pub struct LlmOrderItem {
    pub product_id: String,
    pub product_name: String,
    pub product_brand: String,
    pub price: f32,
    pub quantity: u32,
}

impl From<OrderItem> for LlmOrderItem {
    fn from(item: OrderItem) -> Self {
        LlmOrderItem {
            product_id: item.product_id,
            product_name: item.product_name,
            product_brand: item.product_brand,
            price: item.price,
            quantity: item.quantity,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct LlmRecommendedItems {
    pub product_ids: Vec<String>,
    pub product_brands: Vec<String>,
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct RecommendedItems {
    pub product_ids: Vec<String>,
    pub product_brands: Vec<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

async fn get_order_items(id: String) -> Vec<OrderItem> {
    let cart = CartAgentClient::get(id).get_cart().await;

    if let Some(cart) = cart {
        let order_ids = cart.previous_order_ids;

        let clients: Vec<_> = order_ids
            .into_iter()
            .map(|order_id| OrderAgentClient::get(order_id.clone()))
            .collect();

        let tasks: Vec<_> = clients.iter().map(|client| client.get_order()).collect();

        let orders = join_all(tasks).await;

        let items = orders.into_iter().flatten().flat_map(|o| o.items).collect();

        reduce_order_items(items)
    } else {
        vec![]
    }
}

fn reduce_order_items(items: Vec<OrderItem>) -> Vec<OrderItem> {
    let mut items_map: HashMap<String, OrderItem> = HashMap::new();

    for item in items {
        items_map
            .entry(item.product_id.clone())
            .and_modify(|i| {
                i.quantity += item.quantity;
            })
            .or_insert(item);
    }

    let mut result: Vec<_> = items_map.values().cloned().collect();

    result.sort_by_key(|v| v.quantity);

    result
        .into_iter()
        .take(RECOMMENDATION_INPUT_COUNT as usize)
        .collect()
}

async fn get_llm_recommendations(
    items: Vec<OrderItem>,
    config: LlmConfig,
) -> Result<LlmRecommendedItems, String> {
    log::info!("LLM recommendations - items: {}", items.len());
    let current_items: Vec<LlmOrderItem> = items.into_iter().map(LlmOrderItem::from).collect();
    let current_items_string = serde_json::to_string(&current_items).map_err(|e| e.to_string())?;

    let schema = schema_for!(LlmRecommendedItems);
    let schema_json = serde_json::to_string_pretty(&schema).map_err(|e| e.to_string())?;

    let system_message = format!(
        r#"
            You MUST respond with JSON in the following schema:
                {schema_json}
            Return ONLY valid JSON, no other text.
        "#
    );

    let user_message = format!(
        r#"
           We have a list of order items: {current_items_string}.
           Can you do {RECOMMENDATION_PRODUCT_COUNT} recommendations for products items to buy based on previous order items.
           Can you do {RECOMMENDATION_BRAND_COUNT} recommendations for product brands to buy based on previous order items.
           Return the list of product_id-s and list of product_brand-s as a valid JSON object. Return JSON only.
        "#
    );

    let api_key = config.api_key.get().unwrap_or_default();
    let model = config.model.get().unwrap_or_default();
    let api_url = config.api_url.get().ok();

    let result = execute_agent_chat_turn(
        &system_message,
        &[],
        &api_key,
        &model,
        api_url.as_deref(),
        &[],
        &user_message,
        Some("Recommendation agent is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets."),
        &NoopDispatcher,
    )
    .await;

    if result.is_error {
        log::error!("LLM recommendations failed: {}", result.reply);
        return Err(result.reply);
    }

    let json_str = extract_json_str(&result.reply);
    serde_json::from_str(json_str).map_err(|e| {
        log::error!("LLM recommendations - response: {}, error: {}", json_str, e);
        e.to_string()
    })
}

#[agent_definition(mount = "/v1/recommendation/{id}")]
pub trait ShoppingRecommendationAgent {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self;

    #[read_only]
    #[endpoint(get = "/recommended-items")]
    fn get_recommended_items(&self) -> RecommendedItems;

    async fn recommend_items(&mut self) -> bool;
}

pub struct ShoppingRecommendationAgentImpl {
    _id: String,
    config: Config<AgentConfig>,
    recommended_items: RecommendedItems,
}

#[agent_implementation]
impl ShoppingRecommendationAgent for ShoppingRecommendationAgentImpl {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self {
        ShoppingRecommendationAgentImpl {
            _id: id,
            config,
            recommended_items: RecommendedItems {
                product_ids: Vec::new(),
                product_brands: Vec::new(),
                updated_at: chrono::Utc::now(),
            },
        }
    }

    fn get_recommended_items(&self) -> RecommendedItems {
        self.recommended_items.clone()
    }

    async fn recommend_items(&mut self) -> bool {
        let order_items = get_order_items(self._id.clone()).await;
        let llm_config = match self.config.get() {
            Ok(config) => config.llm,
            Err(e) => {
                log::error!("Config error: {:?}", e);
                return false;
            }
        };
        let recommended_items = get_llm_recommendations(order_items, llm_config).await;

        match recommended_items {
            Ok(recommended_items) => {
                log::info!(
                    "Recommended items - product count: {}, product brands count: {}",
                    recommended_items.product_ids.len(),
                    recommended_items.product_brands.len()
                );
                self.recommended_items = RecommendedItems {
                    product_ids: recommended_items.product_ids,
                    product_brands: recommended_items.product_brands,
                    updated_at: chrono::Utc::now(),
                };
                true
            }
            Err(e) => {
                log::error!("Recommended items - error: {}", e);
                false
            }
        }
    }

    async fn load_snapshot(
        bytes: Vec<u8>,
        context: golem_rust::agentic::SnapshotRestoreContext,
    ) -> Result<Self, String> {
        let data: RecommendedItems = crate::common::snapshot::deserialize(&bytes)?;
        let mut parameters = golem_rust::agentic::DirectAgentInput::new(context.parameters)
            .map_err(|error| error.to_string())?;
        let id: String = parameters.take().map_err(|error| error.to_string())?;
        parameters.finish().map_err(|error| error.to_string())?;

        Ok(Self {
            _id: id,
            config: Config::new(),
            recommended_items: data,
        })
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.recommended_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reduce_order_items() {
        let items = vec![
            OrderItem {
                product_id: "prod-1".to_string(),
                product_name: "Item 1".to_string(),
                product_brand: "Brand A".to_string(),
                price: 10.0,
                quantity: 2,
            },
            OrderItem {
                product_id: "prod-1".to_string(),
                product_name: "Item 1".to_string(),
                product_brand: "Brand A".to_string(),
                price: 10.0,
                quantity: 3,
            },
            OrderItem {
                product_id: "prod-2".to_string(),
                product_name: "Item 2".to_string(),
                product_brand: "Brand B".to_string(),
                price: 20.0,
                quantity: 1,
            },
        ];

        let reduced = reduce_order_items(items);
        assert_eq!(reduced.len(), 2);
        // Sorted ascending by quantity
        assert_eq!(reduced[0].product_id, "prod-2");
        assert_eq!(reduced[0].quantity, 1);
        assert_eq!(reduced[1].product_id, "prod-1");
        assert_eq!(reduced[1].quantity, 5);
    }

    #[test]
    fn test_llm_recommended_items_deserialization() {
        let json_data = r#"{
            "product_ids": ["prod-1", "prod-2"],
            "product_brands": ["Brand A", "Brand B"]
        }"#;

        let parsed: Result<LlmRecommendedItems, _> = serde_json::from_str(json_data);
        assert!(parsed.is_ok());
        let items = parsed.unwrap();
        assert_eq!(items.product_ids, vec!["prod-1", "prod-2"]);
        assert_eq!(items.product_brands, vec!["Brand A", "Brand B"]);
    }
}

