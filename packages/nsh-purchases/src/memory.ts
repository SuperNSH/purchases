import type { PurchasesBackend } from "./backend";
import { purchasesError, type PurchaseErrorCode } from "./errors";
import type { PurchaseOptions } from "./options";
import type { Entitlement, Product, Purchase, PurchaseState } from "./store";

/** The sandbox time the fixtures stamp their purchases with, mirroring the crate's fake. */
const SANDBOX_NOW_MS = 1_735_689_600_000;
/** How long the fixture subscription's entitlement runs from the sandbox time. */
const MONTH_MS = 30 * 24 * 60 * 60 * 1000;

const FIXTURES: Product[] = [
  {
    id: "pro_unlock",
    title: "Pro Unlock",
    description: "Every pro feature, owned for good.",
    price: "4.99",
    currencyCode: "USD",
    kind: "nonConsumable",
  },
  {
    id: "coins_100",
    title: "100 Coins",
    description: "A bag of 100 coins, consumed on use.",
    price: "0.99",
    currencyCode: "USD",
    kind: "consumable",
  },
  {
    id: "pro_monthly",
    title: "Pro Monthly",
    description: "Every pro feature, billed monthly.",
    price: "1.99",
    currencyCode: "USD",
    kind: "subscription",
  },
];

/** The answers a memory backend gives. Each call answers what the config says. */
export interface MemoryPurchasesConfig {
  /** Whether a store exists. `false` answers `ERR_PURCHASES_NOT_AVAILABLE` like an unsupported target. */
  available?: boolean;
  /** The catalog `products` answers with. Defaults to the sandbox fixtures above. */
  products?: Product[];
  /** The storefront `countryCode` answers with. Defaults to `"US"`. */
  storefront?: string;
}

/** A backend the test drives: it finishes purchases and counts what was asked. */
export interface MemoryPurchases extends PurchasesBackend {
  /** Answers the waiting purchase: a state to resolve with, a full purchase, or a code to reject with. */
  finishPurchase(answer: PurchaseState | Purchase | PurchaseErrorCode): boolean;
  purchasePending(): boolean;
  purchases(): number;
  finishes(): number;
  consumes(): number;
  restores(): number;
  cancels(): number;
  lastProductIds(): string[] | undefined;
  lastProductId(): string | undefined;
  lastOptions(): PurchaseOptions | undefined;
  setNextPurchaseError(code: PurchaseErrorCode): void;
  setAvailable(value: boolean): void;
}

function isErrorCode(value: PurchaseState | PurchaseErrorCode): value is PurchaseErrorCode {
  return value.startsWith("ERR_");
}

/**
 * A memory backend, for tests and for apps that run without a store. It answers with the sandbox
 * fixtures the Rust fake uses: `pro_unlock`, `coins_100` and `pro_monthly`, with sandbox receipts.
 * Like a store, a cancel does not answer the purchase: the test answers it, as the store would.
 */
export function memoryBackend(config: MemoryPurchasesConfig = {}): MemoryPurchases {
  let available = config.available ?? true;
  const catalog = [...(config.products ?? FIXTURES)];
  const storefront = config.storefront ?? "US";
  let nextError: PurchaseErrorCode | null = null;
  let waiting: ((answer: PurchaseState | Purchase | PurchaseErrorCode) => void) | null = null;
  let started = 0;
  let finishCalls = 0;
  let consumeCalls = 0;
  let restoreCalls = 0;
  let cancelCalls = 0;
  let lastIds: string[] | undefined;
  let lastId: string | undefined;
  let lastOpts: PurchaseOptions | undefined;
  const delivered: Purchase[] = [];
  const finished = new Set<string>();
  const consumed = new Set<string>();

  const listed = (productId: string): Product | undefined => catalog.find((product) => product.id === productId);

  const entitlementFor = (purchase: Purchase): Entitlement => {
    const entitlement: Entitlement = { productId: purchase.productId, active: true };
    if (listed(purchase.productId)?.kind === "subscription") {
      entitlement.expiresAtMs = SANDBOX_NOW_MS + MONTH_MS;
    }
    return entitlement;
  };

  return {
    isAvailable: async () => available,

    initialize: async () => {
      if (!available) throw purchasesError("ERR_PURCHASES_NOT_AVAILABLE");
    },

    products: async (ids) => {
      lastIds = [...ids];
      if (!available) throw purchasesError("ERR_PURCHASES_NOT_AVAILABLE");
      return catalog.filter((product) => ids.includes(product.id));
    },

    purchase: (productId, options) => {
      lastId = productId;
      lastOpts = options;
      return new Promise<Purchase>((resolve, reject) => {
        if (!available) {
          reject(purchasesError("ERR_PURCHASES_NOT_AVAILABLE"));
          return;
        }
        if (nextError !== null) {
          const code = nextError;
          nextError = null;
          reject(purchasesError(code));
          return;
        }
        const product = listed(productId);
        if (product === undefined) {
          reject(purchasesError("ERR_PURCHASES_PRODUCT_NOT_FOUND"));
          return;
        }
        started += 1;
        const prepared: Purchase = {
          id: `fake-${product.id}-${started}`,
          productId: product.id,
          state: "purchased",
          receipt: `sandbox-receipt-${product.id}`,
          purchasedAtMs: SANDBOX_NOW_MS,
        };
        waiting = (answer) => {
          if (typeof answer === "object") {
            delivered.push(answer);
            resolve(answer);
            return;
          }
          if (isErrorCode(answer)) {
            reject(purchasesError(answer));
            return;
          }
          const purchase = { ...prepared, state: answer };
          delivered.push(purchase);
          resolve(purchase);
        };
      });
    },

    finish: async (purchase) => {
      if (!available) throw purchasesError("ERR_PURCHASES_NOT_AVAILABLE");
      if (!delivered.some((known) => known.id === purchase.id) || finished.has(purchase.id)) {
        throw purchasesError("ERR_PURCHASES_NOT_OWNED");
      }
      finished.add(purchase.id);
      finishCalls += 1;
    },

    consume: async (purchase) => {
      if (!available) throw purchasesError("ERR_PURCHASES_NOT_AVAILABLE");
      const kind = listed(purchase.productId)?.kind;
      const known = delivered.some((record) => record.id === purchase.id);
      if (kind === "consumable") {
        if (!known || consumed.has(purchase.id)) throw purchasesError("ERR_PURCHASES_NOT_OWNED");
        consumed.add(purchase.id);
        consumeCalls += 1;
        return;
      }
      if (kind === undefined) throw purchasesError("ERR_PURCHASES_NOT_OWNED");
      throw purchasesError("ERR_PURCHASES_INVALID_ARGUMENT");
    },

    restore: async () => {
      restoreCalls += 1;
      if (!available) throw purchasesError("ERR_PURCHASES_NOT_AVAILABLE");
      return delivered.map((purchase) => ({ ...purchase, state: "restored" as const }));
    },

    pending: async () => {
      if (!available) throw purchasesError("ERR_PURCHASES_NOT_AVAILABLE");
      return delivered
        .filter((purchase) => !finished.has(purchase.id) && !consumed.has(purchase.id))
        .map((purchase) => ({ ...purchase }));
    },

    entitlements: async () => {
      if (!available) throw purchasesError("ERR_PURCHASES_NOT_AVAILABLE");
      // A right comes from a delivered, unconsumed, non-pending purchase; the newest wins.
      const latest = new Map<string, Purchase>();
      for (const purchase of delivered) {
        if (consumed.has(purchase.id) || purchase.state === "pending") continue;
        latest.set(purchase.productId, purchase);
      }
      return [...latest.values()].map(entitlementFor);
    },

    countryCode: async () => {
      if (!available) throw purchasesError("ERR_PURCHASES_NOT_AVAILABLE");
      return storefront;
    },

    cancel: () => {
      cancelCalls += 1;
    },

    finishPurchase: (answer) => {
      const done = waiting;
      waiting = null;
      done?.(answer);
      return done !== null;
    },
    purchasePending: () => waiting !== null,
    purchases: () => started,
    finishes: () => finishCalls,
    consumes: () => consumeCalls,
    restores: () => restoreCalls,
    cancels: () => cancelCalls,
    lastProductIds: () => lastIds,
    lastProductId: () => lastId,
    lastOptions: () => lastOpts,
    setNextPurchaseError: (code) => {
      nextError = code;
    },
    setAvailable: (value) => {
      available = value;
    },
  };
}
