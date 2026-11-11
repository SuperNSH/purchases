//! The seam between `Purchases` and a store: the queries, and one purchase that answers once.

use std::{future::Future, pin::Pin};

use crate::{
    error::PurchasesError,
    model::{Entitlement, Product, Purchase},
    options::PurchaseOptions,
};

/// A future a backend returns. Each call's future borrows the backend; it is `Send`, so a
/// caller may spawn the call.
pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What a platform provides. [`crate::Purchases`] adds the rules: initialization before any
/// store call, and one purchase at a time that resolves exactly once.
pub trait Backend: Send + Sync {
    /// Whether a store exists on this target. The one call that needs no `initialize`.
    fn is_available(&self) -> BackendFuture<'_, Result<bool, PurchasesError>>;

    /// Connects to the store. `Ok` means the store is ready for calls.
    fn initialize(&self) -> BackendFuture<'_, Result<(), PurchasesError>>;

    /// The products the store lists for `ids`, found ones only; unknown ids are silently
    /// dropped, as the references do.
    fn products(&self, ids: &[String]) -> BackendFuture<'_, Result<Vec<Product>, PurchasesError>>;

    /// Buys `product_id`. The future resolves with the store's answer exactly once: `Ok` for a
    /// paid or pending purchase, `Err` for a rejection the store codes.
    fn purchase(
        &self,
        product_id: &str,
        options: &PurchaseOptions,
    ) -> BackendFuture<'_, Result<Purchase, PurchasesError>>;

    /// Finishes the purchase: Apple's finish, Android's acknowledge.
    fn finish(&self, purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>>;

    /// Consumes a consumable's purchase, so it can be bought again. Consumables only.
    fn consume(&self, purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>>;

    /// Returns the owned purchases, as restore delivers them.
    fn restore(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>>;

    /// The unfinished purchases: the queue awaiting finish, consume or approval.
    fn pending(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>>;

    /// The rights the owned products grant right now.
    fn entitlements(&self) -> BackendFuture<'_, Result<Vec<Entitlement>, PurchasesError>>;

    /// The storefront's ISO country code, such as `"US"`.
    fn country_code(&self) -> BackendFuture<'_, Result<String, PurchasesError>>;

    /// Asks the store to dismiss the running purchase. Where the platform cannot dismiss it,
    /// this does nothing, and the purchase still resolves exactly once on its own.
    fn cancel(&self);
}
