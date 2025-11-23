use golem_rust::Schema;

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