//! The Android bridge against a scripted Billing host: every raw code maps to its documented
//! kind, and finishing acknowledges while consuming consumes.

use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex, OnceLock, PoisonError},
};

use nsh_purchases::android::{self, BillingCode, BillingHost, BillingPurchase};
use nsh_purchases::{ErrorKind, PurchaseOptions, Purchases};

fn run<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(future)
}

/// The tests take turns: the process has the one host, answering from one scripted fixture.
static TURNS: Mutex<()> = Mutex::new(());

/// Every BillingResponseCode the bridge documents, with the stable kind it becomes.
const BILLING_MAP: [(i64, &str); 12] = [
    (android::SERVICE_TIMEOUT, "ERR_PURCHASES_NETWORK"),
    (
        android::FEATURE_NOT_SUPPORTED,
        "ERR_PURCHASES_NOT_AVAILABLE",
    ),
    (android::SERVICE_DISCONNECTED, "ERR_PURCHASES_NETWORK"),
    (android::USER_CANCELED, "ERR_PURCHASES_CANCELLED"),
    (android::SERVICE_UNAVAILABLE, "ERR_PURCHASES_NETWORK"),
    (android::BILLING_UNAVAILABLE, "ERR_PURCHASES_NOT_AVAILABLE"),
    (android::ITEM_UNAVAILABLE, "ERR_PURCHASES_PRODUCT_NOT_FOUND"),
    (android::DEVELOPER_ERROR, "ERR_PURCHASES_INVALID_ARGUMENT"),
    (android::ERROR, "ERR_PURCHASES_BACKEND"),
    (android::ITEM_ALREADY_OWNED, "ERR_PURCHASES_ALREADY_OWNED"),
    (android::ITEM_NOT_OWNED, "ERR_PURCHASES_NOT_OWNED"),
    (android::NETWORK_ERROR, "ERR_PURCHASES_NETWORK"),
];

/// The scripted answers the shared host hands out. A queue with nothing queued answers the default.
#[derive(Default)]
struct Fixture {
    products_errors: Arc<Mutex<VecDeque<i64>>>,
    purchase_answers: Arc<Mutex<VecDeque<Result<BillingPurchase, BillingCode>>>>,
    unfinished: Arc<Mutex<Vec<BillingPurchase>>>,
    acknowledged: Arc<Mutex<Vec<String>>>,
    consumed: Arc<Mutex<Vec<String>>>,
}

fn lock<T>(slot: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Fixture {
    /// Makes the next products call answer with the raw store code.
    fn push_products_error(&self, raw: i64) {
        lock(&self.products_errors).push_back(raw);
    }

    /// Makes the next purchase answer with `answer`.
    fn push_purchase_answer(&self, answer: Result<BillingPurchase, BillingCode>) {
        lock(&self.purchase_answers).push_back(answer);
    }

    /// Sets what queryPurchases reports as still needing action.
    fn set_unfinished(&self, waiting: Vec<BillingPurchase>) {
        *lock(&self.unfinished) = waiting;
    }

    /// The tokens acknowledge was asked to act on, in order.
    fn acknowledged(&self) -> Vec<String> {
        lock(&self.acknowledged).clone()
    }

    /// The tokens consume was asked to act on, in order.
    fn consumed(&self) -> Vec<String> {
        lock(&self.consumed).clone()
    }
}

/// A purchased raw transaction, as queryPurchases reports it, with its kind riding along.
fn purchased_raw(product_id: &str, kind: i64) -> BillingPurchase {
    BillingPurchase {
        purchase_token: format!("token-{product_id}"),
        product_id: product_id.to_owned(),
        state: android::PURCHASE_STATE_PURCHASED,
        kind,
        receipt: format!("{{\"productId\":\"{product_id}\"}}"),
        purchased_at_ms: Some(1_700_000_000_000),
    }
}

/// The one host this process registers, answering from the shared fixture.
fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let fixture = Fixture::default();
        android::install_host(BillingHost {
            connect: Arc::new(|_unit: ()| Box::pin(async { Ok(()) })),
            products: {
                let errors = Arc::clone(&fixture.products_errors);
                Arc::new(move |_ids: Vec<String>| {
                    let raw = lock(&errors).pop_front();
                    Box::pin(async move {
                        match raw {
                            Some(raw) => Err(BillingCode {
                                raw,
                                message: "scripted".to_owned(),
                            }),
                            None => Ok(Vec::new()),
                        }
                    })
                })
            },
            purchase: {
                let answers = Arc::clone(&fixture.purchase_answers);
                Arc::new(move |_request: android::PurchaseRequest| {
                    let answer = lock(&answers).pop_front().unwrap_or_else(|| {
                        Ok(purchased_raw("coins_100", android::KIND_CONSUMABLE))
                    });
                    Box::pin(async move { answer })
                })
            },
            acknowledge: {
                let tokens = Arc::clone(&fixture.acknowledged);
                Arc::new(move |token: String| {
                    lock(&tokens).push(token);
                    Box::pin(async { Ok(()) })
                })
            },
            consume: {
                let tokens = Arc::clone(&fixture.consumed);
                Arc::new(move |token: String| {
                    lock(&tokens).push(token);
                    Box::pin(async { Ok(()) })
                })
            },
            restore: Arc::new(|_unit: ()| Box::pin(async { Ok(Vec::<BillingPurchase>::new()) })),
            unfinished: {
                let waiting = Arc::clone(&fixture.unfinished);
                Arc::new(move |_unit: ()| {
                    let waiting = lock(&waiting).clone();
                    Box::pin(async move { Ok(waiting) })
                })
            },
            entitlements: Arc::new(|_unit: ()| Box::pin(async { Ok(Vec::new()) })),
            country_code: Arc::new(|_unit: ()| Box::pin(async { Ok("US".to_owned()) })),
            cancel: || {},
        });
        fixture
    })
}

#[test]
fn every_billing_raw_code_maps_to_its_documented_code() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases =
        Purchases::with_backend(android::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();
        for (raw, code) in BILLING_MAP {
            fixture.push_products_error(raw);
            let error = purchases
                .products(&["pro_unlock".to_owned()])
                .await
                .unwrap_err();
            assert_eq!(error.code(), code, "raw {raw} maps to its documented code");
        }
    });
}

#[test]
fn unknown_billing_raw_codes_are_backend() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases =
        Purchases::with_backend(android::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();
        for raw in [99_i64, -7] {
            fixture.push_products_error(raw);
            let error = purchases
                .products(&["pro_unlock".to_owned()])
                .await
                .unwrap_err();
            assert_eq!(
                error.kind(),
                ErrorKind::Backend,
                "raw {raw} is not documented"
            );
            assert_eq!(
                error.status(),
                Some(raw),
                "the raw code is kept for the app to read"
            );
        }
    });
}

#[test]
fn finishing_acknowledges_and_consuming_consumes() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases =
        Purchases::with_backend(android::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();

        // The consumable buys, and consuming it reaches the store's consume.
        fixture.push_purchase_answer(Ok(purchased_raw("coins_100", android::KIND_CONSUMABLE)));
        let coins = purchases
            .purchase("coins_100", PurchaseOptions::default())
            .await
            .unwrap();
        fixture.set_unfinished(vec![purchased_raw("coins_100", android::KIND_CONSUMABLE)]);
        purchases.consume(&coins).await.unwrap();
        assert_eq!(
            fixture.consumed(),
            ["token-coins_100".to_owned()],
            "consuming consumes the token"
        );
        assert!(
            fixture.acknowledged().is_empty(),
            "a consumable is not acknowledged"
        );

        // The non-consumable buys, and finishing it reaches the store's acknowledge.
        fixture.push_purchase_answer(Ok(purchased_raw(
            "pro_unlock",
            android::KIND_NON_CONSUMABLE,
        )));
        let unlock = purchases
            .purchase("pro_unlock", PurchaseOptions::default())
            .await
            .unwrap();
        purchases.finish(&unlock).await.unwrap();
        assert_eq!(
            fixture.acknowledged(),
            ["token-pro_unlock".to_owned()],
            "finishing acknowledges the token once"
        );

        // A non-consumable never reaches the store's consume.
        fixture.set_unfinished(vec![purchased_raw(
            "pro_unlock",
            android::KIND_NON_CONSUMABLE,
        )]);
        let error = purchases.consume(&unlock).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
        assert_eq!(
            fixture.consumed().len(),
            1,
            "only the consumable reached consume"
        );
    });
}

#[test]
fn unknown_purchase_states_are_backend() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases =
        Purchases::with_backend(android::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();
        // UNSPECIFIED (0) and undocumented numbers name no action, so the number rides along.
        for state in [0_i64, 7] {
            let mut raw = purchased_raw("coins_100", android::KIND_CONSUMABLE);
            raw.state = state;
            fixture.push_purchase_answer(Ok(raw));
            let error = purchases
                .purchase("coins_100", PurchaseOptions::default())
                .await
                .unwrap_err();
            assert_eq!(
                error.kind(),
                ErrorKind::Backend,
                "purchase state {state} is not documented"
            );
            assert_eq!(
                error.status(),
                Some(state),
                "the raw state is kept for the app to read"
            );
        }
    });
}
