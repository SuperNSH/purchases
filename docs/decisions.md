# Decisions

Each entry says what was chosen and why. `docs/goals.md` lists the goals, the error codes and the edge cases; `docs/parity.md` lists what each reference API does here.

## The reference repositories

The brief names four repos to clone into `.refs/` (read-only, git-ignored). Three cloned as named. The fourth, `hyochan/react-native-iap`, is archived and read-only: development moved to the OpenIAP monorepo, so its canonical source is `hyodotdev/openiap` (`libraries/react-native-iap`), which was cloned alongside the archive. All four sources are read from `.refs/`.

- `flutter/packages` — sparse clone of `packages/in_app_purchase` with its platform packages.
- `hyochan/react-native-iap` — the archived repo, with `hyodotdev/openiap` as the canonical continuation.
- `longbridge/nsh` — Native Shell main: the Rust crates and TSX typings this library wraps.
- `SuperNSH/biometrics` — the org layout, API naming and error-code style this repo follows.

## NSH 0.5.0

The brief pinned NSH 0.4.0; the pin moved to 0.5.0 on 2026-11-12 with no source change. `nsh-core` 0.5.0 is on crates.io and CI installs the 0.5.0 CLI, so the crate builds against the released version; no unreleased `main` feature is required.

## Native StoreKit 2 and Play Billing, no hosted service

The brief's recommended path. RevenueCat is rejected: it is a hosted service and this repo's premise is purchases without a backend.

## The store talks through host callbacks

The Apple and Android bridges do not link a store SDK. Each declares the callbacks a native shell host registers (`apple::StoreKitHost`, `android::BillingHost`), maps the store's raw codes to the stable `ERR_PURCHASES_*` codes, and never panics or hangs. The real StoreKit 2 and BillingClient glue lives in the NSH hosts, outside this repo.

Why: everything here is tested against mocks and sandbox fixtures, and a bridge that only compiles on its own OS is code no run in this repo can verify. With the seam, every line compiles on every runner and every mapping has a test.

Consequence: a target reports `ERR_PURCHASES_NOT_AVAILABLE` until its host registers callbacks at startup.

## Platforms in scope for the first release

Apple (iOS and macOS) and Android, the two platforms both references implement. Windows, Linux and the web get the unsupported backend: neither reference offers store purchases there, so the honest answer is `ERR_PURCHASES_NOT_AVAILABLE`, not an empty stub.

## Cancels are errors, pending is not

A cancel resolves the call with `ERR_PURCHASES_CANCELLED`: the user chose to stop, and the call still resolves exactly once. Pending is the exception — a purchase awaiting approval resolves `Ok` with a pending state because nothing failed, and `pending()` surfaces unfinished purchases later.
