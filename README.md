# nsh-purchases

Products, purchases, restores and entitlements, without a backend.

Part of the Native Shell (NSH) capability set. Same layout as the other SuperNSH repos: a Rust crate, a TypeScript package, docs in `docs/`, CI on GitHub Actions, and a web demo on GitHub Pages at https://supernsh.github.io/purchases/.

## Install

In the app's directory:

```sh
nsh add nsh-purchases
```

`nsh add` needs NSH main or the nightly: the 0.6.0 and 0.6.1 releases do not have it. In a Rust app, `cargo add nsh-purchases` works with any of them.

The TSX imports are `nsh-purchases`, `nsh-purchases/rust` and `nsh-purchases/memory`.

## Status

Implemented against mocks. The Rust crate in `crates/nsh-purchases` and the TypeScript package in `packages/nsh-purchases` carry the facade, the stable error codes, the StoreKit and Billing bridges and their tests; `docs/goals.md` is the contract and [docs/parity.md](docs/parity.md) maps each reference API to ours; the gates pass and CI runs them on every push to `master`. Every store interaction is a mock or a sandbox fixture — nothing here has touched a real store account, product or purchase. The demo in `examples/demo` exercises the same API on the in-memory fake store ([Try the demo](#try-the-demo)).

## Try the demo

The demo in `examples/demo` shows the whole API on the in-memory fake store — no real store, no real money: the three fixture products, one of each kind; a purchase sheet you answer as a store would (approve, approve as pending, cancel, decline, already owned, or cancel from the app); the pending queue with finish and consume; restore; and entitlements with the subscription's expiry. Open it on the web at https://supernsh.github.io/purchases/, or download a build of the latest `master`:

| Platform | Download |
|---|---|
| macOS (Apple silicon) | [purchases-demo-macos-arm64.zip](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-macos-arm64.zip) |
| macOS (Intel) | [purchases-demo-macos-x86_64.zip](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-macos-x86_64.zip) |
| Linux x86_64 | [purchases-demo-linux-x86_64.tar.gz](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-linux-x86_64.tar.gz) |
| Linux arm64 | [purchases-demo-linux-arm64.tar.gz](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-linux-arm64.tar.gz) |
| Windows x86_64 | [purchases-demo-windows-x86_64.zip](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-windows-x86_64.zip) |
| Windows arm64 | [purchases-demo-windows-arm64.zip](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-windows-arm64.zip) |
| iOS simulator | [purchases-demo-ios-simulator.zip](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-ios-simulator.zip) |
| Android (arm64) | [purchases-demo-android.apk](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-android.apk) |
| Web (static site) | [purchases-demo-web.tar.gz](https://github.com/SuperNSH/purchases/releases/latest/download/purchases-demo-web.tar.gz) |

- **macOS:** the app is signed ad hoc, not notarized, and needs macOS 14 or later. macOS quarantines what a browser downloads, so clear that before opening it: `xattr -dr com.apple.quarantine <the unzipped .app>`.
- **iOS simulator:** an arm64 simulator build, so it needs Xcode with an iOS simulator runtime on an Apple silicon Mac. Unzip it and install it on a booted simulator with `xcrun simctl install booted <the unzipped .app>`, or drag the `.app` onto the simulator window.

## Approach

Native StoreKit 2 and Play Billing behind one facade. The Apple and Android bridges declare host callbacks (`apple::StoreKitHost`, `android::BillingHost`) and map raw store codes to the stable `ERR_PURCHASES_*` codes; the store glue itself lives in the NSH hosts. Receipt validation stays in the app — the trade-off is in `docs/goals.md`.

## Layout

- `crates/nsh-purchases/`: the Rust crate — facade, error type, StoreKit and Billing bridges, tests.
- `packages/nsh-purchases/`: the TypeScript package over the native module, with its tests.
- `examples/demo/`: the demo app — the whole API on the in-memory fake store.
- `docs/`: `goals.md` (goals, error codes, edge cases, acceptance), `decisions.md`, `parity.md`, and the research `brief.md`.
- `.zcode/`: the workflow phases this repo runs through.

## Checks

Nothing is built or tested on a developer's machine. `.github/workflows/ci.yml` runs on every push to `master` and on pull requests, on GitHub's runners:

- `cargo fmt --all -- --check`.
- `cargo clippy -p nsh-purchases --all-targets --all-features -- -D warnings`, then the test suite, on Linux, macOS and Windows.
- Clippy for the targets beyond the desktop this library ships for: `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-linux-android` (through the NDK) and `wasm32-unknown-unknown`.
- `nsh check`, `nsh build` failing on any warning, and `nsh test`, in `packages/nsh-purchases` and `examples/demo`.

On pushes to `master` only, the same workflow also builds the demo for nine targets (the downloads above), replaces the `latest` release with those nine builds, and deploys the web demo to GitHub Pages. Read the results with `gh run`.

## Licence

MIT. See [LICENSE](LICENSE).

Repository: https://github.com/SuperNSH/purchases
