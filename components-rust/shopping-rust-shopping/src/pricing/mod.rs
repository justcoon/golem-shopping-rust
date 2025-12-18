use golem_rust::{agent_definition, agent_implementation, Schema};
use std::collections::HashMap;

#[derive(Schema, Clone)]
pub struct Pricing {
    pub product_id: String,
    pub msrp_prices: Vec<PricingItem>,
    pub list_prices: Vec<PricingItem>,
    pub sale_prices: Vec<SalePricingItem>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Pricing {
    fn new(product_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            product_id,
            msrp_prices: vec![],
            list_prices: vec![],
            sale_prices: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    fn get_price(&self, currency: String, zone: String) -> Option<PricingItem> {
        get_price(currency, zone, self.clone())
    }

    fn set_prices(
        &mut self,
        msrp_prices: Vec<PricingItem>,
        list_prices: Vec<PricingItem>,
        sale_prices: Vec<SalePricingItem>,
    ) {
        self.msrp_prices = msrp_prices;
        self.list_prices = list_prices;
        self.sale_prices = sale_prices;
        self.updated_at = chrono::Utc::now();
    }

    fn update_prices(
        &mut self,
        msrp_prices: Vec<PricingItem>,
        list_prices: Vec<PricingItem>,
        sale_prices: Vec<SalePricingItem>,
    ) {
        self.msrp_prices = merge_items(msrp_prices, self.msrp_prices.clone());
        self.list_prices = merge_items(list_prices, self.list_prices.clone());
        self.sale_prices = merge_sale_items(sale_prices, self.sale_prices.clone());
        self.updated_at = chrono::Utc::now();
    }
}

#[derive(Schema, Clone)]
pub struct PricingItem {
    pub price: f32,
    pub currency: String,
    pub zone: String,
}

impl PricingItem {
    fn key(&self) -> (String, String) {
        (self.zone.clone(), self.currency.clone())
    }
}

#[derive(Schema, Clone)]
pub struct SalePricingItem {
    pub price: f32,
    pub currency: String,
    pub zone: String,
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

impl SalePricingItem {
    fn key(
        &self,
    ) -> (
        String,
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) {
        (
            self.zone.clone(),
            self.currency.clone(),
            self.start,
            self.end,
        )
    }
}

impl From<SalePricingItem> for PricingItem {
    fn from(value: SalePricingItem) -> Self {
        Self {
            price: value.price,
            currency: value.currency,
            zone: value.zone,
        }
    }
}

fn get_price(currency: String, zone: String, pricing: Pricing) -> Option<PricingItem> {
    let now = chrono::Utc::now();

    let sale_price = pricing.sale_prices.into_iter().find(|x| {
        x.zone == zone
            && x.currency == currency
            && x.start.is_none_or(|v| now >= v)
            && x.end.is_none_or(|v| now < v)
    });

    if sale_price.is_some() {
        sale_price.map(|p| p.into())
    } else {
        let list_price = pricing
            .list_prices
            .into_iter()
            .find(|x| x.zone == zone && x.currency == currency);

        if list_price.is_some() {
            list_price
        } else {
            pricing
                .msrp_prices
                .into_iter()
                .find(|x| x.zone == zone && x.currency == currency)
        }
    }
}

fn merge_items(updates: Vec<PricingItem>, current: Vec<PricingItem>) -> Vec<PricingItem> {
    if updates.is_empty() {
        current
    } else if current.is_empty() {
        updates
    } else {
        let mut merge_map: HashMap<(String, String), PricingItem> = HashMap::new();

        for item in updates {
            merge_map.insert(item.key(), item);
        }

        for item in current {
            let key = item.key();
            merge_map.entry(key).or_insert(item);
        }

        merge_map.into_values().collect()
    }
}

fn merge_sale_items(
    updates: Vec<SalePricingItem>,
    current: Vec<SalePricingItem>,
) -> Vec<SalePricingItem> {
    if updates.is_empty() {
        current
    } else if current.is_empty() {
        updates
    } else {
        let mut merge_map: HashMap<
            (
                String,
                String,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
            SalePricingItem,
        > = HashMap::new();

        for item in updates {
            merge_map.insert(item.key(), item);
        }

        for item in current {
            let key = item.key();
            merge_map.entry(key).or_insert(item);
        }

        let mut values: Vec<SalePricingItem> = merge_map.into_values().collect();
        values.sort_by(|a, b| match (a.start, b.start) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        });
        values
    }
}

#[agent_definition]
trait PricingAgent {
    fn new(id: String) -> Self;

    fn get_pricing(&self) -> Option<Pricing>;

    fn get_price(&self, currency: String, zone: String) -> Option<PricingItem>;

    fn initialize_pricing(
        &mut self,
        msrp_prices: Vec<PricingItem>,
        list_prices: Vec<PricingItem>,
        sale_prices: Vec<SalePricingItem>,
    );

    fn update_pricing(
        &mut self,
        msrp_prices: Vec<PricingItem>,
        list_prices: Vec<PricingItem>,
        sale_prices: Vec<SalePricingItem>,
    );
}

struct PricingAgentImpl {
    _id: String,
    state: Option<Pricing>,
}

impl PricingAgentImpl {
    fn get_state(&mut self) -> &mut Pricing {
        self.state.get_or_insert(Pricing::new(self._id.clone()))
    }
}

#[agent_implementation]
impl PricingAgent for PricingAgentImpl {
    fn new(id: String) -> Self {
        PricingAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_price(&self, currency: String, zone: String) -> Option<PricingItem> {
        println!("Getting pricing for currency: {} zone: {}", currency, zone);
        self.state
            .clone()
            .and_then(|pricing| pricing.get_price(currency, zone))
    }

    fn get_pricing(&self) -> Option<Pricing> {
        self.state.clone()
    }

    fn initialize_pricing(
        &mut self,
        msrp_prices: Vec<PricingItem>,
        list_prices: Vec<PricingItem>,
        sale_prices: Vec<SalePricingItem>,
    ) {
        self.get_state()
            .set_prices(msrp_prices, list_prices, sale_prices);
    }

    fn update_pricing(
        &mut self,
        msrp_prices: Vec<PricingItem>,
        list_prices: Vec<PricingItem>,
        sale_prices: Vec<SalePricingItem>,
    ) {
        self.get_state()
            .update_prices(msrp_prices, list_prices, sale_prices);
    }
}
