//! `Purchases`: the calls an app makes, with one purchase at a time and one answer per call.

use std::{
    sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError},
    time::Duration,
};

use tokio::sync::oneshot;

use crate::{
    backend::Backend,
    error::{ErrorKind, PurchasesError},
    model::{Entitlement, Product, Purchase},
    options::PurchaseOptions,
};

type Answer = Result<Purchase, PurchasesError>;

/// The purchase gate. `busy` lasts from a purchase's start until it resolves, so a second
/// purchase never stacks on one the store still shows.
///
/// `owner` names the generation that holds the gate, so the release of a call that timed out
/// and lost the gate cannot touch a newer call that took it over, and a late answer is dropped
/// with the channel it came through.
#[derive(Default)]
struct Gate {
    locked: Mutex<GateState>,
}

#[derive(Default)]
struct GateState {
    busy: bool,
    next_id: u64,
    owner: Option<u64>,
    waiting: Option<oneshot::Sender<Answer>>,
    initialized: bool,
}

impl Gate {
    fn lock(&self) -> MutexGuard<'_, GateState> {
        self.locked.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Purchases through a store, as the references' apps make them.
///
/// Clones share one purchase gate. [`Purchases::new`] returns the platform's shared handle, so
/// two calls in one app share it too.
#[derive(Clone)]
pub struct Purchases {
    backend: Arc<dyn Backend>,
    state: Arc<Gate>,
}

impl Purchases {
    /// The platform's store, with one purchase gate for the whole process.
    pub fn new() -> Self {
        static PLATFORM: OnceLock<Purchases> = OnceLock::new();
        PLATFORM
            .get_or_init(|| Self::from_backend(crate::platform::default_backend()))
            .clone()
    }

    /// The store through `backend`, with its own purchase gate. Tests and fakes use this.
    pub fn with_backend(backend: Arc<dyn Backend>) -> Self {
        Self::from_backend(backend)
    }

    fn from_backend(backend: Arc<dyn Backend>) -> Self {
        Self {
            backend,
            state: Arc::new(Gate::default()),
        }
    }

    /// Whether a store exists on this target.
    pub async fn is_available(&self) -> Result<bool, PurchasesError> {
        self.backend.is_available().await
    }

    /// Connects to the store. `Ok` once connected; further calls are `Ok` without asking again.
    pub async fn initialize(&self) -> Result<(), PurchasesError> {
        if self.state.lock().initialized {
            return Ok(());
        }
        self.backend.initialize().await?;
        self.state.lock().initialized = true;
        Ok(())
    }

    /// The products the store lists for `ids`, found ones only; unknown ids are dropped.
    pub async fn products(&self, ids: &[String]) -> Result<Vec<Product>, PurchasesError> {
        if ids.is_empty() {
            return Err(PurchasesError::new(ErrorKind::InvalidArgument));
        }
        self.require_ready()?;
        self.backend.products(ids).await
    }

    /// Buys `product_id` and waits for the store's answer.
    ///
    /// Every outcome resolves exactly once: `Ok` with the purchase — a pending one included,
    /// as nothing failed — or an error with a code: a cancel, a rejection, a timeout. A second
    /// call while one runs gets `ERR_PURCHASES_IN_PROGRESS`; `cancel` resolves the running call
    /// with `ERR_PURCHASES_CANCELLED` and a late store answer is dropped. Dropping the future
    /// asks the store to dismiss the request. With `PurchaseOptions::timeout` set, the call
    /// gives up at the deadline with `ERR_PURCHASES_TIMEOUT` and the store's late answer is
    /// dropped.
    pub async fn purchase(
        &self,
        product_id: &str,
        options: PurchaseOptions,
    ) -> Result<Purchase, PurchasesError> {
        if product_id.trim().is_empty()
            || options.quantity == 0
            || options.timeout == Some(Duration::ZERO)
        {
            return Err(PurchasesError::new(ErrorKind::InvalidArgument));
        }
        self.require_ready()?;
        let (sender, receiver) = oneshot::channel();
        let generation = {
            let mut gate = self.state.lock();
            if gate.busy {
                return Err(PurchasesError::new(ErrorKind::InProgress));
            }
            gate.busy = true;
            let generation = gate.next_id;
            gate.next_id += 1;
            gate.owner = Some(generation);
            gate.waiting = Some(sender);
            generation
        };
        let mut owner = Owner {
            backend: Arc::clone(&self.backend),
            state: Arc::clone(&self.state),
            generation,
            armed: true,
        };
        let call = self.backend.purchase(product_id, &options);
        // The timer needs tokio's time driver, which wasm32 has no runtime for; there the
        // option is accepted and unused, as the web has no store that could time out.
        #[cfg(not(target_arch = "wasm32"))]
        let answer = match options.timeout {
            Some(timeout) => {
                tokio::select! {
                    answer = receiver => {
                        answer.unwrap_or_else(|_| Err(PurchasesError::new(ErrorKind::Backend)))
                    }
                    answer = call => answer,
                    _ = tokio::time::sleep(timeout) => {
                        Err(PurchasesError::new(ErrorKind::Timeout))
                    }
                }
            }
            None => tokio::select! {
                answer = receiver => {
                    answer.unwrap_or_else(|_| Err(PurchasesError::new(ErrorKind::Backend)))
                }
                answer = call => answer,
            },
        };
        #[cfg(target_arch = "wasm32")]
        let answer = tokio::select! {
            answer = receiver => {
                answer.unwrap_or_else(|_| Err(PurchasesError::new(ErrorKind::Backend)))
            }
            answer = call => answer,
        };
        owner.armed = false;
        release(&self.state, generation);
        answer
    }

    /// Finishes the purchase: Apple's finish, Android's acknowledge.
    pub async fn finish(&self, purchase: &Purchase) -> Result<(), PurchasesError> {
        self.require_ready()?;
        self.backend.finish(purchase).await
    }

    /// Consumes a consumable's purchase, so it can be bought again.
    pub async fn consume(&self, purchase: &Purchase) -> Result<(), PurchasesError> {
        self.require_ready()?;
        self.backend.consume(purchase).await
    }

    /// Returns the owned purchases, as restore delivers them.
    pub async fn restore(&self) -> Result<Vec<Purchase>, PurchasesError> {
        self.require_ready()?;
        self.backend.restore().await
    }

    /// The unfinished purchases: the queue awaiting finish, consume or approval.
    pub async fn pending(&self) -> Result<Vec<Purchase>, PurchasesError> {
        self.require_ready()?;
        self.backend.pending().await
    }

    /// The rights the owned products grant right now.
    pub async fn entitlements(&self) -> Result<Vec<Entitlement>, PurchasesError> {
        self.require_ready()?;
        self.backend.entitlements().await
    }

    /// The storefront's ISO country code, such as `"US"`.
    pub async fn country_code(&self) -> Result<String, PurchasesError> {
        self.require_ready()?;
        self.backend.country_code().await
    }

    /// Cancels the running purchase. The pending `purchase` call resolves with
    /// `ERR_PURCHASES_CANCELLED`, and the store's late answer, if one still comes, is dropped.
    /// Returns `true` when a purchase was running.
    pub async fn cancel(&self) -> Result<bool, PurchasesError> {
        let (waiting, busy) = {
            let mut gate = self.state.lock();
            (gate.waiting.take(), gate.busy)
        };
        if let Some(sender) = waiting {
            let _ = sender.send(Err(PurchasesError::new(ErrorKind::Cancelled)));
        }
        if busy {
            self.backend.cancel();
        }
        Ok(busy)
    }

    /// Every store call except `is_available` needs an `initialize` that succeeded.
    fn require_ready(&self) -> Result<(), PurchasesError> {
        if self.state.lock().initialized {
            Ok(())
        } else {
            Err(PurchasesError::new(ErrorKind::NotInitialized))
        }
    }
}

impl Default for Purchases {
    fn default() -> Self {
        Self::new()
    }
}

/// Frees the gate for the call of `generation`, whichever way it resolved. A generation the
/// gate no longer holds is left alone, so a stale release cannot free a newer call's claim.
fn release(state: &Gate, generation: u64) {
    let mut gate = state.lock();
    if gate.owner == Some(generation) {
        gate.busy = false;
        gate.owner = None;
        gate.waiting = None;
    }
}

/// Asks the store to dismiss the request if the call's future is dropped before the answer.
struct Owner {
    backend: Arc<dyn Backend>,
    state: Arc<Gate>,
    generation: u64,
    armed: bool,
}

impl Drop for Owner {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        // The store call went away with the future, so its answer can never arrive: free the
        // gate for the next purchase and ask the store to dismiss the request.
        release(&self.state, self.generation);
        self.backend.cancel();
    }
}
