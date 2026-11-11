# nsh-purchases build report

Seven areas ran as independent chains (implement, then audit→fix rounds capped at 3,
until zero findings), followed by the five integrated gates run once over the whole
tree. No commits or pushes were made; the changed files in the checkout are the
deliverable.

## Areas

- **Goals docs** — done (1 audit round, clean): `docs/goals.md` with goals, acceptance
  criteria, the 13-code error enum and the edge-case→test map; `README.md`
  Status/Layout updated.
- **Parity docs** — done (2 rounds): `docs/parity.md`, 105 rows covering every public
  API of the Flutter in_app_purchase facade/platform packages and the OpenIAP
  react-native-iap index, each row naming our Rust + TypeScript API, an honest status
  and the covering test.
- **Rust core** — done (1 round, clean): workspace + `crates/nsh-purchases` (error,
  model, options, `Backend` trait seam, `FakeBackend` with sandbox fixtures,
  gate-guarded `Purchases` facade, `use_entitlements`, `NativePurchases` native module
  with 4 unit tests in `src/native.rs`, platform dispatch, unsupported backend).
- **Apple bridge** — done (2 rounds): `src/apple.rs` — StoreKit 2 host-callback seam,
  SKError/SK2 mapping table verified against the Flutter reference wrappers.
- **Android bridge** — done (3 rounds, the full cap; round 3 was clean):
  `src/android.rs` — Play Billing host-callback seam, BillingResponse mapping table,
  PENDING/PURCHASED states (no invented Restored), finish-as-acknowledge, consume
  kind gate.
- **TypeScript package** — done (2 rounds): `packages/nsh-purchases` (13 files)
  mirroring the Rust API name-for-name: same methods, same 13 codes, same gate
  semantics, `memoryBackend` with the fake's fixtures, `rustBackend`, `useEntitlements`.
- **Tests and CI** — done (2 rounds): 26 canonical `FakeBackend` tests, 5 Apple + 4
  Android mock-host mapping tests (the 5 and the 4 include the two coverage additions
  routed from the bridge audits), 19 + 5 = 24 TS tests, and
  `.github/workflows/ci.yml` gained a typescript job (NSH v0.4.0 installer,
  `nsh check`, `nsh test`) with the rust job untouched.

Every area converged to zero audit findings within the 3-round cap. Audit findings
total 18 (7 medium, 11 low), all resolved. Two of the mediums were coverage gaps in
`tests/` files owned by the Tests area and were routed there (Apple consume/malformed
arms; Android unknown purchase states) rather than fixed by the bridge areas. The
substantive mediums: the TS package invented an `AlreadyOwned` gate, skipped productId
validation and diverged on entitlements/finish/consume/restore semantics versus the
Rust fake (8 findings, all fixed); the Parity intro claimed two legacy facade names
exist in neither clone while the archived mirror exports them as deprecated aliases
(reworded); the TS entitlements test never asserted the exclusion its name promises
(now an exact-list assertion).

## Integrated gates (whole tree)

First run: `cargo fmt` passed; `cargo clippy` failed with 7 compile errors
(cross-area lifetime seams in the bridges, one moved-value borrow in the fake).
`cargo test`, `nsh check` and `nsh test` were first run after one bounded gate-fixer
round repaired those errors. Final state, re-verified by an independent orchestrator
rerun:

- pass: `cargo fmt --all -- --check`
- pass: `cargo clippy --all-targets -- -D warnings`
- pass: `cargo test --all-targets` — 39 tests: 4 lib (native.rs unit tests), 26
  purchases, 5 apple, 4 android
- pass: `nsh check` (packages/nsh-purchases)
- pass: `nsh test` (packages/nsh-purchases) — 24/24

The gate-fixer repairs (owning borrowed args before async blocks, `+ Send` on
`BackendFuture`, receipt built before the struct literal moves the id, `Clone` on
`BillingPurchase`, an `Arc<dyn Backend>` coercion) were verified by clippy and the
full 39+24 test suites, not by a fresh persona audit; they are mechanical
lifetime/ownership fixes with no intended behaviour change, and every frozen
canonical test still passes.

## Notes

- Local runs: AGENTS.md wants every compile and test on GitHub Actions, but this
  brief forbids commits and pushes, so the gates ran locally in this checkout, on
  mocks and sandbox fixtures only. `.github/workflows/ci.yml` now runs the same
  gates in CI once pushed. No CI run has executed — nothing has been pushed.
- README's Status section says "Implemented against mocks" and names what exists; it
  was edited at the join so it no longer implies CI has already run the gates.
- No real store accounts, products or purchases: every store interaction is
  `FakeBackend`, a mock host, or the memory backend. The real StoreKit 2 /
  BillingClient glue lives in the NSH hosts (outside this repo); the bridges here are
  the host-callback seam with its mapping fully tested.
- Platforms in scope: Apple and Android. Windows, Linux and web answer
  `ERR_PURCHASES_NOT_AVAILABLE` (see docs/decisions.md).
- `docs/decisions.md` was created and edited by the run's pre-workflow STEP 0
  (reference cloning, canonical-repo substitution, platform and seam decisions); no
  implementation area owns it.
- react-native-iap is archived; its canonical continuation (hyodotdev/openiap,
  libraries/react-native-iap) was cloned alongside and recorded in docs/decisions.md.
  References are cloned read-only under `.refs/` (git-ignored).
- Parity table mechanics: many rows cite the same canonical test (many-to-one by
  design); rows for reference APIs we deliberately accept-but-ignore or do not
  platform-support carry Accepted / Platform-only statuses without a test. The 32
  canonical test names in docs/goals.md are the 26 FakeBackend tests plus the 6
  bridge mapping tests; all 32 exist and pass.
- Join decision: `BackendFuture` is `+ Send` (the biometrics reference alias is not)
  because the test harness spawns `Purchases::purchase` via `tokio::spawn`; both
  platforms' host futures were already Send. No contract conflict.
- Known accepted deviations in tests: `#[test]` + `run()` helper instead of
  `#[tokio::test]` (dev-deps mirror biometrics and lack the macros feature);
  `entitlements_respect_expiry` stages the lapsed subscription via `set_entitlements`
  rather than a recomputed expiry.
- Workflow honesty: a workflow-launcher agent did all pre-workflow planning (STEP 0,
  area split, API signatures, canonical test names) and wrote a dynamic-workflow
  script; this session's harness exposed no engine to submit it to, so the
  orchestrator executed that script's exact logic — areas, personas, audit rounds,
  join gates, reader-checked report — via parallel subagents on GLM-5.3 (high). The
  one structural deviation: audit dispatch was batched per area instead of fully
  event-driven; no area's phase ever waited on another area's. One audit call (Parity
  docs, round 1) died on an API rate limit mid-audit and was resumed with its context
  intact; its 3-finding verdict came after the resume, then the fix landed and round
  2 was clean.
- Status: the crate and package build and their suites pass locally under the gates
  the brief names. CI has NOT executed them yet (no commits/pushes, per the brief).
