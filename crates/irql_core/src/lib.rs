//! Core types and traits for IRQL safety.
//!
//! This is an internal crate. Use the `irql` crate instead.

#![no_std]
#![warn(missing_docs)]

/// Private module to prevent external implementations of sealed traits.
mod private {
    /// Sealed trait to prevent external implementations.
    pub trait Sealed {}
}

/// Marker trait for IRQL level types.
pub trait IrqlLevel: private::Sealed {}

/// Trait indicating that an IRQL level can be raised to a target level.
///
/// This trait is automatically implemented for valid IRQL transitions.
/// IRQL can only stay the same or be raised, never lowered.
#[diagnostic::on_unimplemented(
    message = "IRQL violation: cannot call function at `{Target}` IRQL from current IRQL level `{Self}`",
    label = "cannot lower IRQL or call incompatible IRQL level",
    note = "IRQL can only stay the same or be raised, never lowered"
)]
pub trait IrqlCanRaiseTo<Target: IrqlLevel>: private::Sealed {}

macro_rules! define_irql_hierarchy {
    ($($level:ident),*) => {
        $(
            #[doc = concat!("IRQL level: ", stringify!($level))]
            #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $level;

            impl private::Sealed for $level {}
            impl IrqlLevel for $level {}
        )*

        define_irql_hierarchy!(@rules $($level),*);
    };

    (@rules $current:ident, $($rest:ident),*) => {
        impl IrqlCanRaiseTo<$current> for $current {}

        $(
            impl IrqlCanRaiseTo<$rest> for $current {}
        )*

        define_irql_hierarchy!(@rules $($rest),*);
    };

    (@rules $current:ident) => {
        impl IrqlCanRaiseTo<$current> for $current {}
    };
}

define_irql_hierarchy!(
    Passive, Apc, Dispatch, Dirql, Profile, Clock, Ipi, Power, High
);

/// A function trait that is safe to call at a specific IRQL level.
///
/// This trait is similar to `Fn`, but with IRQL safety guarantees.
/// The function can only be called from an IRQL level that can raise to `Level`.
pub trait IrqlFn<Level: IrqlLevel, Args> {
    /// The return type of the function.
    type Output;

    /// Call the function with IRQL safety.
    ///
    /// # Safety
    /// This method is only callable when `IRQL` can raise to `Level`.
    fn call<IRQL>(&self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level>;
}

/// A mutable function trait that is safe to call at a specific IRQL level.
///
/// This trait is similar to `FnMut`, but with IRQL safety guarantees.
pub trait IrqlFnMut<Level: IrqlLevel, Args> {
    /// The return type of the function.
    type Output;

    /// Call the function mutably with IRQL safety.
    ///
    /// # Safety
    /// This method is only callable when `IRQL` can raise to `Level`.
    fn call_mut<IRQL>(&mut self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level>;
}

/// A once-callable function trait that is safe to call at a specific IRQL level.
///
/// This trait is similar to `FnOnce`, but with IRQL safety guarantees.
pub trait IrqlFnOnce<Level: IrqlLevel, Args> {
    /// The return type of the function.
    type Output;

    /// Call the function, consuming it, with IRQL safety.
    ///
    /// # Safety
    /// This method is only callable when `IRQL` can raise to `Level`.
    fn call_once<IRQL>(self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level>;
}

/// An async function trait that is safe to call at a specific IRQL level.
///
/// This trait represents async functions with IRQL safety guarantees.
/// Note: async operations are typically only safe at `Passive` or `Apc` IRQL.
pub trait IrqlAsyncFn<Level: IrqlLevel, Args> {
    /// The future type returned by the async function.
    type Future: core::future::Future<Output = Self::Output>;

    /// The return type of the async function.
    type Output;

    /// Call the async function.
    ///
    /// # Safety
    /// This method is only callable when `IRQL` can raise to `Level`.
    fn call_async<IRQL>(&self, args: Args) -> Self::Future
    where
        IRQL: IrqlCanRaiseTo<Level>;
}

/// An async mutable function trait that is safe to call at a specific IRQL level.
pub trait IrqlAsyncFnMut<Level: IrqlLevel, Args>: IrqlAsyncFn<Level, Args> {
    /// Call the async function mutably.
    ///
    /// # Safety
    /// This method is only callable when `IRQL` can raise to `Level`.
    fn call_async_mut<IRQL>(&mut self, args: Args) -> Self::Future
    where
        IRQL: IrqlCanRaiseTo<Level>;
}
