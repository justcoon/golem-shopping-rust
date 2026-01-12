pub mod cart;
pub mod common;
pub mod order;
pub mod pricing;
pub mod product;
pub mod product_search;
pub mod shopping_assistant;

mod snapshot {
    use serde::{de, Serialize};

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
        let (version, data) = bytes.split_at(1);

        match version[0] {
            SERIALIZATION_VERSION_V1 => {
                let value: T = serde_json::from_slice(data).map_err(|err| err.to_string())?;

                Ok(value)
            }
            _ => Err("Unsupported serialization version".to_string()),
        }
    }
}
