# In-app purchases: brief

Products, purchases, restores and entitlements, without a backend.

## Options

- StoreKit 2 (iOS 15+): native
- Play Billing: native, Android
- RevenueCat: hosted, only if a hosted service is acceptable
- No Rust library found

## Recommended path

Native StoreKit 2 and Play Billing behind one NSH trait. Receipt validation in the app.

## Edge cases to design for

- Sandbox and production use different receipts and keys
- Restore flow is required by App Review
- Android pending purchases must be acknowledged
- Receipt validation without a backend is limited; document the trade-off

## Research status

Research done 2026-11-09 by web search. Versions and licences are unverified until checked on the registry before pinning. Full catalogue: the research notes for the SuperNSH library set.
