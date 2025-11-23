use crate::product::Product;
use golem_rust::{agent_definition, agent_implementation, Schema};

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
    pub fn matches(&self, product: Product) -> bool {
        fn text_matches(text: &str, query: &str) -> bool {
            query == "*" || text.to_lowercase().contains(&query.to_lowercase())
        }

        // Check field filters first
        for (field, value) in self.field_filters.iter() {
            let matches = match field.to_lowercase().as_str() {
                "name" => text_matches(&product.name, &value),
                "brand" => text_matches(&product.brand, &value),
                "description" => text_matches(&product.description, &value),
                "tag" | "tags" => product.tags.iter().any(|tag| text_matches(tag, &value)),
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
            let matches = text_matches(&product.name, &term)
                || text_matches(&product.brand, &term)
                || text_matches(&product.description, &term)
                || product.tags.iter().any(|tag| text_matches(tag, &term));

            if !matches {
                return false;
            }
        }

        true
    }
}

#[agent_definition(mode = "ephemeral")]
trait ProductSearch {
    fn new(init: ProductSearchId) -> Self;
    async fn search(&mut self, query: String) -> Result<Vec<Product>, String>;
}

struct ProductSearchImpl {
    _id: ProductSearchId,
}

#[agent_implementation]
impl ProductSearch for ProductSearchImpl {
    fn new(id: ProductSearchId) -> Self {
        ProductSearchImpl { _id: id }
    }

    async fn search(&mut self, query: String) -> Result<Vec<Product>, String> {
        todo!()
    }
}

#[derive(Schema)]
struct ProductSearchId {
    id: String,
}
