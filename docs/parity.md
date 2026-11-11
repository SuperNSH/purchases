# Parity with `in_app_purchase` and `react-native-iap`

Each row names one public API of Flutter's `in_app_purchase` (the names come from `packages/in_app_purchase` in `.refs/flutter-packages`, read on 2026-11-09) or one export of `react-native-iap` (the OpenIAP monorepo's `libraries/react-native-iap` in `.refs/openiap`, the canonical source; see `docs/decisions.md`), the status here, and the test that covers it. The archived `hyochan` mirror kept a few older names (`validateReceipt`, `presentCodeRedemptionSheetIOS`, `requestPromotedProductIOS`, `getStorefrontIOS`); where it differs the row says so. The older facade names `getProducts`, `getSubscriptions`, `requestSubscription` and `getPurchaseHistories` exist in neither clone's index: upstream folded them into `fetchProducts`, `requestPurchase` and `getAvailablePurchases`. `acknowledgePurchase` and `consumePurchase` survive only as deprecated aliases of `acknowledgePurchaseAndroid` and `consumePurchaseAndroid` in the archived mirror; the OpenIAP index drops them. The rows below follow the source.

Status words:

- **Done**: works on every platform the row names, as the references do.
- **Partial**: works, with a limit named in the row.
- **Accepted**: the option is accepted and passed on, but the store does nothing with it.
- **Platform only**: the effect exists on a device and no CI runner can show it; the test checks what the library passes on.

Rust test names are in `crates/nsh-purchases/tests/` (`purchases.rs`, `apple.rs`, `android.rs`). TypeScript tests are in `packages/nsh-purchases/tests/` (`purchases.test.tsx`, `errors.test.tsx`).

Three design differences shape the whole table. Ours has no purchase stream: `purchase()` resolves exactly once with the `Purchase` or a coded error, and unfinished purchases surface through `pending()`. `restore()` delivers through its return value, not a stream or a later listener event. Receipt validation is app-side: the receipt rides the `Purchase` and this library never verifies it. Web, Windows and Linux get the unsupported backend, so every store call there resolves `ERR_PURCHASES_NOT_AVAILABLE` (`an_unsupported_target_is_not_available`); neither reference ships a store on those targets either. Apple covers iOS and macOS through one StoreKit host; Android through one Billing host (`docs/decisions.md`).

## Methods of `InAppPurchase`

| `in_app_purchase` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `InAppPurchase.instance` | `Purchases::new()`; `with_backend` swaps the backend for tests | a `Purchases` over `rustBackend()` or `memoryBackend()` | Done. The default backend picks the Apple or Android host that registered; with none, the unsupported backend answers `ERR_PURCHASES_NOT_AVAILABLE` | `an_unsupported_target_is_not_available`; "unsupported target via a memory backend configured as unavailable" in `purchases.test.tsx` |
| `purchaseStream` | not in the API; `purchase()` resolves once, `pending()` lists the unfinished | not in the API | Partial. By design: one call, one answer, no broadcast to subscribe to before launch; a late or duplicate answer is dropped by the generation guard | `a_purchase_resolves_exactly_once`; `pending_returns_unfinished_purchases`; "the gate, cancel and late answers" in `purchases.test.tsx` |
| `isAvailable()` | `Purchases::is_available` | `isAvailable()` | Done. The one call allowed before `initialize()`; no host or an unavailable backend answers `false` | `an_unsupported_target_is_not_available` |
| `queryProductDetails(Set<String>)` | `products(&[String]) -> Result<Vec<Product>>` | `products(ids)` | Done. Found products only; unknown ids are dropped silently, as both references do, and there is no `notFoundIDs` list to check. An empty id list resolves `ERR_PURCHASES_INVALID_ARGUMENT` before the store is asked | `empty_product_ids_are_an_invalid_argument`; `unknown_product_ids_are_dropped_from_the_answer` |
| `buyNonConsumable({purchaseParam})` | `purchase(product_id, options)` | `purchase(productId, options)` | Done. One entry point; the product's own kind picks the store flow, and a non-consumable or active subscription already owned resolves `ERR_PURCHASES_ALREADY_OWNED` | `an_already_owned_product_is_already_owned`; `a_purchase_resolves_exactly_once` |
| `buyConsumable({purchaseParam, autoConsume})` | `purchase(product_id, options)`, then `consume(&Purchase)` | `purchase`, then `consume(purchase)` | Partial. `autoConsume` is not offered: consumption is an explicit call, so the app delivers before it consumes | `finishing_acknowledges_and_consuming_consumes`; `consuming_a_non_consumable_is_an_invalid_argument` |
| `completePurchase(PurchaseDetails)` | `finish(&Purchase)` | `finish(purchase)` | Done. Apple finish, Android acknowledge. Finishing an unknown or already finished purchase resolves `ERR_PURCHASES_NOT_OWNED`, where the reference throws | `finishing_marks_the_purchase_finished_once`; `finishing_an_unknown_or_finished_purchase_is_not_owned` |
| `restorePurchases({applicationUserName})` | `restore() -> Result<Vec<Purchase>>` | `restore()` | Done. The restored list is the return value, not a stream. `applicationUserName` is not taken on restore; it rides the purchase options where a store uses it (row below) | `restore_returns_purchased_and_restored_purchases`; `restore_with_no_purchases_is_empty` |
| `countryCode()` | `country_code()` | `countryCode()` | Platform only. StoreKit's storefront or Play's billing config, read through the host; CI can only show the pass-through | none beyond the pass-through |
| `getPlatformAddition<T>()` | not in the API | not in the API | Partial. No per-platform escape hatch: Android's consume is first-class on the facade, the rest of the additions is out of scope | `finishing_acknowledges_and_consuming_consumes` (the consume addition) |
| not in `in_app_purchase` | `cancel()` | `cancel()` | Done. Resolves the running purchase as `ERR_PURCHASES_CANCELLED` at once; the late store answer is dropped. Dropping the in-flight future cancels the store request the same way | `cancel_resolves_the_running_purchase_as_cancelled`; `a_dropped_purchase_call_cancels_the_store_request`; "cancel resolves the running purchase" in `purchases.test.tsx` |
| not in `in_app_purchase` | `PurchaseOptions::timeout` | `timeoutMs` | Done. Opt-in. The deadline passes, the call resolves `ERR_PURCHASES_TIMEOUT`, the gate frees and the late answer is dropped. A zero timeout is `ERR_PURCHASES_INVALID_ARGUMENT` before the gate is claimed | `a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers`; `a_zero_timeout_is_an_invalid_argument`; "the timeoutMs watchdog" in `purchases.test.tsx` |

## The platform-interface types

| type | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `ProductDetails` | `Product { id, title, description, price, currency_code, kind }` | `Product` (camelCase) | Partial. `price` is the store's display string; `rawPrice` and `currencySymbol` are not carried. `kind` is added because both references make the app know the kind out of band | `unknown_product_ids_are_dropped_from_the_answer` (the fixture products round-trip) |
| `PurchaseDetails` | `Purchase { id, product_id, state, receipt, purchased_at_ms }` | `Purchase` (camelCase) | Partial. `pendingCompletePurchase` is a query, not a field: `pending()` lists the unfinished purchases instead | `pending_returns_unfinished_purchases` |
| `PurchaseStatus.pending` / `.purchased` / `.restored` | `PurchaseState::Pending` / `Purchased` / `Restored` | `"pending"` / `"purchased"` / `"restored"` | Done | `a_pending_purchase_resolves_with_a_pending_state`; `restore_returns_purchased_and_restored_purchases` |
| `PurchaseStatus.error` / `.canceled` | not states here | not states here | Partial. A failure resolves the `purchase()` call with its code — `ERR_PURCHASES_CANCELLED` for a user or system cancel — so only outcomes that leave a purchase alive are states | `a_cancelled_purchase_resolves_with_the_cancelled_code` |
| `PurchaseParam.productDetails` | the `product_id: &str` argument | the `productId` argument | Partial. The store is the authority on the product; the id is passed, not a client-held details object | `purchasing_an_unknown_product_is_product_not_found` |
| `PurchaseParam.applicationUserName` | `PurchaseOptions::application_username` | `applicationUsername` | Accepted. Forwarded to the Apple host's purchase request; the Android bridge and both fakes accept it and do nothing with it (Play's obfuscated account ids are not carried) | none beyond the field's pass-through |
| not in `PurchaseParam` | `PurchaseOptions::quantity` | `quantity` | Accepted. Sent to the Apple host; Play Billing has no per-purchase quantity, so the Android bridge ignores it. Zero is `ERR_PURCHASES_INVALID_ARGUMENT` everywhere | `a_zero_quantity_is_an_invalid_argument` |
| `ProductDetailsResponse` | `Result<Vec<Product>>` | `Promise<Product[]>` | Partial. Two lists and an optional error become one answer: found products on `Ok`, unknown ids dropped, a store failure resolves the call with its code | `unknown_product_ids_are_dropped_from_the_answer` |
| `PurchaseVerificationData` | `Purchase.receipt` (one string) | `Purchase.receipt` | Partial. One payload string in the store's own format (StoreKit 2 JWS on Apple); validation is app-side and this library never verifies | the receipt rides every purchase in `a_purchase_resolves_exactly_once` |
| `IAPError` / `InAppPurchaseException` | `PurchasesError { kind, status }` | `PurchasesError { code }` | Done. Every store failure maps to a stable `ERR_PURCHASES_*` code; the raw store status rides the error where one is given | `every_error_code_round_trips`; the `asPurchasesError` cases in `errors.test.tsx` |

## Apple: StoreKit 2, through the host seam

The bridge compiles on every runner; the real StoreKit 2 glue lives in the NSH host, which registers `StoreKitHost` at startup (`docs/decisions.md`). The mapping tests run in CI against a mock host.

| StoreKit 2 wrapper | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `SK2Product.products(ids)` | `products` | `products` | Done | `unknown_product_ids_are_dropped_from_the_answer` |
| `SK2Product.purchase(options)` | `purchase` | `purchase` | Done. `userCancelled` maps to `ERR_PURCHASES_CANCELLED`; a pending outcome resolves `Ok` with the pending state, not an error | `a_storekit_cancel_and_a_pending_purchase_resolve_as_documented` |
| `SK2ProductPurchaseOptions.quantity` / offers | `PurchaseOptions { quantity, application_username }` | `quantity`, `applicationUsername` | Partial. Quantity and the account name are forwarded; discount and win-back offers are out of scope for the first release | `a_zero_quantity_is_an_invalid_argument` |
| `SK2Product.isIntroductoryOfferEligible` / `isWinBackOfferEligible` | not in the API | not in the API | Partial. Offer eligibility is out of scope | none |
| `SK2Transaction.finish(id)` | `finish` | `finish` | Done | `finishing_marks_the_purchase_finished_once` |
| `SK2Transaction.transactions()` / `unfinishedTransactions()` | `pending()` | `pending()` | Done. The unfinished queue is the query; the start-of-session redelivery the stream gave is `pending()` after `initialize()` | `pending_returns_unfinished_purchases` |
| `SK2Transaction.currentEntitlements()` | `entitlements()` | `entitlements()` | Done. An expired subscription reports `active: false`; consumed consumables are gone | `entitlements_respect_expiry`; `entitlements_exclude_consumed_consumables` |
| `SK2Transaction.restorePurchases()` | `restore()` | `restore()` | Done. The restored list is the return value | `restore_returns_purchased_and_restored_purchases` |
| `AppStore.canMakePayments()` | not in the API; the purchase resolves `ERR_PURCHASES_NOT_ALLOWED` | same | Partial. A restriction (parental controls, revoked cloud rights) is the purchase's coded failure, not a separate query | `every_storekit_raw_code_maps_to_its_documented_code` (`paymentNotAllowed`) |
| `AppStore.sync()` | folded into `restore()` | same | Partial. No separate sync call; restore asks the host, which syncs | `restore_returns_purchased_and_restored_purchases` |
| `Storefront.countryCode()` | `country_code()` | `countryCode()` | Platform only | none beyond the pass-through |
| `AppStore.presentOfferCodeRedeemSheet()` / `showManageSubscriptions()` | not in the API | not in the API | Partial. Store UI sheets are out of scope; the app opens the store itself | none |
| StoreKit 1 wrappers (`SKPaymentQueue…`, `SKReceiptManager.refreshReceipt`) | not wrapped | not wrapped | Partial. StoreKit 2 only, as the reference defaults on iOS 15+; a receipt refresh is what `restore()` asks the host for | `restore_returns_purchased_and_restored_purchases` |
| `SKError` codes (`clientInvalid`, `paymentCancelled`, `paymentInvalid`, `paymentNotAllowed`, `cloudService…`) | `ErrorKind` per code: `InvalidArgument`, `Cancelled`, `PaymentInvalid`, `NotAllowed`, `Network`, `NotAllowed`, `NotAllowed` | the matching `ERR_PURCHASES_*` strings | Done. Every documented raw code has one mapped code; an unknown raw code is `ERR_PURCHASES_BACKEND` carrying the status, never a panic | `every_storekit_raw_code_maps_to_its_documented_code`; `unknown_storekit_raw_codes_are_backend` |

## Android: BillingClient, through the host seam

Same seam: `BillingHost` registers at startup, the mapping tests run in CI against a mock host.

| BillingClient wrapper | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `startConnection` / `endConnection` | `initialize()` | `initialize()` | Done. Idempotent `Ok`; a store call before it resolves `ERR_PURCHASES_NOT_INITIALIZED`. There is no end call — the host owns the connection | `initialize_is_idempotent`; `calls_before_initialize_are_not_initialized` |
| `isReady()` | `is_available` | `isAvailable` | Done | `an_unsupported_target_is_not_available` |
| `queryProductDetails(skus)` | `products` | `products` | Done | `unknown_product_ids_are_dropped_from_the_answer` |
| `launchBillingFlow` | `purchase` | `purchase` | Done. One purchase at a time: a second purchase while one runs resolves `ERR_PURCHASES_IN_PROGRESS` at once; non-purchase calls never claim the gate | `purchase_twice_in_parallel_is_in_progress`; `concurrent_non_purchase_calls_do_not_claim_the_gate` |
| `queryPurchases(ProductType)` | `pending()` for the unfinished, `entitlements()` for the active rights | same | Done. Split by the question asked | `pending_returns_unfinished_purchases`; `entitlements_exclude_consumed_consumables` |
| `acknowledgePurchase(token)` | `finish` | `finish` | Done. Finish acknowledges on Android; both counters are on the mock host | `finishing_acknowledges_and_consuming_consumes` |
| `consumeAsync(token)` | `consume` | `consume` | Done. Consuming a non-consumable resolves `ERR_PURCHASES_INVALID_ARGUMENT` | `finishing_acknowledges_and_consuming_consumes`; `consuming_a_non_consumable_is_an_invalid_argument` |
| `PENDING` purchases (the 3-day acknowledge window) | `Pending` state, surfaced by `pending()` | same | Done. Pending resolves `Ok` — nothing failed — and the app finishes it after approval | `a_pending_purchase_resolves_with_a_pending_state`; `pending_returns_unfinished_purchases` |
| `getBillingConfig()` (the country code) | `country_code()` | `countryCode()` | Platform only | none beyond the pass-through |
| Alternative billing (`isAlternativeBillingOnlyAvailable`, `showAlternativeBillingOnlyInformationDialog`, `createAlternativeBillingOnlyReportingDetails`) and the user-choice / developer-provided billing listeners | not in the API | not in the API | Partial. Alternative billing is out of scope for the first release | none |
| `showInAppMessages()` | not in the API | not in the API | Partial. Out of scope | none |
| `BillingResponse` codes (`ITEM_ALREADY_OWNED`, `ITEM_NOT_OWNED`, `ITEM_UNAVAILABLE`, `DEVELOPER_ERROR`, `BILLING_UNAVAILABLE`, `NETWORK_ERROR`, `SERVICE_*`, …) | the matching `ErrorKind`: `AlreadyOwned`, `NotOwned`, `ProductNotFound`, `InvalidArgument`, `NotAvailable`, `Network`, `Backend` | the matching `ERR_PURCHASES_*` strings | Done. Every documented raw code has one mapped code; an unknown raw code is `ERR_PURCHASES_BACKEND` carrying the status | `every_billing_raw_code_maps_to_its_documented_code`; `unknown_billing_raw_codes_are_backend` |

## `react-native-iap`

| `react-native-iap` | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `initConnection(config?)` | `initialize()` | `initialize()` | Done. No config is taken; the host carries none | `initialize_is_idempotent`; `calls_before_initialize_are_not_initialized`; "initialize and arg checks" in `purchases.test.tsx` |
| `endConnection()` | not in the API | not in the API | Partial. Nothing to close: the bridge holds no connection resource to release | none |
| `fetchProducts({skus, type})` | `products(ids)` | `products(ids)` | Done. One call for both kinds; the kind rides each product, so no `type` split. Empty skus resolve `ERR_PURCHASES_INVALID_ARGUMENT` | `empty_product_ids_are_an_invalid_argument`; `unknown_product_ids_are_dropped_from_the_answer` |
| `requestPurchase(request)` | `purchase(product_id, options)` | `purchase(productId, options)` | Partial. By design: ours resolves exactly once with the purchase or a coded error, where the reference dispatches and delivers through listeners. `andFinishIOS`-style helpers are not needed for the same reason | `a_purchase_resolves_exactly_once`; `purchase_twice_in_parallel_is_in_progress` |
| `purchaseUpdatedListener(listener, options?)` | not in the API; the resolved `purchase()` and `pending()` cover it | not in the API | Partial. By design, no broadcast stream; the iOS `dedupeTransactionIOS` option has no duplicate stream to deduplicate | `a_purchase_resolves_exactly_once`; `pending_returns_unfinished_purchases` |
| `purchaseErrorListener(listener)` | not in the API; failures resolve the call with their code | not in the API | Partial. By design | `a_cancelled_purchase_resolves_with_the_cancelled_code`; `a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers` |
| `finishTransaction({purchase, isConsumable})` | `finish(purchase)` and `consume(purchase)` | `finish` and `consume` | Done. Two calls; the product's kind decides which, and a non-consumable consume is `ERR_PURCHASES_INVALID_ARGUMENT` | `finishing_marks_the_purchase_finished_once`; `consuming_a_non_consumable_is_an_invalid_argument` |
| `acknowledgePurchaseAndroid(token)` | `finish` | `finish` | Done | `finishing_acknowledges_and_consuming_consumes` |
| `consumePurchaseAndroid(token)` | `consume` | `consume` | Done | `finishing_acknowledges_and_consuming_consumes` |
| `getAvailablePurchases(options)` | `pending()` for the unfinished, `restore()` for the store's own list | same | Partial. Split by the question; the iOS-only options (`alsoPublishToEventListenerIOS`, `onlyIncludeActiveItemsIOS`) and `includeSuspendedAndroid` have no counterpart | `pending_returns_unfinished_purchases`; `restore_returns_purchased_and_restored_purchases` |
| `restorePurchases()` | `restore()` | `restore()` | Done. The list is the return value; the reference returns nothing and relies on a follow-up `getAvailablePurchases` | `restore_returns_purchased_and_restored_purchases`; `restore_with_no_purchases_is_empty` |
| `getStorefront()` | `country_code()` | `countryCode()` | Platform only. The archived mirror keeps a separate iOS-only `getStorefrontIOS` export; the OpenIAP index has only this one | none beyond the pass-through |
| `getActiveSubscriptions(ids?)` / `hasActiveSubscriptions(ids?)` | `entitlements()` | `entitlements()` | Done. Ours answers for every product with `active` and `expires_at_ms`, expiry honoured; consumables are excluded once consumed | `entitlements_respect_expiry`; `entitlements_exclude_consumed_consumables` |
| `useIAP()` | `use_entitlements(&Purchases)` | `useEntitlements(options)` | Partial. Ours tracks entitlements only (checking / ready / failed, refreshed on lifecycle active); it does not mirror the hook's purchase events or product lists | `entitlements_respect_expiry`; "entitlements incl. expiry" in `purchases.test.tsx` |
| `verifyPurchase` (`validateReceipt` in the archived mirror) | not in the API | not in the API | Partial. By design: the receipt is returned on the purchase and validation is app-side; there is no verification backend in this repo's premise | the receipt rides `a_purchase_resolves_exactly_once` |
| `verifyPurchaseWithProvider(props)` | not in the API | not in the API | Partial. Hosted verification is out of scope | none |
| `syncIOS()` | folded into `restore()` | same | Partial | `restore_returns_purchased_and_restored_purchases` |
| `getPendingTransactionsIOS()` | `pending()` | `pending()` | Done | `pending_returns_unfinished_purchases` |
| `currentEntitlementIOS(productId)` | `entitlements()` | `entitlements()` | Done. Store-wide, not per product | `entitlements_respect_expiry` |
| `subscriptionStatusIOS(productId)` / `latestTransactionIOS(productId)` | `entitlements()` / the `Purchase` the call resolved | same | Partial. Status is answered by expiry; the latest transaction is the one the resolved call returned | `entitlements_respect_expiry` |
| `getReceiptDataIOS()` / `requestReceiptRefreshIOS()` | `Purchase.receipt`; refresh is `restore()` | same | Partial. The receipt rides each purchase; restore refreshes through the host | `a_purchase_resolves_exactly_once`; `restore_returns_purchased_and_restored_purchases` |
| `isTransactionVerifiedIOS(id)` / `getTransactionJwsIOS(id)` | not in the API; the JWS payload is the `receipt` string | same | Partial. Verification is app-side | the receipt rides `a_purchase_resolves_exactly_once` |
| `getAppTransactionIOS()` / `verifyAppOwnership()` | not in the API | not in the API | Partial. App-level ownership checks are out of scope | none |
| `getAllTransactionsIOS()` | not in the API | not in the API | Partial. History beyond pending and restore is out of scope | none |
| `isEligibleForIntroOfferIOS(productId)` | not in the API | not in the API | Partial. Offer eligibility is out of scope (same answer as the StoreKit 2 section) | none |
| `getPromotedProductIOS()` / `promotedProductListenerIOS(cb)` / `requestPromotedProductIOS` (archived) | not in the API | not in the API | Partial. Promoted purchases are out of scope; a store-initiated one lands in the unfinished queue, so `pending()` surfaces it | `pending_returns_unfinished_purchases` |
| `showManageSubscriptionsIOS()` / `openRedeemOfferCode` (`presentCodeRedemptionSheetIOS` in the archived mirror) / `clearTransactionIOS()` / `beginRefundRequestIOS()` | not in the API | not in the API | Partial. Store UI, redeem sheets and refunds are out of scope; the app opens the store itself | none |
| `deepLinkToSubscriptions(options)` / `deepLinkToSubscriptionsIOS()` | not in the API | not in the API | Partial. The app opens the URL itself | none |
| External purchase and billing programs (`enableBillingProgramAndroid`, `isBillingProgramAvailableAndroid`, `getBillingChoiceInfoAndroid`, `createBillingProgramReportingDetailsAndroid`, `showBillingProgramInformationDialogAndroid`, `launchExternalLinkAndroid`, `showInAppMessagesAndroid`, `canPresentExternalPurchaseNoticeIOS`, `presentExternalPurchaseNoticeSheetIOS`, `presentExternalPurchaseLinkIOS`, `isEligibleForExternalPurchaseCustomLinkIOS`, `getExternalPurchaseCustomLinkTokenIOS`, `showExternalPurchaseCustomLinkNoticeIOS`, `subscriptionBillingIssueListener`) | not in the API | not in the API | Partial. EU external-purchase and alternative-billing flows are out of scope for the first release | none |
| Purchase-flow kit (`createPurchaseFlow`, `confirmWithIapkit`, `confirmOnDeviceIOS`, `describePurchaseFlowStatus`, `toPurchaseConfirmationRequest`, `kitApi`, `KitApiError`, `resolveLocalizedPayload`, `getUserFriendlyErrorMessage`) | not in the API | not in the API | Partial. The hosted kit flows are out of scope by the no-backend premise | none |
| Platform probes and bridge helpers (`isNitroReady`, `isTVOS`, `isMacOS`, `isStandardIOS`, `isVegaOS`, `getVegaIapModule`, `resetListenerState`, the type-bridge conversion and validation helpers) | not in the API | not in the API | Partial. The target question is answered by `is_available` and `ERR_PURCHASES_NOT_AVAILABLE`; the rest is another library's plumbing | `an_unsupported_target_is_not_available` |

## `ErrorCode` (react-native-iap) here

The reference reports these on `PurchaseError`. The mapping is the bridge's documented table (`crates/nsh-purchases/src/apple.rs` and `src/android.rs`); the Rust column names the `ErrorKind`.

| `ErrorCode` member | Rust | TypeScript | Status | Covered by |
|---|---|---|---|---|
| `user-cancelled` | `Cancelled` | `ERR_PURCHASES_CANCELLED` | Done | `a_cancelled_purchase_resolves_with_the_cancelled_code` |
| `already-owned` / `duplicate-purchase` | `AlreadyOwned` | `ERR_PURCHASES_ALREADY_OWNED` | Done | `an_already_owned_product_is_already_owned` |
| `item-not-owned` / `not-ended` | `NotOwned` | `ERR_PURCHASES_NOT_OWNED` | Done | `finishing_an_unknown_or_finished_purchase_is_not_owned` |
| `item-unavailable` / `sku-not-found` | `ProductNotFound` | `ERR_PURCHASES_PRODUCT_NOT_FOUND` | Done | `purchasing_an_unknown_product_is_product_not_found` |
| `empty-sku-list` / `developer-error` / `user-error` | `InvalidArgument` | `ERR_PURCHASES_INVALID_ARGUMENT` | Done. Checked before the gate is claimed | `empty_product_ids_are_an_invalid_argument`; `a_zero_quantity_is_an_invalid_argument` |
| `pending` / `deferred-payment` | not an error: `PurchaseState::Pending` resolves `Ok` | `"pending"` | Done. Nothing failed, so the call succeeds and `pending()` tracks it | `a_pending_purchase_resolves_with_a_pending_state` |
| `not-prepared` / `init-connection` / `connection-closed` | `NotInitialized` | `ERR_PURCHASES_NOT_INITIALIZED` | Done | `calls_before_initialize_are_not_initialized` |
| `already-prepared` | never an error: `initialize()` is idempotent `Ok` | same | Done | `initialize_is_idempotent` |
| `iap-not-available` / `billing-unavailable` / `feature-not-supported` / `activity-unavailable` | `NotAvailable` | `ERR_PURCHASES_NOT_AVAILABLE` | Done | `an_unsupported_target_is_not_available` |
| `network-error` / `service-disconnected` | `Network` | `ERR_PURCHASES_NETWORK` | Done | `every_billing_raw_code_maps_to_its_documented_code` |
| `service-timeout` | `Timeout` when the opt-in deadline passed, `Network` when the store connection died | `ERR_PURCHASES_TIMEOUT` / `ERR_PURCHASES_NETWORK` | Partial. The timeout is ours: it fires only when `timeout`/`timeoutMs` was set | `a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers` |
| `payment-declined` (sub-response) / `purchase-error` | `PaymentInvalid` | `ERR_PURCHASES_PAYMENT_INVALID` | Done. The store rejected the payment itself | `every_billing_raw_code_maps_to_its_documented_code` |
| `purchase-verification-*` / `transaction-validation-failed` | not errors here | not errors here | Partial. Validation is app-side; a failed check is the app's own logic, not this library's | none |
| the rest (`unknown`, `remote-error`, `service-error`, `interrupted`, `sync-error`, `query-product`, `sku-offer-mismatch`, `billing-response-json-parse-error`, …) | `Backend`, with the raw status where one is given | `ERR_PURCHASES_BACKEND` | Done. The documented catch-all, never a panic | `unknown_billing_raw_codes_are_backend`; `unknown_storekit_raw_codes_are_backend`; "an unknown prefix or unknown code becomes ERR_PURCHASES_BACKEND" in `errors.test.tsx` |

The library adds one code neither reference has: `ERR_PURCHASES_IN_PROGRESS`, because ours allows exactly one purchase at a time and refuses the second at once instead of stacking another store flow (`purchase_twice_in_parallel_is_in_progress`).
