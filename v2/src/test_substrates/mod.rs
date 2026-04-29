//! Shared test substrate streams used across examples.
//!
//! Until 2026-04-29, OQ#1 and long5k stream constructors were
//! copy-pasted into ~17 example files. ADR 0066 / Phase Beta C.1
//! consolidates them here as a single source of truth.
//!
//! These streams are deterministic; reproducibility across runs is
//! guaranteed by the fixed event schedule.

pub mod oq1;
pub mod long5k;
