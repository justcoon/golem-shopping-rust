use golem_rust::{Schema, agent_definition, agent_implementation, endpoint};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct InventoryItem {
    pub region: String,
    pub total_quantity: u32,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct OrderAllocation {
    pub order_id: String,
    pub region: String,
    pub quantity: u32,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub product_id: String,
    pub items: Vec<InventoryItem>,
    pub allocations: Vec<OrderAllocation>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Inventory {
    pub fn new(product_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            product_id,
            items: vec![],
            allocations: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_with_items(product_id: String, items: Vec<InventoryItem>) -> Self {
        let now = chrono::Utc::now();
        Self {
            product_id,
            items,
            allocations: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    pub fn get_available_quantity(&self, region: String) -> Availability {
        let item = self.items.iter().find(|i| i.region == region);
        let total_quantity = item.map(|i| i.total_quantity).unwrap_or(0);
        let allocated_quantity: u32 = self
            .allocations
            .iter()
            .filter(|a| a.region == region)
            .map(|a| a.quantity)
            .sum();
        let available_quantity = total_quantity.saturating_sub(allocated_quantity);

        Availability {
            product_id: self.product_id.clone(),
            region,
            available_quantity,
            total_quantity,
            allocated_quantity,
        }
    }

    pub fn set_items(&mut self, items: Vec<InventoryItem>) {
        self.items = items;
        self.allocations.clear();
        self.updated_at = chrono::Utc::now();
    }

    pub fn update_items(&mut self, items: Vec<InventoryItem>) {
        self.items = merge_items(items, self.items.clone());
        self.updated_at = chrono::Utc::now();
    }

    pub fn reserve_quantity(
        &mut self,
        order_id: String,
        region: String,
        quantity: u32,
    ) -> Result<(), ReserveQuantityError> {
        let item = self.items.iter().find(|i| i.region == region);
        let total_quantity = item.map(|i| i.total_quantity).unwrap_or(0);

        let other_allocated: u32 = self
            .allocations
            .iter()
            .filter(|a| a.region == region && a.order_id != order_id)
            .map(|a| a.quantity)
            .sum();

        let available_for_order = total_quantity.saturating_sub(other_allocated);

        if quantity > available_for_order {
            return Err(ReserveQuantityError::InsufficientInventory(
                InsufficientInventoryError {
                    message: format!(
                        "Insufficient inventory for product {} in region {}. Requested: {}, available: {}",
                        self.product_id, region, quantity, available_for_order
                    ),
                    product_id: self.product_id.clone(),
                    region,
                    requested: quantity,
                    available: available_for_order,
                },
            ));
        }

        let now = chrono::Utc::now();
        self.allocations.retain(|a| a.order_id != order_id);

        if quantity > 0 {
            self.allocations.push(OrderAllocation {
                order_id,
                region,
                quantity,
                updated_at: now,
            });
        }

        self.updated_at = now;
        Ok(())
    }

    pub fn release_reservation(&mut self, order_id: &str) {
        let prev_len = self.allocations.len();
        self.allocations.retain(|a| a.order_id != order_id);
        if self.allocations.len() != prev_len {
            self.updated_at = chrono::Utc::now();
        }
    }

    pub fn fulfill_reservation(&mut self, order_id: &str) {
        let order_allocations: Vec<OrderAllocation> = self
            .allocations
            .iter()
            .filter(|a| a.order_id == order_id)
            .cloned()
            .collect();

        if order_allocations.is_empty() {
            return;
        }

        let mut deductions: HashMap<String, u32> = HashMap::new();
        for alloc in &order_allocations {
            *deductions.entry(alloc.region.clone()).or_insert(0) += alloc.quantity;
        }

        for item in &mut self.items {
            if let Some(deducted) = deductions.get(&item.region) {
                item.total_quantity = item.total_quantity.saturating_sub(*deducted);
            }
        }

        self.allocations.retain(|a| a.order_id != order_id);
        self.updated_at = chrono::Utc::now();
    }
}

pub fn merge_items(updates: Vec<InventoryItem>, current: Vec<InventoryItem>) -> Vec<InventoryItem> {
    let mut merge_map: HashMap<String, InventoryItem> = HashMap::new();

    for item in updates {
        merge_map.insert(item.region.clone(), item);
    }

    for item in current {
        merge_map.entry(item.region.clone()).or_insert(item);
    }

    merge_map.into_values().collect()
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Availability {
    pub product_id: String,
    pub region: String,
    pub available_quantity: u32,
    pub total_quantity: u32,
    pub allocated_quantity: u32,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct InsufficientInventoryError {
    pub message: String,
    pub product_id: String,
    pub region: String,
    pub requested: u32,
    pub available: u32,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct InventoryNotFoundError {
    pub message: String,
    pub product_id: String,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ReserveQuantityError {
    InsufficientInventory(InsufficientInventoryError),
    InventoryNotFound(InventoryNotFoundError),
}

#[agent_definition(mount = "/v1/inventory/{id}")]
pub trait InventoryAgent {
    fn new(id: String) -> Self;

    #[endpoint(get = "/")]
    fn get_inventory(&self) -> Option<Inventory>;

    #[endpoint(get = "/availability?region={region}")]
    fn get_available_quantity(&self, region: String) -> Availability;

    #[endpoint(post = "/")]
    fn initialize_inventory(&mut self, items: Vec<InventoryItem>);

    #[endpoint(put = "/")]
    fn update_inventory(&mut self, items: Vec<InventoryItem>);

    fn reserve_quantity(
        &mut self,
        order_id: String,
        region: String,
        quantity: u32,
    ) -> Result<(), ReserveQuantityError>;

    fn release_reservation(&mut self, order_id: String);

    fn fulfill_reservation(&mut self, order_id: String);
}

pub struct InventoryAgentImpl {
    _id: String,
    state: Option<Inventory>,
}

#[agent_implementation]
impl InventoryAgent for InventoryAgentImpl {
    fn new(id: String) -> Self {
        InventoryAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_inventory(&self) -> Option<Inventory> {
        self.state.clone()
    }

    fn get_available_quantity(&self, region: String) -> Availability {
        info!(
            "Getting available quantity for product {} region: {}",
            self._id, region
        );
        match &self.state {
            Some(inventory) => inventory.get_available_quantity(region),
            None => Availability {
                product_id: self._id.clone(),
                region,
                available_quantity: 0,
                total_quantity: 0,
                allocated_quantity: 0,
            },
        }
    }

    fn initialize_inventory(&mut self, items: Vec<InventoryItem>) {
        info!(
            "Initializing inventory for product {} with {} regional items",
            self._id,
            items.len()
        );
        self.state = Some(Inventory::new_with_items(self._id.clone(), items));
    }

    fn update_inventory(&mut self, items: Vec<InventoryItem>) {
        info!(
            "Updating inventory for product {} with {} regional items",
            self._id,
            items.len()
        );
        if let Some(inventory) = self.state.as_mut() {
            inventory.update_items(items);
        }
    }

    fn reserve_quantity(
        &mut self,
        order_id: String,
        region: String,
        quantity: u32,
    ) -> Result<(), ReserveQuantityError> {
        info!(
            "Reserving quantity {} for order {} in region {} on product {}",
            quantity, order_id, region, self._id
        );
        match self.state.as_mut() {
            Some(inventory) => inventory.reserve_quantity(order_id, region, quantity),
            None => Err(ReserveQuantityError::InventoryNotFound(
                InventoryNotFoundError {
                    message: format!("Inventory for product {} not found", self._id),
                    product_id: self._id.clone(),
                },
            )),
        }
    }

    fn release_reservation(&mut self, order_id: String) {
        info!(
            "Releasing inventory reservation for order {} on product {}",
            order_id, self._id
        );
        if let Some(inventory) = self.state.as_mut() {
            inventory.release_reservation(&order_id);
        }
    }

    fn fulfill_reservation(&mut self, order_id: String) {
        info!(
            "Fulfilling inventory reservation for order {} on product {}",
            order_id, self._id
        );
        if let Some(inventory) = self.state.as_mut() {
            inventory.fulfill_reservation(&order_id);
        }
    }

    async fn load_snapshot(
        bytes: Vec<u8>,
        context: golem_rust::agentic::SnapshotRestoreContext,
    ) -> Result<Self, String> {
        let data: Option<Inventory> = crate::common::snapshot::deserialize(&bytes)?;
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
    fn test_availability_calculation() {
        let items = vec![
            InventoryItem {
                region: "global".to_string(),
                total_quantity: 10,
            },
            InventoryItem {
                region: "us".to_string(),
                total_quantity: 5,
            },
        ];
        let mut inv = Inventory::new_with_items("p1".to_string(), items);

        let avail = inv.get_available_quantity("global".to_string());
        assert_eq!(avail.available_quantity, 10);
        assert_eq!(avail.total_quantity, 10);
        assert_eq!(avail.allocated_quantity, 0);

        assert!(
            inv.reserve_quantity("ord-1".to_string(), "global".to_string(), 4)
                .is_ok()
        );
        let avail_after = inv.get_available_quantity("global".to_string());
        assert_eq!(avail_after.available_quantity, 6);
        assert_eq!(avail_after.allocated_quantity, 4);

        // Region isolation
        let avail_us = inv.get_available_quantity("us".to_string());
        assert_eq!(avail_us.available_quantity, 5);
        assert_eq!(avail_us.allocated_quantity, 0);
    }

    #[test]
    fn test_reserve_over_limit_fails() {
        let items = vec![InventoryItem {
            region: "global".to_string(),
            total_quantity: 5,
        }];
        let mut inv = Inventory::new_with_items("p1".to_string(), items);

        let res = inv.reserve_quantity("ord-1".to_string(), "global".to_string(), 6);
        assert!(matches!(
            res,
            Err(ReserveQuantityError::InsufficientInventory(_))
        ));

        // Available quantity unchanged
        let avail = inv.get_available_quantity("global".to_string());
        assert_eq!(avail.available_quantity, 5);
    }

    #[test]
    fn test_release_and_fulfill() {
        let items = vec![InventoryItem {
            region: "global".to_string(),
            total_quantity: 10,
        }];
        let mut inv = Inventory::new_with_items("p1".to_string(), items);

        assert!(
            inv.reserve_quantity("ord-1".to_string(), "global".to_string(), 3)
                .is_ok()
        );
        assert!(
            inv.reserve_quantity("ord-2".to_string(), "global".to_string(), 4)
                .is_ok()
        );

        let avail = inv.get_available_quantity("global".to_string());
        assert_eq!(avail.available_quantity, 3);
        assert_eq!(avail.allocated_quantity, 7);

        // Release ord-1
        inv.release_reservation("ord-1");
        let avail2 = inv.get_available_quantity("global".to_string());
        assert_eq!(avail2.available_quantity, 6);
        assert_eq!(avail2.allocated_quantity, 4);

        // Fulfill ord-2 (shipped)
        inv.fulfill_reservation("ord-2");
        let avail3 = inv.get_available_quantity("global".to_string());
        assert_eq!(avail3.available_quantity, 6);
        assert_eq!(avail3.total_quantity, 6);
        assert_eq!(avail3.allocated_quantity, 0);
    }
}
