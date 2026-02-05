<p align="center">
  <img src="https://github.com/naorhaziz/irql/blob/main/Logo.png?raw=true" alt="irql" width="350">
</p>

# IRQL - Compile-Time IRQL Safety for Windows Kernel Drivers

[![Crates.io](https://img.shields.io/crates/v/irql.svg)](https://crates.io/crates/irql)
[![Documentation](https://docs.rs/irql/badge.svg)](https://docs.rs/irql)
[![Build Status](https://img.shields.io/github/actions/workflow/status/naorhaziz/irql/ci.yml?branch=main)](https://github.com/naorhaziz/irql/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

A Rust library providing compile-time verification of IRQL (Interrupt Request Level) constraints for Windows kernel-mode drivers. IRQL violations are caught at compile time, preventing a common class of bugs before code execution.

## Features

- **Compile-Time Safety**: IRQL violations are caught by the Rust compiler, not at runtime
- **Zero Runtime Overhead**: All checks happen at compile time using Rust's type system
- **Clear Error Messages**: Helpful diagnostics explain exactly what IRQL violation occurred
- **Ergonomic API**: Natural Rust syntax with attributes and macros
- **no_std Compatible**: Designed for kernel-mode environments

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
irql = "0.1.2"
```

## Quick Start

```rust
use irql::{requires_irql, root_irql, Dispatch, Passive};

// Function requiring Dispatch IRQL or higher
#[requires_irql(Dispatch)]
fn process_interrupt() {
    // Interrupt processing code
}

// Function requiring Passive IRQL or higher
#[requires_irql(Passive)]
fn driver_routine() {
    // Call function requiring higher IRQL
    call_irql!(process_interrupt());
}

// Entry point at Passive IRQL
#[root_irql(Passive)]
fn main() {
    call_irql!(driver_routine());
}
```

## Motivation

In Windows kernel programming, every piece of code runs at a specific IRQL (Interrupt Request Level). Many kernel APIs are only valid at certain IRQLs, and calling them at the wrong IRQL can cause:

- System crashes (BSOD)
- Data corruption
- Deadlocks
- Undefined behavior

Traditional approaches rely on:
- Runtime checks that discover errors too late
- Developer discipline which is error-prone
- Code reviews which are time-consuming and can miss subtle bugs

This library uses Rust's type system to encode IRQL constraints, ensuring that:

1. IRQL can only be raised or stay the same, never lowered
2. Functions requiring specific IRQLs can only be called from compatible contexts
3. Violations are caught at compile time with clear error messages

## Concepts

### IRQL Hierarchy

From lowest to highest:

| IRQL | Level      | Description                                           |
| ---- | ---------- | ----------------------------------------------------- |
| 0    | `Passive`  | Normal kernel-mode execution, can access paged memory |
| 1    | `Apc`      | Asynchronous Procedure Call level                     |
| 2    | `Dispatch` | Dispatcher/DPC level, most common for driver code     |
| 3-26 | `Dirql`    | Device interrupt levels                               |
| 27   | `Profile`  | Profiling interrupt                                   |
| 28   | `Clock`    | Clock interrupt                                       |
| 29   | `Ipi`      | Inter-processor interrupt                             |
| 30   | `Power`    | Power failure                                         |
| 31   | `High`     | Machine check and catastrophic errors                 |

### The Golden Rule

**IRQL can only stay the same or be raised, never lowered.**

This library enforces this rule at compile time through the `IrqlCanRaiseTo` trait.

## API Reference

### Attributes

#### `#[requires_irql(Level)]`

Marks a function or impl block as requiring a minimum IRQL level.

```rust
#[requires_irql(Dispatch)]
fn acquire_spinlock() {
    // Must be called at Dispatch IRQL or higher
}

#[requires_irql(Passive)]
impl Driver {
    fn initialize(&self) {
        // All methods require Passive IRQL or higher
    }
}
```

#### `#[root_irql(Level)]`

Marks an entry point function with a specific IRQL context.

```rust
#[root_irql(Passive)]
fn driver_entry() {
    // Entry point at Passive IRQL
}
```

### Macros

#### `call_irql!()`

Calls IRQL-constrained functions with appropriate type parameters.

```rust
#[requires_irql(Passive)]
fn example() {
    // Call a function
    call_irql!(some_function());
    
    // Call a method
    call_irql!(device.process());
    
    // Call a constructor
    let obj = call_irql!(Object::new(args));
}
```

## Examples

### Basic Functions

```rust
use irql::{requires_irql, root_irql, Dispatch, Passive};

#[requires_irql(Dispatch)]
fn dispatch_work() {
    // Work that requires Dispatch IRQL
}

#[requires_irql(Passive)]
fn passive_work() {
    // Can call higher IRQL functions
    call_irql!(dispatch_work());
}

#[root_irql(Passive)]
fn main() {
    call_irql!(passive_work());
}
```

### Structs and Methods

```rust
use irql::{requires_irql, root_irql, Dispatch, Passive};

struct Device {
    id: u32,
}

#[requires_irql(Dispatch)]
impl Device {
    fn new(id: u32) -> Self {
        Device { id }
    }
    
    fn process_interrupt(&self) {
        // Interrupt handling at Dispatch level
    }
}

#[root_irql(Passive)]
fn main() {
    let device = call_irql!(Device::new(1));
    call_irql!(device.process_interrupt());
}
```

### Compile-Time Error Prevention

```rust
use irql::{requires_irql, Dispatch, Passive};

#[requires_irql(Dispatch)]
fn high_level() { }

#[requires_irql(Passive)]
fn low_level() { }

#[requires_irql(Dispatch)]
fn caller() {
    // This will NOT compile!
    call_irql!(low_level()); // Error: Cannot lower IRQL
}
```

Error message:

```
error: IRQL violation: cannot call function at `Passive` IRQL from current IRQL level `Dispatch`
  --> src/main.rs:10:5
   |
10 |     call_irql!(low_level());
   |     ^^^^^^^^^^^^^^^^^^^^^^^ cannot lower IRQL or call incompatible IRQL level
   |
   = note: IRQL can only stay the same or be raised, never lowered
```

## How It Works

### Type-Level IRQL Encoding

Each IRQL level is represented as a zero-sized type with no runtime cost:

```rust
pub struct Passive;   // No runtime cost
pub struct Dispatch;  // Just a compile-time marker
```

### Trait-Based Constraints

The `IrqlCanRaiseTo` trait encodes the IRQL hierarchy:

```rust
trait IrqlCanRaiseTo<Target> { }

// Passive can raise to Dispatch
impl IrqlCanRaiseTo<Dispatch> for Passive { }

// Dispatch CANNOT raise to Passive (no such impl exists)
```

### Generic Parameters

Functions annotated with `#[requires_irql]` gain a generic IRQL parameter:

```rust
// Before macro:
#[requires_irql(Dispatch)]
fn process() { }

// After macro expansion:
fn process<IRQL>()
where
    IRQL: IrqlCanRaiseTo<Dispatch>
{
    // ...
}
```

### Compile-Time Verification

The Rust compiler verifies the constraints:

```rust
// Compiles: Passive -> Dispatch (raising IRQL)
process::<Dispatch>();

// Compile error: Dispatch -> Passive (lowering IRQL)
process::<Passive>(); // Error!
```

## Safety Considerations

### Guarantees

- Functions are only called from compatible IRQL contexts
- IRQL is never lowered (only raised or maintained)
- Zero runtime overhead

### Limitations

This library provides compile-time verification only. Developers must ensure:

- Runtime IRQL actually matches compile-time annotations
- Entry points are annotated with their actual runtime IRQL
- IRQL-raising operations (spinlocks, etc.) are properly tracked

### Best Practices

1. **Annotate entry points correctly**: Use `#[root_irql]` with the actual runtime IRQL
2. **Be conservative**: When in doubt, use a lower IRQL requirement
3. **Test thoroughly**: Compile-time checks complement but don't replace testing
4. **Document IRQL assumptions**: Help future maintainers understand your code
