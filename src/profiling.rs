//! Profiling utilities for performance analysis.
//!
//! Enable with `maturin develop --release --features profiling`

#[cfg(feature = "profiling")]
use std::time::Instant;

/// Time a phase and optionally print the duration.
/// When profiling is disabled, this is a no-op.
#[cfg(feature = "profiling")]
macro_rules! time_phase {
    ($name:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        let result = $block;
        let elapsed = start.elapsed();
        eprintln!("[PROFILE] {}: {:?}", $name, elapsed);
        result
    }};
}

#[cfg(not(feature = "profiling"))]
macro_rules! time_phase {
    ($name:expr, $block:expr) => {
        $block
    };
}

pub(crate) use time_phase;

/// Phase timing accumulator for aggregate profiling.
#[cfg(feature = "profiling")]
pub struct PhaseTimer {
    name: &'static str,
    start: Instant,
}

#[cfg(feature = "profiling")]
impl PhaseTimer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }

    pub fn elapsed_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }
}

#[cfg(feature = "profiling")]
impl Drop for PhaseTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        eprintln!("[PROFILE] {}: {:?}", self.name, elapsed);
    }
}

/// Start a phase timer (profiling builds only).
#[cfg(feature = "profiling")]
macro_rules! start_phase {
    ($name:expr) => {
        let _timer = $crate::profiling::PhaseTimer::new($name);
    };
}

#[cfg(not(feature = "profiling"))]
macro_rules! start_phase {
    ($name:expr) => {};
}

pub(crate) use start_phase;
