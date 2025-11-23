use crate::cart::Cart;
use crate::common::{Address, CURRENCY_DEFAULT};
use golem_rust::{agent_definition, agent_implementation, Schema};

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

#[derive(Schema, Clone)]
pub enum OrderStatus {
    New,
    Shipped,
    Cancelled,
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
}

struct OrderAgentImpl {
    _id: OrderAgentId,
    state: Option<Order>,
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
}

#[derive(Schema)]
struct OrderAgentId {
    id: String,
}
