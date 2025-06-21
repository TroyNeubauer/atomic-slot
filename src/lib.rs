#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

//! A simple, lock-free, atomic slot for transferring ownership of `Box<T>`.
//!
//! The `AtomicSlot<T>` holds at most one `Box<T>` and allows you to `swap`, `take` or
//! `store` an optional value using only atomic operations.
//!
//! # Examples
//!
//! ```rust
//! use atomic_slot::AtomicSlot;
//! use std::sync::atomic::Ordering;
//!
//! let slot = AtomicSlot::new(Box::new(7));
//! assert_eq!(*slot.take().unwrap(), 7);
//! assert!(slot.is_none());
//! ```

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

/// A lock-free, thread-safe slot that may contain a `Box<T>`.
///
/// Internally wraps an `AtomicPtr<T>`, using a null pointer to represent `None`.
///
/// For most use cases, the default methods—`swap`, `take`, and `store`—use
/// acquire–release ordering.  If you need finer control, see the `_ordered`
/// variants.
pub struct AtomicSlot<T> {
    inner: AtomicPtr<T>,
    _phantom: PhantomData<Option<Box<T>>>,
}

impl<T> Default for AtomicSlot<T> {
    /// Creates an empty `AtomicSlot<T>`.
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> AtomicSlot<T> {
    /// Creates a new `AtomicSlot` containing `value`.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// let slot = AtomicSlot::new(Box::new("hello"));
    /// assert!(slot.is_some());
    /// ```
    pub fn new(value: Box<T>) -> Self {
        Self {
            inner: AtomicPtr::new(Box::into_raw(value)),
            _phantom: PhantomData,
        }
    }

    /// Creates an empty `AtomicSlot` (contains no value).
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// let slot: AtomicSlot<i32> = AtomicSlot::empty();
    /// assert!(slot.is_none());
    /// ```
    pub fn empty() -> Self {
        Self {
            inner: AtomicPtr::new(ptr::null_mut()),
            _phantom: PhantomData,
        }
    }

    /// Atomically swaps out the current contents for `value`, returning the old contents.
    ///
    /// Uses acquire–release ordering.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// let slot = AtomicSlot::new(Box::new(1));
    /// let old = slot.swap(Some(Box::new(2)));
    /// assert_eq!(*old.unwrap(), 1);
    /// ```
    pub fn swap(&self, value: Option<Box<T>>) -> Option<Box<T>> {
        self.swap_ordered(value, Ordering::AcqRel)
    }

    /// Takes the current contents, leaving the slot empty.
    ///
    /// Uses acquire–release ordering.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// let slot = AtomicSlot::new(Box::new(7));
    /// assert_eq!(*slot.take().unwrap(), 7);
    /// ```
    pub fn take(&self) -> Option<Box<T>> {
        self.take_ordered(Ordering::AcqRel)
    }

    /// Stores `value` into the slot, dropping whatever was there before.
    ///
    /// Uses acquire–release ordering.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// let slot: AtomicSlot<i32> = AtomicSlot::empty();
    /// slot.store(Some(Box::new(5)));
    /// assert_eq!(*slot.take().unwrap(), 5);
    /// ```
    pub fn store(&self, value: Option<Box<T>>) {
        self.store_ordered(value, Ordering::AcqRel)
    }

    /// Atomically swaps out the current contents for `value`, returning the old contents,
    /// with the specified memory `order`.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// # use std::sync::atomic::Ordering;
    /// let slot = AtomicSlot::new(Box::new(3));
    /// let old = slot.swap_ordered(Some(Box::new(4)), Ordering::SeqCst);
    /// assert_eq!(*old.unwrap(), 3);
    /// ```
    pub fn swap_ordered(&self, value: Option<Box<T>>, order: Ordering) -> Option<Box<T>> {
        let raw = value.map(Box::into_raw).unwrap_or(ptr::null_mut());
        let prev = self.inner.swap(raw, order);
        if prev.is_null() {
            None
        } else {
            // Safety: `prev` was originally from `Box::into_raw`
            Some(unsafe { Box::from_raw(prev) })
        }
    }

    /// Takes the current contents, leaving the slot empty, with the specified memory `order`.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// # use std::sync::atomic::Ordering;
    /// let slot = AtomicSlot::new(Box::new(8));
    /// assert_eq!(*slot.take_ordered(Ordering::Acquire).unwrap(), 8);
    /// ```
    pub fn take_ordered(&self, order: Ordering) -> Option<Box<T>> {
        self.swap_ordered(None, order)
    }

    /// Stores `value` into the slot, dropping whatever was there before,
    /// with the specified memory `order`.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// # use std::sync::atomic::Ordering;
    /// let slot: AtomicSlot<&str> = AtomicSlot::empty();
    /// slot.store_ordered(Some(Box::new("data")), Ordering::Release);
    /// ```
    pub fn store_ordered(&self, value: Option<Box<T>>, order: Ordering) {
        let _ = self.swap_ordered(value, order);
    }

    /// Returns `true` if the slot currently contains a value.
    ///
    /// Uses acquire ordering.
    ///
    /// **NOTE:** Because another thread might change the slot right after you check it,  
    /// seeing `is_some() == true` doesn’t ensure that a subsequent `take()`  
    /// will find the same (or any) value.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// let slot: AtomicSlot<i32> = AtomicSlot::new(Box::new(10));
    /// assert!(slot.is_some());
    /// ```
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Returns `true` if the slot is empty.
    ///
    /// Uses acquire ordering.
    ///
    /// **NOTE:** Because another thread might change the slot right after you check it,  
    /// seeing `is_none() == true` doesn’t ensure that a subsequent `take()` will find `None`.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// let slot: AtomicSlot<i32> = AtomicSlot::empty();
    /// assert!(slot.is_none());
    /// ```
    pub fn is_none(&self) -> bool {
        self.is_none_ordered(Ordering::Acquire)
    }

    /// Returns `true` if the slot is empty, with the specified memory `order`.
    ///
    /// ```
    /// # use atomic_slot::AtomicSlot;
    /// # use std::sync::atomic::Ordering;
    /// let slot: AtomicSlot<i32> = AtomicSlot::empty();
    /// assert!(slot.is_none_ordered(Ordering::Relaxed));
    /// ```
    pub fn is_none_ordered(&self, order: Ordering) -> bool {
        self.inner.load(order).is_null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn sequential_swap_and_take() {
        let slot = AtomicSlot::new(Box::new(10));

        let old = slot.swap(Some(Box::new(20)));
        assert_eq!(*old.unwrap(), 10);

        let taken = slot.take();
        assert_eq!(*taken.unwrap(), 20);
        assert!(slot.is_none());
    }

    #[test]
    fn sequential_empty_store() {
        let slot = AtomicSlot::<i32>::empty();
        assert!(slot.is_none());

        slot.store(Some(Box::new(5)));
        assert!(slot.is_some());
        assert_eq!(*slot.take().unwrap(), 5);
    }

    /// Verify that AtomicSlot<T> is Send by moving it into a thread.
    #[test]
    fn atomic_slot_is_send() {
        let slot = AtomicSlot::new(Box::new(42));
        let handle = thread::spawn(move || {
            // We can take the value inside the spawned thread
            assert_eq!(*slot.take().unwrap(), 42);
        });
        handle.join().unwrap();
    }

    /// Verify that AtomicSlot<T> is Sync by sharing a reference across threads.
    #[test]
    fn atomic_slot_is_sync() {
        let slot = Arc::new(AtomicSlot::new(Box::new(100)));
        let slot_clone = slot.clone();
        let handle = thread::spawn(move || {
            // &AtomicSlot<i32> must be Sync to allow shared access
            assert_eq!(*slot_clone.take().unwrap(), 100);
        });
        handle.join().unwrap();
    }

    /// Multiple threads racing to take the single value: only one succeeds.
    #[test]
    fn racing_threads_take_once() {
        let slot = Arc::new(AtomicSlot::new(Box::new(7)));
        let mut handles = Vec::new();

        for _ in 0..4 {
            let slot = slot.clone();
            handles.push(thread::spawn(move || {
                slot.take().map(|b| *b)
            }));
        }

        let results: Vec<_> = handles.into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        // Exactly one thread saw the value
        assert_eq!(results.iter().filter(|r| r.is_some()).count(), 1);
        // The other three saw None
        assert_eq!(results.iter().filter(|r| r.is_none()).count(), 3);
    }
}
