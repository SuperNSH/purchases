//! The reactive entitlements read: the answer is a signal that wakes its bindings only when it
//! changes.

use std::{cell::Cell, rc::Rc};

use nsh_core::platform::Lifecycle;
use nsh_core::prelude::*;

use crate::{error::PurchasesError, model::Entitlement, purchases::Purchases};

/// Where the entitlements read stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntitlementsState {
    /// The first read has not answered yet.
    Checking,
    /// The last read answered.
    Ready(Vec<Entitlement>),
    /// The last read failed. The error says why.
    Failed(PurchasesError),
}

/// A reactive handle on the entitlements. A `Copy` handle: bindings may hold it freely.
///
/// It reads when its owner starts, and again each time the app returns to the foreground,
/// because a subscription can lapse or renew out of band. A binding on
/// [`Entitlements::state`] runs again only when the answer changes.
#[derive(Clone, Copy)]
pub struct Entitlements {
    state: Signal<EntitlementsState>,
    refresh: Stored<Rc<dyn Fn()>>,
}

impl Entitlements {
    /// The current state. Reading it in a binding subscribes to changes.
    pub fn state(&self) -> EntitlementsState {
        self.state.get()
    }

    /// Reads again now, without waiting for the app to return to the foreground.
    pub fn refresh(&self) {
        let check = self.refresh.get();
        check();
    }
}

/// The store's entitlements, read reactively in the current owner. See [`Entitlements`].
pub fn use_entitlements(purchases: &Purchases) -> Entitlements {
    let purchases = purchases.clone();
    let owner = Owner::current();
    let state = signal(EntitlementsState::Checking);
    // Dropping a task cancels it, so a new read replaces a read still in flight.
    let inflight = stored(None::<Spawned<()>>);
    // The handle can outlive its owner, and a read then has no owner to land in.
    let gone = Rc::new(Cell::new(false));
    let check: Rc<dyn Fn()> = {
        let gone = Rc::clone(&gone);
        Rc::new(move || {
            if gone.get() {
                return;
            }
            let purchases = purchases.clone();
            let task = owner.run(|| {
                spawn(async move {
                    let answer = match purchases.entitlements().await {
                        Ok(entitlements) => EntitlementsState::Ready(entitlements),
                        Err(error) => EntitlementsState::Failed(error),
                    };
                    state.set(answer);
                })
            });
            drop(inflight.replace(Some(task)));
        })
    };
    let on_foreground = check.clone();
    effect(move || {
        if lifecycle() == Lifecycle::Active {
            on_foreground();
        }
    });
    cleanup({
        let gone = Rc::clone(&gone);
        move || {
            gone.set(true);
            drop(inflight.replace(None));
        }
    });
    Entitlements {
        state,
        refresh: stored(check),
    }
}
