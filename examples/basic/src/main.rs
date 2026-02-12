use irql::*;

// max-only: callable from Dispatch or below (same as #[requires_irql(Dispatch)])
#[irql(max = Dispatch)]
fn dispatch_work() {}

// max-only: callable from Passive or below
#[irql(max = Passive)]
fn passive_work() {
    // Passive can raise to Dispatch
    call_irql!(dispatch_work());
}

// Range: callable from Passive through Dispatch (not Dirql+)
#[irql(min = Passive, max = Dispatch)]
fn passive_to_dispatch_only() {}

// Uncomment to see a compile error: Dispatch cannot lower to Passive
// fn dispatch_calls_passive() {
//     call_irql!(passive_work::<()>());  // ERROR: Cannot lower IRQL!
// }

#[irql(at = Passive)]
fn main() {
    call_irql!(passive_work());
    call_irql!(passive_to_dispatch_only());
}
