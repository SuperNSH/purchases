import { expect, test } from "nsh/test";
import {
  memoryBackend,
  Purchases,
  type Purchase,
  type PurchaseErrorCode,
  type PurchasesBackend,
} from "../src";

/** Lets the answer of a finished backend call reach the gate, which frees itself a microtask later. */
async function delivered(): Promise<void> {
  for (let turn = 0; turn < 4; turn += 1) await Promise.resolve();
}

/** A sandbox purchase of `productId`, as the store would report it. */
function storePurchase(productId: string, state: Purchase["state"] = "purchased"): Purchase {
  return {
    id: `tx-${productId}`,
    productId,
    state,
    receipt: `sandbox-receipt-${productId}`,
    purchasedAtMs: 1_700_000_000_000,
  };
}

const CODES: PurchaseErrorCode[] = [
  "ERR_PURCHASES_NOT_AVAILABLE",
  "ERR_PURCHASES_NOT_INITIALIZED",
  "ERR_PURCHASES_IN_PROGRESS",
  "ERR_PURCHASES_TIMEOUT",
  "ERR_PURCHASES_CANCELLED",
  "ERR_PURCHASES_PRODUCT_NOT_FOUND",
  "ERR_PURCHASES_ALREADY_OWNED",
  "ERR_PURCHASES_NOT_OWNED",
  "ERR_PURCHASES_PAYMENT_INVALID",
  "ERR_PURCHASES_NOT_ALLOWED",
  "ERR_PURCHASES_NETWORK",
  "ERR_PURCHASES_INVALID_ARGUMENT",
  "ERR_PURCHASES_BACKEND",
];

test("one purchase runs at a time, and the answer resolves its call once", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const first = purchases.purchase("coins_100", {});
  await expect(purchases.purchase("pro_unlock", {})).rejects.toMatchObject({
    code: "ERR_PURCHASES_IN_PROGRESS",
  });
  expect(backend.purchases()).toBe(1);
  expect(backend.finishPurchase("purchased")).toBe(true);
  expect((await first).productId).toBe("coins_100");
  expect(backend.finishPurchase("purchased")).toBe(false);
});

test("cancel resolves the running purchase as cancelled, and the late answer is dropped", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const first = purchases.purchase("pro_unlock", {});
  expect(purchases.cancel()).toBe(true);
  await expect(first).rejects.toMatchObject({ code: "ERR_PURCHASES_CANCELLED" });
  expect(backend.cancels()).toBe(1);

  // The store still answers after the cancel; that answer must be dropped whole.
  expect(backend.finishPurchase("purchased")).toBe(true);
  const next = purchases.purchase("coins_100", {});
  expect(backend.finishPurchase("purchased")).toBe(true);
  expect((await next).productId).toBe("coins_100");
});

test("a timeoutMs resolves with the timeout code and the gate recovers", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const timedOut = purchases.purchase("coins_100", { timeoutMs: 5 });
  await expect(timedOut).rejects.toMatchObject({ code: "ERR_PURCHASES_TIMEOUT" });
  const next = purchases.purchase("pro_unlock", {});
  expect(backend.finishPurchase("purchased")).toBe(true);
  expect((await next).productId).toBe("pro_unlock");
});

test("a late store answer after a timeout does not disturb the newer call", async () => {
  // One resolver per purchase, in call order, so a timed-out purchase can answer late.
  const answers: Array<(purchase: Purchase) => void> = [];
  const backend: PurchasesBackend = {
    isAvailable: async () => true,
    initialize: async () => {},
    products: async () => [],
    purchase: () =>
      new Promise<Purchase>((resolve) => {
        answers.push(resolve);
      }),
    finish: async () => {},
    consume: async () => {},
    restore: async () => [],
    pending: async () => [],
    entitlements: async () => [],
    countryCode: async () => "US",
    cancel: () => {},
  };
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const timedOut = purchases.purchase("coins_100", { timeoutMs: 5 });
  await expect(timedOut).rejects.toMatchObject({ code: "ERR_PURCHASES_TIMEOUT" });

  const newer = purchases.purchase("coins_100", {});
  answers[0](storePurchase("coins_100"));
  await delivered();
  await expect(purchases.purchase("coins_100", {})).rejects.toMatchObject({
    code: "ERR_PURCHASES_IN_PROGRESS",
  });
  answers[1](storePurchase("coins_100"));
  expect((await newer).productId).toBe("coins_100");
});

test("invalid arguments are refused without the store being asked", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  await expect(purchases.purchase("   ", {})).rejects.toMatchObject({
    code: "ERR_PURCHASES_INVALID_ARGUMENT",
  });
  for (const quantity of [0, -2]) {
    await expect(purchases.purchase("coins_100", { quantity })).rejects.toMatchObject({
      code: "ERR_PURCHASES_INVALID_ARGUMENT",
    });
  }
  for (const timeoutMs of [0, -5, Number.NaN]) {
    await expect(purchases.purchase("coins_100", { timeoutMs })).rejects.toMatchObject({
      code: "ERR_PURCHASES_INVALID_ARGUMENT",
    });
  }
  await expect(purchases.products([])).rejects.toMatchObject({
    code: "ERR_PURCHASES_INVALID_ARGUMENT",
  });
  expect(backend.purchases()).toBe(0);
});

test("calls before initialize are rejected as not initialized", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await expect(purchases.products(["pro_unlock"])).rejects.toMatchObject({
    code: "ERR_PURCHASES_NOT_INITIALIZED",
  });
  await expect(purchases.purchase("coins_100", {})).rejects.toMatchObject({
    code: "ERR_PURCHASES_NOT_INITIALIZED",
  });
  await expect(purchases.restore()).rejects.toMatchObject({
    code: "ERR_PURCHASES_NOT_INITIALIZED",
  });
  await expect(purchases.finish(storePurchase("pro_unlock"))).rejects.toMatchObject({
    code: "ERR_PURCHASES_NOT_INITIALIZED",
  });
  expect(await purchases.isAvailable()).toBe(true);
});

test("initialize is idempotent, and unknown product ids are dropped", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  await purchases.initialize();
  const products = await purchases.products(["pro_unlock", "not_a_product", "pro_monthly"]);
  expect(products.map((product) => product.id)).toEqual(["pro_unlock", "pro_monthly"]);
  expect(backend.lastProductIds()).toEqual(["pro_unlock", "not_a_product", "pro_monthly"]);
});

test("an unknown product rejects with product not found", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  await expect(purchases.purchase("not_a_product", {})).rejects.toMatchObject({
    code: "ERR_PURCHASES_PRODUCT_NOT_FOUND",
  });
  expect(backend.purchases()).toBe(0);
});

test("a pending purchase resolves pending, not as a failure", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const call = purchases.purchase("pro_monthly", {});
  expect(backend.finishPurchase("pending")).toBe(true);
  const purchase = await call;
  expect(purchase.state).toBe("pending");
  expect(purchase.productId).toBe("pro_monthly");
});

test("an already owned product rejects with its code, once", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  backend.setNextPurchaseError("ERR_PURCHASES_ALREADY_OWNED");
  await expect(purchases.purchase("pro_unlock", {})).rejects.toMatchObject({
    code: "ERR_PURCHASES_ALREADY_OWNED",
  });
  const next = purchases.purchase("coins_100", {});
  expect(backend.finishPurchase("purchased")).toBe(true);
  expect((await next).productId).toBe("coins_100");
});

test("every code reaches the caller as its code", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  for (const code of CODES) {
    if (code === "ERR_PURCHASES_IN_PROGRESS") continue;
    backend.setNextPurchaseError(code);
    await expect(purchases.purchase("coins_100", {})).rejects.toMatchObject({ code });
    await delivered();
  }
});

test("finishing marks the purchase finished once, and consuming consumes", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();

  const coins = purchases.purchase("coins_100", {});
  backend.finishPurchase("purchased");
  await purchases.consume(await coins);
  expect(backend.consumes()).toBe(1);

  const unlock = purchases.purchase("pro_unlock", {});
  backend.finishPurchase("purchased");
  await purchases.finish(await unlock);
  expect(backend.finishes()).toBe(1);
  await expect(purchases.finish(await unlock)).rejects.toMatchObject({
    code: "ERR_PURCHASES_NOT_OWNED",
  });
  await expect(purchases.consume(await unlock)).rejects.toMatchObject({
    code: "ERR_PURCHASES_INVALID_ARGUMENT",
  });
  expect(backend.consumes()).toBe(1);
});

test("finishing an unknown purchase and double-consuming are not owned", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();

  await expect(purchases.finish(storePurchase("never_bought"))).rejects.toMatchObject({
    code: "ERR_PURCHASES_NOT_OWNED",
  });

  const coins = purchases.purchase("coins_100", {});
  backend.finishPurchase("purchased");
  const coinsPurchase = await coins;
  await purchases.consume(coinsPurchase);
  await expect(purchases.consume(coinsPurchase)).rejects.toMatchObject({
    code: "ERR_PURCHASES_NOT_OWNED",
  });
});

test("restore returns the delivered purchases as restored, and empty is empty", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();

  const fresh = new Purchases({ backend: memoryBackend() });
  await fresh.initialize();
  expect(await fresh.restore()).toEqual([]);

  const coins = purchases.purchase("coins_100", {});
  backend.finishPurchase("purchased");
  await coins;
  const unlock = purchases.purchase("pro_unlock", {});
  backend.finishPurchase("purchased");
  await unlock;
  const restored = await purchases.restore();
  expect(restored.map((purchase) => purchase.state)).toEqual(["restored", "restored"]);
  expect(restored.every((purchase) => purchase.receipt !== "")).toBe(true);
  expect(backend.restores()).toBe(1);
});

test("pending lists the unfinished purchases until they are finished", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const call = purchases.purchase("pro_unlock", {});
  backend.finishPurchase("purchased");
  const purchase = await call;
  expect((await purchases.pending()).map((waiting) => waiting.productId)).toEqual(["pro_unlock"]);
  await purchases.finish(purchase);
  expect(await purchases.pending()).toEqual([]);
});

test("entitlements exclude consumed consumables and respect expiry", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();

  const unlock = purchases.purchase("pro_unlock", {});
  backend.finishPurchase("purchased");
  await unlock;
  const coins = purchases.purchase("coins_100", {});
  backend.finishPurchase("purchased");
  const coinsPurchase = await coins;
  await purchases.consume(coinsPurchase);
  const monthly = purchases.purchase("pro_monthly", {});
  backend.finishPurchase("purchased");
  await monthly;

  const entitlements = await purchases.entitlements();
  const ids = entitlements.map((entitlement) => entitlement.productId);
  // The backend emits rights in delivery order, so the consumed coins_100 is exactly absent.
  expect(ids).toEqual(["pro_unlock", "pro_monthly"]);

  const unlockRight = entitlements.find((right) => right.productId === "pro_unlock");
  // An unlock never lapses: no expiry on its right. The fixture stamps the sandbox times the
  // Rust fake uses; the expired-is-inactive half of expiry is the crate's scripted test.
  expect(unlockRight).toEqual({ productId: "pro_unlock", active: true });

  const monthlyRight = entitlements.find((right) => right.productId === "pro_monthly");
  expect(monthlyRight).toEqual({
    productId: "pro_monthly",
    active: true,
    expiresAtMs: 1_735_689_600_000 + 30 * 24 * 60 * 60 * 1000,
  });
});

test("a memory backend configured unavailable answers not available", async () => {
  const backend = memoryBackend({ available: false });
  const purchases = new Purchases({ backend });
  expect(await purchases.isAvailable()).toBe(false);
  await expect(purchases.initialize()).rejects.toMatchObject({
    code: "ERR_PURCHASES_NOT_AVAILABLE",
  });
});

test("the options and the storefront pass through to the backend", async () => {
  const backend = memoryBackend({ storefront: "GB" });
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const call = purchases.purchase("coins_100", { quantity: 2, applicationUsername: "amir" });
  expect(backend.lastProductId()).toBe("coins_100");
  expect(backend.lastOptions()).toEqual({ quantity: 2, applicationUsername: "amir" });
  backend.finishPurchase("purchased");
  await call;
  expect(await purchases.countryCode()).toBe("GB");
});

test("queries answer while a purchase runs, without claiming the gate", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const call = purchases.purchase("coins_100", {});
  const products = await purchases.products(["pro_unlock"]);
  expect(products.length).toBe(1);
  await purchases.pending();
  await purchases.entitlements();
  await purchases.countryCode();
  expect(backend.purchases()).toBe(1);
  backend.finishPurchase("purchased");
  expect((await call).productId).toBe("coins_100");
});
