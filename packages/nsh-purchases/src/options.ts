/** The options of one purchase. */
export interface PurchaseOptions {
  /** How many of a consumable to buy. Stores that cap it buy the cap. Defaults to 1. */
  quantity?: number;
  /** The store's own name for the user, for receipts and fraud checks. Apple only. */
  applicationUsername?: string;
  /** Milliseconds until the call gives up with `ERR_PURCHASES_TIMEOUT`. Undefined waits for the store. */
  timeoutMs?: number;
}
