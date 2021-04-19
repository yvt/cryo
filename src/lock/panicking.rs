use core::{
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{Lock, SendMarker};

/// An implementation of [`Lock`] that uses atomic operations. Panics on borrow
/// failure.
#[cfg_attr(docsrs, doc(cfg(feature = "atomic")))]
pub struct AtomicLock {
    count: AtomicUsize,
}

const EXCLUSIVE_FLAG: usize = !(usize::max_value() >> 1);

impl fmt::Debug for AtomicLock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count.load(Ordering::Relaxed);
        if (count & EXCLUSIVE_FLAG) != 0 {
            write!(f, "AtomicLock {{ <locked exclusively> }}",)
        } else {
            write!(f, "AtomicLock {{ num_shared_locks: {} }}", count)
        }
    }
}

unsafe impl Lock for AtomicLock {
    // Any thread can lock
    type LockMarker = SendMarker;

    // Any thread can unlock
    type UnlockMarker = SendMarker;

    #[inline]
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    #[inline]
    unsafe fn lock_shared(&self) {
        if !self.try_lock_shared() {
            borrow_fail();
        }
    }

    #[inline]
    unsafe fn try_lock_shared(&self) -> bool {
        let old_count = self.count.fetch_add(1, Ordering::Acquire);

        // Technically, it can have `EXCLUSIVE_FLAG - 1` shared borrows, but
        // we let it fail earlier so that the counter won't overflow.
        if old_count < EXCLUSIVE_FLAG / 2 {
            // Success
            return true;
        }

        // Failure; revert the change
        self.count.fetch_sub(1, Ordering::Relaxed);
        false
    }

    #[inline]
    unsafe fn unlock_shared(&self) {
        let old_count = self.count.fetch_sub(1, Ordering::Release);
        debug_assert!((old_count & EXCLUSIVE_FLAG) == 0);
    }

    #[inline]
    unsafe fn lock_exclusive(&self) {
        if !self.try_lock_exclusive() {
            borrow_fail();
        }
    }

    #[inline]
    unsafe fn try_lock_exclusive(&self) -> bool {
        self.count
            .compare_exchange(0, EXCLUSIVE_FLAG, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    #[inline]
    unsafe fn unlock_exclusive(&self) {
        let old_count = self.count.fetch_sub(EXCLUSIVE_FLAG, Ordering::Release);
        debug_assert!((old_count & EXCLUSIVE_FLAG) != 0);
    }
}

#[cold]
fn borrow_fail() -> ! {
    panic!("locked")
}
