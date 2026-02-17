//! IRQL-aware kernel pool allocator types.
//!
//! Provides [`IrqlBox`] and [`IrqlVec`] — heap-allocated containers that
//! enforce IRQL constraints at compile time for both allocation and access.
//!
//! **Internal crate** — use [`irql`](https://docs.rs/irql) with the `alloc`
//! feature instead.
//!
//! # Nightly requirement
//!
//! This crate uses the unstable `allocator_api`,
//! `vec_push_within_capacity`, and `negative_impls` features and
//! **requires a nightly Rust compiler**.
//!
//! # Kernel pool allocation
//!
//! In WDM/KMDF driver builds, pool allocations use `ExAllocatePool2` /
//! `ExFreePool` from [`wdk-sys`](https://crates.io/crates/wdk-sys).
//! [`PagedPool`] uses `POOL_FLAG_PAGED` and [`NonPagedPool`] uses
//! `POOL_FLAG_NON_PAGED`. All allocations are tagged with `"IrqL"` for
//! WinDbg diagnostics.
//!
//! Outside a WDK build (e.g. testing in user mode), the global allocator
//! is used as a fallback.
//!
//! # Pool types
//!
//! | Pool | Allocable at | Accessible at |
//! |------|-------------|---------------|
//! | [`PagedPool`] | `Passive`, `Apc` | `Passive`, `Apc` |
//! | [`NonPagedPool`] | `Passive`, `Apc`, `Dispatch` | Any IRQL |
//!
//! # Automatic pool selection
//!
//! [`IrqlBox::new`] and [`IrqlVec::new`] pick the cheapest legal pool for
//! the current IRQL:
//!
//! | IRQL | Default pool |
//! |------|-------------|
//! | `Passive` / `Apc` | [`PagedPool`] |
//! | `Dispatch` | [`NonPagedPool`] |
//! | `Dirql`+ | *compile error* |

#![no_std]
#![deny(missing_docs)]
#![feature(allocator_api)]
#![feature(vec_push_within_capacity)]
#![feature(negative_impls)]

extern crate alloc;

mod irql_box;
mod irql_vec;
mod pool;

pub use alloc::collections::TryReserveError;
pub use core::alloc::AllocError;
pub use irql_box::IrqlBox;
pub use irql_vec::IrqlVec;
pub use pool::{
    AccessibleAt, AllocableAt, DefaultPool, NonPagedPool, PagedPool, PoolAlloc, PoolAllocator,
};
