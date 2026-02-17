//! Basic IRQL constraint examples.
//!
//! Demonstrates `#[irql(max = ...)]`, `#[irql(min = ..., max = ...)]`,
//! and `#[irql(at = ...)]` on standalone functions.

use irql::*;

// Callable from Dispatch or below.
#[irql(max = Dispatch)]
fn dispatch_work() {}

// Callable from Passive or below.
#[irql(max = Passive)]
fn passive_work() {
    // Passive can raise to Dispatch
    call_irql!(dispatch_work());
}

// Callable from Passive through Dispatch (not Dirql+).
#[irql(min = Passive, max = Dispatch)]
fn passive_to_dispatch_only() {}

// Uncomment to see a compile error: Dispatch cannot lower to Passive.
// #[irql(max = Dispatch)]
// fn dispatch_calls_passive() {
//     call_irql!(passive_work());  // ERROR: cannot lower IRQL
// }

#[irql(at = Passive)]
fn main() {
    call_irql!(passive_work());
    call_irql!(passive_to_dispatch_only());
}
