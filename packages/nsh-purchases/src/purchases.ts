import type { PurchasesBackend } from "./backend";
import { asPurchasesError, purchasesError, type PurchasesError } from "./errors";
import { rustBackend } from "./rust";
import type { PurchaseOptions } from "./options";
import type { Entitlement, Product, Purchase } from "./store";

type Outcome = { ok: true; value: Purchase } | { ok: false; error: PurchasesError };

/**
 * The store's products, purchases and entitlements, with the references' names. It runs on the
 * Rust crate unless it is given another backend. One purchase runs at a time, every call resolves
 * exactly once, and each store call except `isAvailable` needs `initialize` first.
 *
 * Two instances hold one TS gate each; through the Rust crate every handle also shares the
 * platform's own gate, so a second purchase there rejects with `ERR_PURCHASES_IN_PROGRESS`.
 */
export class Purchases {
  private readonly backend: PurchasesBackend;
  private initialized = false;
  private busy = false;
  private waiting: ((outcome: Outcome) => void) | null = null;

  constructor(options: { backend?: PurchasesBackend } = {}) {
    this.backend = options.backend ?? rustBackend();
  }

  isAvailable(): Promise<boolean> {
    return this.settle(() => this.backend.isAvailable());
  }

  /** Connects to the store. Idempotent: a second call resolves at once without asking again. */
  async initialize(): Promise<void> {
    if (this.initialized) return;
    await this.settle(() => this.backend.initialize());
    this.initialized = true;
  }

  products(ids: string[]): Promise<Product[]> {
    if (ids.length === 0) {
      return Promise.reject(purchasesError("ERR_PURCHASES_INVALID_ARGUMENT"));
    }
    return this.initializedCall(() => this.backend.products(ids));
  }

  /**
   * Buys `productId`. Resolves once, with the purchase the store made, a `pending` one included.
   * Every failure rejects with a `PurchasesError` that has its code: an unknown product, an
   * already-owned non-consumable, a cancel, and so on. A second purchase while one runs rejects
   * with `ERR_PURCHASES_IN_PROGRESS`, and an empty product id, a zero or invalid `quantity` or a
   * zero `timeoutMs` with `ERR_PURCHASES_INVALID_ARGUMENT`.
   */
  purchase(productId: string, options: PurchaseOptions = {}): Promise<Purchase> {
    const invalid =
      productId.trim() === "" ||
      (options.quantity !== undefined && !(options.quantity > 0)) ||
      (options.timeoutMs !== undefined && !(options.timeoutMs > 0));
    if (invalid) {
      return Promise.reject(purchasesError("ERR_PURCHASES_INVALID_ARGUMENT"));
    }
    if (!this.initialized) {
      return Promise.reject(purchasesError("ERR_PURCHASES_NOT_INITIALIZED"));
    }
    if (this.busy) {
      return Promise.reject(purchasesError("ERR_PURCHASES_IN_PROGRESS"));
    }
    this.busy = true;
    return new Promise<Purchase>((resolve, reject) => {
      let settled = false;
      let timer: ReturnType<typeof setTimeout> | null = null;
      const settle = (outcome: Outcome) => {
        if (settled) return;
        settled = true;
        if (timer !== null) clearTimeout(timer);
        if (this.waiting === settle) this.waiting = null;
        if (outcome.ok) resolve(outcome.value);
        else reject(outcome.error);
      };
      this.waiting = settle;
      if (options.timeoutMs !== undefined) {
        timer = setTimeout(() => {
          // The watchdog frees the gate at once, so a new purchase may start while the store answers the old one.
          if (this.waiting === settle) {
            this.busy = false;
            this.waiting = null;
          }
          settle({ ok: false, error: purchasesError("ERR_PURCHASES_TIMEOUT") });
        }, options.timeoutMs);
      }
      let call: Promise<Purchase>;
      try {
        call = this.backend.purchase(productId, options);
      } catch (error: unknown) {
        // A backend may throw before returning its promise; that rejects the call like any failure.
        call = Promise.reject(error);
      }
      // The backend answer frees the gate unless a newer purchase owns it now.
      const answerFreesGate = () => this.waiting === settle || this.waiting === null;
      call.then(
        (value) => {
          if (answerFreesGate()) this.busy = false;
          settle({ ok: true, value });
        },
        (error: unknown) => {
          if (answerFreesGate()) this.busy = false;
          settle({ ok: false, error: asPurchasesError(error) });
        },
      );
    });
  }

  /** Finishes (Apple) or acknowledges (Android) `purchase`, once. */
  finish(purchase: Purchase): Promise<void> {
    return this.initializedCall(() => this.backend.finish(purchase));
  }

  /** Consumes `purchase`. Consumables only; a non-consumable rejects with `ERR_PURCHASES_INVALID_ARGUMENT`. */
  consume(purchase: Purchase): Promise<void> {
    return this.initializedCall(() => this.backend.consume(purchase));
  }

  restore(): Promise<Purchase[]> {
    return this.initializedCall(() => this.backend.restore());
  }

  pending(): Promise<Purchase[]> {
    return this.initializedCall(() => this.backend.pending());
  }

  entitlements(): Promise<Entitlement[]> {
    return this.initializedCall(() => this.backend.entitlements());
  }

  countryCode(): Promise<string> {
    return this.initializedCall(() => this.backend.countryCode());
  }

  /**
   * Cancels the running purchase. The pending `purchase` rejects with `ERR_PURCHASES_CANCELLED`
   * right away, and the gate frees with it; the store's late answer, if one still comes, is
   * dropped. Returns `true` when a purchase was running.
   */
  cancel(): boolean {
    const busy = this.busy;
    const waiting = this.waiting;
    this.waiting = null;
    this.busy = false;
    waiting?.({ ok: false, error: purchasesError("ERR_PURCHASES_CANCELLED") });
    if (busy) {
      try {
        this.backend.cancel();
      } catch {
        // The dismissal is best-effort: the call is already resolved, and the store's answer is dropped.
      }
    }
    return busy;
  }

  /** A store call that never claims the gate: initialized first, then the backend, once. */
  private initializedCall<T>(operation: () => Promise<T>): Promise<T> {
    if (!this.initialized) {
      return Promise.reject(purchasesError("ERR_PURCHASES_NOT_INITIALIZED"));
    }
    return this.settle(operation);
  }

  private async settle<T>(operation: () => Promise<T>): Promise<T> {
    try {
      return await operation();
    } catch (error) {
      throw asPurchasesError(error);
    }
  }
}
