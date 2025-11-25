use golem_rust::value_and_type::{
    FromValueAndType, IntoValue, NodeBuilder, TypeNodeBuilder, WitValueExtractor,
};
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

#[derive(Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Datetime(chrono::DateTime<chrono::Utc>);

impl Datetime {
    pub fn now() -> Self {
        Self(chrono::Utc::now())
    }
}

impl IntoValue for Datetime {
    fn add_to_builder<T: NodeBuilder>(self, builder: T) -> T::Result {
        builder.string(self.0.to_string().as_str())
    }

    fn add_to_type_builder<T: TypeNodeBuilder>(builder: T) -> T::Result {
        builder.string()
    }
}

impl FromValueAndType for Datetime {
    fn from_extractor<'a, 'b>(
        extractor: &'a impl WitValueExtractor<'a, 'b>,
    ) -> Result<Self, String> {
        extractor
            .string()
            .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
            .map(|d| Datetime(d))
            .ok_or_else(|| "Expected datetime string".to_string())
    }
}

// #[derive(Schema, Clone)]
// pub struct Test {
//     pub id2: uuid::Uuid,
//     pub date: chrono::DateTime<chrono::Utc>
// }
