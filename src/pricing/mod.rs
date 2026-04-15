use golem_rust::{Schema, agent_definition, agent_implementation, endpoint};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Schema, Clone, Serialize, Deserialize)]
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

    fn get_price(&self, currency: String, region: String) -> Option<PricingItem> {
        get_price(currency, region, self.clone())
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

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct PricingItem {
    pub price: f32,
    pub currency: String,
    pub region: String,
}

impl PricingItem {
    fn key(&self) -> (String, String) {
        (self.region.clone(), self.currency.clone())
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct SalePricingItem {
    pub price: f32,
    pub currency: String,
    pub region: String,
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct PricingRequest {
    pub msrp_prices: Vec<PricingItem>,
    pub list_prices: Vec<PricingItem>,
    pub sale_prices: Vec<SalePricingItem>,
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
            self.region.clone(),
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
            region: value.region,
        }
    }
}

fn get_price(currency: String, region: String, pricing: Pricing) -> Option<PricingItem> {
    let now = chrono::Utc::now();

    let sale_price = pricing.sale_prices.into_iter().find(|x| {
        x.region == region
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
            .find(|x| x.region == region && x.currency == currency);

        if list_price.is_some() {
            list_price
        } else {
            pricing
                .msrp_prices
                .into_iter()
                .find(|x| x.region == region && x.currency == currency)
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

#[agent_definition(mount = "/v1/pricing/{id}")]
trait PricingAgent {
    fn new(id: String) -> Self;

    #[endpoint(get = "/")]
    fn get_pricing(&self) -> Option<Pricing>;

    fn get_price(&self, currency: String, region: String) -> Option<PricingItem>;

    #[endpoint(post = "/")]
    fn initialize_pricing(&mut self, request: PricingRequest);

    #[endpoint(put = "/")]
    fn update_pricing(&mut self, request: PricingRequest);
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

    fn get_price(&self, currency: String, region: String) -> Option<PricingItem> {
        info!(
            "Getting pricing for currency: {} region: {}",
            currency, region
        );
        self.state
            .clone()
            .and_then(|pricing| pricing.get_price(currency, region))
    }

    fn get_pricing(&self) -> Option<Pricing> {
        self.state.clone()
    }

    fn initialize_pricing(&mut self, request: PricingRequest) {
        self.get_state().set_prices(
            request.msrp_prices,
            request.list_prices,
            request.sale_prices,
        );
    }

    fn update_pricing(&mut self, request: PricingRequest) {
        self.get_state().update_prices(
            request.msrp_prices,
            request.list_prices,
            request.sale_prices,
        );
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: Option<Pricing> = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}
