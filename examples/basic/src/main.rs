use irql::*;

#[requires_irql(Dispatch)]
fn dispatch_work() {}

#[requires_irql(Passive)]
fn passive_work() {
    // Passive can raise to Dispatch
    call_irql!(dispatch_work());
}

// Uncomment to see a compile error: Dispatch cannot lower to Passive
// #[requires_irql(Dispatch)]
// fn dispatch_calls_passive() {
//     // This would fail - Dispatch cannot lower to Passive
//     call_irql!(passive_work());
// }

#[root_irql(Passive)]
fn main() {
    call_irql!(passive_work());
}
