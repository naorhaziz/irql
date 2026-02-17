//! IRQL-aware dynamically-sized array.
//!
//! [`IrqlVec`] wraps a [`Vec`] with a kernel pool allocator and enforces
//! IRQL constraints at compile time for allocation, mutation, and access.
//!
//! All operations that may allocate are **fallible** — they return
//! `Result` instead of panicking on OOM. This is essential for kernel code
//! where allocation failure must be handled gracefully.
//!
//! # Quick start
//!
//! ```ignore
//! #[irql(max = Passive)]
//! fn example() -> Result<(), TryReserveError> {
//!     let v = irql_vec![1, 2, 3]?;
//!     let s = call_irql!(v.as_slice());       // &[1, 2, 3]
//!
//!     let mut v = irql_vec![0u8; 64]?;        // 64 zeroes
//!     call_irql!(v.push(0xFF))?;              // now 65 elements
//!     Ok(())
//! }
//! ```

use alloc::{collections::TryReserveError, vec::Vec};

use irql_core::IrqlLevel;

use crate::pool::{AccessibleAt, AllocableAt, DefaultPool, PoolAlloc, PoolAllocator};

/// A dynamically-sized array in a kernel pool, with compile-time IRQL access
/// control.
///
/// Thin wrapper around [`Vec<T, PoolAlloc<P>>`](alloc::vec::Vec) that gates
/// every method behind IRQL constraints. Only exposes fallible
/// (no-global-oom-handling) APIs — every operation that may allocate returns
/// a [`Result`].
///
/// [`Index`](core::ops::Index) and [`Deref`](core::ops::Deref) are
/// intentionally **not** implemented — all access must go through
/// IRQL-gated methods so the compiler can verify safety.
pub struct IrqlVec<T, P: PoolAllocator> {
    inner: Vec<T, PoolAlloc<P>>,
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

impl<T, P: PoolAllocator> IrqlVec<T, P> {
    /// Create a new empty `IrqlVec` (does **not** allocate).
    pub fn new<IRQL: DefaultPool<Pool = P>>() -> Self
    where
        P: AllocableAt<IRQL>,
    {
        IrqlVec {
            inner: Vec::new_in(PoolAlloc::new()),
        }
    }

    /// Create a new empty `IrqlVec` with pre-reserved capacity.
    pub fn with_capacity<IRQL: DefaultPool<Pool = P>>(cap: usize) -> Result<Self, TryReserveError>
    where
        P: AllocableAt<IRQL>,
    {
        let mut v = Vec::new_in(PoolAlloc::new());
        v.try_reserve(cap)?;
        Ok(IrqlVec { inner: v })
    }

    /// Build an `IrqlVec` from an iterator (backbone of `irql_vec!`).
    pub fn try_from_iter<IRQL: DefaultPool<Pool = P>>(
        iter: impl IntoIterator<Item = T>,
    ) -> Result<Self, TryReserveError>
    where
        P: AllocableAt<IRQL>,
    {
        let iter = iter.into_iter();
        let (hint, _) = iter.size_hint();
        let mut v = Vec::new_in(PoolAlloc::new());
        v.try_reserve(hint)?;
        for item in iter {
            v.try_reserve(1)?;
            // Infallible — we just reserved one slot.
            let _ = v.push_within_capacity(item);
        }
        Ok(IrqlVec { inner: v })
    }
}

// ---------------------------------------------------------------------------
// Capacity — forwards Vec::try_reserve / try_reserve_exact
// ---------------------------------------------------------------------------

impl<T, P: PoolAllocator> IrqlVec<T, P> {
    /// Forwards [`Vec::try_reserve`].
    pub fn try_reserve<IRQL: IrqlLevel>(&mut self, additional: usize) -> Result<(), TryReserveError>
    where
        P: AllocableAt<IRQL>,
    {
        self.inner.try_reserve(additional)
    }

    /// Forwards [`Vec::try_reserve_exact`].
    pub fn try_reserve_exact<IRQL: IrqlLevel>(
        &mut self,
        additional: usize,
    ) -> Result<(), TryReserveError>
    where
        P: AllocableAt<IRQL>,
    {
        self.inner.try_reserve_exact(additional)
    }
}

// ---------------------------------------------------------------------------
// Mutation — forwards Vec's no-oom-handling mutation methods
// ---------------------------------------------------------------------------

impl<T, P: PoolAllocator> IrqlVec<T, P> {
    /// Push a value if capacity is available (no allocation).
    ///
    /// Call [`try_reserve`](Self::try_reserve) first to ensure space.
    /// Returns `Ok(&mut T)` on success, or `Err(value)` if full.
    pub fn push_within_capacity<IRQL: IrqlLevel>(&mut self, value: T) -> Result<&mut T, T>
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.push_within_capacity(value)
    }

    /// Convenience: `try_reserve(1)` + `push_within_capacity`.
    ///
    /// ```ignore
    /// call_irql!(v.push(42))?;
    /// ```
    pub fn push<IRQL: IrqlLevel>(&mut self, value: T) -> Result<(), TryReserveError>
    where
        P: AccessibleAt<IRQL> + AllocableAt<IRQL>,
    {
        self.inner.try_reserve(1)?;
        // Infallible — we just reserved one slot.
        let _ = self.inner.push_within_capacity(value);
        Ok(())
    }

    /// Remove and return the last element, or `None` if empty.
    pub fn pop<IRQL: IrqlLevel>(&mut self) -> Option<T>
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.pop()
    }

    /// Shorten the vec to `len` elements, dropping the rest.
    pub fn truncate<IRQL: IrqlLevel>(&mut self, len: usize)
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.truncate(len);
    }

    /// Remove all elements (length becomes 0, capacity unchanged).
    pub fn clear<IRQL: IrqlLevel>(&mut self)
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.clear();
    }

    /// Insert an element at `index`, shifting subsequent elements right.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    pub fn insert<IRQL: IrqlLevel>(&mut self, index: usize, value: T) -> Result<(), TryReserveError>
    where
        P: AccessibleAt<IRQL> + AllocableAt<IRQL>,
    {
        self.inner.try_reserve(1)?;
        self.inner.insert(index, value);
        Ok(())
    }

    /// Remove and return the element at `index`, shifting subsequent elements left.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`.
    pub fn remove<IRQL: IrqlLevel>(&mut self, index: usize) -> T
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.remove(index)
    }

    /// Remove the element at `index` by swapping it with the last element.
    ///
    /// O(1) but does not preserve ordering.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`.
    pub fn swap_remove<IRQL: IrqlLevel>(&mut self, index: usize) -> T
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.swap_remove(index)
    }

    /// Retain only elements for which `f` returns `true`.
    ///
    /// Elements are visited in order and removed in place.
    pub fn retain<IRQL: IrqlLevel>(&mut self, f: impl FnMut(&T) -> bool)
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.retain(f);
    }

    /// Extend with all elements from a slice (requires `T: Clone`).
    ///
    /// Reserves capacity for the entire slice before copying.
    pub fn extend_from_slice<IRQL: IrqlLevel>(&mut self, other: &[T]) -> Result<(), TryReserveError>
    where
        T: Clone,
        P: AccessibleAt<IRQL> + AllocableAt<IRQL>,
    {
        self.inner.try_reserve(other.len())?;
        self.inner.extend_from_slice(other);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Access — forwards Vec's read methods
// ---------------------------------------------------------------------------

impl<T, P: PoolAllocator> IrqlVec<T, P> {
    /// Returns a shared slice of the contents.
    pub fn as_slice<IRQL: IrqlLevel>(&self) -> &[T]
    where
        P: AccessibleAt<IRQL>,
    {
        &self.inner
    }

    /// Returns a mutable slice of the contents.
    pub fn as_mut_slice<IRQL: IrqlLevel>(&mut self) -> &mut [T]
    where
        P: AccessibleAt<IRQL>,
    {
        &mut self.inner
    }

    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    pub fn get<IRQL: IrqlLevel>(&self, index: usize) -> Option<&T>
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.get(index)
    }

    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    pub fn get_mut<IRQL: IrqlLevel>(&mut self, index: usize) -> Option<&mut T>
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.get_mut(index)
    }

    /// Returns an iterator over shared references.
    ///
    /// ```ignore
    /// for val in call_irql!(v.iter()) {
    ///     // use val
    /// }
    /// ```
    pub fn iter<IRQL: IrqlLevel>(&self) -> core::slice::Iter<'_, T>
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.iter()
    }

    /// Returns an iterator over mutable references.
    pub fn iter_mut<IRQL: IrqlLevel>(&mut self) -> core::slice::IterMut<'_, T>
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.iter_mut()
    }

    /// Returns the first element, or `None` if empty.
    pub fn first<IRQL: IrqlLevel>(&self) -> Option<&T>
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.first()
    }

    /// Returns the last element, or `None` if empty.
    pub fn last<IRQL: IrqlLevel>(&self) -> Option<&T>
    where
        P: AccessibleAt<IRQL>,
    {
        self.inner.last()
    }

    /// Returns `true` if the vec contains the given value.
    pub fn contains<IRQL: IrqlLevel>(&self, x: &T) -> bool
    where
        T: PartialEq,
        P: AccessibleAt<IRQL>,
    {
        self.inner.contains(x)
    }
}

// ---------------------------------------------------------------------------
// Metadata — no IRQL gate (these don't touch heap memory)
// ---------------------------------------------------------------------------

impl<T, P: PoolAllocator> IrqlVec<T, P> {
    /// Number of elements in the vec.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the vec contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of elements the vec can hold without re-allocating.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}

// SAFETY: `IrqlVec` is `Send` when `T: Send` because the inner `Vec` owns
// its elements exclusively, and pool memory has no thread affinity.  The
// `PoolAlloc<P>` allocator is a stateless ZST.
unsafe impl<T: Send, P: PoolAllocator> Send for IrqlVec<T, P> {}

// SAFETY: `IrqlVec` is `Sync` when `T: Sync` because shared access to
// elements is only possible through `&IrqlVec`, which yields `&[T]` via
// `as_slice()`. The pool allocator is stateless.
unsafe impl<T: Sync, P: PoolAllocator> Sync for IrqlVec<T, P> {}

// ---------------------------------------------------------------------------
// irql_vec! macro
// ---------------------------------------------------------------------------

/// Create an [`IrqlVec`] with automatic pool selection, similar to `vec![]`.
///
/// Must be called inside an `#[irql]`-annotated function body where the
/// local `call_irql!` macro is available.
///
/// ```ignore
/// let v = irql_vec![1, 2, 3]?;
/// let v = irql_vec![0; 5]?;
/// ```
#[macro_export]
macro_rules! irql_vec {
    () => {
        call_irql!($crate::IrqlVec::try_from_iter(core::iter::empty()))
    };
    ($elem:expr; $count:expr) => {
        call_irql!($crate::IrqlVec::try_from_iter([$elem; $count]))
    };
    ($($elem:expr),+ $(,)?) => {
        call_irql!($crate::IrqlVec::try_from_iter([$($elem),+]))
    };
}
