use golem_rust::{agent_definition, agent_implementation, Schema};

#[derive(Schema, Clone)]
pub struct Product {
    pub product_id: String,
    pub name: String,
    pub brand: String,
    pub description: String,
    pub tags: Vec<String>,
    // pub created_at: Datetime, //chrono::DateTime<chrono::Utc>,
    // pub updated_at: Datetime, // chrono::DateTime<chrono::Utc>,
}

#[agent_definition]
trait ProductAgent {
    fn new(init: ProductAgentId) -> Self;
    async fn get_product(&self) -> Option<Product>;

    async fn initialize_product(
        &mut self,
        name: String,
        brand: String,
        description: String,
        tags: Vec<String>,
    );
}

struct ProductAgentImpl {
    _id: ProductAgentId,
    state: Option<Product>,
}

#[agent_implementation]
impl ProductAgent for ProductAgentImpl {
    fn new(id: ProductAgentId) -> Self {
        ProductAgentImpl {
            _id: id,
            state: None,
        }
    }

    async fn get_product(&self) -> Option<Product> {
        self.state.clone()
    }

    async fn initialize_product(
        &mut self,
        name: String,
        brand: String,
        description: String,
        tags: Vec<String>,
    ) {
        self.state = Some(Product {
            product_id: self._id.id.clone(),
            name,
            brand,
            description,
            tags,
        });
    }
}

#[derive(Schema)]
struct ProductAgentId {
    id: String,
}
