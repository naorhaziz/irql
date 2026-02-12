//! Core types and traits for compile-time IRQL safety.
//!
//! **Internal crate** — use [`irql`](https://docs.rs/irql) instead.

#![no_std]
#![warn(missing_docs)]

mod private {
    pub trait Sealed {}
}

/// Marker trait implemented by all IRQL level types.
pub trait IrqlLevel: private::Sealed {}

/// The current IRQL can be raised to `Target`.
///
/// Holds when `Self <= Target` in the IRQL hierarchy.
/// Implemented automatically for all valid transitions.
#[diagnostic::on_unimplemented(
    message = "IRQL violation: cannot reach `{Target}` from `{Self}` -- would require lowering",
    label = "cannot lower IRQL",
    note = "IRQL can only stay the same or be raised, never lowered"
)]
pub trait IrqlCanRaiseTo<Target: IrqlLevel>: private::Sealed {}

/// The current IRQL is at or above `Target`.
///
/// Holds when `Self >= Target` in the IRQL hierarchy.
/// Used by `#[irql(min = Level)]` to enforce a floor constraint.
#[diagnostic::on_unimplemented(
    message = "IRQL violation: `{Self}` is below the required minimum `{Target}`",
    label = "IRQL too low",
    note = "this operation requires IRQL >= `{Target}`"
)]
pub trait IrqlCanLowerTo<Target: IrqlLevel>: private::Sealed {}

macro_rules! define_irql_hierarchy {
    ($($level:ident),+) => {
        $(
            #[doc = concat!("IRQL level: `", stringify!($level), "`.")]
            #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $level;
            impl private::Sealed for $level {}
            impl IrqlLevel for $level {}
        )+
        define_irql_hierarchy!(@raise $($level),+);
        define_irql_hierarchy!(@lower $($level),+);
    };

    // Each level can raise to itself and all higher levels.
    (@raise $head:ident, $($tail:ident),+) => {
        impl IrqlCanRaiseTo<$head> for $head {}
        $( impl IrqlCanRaiseTo<$tail> for $head {} )+
        define_irql_hierarchy!(@raise $($tail),+);
    };
    (@raise $last:ident) => {
        impl IrqlCanRaiseTo<$last> for $last {}
    };

    // Each level can lower to itself and all lower levels.
    (@lower $head:ident, $($tail:ident),+) => {
        impl IrqlCanLowerTo<$head> for $head {}
        $( impl IrqlCanLowerTo<$head> for $tail {} )+
        define_irql_hierarchy!(@lower $($tail),+);
    };
    (@lower $last:ident) => {
        impl IrqlCanLowerTo<$last> for $last {}
    };
}

define_irql_hierarchy!(
    Passive, Apc, Dispatch, Dirql, Profile, Clock, Ipi, Power, High
);

/// IRQL-safe analogue of [`Fn`].
///
/// Callable from any IRQL in the range \[`Min`, `Level`\].
/// `Min` defaults to [`Passive`] (no floor) when omitted.
pub trait IrqlFn<Level: IrqlLevel, Args, Min: IrqlLevel = Passive> {
    /// The return type.
    type Output;

    /// Call the function. Only compiles when the caller's IRQL satisfies both bounds.
    fn call<IRQL>(&self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level> + IrqlCanLowerTo<Min>;
}

/// IRQL-safe analogue of [`FnMut`]. See [`IrqlFn`] for details.
pub trait IrqlFnMut<Level: IrqlLevel, Args, Min: IrqlLevel = Passive> {
    /// The return type.
    type Output;

    /// Call the function mutably.
    fn call_mut<IRQL>(&mut self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level> + IrqlCanLowerTo<Min>;
}

/// IRQL-safe analogue of [`FnOnce`]. See [`IrqlFn`] for details.
pub trait IrqlFnOnce<Level: IrqlLevel, Args, Min: IrqlLevel = Passive> {
    /// The return type.
    type Output;

    /// Call the function, consuming it.
    fn call_once<IRQL>(self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level> + IrqlCanLowerTo<Min>;
}
