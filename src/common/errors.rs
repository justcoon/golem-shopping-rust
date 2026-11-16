use golem_rust::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ItemNotFoundError {
    pub message: String,
    pub product_id: String,
}

impl ItemNotFoundError {
    pub fn new(product_id: String) -> Self {
        Self {
            message: "Item not found".to_string(),
            product_id,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct PricingNotFoundError {
    pub message: String,
    pub product_id: String,
}

impl PricingNotFoundError {
    pub fn new(product_id: String) -> Self {
        Self {
            message: "Pricing not found".to_string(),
            product_id,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ProductNotFoundError {
    pub message: String,
    pub product_id: String,
}

impl ProductNotFoundError {
    pub fn new(product_id: String) -> Self {
        Self {
            message: "Product not found".to_string(),
            product_id,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct EmailNotValidError {
    pub message: String,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct EmptyItemsError {
    pub message: String,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct AddressNotValidError {
    pub message: String,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BillingAddressNotSetError {
    pub message: String,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct EmptyEmailError {
    pub message: String,
}
