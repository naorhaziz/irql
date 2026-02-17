use crate::private;

/// Marker trait implemented by all IRQL level types.
///
/// Every IRQL level is a zero-sized type that implements this trait.
///
/// # Hierarchy (lowest → highest)
///
/// `Passive(0)` → `Apc(1)` → `Dispatch(2)` → `Dirql(3–26)` → `Profile(27)`
/// → `Clock(28)` → `Ipi(29)` → `Power(30)` → `High(31)`
///
/// The hierarchy is enforced at compile time through [`IrqlCanRaiseTo`] and
/// [`IrqlCanLowerTo`].
pub trait IrqlLevel: private::Sealed {}

/// The current IRQL can be raised to `Target`.
///
/// Holds when `Self <= Target` in the IRQL hierarchy.
///
/// # Example
///
/// ```ignore
/// // Passive can raise to Dispatch (impl exists):
/// fn ok<I: IrqlCanRaiseTo<Dispatch>>() {}
///
/// // Dispatch cannot raise to Passive (no impl — compile error):
/// // fn bad<I: IrqlCanRaiseTo<Passive>>() where Dispatch: IrqlCanRaiseTo<Passive> {}
/// ```
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
///
/// # Example
///
/// ```ignore
/// // Dispatch is at or above Passive (impl exists):
/// fn ok<I: IrqlCanLowerTo<Passive>>() where Dispatch: IrqlCanLowerTo<Passive> {}
///
/// // Passive is not at or above Dispatch (no impl — compile error):
/// // fn bad<I: IrqlCanLowerTo<Dispatch>>() where Passive: IrqlCanLowerTo<Dispatch> {}
/// ```
#[diagnostic::on_unimplemented(
    message = "IRQL violation: `{Self}` is below the required minimum `{Target}`",
    label = "IRQL too low",
    note = "this operation requires IRQL >= `{Target}`"
)]
pub trait IrqlCanLowerTo<Target: IrqlLevel>: private::Sealed {}

/// `Self` can be safely dropped at IRQL `Level`.
///
/// With the `drop-safety` feature, this is backed by per-level auto traits
/// that propagate through struct fields automatically. Without it, this is
/// blanket-implemented for all types (no-op).
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be safely dropped at IRQL `{Level}`",
    label = "not safe to drop at this IRQL",
    note = "this type (or a type it contains) has opted out via a negative impl.\nConsider passing by reference (`&`) or transferring ownership without dropping (e.g. `leak()` / `into_raw()`)."
)]
pub trait SafeToDropAt<Level: IrqlLevel> {}

macro_rules! define_irql_hierarchy {
    ($($level:ident),+) => {
        $(
            #[doc = concat!(
                "IRQL level: **", stringify!($level), "**.\n",
                "\n",
                "See [`IrqlLevel`] for the full hierarchy.",
            )]
            pub struct $level;

            impl private::Sealed for $level {}
            impl IrqlLevel for $level {}
        )+

        define_irql_hierarchy!(@raise_all $($level),+);
        define_irql_hierarchy!(@lower_all $($level),+);
    };

    // Each level can raise to itself and all higher levels.
    (@raise_all $head:ident, $($tail:ident),+) => {
        impl IrqlCanRaiseTo<$head> for $head {}
        $( impl IrqlCanRaiseTo<$tail> for $head {} )+
        define_irql_hierarchy!(@raise_all $($tail),+);
    };
    (@raise_all $last:ident) => {
        impl IrqlCanRaiseTo<$last> for $last {}
    };

    // Each level can lower to itself and all lower levels.
    (@lower_all $head:ident, $($tail:ident),+) => {
        impl IrqlCanLowerTo<$head> for $head {}
        $( impl IrqlCanLowerTo<$head> for $tail {} )+
        define_irql_hierarchy!(@lower_all $($tail),+);
    };
    (@lower_all $last:ident) => {
        impl IrqlCanLowerTo<$last> for $last {}
    };
}

define_irql_hierarchy!(
    Passive, Apc, Dispatch, Dirql, Profile, Clock, Ipi, Power, High
);

// Without drop-safety: blanket impl so SafeToDropAt<L> bounds are trivially satisfied.
#[cfg(not(feature = "drop-safety"))]
impl<T, L: IrqlLevel> SafeToDropAt<L> for T {}
