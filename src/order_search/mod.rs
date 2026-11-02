use crate::order::{Order, OrderAgentClient, OrderStatus};
use futures::future::join_all;
use golem_rust::{
    AgentAllFilter, AgentAnyFilter, AgentNameFilter, AgentPropertyFilter, ComponentId, GetAgents,
    StringFilterComparator, agent_definition, agent_implementation, endpoint, resolve_component_id,
};
use log::info;
use regex::Regex;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct OrderQueryMatcher {
    terms: Vec<String>,
    field_filters: Vec<(String, String)>,
}

impl OrderQueryMatcher {
    pub fn new(query: &str) -> Self {
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

    pub fn tokenize(query: &str) -> Vec<String> {
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

    pub fn matches(&self, order: &Order) -> bool {
        fn text_matches(text: &str, query: &str) -> bool {
            query == "*" || text.to_lowercase().contains(&query.to_lowercase())
        }

        fn text_exact_matches(text: &str, query: &str) -> bool {
            query == "*" || text.eq_ignore_ascii_case(query)
        }

        // 1. Check field filters
        for (field, value) in &self.field_filters {
            let matches = match field.to_lowercase().as_str() {
                "order-id" | "orderid" | "id" => text_exact_matches(&order.order_id, value),
                "user-id" | "userid" => text_exact_matches(&order.user_id, value),
                "status" | "orderstatus" => {
                    let status_str = match order.order_status {
                        OrderStatus::New => "new",
                        OrderStatus::Shipped => "shipped",
                        OrderStatus::Cancelled => "cancelled",
                    };
                    text_exact_matches(status_str, value)
                }
                "email" => order
                    .email
                    .as_ref()
                    .map(|e| text_matches(e, value))
                    .unwrap_or(false),
                "product-id" | "productid" => order
                    .items
                    .iter()
                    .any(|item| text_exact_matches(&item.product_id, value)),
                _ => true, // Unknown field: ignore or match
            };

            if !matches {
                return false;
            }
        }

        // 2. Check general search terms
        if self.terms.is_empty() {
            return true;
        }

        for term in &self.terms {
            let status_str = match order.order_status {
                OrderStatus::New => "new",
                OrderStatus::Shipped => "shipped",
                OrderStatus::Cancelled => "cancelled",
            };

            let matches = text_matches(&order.order_id, term)
                || text_matches(&order.user_id, term)
                || text_matches(status_str, term)
                || order
                    .email
                    .as_ref()
                    .map(|e| text_matches(e, term))
                    .unwrap_or(false)
                || order
                    .shipping_address
                    .as_ref()
                    .and_then(|a| a.name.as_ref())
                    .map(|n| text_matches(n, term))
                    .unwrap_or(false)
                || order
                    .billing_address
                    .as_ref()
                    .and_then(|a| a.name.as_ref())
                    .map(|n| text_matches(n, term))
                    .unwrap_or(false)
                || order.items.iter().any(|item| {
                    text_matches(&item.product_name, term)
                        || text_matches(&item.product_brand, term)
                        || text_matches(&item.product_id, term)
                });

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
                value: "OrderAgent(".to_string(),
            })],
        }],
    }
}

fn get_order_agent_id(agent_name: &str) -> Option<String> {
    Regex::new(r#"OrderAgent\("([^)]+)"\)"#)
        .ok()?
        .captures(agent_name)
        .filter(|caps| caps.len() > 0)
        .map(|caps| caps[1].to_string())
}

async fn fetch_orders(agent_ids: HashSet<String>) -> Result<Vec<Order>, String> {
    let clients: Vec<OrderAgentClient> = agent_ids.into_iter().map(OrderAgentClient::get).collect();

    let tasks: Vec<_> = clients.iter().map(|client| client.get_order()).collect();

    let responses = join_all(tasks).await;

    let result: Vec<Order> = responses.into_iter().flatten().collect();

    Ok(result)
}

fn sort_orders_desc(orders: &mut [Order]) {
    orders.sort_by_key(|a| std::cmp::Reverse(a.created_at));
}

#[agent_definition(ephemeral, mount = "/v1/order-search")]
trait OrderSearchAgent {
    fn new() -> Self;

    #[endpoint(get = "/search?query={query}&status={status}&userId={user_id}&limit={limit}")]
    async fn search_orders(
        &self,
        query: Option<String>,
        status: Option<OrderStatus>,
        user_id: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String>;

    #[endpoint(get = "/user/{user_id}?limit={limit}")]
    async fn get_orders_by_user_id(
        &self,
        user_id: String,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String>;

    #[endpoint(get = "/status/{status}?limit={limit}")]
    async fn get_orders_by_status(
        &self,
        status: OrderStatus,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String>;

    #[endpoint(get = "/recent?limit={limit}")]
    async fn get_recent_orders(&self, limit: Option<u32>) -> Result<Vec<Order>, String>;
}

struct OrderSearchAgentImpl {
    component_id: Option<ComponentId>,
}

impl OrderSearchAgentImpl {
    async fn get_all_orders(&self) -> Result<Vec<Order>, String> {
        if let Some(component_id) = self.component_id.clone() {
            let filter = get_agent_filter();
            let get_agents = GetAgents::new(component_id, Some(&filter), false);
            let mut processed_agent_ids: HashSet<String> = HashSet::new();
            let mut all_orders: Vec<Order> = Vec::new();

            while let Some(agents) = get_agents.get_next().map_err(|e| e.to_string())? {
                let agent_ids = agents
                    .iter()
                    .filter_map(|a| get_order_agent_id(a.agent_id.agent_id.as_str()))
                    .filter(|n| !processed_agent_ids.contains(n))
                    .collect::<HashSet<_>>();

                if !agent_ids.is_empty() {
                    let orders = fetch_orders(agent_ids.clone()).await?;
                    processed_agent_ids.extend(agent_ids);
                    all_orders.extend(orders);
                }
            }

            Ok(all_orders)
        } else {
            Err("Component not found".to_string())
        }
    }
}

#[agent_implementation]
impl OrderSearchAgent for OrderSearchAgentImpl {
    fn new() -> Self {
        let component_id = resolve_component_id("shopping-rust:shopping");
        OrderSearchAgentImpl { component_id }
    }

    async fn search_orders(
        &self,
        query: Option<String>,
        status: Option<OrderStatus>,
        user_id: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String> {
        info!(
            "searching for orders - query: {:?}, status: {:?}, user_id: {:?}, limit: {:?}",
            query, status, user_id, limit
        );

        let all_orders = self.get_all_orders().await?;
        let matcher = query.as_deref().map(OrderQueryMatcher::new);

        let mut matched: Vec<Order> = all_orders
            .into_iter()
            .filter(|order| {
                if let Some(s) = status
                    && order.order_status != s
                {
                    return false;
                }
                if let Some(ref uid) = user_id
                    && &order.user_id != uid
                {
                    return false;
                }
                if let Some(ref m) = matcher
                    && !m.matches(order)
                {
                    return false;
                }
                true
            })
            .collect();

        sort_orders_desc(&mut matched);
        let max = limit.unwrap_or(20) as usize;
        matched.truncate(max);
        Ok(matched)
    }

    async fn get_orders_by_user_id(
        &self,
        user_id: String,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String> {
        info!("getting orders for user: {}, limit: {:?}", user_id, limit);

        let all_orders = self.get_all_orders().await?;
        let mut matched: Vec<Order> = all_orders
            .into_iter()
            .filter(|order| order.user_id == user_id)
            .collect();

        sort_orders_desc(&mut matched);
        let max = limit.unwrap_or(20) as usize;
        matched.truncate(max);
        Ok(matched)
    }

    async fn get_orders_by_status(
        &self,
        status: OrderStatus,
        limit: Option<u32>,
    ) -> Result<Vec<Order>, String> {
        info!(
            "getting orders with status: {:?}, limit: {:?}",
            status, limit
        );

        let all_orders = self.get_all_orders().await?;
        let mut matched: Vec<Order> = all_orders
            .into_iter()
            .filter(|order| order.order_status == status)
            .collect();

        sort_orders_desc(&mut matched);
        let max = limit.unwrap_or(20) as usize;
        matched.truncate(max);
        Ok(matched)
    }

    async fn get_recent_orders(&self, limit: Option<u32>) -> Result<Vec<Order>, String> {
        info!("getting recent orders - limit: {:?}", limit);

        let mut all_orders = self.get_all_orders().await?;
        sort_orders_desc(&mut all_orders);
        let max = limit.unwrap_or(20) as usize;
        all_orders.truncate(max);
        Ok(all_orders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::OrderItem;

    fn sample_order(
        order_id: &str,
        user_id: &str,
        status: OrderStatus,
        email: Option<&str>,
    ) -> Order {
        let now = chrono::Utc::now();
        Order {
            order_id: order_id.to_string(),
            user_id: user_id.to_string(),
            email: email.map(|e| e.to_string()),
            order_status: status,
            items: vec![OrderItem {
                product_id: "prod-1".to_string(),
                product_name: "Mechanical Keyboard".to_string(),
                product_brand: "Keychron".to_string(),
                price: 99.0,
                quantity: 1,
            }],
            billing_address: None,
            shipping_address: None,
            total: 99.0,
            currency: "USD".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_extract_order_agent_id() {
        assert_eq!(
            get_order_agent_id(r#"OrderAgent("order-123")"#),
            Some("order-123".to_string())
        );
        assert_eq!(
            get_order_agent_id(r#"OrderAgent("uuid-abc-def")"#),
            Some("uuid-abc-def".to_string())
        );
        assert_eq!(get_order_agent_id(r#"ProductAgent("p1")"#), None);
    }

    #[test]
    fn test_query_matcher_terms() {
        let order = sample_order(
            "ord-1",
            "user-42",
            OrderStatus::New,
            Some("test@example.com"),
        );
        let matcher = OrderQueryMatcher::new("keyboard");
        assert!(matcher.matches(&order));

        let non_matching = OrderQueryMatcher::new("monitor");
        assert!(!non_matching.matches(&order));
    }

    #[test]
    fn test_query_matcher_field_filters() {
        let order = sample_order(
            "ord-1",
            "user-42",
            OrderStatus::Shipped,
            Some("alice@example.com"),
        );
        let matcher_user = OrderQueryMatcher::new("userid:user-42");
        assert!(matcher_user.matches(&order));

        let matcher_wrong_user = OrderQueryMatcher::new("userid:user-99");
        assert!(!matcher_wrong_user.matches(&order));

        let matcher_status = OrderQueryMatcher::new("status:shipped");
        assert!(matcher_status.matches(&order));

        let matcher_email = OrderQueryMatcher::new("email:alice");
        assert!(matcher_email.matches(&order));

        let matcher_product = OrderQueryMatcher::new("productid:prod-1");
        assert!(matcher_product.matches(&order));
    }

    #[test]
    fn test_query_matcher_quoted_terms() {
        let order = sample_order("ord-1", "user-42", OrderStatus::New, None);
        let matcher = OrderQueryMatcher::new(r#""Mechanical Keyboard""#);
        assert!(matcher.matches(&order));
    }
}
