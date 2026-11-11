# nsh-purchases

Products, purchases, restores and entitlements, without a backend.

Part of the Native Shell (NSH) capability set. Same layout as the other SuperNSH repos: a Rust crate, a TypeScript package, docs in `docs/`, CI on GitHub Actions, and a web status page on GitHub Pages at https://supernsh.github.io/purchases/.

## Status

Implemented against mocks. The Rust crate in `crates/nsh-purchases` and the TypeScript package in `packages/nsh-purchases` carry the facade, the stable error codes, the StoreKit and Billing bridges and their tests; `docs/goals.md` is the contract and [docs/parity.md](docs/parity.md) maps each reference API to ours; the gates pass locally and CI runs them on push (no CI run yet — nothing has been pushed). Every store interaction is a mock or a sandbox fixture — nothing here has touched a real store account, product or purchase.

## Approach

Native StoreKit 2 and Play Billing behind one facade. The Apple and Android bridges declare host callbacks (`apple::StoreKitHost`, `android::BillingHost`) and map raw store codes to the stable `ERR_PURCHASES_*` codes; the store glue itself lives in the NSH hosts. Receipt validation stays in the app — the trade-off is in `docs/goals.md`.

## Layout

- `crates/nsh-purchases/`: the Rust crate — facade, error type, StoreKit and Billing bridges, tests.
- `packages/nsh-purchases/`: the TypeScript package over the native module, with its tests.
- `docs/`: `goals.md` (goals, error codes, edge cases, acceptance), `decisions.md`, `parity.md`, and the research `brief.md`.
- `site/`: static status page, published by `.github/workflows/pages.yml`.
- `.zcode/`: the workflow phases this repo runs through.

## Checks

Nothing is built or tested on a developer's machine. `.github/workflows/ci.yml` runs `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and `nsh check` with `nsh test` in `packages/nsh-purchases`. Read the results with `gh run`.

## Licence

MIT. See [LICENSE](LICENSE).

Repository: https://github.com/SuperNSH/purchases
