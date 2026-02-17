#![feature(allocator_api)]

//! IRQL-aware allocation example.
//!
//! Demonstrates compile-time enforcement of:
//! - Automatic pool selection based on IRQL level
//! - Access control on pool memory
//! - Explicit pool selection
//!
//! Outside a WDK build, `PoolAllocator` impls delegate to the global
//! allocator. In a real driver build, they use `ExAllocatePool2` / `ExFreePool`
//! from `wdk-sys` automatically.

use irql::*;

// ---------------------------------------------------------------------------
// Driver code
// ---------------------------------------------------------------------------

/// At Passive level, automatic allocation uses PagedPool.
#[irql(max = Passive)]
fn allocate_at_passive() -> Result<(), AllocError> {
    // IrqlBox::new(value) automatically selects PagedPool at Passive
    let data = call_irql!(IrqlBox::new(42))?;

    // Access is allowed: PagedPool is accessible at Passive
    let val = call_irql!(data.get());
    assert_eq!(*val, 42);

    // Mutable access
    let mut data = call_irql!(IrqlBox::new(0))?;
    *call_irql!(data.get_mut()) = 99;
    assert_eq!(*call_irql!(data.get()), 99);

    println!("Passive allocation OK: PagedPool, value = {val}");
    Ok(())
}

/// At Dispatch level, automatic allocation uses NonPagedPool.
#[irql(max = Dispatch)]
fn allocate_at_dispatch() -> Result<(), AllocError> {
    // IrqlBox::new(value) automatically selects NonPagedPool at Dispatch
    let data = call_irql!(IrqlBox::new(99))?;

    // Access is allowed: NonPagedPool is accessible at Dispatch
    let val = call_irql!(data.get());
    assert_eq!(*val, 99);

    println!("Dispatch allocation OK: NonPagedPool, value = {val}");
    Ok(())
}

/// Explicit pool selection: force NonPagedPool even at Passive.
#[irql(max = Passive)]
fn explicit_nonpaged() -> Result<(), AllocError> {
    let data = call_irql!(IrqlBox::<_, NonPagedPool>::new_in(123))?;
    let val = call_irql!(data.get());
    assert_eq!(*val, 123);

    println!("Explicit NonPagedPool at Passive OK: value = {val}");
    Ok(())
}

/// Demonstrate into_inner (consumes the box).
#[irql(max = Passive)]
fn consume_box() -> Result<(), AllocError> {
    let data = call_irql!(IrqlBox::new(String::from("hello")))?;
    let s = call_irql!(data.into_inner());
    assert_eq!(s, "hello");

    println!("into_inner OK: {s}");
    Ok(())
}

/// Demonstrate leak (prevents deallocation).
#[irql(max = Passive)]
fn leak_box() -> Result<(), AllocError> {
    let data = call_irql!(IrqlBox::new(777))?;
    let leaked: &'static mut i32 = data.leak();
    assert_eq!(*leaked, 777);

    println!("leak OK: {leaked}");
    // NOTE: memory is intentionally leaked (useful for passing to elevated IRQL)
    Ok(())
}

/// Demonstrate IrqlVec with the irql_vec! macro.
#[irql(max = Passive)]
fn vec_example() -> Result<(), TryReserveError> {
    // irql_vec![...] works like vec![...]
    let v = irql_vec![10, 20, 30]?;
    let s = call_irql!(v.as_slice());
    assert_eq!(s, &[10, 20, 30]);
    println!("irql_vec! OK: {s:?}");

    // Repeat form
    let v = irql_vec![42; 4]?;
    assert_eq!(call_irql!(v.as_slice()), &[42, 42, 42, 42]);
    println!("irql_vec![42; 4] OK: len = {}", v.len());

    // Push / pop
    let mut v = irql_vec![1, 2]?;
    call_irql!(v.push(3))?;
    assert_eq!(call_irql!(v.as_slice()), &[1, 2, 3]);
    let last = call_irql!(v.pop());
    assert_eq!(last, Some(3));
    println!("push/pop OK: {:?}", call_irql!(v.as_slice()));

    Ok(())
}

// ---------------------------------------------------------------------------
// Compile-error demonstrations (uncomment to verify)
// ---------------------------------------------------------------------------

// ERROR: cannot allocate at Dirql (no DefaultPool impl)
// #[irql(max = Dirql)]
// fn cannot_alloc_at_dirql() {
//     let _ = call_irql!(IrqlBox::new(1));
// }

// ERROR: cannot drop PagedPool memory at Dispatch (SafeToDropAtDispatch)
// #[irql(max = Dispatch)]
// fn cannot_drop_paged_at_dispatch(paged_box: IrqlBox<i32, PagedPool>) {
//     // Compile error: IrqlBox<i32, PagedPool> is !SafeToDropAtDispatch
// }

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[irql(at = Passive)]
fn main() {
    call_irql!(allocate_at_passive()).expect("passive alloc failed");
    call_irql!(allocate_at_dispatch()).expect("dispatch alloc failed");
    call_irql!(explicit_nonpaged()).expect("explicit nonpaged failed");
    call_irql!(consume_box()).expect("consume box failed");
    call_irql!(leak_box()).expect("leak box failed");
    call_irql!(vec_example()).expect("vec example failed");

    println!("\nAll IRQL-aware allocation tests passed!");
}
