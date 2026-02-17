//! Core types and traits for compile-time IRQL safety.
//!
//! **Internal crate** — use [`irql`](https://docs.rs/irql) instead.
//!
//! This crate provides:
//!
//! - **IRQL level types**: [`Passive`], [`Apc`], [`Dispatch`], [`Dirql`],
//!   [`Profile`], [`Clock`], [`Ipi`], [`Power`], [`High`] — zero-sized marker
//!   types.
//!
//! - **Hierarchy traits**: [`IrqlCanRaiseTo`] and [`IrqlCanLowerTo`] encode
//!   which transitions are valid. Invalid transitions produce a compile error
//!   with a clear diagnostic message.
//!
//! - **Drop-safety auto traits** (`drop-safety` feature, requires nightly):
//!   `SafeToDropAtPassive`, `SafeToDropAtDispatch`, etc. — one per IRQL level.
//!   Types that are unsafe to drop above a certain IRQL opt out with negative
//!   impls (e.g. `PagedPool` in `irql_alloc`). Without `drop-safety`,
//!   [`SafeToDropAt`] is blanket-implemented for all types.
//!
//! - **Function traits**: [`IrqlFn`], [`IrqlFnMut`], [`IrqlFnOnce`] — IRQL-safe
//!   analogues of [`Fn`], [`FnMut`], [`FnOnce`].

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "drop-safety", feature(auto_traits))]

mod function_traits;
mod levels;
mod private;

#[cfg(feature = "drop-safety")]
mod drop_safety;

pub use function_traits::{IrqlFn, IrqlFnMut, IrqlFnOnce};
pub use levels::{
    Apc, Clock, Dirql, Dispatch, High, Ipi, IrqlCanLowerTo, IrqlCanRaiseTo, IrqlLevel, Passive,
    Power, Profile, SafeToDropAt,
};

#[cfg(feature = "drop-safety")]
pub use drop_safety::{
    SafeToDropAtApc, SafeToDropAtClock, SafeToDropAtDirql, SafeToDropAtDispatch, SafeToDropAtHigh,
    SafeToDropAtIpi, SafeToDropAtPassive, SafeToDropAtPower, SafeToDropAtProfile,
};
