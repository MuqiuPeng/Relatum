//! Lifecycle, mode, and budget — the macro and micro state of the
//! autonomous runtime, plus the per-tick action budget. ADR 0052.

/// Macro state of the runtime. ADR 0052.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Booting,
    Running,
    Sleeping,
    Stopped,
}

/// Micro state within Running — what kind of work the runtime is
/// doing. Phase A0/A1 only uses `Expand`. ADR 0052.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeMode {
    Expand,
    Consolidate,
    Reflect,
}

/// Two-dimensional budget (step counts only; no wall-clock). ADR 0052.
#[derive(Debug, Clone, Copy)]
pub struct BudgetState {
    pub ticks_remaining: Option<u64>,
    pub actions_remaining_this_tick: u32,
    pub actions_per_tick_cap: u32,
}

impl BudgetState {
    pub fn new(actions_per_tick_cap: u32) -> Self {
        Self {
            ticks_remaining: None,
            actions_remaining_this_tick: actions_per_tick_cap,
            actions_per_tick_cap,
        }
    }

    pub(crate) fn reset_per_tick(&mut self) {
        self.actions_remaining_this_tick = self.actions_per_tick_cap;
    }
}
