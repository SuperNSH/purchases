// The demo's screen: the fake store's products, its sheet answers (approved, pending,
// cancelled, declined), the pending queue with finish and consume, restore, and the
// entitlements with the subscription's expiry. Every test opens its own fresh store.
import { open, test } from "nsh/test";

const WINDOW = { width: 700, height: 900 };

test("the fake store initializes and lists its three fixture products", async () => {
  const app = await open("../App.tsx", WINDOW);
  await app.waitFor("id:product-pro_unlock");
  await app.waitFor("id:product-coins_100");
  await app.waitFor("id:product-pro_monthly");
  await app.waitFor("text:initialized · storefront US");
});

test("an approved purchase lands in pending and the rights, and consuming clears it", async () => {
  const app = await open("../App.tsx", WINDOW);
  await app.waitFor("id:product-coins_100");
  await app.press("id:buy-coins_100");
  await app.waitFor("id:sheet");
  await app.press("id:answer-purchased");
  await app.waitFor("text:bought coins_100 (purchased)");
  await app.waitFor("id:pending-fake-coins_100-1");
  await app.waitFor("id:right-coins_100");
  await app.press("id:consume-fake-coins_100-1");
  await app.waitFor("text:consumed fake-coins_100-1");
  await app.waitFor("id:no-pending");
  await app.waitUntil(() => !app.texts().includes("coins_100 · active"));
});

test("finishing acknowledges a non-consumable, the right stays, and restore reports it", async () => {
  const app = await open("../App.tsx", WINDOW);
  await app.waitFor("id:product-pro_unlock");
  await app.press("id:buy-pro_unlock");
  await app.waitFor("id:sheet");
  await app.press("id:answer-purchased");
  await app.waitFor("text:bought pro_unlock (purchased)");
  await app.press("id:finish-fake-pro_unlock-1");
  await app.waitFor("text:finished fake-pro_unlock-1");
  await app.waitFor("id:no-pending");
  await app.waitFor("id:right-pro_unlock");
  await app.press("id:restore");
  await app.waitFor("text:restored pro_unlock");
});

test("a pending approval resolves pending and waits in the queue without a right", async () => {
  const app = await open("../App.tsx", WINDOW);
  await app.waitFor("id:product-pro_monthly");
  await app.press("id:buy-pro_monthly");
  await app.waitFor("id:sheet");
  await app.press("id:answer-pending");
  await app.waitFor("text:bought pro_monthly (pending)");
  await app.waitFor("id:pending-fake-pro_monthly-1");
  await app.waitUntil(() => !app.texts().some((text) => text.startsWith("pro_monthly · active")));
});

test("a subscription's entitlement carries its expiry", async () => {
  const app = await open("../App.tsx", WINDOW);
  await app.waitFor("id:product-pro_monthly");
  await app.press("id:buy-pro_monthly");
  await app.waitFor("id:sheet");
  await app.press("id:answer-purchased");
  await app.waitFor("text:pro_monthly · active · expires 2025-01-31");
});

test("the cancelled, declined, owned and app-side paths surface their codes readably", async () => {
  const app = await open("../App.tsx", WINDOW);
  await app.waitFor("id:product-pro_unlock");

  await app.press("id:buy-pro_unlock");
  await app.waitFor("id:sheet");
  await app.press("id:answer-cancelled");
  await app.waitFor("text:ERR_PURCHASES_CANCELLED");
  await app.waitFor("text:The purchase was cancelled.");

  await app.press("id:buy-pro_unlock");
  await app.waitFor("id:sheet");
  await app.press("id:answer-owned");
  await app.waitFor("text:ERR_PURCHASES_ALREADY_OWNED");
  await app.waitFor("text:The product is already owned.");

  await app.press("id:buy-pro_monthly");
  await app.waitFor("id:sheet");
  await app.press("id:answer-payment");
  await app.waitFor("text:ERR_PURCHASES_PAYMENT_INVALID");
  await app.waitFor("text:The store rejected the payment.");

  await app.press("id:buy-pro_monthly");
  await app.waitFor("id:sheet");
  await app.press("id:app-cancel");
  await app.waitFor("text:ERR_PURCHASES_CANCELLED");
});
