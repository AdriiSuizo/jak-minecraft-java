//! Named CPU spans within a frame, for the frame profile
//! (`MINECRAFTOSS_PROFILE_FRAMES`). Recording is a push onto one list for
//! the whole process (the game's systems run on several threads) when
//! profiling is on, and nothing otherwise.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static ENABLED: AtomicBool = AtomicBool::new(false);

static SPANS: Mutex<Vec<(&'static str, f64)>> = Mutex::new(Vec::new());

fn push(name: &'static str, ms: f64) {
    if let Ok(mut spans) = SPANS.lock() {
        spans.push((name, ms));
    }
}

pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Times the rest of the enclosing scope under `name`.
pub struct Span {
    name: &'static str,
    started: Option<Instant>,
}

pub fn span(name: &'static str) -> Span {
    Span {
        name,
        started: ENABLED.load(Ordering::Relaxed).then(Instant::now),
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(started) = self.started {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            push(self.name, ms);
        }
    }
}

/// Records a duration measured elsewhere.
pub fn record(name: &'static str, ms: f64) {
    if ENABLED.load(Ordering::Relaxed) {
        push(name, ms);
    }
}

/// The spans since the last call, summed by name.
pub fn take() -> Vec<(&'static str, f64)> {
    let drained: Vec<(&'static str, f64)> = SPANS.lock().map(|mut s| s.drain(..).collect()).unwrap_or_default();
    let mut out: Vec<(&'static str, f64)> = Vec::new();
    for (name, ms) in drained {
        match out.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 += ms,
            None => out.push((name, ms)),
        }
    }
    out
}
