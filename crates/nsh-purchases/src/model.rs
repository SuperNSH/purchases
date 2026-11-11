//! The store's records: the products it lists, the purchases it takes, the rights they grant.

/// A product's kind, as the store lists it.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductKind {
    /// Bought, consumed and buyable again, such as a bag of coins.
    Consumable,
    /// Bought once and owned for good, such as an unlock.
    NonConsumable,
    /// Owned while the recurring payment stays active.
    Subscription,
}

/// Where a purchase stands.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PurchaseState {
    /// Awaiting approval, such as ask-a-parent. Nothing failed; `pending` surfaces it later.
    Pending,
    /// Paid, waiting to be finished or consumed.
    Purchased,
    /// Returned by restore for something already owned.
    Restored,
}

/// A product the store lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    /// The store's id, as `products` was asked for it.
    pub id: String,
    /// The store's display name.
    pub title: String,
    /// The store's description.
    pub description: String,
    /// The price as the store shows it, such as `"4.99"`.
    pub price: String,
    /// The storefront's ISO currency code, when the store names one.
    pub currency_code: Option<String>,
    /// What kind of product this is.
    pub kind: ProductKind,
}

/// One purchase: a payment the store recorded against a product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purchase {
    /// The store's transaction id.
    pub id: String,
    /// The product's id.
    pub product_id: String,
    /// Where the purchase stands.
    pub state: PurchaseState,
    /// The store's verification payload, for app-side validation.
    pub receipt: String,
    /// When the store says the payment happened, in milliseconds since the epoch.
    pub purchased_at_ms: Option<u64>,
}

/// A right an owned product grants right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entitlement {
    /// The product's id.
    pub product_id: String,
    /// Whether the right holds right now.
    pub active: bool,
    /// When the right ends, in milliseconds since the epoch; subscriptions only.
    pub expires_at_ms: Option<u64>,
}
