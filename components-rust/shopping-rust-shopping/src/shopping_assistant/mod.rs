use crate::cart::{CartAgentClient, CartAgentId};
use crate::order::{OrderAgentClient, OrderAgentId, OrderItem};
use futures::future::join_all;
use golem_rust::{agent_definition, agent_implementation, Schema};
use std::collections::HashMap;
use crate::common::Datetime;

async fn get_order_items(id: String) -> Vec<OrderItem> {
    let cart = CartAgentClient::get(CartAgentId::new(id)).get_cart().await;

    if let Some(cart) = cart {
        let order_ids = cart.previous_order_ids;

        let clients: Vec<_> = order_ids
            .into_iter()
            .map(|order_id| OrderAgentClient::get(OrderAgentId::new(order_id.clone())))
            .collect();

        let tasks: Vec<_> = clients.iter().map(|client| client.get_order()).collect();

        let orders = join_all(tasks).await;

        let items = orders
            .into_iter()
            .filter_map(|o| o)
            .flat_map(|o| o.items)
            .collect();

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

    let mut result: Vec<_> = items_map.values().map(|c| c.clone()).collect();

    result.sort_by_key(|v| v.quantity);

    result.into_iter().take(100).collect()
}

async fn get_llm_recommendations(items: Vec<OrderItem>) -> Option<RecommendedItems> {
    println!("LLM recommendations - items: {}", items.len());

    None
}

#[derive(Schema, Clone)]
pub struct RecommendedItems {
    pub product_ids: Vec<String>,
    pub product_brands: Vec<String>,
    pub updated_at: Datetime,
}

#[agent_definition]
trait ShoppingAssistantAgent {
    fn new(init: ShoppingAssistantAgentId) -> Self;

    fn get_recommended_items(&self) -> RecommendedItems;

    async fn recommend_items(&mut self) -> bool;
}

struct ShoppingAssistantAgentImpl {
    _id: ShoppingAssistantAgentId,
    recommended_items: RecommendedItems,
}

#[agent_implementation]
impl ShoppingAssistantAgent for ShoppingAssistantAgentImpl {
    fn new(id: ShoppingAssistantAgentId) -> Self {
        ShoppingAssistantAgentImpl {
            _id: id,
            recommended_items: RecommendedItems {
                product_ids: Vec::new(),
                product_brands: Vec::new(),
                updated_at: Datetime::now()
            },
        }
    }

    fn get_recommended_items(&self) -> RecommendedItems {
        self.recommended_items.clone()
    }

    async fn recommend_items(&mut self) -> bool {
        let order_items = get_order_items(self._id.id.clone()).await;
        let recommended_items = get_llm_recommendations(order_items).await;
        if let Some(recommended_items) = recommended_items {
            self.recommended_items = recommended_items;
            true
        } else {
            false
        }
    }
}

#[derive(Schema)]
pub struct ShoppingAssistantAgentId {
    id: String,
}

impl ShoppingAssistantAgentId {
    pub fn new(id: String) -> Self {
        ShoppingAssistantAgentId { id }
    }
}
