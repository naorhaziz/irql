//! Per-level auto traits for compile-time drop safety.
//!
//! Each auto trait is automatically implemented for any type whose fields all
//! implement it. Types unsafe to drop at a given IRQL opt out with negative
//! impls (e.g. `impl !SafeToDropAtDispatch for PagedPool {}`).

use super::levels::*;

macro_rules! define_drop_safety_traits {
    ($(($level:ident, $safe_trait:ident)),+) => {
        $(
            #[doc = concat!(
                "Memory owned by this type can be safely freed at **",
                stringify!($level), "** IRQL.\n",
                "\n",
                "This is an [auto trait]: it is automatically implemented for any\n",
                "type whose fields all implement it. Types that are unsafe to drop\n",
                "at this IRQL opt out with a negative impl.\n",
                "\n",
                "# Safety\n",
                "\n",
                "This is an auto trait — do not implement it manually. Opt out with\n",
                "a negative impl (`impl !",  stringify!($safe_trait), " for MyType {}`).\n",
                "\n",
                "[auto trait]: https://doc.rust-lang.org/reference/special-types-and-traits.html#auto-traits",
            )]
            #[diagnostic::on_unimplemented(
                message = "`{Self}` cannot be safely dropped at this IRQL",
                label   = "not safe to drop here",
                note    = "this type (or a type it contains) has opted out via a negative impl.\nConsider passing by reference (`&`) or transferring ownership without dropping (e.g. `leak()` / `into_raw()`).",
            )]
            pub unsafe auto trait $safe_trait {}

            impl<T: $safe_trait> SafeToDropAt<$level> for T {}
        )+
    };
}

define_drop_safety_traits!(
    (Passive, SafeToDropAtPassive),
    (Apc, SafeToDropAtApc),
    (Dispatch, SafeToDropAtDispatch),
    (Dirql, SafeToDropAtDirql),
    (Profile, SafeToDropAtProfile),
    (Clock, SafeToDropAtClock),
    (Ipi, SafeToDropAtIpi),
    (Power, SafeToDropAtPower),
    (High, SafeToDropAtHigh)
);
