# Audit: docs versus code

Every claim in `docs/goals.md`, `docs/parity.md`, `docs/decisions.md`, `docs/brief.md`, `README.md`, `site/index.html` and `.zcode/` checked against the code and tests, by reading them. Nothing below is taken from a status line, a checkbox or a commit message. Test line numbers are from this checkout. CI evidence: `gh run list --repo SuperNSH/purchases --limit 5` shows run **43967641194** (CI) and **43967641287** (Pages), both `completed success` on 2026-11-16.

Statuses: **Done** (code + a test proves it), **Untested** (code, no test), **Partial**, **Missing**, **Blocked** (needs a change outside this repo), **Stale** (doc wrong, code right).

Key files: Rust src `crates/nsh-purchases/src/{lib,error,model,options,backend,fake,purchases,native,entitlements,apple,android}.rs` + `platform/{mod,unsupported}.rs`; Rust tests `crates/nsh-purchases/tests/{purchases,apple,android}.rs` (26 + 5 + 4 integration tests, plus 4 unit tests in `src/native.rs`); TS `packages/nsh-purchases/src/*.ts`, tests `packages/nsh-purchases/tests/{purchases.errors}.test.tsx` (19 + 5 tests).

Note on TS citations: `goals.md:52` says TS mirrors use "the same phrases in human wording", and `parity.md` cites paraphrases. Each paraphrase was matched to a real test; where the wording differs from the actual test name the row says so.

## docs/goals.md

### Goals (goals.md:9-20)

| # | Claim (quoted, with doc and line) | Where it should live in code | Evidence it works (test name or command) | Status |
|---|---|---|---|---|
| G1 | "One purchase API from Rust and from TSX, with the same names and the same behaviour" (goals.md:9) | `Purchases` in `src/purchases.rs` and `packages/.../src/purchases.ts` | Methods match name-for-name (is_available/isAvailable, initialize, products, purchase, finish, consume, restore, pending, entitlements, country_code/countryCode, cancel); both gates tested — `purchase_twice_in_parallel_is_in_progress` (tests/purchases.rs:100) and "one purchase runs at a time..." (purchases.test.tsx:42) | Done |
| G2 | "Every operation resolves exactly once ... a dropped call cancels its store request" (goals.md:10) | gate + `Owner::drop` in `src/purchases.rs:100-173,255-273`; TS settle/watchdog in `src/purchases.ts:67-108` | `a_purchase_resolves_exactly_once` (tests/purchases.rs:295); `a_dropped_purchase_call_cancels_the_store_request` (tests/purchases.rs:573); `cancel_resolves_the_running_purchase_as_cancelled` (tests/purchases.rs:123) | Done |
| G3 | "What the references handle, this library handles" — products/unknown-id drop, consumables with quantity, non-consumables, subscriptions, pending/cancelled/failed with mapped codes, finishing, restore incl. empty, entitlements with expiry and consumed exclusion, storefront country, receipts returned raw (goals.md:11-19) | facade + backends | Each has a named test (rows E7-E25 below); quantity>1 pass-through asserted in "the options and the storefront pass through to the backend" (purchases.test.tsx:331, `lastOptions()` records quantity 2); country: `concurrent_non_purchase_calls_do_not_claim_the_gate` calls `country_code` (tests/purchases.rs:560) + GB pass-through (purchases.test.tsx:340) | Done |
| G4 | "Testing without a store. Every store interaction in this repo is a mock or a sandbox fixture" (goals.md:20) | all tests | Every test uses `FakeBackend`, a mock host, or `memoryBackend`; no store credential anywhere in the tree; ci.yml (read in full) contacts no store | Done |
| G5 | "a target whose host has not registered answers `ERR_PURCHASES_NOT_AVAILABLE`" (goals.md:3); Windows/Linux/web same (goals.md:5) | `platform/mod.rs:11-19`, `platform/unsupported.rs` | `an_unsupported_target_is_not_available` (tests/purchases.rs:535, `Purchases::new()` with no host: `is_available == false`, initialize → `ERR_PURCHASES_NOT_AVAILABLE`) | Done |

### The error enum (goals.md:24-40)

`ErrorKind::code()` in `src/error.rs:79-95`; TS strings in `packages/.../src/errors.ts:2-15`. `every_error_code_round_trips` (tests/purchases.rs:589) asserts all 13 pairs plus Display and reach-through-the-store; `ALL_KINDS` at tests/purchases.rs:80-97. TS: "every code reaches the caller as its code" (purchases.test.tsx:202) and "known codes pass through with their own code" (errors.test.tsx:45).

| # | Kind / Code | Rust string | TS string | Status |
|---|---|---|---|---|
| K1 | `NotAvailable` / `ERR_PURCHASES_NOT_AVAILABLE` | error.rs:81 ✓ | errors.ts:3 ✓ | Done |
| K2 | `NotInitialized` / `ERR_PURCHASES_NOT_INITIALIZED` | error.rs:82 ✓ | errors.ts:4 ✓ | Done |
| K3 | `InProgress` / `ERR_PURCHASES_IN_PROGRESS` | error.rs:83 ✓ | errors.ts:5 ✓ | Done |
| K4 | `Timeout` / `ERR_PURCHASES_TIMEOUT` | error.rs:84 ✓ | errors.ts:6 ✓ | Done |
| K5 | `Cancelled` / `ERR_PURCHASES_CANCELLED` | error.rs:85 ✓ | errors.ts:7 ✓ | Done |
| K6 | `ProductNotFound` / `ERR_PURCHASES_PRODUCT_NOT_FOUND` | error.rs:86 ✓ | errors.ts:8 ✓ | Done |
| K7 | `AlreadyOwned` / `ERR_PURCHASES_ALREADY_OWNED` | error.rs:87 ✓ | errors.ts:9 ✓ | Done |
| K8 | `NotOwned` / `ERR_PURCHASES_NOT_OWNED` | error.rs:88 ✓ | errors.ts:10 ✓ | Done |
| K9 | `PaymentInvalid` / `ERR_PURCHASES_PAYMENT_INVALID` | error.rs:89 ✓ | errors.ts:11 ✓ | Done |
| K10 | `NotAllowed` / `ERR_PURCHASES_NOT_ALLOWED` | error.rs:90 ✓ | errors.ts:12 ✓ | Done |
| K11 | `Network` / `ERR_PURCHASES_NETWORK` | error.rs:91 ✓ | errors.ts:13 ✓ | Done |
| K12 | `InvalidArgument` / `ERR_PURCHASES_INVALID_ARGUMENT` | error.rs:92 ✓ | errors.ts:14 ✓ | Done |
| K13 | `Backend` / `ERR_PURCHASES_BACKEND` — "carries its raw status where one was given" (goals.md:24,40) | error.rs:93 ✓; `status: Option<i64>` + `with_status` (error.rs:44-69); Backend-with-status: apple map_code all arms (apple.rs:147-159), android unknown codes (android.rs:87) | errors.ts:15 ✓ | Done |

TS wording identical to Rust `message()` for all 13, asserted in "the messages match the Rust crate's wording" (errors.test.tsx:74) against a table copied from error.rs.

### The shared types (goals.md:44-48)

| # | Claim | Where | Evidence | Status |
|---|---|---|---|---|
| T1 | `Product` — id, title, description, price (display string), currencyCode optional, kind Consumable/NonConsumable/Subscription; snake_case Rust / camelCase TS | `src/model.rs:29-42`; `src/store.ts:5-12` | Fixture products round-trip in `unknown_product_ids_are_dropped_from_the_answer` (tests/purchases.rs:259, ids asserted); TS fixtures (memory.ts:11-36); native seam camelCases via JsObject (`NativeProduct.currencyCode`, `.nsh/types/nsh-purchases/native.d.ts:24`) | Done |
| T2 | `Purchase` — id, productId, state Pending/Purchased/Restored, receipt string, purchasedAtMs optional | `src/model.rs:46-57`; `src/store.ts:18-24` | State values asserted in `a_pending_purchase_resolves_with_a_pending_state` (tests/purchases.rs:310) and `restore_returns_purchased_and_restored_purchases` (tests/purchases.rs:410); receipt on every purchase | Done |
| T3 | `Entitlement` — productId, active, expiresAtMs optional | `src/model.rs:61-68`; `src/store.ts:27-31` | `entitlements_respect_expiry` (tests/purchases.rs:500) asserts active and expiresAtMs; TS exact-shape asserts (purchases.test.tsx:312-319) | Done |

### Edge cases (goals.md:54-88) — one row per edge-case row

| # | Case (goal) | Test (exact name, file:line) | Asserts what the goal says? | Status |
|---|---|---|---|---|
| E1 | Second purchase while one runs → `ERR_PURCHASES_IN_PROGRESS`, running purchase untouched | `purchase_twice_in_parallel_is_in_progress` (tests/purchases.rs:100) | Yes: second call's kind, `purchases_started()==1` (refused call never reached the store), first still resolves Ok | Done |
| E2 | "Products, pending, entitlements **and restore** answer normally; the gate gates purchases only" | `concurrent_non_purchase_calls_do_not_claim_the_gate` (tests/purchases.rs:546) | Partially: exercises products, entitlements, pending, country_code during a run + `purchases_started()==1`; **restore is named in the goal and never called during the run** | Partial |
| E3 | Any store call before `initialize()` → `ERR_PURCHASES_NOT_INITIALIZED` | `calls_before_initialize_are_not_initialized` (tests/purchases.rs:189) | Products, purchase and restore asserted + `is_available` allowed; the other five gated calls (finish, consume, pending, entitlements, country_code) share the same `require_ready` guard (purchases.rs:229-235) but are not individually asserted. TS side: "calls before initialize..." (purchases.test.tsx:140) adds finish | Done |
| E4 | `initialize()` again → not an error, one store session | `initialize_is_idempotent` (tests/purchases.rs:212) | Yes: two Ok initializes, store still usable. TS: purchases.test.tsx:158 | Done |
| E5 | Empty product-id list → `ERR_PURCHASES_INVALID_ARGUMENT`, "refused before the store is asked" | `empty_product_ids_are_an_invalid_argument` (tests/purchases.rs:231) | Asserts the kind only; **no `product_queries()==0` assertion**, so "refused before the store is asked" is unproven (contrast E6/E8/E14 which do assert not-started). Code order (purchases.rs:93-97) does refuse before the backend | Partial |
| E6 | Zero quantity → `ERR_PURCHASES_INVALID_ARGUMENT` | `a_zero_quantity_is_an_invalid_argument` (tests/purchases.rs:241) | Yes: kind + `purchases_started()==0` ("refused before the store was asked") | Done |
| E7 | Mixed known/unknown ids → known answered, unknown dropped, no error | `unknown_product_ids_are_dropped_from_the_answer` (tests/purchases.rs:259) | Yes: exact id list asserted | Done |
| E8 | Buying an id the store does not have → `ERR_PURCHASES_PRODUCT_NOT_FOUND` | `purchasing_an_unknown_product_is_product_not_found` (tests/purchases.rs:277) | Yes: kind + `purchases_started()==0` | Done |
| E9 | Two store answers for one purchase → first resolves, second ignored | `a_purchase_resolves_exactly_once` (tests/purchases.rs:295) | Yes: first resolves; second `finish_purchase` returns `false` (dropped). TS: purchases.test.tsx:42 asserts `finishPurchase` second call `false` | Done |
| E10 | App cancels running purchase → `ERR_PURCHASES_CANCELLED`, gate frees for next call | `cancel_resolves_the_running_purchase_as_cancelled` (tests/purchases.rs:123) | Yes: `cancel()==true`, error kind, `cancels()==1`, late store answer delivered-and-dropped, next purchase runs. TS: purchases.test.tsx:56 | Done |
| E11 | User cancels the store sheet → `ERR_PURCHASES_CANCELLED` | `a_cancelled_purchase_resolves_with_the_cancelled_code` (tests/purchases.rs:324) | Yes: store answers Cancelled, code asserted | Done |
| E12 | StoreKit cancel + pending raw codes: cancel → cancelled; pending → Ok with state Pending, not Backend | `a_storekit_cancel_and_a_pending_purchase_resolve_as_documented` (tests/apple.rs:220) | Yes: raw 2 → `ErrorKind::Cancelled`; state 0 → `Ok` with `PurchaseState::Pending` | Done |
| E13 | Opt-in timeout passes unanswered → `ERR_PURCHASES_TIMEOUT`, gate recovers | `a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers` (tests/purchases.rs:146) | Yes: code asserted (tokio pause/advance), then a second purchase runs while the first is unanswered. TS: purchases.test.tsx:72 and the late-answer case :83 | Done |
| E14 | Zero timeout → `ERR_PURCHASES_INVALID_ARGUMENT`, refused up front | `a_zero_timeout_is_an_invalid_argument` (tests/purchases.rs:171) | Yes: kind + `purchases_started()==0` | Done |
| E15 | iOS ask-to-buy / Android awaiting funds → Ok with state Pending | `a_pending_purchase_resolves_with_a_pending_state` (tests/purchases.rs:310) | Yes: resolves Ok, state Pending. Android PENDING(2)→Pending in android.rs:230-231, exercised via `finishing_acknowledges_and_consuming_consumes` fixtures; TS: purchases.test.tsx:178 | Done |
| E16 | Purchases never finished listed by `pending()` until they are | `pending_returns_unfinished_purchases` (tests/purchases.rs:453) | Yes: queued before finish, empty after. TS: purchases.test.tsx:276 | Done |
| E17 | Android pending must be acknowledged: `finish()` acknowledges; consuming a consumable consumes | `finishing_acknowledges_and_consuming_consumes` (tests/android.rs:205) | Yes: consume reaches host's `consume`, finish reaches host's `acknowledge`, a consumable is not acknowledged, a non-consumable never reaches consume | Done |
| E18 | Finishing marks the purchase finished once | `finishing_marks_the_purchase_finished_once` (tests/purchases.rs:355) | Yes: finish Ok + `pending()` empty (the "already finished" half is E19). TS: purchases.test.tsx:214 | Done |
| E19 | Finishing unknown or already-finished → `ERR_PURCHASES_NOT_OWNED` | `finishing_an_unknown_or_finished_purchase_is_not_owned` (tests/purchases.rs:371) | Yes: both arms asserted. TS: purchases.test.tsx:237 | Done |
| E20 | Consuming a non-consumable → `ERR_PURCHASES_INVALID_ARGUMENT` | `consuming_a_non_consumable_is_an_invalid_argument` (tests/purchases.rs:390) | Yes: kind for pro_unlock + a consumable consume Ok. Apple side same rule (apple.rs:300-305) tested in `consuming_a_consumable_finishes_and_refuses_a_non_consumable` (tests/apple.rs:250) | Done |
| E21 | Owned non-consumable / active subscription bought again → `ERR_PURCHASES_ALREADY_OWNED` | `an_already_owned_product_is_already_owned` (tests/purchases.rs:336) | Yes: kind + scripted error consumed once + gate freed (next purchase runs). (Ownership is scripted, not tracked by the fake — acceptable at facade level.) TS: purchases.test.tsx:189 | Done |
| E22 | Restore returns Purchased and Restored with receipts (App Review requires the path) | `restore_returns_purchased_and_restored_purchases` (tests/purchases.rs:410) | Yes: Restored and Purchased states + receipt non-empty. TS: purchases.test.tsx:255 | Done |
| E23 | Restore with nothing → empty list, not an error | `restore_with_no_purchases_is_empty` (tests/purchases.rs:444) | Yes. TS: purchases.test.tsx:262 | Done |
| E24 | Consumed consumables gone from entitlements | `entitlements_exclude_consumed_consumables` (tests/purchases.rs:470) | Yes: contains pro_unlock, not coins_100. TS exact-list assert (purchases.test.tsx:307) | Done |
| E25 | Expired subscription reports not active | `entitlements_respect_expiry` (tests/purchases.rs:500) | Yes: `active:false` + expiry asserted. Honest caveat (also in .zcode run report): staged via `set_entitlements`, not a computed clock. TS asserts the expiresAtMs value only; the inactive half is Rust-only (said outright in purchases.test.tsx:310-311) | Done |
| E26 | No store / no host → `ERR_PURCHASES_NOT_AVAILABLE` | `an_unsupported_target_is_not_available` (tests/purchases.rs:535) | Yes: `Purchases::new()` (no host registered) → `is_available false`, initialize `ERR_PURCHASES_NOT_AVAILABLE`. TS: "a memory backend configured unavailable answers not available" (purchases.test.tsx:322) | Done |
| E27 | Caller drops a purchase call → store request cancelled, nothing leaks/hangs | `a_dropped_purchase_call_cancels_the_store_request` (tests/purchases.rs:573) | Yes: `JoinHandle::abort()` → `cancels()==1` via `Owner::drop` (purchases.rs:263-273) | Done |
| E28 | Every StoreKit raw code maps to its documented code | `every_storekit_raw_code_maps_to_its_documented_code` (tests/apple.rs:170) | Yes: all 8 raws 0-7 vs the table (tests/apple.rs:28-37) match goals' mapping (0→Backend, 1→InvalidArgument, 2→Cancelled, 3→PaymentInvalid, 4→NotAllowed, 5→Network, 6,7→NotAllowed), and asserts the raw rides as `status` | Done |
| E29 | Unknown StoreKit raw code → `ERR_PURCHASES_BACKEND`, raw kept | `unknown_storekit_raw_codes_are_backend` (tests/apple.rs:193) | Yes: 99 and -1 → Backend + `status()==Some(raw)` | Done |
| E30 | Every BillingClient raw code maps to its documented code | `every_billing_raw_code_maps_to_its_documented_code` (tests/android.rs:158) | Yes: all 12 documented codes (tests/android.rs:25-41) vs android.rs:66-90 (USER_CANCELED→Cancelled; SERVICE_UNAVAILABLE/TIMEOUT/DISCONNECTED/NETWORK_ERROR→Network; BILLING_UNAVAILABLE/FEATURE_NOT_SUPPORTED→NotAvailable; ITEM_UNAVAILABLE→ProductNotFound; DEVELOPER_ERROR→InvalidArgument; ERROR→Backend; ITEM_ALREADY_OWNED→AlreadyOwned; ITEM_NOT_OWNED→NotOwned) | Done |
| E31 | Unknown Billing raw code → `ERR_PURCHASES_BACKEND`, raw kept | `unknown_billing_raw_codes_are_backend` (tests/android.rs:177) | Yes: 99 and -7 → Backend + status kept | Done |
| E32 | Every stable code round-trips "through the error type and the TS bridge without change" | `every_error_code_round_trips` (tests/purchases.rs:589) | Rust: all 13 kinds × (code, Display, message, reaches caller through the store). TS: purchases.test.tsx:202 + errors.test.tsx:45,74. The literal rustBackend→native leg is type-checked only (see P-TS below), but `asPurchasesError` (the bridge's error path, errors.ts:60-72) is tested | Done |
| E33 | Sandbox vs production receipts: "the receipt reaches the app exactly as the store gave it"; library never branches on environment | cited `a_purchase_resolves_exactly_once`; `restore_returns_purchased_and_restored_purchases` (goals.md:88) | No test asserts receipt **equality** with the store's payload (E9 asserts product_id only; E22 asserts non-empty). "Never branches on environment" is true by reading (no env/branch code anywhere in src/) but untested as a behaviour | Partial |

### Receipt validation section (goals.md:90-92) and acceptance criteria (goals.md:94-108)

| # | Claim | Evidence | Status |
|---|---|---|---|
| A1 | Library returns the raw payload, never verifies, never claims valid; no backend | No verify API exists on any facade/backend (read in full); receipt is an opaque string (model.rs:53-54); doc trade-off matches code | Done |
| A2 | CI runs on every push to `master`: fmt, clippy -D warnings, cargo test --all-targets, nsh check + nsh test in packages/nsh-purchases | `.github/workflows/ci.yml` runs exactly those commands (ci.yml:32-34, 50-52); run 43967641194 green 2026-11-16 | Done |
| A3 | "Every store interaction in those runs is a mock or a sandbox fixture. No job ... touches a real store" | ci.yml (read in full) has no store credentials, no store service, no device steps | Done |
| A4 | "Every edge case above has its named test, **and each test was seen to fail before it passed**" | All 32 canonical names exist verbatim (26 in tests/purchases.rs, 3 in apple.rs — `every_storekit_raw_code_maps_to_its_documented_code`, `unknown_storekit_raw_codes_are_backend`, `a_storekit_cancel_and_a_pending_purchase_resolve_as_documented` — and 3 in android.rs). The seen-to-fail half is a process claim; no artifact in the repo records a red-first run — unverified | Partial |
| A5 | "ErrorKind::code(), the TypeScript `code` field and the table above use the same strings, verified by `every_error_code_round_trips`" | K1-K13 above; errors.test.tsx:74 proves TS wording = error.rs wording | Done |
| A6 | "docs/parity.md carries the evidence, one row per capability, each citing the test CI ran" | parity.md read in full against tests and reference clones (below); one row (P-E14) cites a test that does not cover its claim | Partial |

## docs/parity.md

Intro claims verified against the clones: OpenIAP index (`.refs/openiap/libraries/react-native-iap/src/index.ts`) exports `fetchProducts` (:927), `requestPurchase` (:1641), `getAvailablePurchases` (:1032), `acknowledgePurchaseAndroid` (:1953), `consumePurchaseAndroid` (:1991), `getStorefront` (:1108), `verifyPurchase` (:2045), and does **not** export `getProducts`/`getSubscriptions`/`requestSubscription`/`getPurchaseHistories`/`acknowledgePurchase`/`consumePurchase`/`validateReceipt`/`getStorefrontIOS`. Archived mirror (`.refs/react-native-iap/src/index.ts`) exports `validateReceipt` (:1573, deprecated :1551), `presentCodeRedemptionSheetIOS` (:1793), `requestPromotedProductIOS` (:807), `getStorefrontIOS` (:809), and the aliases `acknowledgePurchase = acknowledgePurchaseAndroid` (:2260), `consumePurchase = consumePurchaseAndroid` (:2265). The intro (parity.md:3) is accurate.

### Table 1 — Methods of `InAppPurchase` (parity.md:18-31)

| # | Reference API | Our API exists | Cited test exists and covers | Status |
|---|---|---|---|---|
| P1-1 | `InAppPurchase.instance` | `Purchases::new`/`with_backend` (purchases.rs:57-67); TS `new Purchases({backend})` (purchases.ts:23-25) over `rustBackend()`/`memoryBackend()` | `an_unsupported_target_is_not_available`; TS "a memory backend configured unavailable..." (purchases.test.tsx:322; parity's phrase is a paraphrase) | Done |
| P1-2 | `purchaseStream` — not in API; resolves once; late/duplicate dropped by generation guard | No stream anywhere; gate `owner`/`waiting` generation guard (purchases.rs:24-37, 244-253); TS `answerFreesGate` (purchases.ts:97) | `a_purchase_resolves_exactly_once`; `pending_returns_unfinished_purchases`; TS late-answer test purchases.test.tsx:83 | Done (Partial as the row itself says — status word honest) |
| P1-3 | `isAvailable()` — one call allowed before initialize; no host → false | `is_available` bypasses `require_ready` (purchases.rs:77-79); unsupported answers false (unsupported.rs:16-18) | `an_unsupported_target_is_not_available` asserts false + TS unavailable test | Done |
| P1-4 | `queryProductDetails` — found only, no notFoundIDs, empty ids → InvalidArgument before the store | purchases.rs:92-98 | `empty_product_ids_are_an_invalid_argument`; `unknown_product_ids_are_dropped_from_the_answer` (see E5 for the weaker half) | Done |
| P1-5 | `buyNonConsumable` — one entry point; owned again → AlreadyOwned | `purchase(product_id, options)` | `an_already_owned_product_is_already_owned`; `a_purchase_resolves_exactly_once` | Done |
| P1-6 | `buyConsumable` — autoConsume not offered (Partial) | No autoConsume; explicit `consume` | `finishing_acknowledges_and_consuming_consumes`; `consuming_a_non_consumable_is_an_invalid_argument` | Done (Partial honest) |
| P1-7 | `completePurchase` — Apple finish / Android acknowledge; unknown/finished → NotOwned where reference throws | `finish` (purchases.rs:176-179); android finish=acknowledge (android.rs:303-313) | `finishing_marks_the_purchase_finished_once`; `finishing_an_unknown_or_finished_purchase_is_not_owned` | Done |
| P1-8 | `restorePurchases` — return value not stream; applicationUserName not taken on restore | `restore() -> Result<Vec<Purchase>>`; no arg | `restore_returns_purchased_and_restored_purchases`; `restore_with_no_purchases_is_empty` | Done |
| P1-9 | `countryCode()` — Platform only; CI shows the pass-through | `country_code` everywhere; fake "US"; TS storefront config | tests/purchases.rs:560 (country_code during run); purchases.test.tsx:340 (GB) | Done (Platform only honest) |
| P1-10 | `getPlatformAddition` — no escape hatch; consume first-class | No such API; `consume` on the facade | `finishing_acknowledges_and_consuming_consumes` | Done (Partial honest) |
| P1-11 | (not in reference) `cancel()` — resolves running as Cancelled, late answer dropped, drop cancels too | `cancel` (purchases.rs:214-226); `Owner::drop` | `cancel_resolves_the_running_purchase_as_cancelled`; `a_dropped_purchase_call_cancels_the_store_request`; TS purchases.test.tsx:56 | Done |
| P1-12 | (not in reference) `timeout`/`timeoutMs` — opt-in, gate frees, late dropped, zero refused before gate | purchases.rs:114-119, 143-162; purchases.ts:53-108 | `a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers`; `a_zero_timeout_is_an_invalid_argument`; TS purchases.test.tsx:72,83 | Done |

### Table 2 — platform-interface types (parity.md:35-46)

| # | Claim | Verdict | Status |
|---|---|---|---|
| P2-1 | `ProductDetails` → `Product` (Partial: no rawPrice/currencySymbol; kind added) | True: model.rs:29-42 carries only display price + currency code; fixture round-trip in tests/purchases.rs:259 | Done |
| P2-2 | `PurchaseDetails` → `Purchase` (Partial: pendingCompletePurchase is `pending()`) | True; `pending_returns_unfinished_purchases` | Done |
| P2-3 | PurchaseStatus pending/purchased/restored → states Done | tests/purchases.rs:310, :410 | Done |
| P2-4 | PurchaseStatus error/canceled not states (Partial) | True — failures resolve the call; `a_cancelled_purchase_resolves_with_the_cancelled_code` | Done |
| P2-5 | `PurchaseParam.productDetails` → `product_id: &str` (Partial) | True; `purchasing_an_unknown_product_is_product_not_found` | Done |
| P2-6 | `applicationUserName` Accepted: forwarded to the Apple host; Android bridge + fakes accept and ignore | True by reading: apple.rs PurchaseRequest carries it (apple.rs:74-81, 269-273); android.rs:149-157 carries it, host ignores; TS pass-through asserted (purchases.test.tsx:337). The Apple-host forwarding itself has no test — the row honestly cites "none beyond the field's pass-through" | Done |
| P2-7 | `quantity` Accepted: sent to Apple; Play ignores; zero InvalidArgument everywhere | apple.rs sends quantity (apple.rs:271); android request carries it "the host ignores it" (android.rs:150-153); zero tested both languages (tests/purchases.rs:241; purchases.test.tsx:124) | Done |
| P2-8 | `ProductDetailsResponse` → `Result<Vec<Product>>` (Partial) | True; `unknown_product_ids_are_dropped_from_the_answer` | Done |
| P2-9 | `PurchaseVerificationData` → one receipt string, never verified (Partial) | True; receipt rides E9's purchase | Done |
| P2-10 | `IAPError` → `PurchasesError { kind, status }` / `{ code }` Done | error.rs:41-75; errors.ts:40-48; `every_error_code_round_trips`; errors.test.tsx:45 | Done |

### Table 3 — Apple StoreKit 2 (parity.md:52-67)

| # | Claim | Verdict | Status |
|---|---|---|---|
| P3-1 | `products`/`purchase`/`finish` Done | tests/purchases.rs:259 / apple.rs:220 / tests/purchases.rs:355 | Done |
| P3-2 | `SK2Product.purchase` — userCancelled→Cancelled, pending→Ok Pending | apple.rs:143-146 (host contract: cancel arrives as raw 2, pending as state 0); tests/apple.rs:220-246 | Done |
| P3-3 | quantity + account name forwarded; offers out of scope (Partial) | apple.rs PurchaseRequest; `a_zero_quantity_is_an_invalid_argument` | Done |
| P3-4 | offer eligibility not in API (Partial) | No such API; verified absence | Done |
| P3-5 | `transactions()`/`unfinishedTransactions()` → `pending()` Done | apple.rs:321-329 maps `unfinished`; `pending_returns_unfinished_purchases` | Done |
| P3-6 | `currentEntitlements()` → `entitlements()` Done, expiry + consumed honoured | `entitlements_respect_expiry`; `entitlements_exclude_consumed_consumables` | Done |
| P3-7 | `restorePurchases()` → `restore()` Done | tests/purchases.rs:410 | Done |
| P3-8 | `AppStore.canMakePayments` not in API; restriction = `ERR_PURCHASES_NOT_ALLOWED` (Partial) | raw 4 → NotAllowed in apple.rs:152 and STOREKIT_MAP (tests/apple.rs:33) | Done |
| P3-9 | `AppStore.sync()` folded into restore (Partial) | No sync API; `restore_returns_purchased_and_restored_purchases` | Done |
| P3-10 | `Storefront.countryCode()` Platform only | apple.rs:341-350 (empty → Backend, tested tests/apple.rs:328-331) | Done |
| P3-11 | Store UI sheets not in API (Partial) | Verified absence | Done |
| P3-12 | StoreKit 1 wrappers not wrapped (Partial) | Verified absence; no SK1 symbols in src/ | Done |
| P3-13 | SKError code table Done; unknown raw → Backend carrying status, never panic | apple.rs:147-168; tests/apple.rs:170, 193; also malformed payloads → Backend (tests/apple.rs:289) | Done |
| P3-14 | Bridge compiles on every runner; mapping tests run in CI against a mock host (parity.md:50) | apple.rs links no SDK (read in full); tests/apple.rs runs on ubuntu CI (run green) | Done |

### Table 4 — Android BillingClient (parity.md:73-86)

| # | Claim | Verdict | Status |
|---|---|---|---|
| P4-1 | `startConnection` → `initialize()` idempotent; before it → NotInitialized; no end call | `initialize_is_idempotent`; `calls_before_initialize_are_not_initialized`; no end API (verified) | Done |
| P4-2 | `isReady()` → `is_available` Done | `an_unsupported_target_is_not_available` | Done |
| P4-3 | `queryProductDetails` → `products` Done | tests/purchases.rs:259 | Done |
| P4-4 | `launchBillingFlow` → `purchase`, one at a time, non-purchase calls never claim the gate | `purchase_twice_in_parallel_is_in_progress`; `concurrent_non_purchase_calls_do_not_claim_the_gate` (see E2 caveat) | Done |
| P4-5 | `queryPurchases` split pending/entitlements Done | `pending_returns_unfinished_purchases`; `entitlements_exclude_consumed_consumables` | Done |
| P4-6 | `acknowledgePurchase` → `finish` Done, counters on the mock host | tests/android.rs:240-245 | Done |
| P4-7 | `consumeAsync` → `consume` Done; non-consumable → InvalidArgument | tests/android.rs:214-229, 247-258; facade tests/purchases.rs:390 | Done |
| P4-8 | PENDING → Pending state via `pending()`, resolves Ok | android.rs:230-231; `a_pending_purchase_resolves_with_a_pending_state`; `pending_returns_unfinished_purchases` | Done |
| P4-9 | `getBillingConfig` country code Platform only | android.rs:364-375; pass-through tested as P1-9 | Done |
| P4-10 | Alternative billing / user-choice listeners not in API (Partial) | Verified absence | Done |
| P4-11 | `showInAppMessages()` not in API (Partial) | Verified absence | Done |
| P4-12 | BillingResponse table Done; unknown → Backend carrying status | android.rs:66-90; tests/android.rs:158, 177; unknown purchase states → Backend (tests/android.rs:263) | Done |

### Table 5 — `react-native-iap` (parity.md:90-122)

All 31 rows' "not in the API" claims verified by absence in src/; every reference export named was found in the clone's index (line numbers in the intro check above). Rows with tests:

| # | Claim | Verdict | Status |
|---|---|---|---|
| P5-1 | `initConnection` → `initialize()` Done, no config taken | True (no config param); `initialize_is_idempotent`; `calls_before_initialize_are_not_initialized`; TS purchases.test.tsx:140 | Done |
| P5-2 | `endConnection` not in API (Partial: nothing to close) | Verified absence | Done |
| P5-3 | `fetchProducts` → `products` Done, empty skus InvalidArgument | `empty_product_ids_are_an_invalid_argument`; `unknown_product_ids_are_dropped_from_the_answer` | Done |
| P5-4 | `requestPurchase` Partial by design (resolve-once, no `andFinishIOS`) | `a_purchase_resolves_exactly_once`; `purchase_twice_in_parallel_is_in_progress` | Done |
| P5-5 | `purchaseUpdatedListener` not in API (Partial) | Verified absence; E9/pending tests cover the replacement | Done |
| P5-6 | `purchaseErrorListener` not in API (Partial) | Failures resolve the call: `a_cancelled_purchase_resolves_with_the_cancelled_code`; `a_timeout_resolves_...` | Done |
| P5-7 | `finishTransaction` → `finish`+`consume` Done | tests/purchases.rs:355, :390 | Done |
| P5-8/9 | `acknowledgePurchaseAndroid`→finish, `consumePurchaseAndroid`→consume Done | tests/android.rs:205 | Done |
| P5-10 | `getAvailablePurchases` → pending+restore (Partial, iOS-only options absent) | Both tests exist | Done |
| P5-11 | `restorePurchases` → `restore()` Done | tests/purchases.rs:410, :444 | Done |
| P5-12 | `getStorefront` Platform only (mirror keeps `getStorefrontIOS` — true, mirror :809) | Pass-through tested as P1-9 | Done |
| P5-13 | `getActiveSubscriptions`/`hasActiveSubscriptions` → `entitlements()` Done | `entitlements_respect_expiry`; `entitlements_exclude_consumed_consumables` | Done |
| P5-14 | `useIAP()` → `use_entitlements(&Purchases)` / `useEntitlements(options)` Partial — entitlements only | The APIs exist (entitlements.rs:47; entitlements.ts:33) but **no test in either language exercises the hook** (grep over tests/: zero hits). The cited tests (`entitlements_respect_expiry`, "entitlements incl. expiry") cover `entitlements()`, not the hook. App.tsx uses it, compiled only | Partial |
| P5-15 | `verifyPurchase`/`validateReceipt` not in API (Partial, app-side) | Verified absence; receipt rides E9 | Done |
| P5-16 | `verifyPurchaseWithProvider` not in API | Verified absence (reference export confirmed :2198) | Done |
| P5-17 | `syncIOS` folded into restore (Partial) | tests/purchases.rs:410 | Done |
| P5-18 | `getPendingTransactionsIOS` → `pending()` Done | tests/purchases.rs:453 | Done |
| P5-19 | `currentEntitlementIOS` → `entitlements()` store-wide Done | `entitlements_respect_expiry` | Done |
| P5-20 | `subscriptionStatusIOS`/`latestTransactionIOS` Partial | `entitlements_respect_expiry`; resolved call returns the purchase (E9) | Done |
| P5-21 | `getReceiptDataIOS`/`requestReceiptRefreshIOS` Partial | E9 + restore tests | Done |
| P5-22 | `isTransactionVerifiedIOS`/`getTransactionJwsIOS` — JWS is the receipt string (Partial) | True (model receipt); E9 | Done |
| P5-23..31 | getAppTransaction/verifyAppOwnership, getAllTransactionsIOS, intro-offer eligibility, promoted purchases (land in pending), store UI sheets/refunds, deep links, external-purchase/billing programs, purchase-flow kit, platform probes — not in API (Partial) | All verified absent from src/; referenced exports confirmed in the clone's index; `pending_returns_unfinished_purchases` covers the promoted-purchase landing claim; `an_unsupported_target_is_not_available` covers the platform-probe answer | Done |

### Table 6 — `ErrorCode` mapping (parity.md:128-143)

| # | ErrorCode member | Claim | Verdict | Status |
|---|---|---|---|---|
| P6-1 | `user-cancelled` → Cancelled Done | tests/purchases.rs:324 | Done |
| P6-2 | `already-owned`/`duplicate-purchase` → AlreadyOwned Done | tests/purchases.rs:336 | Done |
| P6-3 | `item-not-owned`/`not-ended` → NotOwned Done | tests/purchases.rs:371 | Done |
| P6-4 | `item-unavailable`/`sku-not-found` → ProductNotFound Done | tests/purchases.rs:277 | Done |
| P6-5 | `empty-sku-list`/`developer-error`/`user-error` → InvalidArgument Done, checked before the gate | Facade: tests/purchases.rs:231, :241 (arg checks precede gate claim, purchases.rs:114-133); DEVELOPER_ERROR arm tested tests/android.rs:36 | Done |
| P6-6 | `pending`/`deferred-payment` not an error, Ok Pending Done | tests/purchases.rs:310 | Done |
| P6-7 | `not-prepared`/`init-connection`/`connection-closed` → NotInitialized Done | tests/purchases.rs:189 | Done |
| P6-8 | `already-prepared` never an error Done | `initialize_is_idempotent` | Done |
| P6-9 | `iap-not-available`/`billing-unavailable`/`feature-not-supported`/`activity-unavailable` → NotAvailable Done | `an_unsupported_target_is_not_available` + BILLING_UNAVAILABLE/FEATURE_NOT_SUPPORTED arms in tests/android.rs:30,34 | Done |
| P6-10 | `network-error`/`service-disconnected` → Network Done | NETWORK_ERROR and SERVICE_DISCONNECTED arms in tests/android.rs:31,40 | Done |
| P6-11 | `service-timeout` → Timeout (opt-in) / Network (connection died) Partial — honest | SERVICE_TIMEOUT→Network arm (tests/android.rs:26); facade Timeout only when set (tests/purchases.rs:146) | Done (Partial honest) |
| P6-12 | `payment-declined`/`purchase-error` → `PaymentInvalid` **Done**, covered by `every_billing_raw_code_maps_to_its_documented_code` | **Wrong.** `map_billing_code` (android.rs:66-90) has **no arm producing `PaymentInvalid`**, and the cited test's table (tests/android.rs:25-41) has no PaymentInvalid case. The reference's `payment-declined` is an Android sub-response code (`payment-declined-due-to-insufficient-funds`, openiap types.ts:1945); nothing in this repo maps it to `ERR_PURCHASES_PAYMENT_INVALID`. That code is reachable only via the Apple bridge (raw 3) and the facade fake | Partial — status word and citation incorrect |
| P6-13 | `purchase-verification-*` not errors here Partial | Verified absence | Done |
| P6-14 | catch-all → Backend with raw status Done | tests/android.rs:177; tests/apple.rs:193; errors.test.tsx:62 | Done |
| P6-15 | Added `ERR_PURCHASES_IN_PROGRESS` note (parity.md:145) | tests/purchases.rs:100 | Done |

### parity.md design-differences paragraph (parity.md:14)

| # | Claim | Verdict | Status |
|---|---|---|---|
| P-D1 | No purchase stream; resolve-once; restore by return value; receipt app-side; unsupported targets NotAvailable; one StoreKit host covers iOS+macOS, one Billing host | All verified by reading src/ + E9, E22, E26; "neither reference ships a store on those targets either" — consistent with the clones (no web/windows/linux store in either index) | Done |

## docs/decisions.md

| # | Decision | Evidence | Status |
|---|---|---|---|
| D1 | Four reference repos cloned into `.refs/`; react-native-iap archived, canonical continuation `hyodotdev/openiap` cloned alongside (decisions.md:7-12) | `.refs/` contains exactly: `flutter-packages` (has `packages/in_app_purchase`), `react-native-iap` (archived mirror, AGENTS/CHANGELOG present), `openiap` (has `libraries/react-native-iap`), `nsh` (main @091049a, 2026-11-09, workspace version 0.4.0 — main as of the clone date), `biometrics` — all five match the doc | Done |
| D2 | NSH pin 0.5.1; `nsh-core` 0.5.1 on crates.io; CI installs the 0.5.1 CLI (decisions.md:14-16) | crates/nsh-purchases/Cargo.toml:15 `nsh-core = "0.5.1"`; Cargo.lock resolves nsh-core 0.5.1 from the registry; ci.yml:43 `NSH_VERSION: v0.5.1`; CI green. Nit: "The brief pinned NSH 0.4.0" — `docs/brief.md` pins no version; the 0.4.0 pin lived in `.zcode/purchases-run.ts:312` (the run plan) | Done |
| D3 | Native StoreKit 2 + Play Billing, no hosted service; RevenueCat rejected | No hosted anything: no network calls, no SDK, receipts app-side (A1) | Done |
| D4 | Bridges link no store SDK; host-callback seam (`apple::StoreKitHost`, `android::BillingHost`); glue in NSH hosts; not-available until a host registers | apple.rs/android.rs read in full: only `OnceLock` hosts + mapping, no externals; platform/mod.rs falls through to unsupported; E26 | Done |
| D5 | Platforms: Apple (iOS+macOS) + Android; Windows/Linux/web unsupported backend | platform/unsupported.rs; wasm32 cfg in purchases.rs:143-169 keeps web compiling; E26 | Done |
| D6 | Cancels are errors; pending resolves Ok with pending state | error.rs Cancelled; purchases.rs docs; E10, E11, E15 | Done |

## docs/brief.md

| # | Item | Evidence | Status |
|---|---|---|---|
| B1 | Options considered (SK2 native, Play Billing native, RevenueCat hosted, no Rust lib found) — research note (brief.md:5-10) | A brief, not a code claim; the recommended path is what was built | Done |
| B2 | Recommended path: native SK2 + Play Billing behind one NSH trait, app-side validation (brief.md:14) | `Backend` trait + bridges; A1 | Done |
| B3 | Edge: sandbox and production use different receipts and keys (brief.md:18) | Addressed: goals.md E33 + trade-off section; library never branches (true by reading); E33 marked Partial at test level | Done |
| B4 | Edge: restore flow required by App Review (brief.md:19) | `restore()` + E22, E23 (empty restore not an error) | Done |
| B5 | Edge: Android pending purchases must be acknowledged (brief.md:20) | finish=acknowledge; E17 | Done |
| B6 | Edge: receipt validation without a backend is limited; document the trade-off (brief.md:21) | goals.md:90-92 states it plainly; matches code | Done |

## README.md

| # | Claim | Evidence | Status |
|---|---|---|---|
| R1 | "CI on GitHub Actions, and a web status page on GitHub Pages at https://supernsh.github.io/purchases/" (README.md:5) | ci.yml + pages.yml exist; runs 43967641194 (CI) and 43967641287 (Pages) green 2026-11-16 | Done |
| R2 | Status: "...the gates pass locally and CI runs them on push (**no CI run yet — nothing has been pushed**)" (README.md:9) | CI run 43967641194 completed green on 2026-11-16 (`gh run list`); three pushes have run (43053072945, 43217900274/452, 43967641194/287) | **Stale** |
| R3 | Approach: host callbacks `apple::StoreKitHost`, `android::BillingHost`; raw codes mapped; glue in NSH hosts; receipt validation in the app (README.md:13) | D4, A1 | Done |
| R4 | Layout list: crates/, packages/, docs/ (goals, decisions, parity, brief), site/ published by pages.yml, .zcode/ (README.md:17-21) | All exist; pages.yml:29-31 uploads `site/` | Done |
| R5 | "ci.yml runs `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `nsh check` with `nsh test` in packages/nsh-purchases" (README.md:25) | ci.yml:32-34 and :50-52 — exact match | Done |
| R6 | MIT licence, LICENSE file, repo URL (README.md:29-31) | LICENSE exists; `gh` commands against SuperNSH/purchases succeed | Done |

## site/index.html

| # | Claim | Evidence | Status |
|---|---|---|---|
| S1 | "**Status: scaffold. No implementation yet.** This page is a status page, not a demo." (site/index.html:23) | The crate, package, 39 Rust + 24 TS tests and a green CI run exist; App.tsx is a demo | **Stale** |
| S2 | Links to brief.md / parity.md as "Planned approach" / "Feature status" (site/index.html:24-25) | Both docs exist and are current | Done |

## .zcode/workflow.ts

| # | Claim | Evidence | Status |
|---|---|---|---|
| Z1 | Area "purchases": all six phases `"not-started"` (workflow.ts:24-31) | Implementation, audit (this file), unit tests and CI all exist and are green; the phases ran (per the run report and the tree itself) | **Stale** |
| Z2 | Gates: fmt, clippy -D warnings, cargo test (workflow.ts:15-19) | Matches ci.yml's rust job | Done |

## .zcode/purchases-run-report.md

| # | Claim | Evidence | Status |
|---|---|---|---|
| ZR1 | "No commits or pushes were made" / "No CI run has executed — nothing has been pushed" (lines 4-5, 74, 112) | True then; the repo has since been pushed 3+ times with green CI | **Stale** (historical report, now wrong about the world) |
| ZR2 | "ci.yml gained a typescript job (NSH **v0.4.0** installer...)" (line 33) | ci.yml now pins `NSH_VERSION: v0.5.1` (ci.yml:43) | **Stale** detail |
| ZR3 | Test counts: "39 tests: 4 lib, 26 purchases, 5 apple, 4 android" (line 57-58) | Verified by reading: native.rs has 4 `#[test]`s (native.rs:269-308), tests/purchases.rs 26, tests/apple.rs 5, tests/android.rs 4 | Done |
| ZR4 | "24/24" TS tests (line 60) | purchases.test.tsx has 19 `test(` calls, errors.test.tsx 5 — 24 total | Done |
| ZR5 | "The 32 canonical test names in docs/goals.md ... all 32 exist and pass" (lines 92-93) | All 32 names exist verbatim (E1-E31 + E32); "pass" backed by CI run green | Done |
| ZR6 | 105 parity rows (line 15) | parity.md has 105 table lines = 93 data rows + 12 header/separator lines; close but not row-accurate | Stale (minor) |

## NSH 0.4.0 / 0.5.0 mentions

Every match for `0.4.0|0.5.0|0.5.1|NSH_VERSION` outside `.refs/`, `Cargo.lock` and generated `.nsh/` output:

| Where | What it says | True as of 0.5.1? | Verdict |
|---|---|---|---|
| crates/nsh-purchases/Cargo.toml:15 | `nsh-core = "0.5.1"` | Yes; Cargo.lock resolves 0.5.1 from crates.io; CI green on it | Keep |
| docs/decisions.md:14-16 | Pin history 0.4.0 → 0.5.0 (2026-11-12) → 0.5.1 (2026-11-16), no source change; 0.5.1 on crates.io; CI installs 0.5.1 | Matches git log (`build: move to NSH 0.5.0`, `fix: build on wasm32 and move to NSH 0.5.1`), Cargo.lock, ci.yml. Nit: docs/brief.md pins no version — the 0.4.0 pin was in .zcode/purchases-run.ts:312 | Keep (fix the "brief pinned" attribution) |
| packages/nsh-purchases/src/entitlements.ts:23-24 | "NSH 0.4.0's web runtime ships no `lifecycle` accessor: the import stays `undefined` there ... the foreground re-check comes back with an NSH release that has it" | Questionable. Even the 0.4.0-era nsh source in `.refs/nsh` routes lifecycle on web (nsh-platform-web/src/lib.rs:586-608), and the 0.5.1-generated typings export `lifecycle` (.nsh/types/nsh/index.d.ts:1230). Whether the 0.5.1 web **runtime** exports the JS accessor is not verifiable from this repo (needs a web run, which AGENTS.md forbids locally). The `typeof lifecycle === "function"` guard is harmless either way | Likely **Stale** on 0.5.1 — unverified; keep the guard, re-word or verify the comment on device |
| .github/workflows/ci.yml:43 | `NSH_VERSION: v0.5.1` | Yes | Keep |
| .zcode/purchases-run-report.md:33 | "NSH v0.4.0 installer" in ci.yml | File now says v0.5.1 | Stale (historical) |
| .zcode/purchases-run.ts:92, 312, 578 | Plan pins nsh-core 0.4.0 / NSH v0.4.0 installer; "copied from how .refs/biometrics uses nsh-core 0.4.0" | Historical run script (the plan that was executed), not a live claim; biometrics' ci.yml indeed pins v0.4.0 | Historical — no action |

## Gaps versus the reference-repo standard

`.refs/biometrics/.github/workflows/ci.yml` is the org's full pipeline. This repo's ci.yml has: one rust job (fmt + clippy + test, ubuntu only) and one typescript job (`nsh check`, `nsh test`). Absent, relative to the reference standard:

- **Cross-target checks**: biometrics clippy-checks `aarch64-apple-ios`, `aarch64-apple-ios-sim` (NDK cc wiring included), `aarch64-linux-android`, and `wasm32-unknown-unknown`. Nothing here proves the crate compiles for the four targets it ships for — notable because the wasm32 support was added blind and fixed post-hoc (commit "fix: build on wasm32").
- **OS matrix for the rust job** (ubuntu/macos/windows in the reference; ubuntu only here).
- **`nsh build` with a zero-warnings gate** (reference runs `nsh build` and fails on any "warn" in the log; here nothing builds the package bundle in CI).
- **Demo/device-adjacent builds**: the reference builds `examples/demo` for 9 targets on push to master (macos arm64/x86_64, linux x86_64/arm64, windows x86_64/arm64, ios-simulator, android, web). Nothing here builds an app; `examples/` is another agent's in-flight work.
- **Compiler caching** (kache-action in the reference; no cache here — 4-5 min CI runs).
- **Publish**: neither repo has a release/publish workflow — absent from both, listed for completeness.
- Pages: this repo is *ahead* of the reference here (pages.yml exists; biometrics has none) — but the page content is stale (S1).

## Summary

Counts by status over the claim rows above (goals G/K/T/E/A = 60, parity P-* = 87, decisions D = 6, brief B = 6, README R = 6, site S = 2, workflow Z = 2, run report ZR = 6 — **175 rows**):

- **Done**: 162
- **Partial**: 7 — E2, E5, E33, A4, A6, P5-14, P6-12
- **Stale**: 6 — R2, S1, Z1, ZR1, ZR2, ZR6
- **Untested / Missing / Blocked**: 0 rows as standalone statuses. Two untested surfaces are folded into Partial rows: the `use_entitlements`/`useEntitlements` hooks (P5-14) and `rustBackend()` — never instantiated by any test, only type-checked via App.tsx and `nsh check`; no parity row names the latter as a claim.

The NSH-version table above grades 7 mentions separately: 4 keep (current on 0.5.1), 2 stale (run-report's v0.4.0-installer line — same finding as ZR2 — and the entitlements.ts 0.4.0 web-lifecycle comment, likely stale but unverifiable without a web run), 1 historical no-action (the run plan's 0.4.0 pins).

Every non-Done row, with its reason:

1. **E2 (goals edge: non-purchase calls during a run)** — Partial. The goal names restore among the calls that must answer normally; `concurrent_non_purchase_calls_do_not_claim_the_gate` (tests/purchases.rs:546) never calls restore during the run.
2. **E5 (empty product ids)** — Partial. Test asserts the error kind but not "refused before the store is asked" (no `product_queries()==0` assertion, unlike the zero-quantity/zero-timeout tests).
3. **E33 (sandbox/production receipts)** — Partial. No test asserts the receipt reaches the app exactly as the store gave it (only product_id / non-empty checks); "never branches on environment" is true by reading, untested.
4. **A4 (acceptance: tests "seen to fail once")** — Partial. All 32 canonical names exist and are green in CI, but no artifact in the repo records a red-first run; process claim, unverifiable from code.
5. **A6 (acceptance: parity carries per-row evidence)** — Partial. One row (P6-12) cites a test that does not cover its claim.
6. **P5-14 (useIAP → use_entitlements/useEntitlements)** — Partial. Both hooks exist; neither has any test (grep over both test dirs: zero hits). The cited tests cover `entitlements()`, not the hook. Related untested surface: `rustBackend()` is never exercised by a test either.
7. **P6-12 (payment-declined → PaymentInvalid, "Done")** — Partial, and the row is wrong: `map_billing_code` (android.rs:66-90) has no arm producing `PaymentInvalid` and the cited test table (tests/android.rs:25-41) contains no such case. `ERR_PURCHASES_PAYMENT_INVALID` is reachable only via the Apple bridge (raw 3) and the scripted fake. Fix the row (or the bridge) — today doc and code disagree.
8. **R2 (README Status)** — Stale. "no CI run yet — nothing has been pushed": CI run 43967641194 completed green 2026-11-16; three pushes have run.
9. **S1 (site/index.html)** — Stale. "Status: scaffold. No implementation yet." — everything is implemented and CI is green; the published Pages content contradicts the README.
10. **Z1 (.zcode/workflow.ts)** — Stale. All phases "not-started"; implement/unit-tests/audit exist and CI is green.
11. **ZR1 (.zcode/purchases-run-report.md)** — Stale. "No CI run has executed — nothing has been pushed" (x3).
12. **ZR2** — Stale. Describes ci.yml's installer as NSH v0.4.0; the file pins v0.5.1.
13. **ZR6** — Stale (minor). "105 rows" counts table lines; parity.md has 93 data rows.
14. **entitlements.ts:23 (NSH 0.4.0 web-lifecycle comment)** — Likely stale on 0.5.1, unverified from this repo; the defensive guard makes it safe either way. Re-word or verify on a web target.

Surprising finds, for the record: the doc set is unusually honest (Accepted/Platform-only rows really do carry no test, and the TS test that overclaims in its name says so in a comment inside, purchases.test.tsx:310-311); the facade's two genuinely subtle behaviours — the generation guard and drop-cancels-store — are real and tested in both languages; the only outright false row in sixty pages of docs is the Android `payment-declined` mapping; and the store glue living outside this repo means the Apple/Android bridges here are mapping tables plus plumbing, exactly as decisions.md admits.

## Re-audit, 2026-11-17 (Phase 6)

Every gate re-run locally at HEAD `ff620ab` (working tree clean, `git status --short` empty), kache bypassed (`RUSTC_WRAPPER=` unset, kache shims stripped from PATH). Exit codes:

| Gate | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo test --all-targets` | 0 | 44 passed, 0 failed: 4 lib, 26 purchases, 5 entitlements, 5 apple, 4 android |
| clippy `aarch64-apple-ios` | 0 | clean |
| clippy `aarch64-apple-ios-sim` | 0 | clean |
| clippy `wasm32-unknown-unknown` (llvm-ar via `$(brew --prefix llvm)/bin`) | 0 | clean |
| clippy `aarch64-linux-android` (NDK 28.2.13676358 cc/linker/ar) | 0 | clean |
| `packages/nsh-purchases`: `nsh check` / `nsh build` / `nsh test` | 0 / 0 / 0 | build output zero-warning; 30 tests passed, 0 failed |
| `examples/demo`: `nsh check` / `nsh build` / `nsh test` | 0 / 0 / 0 | build output zero-warning; 6 tests passed, 0 failed |

The two known non-issues appeared only in `nsh test` output (the headless runner's dev-profile `__eh_frame` linker note as "1 warning: -v shows it", and the `status bar is not granted` lines) — neither in any `nsh build` log, which is the zero-warning gate. All since commit `bc9d1b4` (tests) and `ff620ab` (this audit's fixes).

### New status of every previously non-Done row

| Row | New status | Evidence |
|---|---|---|
| E2 | **Done** | `concurrent_non_purchase_calls_do_not_claim_the_gate` (tests/purchases.rs:561) now calls `restore()` during the run (:576) and asserts `fake.restores()==1` "the restore ran while a purchase was in flight" (:577-581); the counter is `FakeBackend::restores()` (src/fake.rs:162) |
| E5 | **Done** | `empty_product_ids_are_an_invalid_argument` (tests/purchases.rs:231) now asserts `fake.product_queries()==0` "the store was not asked" (:237; counter at src/fake.rs:172) |
| E33 | **Done** | Receipt equality asserted byte-for-byte against the store's payload in three places: `a_purchase_resolves_exactly_once` (tests/purchases.rs:304-307, `"sandbox-receipt-coins_100"`), `restore_returns_purchased_and_restored_purchases` (:437-441, :450-453, both rows); the payload literal comes from `store_purchase` (:35). The "never branches on environment" half still holds by reading: the only `sandbox` matches in `src/` are fixture names |
| P5-14 | **Done** | Both hooks now tested. Rust: tests/entitlements.rs, 5 tests on `use_entitlements` over a `Headless` engine (first read, re-run only on change, changed answer on foreground return, failed read's code, refresh dead after owner gone — :154, :163, :186, :201, :211). TS: tests/entitlements.test.tsx, 5 tests on `useEntitlements` (first read, failed read, refresh, stale read ignored by generation guard, no read after dispose — :18, :35, :48, :73, :107) |
| rustBackend (folded into P5-14's Partial) | **Done** | "the Rust crate's backend on a desktop with no store host is not available" (purchases.test.tsx:332-338) instantiates `new Purchases({ backend: rustBackend() })` and asserts not-available + the code; parity.md:20 cites it |
| A4 | **Partial — unchanged in kind, better attested** | The fail-first half is still a process claim with no machine artifact in the tree. What the tree now records: commit `bc9d1b4` states "Every new or strengthened assertion was seen to fail first", and the run report gained a dated Corrections section (see ZR1) — a report that corrects itself is more credible, not less. Unverifiable from the tree alone: any individual red-first run; the attestations are commit messages and prose, not logs |
| A6 | **Done** | With P6-12 fixed, every parity row's citation now covers its claim; re-walked rows spot-checked below |
| P6-12 | **Done** (the row is now honest) | parity.md:141 now reads "Apple: `PaymentInvalid`; Android: `Backend` — BillingClient documents no declined-payment raw code, so such a failure lands on `ERROR`", status Partial. Matches the code: `map_billing_code` has no PaymentInvalid arm, `ERROR => Backend` (android.rs:81), unknown keeps status (:87); Apple raw 3 → `PaymentInvalid` (apple.rs:151, table tests/apple.rs:32) |
| R2 | **Done** | README.md:9 now says "the gates pass and CI runs them on every push to `master`"; the "no CI run yet" claim is gone |
| S1 | **Done** | `site/` deleted; the Pages job now deploys the packaged web demo itself (ci.yml:242-274), so no stale scaffold page can be published |
| Z1 | **Done** | .zcode/workflow.ts:26-33: implement/audit/unit-tests `done`, device-test `blocked` (reason at :4-5: needs an NSH host registering StoreKitHost/BillingHost, outside this repo), vision/smoke `not-started` — matches reality |
| ZR1 | **Done** | Run report gained "## Corrections, 2026-11-17" (purchases-run-report.md:115-124) naming all three no-longer-true statements: pushes/CI (green run 43967641194), the v0.5.1 installer, and 93 data rows not 105. The body is left as the historical record, correctly |
| ZR2 | **Done** | Same Corrections section, line 123 |
| ZR6 | **Done** | Same Corrections section, lines 123-124 |
| entitlements.ts 0.4.0 web-lifecycle comment | **Done** | entitlements.ts:19-21 reworded: no checked NSH runtime (0.4.0 and 0.5.1) lacks `lifecycle`; the guard is "cheap safety" defaulting to active. Guard kept verbatim at :22. Commit `bc9d1b4` records the verification "against the 0.5.1 sources"; that check itself left no artifact, but the claim no longer contradicts the sources and the guard makes it non-load-bearing |

### Spot-checks of rows marked Done (10 + 3 extra, all still hold)

1. **G2/E9** — `a_purchase_resolves_exactly_once` now at tests/purchases.rs:296 (cited :295), `a_dropped_purchase_call_cancels_the_store_request` at :594 (cited :573); both asserts intact, drop-cancels asserts `cancels()==1` (:601-605); TS second-`finishPurchase`-false at purchases.test.tsx:54. Citation drift only.
2. **E13** — timeout test unchanged at tests/purchases.rs:146-168: `ERR_PURCHASES_TIMEOUT` + gate recovery asserted.
3. **E12** — tests/apple.rs:220-246 unchanged: raw 2 → `Cancelled`, pending state 0 → `Ok`/`Pending`.
4. **E17** — tests/android.rs:205-260 unchanged: consume reaches `consume` (:221-225), consumable never acknowledged (:226-229), finish acknowledges (:241-245), non-consumable never reaches consume (:252-258).
5. **E28** — `STOREKIT_MAP` still 8 rows at tests/apple.rs:28-37 (incl. `(3, "ERR_PURCHASES_PAYMENT_INVALID")`); test :170-190 asserts code and raw-as-status for every arm.
6. **E29** — tests/apple.rs:193-217: 99 and -1 → `Backend` + status kept.
7. **E30/E31** — `BILLING_MAP` still 12 rows at tests/android.rs:25-41 and contains **no** PaymentInvalid entry — exactly what the fixed P6-12 row now says; tests :158, :177 unchanged.
8. **E26** — `an_unsupported_target_is_not_available` now at :550 (cited :535); asserts unavailable + code (:554-556).
9. **R5 (README Checks)** — README.md:44-51 matches current ci.yml exactly: fmt (:36), clippy `--all-targets --all-features` + tests on the 3-OS matrix (:45, :63-64), the four cross-targets (:74-77), `nsh check` + `nsh build` failed on any warn (`! grep -i warn`, :118-119, :125-126) + `nsh test` for package and demo (:114-127), nine demo targets (:141-149), latest-release replace (:231-240), Pages (:242-274).
10. **R4 (README Layout)** — README.md:34-40 lists crates/, packages/, examples/demo/, docs/, .zcode/ — all exist; `site/` gone and no longer mentioned.
11. **R1 (Pages URL)** — README.md:5 URL unchanged; the deploy exists, now as ci.yml's `pages` job. (The original row's `pages.yml` citation is itself now stale — the job moved into ci.yml — but the claim holds.)
12. **parity.md intro test-file list** — parity.md:12 now names all four Rust test files incl. `entitlements.rs` and all three TS files incl. `entitlements.test.tsx`. Accurate.
13. **A2 (goals acceptance)** — goals.md:96-101's command list is a subset of what ci.yml now runs (which adds cross-targets, the build gate, demo). Holds.

Citation drift, for the record: `bc9d1b4` added asserts to tests/purchases.rs, so every test after `restore_returns_purchased_and_restored_purchases` moved — rows above cite old lines for E16 (:468 now), E22 (:415), E23 (:459), E24 (:485), E25 (:515), E26 (:550), E27 (:594), E32 (:610). Substantively unchanged; apple.rs and android.rs line numbers are untouched.

### NSH-version table re-check

- Cargo.toml:15 `nsh-core = "0.5.1"` — keep; dev-dependencies for the hook tests added `nsh-engine`/`nsh-testing` 0.5.1 (:20-21); Cargo.lock resolves 0.5.1 from the registry (:1088-1092).
- decisions.md:14-16 — the "brief pinned 0.4.0" nit is fixed: it now attributes the pin to `.zcode/purchases-run.ts`. Keep.
- entitlements.ts — resolved (above).
- ci.yml `NSH_VERSION: v0.5.1` — keep; now three mentions (:109, :175, :184), the table's `:43` citation has drifted.
- purchases-run-report.md:33 — resolved by the Corrections section.
- purchases-run.ts 0.4.0 pins — historical, no action, unchanged.

### Correction to "Gaps versus the reference-repo standard"

That section compared against the stale clone in `.refs/biometrics`, which predates the publish/Pages jobs. The live references — `~/fo/nsh-biometrics` (HEAD `170ef18`, 2026-11-17) and `~/fo/nsh-icons` (HEAD `2be7b9f`, 2026-11-17) — both run the full pipeline: rust OS matrix, cross-target checks, TS check/build-with-zero-warnings/test, nine-target demo builds, a replace-`latest` release job, and a Pages deploy. Every gap that section listed (cross-targets, OS matrix, `nsh build` warning gate, demo builds, compiler caching, publish) is now closed in this repo's ci.yml; on Pages this repo matches them.

### Corrected summary

Of the original 175 rows, 174 are now Done and one — A4, the fail-first process claim — remains Partial, attested by commit messages and the corrected run report but unverifiable from the tree by nature. Test counts in the original audit's key-files line and ZR3/ZR4 are superseded: 44 Rust tests (4 lib, 26 purchases, 5 entitlements, 5 apple, 4 android) and 30 package TS tests (20 purchases, 5 errors, 5 entitlements), plus 6 demo tests. All eleven local gates pass with exit 0 at `ff620ab`.

### CI at NSH 0.6.0, 2026-11-18

Run https://github.com/SuperNSH/purchases/actions/runs/44378069680 (commit `f729cfc`, "build: move to NSH 0.6.0") is green: format, rust on ubuntu, macos and windows, the four cross-target checks, typescript (package and demo: check, warning-free build, test), all nine demo builds, and the publish and Pages jobs. The macOS rust job was cancelled once at the 60-minute limit and passed on `gh run rerun --failed`. A4 (the red-first process claim) stays Partial. No store interaction ran anywhere; device tests stay blocked on an NSH host registering `StoreKitHost`/`BillingHost`, and real App Store or Play accounts are out of scope.
