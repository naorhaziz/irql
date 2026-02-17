//! Kernel pool marker types and IRQL-aware traits.

use core::alloc::Layout;
use core::ptr::NonNull;
use irql_core::{Apc, Clock, Dirql, Dispatch, High, Ipi, IrqlLevel, Passive, Power, Profile};
use irql_core::{
    SafeToDropAtClock, SafeToDropAtDirql, SafeToDropAtDispatch, SafeToDropAtHigh, SafeToDropAtIpi,
    SafeToDropAtPower, SafeToDropAtProfile,
};

// ---------------------------------------------------------------------------
// Pool marker types
// ---------------------------------------------------------------------------

/// Paged pool memory.
///
/// - Allocable at `Passive` and `Apc` only.
/// - Accessible at `Passive` and `Apc` only — paged memory may be swapped out
///   at higher IRQLs.
/// - **Cannot be dropped** at `Dispatch` or above (enforced via auto traits).
pub struct PagedPool;

/// Non-paged pool memory.
///
/// - Allocable at `Passive`, `Apc`, and `Dispatch`.
/// - Accessible at any IRQL — non-paged memory is never swapped out.
pub struct NonPagedPool;

// ---------------------------------------------------------------------------
// Drop safety — PagedPool must not be freed at Dispatch or above
// ---------------------------------------------------------------------------
//
// Negative impls propagate automatically: any type containing `PagedPool`
// (or `PhantomData<PagedPool>`) also loses the corresponding auto trait.

impl !SafeToDropAtDispatch for PagedPool {}
impl !SafeToDropAtDirql for PagedPool {}
impl !SafeToDropAtProfile for PagedPool {}
impl !SafeToDropAtClock for PagedPool {}
impl !SafeToDropAtIpi for PagedPool {}
impl !SafeToDropAtPower for PagedPool {}
impl !SafeToDropAtHigh for PagedPool {}

// ---------------------------------------------------------------------------
// AllocableAt
// ---------------------------------------------------------------------------

/// Pool `Self` supports allocation at IRQL `I`.
#[diagnostic::on_unimplemented(
    message = "cannot allocate from `{Self}` at IRQL `{I}`",
    label = "allocation not allowed at this IRQL",
    note = "paged pool requires IRQL <= APC_LEVEL; non-paged pool requires IRQL <= DISPATCH_LEVEL"
)]
pub trait AllocableAt<I: IrqlLevel> {}

impl AllocableAt<Passive> for PagedPool {}
impl AllocableAt<Apc> for PagedPool {}

impl AllocableAt<Passive> for NonPagedPool {}
impl AllocableAt<Apc> for NonPagedPool {}
impl AllocableAt<Dispatch> for NonPagedPool {}

// ---------------------------------------------------------------------------
// AccessibleAt
// ---------------------------------------------------------------------------

/// Memory from pool `Self` can be safely dereferenced at IRQL `I`.
#[diagnostic::on_unimplemented(
    message = "cannot access `{Self}` memory at IRQL `{I}` -- paged memory may be swapped out",
    label = "memory access not safe at this IRQL",
    note = "paged pool memory is only safe to access at IRQL <= APC_LEVEL"
)]
pub trait AccessibleAt<I: IrqlLevel> {}

impl AccessibleAt<Passive> for PagedPool {}
impl AccessibleAt<Apc> for PagedPool {}

impl AccessibleAt<Passive> for NonPagedPool {}
impl AccessibleAt<Apc> for NonPagedPool {}
impl AccessibleAt<Dispatch> for NonPagedPool {}
impl AccessibleAt<Dirql> for NonPagedPool {}
impl AccessibleAt<Profile> for NonPagedPool {}
impl AccessibleAt<Clock> for NonPagedPool {}
impl AccessibleAt<Ipi> for NonPagedPool {}
impl AccessibleAt<Power> for NonPagedPool {}
impl AccessibleAt<High> for NonPagedPool {}

// ---------------------------------------------------------------------------
// DefaultPool
// ---------------------------------------------------------------------------

/// Maps an IRQL level to its default pool type.
///
/// `Passive`/`Apc` → [`PagedPool`], `Dispatch` → [`NonPagedPool`],
/// `Dirql`+ → compile error.
#[diagnostic::on_unimplemented(
    message = "cannot allocate at IRQL `{Self}` -- no kernel pool is available at this level",
    label = "no allocator for this IRQL",
    note = "memory allocation requires IRQL <= DISPATCH_LEVEL"
)]
pub trait DefaultPool: IrqlLevel {
    /// The pool type used by default at this IRQL.
    type Pool;
}

impl DefaultPool for Passive {
    type Pool = PagedPool;
}
impl DefaultPool for Apc {
    type Pool = PagedPool;
}
impl DefaultPool for Dispatch {
    type Pool = NonPagedPool;
}

// ---------------------------------------------------------------------------
// PoolAllocator
// ---------------------------------------------------------------------------

/// Low-level allocator for a kernel pool.
///
/// # Safety
///
/// Implementors must correctly allocate and deallocate memory from the
/// corresponding kernel pool.
pub unsafe trait PoolAllocator {
    /// Allocate memory. Returns null on failure.
    ///
    /// # Safety
    ///
    /// The caller must ensure the current IRQL is appropriate for this pool.
    unsafe fn alloc(layout: Layout) -> *mut u8;

    /// Deallocate memory previously allocated by [`alloc`](PoolAllocator::alloc).
    ///
    /// # Safety
    ///
    /// - `ptr` must have been allocated by this pool's `alloc`.
    /// - `layout` must match the original allocation.
    unsafe fn dealloc(ptr: *mut u8, layout: Layout);
}

// ---------------------------------------------------------------------------
// PoolAllocator impls — WDK kernel pool APIs
// ---------------------------------------------------------------------------

#[cfg(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"))]
use wdk_sys::{
    POOL_FLAG_NON_PAGED, POOL_FLAG_PAGED, SIZE_T, ULONG,
    ntddk::{ExAllocatePool2, ExFreePool},
};

/// Pool tag: appears as `"IrqL"` in WinDbg (`!pool` / `!pooltag`).
#[cfg(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"))]
const IRQL_POOL_TAG: ULONG = u32::from_ne_bytes(*b"IrqL");

#[cfg(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"))]
macro_rules! impl_wdk_pool_allocator {
    ($pool:ty, $flag:expr) => {
        unsafe impl PoolAllocator for $pool {
            unsafe fn alloc(layout: Layout) -> *mut u8 {
                let ptr = unsafe { ExAllocatePool2($flag, layout.size() as SIZE_T, IRQL_POOL_TAG) };
                if ptr.is_null() {
                    return core::ptr::null_mut();
                }
                ptr.cast()
            }

            unsafe fn dealloc(ptr: *mut u8, _layout: Layout) {
                unsafe { ExFreePool(ptr.cast()) }
            }
        }
    };
}

#[cfg(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"))]
impl_wdk_pool_allocator!(PagedPool, POOL_FLAG_PAGED);

#[cfg(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"))]
impl_wdk_pool_allocator!(NonPagedPool, POOL_FLAG_NON_PAGED);

// ---------------------------------------------------------------------------
// PoolAllocator impls — global allocator fallback (non-WDK / testing)
// ---------------------------------------------------------------------------

#[cfg(not(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF")))]
macro_rules! impl_fallback_pool_allocator {
    ($pool:ty) => {
        unsafe impl PoolAllocator for $pool {
            unsafe fn alloc(layout: Layout) -> *mut u8 {
                unsafe { alloc::alloc::alloc(layout) }
            }
            unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
                unsafe { alloc::alloc::dealloc(ptr, layout) }
            }
        }
    };
}

#[cfg(not(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF")))]
impl_fallback_pool_allocator!(PagedPool);

#[cfg(not(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF")))]
impl_fallback_pool_allocator!(NonPagedPool);

// ---------------------------------------------------------------------------
// PoolAlloc<P> — Allocator API adapter
// ---------------------------------------------------------------------------

/// Zero-sized [`Allocator`](core::alloc::Allocator) that delegates to
/// `P`'s [`PoolAllocator`] implementation.
#[derive(Debug, Copy, Clone)]
pub struct PoolAlloc<P>(core::marker::PhantomData<P>);

impl<P> PoolAlloc<P> {
    /// Create a new `PoolAlloc` instance.
    #[must_use]
    pub const fn new() -> Self {
        PoolAlloc(core::marker::PhantomData)
    }
}

impl<P> Default for PoolAlloc<P> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<P: PoolAllocator> core::alloc::Allocator for PoolAlloc<P> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, core::alloc::AllocError> {
        if layout.size() == 0 {
            let ptr = NonNull::new(layout.align() as *mut u8).ok_or(core::alloc::AllocError)?;
            return Ok(NonNull::slice_from_raw_parts(ptr, 0));
        }
        let ptr = unsafe { P::alloc(layout) };
        let ptr = NonNull::new(ptr).ok_or(core::alloc::AllocError)?;
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if layout.size() == 0 {
            return;
        }
        unsafe { P::dealloc(ptr.as_ptr(), layout) };
    }
}
