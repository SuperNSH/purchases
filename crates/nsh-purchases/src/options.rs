//! The options of one purchase.

use std::time::Duration;

/// The options of one purchase. Start from [`PurchaseOptions::default`] and set what you need.
///
/// Options a store cannot apply are accepted and ignored, as `docs/parity.md` lists them.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseOptions {
    /// How many of the product to buy. The default is one; a store that caps it takes the cap.
    pub quantity: u32,
    /// Declared to the store as the buyer's app-side username. Apple reads it; others ignore it.
    pub application_username: Option<String>,
    /// The longest the purchase waits for its answer before resolving with
    /// `ERR_PURCHASES_TIMEOUT`. Opt-in and off by default; unused on the web.
    pub timeout: Option<Duration>,
}

impl Default for PurchaseOptions {
    /// One of the product, no username, no timeout.
    fn default() -> Self {
        Self {
            quantity: 1,
            application_username: None,
            timeout: None,
        }
    }
}
