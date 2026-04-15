use golem_rust::{Schema, agent_definition, agent_implementation};
use serde::{Deserialize, Serialize};

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Product {
    pub product_id: String,
    pub name: String,
    pub brand: String,
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[agent_definition]
trait ProductAgent {
    fn new(id: String) -> Self;

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
    _id: String,
    state: Option<Product>,
}

#[agent_implementation]
impl ProductAgent for ProductAgentImpl {
    fn new(id: String) -> Self {
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
        let now = chrono::Utc::now();
        self.state = Some(Product {
            product_id: self._id.clone(),
            name,
            brand,
            description,
            tags,
            created_at: now,
            updated_at: now,
        });
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: Option<Product> = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}
