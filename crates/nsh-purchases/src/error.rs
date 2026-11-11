//! The errors a purchases call reports. Each has a stable code; a message says what to do.

use std::fmt;

/// What went wrong, as a stable code; see [`PurchasesError::code`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// No store on this target, or no host registered its callbacks.
    NotAvailable,
    /// A store call ran before `initialize` succeeded.
    NotInitialized,
    /// Another purchase is already running. One purchase runs at a time.
    InProgress,
    /// The opt-in timeout passed with the purchase still unanswered.
    Timeout,
    /// The user, the system or the app cancelled the purchase.
    Cancelled,
    /// The id names no product the store has.
    ProductNotFound,
    /// The non-consumable or active subscription is already owned.
    AlreadyOwned,
    /// The purchase is unknown or already finished, so it cannot be finished or consumed.
    NotOwned,
    /// The store rejected the payment itself.
    PaymentInvalid,
    /// The purchase is not allowed, such as parental controls or revoked cloud rights.
    NotAllowed,
    /// The store connection failed.
    Network,
    /// An argument is empty or not allowed, such as a zero quantity, a zero timeout or a
    /// non-consumable consume.
    InvalidArgument,
    /// The platform or host failed; `status` holds its own code where it gives one.
    Backend,
}

/// A failed purchases call or store query.
///
/// `Display` names the code's message. Neither holds the receipt or any user data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}", .kind.message())]
pub struct PurchasesError {
    kind: ErrorKind,
    status: Option<i64>,
}

impl PurchasesError {
    /// An error of `kind`, with no platform status.
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, status: None }
    }

    /// The same error, carrying the platform's own status code.
    #[must_use]
    pub fn with_status(mut self, status: i64) -> Self {
        self.status = Some(status);
        self
    }

    /// What went wrong.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The platform's status code, when the platform gave one.
    pub fn status(&self) -> Option<i64> {
        self.status
    }

    /// The stable code, such as `ERR_PURCHASES_CANCELLED`. Apps match on this.
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }
}

impl ErrorKind {
    /// The stable code of this kind.
    pub fn code(self) -> &'static str {
        match self {
            ErrorKind::NotAvailable => "ERR_PURCHASES_NOT_AVAILABLE",
            ErrorKind::NotInitialized => "ERR_PURCHASES_NOT_INITIALIZED",
            ErrorKind::InProgress => "ERR_PURCHASES_IN_PROGRESS",
            ErrorKind::Timeout => "ERR_PURCHASES_TIMEOUT",
            ErrorKind::Cancelled => "ERR_PURCHASES_CANCELLED",
            ErrorKind::ProductNotFound => "ERR_PURCHASES_PRODUCT_NOT_FOUND",
            ErrorKind::AlreadyOwned => "ERR_PURCHASES_ALREADY_OWNED",
            ErrorKind::NotOwned => "ERR_PURCHASES_NOT_OWNED",
            ErrorKind::PaymentInvalid => "ERR_PURCHASES_PAYMENT_INVALID",
            ErrorKind::NotAllowed => "ERR_PURCHASES_NOT_ALLOWED",
            ErrorKind::Network => "ERR_PURCHASES_NETWORK",
            ErrorKind::InvalidArgument => "ERR_PURCHASES_INVALID_ARGUMENT",
            ErrorKind::Backend => "ERR_PURCHASES_BACKEND",
        }
    }

    /// What to do about it, in one sentence.
    pub fn message(self) -> &'static str {
        match self {
            ErrorKind::NotAvailable => {
                "In-app purchases are not available on this target, or no host has registered."
            }
            ErrorKind::NotInitialized => {
                "Call initialize() and wait for it to succeed before using the store."
            }
            ErrorKind::InProgress => "Another purchase is already running. Wait for it to finish.",
            ErrorKind::Timeout => "The purchase timed out waiting for its answer. Try again.",
            ErrorKind::Cancelled => "The purchase was cancelled.",
            ErrorKind::ProductNotFound => "The store has no product with that id.",
            ErrorKind::AlreadyOwned => "The product is already owned.",
            ErrorKind::NotOwned => "The purchase is unknown or already finished.",
            ErrorKind::PaymentInvalid => "The store rejected the payment.",
            ErrorKind::NotAllowed => "The purchase is not allowed on this device or account.",
            ErrorKind::Network => "The store connection failed. Check the network and try again.",
            ErrorKind::InvalidArgument => {
                "An argument is empty or not allowed, such as a zero quantity or a non-consumable consume."
            }
            ErrorKind::Backend => "The platform's store service failed.",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
