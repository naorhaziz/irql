<p align="center">
  <img src="https://github.com/naorhaziz/irql/blob/main/Logo.png?raw=true" alt="irql" width="350">
</p>

# IRQL — Compile-Time IRQL Safety for Windows Kernel Drivers

[![Crates.io](https://img.shields.io/crates/v/irql.svg)](https://crates.io/crates/irql)
[![Documentation](https://docs.rs/irql/badge.svg)](https://docs.rs/irql)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

Compile-time verification of IRQL (Interrupt Request Level) constraints for
Windows kernel-mode drivers. IRQL violations become compiler errors — zero
runtime cost.

## Features

- **Compile-time safety** — IRQL violations are caught by `rustc`, not at runtime
- **Zero overhead** — all checks use the type system; nothing emitted at runtime
- **Clear diagnostics** — custom `#[diagnostic::on_unimplemented]` messages
- **Ergonomic** — one attribute (`#[irql()]`) for functions, impl blocks, and trait impls
- **Function traits** — IRQL-safe `IrqlFn`, `IrqlFnMut`, `IrqlFnOnce`
- **`no_std`** — designed for kernel-mode environments

## Installation

```toml
[dependencies]
irql = "0.1.4"
```

## Quick start

```rust
use irql::{irql, Dispatch, Passive};

#[irql(max = Dispatch)]
fn acquire_spinlock() { /* … */ }

#[irql(max = Passive)]
fn driver_routine() {
    call_irql!(acquire_spinlock()); // OK — raising IRQL
}

#[irql(at = Passive)]
fn driver_entry() {
    call_irql!(driver_routine());
}
```

## The `#[irql()]` attribute

| Form                        | Meaning                                          |
| --------------------------- | ------------------------------------------------ |
| `#[irql(at = Level)]`       | Fixed entry point — known IRQL, no generic added |
| `#[irql(max = Level)]`      | Callable from `Level` or below (ceiling)         |
| `#[irql(min = A, max = B)]` | Callable in the range \[A, B\]                   |

- **`max` is required** unless using `at` — it defines the IRQL ceiling that
  `call_irql!` relies on.
- **`min` is optional** — adds a floor constraint (`IrqlCanLowerTo`).
- **`at` is mutually exclusive** with `min`/`max`.

Works on **functions**, **inherent impl blocks**, and **trait impl blocks**.

## IRQL levels

| Value | Type       | Description                              |
| ----- | ---------- | ---------------------------------------- |
| 0     | `Passive`  | Normal thread execution; paged memory OK |
| 1     | `Apc`      | Asynchronous Procedure Call delivery     |
| 2     | `Dispatch` | DPC / spinlock level                     |
| 3–26  | `Dirql`    | Device interrupt levels                  |
| 27    | `Profile`  | Profiling timer                          |
| 28    | `Clock`    | Clock interrupt                          |
| 29    | `Ipi`      | Inter-processor interrupt                |
| 30    | `Power`    | Power failure                            |
| 31    | `High`     | Highest — machine check                  |

## The golden rule

> **IRQL can only stay the same or be raised, never lowered.**

Attempting to call a lower-IRQL function produces a compile error:

```rust
#[irql(max = Passive)]
fn passive_only() {}

#[irql(max = Dispatch)]
fn at_dispatch() {
    call_irql!(passive_only()); // ✗ compile error
}
```

```text
error[E0277]: IRQL violation: cannot reach `Passive` from `Dispatch` -- would require lowering
  --> src/main.rs:6:5
   |
   = note: IRQL can only stay the same or be raised, never lowered
```

## Examples

### Basic functions

```rust
use irql::{irql, Dispatch, Passive};

#[irql(max = Dispatch)]
fn dispatch_work() {}

#[irql(max = Passive)]
fn passive_work() {
    call_irql!(dispatch_work()); // Passive can raise to Dispatch
}

#[irql(at = Passive)]
fn main() {
    call_irql!(passive_work());
}
```

### Structs and impl blocks

```rust
use irql::{irql, Dispatch, Passive};

struct Device { name: &'static str }

#[irql(max = Dispatch)]
impl Device {
    fn new(name: &'static str) -> Self {
        Device { name }
    }

    fn process_interrupt(&self) { /* … */ }
}

struct Driver { device: Device }

#[irql(max = Passive)]
impl Driver {
    fn new(name: &'static str) -> Self {
        Driver { device: call_irql!(Device::new(name)) }
    }

    fn start(&self) {
        call_irql!(self.device.process_interrupt());
    }
}

#[irql(at = Passive)]
fn main() {
    let driver = call_irql!(Driver::new("example"));
    call_irql!(driver.start());
}
```

### IRQL-safe function traits

The library provides `IrqlFn`, `IrqlFnMut`, and `IrqlFnOnce` — IRQL-aware
analogues of `Fn`, `FnMut`, and `FnOnce`. Use the same `#[irql()]` attribute
on trait impl blocks; the macro rewrites `IrqlFn<Args>` → `IrqlFn<Level, Args>`
automatically.

```rust
use irql::{irql, IrqlFn, IrqlFnMut, IrqlFnOnce, Dispatch, Passive};

struct Reader { value: u32 }

#[irql(max = Passive)]
impl IrqlFn<()> for Reader {
    type Output = u32;
    fn call(&self, _: ()) -> u32 { self.value }
}

struct Counter { count: u32 }

#[irql(max = Passive)]
impl IrqlFnMut<()> for Counter {
    type Output = u32;
    fn call_mut(&mut self, _: ()) -> u32 {
        self.count += 1;
        self.count
    }
}

struct Message(String);

#[irql(max = Dispatch)]
impl IrqlFnOnce<()> for Message {
    type Output = String;
    fn call_once(self, _: ()) -> String { self.0 }
}

#[irql(at = Passive)]
fn main() {
    let reader = Reader { value: 42 };
    println!("{}", call_irql!(reader.call(())));

    let mut counter = Counter { count: 0 };
    println!("{}", call_irql!(counter.call_mut(())));

    let msg = Message("Hello from Dispatch!".into());
    println!("{}", call_irql!(msg.call_once(())));
}
```

### Range constraints with `min`

Use `min` to enforce a floor — the caller must be **at or above** the minimum:

```rust
use irql::{irql, Passive, Dispatch};

// Only callable from [Passive, Dispatch] — not from Dirql or above
#[irql(min = Passive, max = Dispatch)]
fn passive_to_dispatch_only() {}

#[irql(at = Passive)]
fn main() {
    call_irql!(passive_to_dispatch_only()); // OK: Passive ∈ [Passive, Dispatch]
}
```

## How it works

### Type-level IRQL encoding

Each IRQL level is a zero-sized type — no runtime cost:

```rust
pub struct Passive;
pub struct Dispatch;
// … 9 levels total
```

### Trait-based hierarchy

The `IrqlCanRaiseTo` trait encodes which transitions are valid:

```rust
// Passive can raise to Dispatch (impl exists)
impl IrqlCanRaiseTo<Dispatch> for Passive {}

// Dispatch cannot lower to Passive (no impl — compile error)
```

`IrqlCanLowerTo` works in the opposite direction for `min` constraints.

### Macro expansion

`#[irql(max = Dispatch)]` adds an `IRQL` generic bounded by
`IrqlCanRaiseTo<Dispatch>`:

```rust
// Source:
#[irql(max = Dispatch)]
fn process() {}

// Expands to:
fn process<IRQL>()
where
    IRQL: IrqlCanRaiseTo<Dispatch>,
{
    macro_rules! call_irql { /* injects IRQL as turbofish */ }
}
```

`call_irql!(f())` rewrites the call to `f::<IRQL>()`, threading the IRQL type
through the call chain. The compiler verifies every transition.

## API reference

### `#[irql()]` attribute

| Syntax                      | Target       | Effect                               |
| --------------------------- | ------------ | ------------------------------------ |
| `#[irql(at = Level)]`       | `fn`         | Fixed entry point — no generic added |
| `#[irql(max = Level)]`      | `fn`, `impl` | Adds `IRQL: IrqlCanRaiseTo<Level>`   |
| `#[irql(min = A, max = B)]` | `fn`, `impl` | Also adds `IRQL: IrqlCanLowerTo<A>`  |

### `call_irql!`

Calls an IRQL-constrained function or method, threading the `IRQL` type:

```rust
call_irql!(some_function());        // free function
call_irql!(self.device.process());  // method call
let d = call_irql!(Device::new(1)); // constructor
```

### Function traits

| Trait                                    | Analogous to | Method                      |
| ---------------------------------------- | ------------ | --------------------------- |
| `IrqlFn<Level, Args, Min = Passive>`     | `Fn`         | `call(&self, args)`         |
| `IrqlFnMut<Level, Args, Min = Passive>`  | `FnMut`      | `call_mut(&mut self, args)` |
| `IrqlFnOnce<Level, Args, Min = Passive>` | `FnOnce`     | `call_once(self, args)`     |

When writing an impl, you only provide `Args` — the macro fills in `Level`
(and `Min` if `min` is set):

```rust
#[irql(max = Passive)]
impl IrqlFn<()> for MyType { /* … */ }
// Expands to: impl IrqlFn<Passive, ()> for MyType { … }
```

## Safety considerations

All checks are **compile-time only**. You must ensure:

- Entry points (`#[irql(at = …)]`) match the actual runtime IRQL
- IRQL-raising operations (e.g. acquiring spinlocks) are properly modelled

### Best practices

1. **Annotate entry points** with `#[irql(at = Level)]` matching the real runtime IRQL
2. **Prefer `max`** for most functions — it's the ceiling that `call_irql!` relies on
3. **Use `min`** only when a function genuinely requires a minimum IRQL floor
4. **Keep constraints tight** — don't use `max = High` when `max = Dispatch` suffices

## License

Licensed under either of

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
