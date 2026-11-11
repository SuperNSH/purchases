//! `Purchases` through the fake backend: one purchase at a time, one answer per call, cancellation.

use std::{future::Future, sync::Arc, time::Duration};

use nsh_purchases::{
    Entitlement, ErrorKind, FakeBackend, Purchase, PurchaseOptions, PurchaseState, Purchases,
    PurchasesError,
};

fn run<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(future)
}

/// Yields until `ready` holds, so a spawned call has reached the store.
async fn settle(ready: impl Fn() -> bool) {
    for _ in 0..1000 {
        if ready() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("the store never saw the call");
}

/// A sandbox purchase of `product_id`, as the store would report it.
fn store_purchase(product_id: &str, state: PurchaseState) -> Purchase {
    Purchase {
        id: format!("tx-{product_id}"),
        product_id: product_id.to_owned(),
        state,
        receipt: format!("sandbox-receipt-{product_id}"),
        purchased_at_ms: Some(1_700_000_000_000),
    }
}

/// Starts a purchase on its own task, and waits until the fake holds it in flight.
async fn start_purchase(
    purchases: &Purchases,
    fake: &FakeBackend,
    product_id: &str,
    options: PurchaseOptions,
) -> tokio::task::JoinHandle<Result<Purchase, PurchasesError>> {
    let started = fake.purchases_started();
    let call = purchases.clone();
    let product_id = product_id.to_owned();
    let handle = tokio::spawn(async move { call.purchase(&product_id, options).await });
    settle(|| fake.purchases_started() > started).await;
    handle
}

/// Initializes the store over a fake backend, ready for a call.
fn store(fake: &FakeBackend) -> Purchases {
    let purchases = Purchases::with_backend(Arc::new(fake.clone()));
    run(async {
        purchases.initialize().await.unwrap();
    });
    purchases
}

/// The options with the opt-in timeout. `PurchaseOptions` is non-exhaustive, so fields are set one by one.
#[allow(clippy::field_reassign_with_default)]
fn options_with_timeout(timeout: Duration) -> PurchaseOptions {
    let mut options = PurchaseOptions::default();
    options.timeout = Some(timeout);
    options
}

/// The options with a quantity, for the consumable checks.
#[allow(clippy::field_reassign_with_default)]
fn options_with_quantity(quantity: u32) -> PurchaseOptions {
    let mut options = PurchaseOptions::default();
    options.quantity = quantity;
    options
}

const ALL_KINDS: [(ErrorKind, &str); 13] = [
    (ErrorKind::NotAvailable, "ERR_PURCHASES_NOT_AVAILABLE"),
    (ErrorKind::NotInitialized, "ERR_PURCHASES_NOT_INITIALIZED"),
    (ErrorKind::InProgress, "ERR_PURCHASES_IN_PROGRESS"),
    (ErrorKind::Timeout, "ERR_PURCHASES_TIMEOUT"),
    (ErrorKind::Cancelled, "ERR_PURCHASES_CANCELLED"),
    (
        ErrorKind::ProductNotFound,
        "ERR_PURCHASES_PRODUCT_NOT_FOUND",
    ),
    (ErrorKind::AlreadyOwned, "ERR_PURCHASES_ALREADY_OWNED"),
    (ErrorKind::NotOwned, "ERR_PURCHASES_NOT_OWNED"),
    (ErrorKind::PaymentInvalid, "ERR_PURCHASES_PAYMENT_INVALID"),
    (ErrorKind::NotAllowed, "ERR_PURCHASES_NOT_ALLOWED"),
    (ErrorKind::Network, "ERR_PURCHASES_NETWORK"),
    (ErrorKind::InvalidArgument, "ERR_PURCHASES_INVALID_ARGUMENT"),
    (ErrorKind::Backend, "ERR_PURCHASES_BACKEND"),
];

#[test]
fn purchase_twice_in_parallel_is_in_progress() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let first =
            start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        let second = purchases
            .purchase("pro_unlock", PurchaseOptions::default())
            .await
            .unwrap_err();
        assert_eq!(second.kind(), ErrorKind::InProgress);
        assert_eq!(
            fake.purchases_started(),
            1,
            "the refused call never reached the store"
        );

        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        assert_eq!(first.await.unwrap().unwrap().product_id, "coins_100");
    });
}

#[test]
fn cancel_resolves_the_running_purchase_as_cancelled() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let call =
            start_purchase(&purchases, &fake, "pro_unlock", PurchaseOptions::default()).await;
        assert!(
            purchases.cancel().await.unwrap(),
            "a purchase was running to cancel"
        );
        let error = call.await.unwrap().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Cancelled);
        assert_eq!(fake.cancels(), 1);

        // The store answers after the cancel; the late answer must be dropped whole.
        assert!(fake.finish_purchase(Ok(store_purchase("pro_unlock", PurchaseState::Purchased))));
        let next = start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        assert_eq!(next.await.unwrap().unwrap().product_id, "coins_100");
    });
}

#[test]
fn a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        tokio::time::pause();
        let first = start_purchase(
            &purchases,
            &fake,
            "coins_100",
            options_with_timeout(Duration::from_millis(50)),
        )
        .await;
        tokio::time::advance(Duration::from_millis(100)).await;
        let error = first.await.unwrap().unwrap_err();
        assert_eq!(error.code(), "ERR_PURCHASES_TIMEOUT");

        // The timeout freed the gate: the next purchase runs while the first is still unanswered.
        let second =
            start_purchase(&purchases, &fake, "pro_unlock", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("pro_unlock", PurchaseState::Purchased))));
        assert_eq!(second.await.unwrap().unwrap().product_id, "pro_unlock");
    });
}

#[test]
fn a_zero_timeout_is_an_invalid_argument() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let error = purchases
            .purchase("coins_100", options_with_timeout(Duration::ZERO))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
        assert_eq!(
            fake.purchases_started(),
            0,
            "refused before the store was asked"
        );
    });
}

#[test]
fn calls_before_initialize_are_not_initialized() {
    let fake = FakeBackend::new();
    let purchases = Purchases::with_backend(Arc::new(fake.clone()));
    run(async {
        let error = purchases
            .products(&["pro_unlock".to_owned()])
            .await
            .unwrap_err();
        assert_eq!(error.code(), "ERR_PURCHASES_NOT_INITIALIZED");
        let error = purchases
            .purchase("coins_100", PurchaseOptions::default())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotInitialized);
        let error = purchases.restore().await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotInitialized);

        // is_available is the one call allowed before initialize.
        assert!(purchases.is_available().await.unwrap());
    });
}

#[test]
fn initialize_is_idempotent() {
    let fake = FakeBackend::new();
    let purchases = Purchases::with_backend(Arc::new(fake.clone()));
    run(async {
        purchases.initialize().await.unwrap();
        purchases.initialize().await.unwrap();
        let products = purchases
            .products(&["pro_unlock".to_owned(), "coins_100".to_owned()])
            .await
            .unwrap();
        assert_eq!(
            products.len(),
            2,
            "the second initialize left the store usable"
        );
    });
}

#[test]
fn empty_product_ids_are_an_invalid_argument() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let error = purchases.products(&[]).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    });
}

#[test]
fn a_zero_quantity_is_an_invalid_argument() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let error = purchases
            .purchase("coins_100", options_with_quantity(0))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
        assert_eq!(
            fake.purchases_started(),
            0,
            "refused before the store was asked"
        );
    });
}

#[test]
fn unknown_product_ids_are_dropped_from_the_answer() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let products = purchases
            .products(&[
                "pro_unlock".to_owned(),
                "not_a_product".to_owned(),
                "pro_monthly".to_owned(),
            ])
            .await
            .unwrap();
        let ids: Vec<&str> = products.iter().map(|product| product.id.as_str()).collect();
        assert_eq!(ids, ["pro_unlock", "pro_monthly"]);
    });
}

#[test]
fn purchasing_an_unknown_product_is_product_not_found() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let error = purchases
            .purchase("not_a_product", PurchaseOptions::default())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ProductNotFound);
        assert_eq!(
            fake.purchases_started(),
            0,
            "the store refused the unknown id"
        );
    });
}

#[test]
fn a_purchase_resolves_exactly_once() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let call = start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        let purchase = call.await.unwrap().unwrap();
        assert_eq!(purchase.product_id, "coins_100");

        // A second answer to a finished purchase has nothing to resolve.
        assert!(!fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
    });
}

#[test]
fn a_pending_purchase_resolves_with_a_pending_state() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let call =
            start_purchase(&purchases, &fake, "pro_monthly", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("pro_monthly", PurchaseState::Pending))));
        // Pending is not a failure: the call resolves Ok, awaiting approval.
        let purchase = call.await.unwrap().unwrap();
        assert_eq!(purchase.state, PurchaseState::Pending);
    });
}

#[test]
fn a_cancelled_purchase_resolves_with_the_cancelled_code() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let call = start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Err(PurchasesError::new(ErrorKind::Cancelled))));
        let error = call.await.unwrap().unwrap_err();
        assert_eq!(error.code(), "ERR_PURCHASES_CANCELLED");
    });
}

#[test]
fn an_already_owned_product_is_already_owned() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        fake.set_next_purchase_error(PurchasesError::new(ErrorKind::AlreadyOwned));
        let error = purchases
            .purchase("pro_unlock", PurchaseOptions::default())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::AlreadyOwned);

        // The scripted error was consumed once, and the gate freed: the next purchase runs.
        let next = start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        assert_eq!(next.await.unwrap().unwrap().product_id, "coins_100");
    });
}

#[test]
fn finishing_marks_the_purchase_finished_once() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let call = start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        let purchase = call.await.unwrap().unwrap();
        purchases.finish(&purchase).await.unwrap();
        assert!(
            purchases.pending().await.unwrap().is_empty(),
            "a finished purchase leaves the unfinished queue"
        );
    });
}

#[test]
fn finishing_an_unknown_or_finished_purchase_is_not_owned() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let unknown = store_purchase("not_in_the_store", PurchaseState::Purchased);
        let error = purchases.finish(&unknown).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotOwned);

        let call =
            start_purchase(&purchases, &fake, "pro_unlock", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("pro_unlock", PurchaseState::Purchased))));
        let purchase = call.await.unwrap().unwrap();
        purchases.finish(&purchase).await.unwrap();
        let error = purchases.finish(&purchase).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotOwned);
    });
}

#[test]
fn consuming_a_non_consumable_is_an_invalid_argument() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let unlock =
            start_purchase(&purchases, &fake, "pro_unlock", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("pro_unlock", PurchaseState::Purchased))));
        let pro_unlock = unlock.await.unwrap().unwrap();
        let error = purchases.consume(&pro_unlock).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);

        let coins =
            start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        let coins_100 = coins.await.unwrap().unwrap();
        purchases.consume(&coins_100).await.unwrap();
    });
}

#[test]
fn restore_returns_purchased_and_restored_purchases() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        fake.set_restore_answer(Ok(vec![
            store_purchase("pro_unlock", PurchaseState::Restored),
            store_purchase("coins_100", PurchaseState::Purchased),
        ]));
        let restored = purchases.restore().await.unwrap();
        let by_id = |purchase: &Purchase, id: &str| purchase.product_id == id;
        let pro_unlock_row = restored
            .iter()
            .find(|purchase| by_id(purchase, "pro_unlock"));
        assert_eq!(
            pro_unlock_row.unwrap().state,
            PurchaseState::Restored,
            "an owned non-consumable comes back restored"
        );
        assert!(
            !pro_unlock_row.unwrap().receipt.is_empty(),
            "the receipt rides along"
        );
        let coins_row = restored
            .iter()
            .find(|purchase| by_id(purchase, "coins_100"));
        assert_eq!(
            coins_row.unwrap().state,
            PurchaseState::Purchased,
            "a purchase the store never finished comes back still purchased"
        );
    });
}

#[test]
fn restore_with_no_purchases_is_empty() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        assert!(purchases.restore().await.unwrap().is_empty());
    });
}

#[test]
fn pending_returns_unfinished_purchases() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let call = start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        let purchase = call.await.unwrap().unwrap();
        let waiting = purchases.pending().await.unwrap();
        assert_eq!(waiting.len(), 1, "the unfinished purchase is queued");
        assert_eq!(waiting[0].product_id, purchase.product_id);

        purchases.finish(&purchase).await.unwrap();
        assert!(purchases.pending().await.unwrap().is_empty());
    });
}

#[test]
fn entitlements_exclude_consumed_consumables() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let unlock =
            start_purchase(&purchases, &fake, "pro_unlock", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("pro_unlock", PurchaseState::Purchased))));
        let pro_unlock = unlock.await.unwrap().unwrap();
        purchases.finish(&pro_unlock).await.unwrap();

        let coins =
            start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        let coins_100 = coins.await.unwrap().unwrap();
        purchases.consume(&coins_100).await.unwrap();

        let entitlements = purchases.entitlements().await.unwrap();
        let ids: Vec<&str> = entitlements
            .iter()
            .map(|entitlement| entitlement.product_id.as_str())
            .collect();
        assert!(ids.contains(&"pro_unlock"), "the unlock is entitled");
        assert!(
            !ids.contains(&"coins_100"),
            "a consumed consumable grants nothing"
        );
    });
}

#[test]
fn entitlements_respect_expiry() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        // The store's own records: a lapsed monthly reports not active, with the expiry it lapsed at.
        fake.set_entitlements(Ok(vec![
            Entitlement {
                product_id: "pro_unlock".to_owned(),
                active: true,
                expires_at_ms: None,
            },
            Entitlement {
                product_id: "pro_monthly".to_owned(),
                active: false,
                expires_at_ms: Some(1),
            },
        ]));
        let entitlements = purchases.entitlements().await.unwrap();
        let by_id = |entitlement: &Entitlement, id: &str| entitlement.product_id == id;
        let unlock = entitlements
            .iter()
            .find(|entitlement| by_id(entitlement, "pro_unlock"))
            .unwrap();
        assert!(unlock.active, "a non-expiring unlock stays active");
        assert_eq!(unlock.expires_at_ms, None);
        let monthly = entitlements
            .iter()
            .find(|entitlement| by_id(entitlement, "pro_monthly"))
            .unwrap();
        assert!(!monthly.active, "an expired subscription is inactive");
        assert_eq!(monthly.expires_at_ms, Some(1));
    });
}

#[test]
fn an_unsupported_target_is_not_available() {
    // No host was registered in this process, so new() lands on the unsupported backend.
    let purchases = Purchases::new();
    run(async {
        assert!(!purchases.is_available().await.unwrap());
        let error = purchases.initialize().await.unwrap_err();
        assert_eq!(error.code(), "ERR_PURCHASES_NOT_AVAILABLE");
    });
}

#[test]
fn concurrent_non_purchase_calls_do_not_claim_the_gate() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let call = start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;

        // Queries answer while the purchase runs; none of them claims the purchase gate.
        let products = purchases
            .products(&["pro_unlock".to_owned()])
            .await
            .unwrap();
        assert_eq!(products.len(), 1);
        purchases.entitlements().await.unwrap();
        purchases.pending().await.unwrap();
        purchases.country_code().await.unwrap();
        assert_eq!(
            fake.purchases_started(),
            1,
            "no query started a second purchase"
        );

        assert!(fake.finish_purchase(Ok(store_purchase("coins_100", PurchaseState::Purchased))));
        assert_eq!(call.await.unwrap().unwrap().product_id, "coins_100");
    });
}

#[test]
fn a_dropped_purchase_call_cancels_the_store_request() {
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        let call = start_purchase(&purchases, &fake, "coins_100", PurchaseOptions::default()).await;
        call.abort();
        let _ = call.await;
        assert_eq!(
            fake.cancels(),
            1,
            "the caller going away cancels the store request"
        );
    });
}

#[test]
fn every_error_code_round_trips() {
    for (kind, code) in ALL_KINDS {
        let error = PurchasesError::new(kind);
        assert_eq!(error.code(), code, "{kind:?} carries its stable code");
        assert_eq!(kind.code(), code);
        assert_eq!(kind.to_string(), code);
        assert!(!kind.message().is_empty());
        assert!(!error.to_string().is_empty());
    }

    // Every kind also reaches the caller with its code through the store.
    let fake = FakeBackend::new();
    let purchases = store(&fake);
    run(async {
        for (kind, code) in ALL_KINDS {
            fake.set_next_purchase_error(PurchasesError::new(kind));
            let error = purchases
                .purchase("coins_100", PurchaseOptions::default())
                .await
                .unwrap_err();
            assert_eq!(error.code(), code, "{kind:?} reaches the caller");
        }
    });
}
