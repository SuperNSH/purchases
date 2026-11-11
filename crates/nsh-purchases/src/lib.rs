//! In-app purchases for Native Shell apps.
//!
//! [`Purchases`] wraps the store: query products, buy, finish, restore. Each call resolves
//! exactly once, with a value or a [`PurchasesError`] carrying a stable code.

pub mod android;
pub mod apple;

mod backend;
mod entitlements;
mod error;
mod fake;
mod model;
mod native;
mod options;
mod platform;
mod purchases;

use nsh_core::prelude::*;

pub use backend::{Backend, BackendFuture};
pub use entitlements::{Entitlements, EntitlementsState, use_entitlements};
pub use error::{ErrorKind, PurchasesError};
pub use fake::FakeBackend;
pub use model::{Entitlement, Product, ProductKind, Purchase, PurchaseState};
pub use native::{NativePurchaseOptions, NativePurchases};
pub use options::PurchaseOptions;
pub use purchases::Purchases;

/// The crate's UI set: what TSX imports from `nsh-purchases/native`.
#[native_module("nsh-purchases/native")]
fn native(registry: &mut Registry) {
    registry.class::<NativePurchases>();
}
