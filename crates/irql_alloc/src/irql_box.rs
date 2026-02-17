//! IRQL-aware heap-allocated value.
//!
//! [`IrqlBox`] wraps a value in a kernel pool allocation (paged or non-paged)
//! and enforces IRQL constraints at compile time for both allocation and access.
//!
//! # Pool selection
//!
//! | Method | Pool chosen |
//! |--------|-------------|
//! | [`IrqlBox::new`] | Automatic — [`PagedPool`](crate::PagedPool) at `Passive`/`Apc`, [`NonPagedPool`](crate::NonPagedPool) at `Dispatch` |
//! | [`IrqlBox::new_in`] | Explicit — you specify the pool type parameter |
//!
//! # Drop safety
//!
//! `PagedPool` opts out of `SafeToDropAtDispatch` and above via negative
//! impls (auto traits defined in `irql_core`).  Because `IrqlBox` contains
//! `PagedPool` via `PhantomData`, it inherits those opt-outs automatically.
//! The `#[irql]` macro injects `SafeToDropAt<Level>` bounds on by-value
//! parameters, so passing an `IrqlBox<T, PagedPool>` by value to a
//! function at `Dispatch` or above is a compile error.
//!
//! Use [`leak()`](IrqlBox::leak) or [`into_raw()`](IrqlBox::into_raw)
//! to transfer ownership without dropping.

use alloc::boxed::Box;

use irql_core::IrqlLevel;

use core::alloc::AllocError;

use crate::pool::{AccessibleAt, AllocableAt, DefaultPool, PoolAlloc, PoolAllocator};

/// A heap-allocated value in a kernel pool, with compile-time IRQL access
/// control.
///
/// Wraps [`Box<T, PoolAlloc<P>>`](alloc::boxed::Box) so you get correct
/// allocation/deallocation automatically. Access goes through
/// [`get`](IrqlBox::get) / [`get_mut`](IrqlBox::get_mut), which enforce
/// IRQL constraints at compile time.
///
/// [`Deref`](core::ops::Deref) and [`DerefMut`](core::ops::DerefMut) are
/// intentionally **not** implemented — all access must go through the
/// IRQL-gated [`get`](IrqlBox::get) / [`get_mut`](IrqlBox::get_mut)
/// methods so the compiler can verify the access is safe at the current
/// IRQL.
///
/// # Construction
///
/// ```ignore
/// // Automatic pool (PagedPool at Passive, NonPagedPool at Dispatch):
/// let data = call_irql!(IrqlBox::new(42))?;
///
/// // Explicit pool:
/// let data = call_irql!(IrqlBox::<_, NonPagedPool>::new_in(42))?;
/// ```
pub struct IrqlBox<T, P: PoolAllocator + 'static> {
    inner: Box<T, PoolAlloc<P>>,
}

impl<T, P: PoolAllocator + 'static> IrqlBox<T, P> {
    /// Allocate a value using the default pool for the current IRQL.
    ///
    /// The pool is selected automatically via [`DefaultPool`]:
    /// - `Passive` / `Apc` → [`PagedPool`](crate::PagedPool)
    /// - `Dispatch` → [`NonPagedPool`](crate::NonPagedPool)
    /// - `Dirql`+ → **compile error**
    ///
    /// `call_irql!` injects the IRQL type; `T` and `P` are inferred.
    ///
    /// ```ignore
    /// let data = call_irql!(IrqlBox::new(42))?;
    /// ```
    pub fn new<IRQL: DefaultPool<Pool = P>>(value: T) -> Result<Self, AllocError>
    where
        P: AllocableAt<IRQL>,
    {
        let inner = Box::try_new_in(value, PoolAlloc::new())?;
        Ok(IrqlBox { inner })
    }

    /// Allocate a value in a specific pool, overriding [`DefaultPool`] selection.
    ///
    /// Use this when you need [`NonPagedPool`](crate::NonPagedPool) at low IRQL
    /// (e.g. memory that will be passed to a DPC or ISR).
    ///
    /// ```ignore
    /// let data = call_irql!(IrqlBox::<_, NonPagedPool>::new_in(42))?;
    /// ```
    pub fn new_in<IRQL: IrqlLevel>(value: T) -> Result<Self, AllocError>
    where
        P: AllocableAt<IRQL>,
    {
        let inner = Box::try_new_in(value, PoolAlloc::new())?;
        Ok(IrqlBox { inner })
    }

    /// Shared reference to the contained value.
    ///
    /// ```ignore
    /// let val = call_irql!(data.get());
    /// ```
    pub fn get<IRQL: IrqlLevel>(&self) -> &T
    where
        P: AccessibleAt<IRQL>,
    {
        &self.inner
    }

    /// Mutable reference to the contained value.
    ///
    /// ```ignore
    /// let val = call_irql!(data.get_mut());
    /// ```
    pub fn get_mut<IRQL: IrqlLevel>(&mut self) -> &mut T
    where
        P: AccessibleAt<IRQL>,
    {
        &mut self.inner
    }

    /// Consume the box and return the inner value.
    ///
    /// ```ignore
    /// let value = call_irql!(data.into_inner());
    /// ```
    pub fn into_inner<IRQL: IrqlLevel>(self) -> T
    where
        P: AccessibleAt<IRQL>,
    {
        *self.inner
    }

    /// Consume the box **without** deallocating, returning `&'static mut T`.
    ///
    /// Useful when you need to transfer a [`PagedPool`](crate::PagedPool)
    /// allocation to code running at elevated IRQL without triggering a
    /// drop.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[irql(max = Passive)]
    /// fn prepare_for_dpc() -> Result<&'static mut u32, AllocError> {
    ///     let b = call_irql!(IrqlBox::new(0u32))?;
    ///     Ok(b.leak()) // won't be freed when IRQL rises
    /// }
    /// ```
    #[must_use = "not using the reference will leak memory with no handle"]
    pub fn leak(self) -> &'static mut T {
        Box::leak(self.inner)
    }

    /// Consume the box and return a raw pointer without deallocating.
    ///
    /// The caller is responsible for eventually freeing the memory by
    /// reconstructing the `IrqlBox` via [`from_raw`](IrqlBox::from_raw).
    ///
    /// **No IRQL gate** — useful for transferring ownership across IRQL
    /// boundaries through raw pointers.
    #[must_use = "not using the pointer will leak memory"]
    pub fn into_raw(self) -> *mut T {
        let (ptr, _alloc) = Box::into_raw_with_allocator(self.inner);
        ptr
    }

    /// Reconstruct an `IrqlBox` from a raw pointer previously obtained via
    /// [`into_raw`](IrqlBox::into_raw).
    ///
    /// # Safety
    ///
    /// - `ptr` must have been produced by `IrqlBox::<T, P>::into_raw`.
    /// - The pool type `P` must match the original allocation — a mismatch
    ///   will free memory with the wrong pool, causing corruption or a bugcheck.
    /// - The pointer must not have been freed already.
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        IrqlBox {
            inner: unsafe { Box::from_raw_in(ptr, PoolAlloc::new()) },
        }
    }
}

// SAFETY: `IrqlBox` is `Send` when `T: Send` because the inner `Box` owns
// `T` exclusively, and pool memory itself has no thread affinity.  The
// `PoolAlloc<P>` allocator is a ZST with no state.
unsafe impl<T: Send, P: PoolAllocator + 'static> Send for IrqlBox<T, P> {}

// SAFETY: `IrqlBox` is `Sync` when `T: Sync` because shared access to the
// inner `T` is only possible through `&IrqlBox`, which yields `&T` via
// `get()`. The pool allocator is stateless.
unsafe impl<T: Sync, P: PoolAllocator + 'static> Sync for IrqlBox<T, P> {}
