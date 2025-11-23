use crate::common::{Address, CURRENCY_DEFAULT};
use email_address::EmailAddress;
use golem_rust::{agent_definition, agent_implementation, Schema};
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

#[agent_definition]
trait CartAgent {
    fn new(init: CartAgentId) -> Self;

    async fn get_cart(&self) -> Option<Cart>;
    async fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError>;
}

struct CartAgentImpl {
    _id: CartAgentId,
    state: Option<Cart>,
}

impl CartAgentImpl {
    fn with_state<T>(&mut self, f: impl FnOnce(&mut Cart) -> T) -> T {
        if self.state.is_none() {
            let value = Cart::new(self._id.id.clone());
            self.state = Some(value);
        }

        f(self.state.as_mut().unwrap())
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

    async fn get_cart(&self) -> Option<Cart> {
        self.state.clone()
    }

    async fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError> {
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
}

#[derive(Schema)]
struct CartAgentId {
    id: String,
}
