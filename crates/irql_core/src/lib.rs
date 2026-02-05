//! Core types and traits for IRQL safety.
//!
//! This is an internal crate. Use the `irql` crate instead.

#![no_std]
#![warn(missing_docs)]

mod private {
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
