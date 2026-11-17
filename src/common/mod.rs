use golem_rust::Schema;
use serde::{Deserialize, Serialize};

pub mod errors;
pub mod query;

pub use errors::*;

pub const CURRENCY_DEFAULT: &str = "USD";
pub const PRICING_REGION_DEFAULT: &str = "global";
pub const INVENTORY_REGION_DEFAULT: &str = "global";
pub const CHECKOUT_RESERVATION_TTL_SECONDS: i64 = 600;

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state_or_region: String,
    pub country: String,
    pub postal_code: String,
    pub name: Option<String>,
    pub phone_number: Option<String>,
}

pub(crate) mod snapshot {
    use serde::{Serialize, de};

    pub const SERIALIZATION_VERSION_V1: u8 = 1u8;

    pub(crate) fn serialize<T>(value: &T) -> Result<Vec<u8>, String>
    where
        T: ?Sized + Serialize,
    {
        let data = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;

        let mut result = vec![SERIALIZATION_VERSION_V1];
        result.extend(data);

        Ok(result)
    }

    pub(crate) fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T, String>
    where
        T: de::Deserialize<'a>,
    {
        let (&version, data) = bytes
            .split_first()
            .ok_or_else(|| "Empty snapshot data".to_string())?;

        match version {
            SERIALIZATION_VERSION_V1 => {
                let value: T = serde_json::from_slice(data).map_err(|err| err.to_string())?;

                Ok(value)
            }
            _ => Err("Unsupported serialization version".to_string()),
        }
    }

    /// Extracts the agent ID string from the snapshot restore context parameters.
    pub(crate) fn extract_agent_id(
        context: golem_rust::agentic::SnapshotRestoreContext,
    ) -> Result<String, String> {
        let mut input = golem_rust::agentic::DirectAgentInput::new(context.parameters)
            .map_err(|error| error.to_string())?;
        let param: String = input.take().map_err(|error| error.to_string())?;
        input.finish().map_err(|error| error.to_string())?;
        Ok(param)
    }
}
