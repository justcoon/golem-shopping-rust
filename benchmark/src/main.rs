mod data;
mod domain;
mod goose_ext;

use goose::prelude::*;
use goose_ext::GooseRequestExt;
use goose_ext::GooseResponseExt;
use rand::seq::SliceRandom;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    let custom_host = match std::env::var("HOST") {
        Ok(host) => host,
        Err(_) => "".to_string(),
    };

    GooseAttack::initialize()?
        .register_scenario(
            scenario!("Get Products")
                .set_wait_time(Duration::from_secs(5), Duration::from_secs(15))?
                .register_transaction(transaction!(get_products)),
        )
        .register_scenario(
            scenario!("Search Products By Brand")
                .set_wait_time(Duration::from_secs(5), Duration::from_secs(15))?
                .register_transaction(transaction!(search_products_by_brand)),
        )
        .register_scenario(
            scenario!("Get Pricing")
                .set_wait_time(Duration::from_secs(5), Duration::from_secs(15))?
                .register_transaction(transaction!(get_pricing)),
        )
        .register_scenario(
            scenario!("Create, checkout Cart and get Order")
                .set_wait_time(Duration::from_secs(5), Duration::from_secs(15))?
                .register_transaction(transaction!(create_and_checkout_cart)),
        )
        .set_default(GooseDefault::Host, custom_host.as_str())?
        .execute()
        .await?;

    Ok(())
}

async fn get_products(user: &mut GooseUser) -> TransactionResult {
    let product_id = data::rand_product_id();

    let _response =
        user.get_request("product-get", format!("/v1/product/{product_id}").as_str()).await?;

    Ok(())
}

async fn search_products_by_brand(user: &mut GooseUser) -> TransactionResult {
    let brand = data::rand_product_brand();

    let _response = user
        .get_request("product-search-by-brand", format!("/v1/product/search?query=brand:\"{brand}\"").as_str())
        .await?;

    Ok(())
}

async fn get_pricing(user: &mut GooseUser) -> TransactionResult {
    let product_id = data::rand_product_id();

    let _response =
        user.get_request("pricing-get", format!("/v1/pricing/{product_id}").as_str()).await?;

    Ok(())
}

async fn create_and_checkout_cart(user: &mut GooseUser) -> TransactionResult {
    let item_count = 4;

    let product_ids = data::get_product_ids();
    let product_ids: Vec<_> =
        product_ids.choose_multiple(&mut rand::thread_rng(), item_count).collect();

    let user_id = data::get_user_ids().choose(&mut rand::thread_rng()).unwrap().to_string();

    let address = data::get_addresses().choose(&mut rand::thread_rng()).unwrap().to_owned();

    let set_email = domain::common::SetEmail::new(data::get_email(user_id.clone()));

    for product_id in product_ids.iter() {
        let _response = user
            .put_request(
                "cart-add-item",
                format!("/v1/cart/{user_id}/items/{product_id}").as_str(),
                &domain::common::AddItem::new(1),
            )
            .await?;
    }

    let product_id = product_ids[0];

    let _response = user
        .delete_request(
            "cart-delete-item",
            format!("/v1/cart/{user_id}/items/{product_id}").as_str(),
        )
        .await?;

    let _response = user
        .put_request("cart-set-email", format!("/v1/cart/{user_id}/email").as_str(), &set_email)
        .await?;

    let _response = user
        .put_request(
            "cart-set-billing-address",
            format!("/v1/cart/{user_id}/billing-address").as_str(),
            &address,
        )
        .await?;

    let _response = user.get_request("cart-get", format!("/v1/cart/{user_id}").as_str()).await?;

    let response = user
        .post_request(
            "cart-checkout",
            format!("/v1/cart/{user_id}/checkout").as_str(),
            &serde_json::Value::Null,
        )
        .await?;

    // let order_created: domain::cart::OrderCreated = response.json().await?;
    let ok_order_created: domain::cart::OkOrderCreated = response.json().await?;
    let order_created: domain::cart::OrderCreated = ok_order_created.ok;

    let order_id = order_created.order_id;

    let _response = user.get_request("order-get", format!("/v1/order/{order_id}").as_str()).await?;

    Ok(())
}
