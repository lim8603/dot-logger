//! Locks that do not allocate after construction on any supported OS.
//!
//! A std `Mutex` allocates nothing on Linux (futex) or Windows (SRWLock), but on
//! Apple targets it allocates its `pthread_mutex_t` on the first lock. A
//! management path that first locks during operation would then allocate
//! after warm-up. Locking once at construction moves that allocation there
//! (TASK-036 S5).

use std::sync::Mutex;

/// Heap bytes a std `Mutex` allocates on its first lock: the Apple
/// `pthread_mutex_t`, and nothing elsewhere. Reservations that cover a struct
/// holding a mutex add this.
pub(crate) const MUTEX_BYTES: usize = if cfg!(target_vendor = "apple") { 64 } else { 0 };

/// A std `Mutex` already locked once, so later locks never allocate.
pub(crate) fn warm_mutex<T>(value: T) -> Mutex<T> {
    let mutex = Mutex::new(value);
    drop(mutex.lock());
    mutex
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_warm_mutex_holds_its_value_and_is_not_poisoned() {
        let mutex = super::warm_mutex(7);
        assert!(!mutex.is_poisoned());
        assert_eq!(*mutex.lock().unwrap_or_else(|e| e.into_inner()), 7);
    }
}
