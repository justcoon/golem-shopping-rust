pub mod common {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    // #[serde(rename_all = "kebab-case")]
    pub struct Address {
        pub street: String,
        pub city: String,
        pub state_or_region: String,
        pub country: String,
        pub postal_code: String,
        pub name: Option<String>,
        pub phone_number: Option<String>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    // #[serde(rename_all = "kebab-case")]
    pub struct AddItem {
        pub quantity: u32,
    }

    impl AddItem {
        pub fn new(quantity: u32) -> Self {
            Self { quantity }
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    // #[serde(rename_all = "kebab-case")]
    pub struct SetEmail {
        pub email: String,
    }

    impl SetEmail {
        pub fn new(email: String) -> Self {
            Self { email }
        }
    }
}

pub mod cart {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CheckoutDraft {
        pub order_id: String,
        pub expires_at: String,
        pub created_at: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CartWithDraft {
        pub draft: Option<CheckoutDraft>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct OrderConfirmation {
        pub user_id: String,
        pub order_id: String,
    }
}
