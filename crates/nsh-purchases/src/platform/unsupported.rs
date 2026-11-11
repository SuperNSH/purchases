//! A target with no store backend, the web included: nothing is available, and no purchase
//! can be made.

use std::future::ready;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, PurchasesError},
    model::{Entitlement, Product, Purchase},
    options::PurchaseOptions,
};

pub(crate) struct NativeBackend;

impl Backend for NativeBackend {
    fn is_available(&self) -> BackendFuture<'_, Result<bool, PurchasesError>> {
        Box::pin(ready(Ok::<bool, PurchasesError>(false)))
    }

    fn initialize(&self) -> BackendFuture<'_, Result<(), PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn products(&self, _ids: &[String]) -> BackendFuture<'_, Result<Vec<Product>, PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn purchase(
        &self,
        _product_id: &str,
        _options: &PurchaseOptions,
    ) -> BackendFuture<'_, Result<Purchase, PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn finish(&self, _purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn consume(&self, _purchase: &Purchase) -> BackendFuture<'_, Result<(), PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn restore(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn pending(&self) -> BackendFuture<'_, Result<Vec<Purchase>, PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn entitlements(&self) -> BackendFuture<'_, Result<Vec<Entitlement>, PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn country_code(&self) -> BackendFuture<'_, Result<String, PurchasesError>> {
        Box::pin(ready(Err(PurchasesError::new(ErrorKind::NotAvailable))))
    }

    fn cancel(&self) {}
}
