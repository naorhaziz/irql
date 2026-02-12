//! IRQL-safe function traits: [`IrqlFn`], [`IrqlFnMut`], [`IrqlFnOnce`].

use irql::{Dispatch, IrqlFn, IrqlFnMut, IrqlFnOnce, Passive, irql};

// -- IrqlFn: immutable, callable multiple times -------------------------

struct Reader {
    value: u32,
}

#[irql(max = Passive)]
impl IrqlFn<()> for Reader {
    type Output = u32;
    fn call(&self, _: ()) -> u32 {
        self.value
    }
}

// -- IrqlFnMut: mutable state -------------------------------------------

struct Counter {
    count: u32,
}

#[irql(max = Passive)]
impl IrqlFnMut<()> for Counter {
    type Output = u32;
    fn call_mut(&mut self, _: ()) -> u32 {
        self.count += 1;
        self.count
    }
}

// -- IrqlFnOnce: consumes self ------------------------------------------

struct Message(String);

#[irql(max = Dispatch)]
impl IrqlFnOnce<()> for Message {
    type Output = String;
    fn call_once(self, _: ()) -> String {
        self.0
    }
}

// -----------------------------------------------------------------------

#[irql(at = Passive)]
fn main() {
    // IrqlFn — can call multiple times
    let reader = Reader { value: 42 };
    println!("Reader: {}", call_irql!(reader.call(())));
    println!("Reader: {}", call_irql!(reader.call(())));

    // IrqlFnMut — mutable calls
    let mut counter = Counter { count: 0 };
    println!("Counter: {}", call_irql!(counter.call_mut(())));
    println!("Counter: {}", call_irql!(counter.call_mut(())));
    println!("Counter: {}", call_irql!(counter.call_mut(())));

    // IrqlFnOnce — consumes self
    let msg = Message("Hello from Dispatch!".to_string());
    println!("Message: {}", call_irql!(msg.call_once(())));
}
