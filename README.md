<p align="center">
  <img src="https://github.com/naorhaziz/irql/blob/main/Logo.png?raw=true" alt="irql" width="350">
</p>

<h1 align="center">IRQL — Compile-Time IRQL Safety for Windows Kernel Drivers</h1>

<p align="center">
  <a href="https://crates.io/crates/irql"><img src="https://img.shields.io/crates/v/irql.svg" alt="Crates.io"></a>
  <a href="https://docs.rs/irql"><img src="https://docs.rs/irql/badge.svg" alt="Documentation"></a>
  <a href="https://github.com/naorhaziz/irql/actions/workflows/ci.yml"><img src="https://github.com/naorhaziz/irql/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
  <a href="LICENSE-APACHE"><img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg" alt="License: Apache 2.0"></a>
</p>

<p align="center">
  IRQL violations cause blue screens. This crate catches them at compile time.<br>
  <strong>Zero runtime cost. Zero binary overhead.</strong>
</p>

---

```toml
[dependencies]
irql = "0.1.6"
```

```rust
use irql::{irql, Dispatch, Passive};

#[irql(max = Dispatch)]
fn acquire_spinlock() { /* … */ }

#[irql(max = Passive)]
fn driver_routine() {
    call_irql!(acquire_spinlock()); // Passive can raise to Dispatch
}

#[irql(at = Passive)]
fn driver_entry() {
    call_irql!(driver_routine());
}
```

If it compiles, your IRQL transitions are valid.

## How it works

`#[irql(max = Dispatch)]` adds a hidden `IRQL` type parameter bounded by `IrqlCanRaiseTo<Dispatch>`. `call_irql!` threads it through every call as a turbofish argument. The compiler checks every transition — trying to lower IRQL is a compile error:

```text
error[E0277]: IRQL violation: cannot reach `Passive` from `Dispatch`
              -- would require lowering
```

## The `#[irql()]` attribute

| Form                        | Meaning                                    |
| --------------------------- | ------------------------------------------ |
| `#[irql(at = Level)]`       | Fixed entry point — known IRQL, no generic |
| `#[irql(max = Level)]`      | Callable from `Level` or below             |
| `#[irql(min = A, max = B)]` | Callable in \[A, B\]                       |

Works on **functions**, **impl blocks**, and **trait impl blocks**.

## IRQL levels

| Value | Type       | Description                    |
| ----- | ---------- | ------------------------------ |
| 0     | `Passive`  | Normal thread; paged memory OK |
| 1     | `Apc`      | APC delivery                   |
| 2     | `Dispatch` | DPC / spinlock                 |
| 3–26  | `Dirql`    | Device interrupts              |
| 27    | `Profile`  | Profiling timer                |
| 28    | `Clock`    | Clock interrupt                |
| 29    | `Ipi`      | Inter-processor interrupt      |
| 30    | `Power`    | Power failure                  |
| 31    | `High`     | Highest — machine check        |

## Impl blocks

Apply `#[irql]` to an entire `impl` block — every method gets the constraint:

```rust
struct Device { name: &'static str }

#[irql(max = Dispatch)]
impl Device {
    fn new(name: &'static str) -> Self { Device { name } }
    fn process(&self) { /* … */ }
}
```

## Function traits

`IrqlFn`, `IrqlFnMut`, `IrqlFnOnce` — IRQL-aware analogues of `Fn`, `FnMut`, `FnOnce`:

```rust
#[irql(max = Passive)]
impl IrqlFn<()> for Reader {
    type Output = u32;
    fn call(&self, _: ()) -> u32 { self.value }
}
```

The macro rewrites `IrqlFn<()>` to `IrqlFn<Passive, ()>` automatically.

## IRQL-aware allocation (nightly)

```toml
irql = { version = "0.1.6", features = ["alloc"] }
```

Requires nightly (`allocator_api`, `vec_push_within_capacity`, `auto_traits`, `negative_impls`).

### Pool types

| Pool           | Allocable at                 | Accessible at    |
| -------------- | ---------------------------- | ---------------- |
| `PagedPool`    | `Passive`, `Apc`             | `Passive`, `Apc` |
| `NonPagedPool` | `Passive`, `Apc`, `Dispatch` | Any IRQL         |

`IrqlBox::new` and `IrqlVec::new` pick the cheapest legal pool automatically.

### IrqlBox and IrqlVec

```rust
#[irql(max = Passive)]
fn example() -> Result<(), AllocError> {
    let data = call_irql!(IrqlBox::new(42))?;
    let val = call_irql!(data.get());

    let v = irql_vec![1, 2, 3]?;
    call_irql!(v.push(42))?;

    // FFI: transfer ownership via raw pointer
    let ptr = data.into_raw();
    let data = unsafe { IrqlBox::<_, PagedPool>::from_raw(ptr) };
    Ok(())
}
```

### Drop safety

Paged-pool containers cannot be dropped at `Dispatch` or above. The `#[irql]` macro injects `SafeToDropAt<Level>` bounds on by-value parameters, so passing paged-pool memory into elevated-IRQL code is a compile error. References (`&IrqlBox`) are not gated. Use `leak()` or `into_raw()` to transfer ownership across IRQL boundaries.

## Crate architecture

```text
irql           ← public facade (re-exports everything)
├── irql_core  ← levels, hierarchy traits, function traits, SafeToDropAt
├── irql_macro ← #[irql] proc macro, call_irql! rewriter
└── irql_alloc ← IrqlBox, IrqlVec, pool allocator (optional, nightly)
```

## Safety

All checks are compile-time only. You must ensure entry points (`#[irql(at = …)]`) match the actual runtime IRQL and that IRQL-raising operations are properly modelled.

## License

[MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
