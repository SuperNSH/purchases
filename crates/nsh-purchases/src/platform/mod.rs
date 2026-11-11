//! The platform backends. A host that registers at startup provides the store; a target with
//! no host registered, the web included, reports that nothing is available.

mod unsupported;

use std::sync::Arc;

use crate::backend::Backend;

/// The backend of the target this build runs on: the first host that has registered.
pub(crate) fn default_backend() -> Arc<dyn Backend> {
    if let Some(backend) = crate::apple::native_backend() {
        return backend;
    }
    if let Some(backend) = crate::android::native_backend() {
        return backend;
    }
    Arc::new(unsupported::NativeBackend)
}
