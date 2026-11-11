/** What a product is, as the stores name it. */
export type ProductKind = "consumable" | "nonConsumable" | "subscription";

/** A product the store lists. `price` is the display string, as the store shows it. */
export interface Product {
  id: string;
  title: string;
  description: string;
  price: string;
  currencyCode?: string;
  kind: ProductKind;
}

/** Where a purchase stands. `pending` is an approval still to come, as Android's ask-a-parent. */
export type PurchaseState = "pending" | "purchased" | "restored";

/** One purchase. `receipt` is the store's verification payload, for validation on the app's side. */
export interface Purchase {
  id: string;
  productId: string;
  state: PurchaseState;
  receipt: string;
  purchasedAtMs?: number;
}

/** A right an owned product grants right now. `expiresAtMs` is subscriptions only. */
export interface Entitlement {
  productId: string;
  active: boolean;
  expiresAtMs?: number;
}
