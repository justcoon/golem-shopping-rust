use crate::common::{Address, CURRENCY_DEFAULT, PRICING_ZONE_DEFAULT};
use crate::pricing::PricingAgentClient;
use crate::product::ProductAgentClient;
use email_address::EmailAddress;
use futures::future::join;
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Order {
    fn new(order_id: String, user_id: String) -> Self {
        let now = chrono::Utc::now();
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
            created_at: now,
            updated_at: now,
        }
    }

    fn recalculate_total(&mut self) {
        self.total = get_total_price(self.items.clone());
        self.updated_at = chrono::Utc::now();
    }

    fn set_billing_address(&mut self, address: Address) {
        self.billing_address = Some(address);
        self.updated_at = chrono::Utc::now();
    }

    fn set_shipping_address(&mut self, address: Address) {
        self.shipping_address = Some(address);
        self.updated_at = chrono::Utc::now();
    }

    fn set_email(&mut self, email: String) {
        self.email = Some(email);
        self.updated_at = chrono::Utc::now();
    }

    fn set_order_status(&mut self, status: OrderStatus) {
        self.order_status = status;
        self.updated_at = chrono::Utc::now();
    }

    fn add_item(&mut self, item: OrderItem) -> bool {
        self.items.push(item);
        self.recalculate_total();
        true
    }

    fn update_item_quantity(&mut self, product_id: String, quantity: u32, add: bool) -> bool {
        let mut updated = false;

        for item in &mut self.items {
            if item.product_id == product_id {
                if add {
                    item.quantity += quantity;
                } else {
                    item.quantity = quantity;
                }
                updated = true;
            }
        }

        if updated {
            self.recalculate_total();
        }

        updated
    }

    fn remove_item(&mut self, product_id: String) -> bool {
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

impl ActionNotAllowedError {
    fn new(status: OrderStatus) -> ActionNotAllowedError {
        ActionNotAllowedError {
            message: "Can not update order with status".to_string(),
            status,
        }
    }
}

#[derive(Schema, Clone)]
pub struct ItemNotFoundError {
    pub message: String,
    pub product_id: String,
}

impl ItemNotFoundError {
    fn new(product_id: String) -> ItemNotFoundError {
        ItemNotFoundError {
            message: "Item not found".to_string(),
            product_id,
        }
    }
}

#[derive(Schema, Clone)]
pub struct PricingNotFoundError {
    pub message: String,
    pub product_id: String,
}

impl PricingNotFoundError {
    fn new(product_id: String) -> PricingNotFoundError {
        PricingNotFoundError {
            message: "Pricing not found".to_string(),
            product_id,
        }
    }
}

#[derive(Schema, Clone)]
pub struct ProductNotFoundError {
    pub message: String,
    pub product_id: String,
}

impl ProductNotFoundError {
    fn new(product_id: String) -> ProductNotFoundError {
        ProductNotFoundError {
            message: "Product not found".to_string(),
            product_id,
        }
    }
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
#[derive(Schema, Clone)]
pub enum UpdateAddressError {
    AddressNotValid(AddressNotValidError),
    ActionNotAllowed(ActionNotAllowedError),
}

fn get_total_price(items: Vec<OrderItem>) -> f32 {
    let mut total = 0f32;

    for item in items {
        total += item.price * item.quantity as f32;
    }

    total
}

#[agent_definition]
trait OrderAgent {
    fn new(id: String) -> Self;
    fn initialize_order(&mut self, data: CreateOrder) -> Result<(), InitOrderError>;
    fn get_order(&self) -> Option<Order>;
    async fn add_item(&mut self, product_id: String, quantity: u32) -> Result<(), AddItemError>;
    fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError>;
    fn remove_item(&mut self, product_id: String) -> Result<(), RemoveItemError>;
    fn update_billing_address(&mut self, address: Address) -> Result<(), UpdateAddressError>;
    fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<(), UpdateItemQuantityError>;
    fn update_shipping_address(&mut self, address: Address) -> Result<(), UpdateAddressError>;
    fn ship_order(&mut self) -> Result<(), ShipOrderError>;
    fn cancel_order(&mut self) -> Result<(), CancelOrderError>;
}

struct OrderAgentImpl {
    _id: String,
    state: Option<Order>,
}

impl OrderAgentImpl {
    fn get_state(&mut self) -> &mut Order {
        self.state
            .get_or_insert(Order::new(self._id.clone(), "anonymous".to_string()))
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut Order) -> T) -> T {
        f(self.get_state())
    }
}

#[agent_implementation]
impl OrderAgent for OrderAgentImpl {
    fn new(id: String) -> Self {
        OrderAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_order(&self) -> Option<Order> {
        self.state.clone()
    }

    fn initialize_order(&mut self, data: CreateOrder) -> Result<(), InitOrderError> {
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
                Err(InitOrderError::ActionNotAllowed(
                    ActionNotAllowedError::new(state.order_status),
                ))
            }
        })
    }

    fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError> {
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
                    ActionNotAllowedError::new(state.order_status),
                ))
            }
        })
    }

    async fn add_item(&mut self, product_id: String, quantity: u32) -> Result<(), AddItemError> {
        let state = self.get_state();

        println!(
            "Adding item with product {} to the order {} of user {}",
            product_id, state.order_id, state.user_id
        );

        let updated = state.update_item_quantity(product_id.clone(), quantity, true);

        if !updated {
            let product_client = ProductAgentClient::get(product_id.clone());
            let pricing_client = PricingAgentClient::get(product_id.clone());

            let (product, pricing) = join(
                product_client.get_product(),
                pricing_client.get_price(state.currency.clone(), PRICING_ZONE_DEFAULT.to_string()),
            )
            .await;
            match (product, pricing) {
                (Some(product), Some(pricing)) => {
                    state.add_item(OrderItem {
                        product_id,
                        product_name: product.name,
                        product_brand: product.brand,
                        price: pricing.price,
                        quantity,
                    });
                }
                (None, _) => {
                    return Err(AddItemError::ProductNotFound(ProductNotFoundError::new(
                        product_id,
                    )));
                }
                _ => {
                    return Err(AddItemError::PricingNotFound(PricingNotFoundError::new(
                        product_id,
                    )))
                }
            }
        }
        Ok(())
    }

    fn remove_item(&mut self, product_id: String) -> Result<(), RemoveItemError> {
        self.with_state(|state| {
            println!(
                "Removing item with product {} from the order {} of user {}",
                product_id, state.order_id, state.user_id
            );
            if state.order_status == OrderStatus::New {
                if state.remove_item(product_id.clone()) {
                    Ok(())
                } else {
                    Err(RemoveItemError::ItemNotFound(ItemNotFoundError::new(
                        product_id,
                    )))
                }
            } else {
                Err(RemoveItemError::ActionNotAllowed(
                    ActionNotAllowedError::new(state.order_status),
                ))
            }
        })
    }

    fn update_billing_address(&mut self, address: Address) -> Result<(), UpdateAddressError> {
        self.with_state(|state| {
            println!(
                "Updating billing address in the order {} of user {}",
                state.order_id, state.user_id
            );
            if state.order_status == OrderStatus::New {
                state.set_billing_address(address);
                Ok(())
            } else {
                Err(UpdateAddressError::ActionNotAllowed(
                    ActionNotAllowedError::new(state.order_status),
                ))
            }
        })
    }

    fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<(), UpdateItemQuantityError> {
        self.with_state(|state| {
            println!(
                "Updating quantity of item with product {} to {} in the order {} of user {}",
                product_id, quantity, state.order_id, state.user_id
            );
            if state.order_status == OrderStatus::New {
                let updated = state.update_item_quantity(product_id.clone(), quantity, false);

                if updated {
                    Ok(())
                } else {
                    Err(UpdateItemQuantityError::ItemNotFound(
                        ItemNotFoundError::new(product_id),
                    ))
                }
            } else {
                Err(UpdateItemQuantityError::ActionNotAllowed(
                    ActionNotAllowedError::new(state.order_status),
                ))
            }
        })
    }

    fn update_shipping_address(&mut self, address: Address) -> Result<(), UpdateAddressError> {
        self.with_state(|state| {
            println!(
                "Updating shipping address in the order {} of user {}",
                state.order_id, state.user_id
            );
            if state.order_status == OrderStatus::New {
                state.set_shipping_address(address);
                Ok(())
            } else {
                Err(UpdateAddressError::ActionNotAllowed(
                    ActionNotAllowedError::new(state.order_status),
                ))
            }
        })
    }

    fn ship_order(&mut self) -> Result<(), ShipOrderError> {
        self.with_state(|state| {
            println!(
                "Shipping order {} of user {}",
                state.order_id, state.user_id
            );
            if state.order_status != OrderStatus::New {
                Err(ShipOrderError::ActionNotAllowed(
                    ActionNotAllowedError::new(state.order_status),
                ))
            } else if state.items.is_empty() {
                Err(ShipOrderError::EmptyItems(EmptyItemsError {
                    message: "Empty items".to_string(),
                }))
            } else if state.billing_address.is_none() {
                Err(ShipOrderError::BillingAddressNotSet(
                    BillingAddressNotSetError {
                        message: "Billing address not set".to_string(),
                    },
                ))
            } else if state.email.is_none() {
                Err(ShipOrderError::EmptyEmail(EmptyEmailError {
                    message: "Email not set".to_string(),
                }))
            } else {
                state.set_order_status(OrderStatus::Shipped);
                Ok(())
            }
        })
    }

    fn cancel_order(&mut self) -> Result<(), CancelOrderError> {
        self.with_state(|state| {
            println!(
                "Cancelling order {} of user {}",
                state.order_id, state.user_id
            );

            if state.order_status == OrderStatus::New {
                println!(
                    "Cancelling order {} of user {}",
                    state.order_id, state.user_id
                );
                state.set_order_status(OrderStatus::Cancelled);
                Ok(())
            } else {
                println!(
                    "Cancelling order {} of user {}",
                    state.order_id, state.user_id
                );
                Err(CancelOrderError::ActionNotAllowed(
                    ActionNotAllowedError::new(state.order_status),
                ))
            }
        })
    }
}
