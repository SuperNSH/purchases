import { cleanup, effect, lifecycle, signal } from "nsh";
import { asPurchasesError, type PurchasesError } from "./errors";
import { Purchases } from "./purchases";
import type { Entitlement } from "./store";

/** Where the entitlements read stands. */
export type EntitlementsState =
  | { status: "checking" }
  | { status: "ready"; entitlements: Entitlement[] }
  | { status: "failed"; error: PurchasesError };

/** A reactive read of the entitlements. */
export interface Entitlements {
  state(): EntitlementsState;
  refresh(): void;
}

export interface UseEntitlementsOptions {
  /** The store to ask. Defaults to the Rust crate's. */
  purchases?: Purchases;
}

// NSH 0.4.0's web runtime ships no `lifecycle` accessor: the import stays `undefined` there.
// Treat that as active, so the first read still runs; the foreground re-check comes back with an NSH release that has it.
function lifecycleState(): "active" | "inactive" | "background" {
  return typeof lifecycle === "function" ? lifecycle() : "active";
}

/**
 * Reads what the user owns, as a reactive value. A binding on `state()` runs again only when the
 * answer changes. The read ends when the component goes.
 */
export function useEntitlements(options: UseEntitlementsOptions = {}): Entitlements {
  const purchases = options.purchases ?? new Purchases();
  const state = signal<EntitlementsState>({ status: "checking" });
  let shown = "checking";
  let latest = 0;
  let gone = false;

  const show = (next: EntitlementsState) => {
    const key = describeKey(next);
    if (key === shown) return;
    shown = key;
    state.set(next);
  };

  const check = () => {
    // The handle can outlive its component, and a read then has no owner to land in.
    if (gone) return;
    const attempt = ++latest;
    purchases.entitlements().then(
      (entitlements) => {
        if (attempt === latest) show({ status: "ready", entitlements });
      },
      (error: unknown) => {
        if (attempt === latest) show({ status: "failed", error: asPurchasesError(error) });
      },
    );
  };

  effect(() => {
    if (lifecycleState() === "active") check();
  });
  cleanup(() => {
    gone = true;
    latest += 1;
  });
  return { state: () => state(), refresh: check };
}

function describeKey(state: EntitlementsState): string {
  if (state.status === "ready") return `ready ${JSON.stringify(state.entitlements)}`;
  if (state.status === "failed") return `failed ${state.error.code}`;
  return "checking";
}
