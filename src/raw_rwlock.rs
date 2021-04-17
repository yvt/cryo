//
// Copyright 2018–2021 yvt, all rights reserved.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
use core::marker::PhantomData;

#[cfg(feature = "std")]
mod stdimp;
#[cfg(feature = "std")]
pub use self::stdimp::*;

mod local;
pub use self::local::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendMarker(());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoSendMarker(PhantomData<*mut ()>);

/// A trait for readers-writer locks.
pub unsafe trait RawRwLock {
    fn new() -> Self;

    /// The `Send`-ness of this type indicates whether a lock can only be
    /// acquired by the same thread as `self`'s creator.
    type LockMarker;
    /// The `Send`-ness of this type indicates whether a lock can only be
    /// released by the same thread as the one that acquired it.
    type UnlockMarker;

    /// Acquire a shared lock, blocking the current thread until the lock
    /// is acquired.
    ///
    /// # Safety
    ///
    /// If [`Self::LockMarker`] is `!`[`Send`], the current thread must be the
    /// same one as `self`'s creator.
    unsafe fn lock_shared(&self);

    /// Acquire a shared lock.
    ///
    /// # Safety
    ///
    /// If [`Self::LockMarker`] is `!`[`Send`], the current thread must be the
    /// same one as `self`'s creator.
    unsafe fn try_lock_shared(&self) -> bool;

    /// Release a shared lock.
    ///
    /// # Safety
    ///
    /// There must be a shared lock to release.
    ///
    /// If [`Self::UnlockMarker`] is `!`[`Send`], the current thread must own a
    /// shared lock on `self`.
    unsafe fn unlock_shared(&self);

    /// Acquire an exclusive lock, blocking the current thread until the lock
    /// is acquired.
    ///
    /// # Safety
    ///
    /// If [`Self::LockMarker`] is `!`[`Send`], the current thread must be the
    /// same one as `self`'s creator.
    unsafe fn lock_exclusive(&self);

    /// Acquire an exclusive lock.
    ///
    /// # Safety
    ///
    /// If [`Self::LockMarker`] is `!`[`Send`], the current thread must be the
    /// same one as `self`'s creator.
    unsafe fn try_lock_exclusive(&self) -> bool;

    /// Release an exclusive lock.
    ///
    /// # Safety
    ///
    /// There must be an exclusive lock to release.
    ///
    /// If [`Self::UnlockMarker`] is `!`[`Send`], the current thread must own an
    /// exclusive lock on `self`.
    unsafe fn unlock_exclusive(&self);
}

#[cfg(feature = "lock_api")]
/// This crate's `RawRwLock` is automatically implemented for types implementing
/// [`lock_api::RawRwLock`]
unsafe impl<T: lock_api::RawRwLock> RawRwLock for T {
    type LockMarker = PhantomData<Self>;
    type UnlockMarker = T::GuardMarker;

    #[inline]
    fn new() -> Self {
        <Self as lock_api::RawRwLock>::INIT
    }

    #[inline]
    unsafe fn lock_shared(&self) {
        lock_api::RawRwLock::lock_shared(self)
    }

    #[inline]
    unsafe fn try_lock_shared(&self) -> bool {
        lock_api::RawRwLock::try_lock_shared(self)
    }

    #[inline]
    unsafe fn unlock_shared(&self) {
        lock_api::RawRwLock::unlock_shared(self)
    }

    #[inline]
    unsafe fn lock_exclusive(&self) {
        lock_api::RawRwLock::lock_exclusive(self)
    }

    #[inline]
    unsafe fn try_lock_exclusive(&self) -> bool {
        lock_api::RawRwLock::try_lock_exclusive(self)
    }

    #[inline]
    unsafe fn unlock_exclusive(&self) {
        lock_api::RawRwLock::unlock_exclusive(self)
    }
}
