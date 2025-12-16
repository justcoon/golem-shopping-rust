use crate::cart::CartAgentClient;
use crate::order::{OrderAgentClient, OrderItem};
use futures::future::join_all;
use golem_rust::golem_ai::golem::llm::llm;
use golem_rust::{agent_definition, agent_implementation, Schema};
use schemars::{schema_for, JsonSchema};
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

async fn get_llm_recommendations(items: Vec<OrderItem>) -> Result<LlmRecommendedItems, String> {
    println!("LLM recommendations - items: {}", items.len());

    let current_items: Vec<LlmOrderItem> = items.into_iter().map(LlmOrderItem::from).collect();
    let current_items_string = serde_json::to_string(&current_items).map_err(|e| e.to_string())?;

    let config = llm::Config {
        model: "tngtech/deepseek-r1t2-chimera:free".to_string(),
        max_tokens: None,
        temperature: None,
        stop_sequences: None,
        tools: None,
        tool_choice: None,
        provider_options: Some(vec![llm::Kv {
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

    let system_event = llm::Event::Message(llm::Message {
        role: llm::Role::System,
        name: None,
        content: vec![llm::ContentPart::Text(system_message.to_string())],
    });

    let user_message = format!(
        r#"
           We have a list of order items: {current_items_string}.
           Can you do {RECOMMENDATION_PRODUCT_COUNT} recommendations for products items to buy based on previous order items.
           Can you do {RECOMMENDATION_BRAND_COUNT} recommendations for product brands to buy based on previous order items.
           Return the list of product_id-s and list of product_brand-s as a valid JSON object. Return JSON only.
        "#
    );

    let user_event = llm::Event::Message(llm::Message {
        role: llm::Role::User,
        name: None,
        content: vec![llm::ContentPart::Text(user_message.to_string())],
    });

    let llm_response = llm::send(&[system_event, user_event], &config);

    match llm_response {
        Ok(response) => {
            let response_content = response
                .content
                .iter()
                .filter_map(|part| match part {
                    llm::ContentPart::Text(text) => Some(text.clone()),
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
                println!("LLM recommendations - response: {}, error: {}", json_str, e);
                e.to_string()
            })
        }
        Err(e) => {
            println!("LLM recommendations - error: {}", e);
            Err(e.to_string())
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

#[derive(Schema, Clone)]
pub struct RecommendedItems {
    pub product_ids: Vec<String>,
    pub product_brands: Vec<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[agent_definition]
trait ShoppingAssistantAgent {
    fn new(id: String) -> Self;

    fn get_recommended_items(&self) -> RecommendedItems;

    async fn recommend_items(&mut self) -> bool;
}

struct ShoppingAssistantAgentImpl {
    _id: String,
    recommended_items: RecommendedItems,
}

#[agent_implementation]
impl ShoppingAssistantAgent for ShoppingAssistantAgentImpl {
    fn new(id: String) -> Self {
        ShoppingAssistantAgentImpl {
            _id: id,
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
        let recommended_items = get_llm_recommendations(order_items).await;

        match recommended_items {
            Ok(recommended_items) => {
                println!(
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
                println!("Recommended items - error: {}", e);
                false
            }
        }
    }
}
