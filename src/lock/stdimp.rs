use std::{
    sync::atomic::{fence, AtomicUsize, Ordering},
    thread,
};

use super::{Lock, NoSendMarker, SendMarker};

/// An implementation of [`Lock`] that uses the synchronization facility
/// provided by [`::std`]. Lock operations are tied to the creator thread, but
/// unlock operations can be done in any threads.
pub struct SyncLock {
    owner: thread::Thread,
    count: AtomicUsize,
}

const PARKED_FLAG: usize = !(usize::max_value() >> 1);
const EXCLUSIVE_FLAG: usize = PARKED_FLAG >> 1;

unsafe impl Lock for SyncLock {
    // Only the creator thread can lock
    type LockMarker = NoSendMarker;

    // Any thread can unlock
    type UnlockMarker = SendMarker;

    #[inline]
    fn new() -> Self {
        Self {
            owner: thread::current(),
            count: AtomicUsize::new(0),
        }
    }

    #[inline]
    unsafe fn lock_shared(&self) {
        // `LockMarker` is `!Send`, so `self`'s creator must be the caller
        debug_assert_eq!(thread::current().id(), self.owner.id());

        let old_count = self.count.fetch_add(1, Ordering::Acquire);
        debug_assert!((old_count & PARKED_FLAG) == 0);

        if old_count < EXCLUSIVE_FLAG - 2 {
            // Success
            return;
        }

        self.lock_shared_slow(old_count);
    }

    #[inline]
    unsafe fn try_lock_shared(&self) -> bool {
        // `LockMarker` is `!Send`, so `self`'s creator must be the caller
        debug_assert_eq!(thread::current().id(), self.owner.id());

        let old_count = self.count.fetch_add(1, Ordering::Acquire);
        debug_assert!((old_count & PARKED_FLAG) == 0);

        if old_count < EXCLUSIVE_FLAG - 2 {
            // Success
            return true;
        }

        // Failure; revert the change
        self.count.fetch_sub(1, Ordering::Relaxed);
        false
    }

    #[inline]
    unsafe fn unlock_shared(&self) {
        const PARKED_FLAG_P1: usize = 1 | PARKED_FLAG;
        match self.count.fetch_sub(1, Ordering::Release) {
            PARKED_FLAG_P1 => {
                // The creator thread is parked in `lock_exclusive_slow`
                self.count.store(0, Ordering::Relaxed);
                self.owner.unpark();
            }
            old_count => {
                debug_assert!((old_count & EXCLUSIVE_FLAG) == 0);
                debug_assert!((old_count & !PARKED_FLAG) > 0);
            }
        }
    }

    #[inline]
    unsafe fn lock_exclusive(&self) {
        // `LockMarker` is `!Send`, so `self`'s creator must be the caller
        debug_assert_eq!(thread::current().id(), self.owner.id());

        match self.count.load(Ordering::Acquire) {
            0 => {
                // Success: The store can be non-atomic because of
                // `LockMarker: !Send`
                self.count.store(EXCLUSIVE_FLAG, Ordering::Relaxed);
            }
            old_count => self.lock_exclusive_slow(old_count),
        }
    }

    #[inline]
    unsafe fn try_lock_exclusive(&self) -> bool {
        // `LockMarker` is `!Send`, so `self`'s creator must be the caller
        debug_assert_eq!(thread::current().id(), self.owner.id());

        match self.count.load(Ordering::Acquire) {
            0 => {
                // Success: The store can be non-atomic because of
                // `LockMarker: !Send`
                self.count.store(EXCLUSIVE_FLAG, Ordering::Relaxed);
                true
            }
            _ => {
                // Failure
                false
            }
        }
    }

    #[inline]
    unsafe fn unlock_exclusive(&self) {
        let old_count = self.count.fetch_sub(EXCLUSIVE_FLAG, Ordering::Release);
        debug_assert!(
            old_count == EXCLUSIVE_FLAG ||
            // a portion of `lock_shared` and `try_lock_shared`
            old_count == EXCLUSIVE_FLAG + 1 ||
            // parking of `lock_shared_slow` or `lock_exclusive_slow`
            old_count == PARKED_FLAG | EXCLUSIVE_FLAG
        );

        if old_count == PARKED_FLAG | EXCLUSIVE_FLAG {
            // The creator thread is parked in `lock_shared_slow` or
            // `lock_exclusive_slow`
            self.count.store(0, Ordering::Relaxed);
            self.owner.unpark();
        }
    }
}

impl SyncLock {
    #[cold]
    fn lock_shared_slow(&self, old_count: usize) {
        if old_count == EXCLUSIVE_FLAG - 2 {
            // overflow imminent
            self.count.fetch_sub(1, Ordering::Acquire);
            panic!("lock counter overflow");
        }

        // It's currently locked exclusively
        // (last read value is `old_count`, which was atomically replaced with
        // `old_count + 1` = `EXCLUSIVE_FLAG + 1`)
        debug_assert_eq!(old_count, EXCLUSIVE_FLAG);

        // Park the current thread
        match self.count.compare_exchange(
            EXCLUSIVE_FLAG + 1,
            PARKED_FLAG | EXCLUSIVE_FLAG,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                // Will be unparked when the exclusive lock is released
                while {
                    thread::park();

                    // Check for spurious wake ups
                    self.count.load(Ordering::Acquire) != 0
                } {}
                self.count.store(1, Ordering::Relaxed);
            }
            Err(old_count2) => {
                // It was unlocked before the `compare_exchange`
                debug_assert_eq!(old_count2, 1);
                fence(Ordering::Acquire);
            }
        }
    }

    #[cold]
    fn lock_exclusive_slow(&self, old_count: usize) {
        debug_assert!((old_count & PARKED_FLAG) == 0);

        // Park the current thread
        match self.count.fetch_add(PARKED_FLAG, Ordering::Relaxed) {
            0 => {
                // It was unlocked before the `fetch_add`
                fence(Ordering::Acquire);
            }
            _ => {
                // Will be unparked when the exclusive or shared lock(s) are
                // released
                while {
                    thread::park();

                    // Check for spurious wake ups
                    self.count.load(Ordering::Acquire) != 0
                } {}
            }
        }
        self.count.store(EXCLUSIVE_FLAG, Ordering::Relaxed);
    }
}
