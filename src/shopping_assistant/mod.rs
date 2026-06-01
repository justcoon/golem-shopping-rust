use crate::cart::CartAgentClient;
use crate::order::{OrderAgentClient, OrderItem};
use futures::future::join_all;
use golem_ai_llm::LlmProvider;
use golem_ai_llm::config::SecretSource;
use golem_rust::agentic::{Config, Secret};
use golem_rust::{ConfigSchema, Schema, agent_definition, agent_implementation, endpoint};
use golem_rust::retry::*;
use std::time::Duration;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const RECOMMENDATION_INPUT_COUNT: u8 = 100;
pub const RECOMMENDATION_PRODUCT_COUNT: u8 = 4;
pub const RECOMMENDATION_BRAND_COUNT: u8 = 3;

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
    use golem_ai_llm::model::*;
    log::info!("LLM recommendations - items: {}", items.len());
    let current_items: Vec<LlmOrderItem> = items.into_iter().map(LlmOrderItem::from).collect();
    let current_items_string = serde_json::to_string(&current_items).map_err(|e| e.to_string())?;

    // let policy = NamedPolicy::named(
    //     "generate-embedding",
    //     Policy::exponential(Duration::from_millis(200), 2.0)
    //         .clamp(Duration::from_millis(100), Duration::from_secs(5))
    //         .with_jitter(0.15)
    //         .max_retries(5)
    // );

    let provider_config = golem_ai_llm_openrouter::OpenRouterConfig {
        api_key: SecretSource::from_handle(config.api_key),
    };

    let config = Config {
        model: config.model.get(),
        max_tokens: None,
        temperature: None,
        stop_sequences: None,
        tools: None,
        tool_choice: None,
        provider_options: Some(vec![Kv {
            key: "responseFormat".to_string(),
            value: "json_object".to_string(),
        }]),
    };

    let schema = schema_for!(LlmRecommendedItems);
    let schema_json = serde_json::to_string_pretty(&schema).map_err(|e| e.to_string())?;

    let system_message = format!(
        r#"
            You MUST respond with JSON in the following schema:
                {schema_json}
            Return ONLY valid JSON, no other text.
        "#
    );

    let system_event = Event::Message(Message {
        role: Role::System,
        name: None,
        content: vec![ContentPart::Text(system_message.to_string())],
    });

    let user_message = format!(
        r#"
           We have a list of order items: {current_items_string}.
           Can you do {RECOMMENDATION_PRODUCT_COUNT} recommendations for products items to buy based on previous order items.
           Can you do {RECOMMENDATION_BRAND_COUNT} recommendations for product brands to buy based on previous order items.
           Return the list of product_id-s and list of product_brand-s as a valid JSON object. Return JSON only.
        "#
    );

    let user_event = Event::Message(Message {
        role: Role::User,
        name: None,
        content: vec![ContentPart::Text(user_message.to_string())],
    });

    // with_named_policy_async(policy)

    let llm_response = golem_ai_llm_openrouter::DurableOpenRouter::send(
        provider_config,
        vec![system_event, user_event],
        config,
    )
    .await;

    match llm_response {
        Ok(response) => {
            let response_content = response
                .content
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text(text) => Some(text.clone()),
                    _ => None,
                })
                .collect::<String>();

            let json_str = response_content
                .trim()
                .strip_prefix("```json")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(&response_content)
                .trim();

            serde_json::from_str(json_str).map_err(|e| {
                log::error!("LLM recommendations - response: {}, error: {}", json_str, e);
                e.to_string()
            })
        }
        Err(e) => {
            log::error!("LLM recommendations - error: {:?}", e);
            Err(format!("{:?}", e))
        }
    }
}

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

#[derive(ConfigSchema)]
pub struct LlmConfig {
    #[config_schema(secret)]
    pub api_key: Secret<String>,
    #[config_schema(secret)]
    pub model: Secret<String>,
}

#[derive(ConfigSchema)]
pub struct AgentConfig {
    #[config_schema(nested)]
    pub llm: LlmConfig,
}

#[agent_definition(mount = "/v1/assistant/{id}")]
trait ShoppingAssistantAgent {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self;

    #[endpoint(get = "/recommended-items")]
    fn get_recommended_items(&self) -> RecommendedItems;

    async fn recommend_items(&mut self) -> bool;
}

struct ShoppingAssistantAgentImpl {
    _id: String,
    config: Config<AgentConfig>,
    recommended_items: RecommendedItems,
}

#[agent_implementation]
impl ShoppingAssistantAgent for ShoppingAssistantAgentImpl {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self {
        ShoppingAssistantAgentImpl {
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
        let recommended_items = get_llm_recommendations(order_items, self.config.get().llm).await;

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

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: RecommendedItems = crate::common::snapshot::deserialize(&bytes)?;
        self.recommended_items = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.recommended_items)
    }
}
