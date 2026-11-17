//! The reactive entitlements read on `Headless`: it wakes only on change, and reads again on foreground.

use std::{
    future::Future,
    sync::{
        Arc, Mutex, OnceLock, PoisonError,
        atomic::{AtomicUsize, Ordering::SeqCst},
    },
    time::{Duration, SystemTime},
};

use nsh_core::{platform::Lifecycle, prelude::*};
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_purchases::{
    Entitlement, Entitlements, EntitlementsState, ErrorKind, FakeBackend, Purchases,
    PurchasesError, use_entitlements,
};
use nsh_testing::Headless;

static RUNS: AtomicUsize = AtomicUsize::new(0);
static FAKE: OnceLock<FakeBackend> = OnceLock::new();
static STORE: OnceLock<Purchases> = OnceLock::new();
/// The latest read's handle, which outlives its owner once Remove dismissed it.
static HANDLE: Mutex<Option<Entitlements>> = Mutex::new(None);
/// The engines share the fake and the counter, so the tests take turns.
static SERIAL: Mutex<()> = Mutex::new(());

fn fake() -> FakeBackend {
    FAKE.get_or_init(FakeBackend::new).clone()
}

fn run<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(future)
}

/// The one initialized store over the fake: the hook reads before its first render otherwise.
fn store() -> Purchases {
    STORE
        .get_or_init(|| {
            let purchases = Purchases::with_backend(Arc::new(fake()));
            run(async {
                purchases.initialize().await.unwrap();
            });
            purchases
        })
        .clone()
}

#[component]
pub fn Probe() -> impl IntoElement {
    let mounted = signal(true);
    view()
        .child(when(mounted, reader))
        .child(
            view()
                .role(AccessibilityRole::Button)
                .label("Remove")
                .on_press(move || mounted.set(false))
                .child(text("Remove")),
        )
        .child(
            view()
                .role(AccessibilityRole::Button)
                .label("Refresh")
                .on_press(|| {
                    if let Some(handle) = HANDLE
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .as_ref()
                    {
                        handle.refresh();
                    }
                })
                .child(text("Refresh")),
        )
}

/// The read, made under the `when` region's owner so Remove dismisses it.
fn reader() -> impl IntoElement {
    let entitlements = use_entitlements(&store());
    *HANDLE.lock().unwrap_or_else(PoisonError::into_inner) = Some(entitlements);
    text(move || {
        RUNS.fetch_add(1, SeqCst);
        match entitlements.state() {
            EntitlementsState::Checking => "checking".to_owned(),
            EntitlementsState::Ready(list) => format!(
                "ready {}",
                list.iter()
                    .map(|entitlement| entitlement.product_id.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            EntitlementsState::Failed(error) => format!("failed {}", error.code()),
        }
    })
}

fn start() -> Result<Headless, String> {
    let config = EngineConfig {
        clock: Clock::manual(SystemTime::UNIX_EPOCH),
        ..EngineConfig::new(Root::of::<Probe>())
    };
    Headless::start(config).map_err(|e| format!("the engine did not start: {e}"))
}

/// The fixture answer: one active unlock.
fn unlock() -> Entitlement {
    Entitlement {
        product_id: "pro_unlock".to_owned(),
        active: true,
        expires_at_ms: None,
    }
}

/// The fixture answer after a renewal: the unlock plus a monthly subscription.
fn renewed() -> Vec<Entitlement> {
    vec![
        unlock(),
        Entitlement {
            product_id: "pro_monthly".to_owned(),
            active: true,
            expires_at_ms: Some(1_735_689_600_000),
        },
    ]
}

fn reset() {
    let fake = fake();
    fake.set_entitlements(Ok(vec![unlock()]));
    RUNS.store(0, SeqCst);
    *HANDLE.lock().unwrap_or_else(PoisonError::into_inner) = None;
    store();
}

fn leave_foreground(app: &mut Headless) -> Result<(), String> {
    app.set_environment("setLifecycle(background)", |e| {
        e.lifecycle = Lifecycle::Background;
    })?;
    app.run_for(Duration::from_millis(50))
}

fn return_to_foreground(app: &mut Headless) -> Result<(), String> {
    app.set_environment("setLifecycle(active)", |e| {
        e.lifecycle = Lifecycle::Active;
    })?;
    app.run_for(Duration::from_millis(50))
}

#[test]
fn the_answer_is_read_when_the_owner_starts() -> Result<(), String> {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    reset();
    let mut app = start()?;
    app.wait_for("ready pro_unlock")?;
    Ok(())
}

#[test]
fn a_binding_runs_again_only_when_the_answer_changes() -> Result<(), String> {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    reset();
    let mut app = start()?;
    app.wait_for("ready pro_unlock")?;
    let settled = RUNS.load(SeqCst);
    let reads = fake().entitlement_reads();

    leave_foreground(&mut app)?;
    return_to_foreground(&mut app)?;
    assert!(
        fake().entitlement_reads() > reads,
        "the foreground read ran"
    );
    assert_eq!(
        RUNS.load(SeqCst),
        settled,
        "the same answer must not run the binding again"
    );
    Ok(())
}

#[test]
fn a_changed_answer_reaches_the_binding_on_return_to_the_foreground() -> Result<(), String> {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    reset();
    let mut app = start()?;
    app.wait_for("ready pro_unlock")?;

    // A subscription renewed while the app was away.
    leave_foreground(&mut app)?;
    fake().set_entitlements(Ok(renewed()));
    return_to_foreground(&mut app)?;
    app.wait_for("ready pro_unlock pro_monthly")?;
    Ok(())
}

#[test]
fn a_failed_read_is_reported_with_its_code() -> Result<(), String> {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    reset();
    fake().set_entitlements(Err(PurchasesError::new(ErrorKind::Backend)));
    let mut app = start()?;
    app.wait_for("failed ERR_PURCHASES_BACKEND")?;
    Ok(())
}

#[test]
fn refresh_does_nothing_once_the_owner_is_gone() -> Result<(), String> {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    reset();
    let mut app = start()?;
    app.wait_for("ready pro_unlock")?;

    // While the owner lives, a refresh reads the store again.
    let reads = fake().entitlement_reads();
    app.press("role:button[Refresh]")?;
    app.run_for(Duration::from_millis(50))?;
    assert_eq!(fake().entitlement_reads(), reads + 1);

    // The handle outlives Remove, and its refresh must not read.
    app.press("role:button[Remove]")?;
    app.run_for(Duration::from_millis(50))?;
    let after = fake().entitlement_reads();
    app.press("role:button[Refresh]")?;
    app.run_for(Duration::from_millis(200))?;
    assert_eq!(fake().entitlement_reads(), after);
    Ok(())
}
