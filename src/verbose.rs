//! A small, dependency-light animated progress reporter behind
//! `bitterasm expand --verbose`/`bitterasm compile --verbose`: one status
//! line per unit of work (one invocation being expanded), written to
//! stderr. A unit that finishes quickly prints its result directly; the
//! "..." animation and running timer only ever appear once a unit has been
//! in flight longer than [`GRACE_PERIOD`], so an ordinary, fast run isn't
//! flooded with intermediate frames — only the slow invocation that's
//! actually worth watching ever animates. See
//! [`VerboseReporter::start`]/[`VerboseReporter::finish_ok`].
//!
//! Animation happens on a dedicated background thread because the actual
//! work (macro expansion) is synchronous, CPU-bound, and single-threaded —
//! nothing on the caller's side ever gets a chance to repaint the terminal
//! itself while it's busy.

use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const GRACE_PERIOD: Duration = Duration::from_millis(300);
const TICK_INTERVAL: Duration = Duration::from_millis(150);

struct Unit {
    description: String,
    start: Instant,
    dots: usize,
    shown: bool,
}

/// Owns a single background thread that repaints the current unit's status
/// line every [`TICK_INTERVAL`]. Only one unit is ever "current" at a time —
/// [`start`](Self::start) replaces whatever was there before, so a caller is
/// expected to `finish_ok`/`finish_err` each unit before starting the next.
pub struct VerboseReporter {
    current: Arc<Mutex<Option<Unit>>>,
    stop: Arc<AtomicBool>,
    ticker: Option<JoinHandle<()>>,
}

impl VerboseReporter {
    pub fn new() -> Self {
        let current: Arc<Mutex<Option<Unit>>> = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));

        // Redirected/non-terminal stderr (a log file, `| cat`, ...) skips
        // the animation entirely — a carriage-return-driven spinner is only
        // meaningful on a real terminal, and printed literally it'd just be
        // noise. `finish` still emits one plain line per unit either way.
        let animate = io::stderr().is_terminal();

        let ticker = {
            let current = Arc::clone(&current);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    thread::sleep(TICK_INTERVAL);
                    if !animate {
                        continue;
                    }
                    let mut guard = current.lock().unwrap_or_else(|poison| poison.into_inner());
                    if let Some(unit) = guard.as_mut() {
                        let elapsed = unit.start.elapsed();
                        if elapsed >= GRACE_PERIOD {
                            unit.dots = unit.dots % 3 + 1;
                            unit.shown = true;
                            eprint!(
                                "\r\x1b[2Kexpanding {} ({:.1}s){}",
                                unit.description,
                                elapsed.as_secs_f64(),
                                ".".repeat(unit.dots),
                            );
                            let _ = io::stderr().flush();
                        }
                    }
                }
            })
        };

        Self { current, stop, ticker: Some(ticker) }
    }

    /// Marks `description` (e.g. `"foo.basm:12 double(...)"`) as the unit
    /// now in progress.
    pub fn start(&self, description: String) {
        let mut guard = self.current.lock().unwrap_or_else(|poison| poison.into_inner());
        *guard = Some(Unit { description, start: Instant::now(), dots: 0, shown: false });
    }

    pub fn finish_ok(&self) {
        self.finish("OK");
    }

    pub fn finish_err(&self) {
        self.finish("ERR");
    }

    fn finish(&self, outcome: &str) {
        let mut guard = self.current.lock().unwrap_or_else(|poison| poison.into_inner());
        let Some(unit) = guard.take() else { return };
        drop(guard);

        let elapsed = unit.start.elapsed();
        let mem = current_memory_mb();
        let line = format!(
            "expanding {} {outcome}  ({:.2}s, {:.1} MB)",
            unit.description,
            elapsed.as_secs_f64(),
            mem,
        );
        if unit.shown {
            eprintln!("\r\x1b[2K{line}");
        } else {
            eprintln!("{line}");
        }
    }
}

impl Default for VerboseReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for VerboseReporter {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(ticker) = self.ticker.take() {
            let _ = ticker.join();
        }
    }
}

/// This process's peak resident set size so far, in megabytes — one
/// syscall, cheap enough to call after every expanded invocation, unlike
/// shelling out to `ps`. `ru_maxrss` is bytes on macOS/BSD but kibibytes on
/// Linux; `getrusage` itself defines no portable unit.
fn current_memory_mb() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let bytes = unsafe {
        if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) != 0 {
            return 0.0;
        }
        let usage = usage.assume_init();
        #[cfg(target_os = "macos")]
        {
            usage.ru_maxrss as f64
        }
        #[cfg(not(target_os = "macos"))]
        {
            usage.ru_maxrss as f64 * 1024.0
        }
    };
    bytes / (1024.0 * 1024.0)
}
