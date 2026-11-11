//! The Apple bridge against a scripted StoreKit host: every raw code maps to its documented kind.

use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex, OnceLock, PoisonError},
};

use nsh_purchases::apple::{
    self, PurchaseRequest, StoreKitCode, StoreKitHost, StoreKitProduct, StoreKitPurchase,
};
use nsh_purchases::{ErrorKind, PurchaseOptions, PurchaseState, Purchases};

fn run<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(future)
}

/// The tests take turns: the process has the one host, answering from one scripted fixture.
static TURNS: Mutex<()> = Mutex::new(());

/// The raw SKError codes with the stable kinds this crate documents for them, from StoreKit's
/// SKError.h numbering: unknown (0), clientInvalid (1), paymentCancelled (2), paymentInvalid (3),
/// paymentNotAllowed (4), and the three cloud-service cases (5-7).
const STOREKIT_MAP: [(i64, &str); 8] = [
    (0, "ERR_PURCHASES_BACKEND"),
    (1, "ERR_PURCHASES_INVALID_ARGUMENT"),
    (2, "ERR_PURCHASES_CANCELLED"),
    (3, "ERR_PURCHASES_PAYMENT_INVALID"),
    (4, "ERR_PURCHASES_NOT_ALLOWED"),
    (5, "ERR_PURCHASES_NETWORK"),
    (6, "ERR_PURCHASES_NOT_ALLOWED"),
    (7, "ERR_PURCHASES_NOT_ALLOWED"),
];

/// The scripted answers the shared host hands out. A queue with nothing queued answers the default.
#[derive(Default)]
struct Fixture {
    products_errors: Arc<Mutex<VecDeque<i64>>>,
    products_answers: Arc<Mutex<VecDeque<Vec<StoreKitProduct>>>>,
    purchase_answers: Arc<Mutex<VecDeque<Result<StoreKitPurchase, StoreKitCode>>>>,
    storefront_answers: Arc<Mutex<VecDeque<String>>>,
    finished: Arc<Mutex<Vec<String>>>,
}

fn lock<T>(slot: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Fixture {
    /// Makes the next products call answer with the raw store code.
    fn push_products_error(&self, raw: i64) {
        lock(&self.products_errors).push_back(raw);
    }

    /// Makes the next products call answer with these raw products.
    fn push_products(&self, raws: Vec<StoreKitProduct>) {
        lock(&self.products_answers).push_back(raws);
    }

    /// Makes the next purchase answer with `answer`.
    fn push_purchase_answer(&self, answer: Result<StoreKitPurchase, StoreKitCode>) {
        lock(&self.purchase_answers).push_back(answer);
    }

    /// Makes the next storefront answer be `code`.
    fn push_storefront(&self, code: String) {
        lock(&self.storefront_answers).push_back(code);
    }

    /// The transaction ids finish was asked to finish, in order.
    fn finished(&self) -> Vec<String> {
        lock(&self.finished).clone()
    }
}

/// A pending Ask-to-Buy transaction, as the host reports it: state 0 and no receipt yet.
fn pending_raw(product_id: &str) -> StoreKitPurchase {
    StoreKitPurchase {
        transaction_id: format!("tx-{product_id}"),
        product_id: product_id.to_owned(),
        state: 0,
        json_receipt: String::new(),
        purchased_at_ms: None,
    }
}

/// A completed transaction, as the host reports it: state 1 with its receipt.
fn completed_raw(product_id: &str) -> StoreKitPurchase {
    StoreKitPurchase {
        transaction_id: format!("tx-{product_id}"),
        product_id: product_id.to_owned(),
        state: 1,
        json_receipt: format!("receipt-{product_id}"),
        purchased_at_ms: Some(1_700_000_000_000),
    }
}

/// A raw product as the host reports it, with its kind number riding along.
fn product_raw(id: &str, kind: i64) -> StoreKitProduct {
    StoreKitProduct {
        id: id.to_owned(),
        title: format!("{id} title"),
        description: String::new(),
        display_price: "0.99".to_owned(),
        currency_code: Some("USD".to_owned()),
        kind,
    }
}

/// The one host this process registers, answering from the shared fixture.
fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let fixture = Fixture::default();
        apple::install_host(StoreKitHost {
            products: {
                let errors = Arc::clone(&fixture.products_errors);
                let answers = Arc::clone(&fixture.products_answers);
                Arc::new(move |_ids: Vec<String>| {
                    // A queued error wins, then a queued listing, then the empty default.
                    let answer = match lock(&errors).pop_front() {
                        Some(raw) => Err(StoreKitCode {
                            raw,
                            message: "scripted".to_owned(),
                        }),
                        None => Ok(lock(&answers).pop_front().unwrap_or_default()),
                    };
                    Box::pin(async move { answer })
                })
            },
            purchase: {
                let answers = Arc::clone(&fixture.purchase_answers);
                Arc::new(move |_request: PurchaseRequest| {
                    let answer = lock(&answers)
                        .pop_front()
                        .unwrap_or_else(|| Ok(pending_raw("pro_unlock")));
                    Box::pin(async move { answer })
                })
            },
            finish: {
                let finished = Arc::clone(&fixture.finished);
                Arc::new(move |transaction_id: String| {
                    lock(&finished).push(transaction_id);
                    Box::pin(async { Ok(()) })
                })
            },
            restore: Arc::new(|| Box::pin(async { Ok(Vec::<StoreKitPurchase>::new()) })),
            unfinished: Arc::new(|| Box::pin(async { Ok(Vec::<StoreKitPurchase>::new()) })),
            entitlements: Arc::new(|| Box::pin(async { Ok(Vec::new()) })),
            storefront: {
                let answers = Arc::clone(&fixture.storefront_answers);
                Arc::new(move || {
                    let code = lock(&answers)
                        .pop_front()
                        .unwrap_or_else(|| "US".to_owned());
                    Box::pin(async move { Ok(code) })
                })
            },
            cancel: || {},
        });
        fixture
    })
}

#[test]
fn every_storekit_raw_code_maps_to_its_documented_code() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases = Purchases::with_backend(apple::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();
        for (raw, code) in STOREKIT_MAP {
            fixture.push_products_error(raw);
            let error = purchases
                .products(&["pro_unlock".to_owned()])
                .await
                .unwrap_err();
            assert_eq!(error.code(), code, "raw {raw} maps to its documented code");
            assert_eq!(
                error.status(),
                Some(raw),
                "the raw code rides along as the status"
            );
        }
    });
}

#[test]
fn unknown_storekit_raw_codes_are_backend() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases = Purchases::with_backend(apple::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();
        for raw in [99_i64, -1] {
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
fn a_storekit_cancel_and_a_pending_purchase_resolve_as_documented() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases = Purchases::with_backend(apple::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();

        // A user cancel carries paymentCancelled's number (2) and resolves Cancelled.
        fixture.push_purchase_answer(Err(StoreKitCode {
            raw: 2,
            message: "payment cancelled".to_owned(),
        }));
        let error = purchases
            .purchase("pro_unlock", PurchaseOptions::default())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Cancelled);

        // A pending Ask-to-Buy outcome arrives as state 0 and resolves Ok, still pending.
        fixture.push_purchase_answer(Ok(pending_raw("pro_unlock")));
        let purchase = purchases
            .purchase("pro_unlock", PurchaseOptions::default())
            .await
            .unwrap();
        assert_eq!(purchase.state, PurchaseState::Pending);
        assert_eq!(purchase.product_id, "pro_unlock");
    });
}

#[test]
fn consuming_a_consumable_finishes_and_refuses_a_non_consumable() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases = Purchases::with_backend(apple::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();

        // The consumable buys; the kind lookup sees 0, and StoreKit's finish is the consume.
        fixture.push_purchase_answer(Ok(completed_raw("coins_100")));
        let coins = purchases
            .purchase("coins_100", PurchaseOptions::default())
            .await
            .unwrap();
        fixture.push_products(vec![product_raw("coins_100", 0)]);
        purchases.consume(&coins).await.unwrap();
        assert_eq!(
            fixture.finished(),
            ["tx-coins_100".to_owned()],
            "consuming a consumable finishes its transaction"
        );

        // The non-consumable buys; consuming it is refused before finish is reached.
        fixture.push_purchase_answer(Ok(completed_raw("pro_unlock")));
        let unlock = purchases
            .purchase("pro_unlock", PurchaseOptions::default())
            .await
            .unwrap();
        fixture.push_products(vec![product_raw("pro_unlock", 1)]);
        let error = purchases.consume(&unlock).await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
        assert_eq!(
            fixture.finished(),
            ["tx-coins_100".to_owned()],
            "only the consumable reached finish"
        );
    });
}

#[test]
fn malformed_storekit_payloads_are_backend() {
    let _turns = lock(&TURNS);
    let fixture = fixture();
    let purchases = Purchases::with_backend(apple::native_backend().expect("the host registered"));
    run(async {
        purchases.initialize().await.unwrap();

        // A product kind the bridge does not know rides along as the status.
        fixture.push_products(vec![product_raw("pro_unlock", 9)]);
        let error = purchases
            .products(&["pro_unlock".to_owned()])
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Backend);
        assert_eq!(error.status(), Some(9));

        // A transaction state the bridge does not know rides along as the status.
        let mut unknown_state = completed_raw("pro_unlock");
        unknown_state.state = 7;
        fixture.push_purchase_answer(Ok(unknown_state));
        let error = purchases
            .purchase("pro_unlock", PurchaseOptions::default())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Backend);
        assert_eq!(error.status(), Some(7));

        // A completed purchase with no receipt could verify against nothing.
        let mut no_receipt = completed_raw("pro_unlock");
        no_receipt.json_receipt = String::new();
        fixture.push_purchase_answer(Ok(no_receipt));
        let error = purchases
            .purchase("pro_unlock", PurchaseOptions::default())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Backend);
        assert_eq!(error.status(), None);

        // An empty storefront answers the same way.
        fixture.push_storefront(String::new());
        let error = purchases.country_code().await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Backend);
        assert_eq!(error.status(), None);
    });
}
