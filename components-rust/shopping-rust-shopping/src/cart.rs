use golem_rust::{agent_definition, agent_implementation, Schema};
use crate::common::{Address, CURRENCY_DEFAULT};
use crate::pricing::{Pricing, PricingItem};

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

pub fn get_total_price(items: Vec<CartItem>) -> f32 {
    let mut total = 0f32;

    for item in items {
        total += item.price * item.quantity as f32;
    }

    total
}


#[agent_definition]
trait CartAgent {
    fn new(init: CartAgentId) -> Self;
    
    async fn get_cart(&self) -> Option<Cart>;

}

struct CartAgentImpl {
    _id: CartAgentId,
    state: Option<Cart>,
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
}

#[derive(Schema)]
struct CartAgentId {
    id: String,
}
