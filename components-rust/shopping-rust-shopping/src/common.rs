use golem_rust::Schema;

pub const CURRENCY_DEFAULT: &str = "USD";
pub const PRICING_ZONE_DEFAULT: &str = "global";


#[derive(Schema, Clone)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state_or_region: String,
    pub country: String,
    pub postal_code: String,
    pub name: Option<String>,
    pub phone_number: Option<String>,
}
