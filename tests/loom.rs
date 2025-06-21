#![cfg(loom)]
use atomic_slot::AtomicSlot;
use std::sync::Arc;

#[test]
fn concurrent_store_and_take() {
    loom::model(|| {
        let slot = Arc::new(AtomicSlot::empty());

        let t1 = {
            let s = slot.clone();
            loom::thread::spawn(move || {
                s.store(Some(Box::new(5)));
            })
        };

        let result = {
            let slot2 = slot.clone();
            loom::thread::spawn(move || slot2.take()).join().unwrap()
        };

        t1.join().unwrap();

        match result {
            Some(b) => {
                // If we saw it, slot is now empty
                assert_eq!(*b, 5);
                assert!(slot.is_none());
            }
            None => {
                // Otherwise the value remains and we take again
                assert!(slot.is_some());
                let second = slot.take().unwrap();
                assert_eq!(*second, 5);
                assert!(slot.is_none());
            }
        }
    });
}

#[test]
fn concurrent_swaps() {
    loom::model(|| {
        let slot = Arc::new(AtomicSlot::empty());

        let threads: Vec<_> = (0..=2)
            .map(|n| {
                let s = slot.clone();
                loom::thread::spawn(move || {
                    // swap in n and get whatever was there
                    let prev = s.swap(Some(Box::new(n)));
                    prev
                })
            })
            .collect();

        let mut seen = Vec::new();
        for th in threads {
            if let Some(v) = th.join().unwrap() {
                seen.push(*v);
            }
        }
        let final_val = slot.take();
        seen.push(*final_val.unwrap());
        seen.sort();

        assert_eq!(&seen, &[0, 1, 2]);
    });
}
