//! A backend with scripted answers. Tests use it, and so can an app that runs without a store.

use std::{
    future::ready,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use tokio::sync::oneshot;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, PurchasesError},
    model::{Entitlement, Product, ProductKind, Purchase, PurchaseState},
    options::PurchaseOptions,
};

/// The sandbox time the fake stamps its purchases with, in milliseconds since the epoch.
const SANDBOX_NOW_MS: u64 = 1_735_689_600_000;
/// How long the fake's subscription entitlement runs from the sandbox time.
const MONTH_MS: u64 = 30 * 24 * 60 * 60 * 1000;

/// A backend whose answers the test sets, and whose purchase the test answers.
///
/// Like a store, a cancel does not answer the purchase: the test answers it, as the store
/// would. Clones share one state.
#[derive(Clone)]
pub struct FakeBackend {
    state: Arc<Mutex<FakeState>>,
}

struct FakeState {
    available: Result<bool, PurchasesError>,
    next_purchase_error: Option<PurchasesError>,
    entitlements: Option<Result<Vec<Entitlement>, PurchasesError>>,
    restore: Option<Result<Vec<Purchase>, PurchasesError>>,
    products: Vec<Product>,
    waiting: Option<oneshot::Sender<Result<Purchase, PurchasesError>>>,
    prepared: Option<Purchase>,
    delivered: Vec<Purchase>,
    finished: Vec<String>,
    consumed: Vec<String>,
    purchases: usize,
    finishes: usize,
    consumes: usize,
    restores: usize,
    cancels: usize,
    product_queries: usize,
    last_product_ids: Option<Vec<String>>,
    last_purchase: Option<(String, PurchaseOptions)>,
    last_finished: Option<String>,
    last_consumed: Option<String>,
}

impl Default for FakeBackend {
    /// A store with the three fixture products and nothing bought yet.
    fn default() -> Self {
        Self::new()
    }
}

impl FakeBackend {
    /// A store listing the fixture products: `pro_unlock`, `coins_100` and `pro_monthly`.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState {
                available: Ok(true),
                next_purchase_error: None,
                entitlements: None,
                restore: None,
                products: fixture_products(),
                waiting: None,
                prepared: None,
                delivered: Vec::new(),
                finished: Vec::new(),
                consumed: Vec::new(),
                purchases: 0,
                finishes: 0,
                consumes: 0,
                restores: 0,
                cancels: 0,
                product_queries: 0,
                last_product_ids: None,
                last_purchase: None,
                last_finished: None,
                last_consumed: None,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, FakeState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets what `is_available` answers.
    pub fn set_available(&self, answer: Result<bool, PurchasesError>) {
        self.lock().available = answer;
    }

    /// Makes the next purchase fail with `error`, as a store does when it rejects the payment.
    pub fn set_next_purchase_error(&self, error: PurchasesError) {
        self.lock().next_purchase_error = Some(error);
    }

    /// Sets what `entitlements` answers. Without one it computes from the delivered purchases.
    pub fn set_entitlements(&self, answer: Result<Vec<Entitlement>, PurchasesError>) {
        self.lock().entitlements = Some(answer);
    }

    /// Sets what `restore` answers. Without one it returns the delivered purchases as restored.
    pub fn set_restore_answer(&self, answer: Result<Vec<Purchase>, PurchasesError>) {
        self.lock().restore = Some(answer);
    }

    /// Whether a purchase is waiting for its answer.
    pub fn purchase_pending(&self) -> bool {
        self.lock().waiting.is_some()
    }

    /// The purchase the fake prepared for the waiting call, receipt and timestamps included.
    pub fn prepared_purchase(&self) -> Option<Purchase> {
        self.lock().prepared.clone()
    }

    /// Answers the waiting purchase. Returns `false` when none waits; an answer no call waits
    /// for anymore is dropped, as a late store answer is.
    pub fn finish_purchase(&self, answer: Result<Purchase, PurchasesError>) -> bool {
        let waiting = {
            let mut state = self.lock();
            state.prepared.take();
            state.waiting.take()
        };
        match waiting {
            Some(sender) => {
                if let Ok(purchase) = &answer {
                    self.lock().delivered.push(purchase.clone());
                }
                let _ = sender.send(answer);
                true
            }
            None => false,
        }
    }

    /// How many purchases went through to the store, answered or still waiting.
    pub fn purchases_started(&self) -> usize {
        self.lock().purchases
    }

    /// How many purchases were finished.
    pub fn finishes(&self) -> usize {
        self.lock().finishes
    }

    /// How many purchases were consumed.
    pub fn consumes(&self) -> usize {
        self.lock().consumes
    }

    /// How many times restore ran.
    pub fn restores(&self) -> usize {
        self.lock().restores
    }

    /// How many times the store was asked to cancel its purchase.
    pub fn cancels(&self) -> usize {
        self.lock().cancels
    }

    /// How many times products were queried.
    pub fn product_queries(&self) -> usize {
        self.lock().product_queries
    }

    /// The ids the last product query asked for.
    pub fn last_product_ids(&self) -> Option<Vec<String>> {
        self.lock().last_product_ids.clone()
    }

    /// The id and options of the last purchase the store took.
    pub fn last_purchase(&self) -> Option<(String, PurchaseOptions)> {
        self.lock().last_purchase.clone()
    }

    /// The id of the last finished purchase.
    pub fn last_finished(&self) -> Option<String> {
        self.lock().last_finished.clone()
    }

    /// The id of the last consumed purchase.
    pub fn last_consumed(&self) -> Option<String> {
        self.lock().last_consumed.clone()
    }
}

impl Backend for FakeBackend {
    fn is_available(&self) -> BackendFuture<'_, Result<bool, PurchasesError>> {
        let answer = self.lock().available.clone();
        Box::pin(ready(answer))
    }

    fn initialize(&self) -> BackendFuture<'_, Result<(), PurchasesError>> {
        Box::pin(ready(Ok::<(), PurchasesError>(())))
    }

    fn products(&self, ids: &[String]) -> BackendFuture<'_, Result<Vec<Product>, PurchasesError>> {
        let found: Vec<Product> = {
            let mut state = self.lock();
            state.product_queries += 1;
            state.last_product_ids = Some(ids.to_vec());
            state
                .products
                .iter()
                .filter(|product| ids.contains(&product.id))
                .cloned()
                .collect()
        };
        Box::pin(ready(Ok::<Vec<Product>, PurchasesError>(found)))
    }

    fn purchase(
        &self,
        product_id: &str,
        options: &PurchaseOptions,
    ) -> BackendFuture<'_, Result<Purchase, PurchasesError>> {
        let receiver = {
            let mut state = self.lock();
            state.last_purchase = Some((product_id.to_owned(), options.clone()));
            if let Some(error) = state.next_purchase_error.take() {
                return Box::pin(ready(Err(error)));
            }
            let Some(product) = state.products.iter().find(|p| p.id == product_id).cloned() else {
                return Box::pin(ready(Err(PurchasesError::new(ErrorKind::ProductNotFound))));
            };
            state.purchases += 1;
            let receipt = format!("sandbox-receipt-{}", product.id);
            let purchase = Purchase {
                id: format!("fake-{}-{}", product.id, state.purchases),
                product_id: product.id,
                state: PurchaseState::Purchased,
                receipt,
                purchased_at_ms: Some(SANDBOX_NOW_MS),
            };
            let (sender, receiver) = oneshot::channel();
            state.waiting = Some(sender);
            state.prepared = Some(purchase);
            receiver
        };
        // The store answers on its own schedule; a dropped listener just drops the answer.
        Box::pin(async move {
            receiver
                .await
                .unwrap_or_else(|_| Err(PurchasesError::new(ErrorKind::Backend)))
        })
    }

    fn finish(&self, purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>> {
        let answer = {
            let mut state = self.lock();
            let known = state.delivered.iter().any(|p| p.id == purchase.id);
            if !known || state.finished.contains(&purchase.id) {
                Err(PurchasesError::new(ErrorKind::NotOwned))
            } else {
                state.finished.push(purchase.id.clone());
                state.last_finished = Some(purchase.id.clone());
                state.finishes += 1;
                Ok(())
            }
        };
        Box::pin(ready(answer))
    }

    fn consume(&self, purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>> {
        let answer = {
            let mut state = self.lock();
            let kind = state
                .products
                .iter()
                .find(|product| product.id == purchase.product_id)
                .map(|product| product.kind);
            let known = state.delivered.iter().any(|p| p.id == purchase.id);
            match kind {
                Some(ProductKind::Consumable)
                    if known && !state.consumed.contains(&purchase.id) =>
                {
                    state.consumed.push(purchase.id.clone());
                    state.last_consumed = Some(purchase.id.clone());
                    state.consumes += 1;
                    Ok(())
                }
                Some(ProductKind::Consumable) => Err(PurchasesError::new(ErrorKind::NotOwned)),
                Some(_) => Err(PurchasesError::new(ErrorKind::InvalidArgument)),
                None => Err(PurchasesError::new(ErrorKind::NotOwned)),
            }
        };
        Box::pin(ready(answer))
    }

    fn restore(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>> {
        let answer = {
            let mut state = self.lock();
            state.restores += 1;
            match state.restore.clone() {
                Some(answer) => answer,
                None => Ok(state
                    .delivered
                    .iter()
                    .map(|purchase| {
                        let mut restored = purchase.clone();
                        restored.state = PurchaseState::Restored;
                        restored
                    })
                    .collect()),
            }
        };
        Box::pin(ready(answer))
    }

    fn pending(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>> {
        let unfinished: Vec<Purchase> = {
            let state = self.lock();
            state
                .delivered
                .iter()
                .filter(|purchase| {
                    !state.finished.contains(&purchase.id) && !state.consumed.contains(&purchase.id)
                })
                .cloned()
                .collect()
        };
        Box::pin(ready(Ok::<Vec<Purchase>, PurchasesError>(unfinished)))
    }

    fn entitlements(&self) -> BackendFuture<'_, Result<Vec<Entitlement>, PurchasesError>> {
        let answer = {
            let state = self.lock();
            match state.entitlements.clone() {
                Some(answer) => answer,
                None => {
                    // A right comes from a delivered, unconsumed, non-pending purchase; the
                    // newest purchase of a product wins.
                    let mut owned: Vec<Entitlement> = Vec::new();
                    for purchase in &state.delivered {
                        if state.consumed.contains(&purchase.id)
                            || purchase.state == PurchaseState::Pending
                        {
                            continue;
                        }
                        let subscription = state.products.iter().any(|product| {
                            product.id == purchase.product_id
                                && product.kind == ProductKind::Subscription
                        });
                        let entitlement = Entitlement {
                            product_id: purchase.product_id.clone(),
                            active: true,
                            expires_at_ms: subscription.then_some(SANDBOX_NOW_MS + MONTH_MS),
                        };
                        match owned
                            .iter()
                            .position(|held| held.product_id == entitlement.product_id)
                        {
                            Some(at) => owned[at] = entitlement,
                            None => owned.push(entitlement),
                        }
                    }
                    Ok::<Vec<Entitlement>, PurchasesError>(owned)
                }
            }
        };
        Box::pin(ready(answer))
    }

    fn country_code(&self) -> BackendFuture<'_, Result<String, PurchasesError>> {
        Box::pin(ready(Ok::<String, PurchasesError>("US".to_owned())))
    }

    fn cancel(&self) {
        self.lock().cancels += 1;
    }
}

/// The sandbox fixture catalogue, as the references' tests list their products.
fn fixture_products() -> Vec<Product> {
    vec![
        Product {
            id: "pro_unlock".to_owned(),
            title: "Pro Unlock".to_owned(),
            description: "Every pro feature, owned for good.".to_owned(),
            price: "4.99".to_owned(),
            currency_code: Some("USD".to_owned()),
            kind: ProductKind::NonConsumable,
        },
        Product {
            id: "coins_100".to_owned(),
            title: "100 Coins".to_owned(),
            description: "A bag of 100 coins, consumed on use.".to_owned(),
            price: "0.99".to_owned(),
            currency_code: Some("USD".to_owned()),
            kind: ProductKind::Consumable,
        },
        Product {
            id: "pro_monthly".to_owned(),
            title: "Pro Monthly".to_owned(),
            description: "Every pro feature, billed monthly.".to_owned(),
            price: "1.99".to_owned(),
            currency_code: Some("USD".to_owned()),
            kind: ProductKind::Subscription,
        },
    ]
}
