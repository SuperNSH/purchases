// Dynamic workflow: build nsh-purchases (Rust crate + TypeScript package) in
// /Users/hmziq/fo/nsh-purchases, laid out like SuperNSH/biometrics, per the
// user's verbatim brief. Seven areas run their own chains in parallel
// (implement, then audit/fix until zero issues, capped at 3 rounds), then the
// whole-tree gates the brief names run once at the join. Every subagent is
// pinned to GLM-5.3 (the user asked for GLM-5.3 Max for every area and audit
// round); no vision interpreters exist in this run. No git machinery, no
// commits, no pushes: the deliverable is the changed files in the checkout.
// STEP 0 (clone references into .refs/, docs/decisions.md entries) already ran
// before this script; .refs/ is a read-only input baked into the asks.

const MODELS = {
  // GLM-5.3 main tier, HIGH thinking — the user's explicit pin for every area
  // and every audit round. No vision phase exists, so GLM-5.3-Flash is unused.
  pro: model("account:zai-individual-coding-plan/GLM-5.3$high"),
};
const KIT = "/Users/hmziq/ZCodeProject/ai/harness/zcode/agents";

const AUDIT_CAP = 3;
const GATE_FIX_CAP = 3;

async function specialist(name: string, fallback: string): Promise<{ persona: string; source: string }> {
  const adapter =
    `You are the ${name} specialist for one area of a parallel workflow. ` +
    `Your full specialist contract follows verbatim — apply it to this area's work; ` +
    `where it conflicts with your ask, the ask wins.` + "\n\n";
  for (const p of [`.zcode/agents/${name}.md`, `${KIT}/${name}.md`]) {
    try {
      const raw = await files.read(p);
      return { persona: adapter + raw.replace(/^---\n[\s\S]*?\n---\n/, "").trim(), source: p };
    } catch { /* try next source */ }
  }
  return { persona: fallback, source: "fallback" };
}

const HONESTY =
  "If a check is impossible to pass, or your instructions contradict each other, " +
  "escalate and say so plainly rather than working around it.";

const COMMON = [
  "Repository: /Users/hmziq/fo/nsh-purchases (the ONLY repo you may touch). Work directly in the current checkout:",
  "no branches, no worktrees, no commits, no pushes, no stash — the deliverable is the changed files.",
  "",
  "Hard rules from AGENTS.md (they override habits):",
  "- Comments are 1-2 lines and state a non-obvious constraint. Docs read direct and concrete, no marketing filler.",
  "- Dead code is deleted; no speculative abstractions.",
  "- Every behaviour needs a test; a test is seen to fail once before it counts (the run's tests area covers this).",
  "- An operation resolves exactly once: success, a coded failure, or a cancel. It never hangs.",
  "- Never panic: map failures to coded errors. Never block forever.",
  "",
  "No real store accounts, products or purchases: every store interaction is a mock or sandbox fixture",
  "(FakeBackend, mock hosts, the memory backend). Never build, run or test emulators or simulators.",
  "",
  "Do NOT run cargo, nsh, or any whole-tree command: the script runs the integrated gates once, after every",
  "area lands (a mid-run compile would read other areas' half-written files). A Rust area may run",
  "`rustfmt --edition 2024 --check <its own files>` as a scoped self-check; nothing else.",
  "",
  "The layout mirrors .refs/biometrics (SuperNSH/biometrics, read-only reference clone):",
  "- Rust crate: crates/nsh-purchases (workspace root Cargo.toml, members = [\"crates/nsh-purchases\"]).",
  "- TypeScript package: packages/nsh-purchases.",
  "- Docs: docs/goals.md, docs/parity.md, docs/decisions.md.",
  "",
  "Stable error codes (final, canonical — ErrorKind::code() strings, TS union members, and docs must use exactly these):",
  "  NotAvailable     ERR_PURCHASES_NOT_AVAILABLE      no store on this target, or no host registered",
  "  NotInitialized   ERR_PURCHASES_NOT_INITIALIZED    a store call before initialize()",
  "  InProgress       ERR_PURCHASES_IN_PROGRESS        another purchase is already running",
  "  Timeout          ERR_PURCHASES_TIMEOUT            the opt-in timeout passed unanswered",
  "  Cancelled        ERR_PURCHASES_CANCELLED          user, system or app cancelled",
  "  ProductNotFound  ERR_PURCHASES_PRODUCT_NOT_FOUND  unknown product id, or the store has no such item",
  "  AlreadyOwned     ERR_PURCHASES_ALREADY_OWNED      the non-consumable or active subscription is already owned",
  "  NotOwned         ERR_PURCHASES_NOT_OWNED          finish/consume of an unknown or already finished purchase",
  "  PaymentInvalid   ERR_PURCHASES_PAYMENT_INVALID    the store rejected the payment itself",
  "  NotAllowed       ERR_PURCHASES_NOT_ALLOWED        purchase not allowed (parental controls, revoked cloud rights)",
  "  Network          ERR_PURCHASES_NETWORK            the store connection failed",
  "  InvalidArgument  ERR_PURCHASES_INVALID_ARGUMENT   empty ids, zero quantity, zero timeout, non-consumable consume",
  "  Backend          ERR_PURCHASES_BACKEND            the platform or host failed; carries its raw status where given",
  "",
  "Shared types (same names in Rust and TypeScript): Product { id, title, description, price (display string),",
  "currency_code/currencyCode (optional), kind: Consumable|NonConsumable|Subscription }; Purchase { id, product_id/productId,",
  "state: Pending|Purchased|Restored, receipt (store verification payload string), purchased_at_ms/purchasedAtMs (optional) };",
  "Entitlement { product_id/productId, active, expires_at_ms/expiresAtMs (optional) }.",
  "",
  "The reference clones (read-only inputs, read freely):",
  "- .refs/flutter-packages/packages/in_app_purchase/ — Flutter plugin (facade in_app_purchase.dart, types in",
  "  in_app_purchase_platform_interface, StoreKit 2 wrappers in in_app_purchase_storekit, BillingClient wrappers in",
  "  in_app_purchase_android).",
  "- .refs/openiap/libraries/react-native-iap/ — the canonical react-native-iap source (hyochan/react-native-iap is",
  "  the archived mirror in .refs/react-native-iap/).",
  "- .refs/nsh/ — Native Shell main (longbridge/nsh): the nsh-core crate and TSX typings this library wraps.",
  "- .refs/biometrics/ — the org template whose layout, naming, error style and CI this repo follows.",
  "",
  "Every nsh-core macro, class and method name must be copied from how .refs/biometrics uses nsh-core 0.4.0",
  "(its sources pin the same version) — never invent an API, and never guess a name the references do not show.",
].join("\n");

interface AuditFinding {
  /** Workspace-relative path the problem is in. */
  file: string;
  /** One sentence: what is wrong. */
  problem: string;
  /** One sentence: what the fix should be. */
  fix: string;
  /** How much it matters. Reserve "high" for a broken rule of the brief or a wrong API. */
  severity: "low" | "medium" | "high";
}

interface Audit {
  /** Actionable findings only. An empty array means the area is clean. */
  findings: AuditFinding[];
  /** One sentence copied VERBATIM from your specialist instructions that most shaped this audit. */
  contractLine: string;
}

interface ImplementResult {
  /** True when everything in the ask was written. False means blocked or refused. */
  done: boolean;
  /** Two or three sentences: what landed. */
  summary: string;
  /** Anything that would need a decision or another area's file; empty when none. */
  blockers?: string[];
  /** One sentence copied VERBATIM from your specialist instructions that most shaped this work. */
  contractLine?: string;
}

interface FixResult {
  /** True when every finding was addressed or explained in blockers. */
  done: boolean;
  /** Two or three sentences: what changed. */
  summary: string;
  /** Findings that cannot be fixed inside this area's own files, with the reason. */
  blockers?: string[];
}

interface GateOutcome {
  name: string;
  ok: boolean;
  output: string;
}

interface AreaOutcome {
  name: string;
  status: "done" | "failed";
  summary: string;
  rounds: number;
  converged: boolean;
}

interface ReaderProblems {
  /** What is unclear, unsupported by the run's facts, or missing — empty when the report reads clean. */
  problems: string[];
}

function areaFileFor(name: string): string {
  switch (name) {
    case "Goals docs":
      return "docs/goals.md";
    case "Parity docs":
      return "docs/parity.md";
    case "Rust core":
      return "crates/nsh-purchases/src/lib.rs";
    case "Apple bridge":
      return "crates/nsh-purchases/src/apple.rs";
    case "Android bridge":
      return "crates/nsh-purchases/src/android.rs";
    case "TypeScript package":
      return "packages/nsh-purchases/src/index.ts";
    case "Tests and CI":
      return "crates/nsh-purchases/tests/purchases.rs";
    default:
      return "unknown";
  }
}

interface RevisedReport {
  /** The full revised markdown, all reader problems addressed. */
  markdown: string;
}

artifact.board("areas", {
  title: "nsh-purchases areas",
  key: "name",
  status: "phase",
  columns: ["implementing", "auditing", "testing", "fixing", "done", "failed"],
});

// ---------------------------------------------------------------------------
// The areas. Every ask below carries the complete predetermined plan: files,
// conventions, exact API signatures, and the rules. Nothing is decided mid-run.
// ---------------------------------------------------------------------------

const AREAS: {
  key: string;
  name: string;
  implPersona: string;
  implFallback: string;
  auditPersona: string;
  auditFallback: string;
  owns: string;
  ask: string;
}[] = [
  {
    key: "goals",
    name: "Goals docs",
    implPersona: "z-nsh-engineer",
    implFallback: "You are a careful NSH ecosystem docs engineer.",
    auditPersona: "z-nsh-engineer",
    auditFallback: "You are a careful NSH ecosystem docs reviewer.",
    owns: "docs/goals.md, README.md",
    ask: [
      "Write docs/goals.md for the nsh-purchases library: goals, acceptance criteria, the error enum with its stable",
      "codes (use the canonical codes from the shared block above, verbatim), and an edge-case list where each case",
      "maps to the named test that covers it. Cover everything the Flutter and React Native references handle",
      "(querying products, buying consumables and non-consumables and subscriptions, pending and cancelled and",
      "failed purchases, finishing/acknowledging/consuming, restoring, entitlements, storefront country, receipts",
      "returned for app-side validation) plus the brief's edge cases: sandbox vs production receipts and keys,",
      "restore required by App Review, Android pending purchases needing acknowledgement, receipt validation",
      "without a backend being limited (document the trade-off).",
      "",
      "The named tests are canonical — the tests area writes exactly these names; the parity doc cites them.",
      "Rust names (crates/nsh-purchases/tests/, snake_case):",
      "  purchase_twice_in_parallel_is_in_progress; cancel_resolves_the_running_purchase_as_cancelled;",
      "  a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers; a_zero_timeout_is_an_invalid_argument;",
      "  calls_before_initialize_are_not_initialized; initialize_is_idempotent;",
      "  empty_product_ids_are_an_invalid_argument; a_zero_quantity_is_an_invalid_argument;",
      "  unknown_product_ids_are_dropped_from_the_answer; purchasing_an_unknown_product_is_product_not_found;",
      "  a_purchase_resolves_exactly_once; a_pending_purchase_resolves_with_a_pending_state;",
      "  a_cancelled_purchase_resolves_with_the_cancelled_code; an_already_owned_product_is_already_owned;",
      "  finishing_marks_the_purchase_finished_once; finishing_an_unknown_or_finished_purchase_is_not_owned;",
      "  consuming_a_non_consumable_is_an_invalid_argument; restore_returns_purchased_and_restored_purchases;",
      "  restore_with_no_purchases_is_empty; pending_returns_unfinished_purchases;",
      "  entitlements_exclude_consumed_consumables; entitlements_respect_expiry;",
      "  an_unsupported_target_is_not_available; concurrent_non_purchase_calls_do_not_claim_the_gate;",
      "  a_dropped_purchase_call_cancels_the_store_request; every_error_code_round_trips;",
      "  every_storekit_raw_code_maps_to_its_documented_code; unknown_storekit_raw_codes_are_backend;",
      "  a_storekit_cancel_and_a_pending_purchase_resolve_as_documented;",
      "  every_billing_raw_code_maps_to_its_documented_code; unknown_billing_raw_codes_are_backend;",
      "  finishing_acknowledges_and_consuming_consumes.",
      "TypeScript tests (packages/nsh-purchases/tests/, same phrases in human wording) mirror the facade and",
      "error-code behaviours; the raw-code mapping tests are Rust-only because the bridges are Rust.",
      "",
      "Acceptance criteria must name the gates exactly: cargo fmt --all -- --check; cargo clippy --all-targets --",
      "-D warnings; cargo test --all-targets; nsh check; nsh test (the last two in packages/nsh-purchases) — run in",
      "CI by .github/workflows/ci.yml, and every store interaction mocked or sandbox-fixture only.",
      "",
      "Then update README.md: the Status section no longer says scaffold (say what exists now: crate + package +",
      "docs + tests, per docs/parity.md), and the Layout section matches the real tree (crates/nsh-purchases,",
      "packages/nsh-purchases, docs/, site/, .zcode/). Keep it direct and short; no marketing filler.",
      "Do not touch docs/decisions.md (owned by this run's STEP 0) or docs/parity.md (another area owns it).",
    ].join("\n"),
  },
  {
    key: "parity",
    name: "Parity docs",
    implPersona: "z-nsh-engineer",
    implFallback: "You are a careful NSH ecosystem docs engineer.",
    auditPersona: "z-nsh-engineer",
    auditFallback: "You are a careful NSH ecosystem docs reviewer.",
    owns: "docs/parity.md",
    ask: [
      "Rewrite docs/parity.md in the format of .refs/biometrics/docs/parity.md: one table row per public API of BOTH",
      "references, with columns: the reference name | Rust | TypeScript | Status | Covered by (the named test).",
      "Use the same status words as biometrics (Done, Partial, Accepted, Platform only) and define them at the top.",
      "",
      "Read the full public surfaces from the reference clones:",
      "- Flutter: .refs/flutter-packages/packages/in_app_purchase/in_app_purchase/lib/in_app_purchase.dart",
      "  (InAppPurchase.instance, purchaseStream, isAvailable, queryProductDetails, buyNonConsumable, buyConsumable,",
      "  completePurchase, restorePurchases, countryCode) and the platform-interface types (IAPError,",
      "  InAppPurchaseException, ProductDetails, ProductDetailsResponse, PurchaseDetails, PurchaseParam,",
      "  PurchaseStatus, PurchaseVerificationData), plus the StoreKit 2 and BillingClient wrappers for the platform",
      "  detail rows.",
      "- React Native: .refs/openiap/libraries/react-native-iap/src/index.ts and types.ts (initConnection, endConnection,",
      "  fetchProducts/getProducts/getSubscriptions, requestPurchase, requestSubscription, purchaseUpdatedListener,",
      "  purchaseErrorListener, finishTransaction, acknowledgePurchase, consumePurchase, getAvailablePurchases,",
      "  getPurchaseHistories, validateReceipt, deepLinkToSubscriptions, getStorefront, useIAP and whatever else the",
      "  index actually exports — enumerate from the source, not from memory).",
      "",
      "For each row say what ours does: the Rust name in crates/nsh-purchases (Purchases facade: is_available,",
      "initialize, products, purchase, finish, consume, restore, pending, entitlements, country_code, cancel; types",
      "Product, Purchase, Entitlement, PurchaseOptions; errors ErrorKind/PurchasesError with the canonical",
      "ERR_PURCHASES_* codes) and the TypeScript name in packages/nsh-purchases (Purchases class with the same",
      "camelCase methods, memoryBackend, rustBackend, useEntitlements, PurchasesError with code).",
      "Be explicit where our design differs and why: our purchase() resolves exactly once instead of a broadcast",
      "purchaseStream (pending completions surface through pending()); restorePurchases delivers through our return",
      "value; receipt validation is app-side (we return the receipt, we do not verify); the web/Windows/Linux rows",
      "are Platform only with ERR_PURCHASES_NOT_AVAILABLE. Rows for accepted-but-ignored reference options (for",
      "example applicationUserName on non-Apple stores, quantity where a store caps it) say Accepted.",
      "",
      "The Covered by column cites the canonical named tests (listed in the shared plan of this run; the goals doc",
      "owns the canonical list) — use exactly those names. Rust test names live in crates/nsh-purchases/tests/,",
      "TypeScript tests in packages/nsh-purchases/tests/. Honest statuses only: a row is Done only where the named",
      "test will run in CI. Read-only reference clones; do not edit them.",
    ].join("\n"),
  },
  {
    key: "core",
    name: "Rust core",
    implPersona: "z-rust-engineer",
    implFallback: "You are a careful Rust library engineer.",
    auditPersona: "z-rust-engineer",
    auditFallback: "You are a careful Rust library reviewer.",
    owns: "Cargo.toml (workspace root), crates/nsh-purchases/Cargo.toml, crates/nsh-purchases/src/lib.rs, error.rs, model.rs, options.rs, backend.rs, fake.rs, purchases.rs, entitlements.rs, native.rs, platform/mod.rs, platform/unsupported.rs (and deletes the old root src/lib.rs)",
    ask: [
      "Build the Rust core of nsh-purchases behind a platform trait, mirroring .refs/biometrics/crates/nsh-biometrics",
      "file-for-file in style. Steps in this order: workspace manifest, error, model, options, backend, fake,",
      "platform, facade, entitlements, native, lib.",
      "",
      "1. Replace the root Cargo.toml with a workspace manifest exactly like .refs/biometrics/Cargo.toml:",
      "   [workspace] members = [\"crates/nsh-purchases\"], resolver = \"3\", workspace.package with version 0.1.0,",
      "   edition 2024, license MIT, repository https://github.com/SuperNSH/purchases, authors [\"hmziqrs\"].",
      "   Delete the old root src/ directory (the stub moves into the crate).",
      "2. crates/nsh-purchases/Cargo.toml mirrors biometrics' crate manifest: [lib] name = \"nsh_purchases\";",
      "   dependencies nsh-core = \"0.4.0\", thiserror = \"2.0.21\", tokio 1.54.1 default-features=false features",
      "   [\"sync\",\"time\"]; dev-dependencies tokio 1.54.1 default-features=false features [\"rt\",\"time\",\"test-util\"].",
      "3. src/error.rs copies .refs/biometrics/.../error.rs structure exactly: #[non_exhaustive] enum ErrorKind",
      "   (NotAvailable, NotInitialized, InProgress, Timeout, Cancelled, ProductNotFound, AlreadyOwned, NotOwned,",
      "   PaymentInvalid, NotAllowed, Network, InvalidArgument, Backend), each with code() -> &'static str returning",
      "   the canonical ERR_PURCHASES_* string and message() -> &'static str with one-sentence advice; struct",
      "   PurchasesError { kind, status: Option<i64> } with thiserror #[error(\"{}\", .kind.message())], new(kind),",
      "   with_status(status), kind(), status(), code(); Display for ErrorKind writes the code.",
      "4. src/model.rs: #[non_exhaustive] enums ProductKind { Consumable, NonConsumable, Subscription } and",
      "   PurchaseState { Pending, Purchased, Restored }; structs Product, Purchase, Entitlement per the shared",
      "   block (owned String fields, u64 ms timestamps, Option where the store may not give one).",
      "5. src/options.rs: PurchaseOptions { quantity: u32 (default 1), application_username: Option<String>,",
      "   timeout: Option<std::time::Duration> } with Default.",
      "6. src/backend.rs copies biometrics' backend.rs seam exactly (BackendFuture<'a, T> = Pin<Box<dyn Future",
      "   Output = T + 'a>>): trait Backend: Send + Sync with is_available() -> BackendFuture<'_, Result<bool,",
      "   PurchasesError>>, initialize() -> Result<()>, products(&self, ids: &[String]) -> Result<Vec<Product>>",
      "   (found products only; unknown ids silently dropped, as the references do), purchase(&self, product_id:",
      "   &str, options: &PurchaseOptions) -> Result<Purchase>, finish(&self, purchase: &Purchase) -> Result<()>",
      "   (Apple finish / Android acknowledge), consume(&self, purchase: &Purchase) -> Result<()> (consumables only),",
      "   restore() -> Result<Vec<Purchase>>, pending() -> Result<Vec<Purchase>> (unfinished queue / pending",
      "   acknowledgements), entitlements() -> Result<Vec<Entitlement>>, country_code() -> Result<String>, and",
      "   cancel() (ask the platform to dismiss; where it cannot, the answer still arrives).",
      "7. src/fake.rs mirrors biometrics' fake.rs: #[derive(Clone)] FakeBackend sharing Arc<Mutex<FakeState>>,",
      "   scripted answers per method, recorded counters (purchases, finishes, consumes, restores, cancels,",
      "   product queries) and last arguments, a resolvable in-flight purchase (the test answers it, as the store",
      "   would), and set_next_purchase_error. Default fixture products: pro_unlock (NonConsumable, 4.99 USD),",
      "   coins_100 (Consumable, 0.99 USD), pro_monthly (Subscription, 1.99 USD/month), each with a sandbox-fixture",
      "   receipt string. The fake enforces kind rules itself (consuming a non-consumable is InvalidArgument;",
      "   finish twice is NotOwned).",
      "8. src/platform/mod.rs: mod apple; mod android; mod unsupported; and default_backend() -> Arc<dyn Backend>",
      "   that returns apple::native_backend() then android::native_backend() then unsupported::NativeBackend,",
      "   whichever host has registered (see below). src/platform/unsupported.rs: NativeBackend where is_available",
      "   resolves Ok(false) and every other call resolves Err(NotAvailable), in the style of biometrics'",
      "   platform/unsupported.rs.",
      "   IMPORTANT: crates/nsh-purchases/src/apple.rs and src/android.rs are authored by two other areas IN",
      "   PARALLEL — do not create them. Their planned public API, which lib.rs may declare and platform/mod.rs may",
      "   call: each exposes `pub fn install_host(host: XHost)` registering a OnceLock, `pub fn native_backend() ->",
      "   Option<Arc<dyn Backend>>` returning Some only once a host is registered, plus its own XHost callback",
      "   struct and raw-code mapping. Code against exactly those names; the files appear during this run.",
      "9. src/purchases.rs: #[derive(Clone)] pub struct Purchases { backend: Arc<dyn Backend>, state: Arc<Gate> }",
      "   (clones share the gate, like biometrics' LocalAuth). new() uses platform::default_backend();",
      "   with_backend(Arc<dyn Backend>) is for tests. Methods: is_available, initialize (idempotent Ok; the facade",
      "   tracks initialized), products(&[String]), purchase(product_id: &str, options: PurchaseOptions),",
      "   finish(&Purchase), consume(&Purchase), restore(), pending(), entitlements(), country_code(), cancel().",
      "   Facade rules (biometrics' gate semantics): one purchase at a time — the gate is busy from call start",
      "   until the backend answers; a second purchase resolves InProgress at once without claiming; cancel()",
      "   resolves the running purchase as Cancelled immediately and a late answer is dropped (generation guard);",
      "   a dropped in-flight purchase calls backend.cancel(); options.timeout races the answer (tokio timer), the",
      "   timer wins resolving Timeout and force-frees the gate, and the backend's late answer is dropped; a zero",
      "   timeout, a zero quantity and an empty id list resolve InvalidArgument before the gate is claimed; every",
      "   store call except is_available resolves NotInitialized before initialize() succeeded; non-purchase calls",
      "   never claim the gate. Every method resolves exactly once and never hangs.",
      "10. src/entitlements.rs mirrors biometrics' availability.rs: use_entitlements(&Purchases) holding a Signal of",
      "    an EntitlementsState (checking / ready(Vec<Entitlement>) / failed(PurchasesError)), read at start and",
      "    again when lifecycle becomes active, comparing by equality so unchanged answers do not re-run bindings.",
      "11. src/native.rs mirrors biometrics' native.rs: #[derive(JsObject)] NativePurchaseOptions (quantity,",
      "    application_username, timeout_ms as Option fields) and plain JsObject results (NativeProduct,",
      "    NativePurchase, NativeEntitlement with camelCase fields); #[class] NativePurchases wrapping Purchases with",
      "    async members is_available, initialize, products, purchase, finish, consume, restore, pending,",
      "    entitlements, country_code, cancel that reject with a JsError whose message starts with the stable code",
      "    (copy biometrics' into_js mapping). lib.rs registers #[native_module(\"nsh-purchases/native\")].",
      "12. src/lib.rs: crate docs (two lines: what it is, that each call resolves exactly once with a coded error),",
      "    `use nsh_core::prelude::*;`, `pub mod apple; pub mod android;`, private `mod` lines for the rest, and",
      "    `pub use` for Backend, BackendFuture, ErrorKind, PurchasesError, FakeBackend, Product, ProductKind,",
      "    Purchase, PurchaseState, Entitlement, PurchaseOptions, Purchases, use_entitlements, NativePurchases and",
      "    the native options, mirroring biometrics' lib.rs re-export list.",
      "",
      "Self-check: `rustfmt --edition 2024 --check` on the files you wrote. No cargo commands.",
    ].join("\n"),
  },
  {
    key: "apple",
    name: "Apple bridge",
    implPersona: "z-rust-engineer",
    implFallback: "You are a careful Rust library engineer.",
    auditPersona: "z-unsafe-ffi-auditor",
    auditFallback: "You are a careful Rust boundary-soundness reviewer.",
    owns: "crates/nsh-purchases/src/apple.rs",
    ask: [
      "Write crates/nsh-purchases/src/apple.rs: the StoreKit 2 bridge as a host seam, in the file style of the",
      "biometrics crate (doc comment at top, mapping table as documented match arms). The bridge compiles on every",
      "target; the real StoreKit 2 glue lives in the NSH hosts, which register callbacks at startup.",
      "",
      "Public API (the Rust core area codes against exactly these names):",
      "- pub struct StoreKitHost { products, purchase, finish, restore, unfinished, entitlements, storefront, cancel }",
      "  where each callback field is Arc<dyn Fn(<args>) -> Pin<Box<dyn Future<Output = Result<R, StoreKitCode>> +",
      "  Send>> + Send + Sync>; cancel is a plain fire-and-forget fn pointer field. Args/results are plain raw",
      "  structs defined here: StoreKitProduct { id, title, description, display_price, currency_code, kind: i64 }",
      "  (kind: 0 consumable, 1 non-consumable, 2 subscription), StoreKitPurchase { transaction_id, product_id,",
      "  state: i64, json_receipt, purchased_at_ms: Option<u64> } (state: 0 pending, 1 purchased, 2 restored),",
      "  StoreKitEntitlement { product_id, active: bool, expires_at_ms: Option<u64> }, and PurchaseRequest",
      "  { product_id, quantity: u32, application_username: Option<String> }.",
      "- pub struct StoreKitCode { raw: i64, message: String } — the host's raw StoreKit error (SKError.code or the",
      "  StoreKitError case number) with its text.",
      "- pub fn install_host(host: StoreKitHost) storing into a process-global OnceLock, and",
      "  pub fn native_backend() -> Option<Arc<dyn Backend>> returning Some(new NativeBackend) once registered.",
      "- struct NativeBackend implementing crate::Backend by calling the registered host and mapping results:",
      "  initialize is a no-op Ok (StoreKit 2 needs no connect) — is_available is Ok(true) when a host is",
      "  registered; products maps raw products to model Products (kind by number, unknown kind numbers become",
      "  Backend errors with status); purchase maps state 0 to Ok(Purchase{ state: Pending }), 1/2 to Ok with the",
      "  matching state, and a StoreKitCode userCancelled to Err(Cancelled); finish/consume call the host (consume",
      "  of a non-consumable is InvalidArgument decided by the raw kind the host reports); restore/unfinished/",
      "  entitlements/storefront pass through with mapping.",
      "- The error mapping table, as match arms with a doc comment, verified against the raw codes enumerated in",
      "  .refs/flutter-packages/packages/in_app_purchase/in_app_purchase_storekit (SKError codes in",
      "  store_kit_wrappers.dart and the StoreKit 2 wrappers) — read them, do not trust memory: unknown -> Backend,",
      "  clientInvalid -> InvalidArgument, paymentCancelled -> Cancelled, paymentInvalid -> PaymentInvalid,",
      "  paymentNotAllowed -> NotAllowed, cloudServiceNetworkConnectionFailed -> Network,",
      "  cloudServicePermissionDenied -> NotAllowed, cloudServiceRevoked -> NotAllowed, and the StoreKit 2",
      "  purchase-result outcomes userCancelled -> Cancelled and pending -> the Ok-Pending path.",
      "- An unknown raw code maps to Backend carrying the status (with_status), never a panic. A malformed payload",
      "  from the host (empty id, empty receipt) is Backend with a message; no unwrap, no expect, no panic anywhere.",
      "- Every call resolves exactly once and never hangs: the bridge awaits the host future and maps its answer;",
      "  it adds no loops, no retries, no blocking.",
      "",
      "Do not create tests (the tests area owns crates/nsh-purchases/tests/apple.rs), do not touch any other file,",
      "do not add dependencies. Self-check: `rustfmt --edition 2024 --check crates/nsh-purchases/src/apple.rs`.",
    ].join("\n"),
  },
  {
    key: "android",
    name: "Android bridge",
    implPersona: "z-rust-engineer",
    implFallback: "You are a careful Rust library engineer.",
    auditPersona: "z-unsafe-ffi-auditor",
    auditFallback: "You are a careful Rust boundary-soundness reviewer.",
    owns: "crates/nsh-purchases/src/android.rs",
    ask: [
      "Write crates/nsh-purchases/src/android.rs: the Play Billing bridge as a host seam, mirroring the planned",
      "shape of src/apple.rs (the Apple area codes the same contract in parallel — keep struct and function names",
      "analogous: BillingHost, BillingCode, BillingProduct, BillingPurchase, BillingEntitlement, PurchaseRequest,",
      "install_host, native_backend, NativeBackend). The real BillingClient glue lives in the NSH hosts.",
      "",
      "Behaviour specific to Play Billing, verified against .refs/flutter-packages/packages/in_app_purchase/",
      "in_app_purchase_android/lib/billing_client_wrappers.dart (read the BillingResponse codes and purchase states",
      "from there, do not trust memory):",
      "- The BillingResponse mapping table as documented match arms: USER_CANCELED -> Cancelled,",
      "  SERVICE_UNAVAILABLE -> Network, BILLING_UNAVAILABLE -> NotAvailable, ITEM_UNAVAILABLE -> ProductNotFound,",
      "  DEVELOPER_ERROR -> InvalidArgument, ERROR -> Backend, ITEM_ALREADY_OWNED -> AlreadyOwned,",
      "  ITEM_NOT_OWNED -> NotOwned, FEATURE_NOT_SUPPORTED -> NotAvailable, network/timeouts -> Network,",
      "  unknown raw codes -> Backend carrying the status.",
      "- Purchase states: PENDING (awaiting approval, e.g. ask-a-parent) maps to Ok(Purchase{ state: Pending });",
      "  PURCHASED to Ok(Purchased); the bridge never invents a Restored state on Android (restore returns the",
      "  host's list as Purchased).",
      "- Android pending purchases must be acknowledged: finish(purchase) asks the host to acknowledge",
      "  (non-consumables and subscriptions) exactly once; consume(purchase) asks the host to consume and only",
      "  consumables reach it (a non-consumable consume is InvalidArgument, decided by the raw kind the host",
      "  reports); finish or consume of an unknown or already finished purchase is NotOwned via the host's",
      "  ITEM_NOT_OWNED.",
      "- initialize connects through the host (a BillingCode here is mapped like any other call) and is idempotent",
      "  from the facade's side.",
      "- Soundness rules identical to the Apple bridge: no unwrap, no expect, no panic, no retry loops, every call",
      "  resolves exactly once, malformed host payloads become Backend with a message, unknown raw codes become",
      "  Backend with with_status(raw).",
      "",
      "Do not create tests (the tests area owns crates/nsh-purchases/tests/android.rs), do not touch any other",
      "file, do not add dependencies. Self-check: `rustfmt --edition 2024 --check crates/nsh-purchases/src/android.rs`.",
    ].join("\n"),
  },
  {
    key: "ts",
    name: "TypeScript package",
    implPersona: "z-nsh-engineer",
    implFallback: "You are a careful NSH TypeScript package engineer.",
    auditPersona: "z-nsh-engineer",
    auditFallback: "You are a careful NSH TypeScript package reviewer.",
    owns: "packages/nsh-purchases/package.json, tsconfig.json, nsh.json, App.tsx, src/index.ts, src/errors.ts, src/options.ts, src/store.ts, src/backend.ts, src/memory.ts, src/rust.ts, src/purchases.ts, src/entitlements.ts",
    ask: [
      "Build the TypeScript package packages/nsh-purchases, mirroring the Rust API name-for-name, in the file style",
      "of .refs/biometrics/packages/nsh-biometrics (read its package.json, tsconfig.json, nsh.json, src/*.ts and",
      "App.tsx first and copy the shapes exactly — including that tests will import { expect, test } from",
      "\"nsh/test\").",
      "",
      "- package.json: name nsh-purchases, version 0.1.0, license MIT, type module, main and types ./src/index.ts,",
      "  exports { \".\": ./src/index.ts, \"./rust\": ./src/rust.ts, \"./memory\": ./src/memory.ts }, files [\"src\"],",
      "  repository https://github.com/SuperNSH/purchases — no dependencies at all (biometrics has none).",
      "- tsconfig.json: extends ./.nsh/tsconfig.json, include App.tsx, src/**/*.ts, tests/**/* (tests are written by",
      "  another area; include them anyway, exactly as biometrics does).",
      "- nsh.json: { name: \"nsh-purchases tests\", id: \"dev.supernsh.purchases.tests\", version 0.1.0, build 1,",
      "  rust: \"../../crates/nsh-purchases\", capabilities: {} }.",
      "- src/errors.ts mirrors the Rust error.rs: type PurchaseErrorCode as the union of the canonical",
      "  ERR_PURCHASES_* strings, a MESSAGES record with the SAME wording as the Rust messages, class PurchasesError",
      "  extends Error { readonly code }, purchasesError(code), and asPurchasesError(error: unknown) that parses a",
      "  host rejection carrying ERR_NSH_HOST_ERROR whose message starts with the code (copy biometrics'",
      "  asBiometricsError logic verbatim, adapted to the purchases codes; unknown codes become",
      "  ERR_PURCHASES_BACKEND).",
      "- src/options.ts: PurchaseOptions { quantity?: number, applicationUsername?: string, timeoutMs?: number }.",
      "- src/store.ts: ProductKind, Product, PurchaseState, Purchase, Entitlement interfaces matching the shared",
      "  block's TS names exactly.",
      "- src/backend.ts: interface PurchasesBackend with isAvailable(): Promise<boolean>, initialize():",
      "  Promise<void>, products(ids: string[]): Promise<Product[]>, purchase(productId: string, options:",
      "  PurchaseOptions): Promise<Purchase>, finish(purchase: Purchase): Promise<void>, consume(purchase:",
      "  Purchase): Promise<void>, restore(): Promise<Purchase[]>, pending(): Promise<Purchase[]>, entitlements():",
      "  Promise<Entitlement[]>, countryCode(): Promise<string>, cancel(): void — each store call resolving once.",
      "- src/memory.ts: memoryBackend(config) returning MemoryPurchases extends PurchasesBackend plus test controls,",
      "  in the style of biometrics' memoryBackend: scripted product list (the same fixture products as the Rust",
      "  fake: pro_unlock nonConsumable 4.99 USD, coins_100 consumable 0.99 USD, pro_monthly subscription 1.99",
      "  USD/month, sandbox receipt strings), a resolvable in-flight purchase, setNextPurchaseError, counters for",
      "  purchases/finishes/consumes/restores/cancels, last arguments, and kind rules enforced like the fake.",
      "- src/rust.ts: rustBackend(): PurchasesBackend wrapping the native class from \"nsh-purchases/native\"",
      "  (new NativePurchases()), settling every call through asPurchasesError, mapping options to the native",
      "  NativePurchaseOptions — copy biometrics' rust.ts shape exactly.",
      "- src/purchases.ts: class Purchases with the same methods as the Rust facade and the same semantics:",
      "  one purchase at a time (a second resolves ERR_PURCHASES_IN_PROGRESS), initialize required before store",
      "  calls (ERR_PURCHASES_NOT_INITIALIZED), cancel() resolves the running purchase as Cancelled, a timeoutMs",
      "  watchdog races the native answer (biometrics runs the TS watchdog for timeout and forwards timeoutMs, so",
      "  the Rust gate frees on the same deadline), invalid arguments rejected before claiming, clones/handles share",
      "  one gate per instance family, every call resolves exactly once. Also export useEntitlements(options),",
      "  mirroring biometrics' useBiometricAvailability: a signal of checking/ready/failed refreshed at start and on",
      "  lifecycle active, using the same nsh imports (cleanup, effect, lifecycle, signal from \"nsh\").",
      "- src/index.ts: re-export the public names in biometrics' index.ts style (Purchases, types, PurchasesError,",
      "  asPurchasesError, purchasesError, memoryBackend, rustBackend, useEntitlements, PurchaseOptions).",
      "- App.tsx: a tiny demo screen in biometrics' App.tsx style (mount, signal, a couple of Text rows and one",
      "  button showing a purchase attempt's outcome); it exists so tsconfig's include compiles and the demo runs.",
      "",
      "Same-named API, same behaviour, same codes as the Rust crate. Do not create tests (the tests area owns",
      "packages/nsh-purchases/tests/), do not touch any other file.",
    ].join("\n"),
  },
  {
    key: "tests",
    name: "Tests and CI",
    implPersona: "z-nsh-engineer",
    implFallback: "You are a careful NSH Rust-and-TSX test engineer.",
    auditPersona: "z-nsh-engineer",
    auditFallback: "You are a careful NSH test reviewer.",
    owns: "crates/nsh-purchases/tests/purchases.rs, crates/nsh-purchases/tests/apple.rs, crates/nsh-purchases/tests/android.rs, packages/nsh-purchases/tests/purchases.test.tsx, packages/nsh-purchases/tests/errors.test.tsx, .github/workflows/ci.yml",
    ask: [
      "Write the test suites in Rust and TypeScript for every goal, edge case and error path, plus the CI update.",
      "Rust tests use the FakeBackend and mock hosts through the public API only, in the style of",
      ".refs/biometrics/crates/nsh-biometrics/tests/auth.rs (tokio current_thread runtime via a local run() helper,",
      "yield-based settle loops, named assertions). TS tests use memoryBackend and import { expect, test } from",
      "\"nsh/test\", in the style of .refs/biometrics/packages/nsh-biometrics/tests/auth.test.tsx.",
      "",
      "crates/nsh-purchases/tests/purchases.rs — with the FakeBackend, one #[tokio::test] per name:",
      "  purchase_twice_in_parallel_is_in_progress; cancel_resolves_the_running_purchase_as_cancelled (and the late",
      "  answer is dropped); a_timeout_resolves_with_the_timeout_code_and_the_gate_recovers; a_zero_timeout_is_an_",
      "invalid_argument; calls_before_initialize_are_not_initialized; initialize_is_idempotent; empty_product_ids_",
      "are_an_invalid_argument; a_zero_quantity_is_an_invalid_argument; unknown_product_ids_are_dropped_from_the_",
      "answer; purchasing_an_unknown_product_is_product_not_found; a_purchase_resolves_exactly_once (a scripted late",
      "second answer is ignored); a_pending_purchase_resolves_with_a_pending_state; a_cancelled_purchase_resolves_",
      "with_the_cancelled_code; an_already_owned_product_is_already_owned; finishing_marks_the_purchase_finished_",
      "once; finishing_an_unknown_or_finished_purchase_is_not_owned; consuming_a_non_consumable_is_an_invalid_",
      "argument; restore_returns_purchased_and_restored_purchases; restore_with_no_purchases_is_empty; pending_",
      "returns_unfinished_purchases; entitlements_exclude_consumed_consumables; entitlements_respect_expiry (an",
      "expired subscription is inactive); an_unsupported_target_is_not_available (Purchases::new() with no host)",
      "  concurrent_non_purchase_calls_do_not_claim_the_gate; a_dropped_purchase_call_cancels_the_store_request",
      "  (drop the future, the fake's cancel counter incremented); every_error_code_round_trips (the table of all",
      "  ErrorKind values against their canonical ERR_PURCHASES_* strings, like biometrics' ALL_KINDS table).",
      "crates/nsh-purchases/tests/apple.rs — a mock StoreKitHost (closures answering from fixtures) proving:",
      "  every_storekit_raw_code_maps_to_its_documented_code (table-driven over the bridge's mapping);",
      "  unknown_storekit_raw_codes_are_backend (status preserved); a_storekit_cancel_and_a_pending_purchase_resolve_",
      "as_documented (userCancelled -> Cancelled; state 0 -> Ok Pending).",
      "crates/nsh-purchases/tests/android.rs — a mock BillingHost proving:",
      "  every_billing_raw_code_maps_to_its_documented_code; unknown_billing_raw_codes_are_backend;",
      "  finishing_acknowledges_and_consuming_consumes (the mock host's acknowledge/consume counters).",
      "packages/nsh-purchases/tests/purchases.test.tsx — the TS mirrors of the facade behaviours above (the gate,",
      "  cancel and late answers, timeoutMs watchdog, initialize and arg checks, unknown ids, pending state,",
      "  cancelled and already-owned codes, finish/consume counters, restore, pending, entitlements incl. expiry,",
      "  unsupported target via a memory backend configured as unavailable, code round-trip over the union).",
      "packages/nsh-purchases/tests/errors.test.tsx — asPurchasesError: known codes pass through, a host rejection",
      "  (ERR_NSH_HOST_ERROR with an ERR_PURCHASES_* prefix) is parsed, an unknown prefix or unknown code becomes",
      "  ERR_PURCHASES_BACKEND, and messages match the Rust wording.",
      "",
      ".github/workflows/ci.yml: keep the existing rust job's fmt/clippy/test steps and checkout/toolchain pins,",
      "and add a typescript job modeled on .refs/biometrics/.github/workflows/ci.yml's typescript job: ubuntu-latest,",
      "install NSH v0.4.0 from SuperNSH/nsh-builds (copy their curl installer steps verbatim), then in",
      "packages/nsh-purchases run `nsh check` and `nsh test`. Name the steps plainly. No other workflow changes.",
      "",
      "The named tests above are canonical: the goals doc lists them and the parity doc cites them — use exactly",
      "these names. Do not touch any other file. The whole-tree gates (fmt, clippy, cargo test, nsh check, nsh test)",
      "run once after all areas land; do not run them yourself.",
    ].join("\n"),
  },
];

// ---------------------------------------------------------------------------
// Per-area state machines: implement, then audit -> fix until zero findings,
// capped at AUDIT_CAP rounds. One area's failure costs one area.
// ---------------------------------------------------------------------------

function boardUpdate(name: string, phase: string, summary: string): void {
  report({ name, phase, summary }, "areas");
}

function personaCheck(persona: string, source: string, line: string | undefined): string {
  if (source === "fallback") return "plain persona";
  if (typeof line === "string" && persona.includes(line.trim())) return "verified";
  return "unverified";
}

phase("Implement and audit each area in parallel");
log(`${AREAS.length} areas start their own chains now`);

const outcomes: AreaOutcome[] = await Promise.all(
  AREAS.map(async (area): Promise<AreaOutcome> => {
    try {
      const impl = await specialist(area.implPersona, area.implFallback);
      const auditor = await specialist(area.auditPersona, area.auditFallback);
      const engineer = agent(`${area.name} engineer`, { system: impl.persona + "\n\n" + HONESTY, model: MODELS.pro });
      const reviewer = agent(`${area.name} auditor`, { system: auditor.persona + "\n\n" + HONESTY, model: MODELS.pro });

      boardUpdate(area.name, "implementing", `writing ${area.owns}`);
      const built = await engineer.ask<ImplementResult>(
        `${COMMON}\n\nYOUR AREA: ${area.name}. You own EXACTLY these files: ${area.owns}.\nCreate or edit only those files.\n\n${area.ask}\n\nFinish the whole ask in this one turn, then report: done, a 2-3 sentence summary of what landed, any blockers, and contractLine — one sentence copied VERBATIM from your specialist instructions that most shaped the work.`,
      );
      let personaNote = personaCheck(impl.persona, impl.source, built.contractLine);
      if (impl.source !== "fallback" && personaNote === "unverified") {
        const retry = await engineer.ask<ImplementResult>(
          "Your contractLine did not match your instructions. Copy ONE sentence from your instructions verbatim into contractLine, and restate done/summary/blockers.",
        );
        personaNote = personaCheck(impl.persona, impl.source, retry.contractLine);
        if (retry.done === false) {
          return { name: area.name, status: "failed", summary: `implement retry reports blocked: ${retry.summary ?? "no summary"} (persona ${personaNote})`, rounds: 0, converged: false };
        }
      }
      if (!built.done) {
        boardUpdate(area.name, "failed", built.summary);
        return { name: area.name, status: "failed", summary: `implement did not finish: ${built.summary} (blockers: ${(built.blockers ?? []).join("; ") || "none"}; persona ${personaNote})`, rounds: 0, converged: false };
      }

      let rounds = 0;
      let converged = false;
      let lastAudit = "";
      while (rounds < AUDIT_CAP) {
        rounds += 1;
        boardUpdate(area.name, "auditing", `audit round ${rounds} over ${area.owns}`);
        const verdict = await reviewer.ask<Audit>(
          `${COMMON}\n\nYOU ARE THE AUDITOR for the area "${area.name}". Audit the area's files — exactly: ${area.owns} — against the plan below. Read the repository and the .refs/ reference clones as needed. DO NOT edit any file and DO NOT run cargo, nsh, or any whole-tree command (the script runs the integrated gates once after every area lands).\n\nReport ONLY actionable findings: concrete problems a fixer can apply — a deviation from this plan, a violated rule, a wrong or invented API name, a missing behaviour, a claim the code does not support. No praise, no PASS notes. An empty findings array means the area is clean.\n\nTHE PLAN:\n${area.ask}\n\nAlso report: contractLine — one sentence copied VERBATIM from your specialist instructions that most shaped this audit.`,
        );
        const auditNote = personaCheck(auditor.persona, auditor.source, verdict.contractLine);
        lastAudit = `${verdict.findings.length} findings (persona ${auditNote})`;
        if (verdict.findings.length === 0) {
          converged = true;
          break;
        }
        if (rounds < AUDIT_CAP) {
          boardUpdate(area.name, "fixing", `fixing ${verdict.findings.length} findings`);
          await engineer.ask<FixResult>(
            `Your audit found problems in the files you own (${area.owns}). Fix every finding with the smallest honest change, keeping the plan's rules. If a finding would need a file you do not own, do NOT edit that file — name it in blockers instead.\n\nFINDINGS:\n${JSON.stringify(verdict.findings, null, 2)}\n\nReport: done, a 2-3 sentence summary, blockers.`,
          );
        } else {
          lastAudit = `${verdict.findings.length} findings remain after the fix cap (persona ${auditNote})`;
        }
      }
      if (converged) {
        boardUpdate(area.name, "done", `clean after ${rounds} audit round${rounds === 1 ? "" : "s"}`);
        return { name: area.name, status: "done", summary: `${built.summary} Audits: clean after ${rounds} round${rounds === 1 ? "" : "s"}.`, rounds, converged: true };
      }
      boardUpdate(area.name, "failed", `still findings at the audit cap: ${lastAudit}`);
      return { name: area.name, status: "failed", summary: `${built.summary} Did NOT converge: ${lastAudit}.`, rounds, converged: false };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      boardUpdate(area.name, "failed", message);
      return { name: area.name, status: "failed", summary: `area error: ${message}`, rounds: 0, converged: false };
    }
  }),
);

// ---------------------------------------------------------------------------
// The whole-tree gates the brief names, once at the join. A persistent fixer
// repairs failures; bounded rounds, reported honestly.
// ---------------------------------------------------------------------------

phase("Run the integrated gates once");
boardUpdate("whole tree", "testing", "fmt, clippy, cargo test, nsh check, nsh test");

const GATES: { name: string; cmd: string; args: string[]; timeoutMs: number }[] = [
  { name: "cargo fmt --all -- --check", cmd: "cargo", args: ["fmt", "--all", "--", "--check"], timeoutMs: 120_000 },
  { name: "cargo clippy --all-targets -- -D warnings", cmd: "cargo", args: ["clippy", "--all-targets", "--", "-D", "warnings"], timeoutMs: 1_800_000 },
  { name: "cargo test --all-targets", cmd: "cargo", args: ["test", "--all-targets"], timeoutMs: 1_800_000 },
  { name: "nsh check (packages/nsh-purchases)", cmd: "bash", args: ["-c", "cd packages/nsh-purchases && nsh check"], timeoutMs: 1_800_000 },
  { name: "nsh test (packages/nsh-purchases)", cmd: "bash", args: ["-c", "cd packages/nsh-purchases && nsh test"], timeoutMs: 1_800_000 },
];

let gateResults: GateOutcome[] = GATES.map((gate) => ({ name: gate.name, ok: false, output: "not run" }));
const fixerPersona = await specialist("z-nsh-engineer", "You are a careful NSH Rust-and-TSX engineer.");
// One persistent fixer across all rounds: reuse keeps what it learned, and a
// fixed name created twice would fail the run.
const gateFixer = agent("Gate fixer", { system: fixerPersona.persona + "\n\n" + HONESTY, model: MODELS.pro });

for (let round = 1; round <= GATE_FIX_CAP; round += 1) {
  const failed: { name: string; cmd: string; args: string[]; timeoutMs: number; index: number }[] = [];
  for (let i = 0; i < GATES.length; i += 1) {
    const gate = GATES[i];
    if (gateResults[i] !== undefined && gateResults[i]!.ok) continue;
    log(`gate: ${gate.name}`);
    const outcome = await world.run(gate.cmd, gate.args, { timeoutMs: gate.timeoutMs });
    const ok = outcome.exitCode === 0;
    gateResults[i] = { name: gate.name, ok, output: (ok ? outcome.stdout : `${outcome.stdout}\n${outcome.stderr}`).slice(-4000) };
    if (!ok) failed.push({ ...gate, index: i });
  }
  if (failed.length === 0) break;
  if (round === GATE_FIX_CAP) break;
  boardUpdate("whole tree", "fixing", `repairing ${failed.length} failed gate${failed.length === 1 ? "" : "s"}`);
  await gateFixer.ask<FixResult>(
    `All areas have landed; the whole tree is yours to repair. These gates failed — fix the code so each passes, keeping behaviour and never deleting a test to make it pass. If a failure exposes a plan conflict you cannot resolve, escalate instead of guessing.\n\n${JSON.stringify(failed.map((f) => ({ gate: f.name, output: gateResults[f.index]!.output })), null, 2)}`,
  );
}

const gatesOk = gateResults.every((g) => g.ok);
boardUpdate("whole tree", gatesOk ? "done" : "failed", gatesOk ? "all gates green" : `${gateResults.filter((g) => !g.ok).length} gate(s) still failing`);

// ---------------------------------------------------------------------------
// The report: composed from the run's own values, independently read, published.
// ---------------------------------------------------------------------------

phase("Independently read the report and publish it");

const failedAreas = outcomes.filter((o) => o.status === "failed");
const unconverged = outcomes.filter((o) => !o.converged);
const gateLines = gateResults.map((g) => `- ${g.ok ? "pass" : "FAIL"}: ${g.name}`).join("\n");
const areaLines = outcomes
  .map((o) => `- **${o.name}** — ${o.status}${o.converged ? "" : " (did not converge)"}: ${o.summary}`)
  .join("\n");

let reportMarkdown = [
  "# nsh-purchases build report",
  "",
  "## Areas",
  areaLines,
  "",
  "## Integrated gates (whole tree, once)",
  gateLines,
  "",
  "## Notes",
  "- Local runs: AGENTS.md wants every compile and test on GitHub Actions, but this brief forbids commits and pushes, so the gates ran locally in this checkout on mocks and sandbox fixtures only. `.github/workflows/ci.yml` now runs the same gates in CI when pushed.",
  "- No real store accounts, products or purchases: every store interaction is FakeBackend, a mock host, or the memory backend. The bridges speak to the store through host callbacks (see docs/decisions.md).",
  "- react-native-iap moved to hyodotdev/openiap; the canonical source was cloned alongside the archived repo and recorded in docs/decisions.md.",
  "- References are cloned read-only under .refs/ (git-ignored).",
  unconverged.length > 0 ? `- Audit convergence: ${unconverged.map((o) => o.name).join(", ")} did not converge within the cap.` : "- Audit convergence: every area reached zero findings within the cap.",
  failedAreas.length === 0 && gatesOk
    ? "- Status: the crate and package build and their suites pass locally under the gates the brief names."
    : "- Status: INCOMPLETE — see the failed areas and gates above.",
].join("\n");

const reader = agent("Report reader", {
  system:
    "You judge a report the way its reader would: from the text alone, without verifying it against the repository. " + HONESTY,
  model: MODELS.pro,
});
const read = await reader.ask<ReaderProblems>(
  `Read this run report and list what is unclear, what the text itself does not support, and what a reader will ask next. Judge the text as given; do not wander off to verify it. Return problems: [] when it reads clean.\n\n${reportMarkdown}`,
);
if (read.problems.length > 0) {
  const editor = agent("Report editor", { system: "You revise reports precisely, changing only what the reader flagged. " + HONESTY, model: MODELS.pro });
  const revised = await editor.ask<RevisedReport>(
    `Revise the report below so every reader problem is addressed. Return the full revised markdown, nothing else.\n\nPROBLEMS:\n${JSON.stringify(read.problems, null, 2)}\n\nREPORT:\n${reportMarkdown}`,
  );
  reportMarkdown = revised.markdown;
}

await artifact.markdown("report", reportMarkdown, {
  title: "nsh-purchases build report",
  description: "Per-area outcomes, audit convergence and the integrated gates",
  primary: true,
});

const notes = [
  "Model pins: every subagent ran on account:zai-individual-coding-plan/GLM-5.3$high per the user's explicit request; no vision phases exist in this run.",
  "Audit budget: 3 rounds per area, until zero issues or the cap, as the brief asked.",
  failedAreas.length === 0 && gatesOk
    ? "All seven areas landed and every integrated gate passed."
    : "One or more areas or gates did not finish clean — see the report artifact.",
];

return {
  conclusion:
    failedAreas.length === 0 && gatesOk
      ? `nsh-purchases now has the Rust crate (crates/nsh-purchases), the TypeScript package (packages/nsh-purchases), docs/goals.md, docs/parity.md and the named-test suites in both languages, laid out like SuperNSH/biometrics; all seven areas reached zero audit findings and all integrated gates passed locally (no commits or pushes, per the brief).`
      : `nsh-purchases build finished with problems: ${failedAreas.map((o) => o.name).join(", ") || "no failed areas"}; ${gateResults.filter((g) => !g.ok).length} of ${GATES.length} integrated gates still failing. See the report artifact for the honest per-area state.`,
  areas: outcomes,
  findings: [
    ...unconverged.map(
      (o): AuditFinding => ({
        file: areaFileFor(o.name),
        problem: `Area did not converge within the ${AUDIT_CAP}-round audit cap.`,
        fix: o.summary,
        severity: "medium",
      }),
    ),
    ...gateResults
      .filter((g) => !g.ok)
      .map(
        (g): AuditFinding => ({
          file: "whole tree",
          problem: `Integrated gate still failing: ${g.name}.`,
          fix: g.output.slice(-500),
          severity: "high",
        }),
      ),
  ],
  verified: gateResults.map((g) => `${g.ok ? "pass" : "FAIL"}: ${g.name}`),
  notCovered: [
    "No GitHub Actions run: the brief forbids commits and pushes, so CI has not executed these gates yet; they ran locally instead.",
    "No device, emulator or simulator testing: AGENTS.md forbids it on a machine, and the brief forbids real store accounts — all store behaviour is mock- and fixture-tested only.",
    "No visual verification of App.tsx (no vision phase; the demo screen is type-checked and bundled only).",
    "The real StoreKit 2 and BillingClient glue lives in the NSH hosts, outside this repo; the bridges here are the host-seam mapping, fully covered by the mapping tests.",
  ],
  notes,
};
