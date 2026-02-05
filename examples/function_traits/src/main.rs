//! Example demonstrating IRQL-aware function traits.

use irql::{
    fn_trait_irql_requires, root_irql, Dispatch, IrqlAsyncFn, IrqlAsyncFnMut, IrqlFn, IrqlFnMut,
    IrqlFnOnce, Passive,
};
use std::future::Future;
use std::pin::Pin;
use tokio::time::{self, Duration};

// Example 1: IrqlFn - immutable function
struct Reader {
    value: u32,
}

#[fn_trait_irql_requires(Passive)]
impl IrqlFn<()> for Reader {
    type Output = u32;

    fn call(&self, _: ()) -> u32 {
        self.value
    }
}

// Example 2: IrqlFnMut - mutable function
struct Counter {
    count: u32,
}

#[fn_trait_irql_requires(Passive)]
impl IrqlFnMut<()> for Counter {
    type Output = u32;

    fn call_mut(&mut self, _: ()) -> u32 {
        self.count += 1;
        self.count
    }
}

// Example 3: IrqlFnOnce - one-time function
struct Message(String);

#[fn_trait_irql_requires(Dispatch)]
impl IrqlFnOnce<()> for Message {
    type Output = String;

    fn call_once(self, _: ()) -> String {
        self.0
    }
}

// Example 4: IrqlAsyncFn - async immutable function
struct AsyncReader {
    value: String,
}

#[fn_trait_irql_requires(Passive)]
impl IrqlAsyncFn<()> for AsyncReader {
    type Output = String;
    type Future = Pin<Box<dyn Future<Output = String> + Send>>;

    fn call_async(&self, _: ()) -> Pin<Box<dyn Future<Output = String> + Send>> {
        let value = self.value.clone();
        Box::pin(async move {
            time::sleep(Duration::from_millis(10)).await;
            value
        })
    }
}

// Example 5: IrqlAsyncFnMut - async mutable function
struct AsyncCounter {
    count: u32,
}

#[fn_trait_irql_requires(Passive)]
impl IrqlAsyncFn<()> for AsyncCounter {
    type Output = u32;
    type Future = Pin<Box<dyn Future<Output = u32> + Send>>;

    fn call_async(&self, _: ()) -> Pin<Box<dyn Future<Output = u32> + Send>> {
        let count = self.count;
        Box::pin(async move { count })
    }
}

#[fn_trait_irql_requires(Passive)]
impl IrqlAsyncFnMut<()> for AsyncCounter {
    fn call_async_mut(&mut self, _: ()) -> Pin<Box<dyn Future<Output = u32> + Send>> {
        self.count += 1;
        let count = self.count;
        Box::pin(async move {
            time::sleep(Duration::from_millis(10)).await;
            count
        })
    }
}

#[tokio::main]
#[root_irql(Passive)]
async fn main() {
    println!("IRQL Function Traits Example\n");

    // IrqlFn - can call multiple times
    let reader = Reader { value: 42 };
    println!("Reader: {}", call_irql!(reader.call(())));
    println!("Reader: {}\n", call_irql!(reader.call(())));

    // IrqlFnMut - mutable calls
    let mut counter = Counter { count: 0 };
    println!("Counter: {}", call_irql!(counter.call_mut(())));
    println!("Counter: {}", call_irql!(counter.call_mut(())));
    println!("Counter: {}\n", call_irql!(counter.call_mut(())));

    // IrqlFnOnce - consumes self
    let msg = Message("Hello from Dispatch!".to_string());
    println!("Message: {}\n", call_irql!(msg.call_once(())));

    // IrqlAsyncFn - async immutable
    let async_reader = AsyncReader {
        value: "Async value".to_string(),
    };
    let result = call_irql!(async_reader.call_async(())).await;
    println!("AsyncReader: {}\n", result);

    // IrqlAsyncFnMut - async mutable
    let mut async_counter = AsyncCounter { count: 100 };
    let result = call_irql!(async_counter.call_async_mut(())).await;
    println!("AsyncCounter: {}", result);
    let result = call_irql!(async_counter.call_async_mut(()));
    println!("AsyncCounter: {}\n", result.await);

    println!("All examples completed!");
}
