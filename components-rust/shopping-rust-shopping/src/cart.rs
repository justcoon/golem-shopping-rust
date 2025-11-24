use crate::common::{Address, CURRENCY_DEFAULT, PRICING_ZONE_DEFAULT};
use crate::order::{CreateOrder, OrderAgentClient, OrderAgentId, OrderItem};
use crate::pricing::{PricingAgentClient, PricingAgentId, PricingItem};
use crate::product::{Product, ProductAgentClient, ProductAgentId};
use crate::shopping_assistant::{ShoppingAssistantAgentClient, ShoppingAssistantAgentId};
use email_address::EmailAddress;
use golem_rust::{agent_definition, agent_implementation, Schema};
use std::future::Future;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Schema, Clone)]
pub struct Cart {
    pub user_id: String,
    pub email: Option<String>,
    pub items: Vec<CartItem>,
    pub billing_address: Option<Address>,
    pub shipping_address: Option<Address>,
    pub total: f32,
    pub currency: String,
    pub previous_order_ids: Vec<String>,
    // pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Cart {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            email: None,
            items: vec![],
            billing_address: None,
            shipping_address: None,
            total: 0.0,
            currency: CURRENCY_DEFAULT.to_string(),
            // updated_at: chrono::Utc::now(),
            previous_order_ids: vec![],
        }
    }

    pub fn order_created(&mut self, order_id: String) {
        self.clear();
        self.previous_order_ids.push(order_id);
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.billing_address = None;
        self.shipping_address = None;
        self.total = 0.0;
        // self.updated_at = chrono::Utc::now();
    }

    pub fn recalculate_total(&mut self) {
        self.total = get_total_price(self.items.clone());
        // self.updated_at = chrono::Utc::now();
    }

    pub fn add_item(&mut self, item: CartItem) -> bool {
        self.items.push(item);
        self.recalculate_total();
        true
    }

    pub fn set_items(&mut self, items: Vec<CartItem>) {
        self.items = items;
        self.recalculate_total();
    }

    pub fn set_billing_address(&mut self, address: Address) {
        self.billing_address = Some(address);
        // self.updated_at = chrono::Utc::now();
    }

    pub fn set_shipping_address(&mut self, address: Address) {
        self.shipping_address = Some(address);
        // self.updated_at = chrono::Utc::now();
    }

    pub fn set_email(&mut self, email: String) {
        self.email = Some(email);
        // self.updated_at = chrono::Utc::now();
    }

    pub fn update_item_quantity(&mut self, product_id: String, quantity: u32) -> bool {
        let mut updated = false;

        for item in &mut self.items {
            if item.product_id == product_id {
                item.quantity = quantity;
                updated = true;
            }
        }

        if updated {
            self.recalculate_total();
        }

        updated
    }

    pub fn remove_item(&mut self, product_id: String) -> bool {
        let exist = self.items.iter().any(|item| item.product_id == product_id);

        if exist {
            self.items.retain(|item| item.product_id != product_id);
            self.recalculate_total();
        }

        exist
    }
}

#[derive(Schema, Clone)]
pub struct CartItem {
    pub product_id: String,
    pub product_name: String,
    pub product_brand: String,
    pub price: f32,
    pub quantity: u32,
}

impl From<CartItem> for OrderItem {
    fn from(value: CartItem) -> Self {
        Self {
            product_id: value.product_id,
            quantity: value.quantity,
            price: value.price,
            product_name: value.product_name,
            product_brand: value.product_brand,
        }
    }
}

impl From<Cart> for CreateOrder {
    fn from(value: Cart) -> Self {
        Self {
            user_id: value.user_id,
            email: value.email,
            items: value.items.into_iter().map(|item| item.into()).collect(),
            total: value.total,
            currency: value.currency,
            shipping_address: value.shipping_address.map(|a| a.into()),
            billing_address: value.billing_address.map(|a| a.into()),
        }
    }
}

#[derive(Schema, Clone)]
pub struct ItemNotFoundError {
    pub message: String,
    pub product_id: String,
}
#[derive(Schema, Clone)]
pub struct PricingNotFoundError {
    pub message: String,
    pub product_id: String,
}
#[derive(Schema, Clone)]
pub struct ProductNotFoundError {
    pub message: String,
    pub product_id: String,
}
#[derive(Schema, Clone)]
pub struct EmailNotValidError {
    pub message: String,
}
#[derive(Schema, Clone)]
pub struct EmptyItemsError {
    pub message: String,
}
#[derive(Schema, Clone)]
pub struct AddressNotValidError {
    pub message: String,
}
#[derive(Schema, Clone)]
pub struct BillingAddressNotSetError {
    pub message: String,
}
#[derive(Schema, Clone)]
pub struct EmptyEmailError {
    pub message: String,
}
#[derive(Schema, Clone)]
pub struct OrderCreateError {
    pub message: String,
}
#[derive(Schema, Clone)]
pub enum AddItemError {
    ProductNotFound(ProductNotFoundError),
    PricingNotFound(PricingNotFoundError),
}
#[derive(Schema, Clone)]
pub enum RemoveItemError {
    ItemNotFound(ItemNotFoundError),
}
#[derive(Schema, Clone)]
pub enum ShipOrderError {
    EmptyItems(EmptyItemsError),
    EmptyEmail(EmptyEmailError),
    BillingAddressNotSet(BillingAddressNotSetError),
}
#[derive(Schema, Clone)]
pub enum UpdateEmailError {
    EmailNotValid(EmailNotValidError),
}
#[derive(Schema, Clone)]
pub enum UpdateItemQuantityError {
    ItemNotFound(ItemNotFoundError),
}
#[derive(Schema, Clone)]
pub enum CheckoutError {
    ProductNotFound(ProductNotFoundError),
    PricingNotFound(PricingNotFoundError),
    EmptyItems(EmptyItemsError),
    EmptyEmail(EmptyEmailError),
    BillingAddressNotSet(BillingAddressNotSetError),
    OrderCreate(OrderCreateError),
}
#[derive(Schema, Clone)]
pub enum UpdateAddressError {
    AddressNotValid(AddressNotValidError),
}

#[derive(Schema, Clone)]
pub struct OrderConfirmation {
    pub order_id: String,
}

fn get_total_price(items: Vec<CartItem>) -> f32 {
    let mut total = 0f32;

    for item in items {
        total += item.price * item.quantity as f32;
    }

    total
}

fn generate_order_id() -> String {
    Uuid::new_v4().to_string()
}

fn item_not_found_error(product_id: String) -> ItemNotFoundError {
    ItemNotFoundError {
        message: "Item not found".to_string(),
        product_id,
    }
}

fn pricing_not_found_error(product_id: String) -> PricingNotFoundError {
    PricingNotFoundError {
        message: "Pricing not found".to_string(),
        product_id,
    }
}

fn product_not_found_error(product_id: String) -> ProductNotFoundError {
    ProductNotFoundError {
        message: "Product not found".to_string(),
        product_id,
    }
}
fn get_cart_item(product: Product, pricing: PricingItem, quantity: u32) -> CartItem {
    CartItem {
        product_id: product.product_id,
        product_name: product.name,
        product_brand: product.brand,
        price: pricing.price,
        quantity,
    }
}

fn validate_cart(cart: Cart) -> Result<(), CheckoutError> {
    if cart.items.is_empty() {
        Err(CheckoutError::EmptyItems(EmptyItemsError {
            message: "Empty items".to_string(),
        }))
    } else if cart.billing_address.is_none() {
        Err(CheckoutError::BillingAddressNotSet(
            BillingAddressNotSetError {
                message: "Billing address not set".to_string(),
            },
        ))
    } else if cart.email.is_none() {
        Err(CheckoutError::EmptyEmail(EmptyEmailError {
            message: "Email not set".to_string(),
        }))
    } else {
        Ok(())
    }
}

async fn create_order(order_id: String, cart: Cart) -> Result<String, CheckoutError> {
    println!("Creating order: {}", order_id);

    validate_cart(cart.clone())?;

    let order = cart.into();

    OrderAgentClient::get(OrderAgentId::new(order_id.clone()))
        .initialize_order(order)
        .await
        .map_err(|_| {
            CheckoutError::OrderCreate(OrderCreateError {
                message: "Failed to create order".to_string(),
            })
        })?;

    Ok(order_id)
}

#[agent_definition]
trait CartAgent {
    fn new(init: CartAgentId) -> Self;
    async fn get_cart(&mut self) -> Option<Cart>;
    async fn add_item(&mut self, product_id: String, quantity: u32) -> Result<(), AddItemError>;
    async fn checkout(&mut self) -> Result<OrderConfirmation, CheckoutError>;
    fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError>;
    fn clear(&mut self);
    fn remove_item(&mut self, product_id: String) -> Result<(), RemoveItemError>;
    fn update_billing_address(&mut self, address: Address) -> Result<(), UpdateAddressError>;
    fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<(), UpdateItemQuantityError>;
    fn update_shipping_address(&mut self, address: Address) -> Result<(), UpdateAddressError>;
}

struct CartAgentImpl {
    _id: CartAgentId,
    state: Option<Cart>,
}

impl CartAgentImpl {
    fn get_state(&mut self) -> &mut Cart {
        if self.state.is_none() {
            let value = Cart::new(self._id.id.clone());
            self.state = Some(value);
        }
        self.state.as_mut().unwrap()
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut Cart) -> T) -> T {
        f(self.get_state())
    }

    async fn with_state_async<T, F, Fut>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Cart) -> Fut,
        Fut: Future<Output = T>,
    {
        f(self.get_state()).await
    }
}

#[agent_implementation]
impl CartAgent for CartAgentImpl {
    fn new(id: CartAgentId) -> Self {
        CartAgentImpl {
            _id: id,
            state: None,
        }
    }

    async fn get_cart(&mut self) -> Option<Cart> {
        println!("Getting cart");
        if let Some(cart) = self.state.as_mut() {
            let mut items = Vec::new();
            for item in cart.items.clone() {
                let product_id = item.product_id;
                let quantity = item.quantity;
                let product = ProductAgentClient::get(ProductAgentId::new(product_id.clone()))
                    .get_product()
                    .await;
                let pricing = PricingAgentClient::get(PricingAgentId::new(product_id.clone()))
                    .get_price(cart.currency.clone(), PRICING_ZONE_DEFAULT.to_string())
                    .await;
                match (product, pricing) {
                    (Some(product), Some(pricing)) => {
                        items.push(get_cart_item(product, pricing, quantity));
                    }
                    _ => (),
                }
            }
            cart.set_items(items);
            Some(cart.clone())
        } else {
            None
        }
    }

    async fn add_item(&mut self, product_id: String, quantity: u32) -> Result<(), AddItemError> {
        let state = self.get_state();

        println!(
            "Adding item with product {} to the cart of user {}",
            product_id, state.user_id
        );

        let updated = state.update_item_quantity(product_id.clone(), quantity);

        if !updated {
            let product = ProductAgentClient::get(ProductAgentId::new(product_id.clone()))
                .get_product()
                .await;
            let pricing = PricingAgentClient::get(PricingAgentId::new(product_id.clone()))
                .get_price(state.currency.clone(), PRICING_ZONE_DEFAULT.to_string())
                .await;
            match (product, pricing) {
                (Some(product), Some(pricing)) => {
                    state.add_item(get_cart_item(product, pricing, quantity));
                }
                (None, _) => {
                    return Err(AddItemError::ProductNotFound(product_not_found_error(
                        product_id,
                    )));
                }
                _ => {
                    return Err(AddItemError::PricingNotFound(pricing_not_found_error(
                        product_id,
                    )))
                }
            }
        }
        Ok(())
    }

    async fn checkout(&mut self) -> Result<OrderConfirmation, CheckoutError> {
        let state = self.get_state();
        let order_id = generate_order_id();
        println!("Checkout for order {}", order_id);

        create_order(order_id.clone(), state.clone()).await?;

        state.order_created(order_id.clone());

        ShoppingAssistantAgentClient::get(ShoppingAssistantAgentId::new(state.user_id.clone()))
            .recommend_items()
            .await;

        Ok(OrderConfirmation { order_id })
    }

    fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError> {
        self.with_state(|state| {
            println!(
                "Updating email {} for the cart of user {}",
                email, state.user_id
            );

            match EmailAddress::from_str(email.as_str()) {
                Ok(_) => {
                    state.set_email(email);
                    Ok(())
                }
                Err(e) => Err(UpdateEmailError::EmailNotValid(EmailNotValidError {
                    message: format!("Invalid email: {e}"),
                })),
            }
        })
    }

    fn clear(&mut self) {
        self.with_state(|state| {
            println!("Clearing the cart of user {}", state.user_id);
            state.clear();
        })
    }

    fn remove_item(&mut self, product_id: String) -> Result<(), RemoveItemError> {
        self.with_state(|state| {
            println!(
                "Removing item with product {} from the cart of user {}",
                product_id, state.user_id
            );

            if state.remove_item(product_id.clone()) {
                Ok(())
            } else {
                Err(RemoveItemError::ItemNotFound(item_not_found_error(
                    product_id,
                )))
            }
        })
    }

    fn update_billing_address(&mut self, address: Address) -> Result<(), UpdateAddressError> {
        self.with_state(|state| {
            println!(
                "Updating billing address in the cart of user {}",
                state.user_id
            );

            state.set_billing_address(address.into());
            Ok(())
        })
    }

    fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<(), UpdateItemQuantityError> {
        self.with_state(|state| {
            println!(
                "Updating quantity of item with product {} to {} in the cart of user {}",
                product_id, quantity, state.user_id
            );

            let updated = state.update_item_quantity(product_id.clone(), quantity);

            if updated {
                Ok(())
            } else {
                Err(UpdateItemQuantityError::ItemNotFound(item_not_found_error(
                    product_id,
                )))
            }
        })
    }

    fn update_shipping_address(&mut self, address: Address) -> Result<(), UpdateAddressError> {
        self.with_state(|state| {
            println!(
                "Updating shipping address in the cart of user {}",
                state.user_id
            );

            state.set_shipping_address(address.into());
            Ok(())
        })
    }
}

#[derive(Schema)]
pub struct CartAgentId {
    id: String,
}

impl CartAgentId {
    pub fn new(id: String) -> Self {
        CartAgentId { id }
    }
}
