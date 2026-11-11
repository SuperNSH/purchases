# Goals: nsh-purchases

Products, purchases, restores and entitlements for Native Shell apps, in Rust and in TSX, without a backend. Native StoreKit 2 and Play Billing sit behind one facade; the Apple and Android bridges declare host callbacks (`apple::StoreKitHost`, `android::BillingHost`), map the store's raw codes to the stable codes below, and never panic or hang. The store glue itself lives in the NSH hosts, outside this repo; a target whose host has not registered answers `ERR_PURCHASES_NOT_AVAILABLE`.

Platforms in scope: Apple (iOS and macOS) and Android. Windows, Linux and the web have no store either reference implements, so they answer `ERR_PURCHASES_NOT_AVAILABLE` instead of carrying an empty stub.

## Goals

- One purchase API from Rust and from TSX, with the same names and the same behaviour on both stores.
- Every operation resolves exactly once: `Ok`, a coded error, or a cancel. Nothing panics, nothing blocks forever, and a dropped call cancels its store request.
- What the references handle, this library handles:
  - querying products by id, with unknown ids dropped from the answer;
  - buying consumables (with a quantity), non-consumables and subscriptions;
  - purchases that end pending (iOS ask-to-buy, Android pending), cancelled, or failed with the store's own code mapped to a stable one;
  - finishing: acknowledging on Android, consuming a consumable, once per purchase;
  - restoring past purchases, including the empty answer;
  - entitlements: what is still active, honouring expiry and consumed consumables;
  - the storefront's country code;
  - receipts: the store's verification payload returned to the app, which validates it itself.
- Testing without a store. Every store interaction in this repo is a mock or a sandbox fixture; no real store account, product or purchase, on a developer's machine or in CI.

## The error enum

`ErrorKind::code()` returns exactly these strings; the TypeScript `code` field and this table use the same words. `Backend` carries the host's raw status where one was given.

| Kind | Code | When |
|---|---|---|
| `NotAvailable` | `ERR_PURCHASES_NOT_AVAILABLE` | no store on this target, or no host registered |
| `NotInitialized` | `ERR_PURCHASES_NOT_INITIALIZED` | a store call before `initialize()` |
| `InProgress` | `ERR_PURCHASES_IN_PROGRESS` | another purchase is already running |
| `Timeout` | `ERR_PURCHASES_TIMEOUT` | the opt-in timeout passed unanswered |
| `Cancelled` | `ERR_PURCHASES_CANCELLED` | user, system or app cancelled |
| `ProductNotFound` | `ERR_PURCHASES_PRODUCT_NOT_FOUND` | unknown product id, or the store has no such item |
| `AlreadyOwned` | `ERR_PURCHASES_ALREADY_OWNED` | the non-consumable or active subscription is already owned |
| `NotOwned` | `ERR_PURCHASES_NOT_OWNED` | finish/consume of an unknown or already finished purchase |
| `PaymentInvalid` | `ERR_PURCHASES_PAYMENT_INVALID` | the store rejected the payment itself |
| `NotAllowed` | `ERR_PURCHASES_NOT_ALLOWED` | purchase not allowed (parental controls, revoked cloud rights) |
| `Network` | `ERR_PURCHASES_NETWORK` | the store connection failed |
| `InvalidArgument` | `ERR_PURCHASES_INVALID_ARGUMENT` | empty ids, zero quantity, zero timeout, non-consumable consume |
| `Backend` | `ERR_PURCHASES_BACKEND` | the platform or host failed; carries its raw status where given |

## The shared types

Same names in Rust and TypeScript; Rust spells the fields snake_case, TypeScript camelCase.

- `Product` — `id`, `title`, `description`, `price` (display string), `currencyCode` (optional), `kind`: `Consumable` | `NonConsumable` | `Subscription`.
- `Purchase` — `id`, `productId`, `state`: `Pending` | `Purchased` | `Restored`, `receipt` (the store's verification payload, a string), `purchasedAtMs` (optional).
- `Entitlement` — `productId`, `active`, `expiresAtMs` (optional).

## Edge cases

Each row is one case the references hit, the behaviour here, and the test that covers it. The Rust test names are canonical: they live in `crates/nsh-purchases/tests/` with exactly these names, and `docs/parity.md` cites them. TypeScript mirrors the facade and error-code behaviours in `packages/nsh-purchases/tests/`, the same phrases in human wording; the raw-code mapping rows are Rust-only, because the bridges are Rust.

| Case | Behaviour here | Covered by |
|---|---|---|
| A second purchase while one runs | `ERR_PURCHASES_IN_PROGRESS`; the running purchase is untouched | `purchase_twice_in_parallel_is_in_progress` |
| Non-purchase calls during a running purchase | Products, pending, entitlements and restore answer normally; the gate gates purchases only | `concurrent_non_purchase_calls_do_not_claim_the_gate` |
| Any store call before `initialize()` | `ERR_PURCHASES_NOT_INITIALIZED` | `calls_before_initialize_are_not_initialized` |
| `initialize()` called again | Not an error; one store session | `initialize_is_idempotent` |
| An empty product-id list | `ERR_PURCHASES_INVALID_ARGUMENT`, refused before the store is asked | `empty_product_ids_are_an_invalid_argument` |
| A zero quantity | `ERR_PURCHASES_INVALID_ARGUMENT` | `a_zero_quantity_is_an_invalid_argument` |
| A query mixing known and unknown ids | The known ones are answered, the unknown dropped, no error | `unknown_product_ids_are_dropped_from_the_answer` |
| Buying an id the store does not have | `ERR_PURCHASES_PRODUCT_NOT_FOUND` | `purchasing_an_unknown_product_is_product_not_found` |
| Two store answers for one purchase | The first resolves the call; the second is ignored | `a_purchase_resolves_exactly_once` |
| The app cancels the running purchase | Resolves `ERR_PURCHASES_CANCELLED`, and the gate frees for the next call | `cancel_resolves_the_running_purchase_as_cancelled` |
| The user cancels the store sheet | `ERR_PURCHASES_CANCELLED` | `a_cancelled_purchase_resolves_with_the_cancelled_code` |
| StoreKit's cancel and pending raw codes | Cancel resolves as cancelled; pending resolves `Ok` with state `Pending`, not `Backend` | `a_storekit_cancel_and_a_pending_purchase_resolve_as_documented` |
| The opt-in timeout passes unanswered | `ERR_PURCHASES_TIMEOUT`, and the gate recovers | `a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers` |
| A zero timeout | `ERR_PURCHASES_INVALID_ARGUMENT`, refused up front | `a_zero_timeout_is_an_invalid_argument` |
| iOS ask-to-buy, or an Android purchase awaiting funds | `Ok` with state `Pending`: nothing failed, the purchase completes later | `a_pending_purchase_resolves_with_a_pending_state` |
| Purchases never finished | Listed by `pending()` until they are | `pending_returns_unfinished_purchases` |
| Android pending purchases must be acknowledged, or Google reverses them after three days | `finish()` acknowledges on Android; consuming a consumable consumes it | `finishing_acknowledges_and_consuming_consumes` |
| Finishing a purchase | Marks it finished once | `finishing_marks_the_purchase_finished_once` |
| Finishing an unknown or already-finished purchase | `ERR_PURCHASES_NOT_OWNED` | `finishing_an_unknown_or_finished_purchase_is_not_owned` |
| Consuming a non-consumable | `ERR_PURCHASES_INVALID_ARGUMENT` | `consuming_a_non_consumable_is_an_invalid_argument` |
| Buying an owned non-consumable or active subscription again | `ERR_PURCHASES_ALREADY_OWNED` | `an_already_owned_product_is_already_owned` |
| Restore | Returns the store's purchases with states `Purchased` and `Restored`, receipts included; App Review requires this path to exist | `restore_returns_purchased_and_restored_purchases` |
| Restore with nothing to restore | An empty list, not an error: a restore button must not fail review | `restore_with_no_purchases_is_empty` |
| Consumed consumables | Gone from entitlements | `entitlements_exclude_consumed_consumables` |
| An expired subscription | Its entitlement reports not active | `entitlements_respect_expiry` |
| A target with no store, or a host that never registered | `ERR_PURCHASES_NOT_AVAILABLE` | `an_unsupported_target_is_not_available` |
| The caller drops a purchase call | The store request is cancelled; nothing leaks, nothing hangs | `a_dropped_purchase_call_cancels_the_store_request` |
| Every StoreKit raw code | Maps to its documented code in the table above | `every_storekit_raw_code_maps_to_its_documented_code` |
| A StoreKit raw code with no documented mapping | `ERR_PURCHASES_BACKEND`, raw status kept | `unknown_storekit_raw_codes_are_backend` |
| Every BillingClient raw code | Maps to its documented code in the table above | `every_billing_raw_code_maps_to_its_documented_code` |
| A BillingClient raw code with no documented mapping | `ERR_PURCHASES_BACKEND`, raw status kept | `unknown_billing_raw_codes_are_backend` |
| Every stable code | Round-trips through the error type and the TS bridge without change | `every_error_code_round_trips` |
| Sandbox and production use different receipts and keys | The library never branches on environment: the receipt reaches the app exactly as the store gave it, and the app's validator picks the key set | `a_purchase_resolves_exactly_once`; `restore_returns_purchased_and_restored_purchases` |

## Receipt validation without a backend

The trade-off, stated plainly. On-device validation can check a signature: StoreKit 2 receipts are JWS blobs verifiable against Apple's root certificate, and Play signs its payloads with the public key from Play Console. What it cannot do is see what only the store servers know — a refund, a revoked shared purchase, a subscription that lapsed after the payload was signed — and a compromised device can lie about all of it. So the library returns the raw payload and never claims a receipt is valid; an app that needs server-grade guarantees adds its own backend. No backend here, and the docs say what that costs instead of hiding it.

## Acceptance criteria

CI (`.github/workflows/ci.yml`) runs on every push to `master`, and this library is accepted when all of it is green:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- `nsh check` and `nsh test`, in `packages/nsh-purchases`

Also required:

- Every store interaction in those runs is a mock or a sandbox fixture. No job and no developer command touches a real store account, product or purchase.
- Every edge case above has its named test, and each test was seen to fail before it passed.
- `ErrorKind::code()`, the TypeScript `code` field and the table above use the same strings, verified by `every_error_code_round_trips`.
- Nothing is claimed done without a passing run: `docs/parity.md` carries the evidence, one row per capability, each citing the test CI ran.
