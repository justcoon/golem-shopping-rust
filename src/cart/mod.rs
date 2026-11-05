use crate::common::{
    Address, CHECKOUT_RESERVATION_TTL_SECONDS, CURRENCY_DEFAULT, INVENTORY_REGION_DEFAULT,
    PRICING_REGION_DEFAULT,
};
use crate::inventory::InventoryAgentClient;
use crate::order::{CreateOrder, Order, OrderAgentClient, OrderItem};
use crate::pricing::{PricingAgentClient, PricingItem};
use crate::product::{Product, ProductAgentClient};
use crate::shopping_assistant::ShoppingAssistantAgentClient;
use email_address::EmailAddress;
use futures::future::{join, join_all};
use golem_rust::{Schema, agent_definition, agent_implementation, description, endpoint, prompt};
use log::info;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct CheckoutDraft {
    pub order_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Cart {
    pub user_id: String,
    pub email: Option<String>,
    pub items: Vec<CartItem>,
    pub billing_address: Option<Address>,
    pub shipping_address: Option<Address>,
    pub total: f32,
    pub currency: String,
    pub previous_order_ids: Vec<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub draft: Option<CheckoutDraft>,
}

impl Cart {
    fn new(user_id: String) -> Self {
        Self {
            user_id,
            email: None,
            items: vec![],
            billing_address: None,
            shipping_address: None,
            total: 0.0,
            currency: CURRENCY_DEFAULT.to_string(),
            updated_at: chrono::Utc::now(),
            previous_order_ids: vec![],
            draft: None,
        }
    }

    fn order_created(&mut self, order_id: String) {
        self.clear();
        self.draft = None;
        self.previous_order_ids.push(order_id);
    }

    pub fn purge_expired_draft(&mut self) {
        let is_expired = self
            .draft
            .as_ref()
            .is_some_and(|d| d.expires_at <= chrono::Utc::now());
        if is_expired {
            self.draft = None;
            self.updated_at = chrono::Utc::now();
        }
    }

    pub fn is_in_checkout(&self) -> bool {
        match &self.draft {
            Some(d) => d.expires_at > chrono::Utc::now(),
            None => false,
        }
    }

    pub fn ensure_modifiable(&mut self) -> Result<(), ActionNotAllowedError> {
        self.purge_expired_draft();
        if let Some(ref draft) = self.draft {
            return Err(ActionNotAllowedError {
                message: format!(
                    "Cannot modify cart while checkout is in progress for order #{}. Please confirm or cancel the active checkout first.",
                    draft.order_id
                ),
            });
        }
        Ok(())
    }

    fn clear(&mut self) {
        self.items.clear();
        self.billing_address = None;
        self.shipping_address = None;
        self.total = 0.0;
        self.updated_at = chrono::Utc::now();
    }

    fn recalculate_total(&mut self) {
        self.total = get_total_price(self.items.clone());
        self.updated_at = chrono::Utc::now();
    }

    fn add_item(&mut self, item: CartItem) -> bool {
        self.items.push(item);
        self.recalculate_total();
        true
    }

    fn set_items(&mut self, items: Vec<CartItem>) {
        self.items = items;
        self.recalculate_total();
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

#[derive(Schema, Clone, Serialize, Deserialize)]
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
            shipping_address: value.shipping_address,
            billing_address: value.billing_address,
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
#[derive(Schema, Clone, Debug)]
pub struct OrderCreateError {
    pub message: String,
}
#[derive(Schema, Clone, Serialize, Deserialize, Debug)]
pub struct InsufficientStockError {
    pub message: String,
    pub product_id: String,
    pub requested: u32,
    pub available: u32,
}
#[derive(Schema, Clone, Serialize, Deserialize, Debug)]
pub struct ActionNotAllowedError {
    pub message: String,
}

#[derive(Schema, Clone)]
pub enum AddItemError {
    ProductNotFound(ProductNotFoundError),
    PricingNotFound(PricingNotFoundError),
    InsufficientStock(InsufficientStockError),
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
}
#[derive(Schema, Clone)]
pub enum UpdateEmailError {
    EmailNotValid(EmailNotValidError),
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone)]
pub enum UpdateItemQuantityError {
    ItemNotFound(ItemNotFoundError),
    InsufficientStock(InsufficientStockError),
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone)]
pub enum PrepareCheckoutError {
    EmptyItems(EmptyItemsError),
    EmptyEmail(EmptyEmailError),
    BillingAddressNotSet(BillingAddressNotSetError),
    InsufficientStock(InsufficientStockError),
    ActionNotAllowed(ActionNotAllowedError),
}
#[derive(Schema, Clone, Debug)]
pub enum ConfirmCheckoutError {
    NoActiveDraft(String),
    OrderMismatch(String),
    ReservationExpired(String),
    OrderCreate(OrderCreateError),
}
#[derive(Schema, Clone)]
pub enum UpdateAddressError {
    AddressNotValid(AddressNotValidError),
    ActionNotAllowed(ActionNotAllowedError),
}

#[derive(Schema, Clone)]
pub struct OrderConfirmation {
    pub user_id: String,
    pub order_id: String,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SavedContactInfo {
    pub shipping_address: Option<Address>,
    pub billing_address: Option<Address>,
    pub email: Option<String>,
    pub source_order_id: Option<String>,
}

#[derive(Schema, Clone)]
pub struct CartUpdated {
    pub user_id: String,
}

fn get_total_price(items: Vec<CartItem>) -> f32 {
    let mut total = 0f32;

    for item in items {
        total += item.price * item.quantity as f32;
    }

    total
}

fn validate_prepare_checkout(cart: &Cart) -> Result<(), PrepareCheckoutError> {
    if cart.items.is_empty() {
        Err(PrepareCheckoutError::EmptyItems(EmptyItemsError {
            message: "Empty items".to_string(),
        }))
    } else if cart.billing_address.is_none() {
        Err(PrepareCheckoutError::BillingAddressNotSet(
            BillingAddressNotSetError {
                message: "Billing address not set".to_string(),
            },
        ))
    } else if cart.email.is_none() {
        Err(PrepareCheckoutError::EmptyEmail(EmptyEmailError {
            message: "Email not set".to_string(),
        }))
    } else {
        Ok(())
    }
}

fn validate_checkout_draft(cart: &mut Cart, order_id: &str) -> Result<(), ConfirmCheckoutError> {
    match &cart.draft {
        None => Err(ConfirmCheckoutError::NoActiveDraft(
            "No active checkout draft found. Please prepare checkout first.".to_string(),
        )),
        Some(draft) if draft.order_id != order_id => {
            Err(ConfirmCheckoutError::OrderMismatch(format!(
                "Requested order_id {} does not match active draft {}",
                order_id, draft.order_id
            )))
        }
        Some(draft) if draft.expires_at <= chrono::Utc::now() => {
            cart.draft = None;
            cart.updated_at = chrono::Utc::now();
            Err(ConfirmCheckoutError::ReservationExpired(
                "The checkout reservation has expired. Please prepare checkout again.".to_string(),
            ))
        }
        Some(_) => Ok(()),
    }
}

fn generate_order_id() -> String {
    Uuid::new_v4().to_string()
}

fn release_inventory_reservations<I, S>(product_ids: I, order_id: &str)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for pid in product_ids {
        let mut client = InventoryAgentClient::get(pid.as_ref().to_string());
        client.trigger_release_reservation(order_id.to_string());
    }
}

fn fulfill_inventory_reservations<I, S>(product_ids: I, order_id: &str)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for pid in product_ids {
        let mut client = InventoryAgentClient::get(pid.as_ref().to_string());
        client.trigger_fulfill_reservation(order_id.to_string());
    }
}

fn map_reservation_error(
    item: &CartItem,
    err: crate::inventory::ReserveQuantityError,
) -> PrepareCheckoutError {
    let (message, requested, available) = match err {
        crate::inventory::ReserveQuantityError::InsufficientInventory(e) => {
            (e.message, e.requested, e.available)
        }
        crate::inventory::ReserveQuantityError::InventoryNotFound(e) => {
            (e.message, item.quantity, 0)
        }
    };

    PrepareCheckoutError::InsufficientStock(InsufficientStockError {
        message,
        product_id: item.product_id.clone(),
        requested,
        available,
    })
}

fn map_init_order_error(err: crate::order::InitOrderError) -> ConfirmCheckoutError {
    let message = match err {
        crate::order::InitOrderError::InsufficientInventory(e) => e.message,
        crate::order::InitOrderError::ActionNotAllowed(e) => e.message,
    };
    ConfirmCheckoutError::OrderCreate(OrderCreateError {
        message: format!("Failed to create order: {message}"),
    })
}

async fn reserve_inventory_for_cart(
    items: &[CartItem],
    order_id: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), PrepareCheckoutError> {
    let region = INVENTORY_REGION_DEFAULT.to_string();
    let mut reserved_products: Vec<String> = Vec::new();

    for item in items {
        let mut inv_client = InventoryAgentClient::get(item.product_id.clone());
        let res = inv_client
            .reserve_quantity(
                order_id.to_string(),
                region.clone(),
                item.quantity,
                Some(expires_at),
            )
            .await;

        match res {
            Ok(_) => reserved_products.push(item.product_id.clone()),
            Err(err) => {
                release_inventory_reservations(&reserved_products, order_id);
                return Err(map_reservation_error(item, err));
            }
        }
    }

    Ok(())
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

#[agent_definition(mount = "/v1/cart/{id}")]
pub trait CartAgent {
    fn new(id: String) -> Self;

    #[endpoint(get = "/")]
    async fn get_cart(&mut self) -> Option<Cart>;

    #[endpoint(put = "/items/{product_id}")]
    async fn add_item(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<CartUpdated, AddItemError>;

    #[endpoint(post = "/checkout/prepare")]
    async fn prepare_checkout(&mut self) -> Result<Cart, PrepareCheckoutError>;

    #[endpoint(post = "/checkout/confirm")]
    async fn confirm_checkout(
        &mut self,
        order_id: String,
    ) -> Result<OrderConfirmation, ConfirmCheckoutError>;

    #[endpoint(post = "/checkout/cancel")]
    async fn cancel_checkout(&mut self, order_id: String) -> Result<(), String>;

    #[endpoint(put = "/email")]
    fn update_email(&mut self, email: String) -> Result<CartUpdated, UpdateEmailError>;

    fn clear(&mut self);

    #[endpoint(delete = "/items/{product_id}")]
    fn remove_item(&mut self, product_id: String) -> Result<CartUpdated, RemoveItemError>;

    #[endpoint(put = "/billing-address")]
    fn update_billing_address(
        &mut self,
        street: String,
        city: String,
        state_or_region: String,
        country: String,
        postal_code: String,
        name: Option<String>,
        phone_number: Option<String>,
    ) -> Result<CartUpdated, UpdateAddressError>;

    async fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<CartUpdated, UpdateItemQuantityError>;

    #[endpoint(put = "/shipping-address")]
    fn update_shipping_address(
        &mut self,
        street: String,
        city: String,
        state_or_region: String,
        country: String,
        postal_code: String,
        name: Option<String>,
        phone_number: Option<String>,
    ) -> Result<CartUpdated, UpdateAddressError>;

    #[endpoint(get = "/saved-contact")]
    #[prompt(
        "Call this before checkout to check if the user has saved shipping/billing details from previous orders."
    )]
    #[description(
        "Retrieves previously saved contact information and addresses from the user's past orders."
    )]
    async fn get_saved_contact_info(&self) -> Option<SavedContactInfo>;

    #[endpoint(post = "/use-saved-contact")]
    #[prompt(
        "Call this when the user confirms they want to use their saved address and contact info for 1-click checkout."
    )]
    #[description(
        "Applies saved contact details and addresses from previous orders to the current cart."
    )]
    async fn apply_saved_contact_info(&mut self) -> Result<CartUpdated, UpdateAddressError>;

    #[endpoint(get = "/orders?limit={limit}")]
    #[prompt(
        "Call this when the user asks about their previous orders, order history, or recent purchases."
    )]
    #[description("Retrieves past orders placed by this customer.")]
    async fn get_order_history(&self, limit: Option<u32>) -> Vec<Order>;
}

struct CartAgentImpl {
    _id: String,
    state: Option<Cart>,
}

impl CartAgentImpl {
    fn get_state(&mut self) -> &mut Cart {
        self.state.get_or_insert(Cart::new(self._id.clone()))
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut Cart) -> T) -> T {
        f(self.get_state())
    }
}

#[agent_implementation]
impl CartAgent for CartAgentImpl {
    fn new(id: String) -> Self {
        CartAgentImpl {
            _id: id,
            state: None,
        }
    }

    async fn get_cart(&mut self) -> Option<Cart> {
        info!("Getting cart");
        if let Some(cart) = self.state.as_mut() {
            cart.purge_expired_draft();
            if cart.is_in_checkout() {
                // Freezes cart state during active checkout draft
                return Some(cart.clone());
            }

            let mut items = Vec::new();
            for item in cart.items.clone() {
                let product_id = item.product_id;
                let quantity = item.quantity;

                let product_client = ProductAgentClient::get(product_id.clone());
                let pricing_client = PricingAgentClient::get(product_id.clone());

                let (product, pricing) = join(
                    product_client.get_product(),
                    pricing_client
                        .get_price(cart.currency.clone(), PRICING_REGION_DEFAULT.to_string()),
                )
                .await;

                if let (Some(product), Some(pricing)) = (product, pricing) {
                    items.push(get_cart_item(product, pricing, quantity));
                }
            }
            cart.set_items(items);
            Some(cart.clone())
        } else {
            None
        }
    }

    async fn add_item(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<CartUpdated, AddItemError> {
        let state = self.get_state();
        state
            .ensure_modifiable()
            .map_err(AddItemError::ActionNotAllowed)?;

        info!(
            "Adding item with product {} to the cart of user {}",
            product_id, state.user_id
        );

        let current_qty = state
            .items
            .iter()
            .find(|i| i.product_id == product_id)
            .map(|i| i.quantity)
            .unwrap_or(0);
        let desired_qty = current_qty + quantity;

        let region = INVENTORY_REGION_DEFAULT.to_string();

        let availability = InventoryAgentClient::get(product_id.clone())
            .get_available_quantity(region)
            .await;

        if desired_qty > availability.available_quantity {
            return Err(AddItemError::InsufficientStock(InsufficientStockError {
                message: format!(
                    "Insufficient stock for product {}. Requested: {}, available: {}",
                    product_id, desired_qty, availability.available_quantity
                ),
                product_id,
                requested: desired_qty,
                available: availability.available_quantity,
            }));
        }

        let updated = state.update_item_quantity(product_id.clone(), quantity, true);

        if !updated {
            let product_client = ProductAgentClient::get(product_id.clone());
            let pricing_client = PricingAgentClient::get(product_id.clone());

            let (product, pricing) = join(
                product_client.get_product(),
                pricing_client
                    .get_price(state.currency.clone(), PRICING_REGION_DEFAULT.to_string()),
            )
            .await;

            match (product, pricing) {
                (Some(product), Some(pricing)) => {
                    state.add_item(get_cart_item(product, pricing, quantity));
                }
                (None, _) => {
                    return Err(AddItemError::ProductNotFound(ProductNotFoundError::new(
                        product_id,
                    )));
                }
                _ => {
                    return Err(AddItemError::PricingNotFound(PricingNotFoundError::new(
                        product_id,
                    )));
                }
            }
        }
        Ok(CartUpdated {
            user_id: state.user_id.clone(),
        })
    }

    async fn prepare_checkout(&mut self) -> Result<Cart, PrepareCheckoutError> {
        let state = self.get_state();
        state
            .ensure_modifiable()
            .map_err(PrepareCheckoutError::ActionNotAllowed)?;
        validate_prepare_checkout(state)?;

        let order_id = generate_order_id();
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::seconds(CHECKOUT_RESERVATION_TTL_SECONDS);

        reserve_inventory_for_cart(&state.items, &order_id, expires_at).await?;

        let draft = CheckoutDraft {
            order_id: order_id.clone(),
            expires_at,
            created_at: now,
        };

        state.draft = Some(draft);
        state.updated_at = now;

        info!(
            "Prepared checkout draft for user {} with order_id {} expiring at {}",
            state.user_id, order_id, expires_at
        );

        Ok(state.clone())
    }

    async fn confirm_checkout(
        &mut self,
        order_id: String,
    ) -> Result<OrderConfirmation, ConfirmCheckoutError> {
        let state = self.get_state();
        validate_checkout_draft(state, &order_id)?;

        let order = state.clone().into();
        OrderAgentClient::get(order_id.clone())
            .initialize_order(order)
            .await
            .map_err(map_init_order_error)?;

        fulfill_inventory_reservations(state.items.iter().map(|i| &i.product_id), &order_id);

        let user_id = state.user_id.clone();
        state.order_created(order_id.clone());

        ShoppingAssistantAgentClient::get(user_id.clone()).trigger_recommend_items();

        info!(
            "Successfully confirmed checkout for user {} and order {}",
            user_id, order_id
        );

        Ok(OrderConfirmation { user_id, order_id })
    }

    async fn cancel_checkout(&mut self, order_id: String) -> Result<(), String> {
        let state = self.get_state();
        let matches = state.draft.as_ref().is_some_and(|d| d.order_id == order_id);
        if matches {
            release_inventory_reservations(state.items.iter().map(|i| &i.product_id), &order_id);
            state.draft = None;
            state.updated_at = chrono::Utc::now();
            info!(
                "Cancelled checkout draft {} for user {}",
                order_id, state.user_id
            );
        }
        Ok(())
    }

    fn update_email(&mut self, email: String) -> Result<CartUpdated, UpdateEmailError> {
        self.with_state(|state| {
            state
                .ensure_modifiable()
                .map_err(UpdateEmailError::ActionNotAllowed)?;

            info!(
                "Updating email {} for the cart of user {}",
                email, state.user_id
            );

            match EmailAddress::from_str(email.as_str()) {
                Ok(_) => {
                    state.set_email(email);
                    Ok(CartUpdated {
                        user_id: state.user_id.clone(),
                    })
                }
                Err(e) => Err(UpdateEmailError::EmailNotValid(EmailNotValidError {
                    message: format!("Invalid email: {e}"),
                })),
            }
        })
    }

    fn clear(&mut self) {
        self.with_state(|state| {
            if state.ensure_modifiable().is_err() {
                return;
            }
            info!("Clearing the cart of user {}", state.user_id);
            state.clear();
        })
    }

    fn remove_item(&mut self, product_id: String) -> Result<CartUpdated, RemoveItemError> {
        self.with_state(|state| {
            state
                .ensure_modifiable()
                .map_err(RemoveItemError::ActionNotAllowed)?;

            info!(
                "Removing item with product {} from the cart of user {}",
                product_id, state.user_id
            );

            if state.remove_item(product_id.clone()) {
                Ok(CartUpdated {
                    user_id: state.user_id.clone(),
                })
            } else {
                Err(RemoveItemError::ItemNotFound(ItemNotFoundError::new(
                    product_id,
                )))
            }
        })
    }

    fn update_billing_address(
        &mut self,
        street: String,
        city: String,
        state_or_region: String,
        country: String,
        postal_code: String,
        name: Option<String>,
        phone_number: Option<String>,
    ) -> Result<CartUpdated, UpdateAddressError> {
        self.with_state(|state| {
            state
                .ensure_modifiable()
                .map_err(UpdateAddressError::ActionNotAllowed)?;

            info!(
                "Updating billing address in the cart of user {}",
                state.user_id
            );

            let address = Address {
                street,
                city,
                state_or_region,
                country,
                postal_code,
                name,
                phone_number,
            };
            state.set_billing_address(address);
            Ok(CartUpdated {
                user_id: state.user_id.clone(),
            })
        })
    }

    async fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<CartUpdated, UpdateItemQuantityError> {
        let state = self.get_state();
        state
            .ensure_modifiable()
            .map_err(UpdateItemQuantityError::ActionNotAllowed)?;

        info!(
            "Updating quantity of item with product {} to {} in the cart of user {}",
            product_id, quantity, state.user_id
        );

        let item_exists = state.items.iter().any(|i| i.product_id == product_id);
        if !item_exists {
            return Err(UpdateItemQuantityError::ItemNotFound(
                ItemNotFoundError::new(product_id),
            ));
        }

        let region = INVENTORY_REGION_DEFAULT.to_string();

        let availability = InventoryAgentClient::get(product_id.clone())
            .get_available_quantity(region)
            .await;

        if quantity > availability.available_quantity {
            return Err(UpdateItemQuantityError::InsufficientStock(
                InsufficientStockError {
                    message: format!(
                        "Insufficient stock for product {}. Requested: {}, available: {}",
                        product_id, quantity, availability.available_quantity
                    ),
                    product_id,
                    requested: quantity,
                    available: availability.available_quantity,
                },
            ));
        }

        state.update_item_quantity(product_id, quantity, false);
        Ok(CartUpdated {
            user_id: state.user_id.clone(),
        })
    }

    fn update_shipping_address(
        &mut self,
        street: String,
        city: String,
        state_or_region: String,
        country: String,
        postal_code: String,
        name: Option<String>,
        phone_number: Option<String>,
    ) -> Result<CartUpdated, UpdateAddressError> {
        self.with_state(|state| {
            state
                .ensure_modifiable()
                .map_err(UpdateAddressError::ActionNotAllowed)?;

            info!(
                "Updating shipping address in the cart of user {}",
                state.user_id
            );

            let address = Address {
                street,
                city,
                state_or_region,
                country,
                postal_code,
                name,
                phone_number,
            };
            state.set_shipping_address(address);
            Ok(CartUpdated {
                user_id: state.user_id.clone(),
            })
        })
    }

    async fn get_saved_contact_info(&self) -> Option<SavedContactInfo> {
        info!("Getting saved contact info for user {}", self._id);
        let cart = self.state.as_ref()?;
        let last_order_id = cart.previous_order_ids.last()?.clone();
        let order = OrderAgentClient::get(last_order_id.clone())
            .get_order()
            .await?;

        Some(SavedContactInfo {
            shipping_address: order.shipping_address,
            billing_address: order.billing_address,
            email: order.email,
            source_order_id: Some(last_order_id),
        })
    }

    async fn apply_saved_contact_info(&mut self) -> Result<CartUpdated, UpdateAddressError> {
        let state = self.get_state();
        state
            .ensure_modifiable()
            .map_err(UpdateAddressError::ActionNotAllowed)?;

        info!("Applying saved contact info for user {}", state.user_id);

        let last_order_id = match state.previous_order_ids.last() {
            Some(id) => id.clone(),
            None => {
                return Err(UpdateAddressError::AddressNotValid(AddressNotValidError {
                    message: "No previous orders found to apply saved contact info".to_string(),
                }));
            }
        };

        let order = match OrderAgentClient::get(last_order_id.clone())
            .get_order()
            .await
        {
            Some(order) => order,
            None => {
                return Err(UpdateAddressError::AddressNotValid(AddressNotValidError {
                    message: format!("Previous order {last_order_id} could not be retrieved"),
                }));
            }
        };

        if let Some(shipping) = order.shipping_address {
            state.set_shipping_address(shipping);
        }
        if let Some(billing) = order.billing_address {
            state.set_billing_address(billing);
        }
        if let Some(email) = order.email {
            state.set_email(email);
        }

        Ok(CartUpdated {
            user_id: state.user_id.clone(),
        })
    }

    async fn get_order_history(&self, limit: Option<u32>) -> Vec<Order> {
        info!(
            "Getting order history for user {}, limit: {:?}",
            self._id, limit
        );
        let cart = match self.state.as_ref() {
            Some(cart) => cart,
            None => return vec![],
        };

        if cart.previous_order_ids.is_empty() {
            return vec![];
        }

        let max_count = limit.unwrap_or(10) as usize;
        let order_ids: Vec<String> = cart
            .previous_order_ids
            .iter()
            .rev()
            .take(max_count)
            .cloned()
            .collect();

        let tasks: Vec<_> = order_ids
            .into_iter()
            .map(|order_id| async move {
                let client = OrderAgentClient::get(order_id);
                client.get_order().await
            })
            .collect();

        let orders = join_all(tasks).await;
        orders.into_iter().flatten().collect()
    }

    async fn load_snapshot(
        bytes: Vec<u8>,
        context: golem_rust::agentic::SnapshotRestoreContext,
    ) -> Result<Self, String> {
        let data: Option<Cart> = crate::common::snapshot::deserialize(&bytes)?;
        let mut parameters = golem_rust::agentic::DirectAgentInput::new(context.parameters)
            .map_err(|error| error.to_string())?;
        let id: String = parameters.take().map_err(|error| error.to_string())?;
        parameters.finish().map_err(|error| error.to_string())?;

        Ok(Self {
            _id: id,
            state: data,
        })
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saved_contact_info_construction() {
        let addr = Address {
            street: "123 Main St".to_string(),
            city: "San Francisco".to_string(),
            state_or_region: "CA".to_string(),
            country: "USA".to_string(),
            postal_code: "94105".to_string(),
            name: Some("Jane Doe".to_string()),
            phone_number: Some("+1-555-0199".to_string()),
        };

        let contact = SavedContactInfo {
            shipping_address: Some(addr.clone()),
            billing_address: Some(addr),
            email: Some("jane@example.com".to_string()),
            source_order_id: Some("ord-999".to_string()),
        };

        assert_eq!(contact.email.as_deref(), Some("jane@example.com"));
        assert_eq!(contact.source_order_id.as_deref(), Some("ord-999"));
    }

    #[test]
    fn test_cart_draft_lifecycle_and_locking() {
        let mut cart = Cart::new("user-1".to_string());
        assert!(!cart.is_in_checkout());
        assert!(cart.ensure_modifiable().is_ok());

        // Set an active draft
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::seconds(CHECKOUT_RESERVATION_TTL_SECONDS);
        cart.draft = Some(CheckoutDraft {
            order_id: "ord-test-123".to_string(),
            expires_at,
            created_at: now,
        });

        // Cart is now in checkout and locked
        assert!(cart.is_in_checkout());
        let mod_err = cart.ensure_modifiable();
        assert!(mod_err.is_err());
        assert!(mod_err.unwrap_err().message.contains("ord-test-123"));

        // When order is created or cancelled, cart unlocks
        cart.draft = None;
        assert!(!cart.is_in_checkout());
        assert!(cart.ensure_modifiable().is_ok());
    }

    #[test]
    fn test_cart_draft_expiration() {
        let mut cart = Cart::new("user-1".to_string());
        let past = chrono::Utc::now() - chrono::Duration::seconds(10);
        cart.draft = Some(CheckoutDraft {
            order_id: "ord-expired-999".to_string(),
            expires_at: past,
            created_at: past - chrono::Duration::seconds(CHECKOUT_RESERVATION_TTL_SECONDS),
        });

        // is_in_checkout is false because expires_at is past
        assert!(!cart.is_in_checkout());

        // ensure_modifiable automatically purges expired draft
        assert!(cart.ensure_modifiable().is_ok());
        assert!(cart.draft.is_none());
    }

    #[test]
    fn test_validate_prepare_checkout() {
        let mut cart = Cart::new("user-1".to_string());

        // 1. Empty items
        assert!(matches!(
            validate_prepare_checkout(&cart),
            Err(PrepareCheckoutError::EmptyItems(_))
        ));

        // Add item
        cart.items.push(CartItem {
            product_id: "prod-1".to_string(),
            product_name: "Item 1".to_string(),
            product_brand: "Brand".to_string(),
            price: 10.0,
            quantity: 2,
        });

        // 2. Missing billing address
        assert!(matches!(
            validate_prepare_checkout(&cart),
            Err(PrepareCheckoutError::BillingAddressNotSet(_))
        ));

        cart.billing_address = Some(Address {
            street: "123 Main St".to_string(),
            city: "City".to_string(),
            state_or_region: "State".to_string(),
            country: "Country".to_string(),
            postal_code: "12345".to_string(),
            name: None,
            phone_number: None,
        });

        // 3. Missing email
        assert!(matches!(
            validate_prepare_checkout(&cart),
            Err(PrepareCheckoutError::EmptyEmail(_))
        ));

        cart.email = Some("test@example.com".to_string());

        // 4. Valid cart ready for checkout
        assert!(validate_prepare_checkout(&cart).is_ok());
    }

    #[test]
    fn test_validate_checkout_draft() {
        let mut cart = Cart::new("user-1".to_string());

        // 1. No active draft
        assert!(matches!(
            validate_checkout_draft(&mut cart, "ord-1"),
            Err(ConfirmCheckoutError::NoActiveDraft(_))
        ));

        // Set draft
        let now = chrono::Utc::now();
        cart.draft = Some(CheckoutDraft {
            order_id: "ord-1".to_string(),
            expires_at: now + chrono::Duration::seconds(600),
            created_at: now,
        });

        // 2. Order ID mismatch
        assert!(matches!(
            validate_checkout_draft(&mut cart, "ord-2"),
            Err(ConfirmCheckoutError::OrderMismatch(_))
        ));

        // 3. Valid draft with matching order ID
        assert!(validate_checkout_draft(&mut cart, "ord-1").is_ok());

        // 4. Expired draft
        let past = now - chrono::Duration::seconds(10);
        cart.draft = Some(CheckoutDraft {
            order_id: "ord-1".to_string(),
            expires_at: past,
            created_at: past - chrono::Duration::seconds(600),
        });
        assert!(matches!(
            validate_checkout_draft(&mut cart, "ord-1"),
            Err(ConfirmCheckoutError::ReservationExpired(_))
        ));
        assert!(cart.draft.is_none());
    }
}
