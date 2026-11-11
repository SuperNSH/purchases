/** The codes a purchases rejection carries. Apps match on `code`. */
export type PurchaseErrorCode =
  | "ERR_PURCHASES_NOT_AVAILABLE"
  | "ERR_PURCHASES_NOT_INITIALIZED"
  | "ERR_PURCHASES_IN_PROGRESS"
  | "ERR_PURCHASES_TIMEOUT"
  | "ERR_PURCHASES_CANCELLED"
  | "ERR_PURCHASES_PRODUCT_NOT_FOUND"
  | "ERR_PURCHASES_ALREADY_OWNED"
  | "ERR_PURCHASES_NOT_OWNED"
  | "ERR_PURCHASES_PAYMENT_INVALID"
  | "ERR_PURCHASES_NOT_ALLOWED"
  | "ERR_PURCHASES_NETWORK"
  | "ERR_PURCHASES_INVALID_ARGUMENT"
  | "ERR_PURCHASES_BACKEND";

/** Same wording as the Rust crate's messages. None holds the receipt or any user data. */
const MESSAGES: Record<PurchaseErrorCode, string> = {
  ERR_PURCHASES_NOT_AVAILABLE:
    "In-app purchases are not available on this target, or no host has registered.",
  ERR_PURCHASES_NOT_INITIALIZED:
    "Call initialize() and wait for it to succeed before using the store.",
  ERR_PURCHASES_IN_PROGRESS: "Another purchase is already running. Wait for it to finish.",
  ERR_PURCHASES_TIMEOUT: "The purchase timed out waiting for its answer. Try again.",
  ERR_PURCHASES_CANCELLED: "The purchase was cancelled.",
  ERR_PURCHASES_PRODUCT_NOT_FOUND: "The store has no product with that id.",
  ERR_PURCHASES_ALREADY_OWNED: "The product is already owned.",
  ERR_PURCHASES_NOT_OWNED: "The purchase is unknown or already finished.",
  ERR_PURCHASES_PAYMENT_INVALID: "The store rejected the payment.",
  ERR_PURCHASES_NOT_ALLOWED: "The purchase is not allowed on this device or account.",
  ERR_PURCHASES_NETWORK: "The store connection failed. Check the network and try again.",
  ERR_PURCHASES_INVALID_ARGUMENT:
    "An argument is empty or not allowed, such as a zero quantity or a non-consumable consume.",
  ERR_PURCHASES_BACKEND: "The platform's store service failed.",
};

const KNOWN = new Set<string>(Object.keys(MESSAGES));

/** A rejection of the purchases API. `code` is the stable name; the message says what to do. */
export class PurchasesError extends Error {
  readonly code: PurchaseErrorCode;

  constructor(code: PurchaseErrorCode) {
    super(MESSAGES[code]);
    this.name = "PurchasesError";
    this.code = code;
  }
}

/** The error for a known code. */
export function purchasesError(code: PurchaseErrorCode): PurchasesError {
  return new PurchasesError(code);
}

/**
 * Turns whatever the native side or a host threw into a coded `PurchasesError`. A host's
 * rejection carries `ERR_NSH_HOST_ERROR`, and its message starts with the code, as
 * `ERR_PURCHASES_CANCELLED: text`. Anything unknown is a backend failure.
 */
export function asPurchasesError(error: unknown): PurchasesError {
  if (error instanceof PurchasesError) return error;
  const raw = (error ?? {}) as { code?: unknown; message?: unknown };
  const code = typeof raw.code === "string" ? raw.code : "";
  const message = typeof raw.message === "string" ? raw.message : "";
  if (code === "ERR_NSH_HOST_ERROR") {
    const named = /^(ERR_PURCHASES_[A-Z_]+): /.exec(message)?.[1];
    return named !== undefined && KNOWN.has(named)
      ? purchasesError(named as PurchaseErrorCode)
      : purchasesError("ERR_PURCHASES_BACKEND");
  }
  return KNOWN.has(code) ? purchasesError(code as PurchaseErrorCode) : purchasesError("ERR_PURCHASES_BACKEND");
}
