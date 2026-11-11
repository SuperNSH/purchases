//! Android: Google Play Billing behind the host seam. The NSH Android host registers its
//! BillingClient callbacks at startup (`install_host`); this bridge maps their raw codes to the
//! stable `ERR_PURCHASES_*` codes and links no store SDK, so it compiles on every target
//! (docs/decisions.md). Every call resolves exactly once: no retries, no loops, no panics.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use crate::backend::{Backend, BackendFuture};
use crate::error::{ErrorKind, PurchasesError};
use crate::model::{Entitlement, Product, ProductKind, Purchase, PurchaseState};
use crate::options::PurchaseOptions;

/// `BillingClient.BillingResponseCode` values, as the NSH host forwards them from
/// `BillingResult.responseCode`. `OK` (0) is absent: success travels as `Ok`, not as a code.
pub const SERVICE_TIMEOUT: i64 = -3;
/// The requested feature is not supported by Play Store on this device.
pub const FEATURE_NOT_SUPPORTED: i64 = -2;
/// The Play Store service is not connected now; a potentially transient state.
pub const SERVICE_DISCONNECTED: i64 = -1;
/// The user pressed back or cancelled the dialog.
pub const USER_CANCELED: i64 = 1;
/// The store is unavailable; also the documented home of timeouts since Play stopped returning
/// SERVICE_TIMEOUT.
pub const SERVICE_UNAVAILABLE: i64 = 2;
/// Billing cannot run here: no Play Store, or the device cannot bill.
pub const BILLING_UNAVAILABLE: i64 = 3;
/// The requested product is not available for purchase.
pub const ITEM_UNAVAILABLE: i64 = 4;
/// The arguments given to the API are invalid.
pub const DEVELOPER_ERROR: i64 = 5;
/// A fatal error during the API action.
pub const ERROR: i64 = 6;
/// The purchase failed because the item is already owned.
pub const ITEM_ALREADY_OWNED: i64 = 7;
/// Acknowledge or consume failed because the item is not owned: unknown, or already finished.
pub const ITEM_NOT_OWNED: i64 = 8;
/// The network connection failed between the device and Play systems.
pub const NETWORK_ERROR: i64 = 12;

/// `BillingProduct.kind` and `BillingPurchase.kind` numbers, one per product kind.
pub const KIND_CONSUMABLE: i64 = 0;
pub const KIND_NON_CONSUMABLE: i64 = 1;
pub const KIND_SUBSCRIPTION: i64 = 2;

/// `Purchase.PurchaseState` values. UNSPECIFIED (0) and any undocumented number: Play documents
/// no action for them, so the bridge answers Backend carrying the number as its status.
pub const PURCHASE_STATE_PURCHASED: i64 = 1;
/// PENDING is a purchase awaiting approval (ask-a-parent); it resolves Ok with a pending state.
pub const PURCHASE_STATE_PENDING: i64 = 2;

/// The host's raw error: a `BillingResponseCode` number with Play's own debug text.
///
/// The text is the host's diagnostic; the mapped error keeps only the stable code and, for
/// unknown numbers, the raw status.
pub struct BillingCode {
    /// The `BillingResult.responseCode` value.
    pub raw: i64,
    /// Play's debug message (en-US; not for users).
    pub message: String,
}

/// The BillingResponse table: every raw code the store documents, and the stable kind it becomes.
/// An unknown raw code becomes Backend carrying the status, never a panic.
fn map_billing_code(code: BillingCode) -> PurchasesError {
    let kind = match code.raw {
        // The user stopped the flow.
        USER_CANCELED => ErrorKind::Cancelled,
        // The store connection failed: unavailable, disconnected, network down, or timed out.
        SERVICE_UNAVAILABLE | SERVICE_TIMEOUT | SERVICE_DISCONNECTED | NETWORK_ERROR => {
            ErrorKind::Network
        }
        // Billing cannot run on this device or store at all.
        BILLING_UNAVAILABLE | FEATURE_NOT_SUPPORTED => ErrorKind::NotAvailable,
        // The store has no such item.
        ITEM_UNAVAILABLE => ErrorKind::ProductNotFound,
        // The call itself was wrong, including a consume the host declined for kind reasons.
        DEVELOPER_ERROR => ErrorKind::InvalidArgument,
        // The store failed with nothing more specific to say.
        ERROR => ErrorKind::Backend,
        // The non-consumable or active subscription is already owned.
        ITEM_ALREADY_OWNED => ErrorKind::AlreadyOwned,
        // Unknown purchase, or one already acknowledged or consumed.
        ITEM_NOT_OWNED => ErrorKind::NotOwned,
        // A number Play has not documented: keep it for the app to read.
        raw => return PurchasesError::new(ErrorKind::Backend).with_status(raw),
    };
    PurchasesError::new(kind)
}

/// A host payload the model cannot represent. The error type carries no free text, so the
/// stable Backend code is the whole answer.
fn malformed() -> PurchasesError {
    PurchasesError::new(ErrorKind::Backend)
}

/// The future a host call resolves on. Send, so the host may answer from any thread.
type HostFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// One host callback: takes `A`, resolves once with `R` or the store's raw code.
type HostCall<A, R> = Arc<dyn Fn(A) -> HostFuture<Result<R, BillingCode>> + Send + Sync>;

/// A product as BillingClient's `queryProductDetails` reports it.
pub struct BillingProduct {
    /// The store's identifier, what `purchase` is called with.
    pub id: String,
    /// The store-localized display name.
    pub title: String,
    /// The store-localized description.
    pub description: String,
    /// The display price string, already formatted for the store's locale.
    pub display_price: String,
    /// The ISO 4217 code of `display_price`, when Play reports one.
    pub currency_code: Option<String>,
    /// One of the `KIND_*` numbers.
    pub kind: i64,
}

/// A purchase as BillingClient reports it. `kind` rides along because acknowledge and consume
/// are decided by the kind the host reports, not by anything in the model `Purchase`.
#[derive(Clone)]
pub struct BillingPurchase {
    /// The purchase token: what acknowledge and consume are asked to act on.
    pub purchase_token: String,
    /// The identifier of the product bought.
    pub product_id: String,
    /// One of the `PURCHASE_STATE_*` numbers.
    pub state: i64,
    /// One of the `KIND_*` numbers.
    pub kind: i64,
    /// The purchase's JSON payload, for app-side receipt validation.
    pub receipt: String,
    /// When Play says the purchase happened, in ms since the epoch.
    pub purchased_at_ms: Option<u64>,
}

/// An entitlement as the host reports it, for non-consumables and active subscriptions.
pub struct BillingEntitlement {
    /// The identifier of the entitled product.
    pub product_id: String,
    /// Whether the entitlement holds right now.
    pub active: bool,
    /// When the entitlement lapses, in ms since the epoch, when Play reports a renewal date.
    pub expires_at_ms: Option<u64>,
}

/// One purchase attempt, as `launchBillingFlow` receives it.
pub struct PurchaseRequest {
    /// The identifier of the product to buy.
    pub product_id: String,
    /// How many units to buy. Rides the request to mirror the Apple shape; the host ignores it
    /// on Play, where launchBillingFlow takes no quantity.
    pub quantity: u32,
    /// The app's own buyer identifier, when the app keeps one.
    pub application_username: Option<String>,
}

/// The callbacks an NSH Android host registers at startup, one per BillingClient operation the
/// bridge needs. The real glue lives in the host; each callback resolves its call exactly once.
pub struct BillingHost {
    /// `startConnection`: Ok once the store is reachable. The facade makes initialize
    /// idempotent; the host glue makes startConnection idempotent.
    pub connect: HostCall<(), ()>,
    /// `queryProductDetails`: the ids to fetch. Unknown ids are simply absent from the answer.
    pub products: HostCall<Vec<String>, Vec<BillingProduct>>,
    /// `launchBillingFlow`: one purchase attempt, answered by the flow's result.
    pub purchase: HostCall<PurchaseRequest, BillingPurchase>,
    /// `acknowledgePurchase`, for non-consumables and subscriptions, once per purchase.
    pub acknowledge: HostCall<String, ()>,
    /// `consumePurchase`, for consumables. Only a consumable reaches this callback.
    pub consume: HostCall<String, ()>,
    /// `queryPurchases`: what the user still owns. Play has no restored state, so the host
    /// answers PURCHASED and the bridge never emits Restored.
    pub restore: HostCall<(), Vec<BillingPurchase>>,
    /// `queryPurchases` filtered to what needs action: PENDING approval, or awaiting
    /// acknowledge or consume.
    pub unfinished: HostCall<(), Vec<BillingPurchase>>,
    /// Active entitlements, by product.
    pub entitlements: HostCall<(), Vec<BillingEntitlement>>,
    /// `getBillingConfig`'s country code, ISO 3166-1 alpha-2.
    pub country_code: HostCall<(), String>,
    /// Asks the host to dismiss a running billing flow. Fire-and-forget: the call it interrupts
    /// still resolves exactly once.
    pub cancel: fn(),
}

/// The host's callbacks, set once at startup. A process has one BillingClient.
static HOST: OnceLock<BillingHost> = OnceLock::new();

/// Registers the Android host's callbacks. The first registration wins; a later one is ignored,
/// because a live BillingClient cannot be swapped under the bridge.
pub fn install_host(host: BillingHost) {
    let _ = HOST.set(host);
}

/// The Play Billing backend, once a host has registered. `None` until then, so a target with no
/// host falls through to the unsupported backend.
pub fn native_backend() -> Option<Arc<dyn Backend>> {
    HOST.get()
        .map(|_| Arc::new(NativeBackend) as Arc<dyn Backend>)
}

fn product_from_raw(raw: BillingProduct) -> Result<Product, PurchasesError> {
    if raw.id.is_empty() {
        return Err(malformed());
    }
    let kind = match raw.kind {
        KIND_CONSUMABLE => ProductKind::Consumable,
        KIND_NON_CONSUMABLE => ProductKind::NonConsumable,
        KIND_SUBSCRIPTION => ProductKind::Subscription,
        kind => return Err(PurchasesError::new(ErrorKind::Backend).with_status(kind)),
    };
    Ok(Product {
        id: raw.id,
        title: raw.title,
        description: raw.description,
        price: raw.display_price,
        currency_code: raw.currency_code,
        kind,
    })
}

fn purchase_from_raw(raw: BillingPurchase) -> Result<Purchase, PurchasesError> {
    if raw.purchase_token.is_empty() || raw.product_id.is_empty() || raw.receipt.is_empty() {
        return Err(malformed());
    }
    let state = match raw.state {
        PURCHASE_STATE_PURCHASED => PurchaseState::Purchased,
        PURCHASE_STATE_PENDING => PurchaseState::Pending,
        // UNSPECIFIED (0) and undocumented numbers: Play says act on neither; the number rides
        // along as the status.
        state => return Err(PurchasesError::new(ErrorKind::Backend).with_status(state)),
    };
    Ok(Purchase {
        id: raw.purchase_token,
        product_id: raw.product_id,
        state,
        receipt: raw.receipt,
        purchased_at_ms: raw.purchased_at_ms,
    })
}

fn entitlement_from_raw(raw: BillingEntitlement) -> Result<Entitlement, PurchasesError> {
    if raw.product_id.is_empty() {
        return Err(malformed());
    }
    Ok(Entitlement {
        product_id: raw.product_id,
        active: raw.active,
        expires_at_ms: raw.expires_at_ms,
    })
}

/// The backend over the registered host: each call forwards once and maps the answer.
struct NativeBackend;

impl Backend for NativeBackend {
    fn is_available(&self) -> BackendFuture<'_, Result<bool, PurchasesError>> {
        // A registered host means the host glue put a BillingClient behind this process.
        Box::pin(async { Ok(HOST.get().is_some()) })
    }

    fn initialize(&self) -> BackendFuture<'_, Result<(), PurchasesError>> {
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            (host.connect)(()).await.map_err(map_billing_code)
        })
    }

    fn products(&self, ids: &[String]) -> BackendFuture<'_, Result<Vec<Product>, PurchasesError>> {
        let ids = ids.to_vec();
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            let raws = (host.products)(ids).await.map_err(map_billing_code)?;
            raws.into_iter().map(product_from_raw).collect()
        })
    }

    fn purchase(
        &self,
        product_id: &str,
        options: &PurchaseOptions,
    ) -> BackendFuture<'_, Result<Purchase, PurchasesError>> {
        let request = PurchaseRequest {
            product_id: product_id.to_owned(),
            quantity: options.quantity,
            application_username: options.application_username.clone(),
        };
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            let raw = (host.purchase)(request).await.map_err(map_billing_code)?;
            purchase_from_raw(raw)
        })
    }

    fn finish(&self, purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>> {
        // On Android finishing is acknowledging: one call, once per purchase. An unknown or
        // already acknowledged token is answered ITEM_NOT_OWNED by the store.
        let token = purchase.id.clone();
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            (host.acknowledge)(token).await.map_err(map_billing_code)
        })
    }

    fn consume(&self, purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>> {
        let token = purchase.id.clone();
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            // The kind gate: only a purchase the host reports as a consumable reaches consume.
            // A token the host no longer reports goes straight to the store, which answers
            // ITEM_NOT_OWNED for the unknown or already finished.
            let unfinished = (host.unfinished)(()).await.map_err(map_billing_code)?;
            if let Some(raw) = unfinished.iter().find(|p| p.purchase_token == token)
                && raw.kind != KIND_CONSUMABLE
            {
                return Err(PurchasesError::new(ErrorKind::InvalidArgument));
            }
            (host.consume)(token).await.map_err(map_billing_code)
        })
    }

    fn restore(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>> {
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            let raws = (host.restore)(()).await.map_err(map_billing_code)?;
            raws.into_iter().map(purchase_from_raw).collect()
        })
    }

    fn pending(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>> {
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            let raws = (host.unfinished)(()).await.map_err(map_billing_code)?;
            raws.into_iter().map(purchase_from_raw).collect()
        })
    }

    fn entitlements(&self) -> BackendFuture<'_, Result<Vec<Entitlement>, PurchasesError>> {
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            let raws = (host.entitlements)(()).await.map_err(map_billing_code)?;
            raws.into_iter().map(entitlement_from_raw).collect()
        })
    }

    fn country_code(&self) -> BackendFuture<'_, Result<String, PurchasesError>> {
        Box::pin(async move {
            let host = HOST
                .get()
                .ok_or_else(|| PurchasesError::new(ErrorKind::NotAvailable))?;
            let code = (host.country_code)(()).await.map_err(map_billing_code)?;
            if code.is_empty() {
                return Err(malformed());
            }
            Ok(code)
        })
    }

    fn cancel(&self) {
        // Where the host cannot dismiss the flow, its call still resolves once on its own.
        if let Some(host) = HOST.get() {
            (host.cancel)();
        }
    }
}
