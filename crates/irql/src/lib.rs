//! Compile-time IRQL safety for Windows kernel drivers.
//!
//! IRQL violations are caught at compile time using Rust's type system —
//! zero runtime cost.
//!
//! # Quick start
//!
//! ```no_run
//! use irql::{irql, Dispatch, Passive};
//!
//! #[irql(max = Dispatch)]
//! fn acquire_spinlock() { /* … */ }
//!
//! #[irql(at = Passive)]
//! fn driver_entry() {
//!     call_irql!(acquire_spinlock());
//! }
//! ```
//!
//! # The `#[irql()]` attribute
//!
//! | Form | Meaning |
//! |------|---------|
//! | `#[irql(at = Level)]` | Fixed entry point — known IRQL, no generic |
//! | `#[irql(max = Level)]` | Callable from `Level` or below |
//! | `#[irql(min = A, max = B)]` | Callable in the range \[A, B\] |
//!
//! `max` is **required** unless using `at` — it defines the ceiling that
//! `call_irql!` relies on. `min` is optional and adds a floor constraint.
//! `at` is mutually exclusive with `min`/`max`.
//!
//! Works on **functions**, **inherent impl blocks**, and **trait impl blocks**.
//!
//! # IRQL levels
//!
//! | Value | Type | Description |
//! |-------|------|-------------|
//! | 0 | [`Passive`] | Normal thread execution; paged memory OK |
//! | 1 | [`Apc`] | APC delivery |
//! | 2 | [`Dispatch`] | DPC / spinlock level |
//! | 3–26 | [`Dirql`] | Device interrupt levels |
//! | 27 | [`Profile`] | Profiling timer |
//! | 28 | [`Clock`] | Clock interrupt |
//! | 29 | [`Ipi`] | Inter-processor interrupt |
//! | 30 | [`Power`] | Power failure |
//! | 31 | [`High`] | Highest — machine check |
//!
//! # The golden rule
//!
//! **IRQL can only stay the same or be raised, never lowered.**
//!
//! Attempting to call a lower-IRQL function produces a compile error:
//!
//! ```compile_fail
//! use irql::{irql, Dispatch, Passive};
//!
//! #[irql(max = Passive)]
//! fn passive_only() {}
//!
//! #[irql(max = Dispatch)]
//! fn at_dispatch() {
//!     call_irql!(passive_only()); // ERROR: cannot lower IRQL
//! }
//! ```
//!
//! # Safety
//!
//! All checks are compile-time only. You must ensure:
//! - Entry points (`#[irql(at = …)]`) match the actual runtime IRQL.
//! - IRQL-raising operations (spinlocks, etc.) are properly modelled.

#![no_std]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

// Re-export IRQL level types
pub use irql_core::{
    Apc, Clock, Dirql, Dispatch, High, Ipi, IrqlCanLowerTo, IrqlCanRaiseTo, IrqlLevel, Passive,
    Power, Profile,
};

// Re-export function traits
pub use irql_core::{IrqlFn, IrqlFnMut, IrqlFnOnce};

#[doc(hidden)]
pub use irql_macro::call_irql_inner;

/// Compile-time IRQL constraint.
///
/// # Forms
///
/// | Syntax | Meaning |
/// |--------|--------|
/// | `#[irql(at = Passive)]` | Fixed entry point (no generic) |
/// | `#[irql(max = Dispatch)]` | Callable from Dispatch or below |
/// | `#[irql(min = Apc, max = Dispatch)]` | Callable in \[Apc, Dispatch\] |
///
/// `max` is **required** unless using `at`. `min` is optional.
///
/// Applies to functions, inherent impl blocks, and trait impl blocks.
pub use irql_macro::irql;
