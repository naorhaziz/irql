//! Compile-time IRQL safety for Windows kernel drivers.
//!
//! IRQL violations are caught at compile time using Rust's type system —
//! zero runtime cost, zero binary size overhead.
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
//! Each level is a zero-sized marker type implementing [`IrqlLevel`].
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
//! # IRQL-aware allocation (`alloc` feature)
//!
//! Enable with `irql = { features = ["alloc"] }`.
//!
//! **Requires a nightly Rust compiler** — depends on unstable
//! `allocator_api`, `vec_push_within_capacity`, `auto_traits`, and
//! `negative_impls`. Add a `rust-toolchain.toml` with
//! `channel = "nightly"` to your project.
//!
//! Pool allocations automatically use `ExAllocatePool2` / `ExFreePool`
//! from [`wdk-sys`](https://crates.io/crates/wdk-sys) in WDM/KMDF driver
//! builds. Outside a WDK build (e.g. testing), the global allocator is
//! used as a fallback.
//!
//! `IrqlBox` and `IrqlVec` enforce pool rules at compile time:
//!
//! ```ignore
//! #[irql(max = Passive)]
//! fn example() -> Result<(), AllocError> {
//!     let data = call_irql!(IrqlBox::new(42))?;  // PagedPool (automatic)
//!     let val = call_irql!(data.get());
//!
//!     let v = irql_vec![1, 2, 3]?;
//!     Ok(())
//! }
//! ```
//!
//! ## FFI interop
//!
//! `IrqlBox` provides raw pointer methods for passing allocations to
//! kernel APIs and C callbacks:
//!
//! ```ignore
//! let b = call_irql!(IrqlBox::new(data))?;
//! let ptr = b.into_raw();  // pass to kernel API
//! // later, reconstruct:
//! let b = unsafe { IrqlBox::<_, PagedPool>::from_raw(ptr) };
//! ```
//!
//! # Drop safety
//!
//! Paged-pool containers (`IrqlBox<T, PagedPool>`, `IrqlVec<T, PagedPool>`)
//! must not be dropped at `Dispatch` or above.  This is enforced at compile
//! time via `SafeToDropAt*` auto traits (enabled automatically with `alloc`).
//! The `#[irql]` macro injects `T: SafeToDropAt<Level>` bounds on by-value
//! parameters, so passing a paged-pool value (or any struct containing one)
//! by value into code at `Dispatch`+ is a compile error.
//!
//! References (`&IrqlBox`) are *not* gated — they don't trigger a drop.
//! Use `IrqlBox::leak()` or `IrqlBox::into_raw()` when you need to transfer
//! ownership across an IRQL boundary.
//!
//! # Safety
//!
//! All checks are compile-time only. You must ensure:
//! - Entry points (`#[irql(at = …)]`) match the actual runtime IRQL.
//! - IRQL-raising operations (spinlocks, etc.) are properly modelled.

#![no_std]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(feature = "alloc", feature(allocator_api))]

// Re-export IRQL level types and hierarchy traits.
pub use irql_core::{
    Apc, Clock, Dirql, Dispatch, High, Ipi, IrqlCanLowerTo, IrqlCanRaiseTo, IrqlLevel, Passive,
    Power, Profile,
};

// SafeToDropAt<L> is always available (blanket-impl'd without `drop-safety`).
pub use irql_core::SafeToDropAt;

// Per-level auto traits (only with `drop-safety` / `alloc`).
#[cfg(feature = "drop-safety")]
pub use irql_core::{
    SafeToDropAtApc, SafeToDropAtClock, SafeToDropAtDirql, SafeToDropAtDispatch, SafeToDropAtHigh,
    SafeToDropAtIpi, SafeToDropAtPassive, SafeToDropAtPower, SafeToDropAtProfile,
};

// IRQL-safe function traits.
pub use irql_core::{IrqlFn, IrqlFnMut, IrqlFnOnce};

#[doc(hidden)]
pub use irql_macro::call_irql_inner;

/// Compile-time IRQL constraint.
///
/// Annotate functions and impl blocks to enforce IRQL rules at compile time.
///
/// # Forms
///
/// | Syntax | Meaning |
/// |--------|--------|
/// | `#[irql(at = Passive)]` | Fixed entry point (no generic added) |
/// | `#[irql(max = Dispatch)]` | Callable from Dispatch or below |
/// | `#[irql(min = Apc, max = Dispatch)]` | Callable in \[Apc, Dispatch\] |
///
/// `max` is **required** unless using `at`. `min` is optional.
///
/// # Supported targets
///
/// - **Functions** — adds an `IRQL` generic type parameter.
/// - **Inherent impl blocks** — each method gets its own `IRQL` generic.
/// - **Trait impl blocks** — e.g. `impl IrqlFn<()> for T` — the macro
///   rewrites the trait's generic arguments and constrains each method.
///
/// # How `call_irql!` works
///
/// Inside an `#[irql]` body, a local `call_irql!` macro is injected that
/// rewrites `call_irql!(f(args))` into `f::<IRQL>(args)`, threading the
/// IRQL type through every call in the chain.
pub use irql_macro::irql;

// IRQL-aware allocator types (optional `alloc` feature, requires nightly).
#[cfg(feature = "alloc")]
pub use irql_alloc::{
    AccessibleAt, AllocError, AllocableAt, DefaultPool, IrqlBox, IrqlVec, NonPagedPool, PagedPool,
    PoolAlloc, PoolAllocator, TryReserveError, irql_vec,
};
