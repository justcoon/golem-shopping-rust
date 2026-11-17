use crate::inventory::{InventoryAgentClient, ReserveQuantityError};
use chrono::{DateTime, Utc};
use golem_rust::{TransactionFailure, boxed, fallible_transaction, operation};

/// Extracts the underlying domain failure from a `TransactionFailure`.
pub(crate) fn unwrap_transaction_failure<E>(err: TransactionFailure<E>) -> E {
    match err {
        TransactionFailure::FailedAndRolledBackCompletely(e)
        | TransactionFailure::FailedAndRolledBackPartially { failure: e, .. }
        | TransactionFailure::RetryExhausted(e)
        | TransactionFailure::RetryPolicyError { failure: e, .. } => e,
    }
}

/// Reusable saga transaction for reserving inventory across multiple products.
/// Automatically compensates (releases reservations in reverse order) if any item fails.
pub async fn reserve_inventory_saga(
    items: &[(String, u32)],
    order_id: &str,
    region: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(), ReserveQuantityError> {
    if items.is_empty() {
        return Ok(());
    }

    let reserve_op = operation(
        |(product_id, order_id, region, qty, exp): (
            String,
            String,
            String,
            u32,
            Option<DateTime<Utc>>,
        )| {
            Box::pin(async move {
                let mut client = InventoryAgentClient::get(product_id.clone());
                client
                    .reserve_quantity(order_id.clone(), region, qty, exp)
                    .await?;
                Ok((product_id, order_id))
            })
        },
        |_, (product_id, order_id): (String, String)| {
            Box::pin(async move {
                let mut client = InventoryAgentClient::get(product_id);
                client.trigger_release_reservation(order_id);
                Ok(())
            })
        },
    );

    let items_vec = items.to_vec();
    let order_id = order_id.to_string();
    let region = region.to_string();

    let res = fallible_transaction(|tx| {
        let reserve_op = reserve_op.clone();
        let order_id = order_id.clone();
        let region = region.clone();
        let items_vec = items_vec.clone();
        boxed(async move {
            for (product_id, quantity) in items_vec {
                tx.execute(
                    reserve_op.clone(),
                    (
                        product_id,
                        order_id.clone(),
                        region.clone(),
                        quantity,
                        expires_at,
                    ),
                )
                .await?;
            }
            Ok(())
        })
    })
    .await;

    match res {
        Ok(()) => Ok(()),
        Err(err) => Err(unwrap_transaction_failure(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwrap_transaction_failure_variants() {
        let err1 = TransactionFailure::FailedAndRolledBackCompletely("err1".to_string());
        assert_eq!(unwrap_transaction_failure(err1), "err1");

        let err2 = TransactionFailure::FailedAndRolledBackPartially {
            failure: "err2".to_string(),
            compensation_failure: "comp_err".to_string(),
        };
        assert_eq!(unwrap_transaction_failure(err2), "err2");

        let err3 = TransactionFailure::RetryExhausted("err3".to_string());
        assert_eq!(unwrap_transaction_failure(err3), "err3");

        let err4 = TransactionFailure::RetryPolicyError {
            failure: "err4".to_string(),
            message: "policy failed".to_string(),
        };
        assert_eq!(unwrap_transaction_failure(err4), "err4");
    }
}
