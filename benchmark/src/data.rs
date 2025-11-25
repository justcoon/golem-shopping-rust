use crate::domain;
use rand::prelude::SliceRandom;
use rand::Rng;

pub fn rand_product_id() -> String {
    let product_ids = get_product_ids();

    product_ids.choose(&mut rand::thread_rng()).unwrap().to_string()
}

pub fn rand_product_brand() -> String {
    let product_brands = get_product_brands();

    product_brands.choose(&mut rand::thread_rng()).unwrap().to_string()
}

pub fn get_product_brands() -> Vec<String> {
    vec!["Brand A".to_string(), "Brand B".to_string(), "Brand C".to_string(), "Brand D".to_string()]
}

pub fn get_product_ids() -> Vec<String> {
    (1..=50).map(|v| format!("p{:03}", v)).collect()
}

pub fn get_user_ids() -> Vec<String> {
    (1..=10).map(|v| format!("user{:03}", v)).collect()
}

pub fn get_email(user: String) -> String {
    let num = rand::thread_rng().gen_range(0..100);
    format!("{}-{:03}@test.com", user, num)
}

pub fn get_addresses() -> Vec<domain::common::Address> {
    let mut addresses = Vec::new();

    addresses.push(domain::common::Address {
        street: "123 Main St".to_string(),
        state_or_region: "CA".to_string(),
        phone_number: Some("555-555-5555".to_string()),
        postal_code: "12345".to_string(),
        name: Some("John Doe".to_string()),
        city: "San Francisco".to_string(),
        country: "USA".to_string(),
    });

    addresses.push(domain::common::Address {
        street: "123 Main St".to_string(),
        state_or_region: "Washington".to_string(),
        phone_number: Some("555-555-1234".to_string()),
        postal_code: "23456".to_string(),
        name: Some("John Doe".to_string()),
        city: "Washington DC".to_string(),
        country: "USA".to_string(),
    });

    addresses.push(domain::common::Address {
        street: "123 Main St".to_string(),
        state_or_region: "NY".to_string(),
        phone_number: Some("555-555-3456".to_string()),
        postal_code: "3456".to_string(),
        name: None,
        city: "New York".to_string(),
        country: "USA".to_string(),
    });

    addresses
}
