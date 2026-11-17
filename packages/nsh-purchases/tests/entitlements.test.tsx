import { mount } from "nsh";
import { expect, test } from "nsh/test";
import {
  memoryBackend,
  Purchases,
  type Entitlement,
  type Entitlements,
  type Purchase,
  type PurchasesBackend,
  useEntitlements,
} from "../src";

/** Lets the hook's read settle, as `delivered` does for the facade's answers. */
async function delivered(): Promise<void> {
  for (let turn = 0; turn < 4; turn += 1) await Promise.resolve();
}

test("the first read reaches ready with what the backend holds", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const call = purchases.purchase("pro_unlock", {});
  expect(backend.finishPurchase("purchased")).toBe(true);
  await call;

  const hook = useEntitlements({ purchases });
  expect(hook.state()).toEqual({ status: "checking" });
  await delivered();
  expect(hook.state()).toEqual({
    status: "ready",
    entitlements: [{ productId: "pro_unlock", active: true }],
  });
});

test("a failed read reports failed with the error's code", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  backend.setAvailable(false);
  const hook = useEntitlements({ purchases });
  await delivered();
  expect(hook.state()).toMatchObject({
    status: "failed",
    error: { code: "ERR_PURCHASES_NOT_AVAILABLE" },
  });
});

test("refresh reads again, so a later purchase appears", async () => {
  const backend = memoryBackend();
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const hook = useEntitlements({ purchases });
  await delivered();
  expect(hook.state()).toMatchObject({ status: "ready", entitlements: [] });

  const call = purchases.purchase("pro_monthly", {});
  expect(backend.finishPurchase("purchased")).toBe(true);
  await call;
  hook.refresh();
  await delivered();
  expect(hook.state()).toEqual({
    status: "ready",
    entitlements: [
      {
        productId: "pro_monthly",
        active: true,
        expiresAtMs: 1_735_689_600_000 + 30 * 24 * 60 * 60 * 1000,
      },
    ],
  });
});

test("a stale read's answer is ignored once a newer one runs", async () => {
  // One resolver per entitlements read, in call order, so a superseded read can answer late.
  const answers: Array<(entitlements: Entitlement[]) => void> = [];
  const backend: PurchasesBackend = {
    isAvailable: async () => true,
    initialize: async () => {},
    products: async () => [],
    purchase: () => new Promise<Purchase>(() => {}),
    finish: async () => {},
    consume: async () => {},
    restore: async () => [],
    pending: async () => [],
    entitlements: () =>
      new Promise<Entitlement[]>((resolve) => {
        answers.push(resolve);
      }),
    countryCode: async () => "US",
    cancel: () => {},
  };
  const purchases = new Purchases({ backend });
  await purchases.initialize();
  const hook = useEntitlements({ purchases });
  hook.refresh();
  answers[0]([{ productId: "stale", active: true }]);
  await delivered();
  expect(hook.state()).toEqual({ status: "checking" });
  answers[1]([{ productId: "fresh", active: true }]);
  await delivered();
  expect(hook.state()).toEqual({
    status: "ready",
    entitlements: [{ productId: "fresh", active: true }],
  });
});

test("the hook stops reading once its component is gone", async () => {
  const backend = memoryBackend();
  let reads = 0;
  const counting: PurchasesBackend = {
    ...backend,
    // The memory backend counts its calls except this one; count it here.
    entitlements: async () => {
      reads += 1;
      return await backend.entitlements();
    },
  };
  const purchases = new Purchases({ backend: counting });
  await purchases.initialize();
  let handle: Entitlements | undefined;
  const dispose = mount(() => {
    handle = useEntitlements({ purchases });
    return null;
  });
  await delivered();
  expect(handle).toBeDefined();
  expect(reads).toBe(1);
  expect(handle?.state()).toEqual({ status: "ready", entitlements: [] });

  dispose();
  // A read now would fail against the flipped backend and flip the state with it.
  backend.setAvailable(false);
  handle?.refresh();
  await delivered();
  expect(reads).toBe(1);
  expect(handle?.state()).toEqual({ status: "ready", entitlements: [] });
});
