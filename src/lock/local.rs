use core::{cell::Cell, debug_assert_eq, fmt};

use super::{Lock, NoSendMarker};

/// A single-thread implementation of [`Lock`]. Panics on borrow failure.
#[derive(Clone)]
pub struct LocalLock {
    count: Cell<usize>,
}

const EXCLUSIVE: usize = usize::max_value();

impl fmt::Debug for LocalLock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.count.get() == EXCLUSIVE {
            write!(f, "LocalLock {{ <locked exclusively> }}")
        } else {
            write!(f, "LocalLock {{ num_shared_locks: {} }}", self.count.get())
        }
    }
}

unsafe impl Lock for LocalLock {
    #[inline]
    fn new() -> Self {
        Self {
            count: Cell::new(0),
        }
    }

    type LockMarker = NoSendMarker;
    type UnlockMarker = NoSendMarker;

    #[inline]
    unsafe fn lock_shared(&self) {
        let count = &self.count;
        if count.get() >= EXCLUSIVE - 1 {
            // Exclusively borrowed or counter overflow. Ignore the latter case
            // because it's a quite degenerate behavior.
            borrow_fail();
        } else {
            count.set(count.get() + 1);
        }
    }

    #[inline]
    unsafe fn try_lock_shared(&self) -> bool {
        let count = &self.count;
        if count.get() >= EXCLUSIVE - 1 {
            false
        } else {
            count.set(count.get() + 1);
            true
        }
    }

    #[inline]
    unsafe fn unlock_shared(&self) {
        debug_assert_ne!(self.count.get(), 0);
        debug_assert_ne!(self.count.get(), EXCLUSIVE);
        self.count.set(self.count.get() - 1);
    }

    #[inline]
    unsafe fn lock_exclusive(&self) {
        let count = &self.count;
        if count.get() != 0 {
            borrow_fail();
        } else {
            count.set(EXCLUSIVE);
        }
    }

    #[inline]
    unsafe fn try_lock_exclusive(&self) -> bool {
        let count = &self.count;
        if count.get() != 0 {
            false
        } else {
            count.set(EXCLUSIVE);
            true
        }
    }

    #[inline]
    unsafe fn unlock_exclusive(&self) {
        debug_assert_eq!(self.count.get(), EXCLUSIVE);
        self.count.set(0);
    }
}

#[cold]
fn borrow_fail() -> ! {
    panic!("deadlock")
}
