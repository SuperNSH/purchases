import { NativePurchases, type NativePurchaseOptions } from "nsh-purchases/native";
import type { PurchasesBackend } from "./backend";
import { asPurchasesError } from "./errors";
import type { PurchaseOptions } from "./options";
import type { Entitlement, Product, Purchase } from "./store";

/**
 * The Rust crate's store (`nsh-purchases/native`). The app or package must list the crate under
 * `"rust"` in `nsh.json`; importing this module without it fails the bundle.
 */
export function rustBackend(): PurchasesBackend {
  return new RustBackend();
}

/** Only the options the crate reads; unset fields keep the crate's defaults. */
function nativeOptions(options: PurchaseOptions): NativePurchaseOptions {
  const result: NativePurchaseOptions = {};
  if (options.quantity !== undefined) result.quantity = options.quantity;
  if (options.applicationUsername !== undefined) result.applicationUsername = options.applicationUsername;
  if (options.timeoutMs !== undefined) result.timeoutMs = options.timeoutMs;
  return result;
}

class RustBackend implements PurchasesBackend {
  private readonly native = new NativePurchases();

  isAvailable(): Promise<boolean> {
    return this.settle(() => this.native.isAvailable());
  }

  initialize(): Promise<void> {
    return this.settle(() => this.native.initialize());
  }

  products(ids: string[]): Promise<Product[]> {
    return this.settle(async () => (await this.native.products(ids)) as Product[]);
  }

  purchase(productId: string, options: PurchaseOptions): Promise<Purchase> {
    return this.settle(async () => (await this.native.purchase(productId, nativeOptions(options))) as Purchase);
  }

  finish(purchase: Purchase): Promise<void> {
    return this.settle(() => this.native.finish(purchase));
  }

  consume(purchase: Purchase): Promise<void> {
    return this.settle(() => this.native.consume(purchase));
  }

  restore(): Promise<Purchase[]> {
    return this.settle(async () => (await this.native.restore()) as Purchase[]);
  }

  pending(): Promise<Purchase[]> {
    return this.settle(async () => (await this.native.pending()) as Purchase[]);
  }

  entitlements(): Promise<Entitlement[]> {
    return this.settle(async () => (await this.native.entitlements()) as Entitlement[]);
  }

  countryCode(): Promise<string> {
    return this.settle(() => this.native.countryCode());
  }

  cancel(): void {
    // Fire-and-forget: the facade resolves the running purchase itself, and the store's answer still frees the crate's gate.
    void this.native.cancel().catch(() => undefined);
  }

  private async settle<T>(operation: () => Promise<T>): Promise<T> {
    try {
      return await operation();
    } catch (error) {
      throw asPurchasesError(error);
    }
  }
}
