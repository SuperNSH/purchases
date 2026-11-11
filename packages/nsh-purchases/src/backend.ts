import type { PurchaseOptions } from "./options";
import type { Entitlement, Product, Purchase } from "./store";

/** What a `Purchases` runs on: the Rust crate, or a memory backend in tests. */
export interface PurchasesBackend {
  isAvailable(): Promise<boolean>;
  initialize(): Promise<void>;
  /** The found products only; unknown ids are silently dropped, as the references do. */
  products(ids: string[]): Promise<Product[]>;
  /** Resolves once, with the purchase the store made, `pending` included. */
  purchase(productId: string, options: PurchaseOptions): Promise<Purchase>;
  /** Apple's finish, or Android's acknowledge. */
  finish(purchase: Purchase): Promise<void>;
  /** Consumables only; the store drops the purchase from the account. */
  consume(purchase: Purchase): Promise<void>;
  restore(): Promise<Purchase[]>;
  /** The unfinished queue, and the purchases still to acknowledge. */
  pending(): Promise<Purchase[]>;
  entitlements(): Promise<Entitlement[]>;
  countryCode(): Promise<string>;
  /** Asks the store to dismiss its sheets; the running call still resolves exactly once. */
  cancel(): void;
}
