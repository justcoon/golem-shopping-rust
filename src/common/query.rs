/// Tokenizes a query string, properly handling quoted phrases and whitespace.
pub fn tokenize_query(query: &str) -> Vec<String> {
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

/// A parsed query containing general search terms and key-value field filters (`field:value`).
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedQuery {
    pub terms: Vec<String>,
    pub field_filters: Vec<(String, String)>,
}

impl ParsedQuery {
    pub fn parse(query: &str) -> Self {
        let mut terms = Vec::new();
        let mut field_filters = Vec::new();

        for part in tokenize_query(query) {
            if let Some((field, value)) = part.split_once(':') {
                field_filters.push((field.to_string(), value.to_string()));
            } else {
                terms.push(part);
            }
        }

        Self {
            terms,
            field_filters,
        }
    }
}

/// Case-insensitive substring match helper, treating `*` as matching anything.
pub fn text_matches(text: &str, query: &str) -> bool {
    query == "*" || text.to_lowercase().contains(&query.to_lowercase())
}

/// Case-insensitive exact match helper, treating `*` as matching anything.
pub fn text_exact_matches(text: &str, query: &str) -> bool {
    query == "*" || text.eq_ignore_ascii_case(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_query_basic_and_quotes() {
        let tokens = tokenize_query(r#"shoes "running shoes" brand:nike"#);
        assert_eq!(tokens, vec!["shoes", "running shoes", "brand:nike"]);
    }

    #[test]
    fn test_parsed_query() {
        let parsed = ParsedQuery::parse(r#"laptop brand:apple "pro max" status:new"#);
        assert_eq!(parsed.terms, vec!["laptop", "pro max"]);
        assert_eq!(
            parsed.field_filters,
            vec![
                ("brand".to_string(), "apple".to_string()),
                ("status".to_string(), "new".to_string())
            ]
        );
    }
}
