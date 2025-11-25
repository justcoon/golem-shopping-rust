use crate::common::Datetime;
use golem_rust::{agent_definition, agent_implementation, Schema};

#[derive(Schema, Clone)]
pub struct Product {
    pub product_id: String,
    pub name: String,
    pub brand: String,
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: Datetime,
    pub updated_at: Datetime,
}

#[agent_definition]
trait ProductAgent {
    fn new(init: ProductAgentId) -> Self;

    fn get_product(&self) -> Option<Product>;

    fn initialize_product(
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

    fn get_product(&self) -> Option<Product> {
        self.state.clone()
    }

    fn initialize_product(
        &mut self,
        name: String,
        brand: String,
        description: String,
        tags: Vec<String>,
    ) {
        let now = Datetime::now();
        self.state = Some(Product {
            product_id: self._id.id.clone(),
            name,
            brand,
            description,
            tags,
            created_at: now,
            updated_at: now,
        });
    }
}

#[derive(Schema)]
pub struct ProductAgentId {
    id: String,
}

impl ProductAgentId {
    pub fn new(id: String) -> Self {
        ProductAgentId { id }
    }
}
