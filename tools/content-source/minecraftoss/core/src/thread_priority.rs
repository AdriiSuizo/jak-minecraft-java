//! Scheduling priority for engine threads: background workers yield to the
//! render and server threads, so a busy pool cannot preempt a frame.
//! Only Windows is handled; elsewhere these do nothing.

#[cfg(windows)]
mod ffi {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn GetCurrentThread() -> *mut c_void;
        pub fn SetThreadPriority(thread: *mut c_void, priority: i32) -> i32;
        pub fn GetCurrentProcess() -> *mut c_void;
        pub fn SetProcessInformation(process: *mut c_void, class: i32, information: *const c_void, size: u32) -> i32;
        pub fn SetPriorityClass(process: *mut c_void, class: u32) -> i32;
    }

    /// `PROCESS_POWER_THROTTLING_STATE`.
    #[repr(C)]
    pub struct PowerThrottlingState {
        pub version: u32,
        pub control_mask: u32,
        pub state_mask: u32,
    }
}

#[cfg(windows)]
fn set(priority: i32) {
    // SAFETY: the pseudo handle of the calling thread is always valid.
    unsafe {
        ffi::SetThreadPriority(ffi::GetCurrentThread(), priority);
    }
}

#[cfg(not(windows))]
fn set(_priority: i32) {}

/// `THREAD_PRIORITY_BELOW_NORMAL`, for worker pools.
pub fn background() {
    set(-1);
}

/// `THREAD_PRIORITY_LOWEST`, for work that should only fill cores the
/// worker pools leave idle (the far terrain).
pub fn lowest() {
    set(-2);
}

/// Asks Windows not to power-throttle this process (EcoQoS): without it a
/// process in the background, such as a benchmark run from a console, can
/// have its threads held to low clock speeds.
pub fn no_power_throttling() {
    #[cfg(windows)]
    {
        // ProcessPowerThrottling, controlling EXECUTION_SPEED, turned off.
        let state = ffi::PowerThrottlingState { version: 1, control_mask: 1, state_mask: 0 };
        // SAFETY: the pseudo handle of this process and a correctly sized state.
        unsafe {
            ffi::SetProcessInformation(ffi::GetCurrentProcess(), 4, &state as *const _ as *const std::ffi::c_void, std::mem::size_of::<ffi::PowerThrottlingState>() as u32);
        }
    }
}

/// `IDLE_PRIORITY_CLASS` for the whole process: benchmarks and tools that
/// must not take CPU time from anything else running.
pub fn idle_process() {
    #[cfg(windows)]
    {
        // SAFETY: the pseudo handle of this process.
        unsafe {
            ffi::SetPriorityClass(ffi::GetCurrentProcess(), 0x40);
        }
    }
}

/// `THREAD_PRIORITY_ABOVE_NORMAL`, for the render thread.
pub fn interactive() {
    set(1);
}

/// Coordinate the three independent terrain pools instead of each claiming
/// most hardware threads. Preserve time for presentation, simulation and audio;
/// existing per-pool environment overrides still apply at their call sites.
pub fn terrain_workers(logical:usize)->[usize;3] {
    let logical=logical.max(1);
    [(logical.saturating_sub(2)/2).max(1),(logical/4).max(1),(logical/6).clamp(1,4)]
}

#[cfg(test)]mod terrain_worker_tests {
    use super::*;
    #[test]fn background_terrain_pools_leave_interactive_capacity(){
        for threads in 4..=256 {
            let pools=terrain_workers(threads);
            assert!(pools.iter().all(|&n|n>0));
            assert!(pools.iter().sum::<usize>()<threads,"{threads}: {pools:?}");
            assert!(pools[2]<=4,"far generation is secondary to real chunks");
        }
        for threads in 0..4 {assert!(terrain_workers(threads).iter().all(|&n|n>0));}
    }
}
