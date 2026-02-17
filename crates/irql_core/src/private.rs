//! Sealed trait to prevent external implementations of IRQL level types.
//!
//! This pattern ensures that only the types defined in [`crate::levels`] can
//! implement [`IrqlLevel`](crate::IrqlLevel), preventing downstream crates
//! from introducing invalid IRQL levels.

/// Sealed supertrait — cannot be implemented outside this crate.
pub trait Sealed {}
