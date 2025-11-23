use crate::common::{Address, CURRENCY_DEFAULT};
use email_address::EmailAddress;
use golem_rust::{agent_definition, agent_implementation, Schema};
use std::str::FromStr;

#[derive(Schema, Clone)]
pub struct Order {
    pub order_id: String,
    pub user_id: String,
    pub email: Option<String>,
    pub order_status: OrderStatus,
    pub items: Vec<OrderItem>,
    pub billing_address: Option<Address>,
    pub shipping_address: Option<Address>,
    pub total: f32,
    pub currency: String,
    // pub created_at: chrono::DateTime<chrono::Utc>,
    // pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Order {
    pub fn new(order_id: String, user_id: String) -> Self {
        // let now = chrono::Utc::now();
        Self {
            order_id,
            user_id,
            email: None,
            order_status: OrderStatus::New,
            items: vec![],
            shipping_address: None,
            billing_address: None,
            total: 0f32,
            currency: CURRENCY_DEFAULT.to_string(),
            // created_at: now,
            // updated_at: now,
        }
    }

    pub fn recalculate_total(&mut self) {
        self.total = get_total_price(self.items.clone());
        // self.updated_at = chrono::Utc::now();
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

    pub fn set_order_status(&mut self, status: OrderStatus) {
        self.order_status = status;
        // self.updated_at = chrono::Utc::now();
    }

    pub fn add_item(&mut self, item: OrderItem) -> bool {
        self.items.push(item);
        self.recalculate_total();
        true
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
pub struct OrderItem {
    pub product_id: String,
    pub product_name: String,
    pub product_brand: String,
    pub price: f32,
    pub quantity: u32,
}

#[derive(Schema, Clone, Copy, Eq, PartialEq)]
pub enum OrderStatus {
    New,
    Shipped,
    Cancelled,
}

#[derive(Schema, Clone)]
pub struct CreateOrder {
    pub user_id: String,
    pub email: Option<String>,
    pub items: Vec<OrderItem>,
    pub billing_address: Option<Address>,
    pub shipping_address: Option<Address>,
    pub total: f32,
    pub currency: String,
}

#[derive(Schema, Clone)]
pub struct ActionNotAllowedError {
    pub message: String,
    pub status: OrderStatus,
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
pub enum AddItemError {
    ProductNotFound(ProductNotFoundError),
    PricingNotFound(PricingNotFoundError),
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone)]
pub enum RemoveItemError {
    ItemNotFound(ItemNotFoundError),
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone)]
pub enum ShipOrderError {
    EmptyItems(EmptyItemsError),
    EmptyEmail(EmptyEmailError),
    BillingAddressNotSet(BillingAddressNotSetError),
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone)]
pub enum UpdateEmailError {
    EmailNotValid(EmailNotValidError),
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone)]
pub enum UpdateItemQuantityError {
    ItemNotFound(ItemNotFoundError),
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone)]
pub enum CancelOrderError {
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone)]
pub enum InitOrderError {
    ActionNotAllowed(ActionNotAllowedError),
}

fn action_not_allowed_error(status: OrderStatus) -> ActionNotAllowedError {
    ActionNotAllowedError {
        message: "Can not update order with status".to_string(),
        status: status.into(),
    }
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

pub fn get_total_price(items: Vec<OrderItem>) -> f32 {
    let mut total = 0f32;

    for item in items {
        total += item.price * item.quantity as f32;
    }

    total
}

#[agent_definition]
trait OrderAgent {
    fn new(init: OrderAgentId) -> Self;

    async fn get_cart(&self) -> Option<Order>;
    async fn initialize_order(&mut self, data: CreateOrder) -> Result<(), InitOrderError>;
    async fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError>;
}

struct OrderAgentImpl {
    _id: OrderAgentId,
    state: Option<Order>,
}

impl OrderAgentImpl {
    fn with_state<T>(&mut self, f: impl FnOnce(&mut Order) -> T) -> T {
        if self.state.is_none() {
            let value = Order::new(self._id.id.clone(), "".to_string());
            self.state = Some(value);
        }

        f(self.state.as_mut().unwrap())
    }
}

#[agent_implementation]
impl OrderAgent for OrderAgentImpl {
    fn new(id: OrderAgentId) -> Self {
        OrderAgentImpl {
            _id: id,
            state: None,
        }
    }

    async fn get_cart(&self) -> Option<Order> {
        self.state.clone()
    }

    async fn initialize_order(&mut self, data: CreateOrder) -> Result<(), InitOrderError> {
        self.with_state(|state| {
            println!(
                "Initializing order {} for user {}",
                state.order_id, data.user_id
            );
            if state.order_status == OrderStatus::New {
                state.user_id = data.user_id;
                state.email = data.email;
                state.items = data.items;
                state.billing_address = data.billing_address;
                state.shipping_address = data.shipping_address;
                state.total = data.total;
                state.currency = data.currency;

                Ok(())
            } else {
                Err(InitOrderError::ActionNotAllowed(action_not_allowed_error(
                    state.order_status,
                )))
            }
        })
    }

    async fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError> {
        self.with_state(|state| {
            println!(
                "Updating email {} for the order {} of user {}",
                email, state.order_id, state.user_id
            );

            if state.order_status == OrderStatus::New {
                match EmailAddress::from_str(email.as_str()) {
                    Ok(_) => {
                        state.set_email(email);
                        Ok(())
                    }
                    Err(e) => Err(UpdateEmailError::EmailNotValid(EmailNotValidError {
                        message: format!("Invalid email: {e}"),
                    })),
                }
            } else {
                Err(UpdateEmailError::ActionNotAllowed(
                    action_not_allowed_error(state.order_status),
                ))
            }
        })
    }
}

#[derive(Schema)]
struct OrderAgentId {
    id: String,
}
