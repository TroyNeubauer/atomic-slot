#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

pub struct AtomicSlot<T> {
    inner: AtomicPtr<T>,
    _phantom: PhantomData<Option<Box<T>>>,
}

unsafe impl<T> Send for AtomicSlot<T> {}
unsafe impl<T> Sync for AtomicSlot<T> {}

impl<T> Default for AtomicSlot<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> AtomicSlot<T> {
    pub fn new(value: Box<T>) -> Self {
        Self {
            inner: AtomicPtr::new(Box::into_raw(value)),
            _phantom: PhantomData,
        }
    }

    pub fn empty() -> Self {
        Self {
            inner: AtomicPtr::new(ptr::null_mut()),
            _phantom: PhantomData,
        }
    }

    pub fn swap(&self, value: Option<Box<T>>) -> Option<Box<T>> {
        let value = value.map(Box::into_raw).unwrap_or(ptr::null_mut());
        let ptr = self.inner.swap(value, Ordering::AcqRel);

        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(ptr) })
        }
    }

    pub fn take(&self) -> Option<Box<T>> {
        self.swap(None)
    }

    pub fn store(&self, value: Option<Box<T>>) {
        self.swap(value);
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn is_none(&self) -> bool {
        self.inner.load(Ordering::Acquire).is_null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
