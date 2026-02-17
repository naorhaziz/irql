use crate::{IrqlCanLowerTo, IrqlCanRaiseTo, IrqlLevel, Passive};

/// IRQL-safe analogue of [`Fn`] — callable multiple times via shared reference.
///
/// Callable from any IRQL in the range \[`Min`, `Level`\]:
/// - The caller's IRQL must be ≤ `Level` (can raise to it).
/// - The caller's IRQL must be ≥ `Min` (is at or above the floor).
///
/// `Min` defaults to [`Passive`] (no floor) when omitted.
///
/// # Implementing
///
/// Use the `#[irql]` attribute on the trait impl block — the macro fills in
/// the `Level` (and `Min`) type parameters automatically:
///
/// ```ignore
/// #[irql(max = Dispatch)]
/// impl IrqlFn<()> for MyReader {
///     type Output = u32;
///     fn call(&self, _: ()) -> u32 { self.value }
/// }
/// // Expands to: impl IrqlFn<Dispatch, ()> for MyReader { … }
/// ```
pub trait IrqlFn<Level: IrqlLevel, Args, Min: IrqlLevel = Passive> {
    /// The return type produced by this callable.
    type Output;

    /// Call the function. Only compiles when the caller's IRQL satisfies both bounds.
    fn call<IRQL>(&self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level> + IrqlCanLowerTo<Min>;
}

/// IRQL-safe analogue of [`FnMut`] — callable multiple times via mutable reference.
///
/// Same IRQL semantics as [`IrqlFn`], but takes `&mut self`.
///
/// ```ignore
/// #[irql(max = Passive)]
/// impl IrqlFnMut<()> for Counter {
///     type Output = u32;
///     fn call_mut(&mut self, _: ()) -> u32 {
///         self.count += 1;
///         self.count
///     }
/// }
/// ```
pub trait IrqlFnMut<Level: IrqlLevel, Args, Min: IrqlLevel = Passive> {
    /// The return type produced by this callable.
    type Output;

    /// Call the function mutably.
    fn call_mut<IRQL>(&mut self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level> + IrqlCanLowerTo<Min>;
}

/// IRQL-safe analogue of [`FnOnce`] — callable exactly once, consuming `self`.
///
/// Same IRQL semantics as [`IrqlFn`], but takes `self` by value.
///
/// ```ignore
/// #[irql(max = Dispatch)]
/// impl IrqlFnOnce<()> for Message {
///     type Output = String;
///     fn call_once(self, _: ()) -> String { self.0 }
/// }
/// ```
pub trait IrqlFnOnce<Level: IrqlLevel, Args, Min: IrqlLevel = Passive> {
    /// The return type produced by this callable.
    type Output;

    /// Call the function, consuming it.
    fn call_once<IRQL>(self, args: Args) -> Self::Output
    where
        IRQL: IrqlCanRaiseTo<Level> + IrqlCanLowerTo<Min>;
}
