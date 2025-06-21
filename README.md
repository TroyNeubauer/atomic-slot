# atomic-slot

A simple, lock-free, atomic slot for transferring ownership of `Box<T>`.

The `AtomicSlot<T>` holds at most one `Box<T>` and allows you to `swap`, `take` or
`store` an optional value using only atomic operations.

## Examples

```rust
use atomic_slot::AtomicSlot;
use std::sync::atomic::Ordering;

let slot = AtomicSlot::new(Box::new(7));
assert_eq!(*slot.take().unwrap(), 7);
assert!(slot.is_none());
```
