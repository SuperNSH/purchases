//! Apple (iOS and macOS): the StoreKit 2 seam. The NSH host registers callbacks at startup;
//! this bridge awaits each one once and maps its raw answer to the crate's types and codes.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, OnceLock},
};

use crate::{
    Backend, BackendFuture, Entitlement, ErrorKind, Product, ProductKind, Purchase,
    PurchaseOptions, PurchaseState, PurchasesError,
};

/// A host answer: one future that resolves exactly once, with the raw result or the store's code.
type HostFuture<T> = Pin<Box<dyn Future<Output = Result<T, StoreKitCode>> + Send>>;

/// The host's raw StoreKit error: `SKError.code` (or the StoreKit 2 case number) with its text.
#[derive(Clone, Debug, PartialEq)]
pub struct StoreKitCode {
    /// The store's own error number, as the host received it.
    pub raw: i64,
    /// The store's own error text, kept for diagnostics.
    pub message: String,
}

/// A raw product as the host reports it, straight from StoreKit 2.
#[derive(Clone, Debug, PartialEq)]
pub struct StoreKitProduct {
    /// The store's product identifier.
    pub id: String,
    /// The localized display name.
    pub title: String,
    /// The localized description.
    pub description: String,
    /// The localized price as a display string, such as `"4.99"`.
    pub display_price: String,
    /// The store's ISO currency code, when the host knows it.
    pub currency_code: Option<String>,
    /// 0 consumable, 1 non-consumable, 2 subscription; any other number is malformed.
    pub kind: i64,
}

/// A raw transaction as the host reports it. The state numbers are this bridge's contract
/// with the host, not StoreKit's own enum order: 0 pending, 1 purchased, 2 restored.
#[derive(Clone, Debug, PartialEq)]
pub struct StoreKitPurchase {
    /// The store's transaction identifier.
    pub transaction_id: String,
    /// The product the transaction bought.
    pub product_id: String,
    /// 0 pending (for example Ask to Buy), 1 purchased, 2 restored; any other number is malformed.
    pub state: i64,
    /// The JWS receipt, the store's verification payload; a pending transaction has none yet.
    pub json_receipt: String,
    /// When the store charged the user, in milliseconds since the epoch.
    pub purchased_at_ms: Option<u64>,
}

/// A raw entitlement as the host reports it, from StoreKit 2's current entitlements.
#[derive(Clone, Debug, PartialEq)]
pub struct StoreKitEntitlement {
    /// The entitled product.
    pub product_id: String,
    /// Whether the entitlement holds right now.
    pub active: bool,
    /// When the entitlement lapses, in milliseconds since the epoch.
    pub expires_at_ms: Option<u64>,
}

/// One purchase as the bridge hands it to the host. The facade's timeout is not the host's
/// concern: the bridge races nothing and only relays.
#[derive(Clone, Debug, PartialEq)]
pub struct PurchaseRequest {
    /// The product to buy.
    pub product_id: String,
    /// How many units; StoreKit applies it to consumables.
    pub quantity: u32,
    /// The app's own user string, which StoreKit records on the transaction.
    pub application_username: Option<String>,
}

/// The callbacks a native shell host registers at startup; every field must be set.
/// Each callback answers once, and none of them may block.
pub struct StoreKitHost {
    /// Asks the store for the products it knows among `ids`; unknown ids are dropped.
    pub products: Arc<dyn Fn(Vec<String>) -> HostFuture<Vec<StoreKitProduct>> + Send + Sync>,
    /// Buys one product; a pending outcome arrives as state 0, a cancel as code 2.
    pub purchase: Arc<dyn Fn(PurchaseRequest) -> HostFuture<StoreKitPurchase> + Send + Sync>,
    /// Finishes (and so consumes, for a consumable) the transaction with this id.
    pub finish: Arc<dyn Fn(String) -> HostFuture<()> + Send + Sync>,
    /// Gathers the user's restored transactions.
    pub restore: Arc<dyn Fn() -> HostFuture<Vec<StoreKitPurchase>> + Send + Sync>,
    /// Gathers the transactions awaiting finish.
    pub unfinished: Arc<dyn Fn() -> HostFuture<Vec<StoreKitPurchase>> + Send + Sync>,
    /// Gathers the current entitlements.
    pub entitlements: Arc<dyn Fn() -> HostFuture<Vec<StoreKitEntitlement>> + Send + Sync>,
    /// The storefront's country code, such as `"US"`.
    pub storefront: Arc<dyn Fn() -> HostFuture<String> + Send + Sync>,
    /// Asks the host to dismiss any running store UI; a late answer still arrives.
    pub cancel: fn(),
}

static HOST: OnceLock<Arc<StoreKitHost>> = OnceLock::new();

/// Registers the host's callbacks for this process. The first registration wins; a later
/// one is ignored, so the store a session started with is the store it keeps.
pub fn install_host(host: StoreKitHost) {
    let _ = HOST.set(Arc::new(host));
}

/// The Apple backend, once a host has registered its callbacks.
pub fn native_backend() -> Option<Arc<dyn Backend>> {
    Some(Arc::new(NativeBackend {
        host: HOST.get()?.clone(),
    }))
}

/// The raw product kinds the host reports; see [`StoreKitProduct::kind`].
const KIND_CONSUMABLE: i64 = 0;
const KIND_NON_CONSUMABLE: i64 = 1;
const KIND_SUBSCRIPTION: i64 = 2;

/// The raw transaction states the host reports; see [`StoreKitPurchase::state`].
const STATE_PENDING: i64 = 0;
const STATE_PURCHASED: i64 = 1;
const STATE_RESTORED: i64 = 2;

/// The error mapping table: the host's raw StoreKit code to the stable `ErrorKind`,
/// every arm carrying the raw number as its status. The raw numbers are `SKError.code`
/// from StoreKit's SKError.h — the numbering the references pass through (flutter's
/// `SKError`, exported by `store_kit_wrappers.dart`, carries the bare int; OpenIAP's
/// `SKError.Code` table names the same cases):
///
/// - `clientInvalid` (1) -> `InvalidArgument`
/// - `paymentCancelled` (2) -> `Cancelled`
/// - `paymentInvalid` (3) -> `PaymentInvalid`
/// - `paymentNotAllowed` (4) -> `NotAllowed`
/// - `cloudServiceNetworkConnectionFailed` (5) -> `Network`
/// - `cloudServicePermissionDenied` (6) -> `NotAllowed`
/// - `cloudServiceRevoked` (7) -> `NotAllowed`
///
/// StoreKit 2's purchase-result outcomes land here too, by this bridge's host contract:
/// the host reports `userCancelled` as raw 2, `paymentCancelled`'s number, and `pending`
/// is never an error — the host answers it as a purchase with state 0. Everything else,
/// `unknown` (0) included, is `Backend` carrying the status.
fn map_code(code: StoreKitCode) -> PurchasesError {
    let kind = match code.raw {
        CODE_CLIENT_INVALID => ErrorKind::InvalidArgument,
        CODE_PAYMENT_CANCELLED => ErrorKind::Cancelled,
        CODE_PAYMENT_INVALID => ErrorKind::PaymentInvalid,
        CODE_PAYMENT_NOT_ALLOWED => ErrorKind::NotAllowed,
        CODE_CLOUD_NETWORK_FAILED => ErrorKind::Network,
        CODE_CLOUD_PERMISSION_DENIED => ErrorKind::NotAllowed,
        CODE_CLOUD_REVOKED => ErrorKind::NotAllowed,
        _ => ErrorKind::Backend,
    };
    PurchasesError::new(kind).with_status(code.raw)
}

// SKError.code numbers, from StoreKit's SKError.h; see `map_code` for their mapping.
const CODE_CLIENT_INVALID: i64 = 1;
const CODE_PAYMENT_CANCELLED: i64 = 2;
const CODE_PAYMENT_INVALID: i64 = 3;
const CODE_PAYMENT_NOT_ALLOWED: i64 = 4;
const CODE_CLOUD_NETWORK_FAILED: i64 = 5;
const CODE_CLOUD_PERMISSION_DENIED: i64 = 6;
const CODE_CLOUD_REVOKED: i64 = 7;

/// Maps a raw product kind by number; an unknown number is a Backend error carrying it.
fn map_kind(kind: i64) -> Result<ProductKind, PurchasesError> {
    match kind {
        KIND_CONSUMABLE => Ok(ProductKind::Consumable),
        KIND_NON_CONSUMABLE => Ok(ProductKind::NonConsumable),
        KIND_SUBSCRIPTION => Ok(ProductKind::Subscription),
        other => Err(PurchasesError::new(ErrorKind::Backend).with_status(other)),
    }
}

/// Maps a raw transaction state by number; an unknown number is a Backend error carrying it.
fn map_state(state: i64) -> Result<PurchaseState, PurchasesError> {
    match state {
        STATE_PENDING => Ok(PurchaseState::Pending),
        STATE_PURCHASED => Ok(PurchaseState::Purchased),
        STATE_RESTORED => Ok(PurchaseState::Restored),
        other => Err(PurchasesError::new(ErrorKind::Backend).with_status(other)),
    }
}

/// Maps one raw product; an empty id or an unknown kind is a Backend error.
fn map_product(raw: StoreKitProduct) -> Result<Product, PurchasesError> {
    if raw.id.is_empty() {
        return Err(PurchasesError::new(ErrorKind::Backend));
    }
    Ok(Product {
        id: raw.id,
        title: raw.title,
        description: raw.description,
        price: raw.display_price,
        currency_code: raw.currency_code,
        kind: map_kind(raw.kind)?,
    })
}

/// Maps one raw transaction. A pending transaction has no receipt yet; a purchased or
/// restored one without a receipt is malformed, because nothing could verify it.
fn map_purchase(raw: StoreKitPurchase) -> Result<Purchase, PurchasesError> {
    if raw.transaction_id.is_empty() || raw.product_id.is_empty() {
        return Err(PurchasesError::new(ErrorKind::Backend));
    }
    let state = map_state(raw.state)?;
    if raw.json_receipt.is_empty() && !matches!(state, PurchaseState::Pending) {
        return Err(PurchasesError::new(ErrorKind::Backend));
    }
    Ok(Purchase {
        id: raw.transaction_id,
        product_id: raw.product_id,
        state,
        receipt: raw.json_receipt,
        purchased_at_ms: raw.purchased_at_ms,
    })
}

/// Maps one raw entitlement; an empty product id is a Backend error.
fn map_entitlement(raw: StoreKitEntitlement) -> Result<Entitlement, PurchasesError> {
    if raw.product_id.is_empty() {
        return Err(PurchasesError::new(ErrorKind::Backend));
    }
    Ok(Entitlement {
        product_id: raw.product_id,
        active: raw.active,
        expires_at_ms: raw.expires_at_ms,
    })
}

/// The Apple backend: calls the registered host once per call and maps the answer.
struct NativeBackend {
    host: Arc<StoreKitHost>,
}

impl Backend for NativeBackend {
    fn is_available(&self) -> BackendFuture<'_, Result<bool, PurchasesError>> {
        // This backend exists only once a host registered; the store is there.
        Box::pin(async { Ok(true) })
    }

    fn initialize(&self) -> BackendFuture<'_, Result<(), PurchasesError>> {
        // StoreKit 2 has no connection to open; the registered host is the store.
        Box::pin(async { Ok(()) })
    }

    fn products(&self, ids: &[String]) -> BackendFuture<'_, Result<Vec<Product>, PurchasesError>> {
        let host = self.host.clone();
        let ids = ids.to_vec();
        Box::pin(async move {
            let raws = (host.products)(ids).await.map_err(map_code)?;
            raws.into_iter()
                .map(map_product)
                .collect::<Result<Vec<Product>, PurchasesError>>()
        })
    }

    fn purchase(
        &self,
        product_id: &str,
        options: &PurchaseOptions,
    ) -> BackendFuture<'_, Result<Purchase, PurchasesError>> {
        let host = self.host.clone();
        let request = PurchaseRequest {
            product_id: product_id.to_owned(),
            quantity: options.quantity,
            application_username: options.application_username.clone(),
        };
        Box::pin(async move {
            let raw = (host.purchase)(request).await.map_err(map_code)?;
            map_purchase(raw)
        })
    }

    fn finish(&self, purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>> {
        let host = self.host.clone();
        let transaction_id = purchase.id.clone();
        Box::pin(async move { (host.finish)(transaction_id).await.map_err(map_code) })
    }

    fn consume(&self, purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>> {
        let host = self.host.clone();
        let product_id = purchase.product_id.clone();
        let transaction_id = purchase.id.clone();
        Box::pin(async move {
            // Only a consumable may be consumed, and the raw kind is the host's to report;
            // StoreKit 2's finish is the consume.
            let raws = (host.products)(vec![product_id.clone()])
                .await
                .map_err(map_code)?;
            let kind = match raws.into_iter().find(|raw| raw.id == product_id) {
                Some(raw) => raw.kind,
                None => return Err(PurchasesError::new(ErrorKind::ProductNotFound)),
            };
            match kind {
                KIND_CONSUMABLE => {}
                KIND_NON_CONSUMABLE | KIND_SUBSCRIPTION => {
                    return Err(PurchasesError::new(ErrorKind::InvalidArgument));
                }
                other => return Err(PurchasesError::new(ErrorKind::Backend).with_status(other)),
            }
            (host.finish)(transaction_id).await.map_err(map_code)
        })
    }

    fn restore(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>> {
        let host = self.host.clone();
        Box::pin(async move {
            let raws = (host.restore)().await.map_err(map_code)?;
            raws.into_iter()
                .map(map_purchase)
                .collect::<Result<Vec<Purchase>, PurchasesError>>()
        })
    }

    fn pending(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>> {
        let host = self.host.clone();
        Box::pin(async move {
            let raws = (host.unfinished)().await.map_err(map_code)?;
            raws.into_iter()
                .map(map_purchase)
                .collect::<Result<Vec<Purchase>, PurchasesError>>()
        })
    }

    fn entitlements(&self) -> BackendFuture<'_, Result<Vec<Entitlement>, PurchasesError>> {
        let host = self.host.clone();
        Box::pin(async move {
            let raws = (host.entitlements)().await.map_err(map_code)?;
            raws.into_iter()
                .map(map_entitlement)
                .collect::<Result<Vec<Entitlement>, PurchasesError>>()
        })
    }

    fn country_code(&self) -> BackendFuture<'_, Result<String, PurchasesError>> {
        let host = self.host.clone();
        Box::pin(async move {
            let code = (host.storefront)().await.map_err(map_code)?;
            if code.is_empty() {
                return Err(PurchasesError::new(ErrorKind::Backend));
            }
            Ok(code)
        })
    }

    fn cancel(&self) {
        // Fire and forget: the dismissed call's answer still arrives, and lands once.
        (self.host.cancel)()
    }
}
