use crate::product::{Product, ProductAgentClient};
use futures::future::join_all;
use golem_rust::bindings::golem::api::host::{
    resolve_component_id, AgentAllFilter, AgentAnyFilter, AgentNameFilter, AgentPropertyFilter,
    GetAgents, StringFilterComparator,
};
use golem_rust::{agent_definition, agent_implementation};
use regex::Regex;
use std::collections::HashSet;
use golem_rust::golem_wasm::ComponentId;

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
                value: "product-agent(".to_string(),
            })],
        }],
    }
}

fn get_product_agent_id(agent_name: &str) -> Option<String> {
    Regex::new(r#"product-agent\("([^)]+)"\)"#)
        .ok()?
        .captures(agent_name)
        .filter(|caps| caps.len() > 0)
        .map(|caps| caps[1].to_string())
}

async fn get_products(
    agent_ids: HashSet<String>,
    matcher: ProductQueryMatcher,
) -> Result<Vec<Product>, String> {
    let clients: Vec<ProductAgentClient> = agent_ids
        .into_iter()
        .map(|agent_id| ProductAgentClient::get(agent_id.to_string()))
        .collect();

    let tasks: Vec<_> = clients.iter().map(|client| client.get_product()).collect();

    let responses = join_all(tasks).await;

    let result: Vec<Product> = responses
        .into_iter()
        .flatten()
        .filter(|p| matcher.matches(p.clone()))
        .collect();

    Ok(result)
}

#[agent_definition(mode = "ephemeral")]
trait ProductSearchAgent {
    fn new() -> Self;

    async fn search(&self, query: String) -> Result<Vec<Product>, String>;
}

struct ProductSearchAgentImpl {
    component_id: Option<ComponentId>,
}

#[agent_implementation]
impl ProductSearchAgent for ProductSearchAgentImpl {
    fn new() -> Self {
        let component_id = resolve_component_id("shopping-rust:shopping");
        ProductSearchAgentImpl { component_id }
    }

    async fn search(&self, query: String) -> Result<Vec<Product>, String> {
        if let Some(component_id) = self.component_id {
            println!("searching for products - query: {}", query);

            let mut values: Vec<Product> = Vec::new();
            let matcher = ProductQueryMatcher::new(&query);

            let filter = get_agent_filter();

            let get_agents = GetAgents::new(component_id, Some(&filter), false);

            let mut processed_agent_ids: HashSet<String> = HashSet::new();

            while let Some(agents) = get_agents.get_next() {
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
        } else {
            Err("Component not found".to_string())
        }
    }
}
