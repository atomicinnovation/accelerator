//! Forwarding an interrupt to the command's process group, which no longer
//! receives the terminal's Ctrl-C itself.
//!
//! Handlers are installed once per process and never removed:
//! `signal-hook` cannot restore the previous disposition on unregister, so
//! per-run registration would leave the CLI ignoring these signals once a
//! command had run. While no run is active the handler emulates the default
//! action, so the CLI behaves as though none were installed. A signal whose
//! inherited disposition is `SIG_IGN` is left alone, so a CLI under `nohup`
//! keeps ignoring it.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::Once;
use std::sync::PoisonError;

use signal_hook::consts::SIGHUP;
use signal_hook::consts::SIGINT;
use signal_hook::consts::SIGTERM;

static IDLE: LazyLock<Arc<AtomicBool>> =
    LazyLock::new(|| Arc::new(AtomicBool::new(true)));
static RECORDED: LazyLock<Arc<AtomicUsize>> =
    LazyLock::new(|| Arc::new(AtomicUsize::new(0)));
static INSTALLED: Once = Once::new();
static ONE_RUN_AT_A_TIME: Mutex<()> = Mutex::new(());

/// The one active run. Its drop acts on any signal recorded during the run,
/// on every exit path, so a signal is never swallowed nor carried into the
/// next run.
pub(super) struct ActiveRun {
    recorded: Arc<AtomicUsize>,
    _exclusive: MutexGuard<'static, ()>,
}

impl ActiveRun {
    pub(super) fn begin() -> Self {
        let exclusive = ONE_RUN_AT_A_TIME
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        INSTALLED.call_once(install);
        RECORDED.store(0, Ordering::SeqCst);
        IDLE.store(false, Ordering::SeqCst);
        Self {
            recorded: Arc::clone(&RECORDED),
            _exclusive: exclusive,
        }
    }

    pub(super) fn interrupted(&self) -> bool {
        self.recorded.load(Ordering::SeqCst) != 0
    }
}

impl Drop for ActiveRun {
    fn drop(&mut self) {
        IDLE.store(true, Ordering::SeqCst);
        let recorded = self.recorded.swap(0, Ordering::SeqCst);
        if let Ok(signal) = i32::try_from(recorded) {
            if signal != 0 {
                let _ = signal_hook::low_level::emulate_default_handler(signal);
            }
        }
    }
}

fn install() {
    for signal in [SIGINT, SIGTERM, SIGHUP] {
        if ignored(signal) {
            continue;
        }
        let _ = signal_hook::flag::register_conditional_default(
            signal,
            Arc::clone(&IDLE),
        );
        if let Ok(value) = usize::try_from(signal) {
            let _ = signal_hook::flag::register_usize(
                signal,
                Arc::clone(&RECORDED),
                value,
            );
        }
    }
}

fn ignored(signal: libc::c_int) -> bool {
    // SAFETY: an all-zero `sigaction` is a valid value for the kernel to
    // overwrite, and a null new action only reads the current disposition.
    let mut current: libc::sigaction = unsafe { std::mem::zeroed() };
    let read =
        unsafe { libc::sigaction(signal, std::ptr::null(), &raw mut current) };
    read == 0 && current.sa_sigaction == libc::SIG_IGN
}
