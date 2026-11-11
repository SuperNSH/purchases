//! The store as TSX sees it: `NativePurchases`, a class whose async members reject with a
//! `code` on failure. Results are plain objects; the store connection stays on the Rust side.

use std::time::Duration;

use nsh_core::prelude::*;

use crate::{
    error::{ErrorKind, PurchasesError},
    model::{Entitlement, Product, ProductKind, Purchase, PurchaseState},
    options::PurchaseOptions,
    purchases::Purchases,
};

/// The options of one purchase, from TSX. Unset fields keep the defaults.
#[derive(JsObject, Clone, Debug, Default)]
pub struct NativePurchaseOptions {
    /// How many of the product to buy. The default is one.
    pub quantity: Option<u32>,
    /// Declared to the store as the buyer's app-side username. Apple reads it; others ignore it.
    pub application_username: Option<String>,
    /// The longest the purchase waits for its answer, in milliseconds. Off unless set; unused
    /// on the web.
    pub timeout_ms: Option<u64>,
}

/// A product the store lists, as TSX reads it.
#[derive(JsObject, Clone, Debug)]
pub struct NativeProduct {
    /// The store's id.
    pub id: String,
    /// The store's display name.
    pub title: String,
    /// The store's description.
    pub description: String,
    /// The price as the store shows it, such as `"4.99"`.
    pub price: String,
    /// The storefront's ISO currency code, when the store names one.
    #[prop(skip_none)]
    pub currency_code: Option<String>,
    /// `"consumable"`, `"nonConsumable"` or `"subscription"`.
    pub kind: String,
}

/// One purchase, as TSX reads it.
#[derive(JsObject, Clone, Debug)]
pub struct NativePurchase {
    /// The store's transaction id.
    pub id: String,
    /// The product's id.
    pub product_id: String,
    /// `"pending"`, `"purchased"` or `"restored"`.
    pub state: String,
    /// The store's verification payload, for app-side validation.
    pub receipt: String,
    /// When the store says the payment happened, in milliseconds since the epoch.
    #[prop(skip_none)]
    pub purchased_at_ms: Option<u64>,
}

/// A right an owned product grants, as TSX reads it.
#[derive(JsObject, Clone, Debug)]
pub struct NativeEntitlement {
    /// The product's id.
    pub product_id: String,
    /// Whether the right holds right now.
    pub active: bool,
    /// When the right ends, in milliseconds since the epoch; subscriptions only.
    #[prop(skip_none)]
    pub expires_at_ms: Option<u64>,
}

/// A store handle. Create one and keep it; every handle shares the platform's purchase gate.
#[class]
pub struct NativePurchases {
    purchases: Purchases,
}

#[class]
impl NativePurchases {
    /// The platform's store.
    pub fn new() -> NativePurchases {
        NativePurchases {
            purchases: Purchases::new(),
        }
    }

    /// Whether a store exists on this target.
    pub async fn is_available(this: Instance<Self>) -> Result<bool, JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        purchases.is_available().await.map_err(into_js)
    }

    /// Connects to the store. `Ok` once connected; further calls are `Ok` without asking again.
    pub async fn initialize(this: Instance<Self>) -> Result<(), JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        purchases.initialize().await.map_err(into_js)
    }

    /// The products the store lists for `ids`, found ones only.
    pub async fn products(
        this: Instance<Self>,
        ids: Vec<String>,
    ) -> Result<Vec<NativeProduct>, JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        let products = purchases.products(&ids).await.map_err(into_js)?;
        Ok(products.into_iter().map(native_product).collect())
    }

    /// Buys `productId`. Resolves once, with the purchase or a coded rejection.
    pub async fn purchase(
        this: Instance<Self>,
        product_id: String,
        options: Option<NativePurchaseOptions>,
    ) -> Result<NativePurchase, JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        let options = to_purchase_options(options.unwrap_or_default());
        let purchase = purchases
            .purchase(&product_id, options)
            .await
            .map_err(into_js)?;
        Ok(native_purchase(purchase))
    }

    /// Finishes the purchase: Apple's finish, Android's acknowledge.
    pub async fn finish(this: Instance<Self>, purchase: NativePurchase) -> Result<(), JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        let purchase = model_purchase(purchase)?;
        purchases.finish(&purchase).await.map_err(into_js)
    }

    /// Consumes a consumable's purchase, so it can be bought again.
    pub async fn consume(this: Instance<Self>, purchase: NativePurchase) -> Result<(), JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        let purchase = model_purchase(purchase)?;
        purchases.consume(&purchase).await.map_err(into_js)
    }

    /// Returns the owned purchases, as restore delivers them.
    pub async fn restore(this: Instance<Self>) -> Result<Vec<NativePurchase>, JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        let restored = purchases.restore().await.map_err(into_js)?;
        Ok(restored.into_iter().map(native_purchase).collect())
    }

    /// The unfinished purchases: the queue awaiting finish, consume or approval.
    pub async fn pending(this: Instance<Self>) -> Result<Vec<NativePurchase>, JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        let pending = purchases.pending().await.map_err(into_js)?;
        Ok(pending.into_iter().map(native_purchase).collect())
    }

    /// The rights the owned products grant right now.
    pub async fn entitlements(this: Instance<Self>) -> Result<Vec<NativeEntitlement>, JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        let entitlements = purchases.entitlements().await.map_err(into_js)?;
        Ok(entitlements.into_iter().map(native_entitlement).collect())
    }

    /// The storefront's ISO country code, such as `"US"`.
    pub async fn country_code(this: Instance<Self>) -> Result<String, JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        purchases.country_code().await.map_err(into_js)
    }

    /// Cancels the running purchase. Resolves `true` when a purchase was running.
    pub async fn cancel(this: Instance<Self>) -> Result<bool, JsError> {
        let purchases = this.with(|native| native.purchases.clone());
        purchases.cancel().await.map_err(into_js)
    }
}

impl Default for NativePurchases {
    fn default() -> Self {
        Self::new()
    }
}

/// The error a rejection carries: its message, and the `code` apps match on.
fn into_js(error: PurchasesError) -> JsError {
    JsError::Error(error.kind().message().to_owned()).with_code(error.code())
}

fn invalid(message: &str) -> JsError {
    JsError::Error(message.to_owned()).with_code(ErrorKind::InvalidArgument.code())
}

fn kind_name(kind: ProductKind) -> String {
    match kind {
        ProductKind::Consumable => "consumable",
        ProductKind::NonConsumable => "nonConsumable",
        ProductKind::Subscription => "subscription",
    }
    .to_owned()
}

fn state_name(state: PurchaseState) -> String {
    match state {
        PurchaseState::Pending => "pending",
        PurchaseState::Purchased => "purchased",
        PurchaseState::Restored => "restored",
    }
    .to_owned()
}

fn native_product(product: Product) -> NativeProduct {
    NativeProduct {
        id: product.id,
        title: product.title,
        description: product.description,
        price: product.price,
        currency_code: product.currency_code,
        kind: kind_name(product.kind),
    }
}

fn native_purchase(purchase: Purchase) -> NativePurchase {
    NativePurchase {
        id: purchase.id,
        product_id: purchase.product_id,
        state: state_name(purchase.state),
        receipt: purchase.receipt,
        purchased_at_ms: purchase.purchased_at_ms,
    }
}

fn native_entitlement(entitlement: Entitlement) -> NativeEntitlement {
    NativeEntitlement {
        product_id: entitlement.product_id,
        active: entitlement.active,
        expires_at_ms: entitlement.expires_at_ms,
    }
}

/// Turns a TSX purchase back into the crate's, refusing a state it does not know.
fn model_purchase(purchase: NativePurchase) -> Result<Purchase, JsError> {
    let state = match purchase.state.as_str() {
        "pending" => PurchaseState::Pending,
        "purchased" => PurchaseState::Purchased,
        "restored" => PurchaseState::Restored,
        _ => {
            return Err(invalid(
                "state must be \"pending\", \"purchased\" or \"restored\"",
            ));
        }
    };
    Ok(Purchase {
        id: purchase.id,
        product_id: purchase.product_id,
        state,
        receipt: purchase.receipt,
        purchased_at_ms: purchase.purchased_at_ms,
    })
}

/// Turns the TSX options into the crate's.
fn to_purchase_options(options: NativePurchaseOptions) -> PurchaseOptions {
    PurchaseOptions {
        quantity: options.quantity.unwrap_or(1),
        application_username: options.application_username,
        timeout: options.timeout_ms.map(Duration::from_millis),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_the_name_tsx_uses() {
        assert_eq!(kind_name(ProductKind::Consumable), "consumable");
        assert_eq!(kind_name(ProductKind::NonConsumable), "nonConsumable");
        assert_eq!(kind_name(ProductKind::Subscription), "subscription");
    }

    #[test]
    fn each_state_has_the_name_tsx_uses() {
        assert_eq!(state_name(PurchaseState::Pending), "pending");
        assert_eq!(state_name(PurchaseState::Purchased), "purchased");
        assert_eq!(state_name(PurchaseState::Restored), "restored");
    }

    fn tsx_purchase(state: &str) -> NativePurchase {
        NativePurchase {
            id: "fake-pro_unlock-1".to_owned(),
            product_id: "pro_unlock".to_owned(),
            state: state.to_owned(),
            receipt: "sandbox-receipt-pro_unlock".to_owned(),
            purchased_at_ms: None,
        }
    }

    #[test]
    fn an_unknown_state_is_refused_and_a_known_one_is_kept() {
        assert!(model_purchase(tsx_purchase("held")).is_err());

        let purchase = model_purchase(tsx_purchase("purchased")).ok();
        assert_eq!(
            purchase.map(|purchase| purchase.state),
            Some(PurchaseState::Purchased)
        );
    }

    #[test]
    fn unset_options_keep_the_defaults() {
        let options = to_purchase_options(NativePurchaseOptions::default());
        assert_eq!(options, PurchaseOptions::default());
    }
}
