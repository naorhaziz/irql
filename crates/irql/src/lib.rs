//! Compile-time IRQL (Interrupt Request Level) safety for Windows kernel drivers.
//!
//! This crate provides compile-time verification of IRQL constraints using Rust's
//! type system. IRQL violations are caught by the compiler, preventing a common
//! class of bugs in Windows kernel-mode code.
//!
//! # Quick Start
//!
//! ```no_run
//! use irql::{requires_irql, root_irql, Dispatch, Passive};
//!
//! // Function requiring Dispatch IRQL or higher
//! #[requires_irql(Dispatch)]
//! fn acquire_spinlock() {
//!     // Spinlock operations require Dispatch level
//! }
//!
//! // Entry point at Passive IRQL
//! #[root_irql(Passive)]
//! fn driver_init() {
//!     // Can call higher IRQL functions using call_irql!
//!     call_irql!(acquire_spinlock());
//! }
//! ```
//!
//! # IRQL Levels
//!
//! The crate provides types for each Windows IRQL level:
//!
//! - [`Passive`] - Normal kernel execution (can access paged memory)
//! - [`Apc`] - Asynchronous Procedure Call level
//! - [`Dispatch`] - DPC level (most common for drivers)
//! - [`Dirql`] - Device interrupt level
//! - [`Profile`], [`Clock`], [`Ipi`], [`Power`], [`High`] - Higher interrupt levels
//!
//! # The IRQL Rule
//!
//! **IRQL can only stay the same or be raised, never lowered.**
//!
//! This fundamental rule is enforced at compile time through the [`IrqlCanRaiseTo`] trait.
//! Attempting to call a lower-IRQL function from a higher-IRQL context results in a
//! compile error with a clear diagnostic message.
//!
//! # Attributes and Macros
//!
//! - [`requires_irql`] - Mark functions requiring a minimum IRQL
//! - [`root_irql`] - Mark entry points with their IRQL level
//! - [`fn_trait_irql_requires`] - Implement IRQL-safe function traits
//! - `call_irql!` - Call IRQL-constrained functions (available inside annotated functions)
//!
//! # Example: Struct Methods
//!
//! ```no_run
//! use irql::{requires_irql, root_irql, Dispatch};
//!
//! struct Device {
//!     id: u32,
//! }
//!
//! #[requires_irql(Dispatch)]
//! impl Device {
//!     fn new(id: u32) -> Self {
//!         Device { id }
//!     }
//!     
//!     fn process_interrupt(&self) {
//!         // All methods require Dispatch IRQL
//!     }
//! }
//!
//! #[root_irql(Dispatch)]
//! fn interrupt_handler() {
//!     let device = call_irql!(Device::new(1));
//!     call_irql!(device.process_interrupt());
//! }
//! ```
//!
//! # IRQL-Safe Function Traits
//!
//! The crate provides IRQL-aware function traits that mirror Rust's standard function traits:
//!
//! - [`IrqlFn`] - Immutable function calls (like `Fn`)
//! - [`IrqlFnMut`] - Mutable function calls (like `FnMut`)
//! - [`IrqlFnOnce`] - One-time consumption (like `FnOnce`)
//! - [`IrqlAsyncFn`] - Async immutable calls
//! - [`IrqlAsyncFnMut`] - Async mutable calls
//!
//! Use [`fn_trait_irql_requires`] to implement these traits with compile-time IRQL safety:
//!
//! ```no_run
//! use irql::{fn_trait_irql_requires, IrqlFn, IrqlFnMut, Passive, root_irql};
//!
//! // Immutable function object
//! struct Reader { value: u32 }
//!
//! #[fn_trait_irql_requires(Passive)]
//! impl IrqlFn<()> for Reader {
//!     type Output = u32;
//!     fn call(&self, _args: ()) -> u32 {
//!         self.value
//!     }
//! }
//!
//! // Mutable function object
//! struct Counter { count: u32 }
//!
//! #[fn_trait_irql_requires(Passive)]
//! impl IrqlFnMut<()> for Counter {
//!     type Output = u32;
//!     fn call_mut(&mut self, _args: ()) -> u32 {
//!         self.count += 1;
//!         self.count
//!     }
//! }
//!
//! // Usage
//! #[root_irql(Passive)]
//! fn example() {
//!     let reader = Reader { value: 42 };
//!     let mut counter = Counter { count: 0 };
//!
//!     let val = call_irql!(reader.call(()));
//!     let count = call_irql!(counter.call_mut(()));
//! }
//! ```
//!
//! # Compile-Time Error Example
//!
//! ```compile_fail
//! use irql::{requires_irql, Dispatch, Passive};
//!
//! #[requires_irql(Passive)]
//! fn low_irql() { }
//!
//! #[requires_irql(Dispatch)]
//! fn high_irql() {
//!     call_irql!(low_irql()); // ERROR: Cannot lower IRQL!
//! }
//! ```
//!
//! The compiler produces:
//! ```text
//! error: IRQL violation: cannot call function at `Passive` IRQL
//!        from current IRQL level `Dispatch`
//! ```
//!
//! # Safety
//!
//! This crate provides compile-time guarantees only. You must ensure:
//! - Entry points are annotated with their actual runtime IRQL
//! - IRQL-raising operations (spinlocks, etc.) are properly tracked
//! - Runtime IRQL matches compile-time annotations
//!
//! For more details, see the [repository](https://github.com/naorhaziz/irql).

#![no_std]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

// Re-export IRQL level types
pub use irql_core::{
    Apc, Clock, Dirql, Dispatch, High, Ipi, IrqlCanRaiseTo, IrqlLevel, Passive, Power, Profile,
};

// Re-export function traits
pub use irql_core::{IrqlAsyncFn, IrqlAsyncFnMut, IrqlFn, IrqlFnMut, IrqlFnOnce};

// Re-export macros
pub use irql_macro::call_irql_inner;

/// Marks a function or impl block as requiring a minimum IRQL level.
///
/// # Example
///
/// ```no_run
/// use irql::{requires_irql, Dispatch};
///
/// #[requires_irql(Dispatch)]
/// fn acquire_spinlock() {
///     // Must be called at Dispatch IRQL or higher
/// }
/// ```
pub use irql_macro::requires_irql;

/// Marks an entry point function with a specific IRQL context.
///
/// # Example
///
/// ```no_run
/// use irql::{root_irql, Passive};
///
/// #[root_irql(Passive)]
/// fn driver_entry() {
///     // Entry point at Passive IRQL
/// }
/// ```
pub use irql_macro::root_irql;

/// Implements IRQL-safe function traits with compile-time safety guarantees.
///
/// Works with: `IrqlFn`, `IrqlFnMut`, `IrqlFnOnce`, `IrqlAsyncFn`, `IrqlAsyncFnMut`.
///
/// # Example
/// ```ignore
/// #[fn_trait_irql_requires(Passive)]
/// impl IrqlFn<()> for MyType {
///     type Output = u32;
///     fn call(&self, _args: ()) -> u32 { self.value }
/// }
/// ```
pub use irql_macro::fn_trait_irql_requires;
