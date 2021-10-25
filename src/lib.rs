//
// Copyright 2018–2021 yvt, all rights reserved.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
//! Requires Rust 1.34.0 or later.
//!
//! This crate provides a cell-like type [`Cryo`] that is similar to `RefCell`
//! except that it constrains the lifetime of its borrowed value
//! through a runtime check mechanism, erasing the compile-time lifetime
//! information. The lock guard [`CryoRef`] created from `Cryo` is
//! `'static` and therefore can be used in various situations that require
//! `'static` types, including:
//!
//!  - Storing [`CryoRef`] temporarily in a `std::any::Any`-compatible container.
//!  - Capturing a reference to create a [Objective-C block](https://crates.io/crates/block).
//!
//! This works by, when a `Cryo` is dropped, not letting the current thread's
//! execution move forward (at least¹) until all references to the expiring
//! `Cryo` are dropped so that none of them can outlive the `Cryo`.
//! This is implemented by [readers-writer locks] under the hood.
//!
//! [readers-writer locks]: https://en.wikipedia.org/wiki/Readers–writer_lock
//!
//! <sub>¹ [`SyncLock`] blocks the current thread's execution on lock failure.
//! [`LocalLock`], on the other hand, panics because it's designed for
//! single-thread use cases and would deadlock otherwise.</sub>
//!
//! # Examples
//!
//! [`with_cryo`], [`Cryo`], and [`LocalLock`] (single-thread lock
//! implementation, used by default):
//!
//! ```
//! # use cryo::*;
//! use std::{thread::spawn, pin::Pin};
//!
//! let cell: usize = 42;
//!
//! // `with_cryo` uses `LocalLock` by default
//! with_cryo(&cell, |cryo: Pin<&Cryo<'_, usize, _>>| {
//!     // Borrow `cryo` and move it into a `'static` closure.
//!     let borrow: CryoRef<usize, _> = cryo.borrow();
//!     let closure: Box<dyn Fn()> =
//!         Box::new(move || { assert_eq!(*borrow, 42); });
//!     closure();
//!     drop(closure);
//!
//!     // Compile-time lifetime works as well.
//!     assert_eq!(*cryo.get(), 42);
//!
//!     // When `cryo` is dropped, it will block until there are no other
//!     // references to `cryo`. In this case, the program will leave
//!     // this block immediately because `CryoRef` has already been dropped.
//! });
//! ```
//!
//! [`with_cryo`], [`Cryo`], and [`SyncLock`] (thread-safe lock implementation):
//!
//! ```
//! # use cryo::*;
//! use std::{thread::spawn, pin::Pin};
//!
//! let cell: usize = 42;
//!
//! // This time we are specifying the lock implementation
//! with_cryo((&cell, lock_ty::<SyncLock>()), |cryo| {
//!     // Borrow `cryo` and move it into a `'static` closure.
//!     // `CryoRef` can be sent to another thread because
//!     // `SyncLock` is thread-safe.
//!     let borrow: CryoRef<usize, _> = cryo.borrow();
//!     spawn(move || { assert_eq!(*borrow, 42); });
//!
//!     // Compile-time lifetime works as well.
//!     assert_eq!(*cryo.get(), 42);
//!
//!     // When `cryo` is dropped, it will block until there are no other
//!     // references to `cryo`. In this case, the program will not leave
//!     // this block until the thread we just spawned completes execution.
//! });
//! ```
//!
//! [`with_cryo`], [`CryoMut`], and [`SyncLock`]:
//!
//! ```
//! # use cryo::*;
//! # use std::{thread::spawn, pin::Pin};
//! # let mut cell: usize = 0;
//! with_cryo((&mut cell, lock_ty::<SyncLock>()), |cryo_mut| {
//!     // Borrow `cryo_mut` and move it into a `'static` closure.
//!     let mut borrow: CryoMutWriteGuard<usize, _> = cryo_mut.write();
//!     spawn(move || { *borrow = 1; });
//!
//!     // When `cryo_mut` is dropped, it will block until there are no other
//!     // references to `cryo_mut`. In this case, the program will not leave
//!     // this block until the thread we just spawned completes execution
//! });
//! assert_eq!(cell, 1);
//! ```
//!
//! **Don't** do these:
//!
//! ```no_run
//! # use cryo::*;
//! # let cell = 0usize;
//! // The following statement will DEADLOCK because it attempts to drop
//! // `Cryo` while a `CryoRef` is still referencing it, and `Cryo`'s
//! // destructor will wait for the `CryoRef` to be dropped first (which
//! // will never happen)
//! let borrow = with_cryo((&cell, lock_ty::<SyncLock>()), |cryo| cryo.borrow());
//! ```
//!
//! ```should_panic
//! # use cryo::*;
//! # let cell = 0usize;
//! // The following statement will ABORT because it attempts to drop
//! // `Cryo` while a `CryoRef` is still referencing it, and `Cryo`'s
//! // destructor will panic, knowing no amount of waiting would cause
//! // the `CryoRef` to be dropped
//! let borrow = with_cryo(&cell, |cryo| cryo.borrow());
//! ```
//!
//! # Caveats
//!
//! - While it's capable of extending the effective lifetime of a reference,
//!   it does not apply to nested references. For example, when
//!   `&'a NonStaticType<'b>` is supplied to `Cryo`'s constructor, the
//!   borrowed type is `CryoRef<NonStaticType<'b>>`, which is still partially
//!   bound to the original lifetime.
//!
//! # Details
//!
//! ## Feature flags
//!
//!  - `std` (enabled by default) enables [`SyncLock`].
//!
//!  - `lock_api` enables the blanket implementation of [`Lock`] on
//!    all types implementing [`lock_api::RawRwLock`], such as
//!    [`spin::RawRwLock`] and [`parking_lot::RawRwLock`].
//!
//!  - `atomic` (enabled by default) enables features that require full atomics,
//!    which is not supported by some targets (detecting such targets is still
//!    unstable ([#32976])). This feature will be deprecated after the
//!    stabilization of #32976.
//!
//! [`spin::RawRwLock`]: https://docs.rs/spin/0.9.0/spin/type.RwLock.html
//! [`parking_lot::RawRwLock`]: https://docs.rs/parking_lot/0.11.1/parking_lot/struct.RawRwLock.html
//! [#32976]: https://github.com/rust-lang/rust/issues/32976
//!
//! ## Overhead
//!
//! `Cryo<T, SyncLock>`'s creation, destruction, borrowing, and unborrowing
//! each take one or two atomic operations in the best cases.
//!
//! Neither of [`SyncLock`] and [`LocalLock`] require dynamic memory allocation.
//!
//! ## Nomenclature
//!
//! From [cryopreservation](https://en.wikipedia.org/wiki/Cryopreservation).
//!
#![warn(rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

use core::{
    cell::UnsafeCell,
    fmt,
    marker::{PhantomData, PhantomPinned},
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
};
use stable_deref_trait::{CloneStableDeref, StableDeref};

#[cfg(feature = "std")]
extern crate std;

// Used by `cryo!`
#[doc(hidden)]
pub use pin_utils::pin_mut;

mod lock;
pub use self::lock::*;

/// A cell-like type that enforces the lifetime restriction of its borrowed
/// value at runtime.
///
/// `Cryo` is a variation of [`CryoMut`] that only can be immutably borrowed.
///
/// When a `Cryo` is dropped, the current thread's execution will be
/// prevented from moving forward (at least) until all references to the
/// expiring `Cryo` are dropped. This ensures that none of the outstanding
/// references can outlive the referent.
///
/// See the [module-level documentation] for more details.
///
/// [module-level documentation]: index.html
pub struct Cryo<'a, T: ?Sized, Lock: crate::Lock> {
    state: UnsafeCell<State<T, Lock>>,
    _phantom: (PhantomData<&'a T>, PhantomPinned),
}

/// `Cryo` may be moved around multiple threads, and on each thread
/// [`CryoRef`] may be created, forming multiple instances of `&T`.
/// Therefore `CryoMut: Send` necessitates `T: Sync`.
///
/// Another interpretation is to think of `Cryo` as `&T` with an erased
/// lifetime.
///
/// `T: Send` is not necessary because `Cryo` doesn't provide `&mut T`.
unsafe impl<'a, T: ?Sized + Sync, Lock: crate::Lock> Send for Cryo<'a, T, Lock> where
    Lock::LockMarker: Send
{
}

/// `&T` can be created from `&Cryo`, so `Cryo: Sync` necessitates `T: Sync`.
///
/// `T: Send` is not necessary because `Cryo` doesn't provide `&mut T`.
unsafe impl<'a, T: ?Sized + Sync, Lock: crate::Lock> Sync for Cryo<'a, T, Lock> where
    Lock::LockMarker: Send
{
}

/// A cell-like type that enforces the lifetime restriction of its borrowed
/// value at runtime.
///
/// `CryoMut` is a variation of [`Cryo`] that can be mutably borrowed.
///
/// When a `CryoMut` is dropped, the current thread's execution will be
/// prevented from moving forward (at least) until all references to the
/// expiring `CryoMut` are dropped. This ensures that none of the outstanding
/// references can outlive the referent.
///
/// See the [module-level documentation] for more details.
///
/// [module-level documentation]: index.html
pub struct CryoMut<'a, T: ?Sized, Lock: crate::Lock> {
    state: UnsafeCell<State<T, Lock>>,
    _phantom: (PhantomData<&'a mut T>, PhantomPinned),
}

/// `CryoMut` may be moved around multiple threads, and on each thread
/// [`CryoMutReadGuard`] may be created, forming multiple instances of `&T`.
/// Therefore `CryoMut: Send` necessitates `T: Sync`.
unsafe impl<'a, T: ?Sized + Send + Sync, Lock: crate::Lock> Send for CryoMut<'a, T, Lock> where
    Lock::LockMarker: Send
{
}

/// `&mut T` may be created from `&CryoMut`, so sending `&CryoMut` to another
/// thread requires `T: Send`.
unsafe impl<'a, T: ?Sized + Send + Sync, Lock: crate::Lock> Sync for CryoMut<'a, T, Lock> where
    Lock::LockMarker: Send
{
}

struct State<T: ?Sized, Lock> {
    data: NonNull<T>,
    lock: Lock,
}

/// The lock guard type of [`Cryo`]. This is currently a type alias but might
/// change in a future.
pub type CryoRef<T, Lock> = CryoMutReadGuard<T, Lock>;

/// The read lock guard type of [`CryoMut`].
pub struct CryoMutReadGuard<T: ?Sized, Lock: crate::Lock> {
    state: NonNull<State<T, Lock>>,
}

/// `CryoMutReadGuard` is essentially `&T` with an indeterminate lifetime.
/// The owning thread may be constrained by [`Lock::UnlockMarker`].
unsafe impl<T: ?Sized + Sync, Lock: crate::Lock> Send for CryoMutReadGuard<T, Lock> where
    Lock::UnlockMarker: Send
{
}

/// `CryoMutReadGuard` is essentially `&T` with an indeterminate lifetime.
unsafe impl<T: ?Sized + Sync, Lock: crate::Lock> Sync for CryoMutReadGuard<T, Lock> {}

/// The write lock guard type of [`CryoMut`].
pub struct CryoMutWriteGuard<T: ?Sized, Lock: crate::Lock> {
    state: NonNull<State<T, Lock>>,
}

/// `CryoMutWriteGuard` is essentially `&mut T` with an indeterminate lifetime.
/// The owning thread may be constrained by [`Lock::UnlockMarker`].
unsafe impl<T: ?Sized + Send, Lock: crate::Lock> Send for CryoMutWriteGuard<T, Lock> where
    Lock::UnlockMarker: Send
{
}

/// `CryoMutWriteGuard` is essentially `&mut T` with an indeterminate lifetime.
unsafe impl<T: ?Sized + Sync, Lock: crate::Lock> Sync for CryoMutWriteGuard<T, Lock> {}

impl<'a, T: ?Sized + 'a, Lock: crate::Lock> Cryo<'a, T, Lock> {
    /// Construct a new `Cryo`.
    ///
    /// # Safety
    ///
    /// The created `Cryo` should be dropped before `x` is invalidated. Pinning
    /// is insufficient as demonstrated below:
    ///
    /// ```rust,should_panic
    /// use cryo::{Cryo, LocalLock};
    /// use std::{pin::Pin, cell::Cell};
    ///
    /// struct OnDrop<F: FnMut()>(u32, F);
    /// impl<F: FnMut()> Drop for OnDrop<F> {
    ///     fn drop(&mut self) { (self.1)(); }
    /// }
    ///
    /// let the_box_dropped = Cell::new(false);
    /// let the_box = Box::new(OnDrop(42, || the_box_dropped.set(true)));
    ///
    /// let cryo = Box::pin(unsafe { Cryo::<_, LocalLock>::new(&*the_box) });
    /// let cryo_ref = Pin::as_ref(&cryo).borrow();
    ///
    /// std::mem::forget(cryo);
    /// drop(the_box);
    ///
    /// // `cryo_ref` is still around, but the referenced `*the_box` is gone.
    /// // Dereferencing `cryo_ref` at this point would cause an undefined
    /// // behavior.
    /// // (`*cryo`, referencing the non-existent `*the_box`, is still present
    /// // in memory as per the pinning guarantee.)
    /// assert!(!the_box_dropped.get());
    /// dbg!(cryo_ref.0);
    /// ```
    ///
    /// In version 0.2.2 and earlier, this method was not `unsafe fn` due to an
    /// oversight.
    #[inline]
    pub unsafe fn new(x: &'a T) -> Self {
        Self {
            state: UnsafeCell::new(State {
                data: NonNull::from(x),
                lock: Lock::new(),
            }),
            _phantom: (PhantomData, PhantomPinned),
        }
    }

    /// Borrow a cell using runtime lifetime rules.
    #[inline]
    pub fn borrow(self: Pin<&Self>) -> CryoRef<T, Lock> {
        // Safety: `Cryo`'s `Send`-ness is constrained by that of `Lock::LockMarker`
        unsafe { (*self.state.get()).lock.lock_shared() };
        CryoRef {
            state: NonNull::new(self.state.get()).unwrap(),
        }
    }

    /// Borrow a cell using compile-time lifetime rules.
    ///
    /// This operation is no-op since `Cryo` only can be immutably borrowed.
    #[inline]
    pub fn get(&self) -> &'a T {
        unsafe { &*(*self.state.get()).data.as_ptr() }
    }
}

impl<'a, T: ?Sized + fmt::Debug, Lock: crate::Lock> fmt::Debug for Cryo<'a, T, Lock> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cryo").field("data", &self.get()).finish()
    }
}

impl<'a, T: ?Sized + 'a, Lock: crate::Lock> Drop for Cryo<'a, T, Lock> {
    #[inline]
    fn drop(&mut self) {
        // Safety: `Cryo`'s `Send`-ness is constrained by that of `Lock::LockMarker`
        unsafe { (*self.state.get()).lock.lock_exclusive() };
        // A write lock ensures there are no other references to
        // the contents
    }
}

impl<'a, T: ?Sized + 'a, Lock: crate::Lock> CryoMut<'a, T, Lock> {
    /// Construct a new `CryoMut`.
    ///
    /// # Safety
    ///
    /// The created `CryoMut` should be dropped before `x` is invalidated.
    /// Pinning is insufficient as demonstrated in [`Cryo::new`]'s example.
    ///
    /// In version 0.2.2 and earlier, this method was not `unsafe fn` due to an
    /// oversight.
    #[inline]
    pub unsafe fn new(x: &'a mut T) -> Self {
        Self {
            state: UnsafeCell::new(State {
                data: NonNull::from(x),
                lock: Lock::new(),
            }),
            _phantom: (PhantomData, PhantomPinned),
        }
    }

    /// Acquire a read (shared) lock on a `CryoMut`.
    #[inline]
    pub fn read(self: Pin<&Self>) -> CryoMutReadGuard<T, Lock> {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `Lock::LockMarker`
        unsafe { (*self.state.get()).lock.lock_shared() };
        CryoMutReadGuard {
            state: NonNull::new(self.state.get()).unwrap(),
        }
    }

    /// Attempt to acquire a read (shared) lock on a `CryoMut`.
    #[inline]
    pub fn try_read(self: Pin<&Self>) -> Option<CryoMutReadGuard<T, Lock>> {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `Lock::LockMarker`
        if unsafe { (*self.state.get()).lock.try_lock_shared() } {
            Some(CryoMutReadGuard {
                state: NonNull::new(self.state.get()).unwrap(),
            })
        } else {
            None
        }
    }

    /// Acquire a write (exclusive) lock on a `CryoMut`.
    #[inline]
    pub fn write(self: Pin<&Self>) -> CryoMutWriteGuard<T, Lock> {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `Lock::LockMarker`
        unsafe { (*self.state.get()).lock.lock_exclusive() };
        CryoMutWriteGuard {
            state: NonNull::new(self.state.get()).unwrap(),
        }
    }

    /// Attempt to acquire a write (exclusive) lock on a `CryoMut`.
    #[inline]
    pub fn try_write(self: Pin<&Self>) -> Option<CryoMutWriteGuard<T, Lock>> {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `Lock::LockMarker`
        if unsafe { (*self.state.get()).lock.try_lock_exclusive() } {
            Some(CryoMutWriteGuard {
                state: NonNull::new(self.state.get()).unwrap(),
            })
        } else {
            None
        }
    }

    /// Attempt to mutably borrow a `CryoMut` using compile-time lifetime rules.
    ///
    /// Returns `None` if the `CryoMut` is already borrowed via
    /// [`CryoMutReadGuard`] or [`CryoMutWriteGuard`].
    #[inline]
    pub fn try_get_mut<'this>(self: Pin<&'this mut Self>) -> Option<&'this mut T> {
        // FIXME: The lifetime elision is not possible here because of
        //        <https://github.com/rust-lang/rust/issues/52675>
        if self.as_ref().try_write().is_some() {
            Some(unsafe { &mut *(*self.state.get()).data.as_ptr() })
        } else {
            None
        }
    }
}

impl<'a, T: ?Sized + fmt::Debug, Lock: crate::Lock> fmt::Debug for CryoMut<'a, T, Lock> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: The constructed `CryoMutReadGuard` doesn't outlive `self`, so
        //         `CryoMutReadGuard::state` won't get dangling.
        let this = unsafe { Pin::new_unchecked(self) };
        if let Some(x) = this.try_read() {
            f.debug_struct("CryoMut").field("data", &&*x).finish()
        } else {
            struct LockedPlaceholder;
            impl fmt::Debug for LockedPlaceholder {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("<locked>")
                }
            }
            f.debug_struct("CryoMut")
                .field("data", &LockedPlaceholder)
                .finish()
        }
    }
}

impl<'a, T: ?Sized + 'a, Lock: crate::Lock> Drop for CryoMut<'a, T, Lock> {
    #[inline]
    fn drop(&mut self) {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `Lock::LockMarker`
        unsafe { (*self.state.get()).lock.lock_exclusive() };
        // A write lock ensures there are no other references to
        // the contents
    }
}

impl<T: ?Sized, Lock: crate::Lock> CryoMutReadGuard<T, Lock> {
    #[inline]
    unsafe fn state(&self) -> &State<T, Lock> {
        self.state.as_ref()
    }
}

impl<T: ?Sized, Lock: crate::Lock> Deref for CryoMutReadGuard<T, Lock> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.state().data.as_ref() }
    }
}

unsafe impl<T: ?Sized, Lock: crate::Lock> StableDeref for CryoMutReadGuard<T, Lock> {}
unsafe impl<T: ?Sized, Lock: crate::Lock> CloneStableDeref for CryoMutReadGuard<T, Lock> {}

impl<T: ?Sized, Lock: crate::Lock> Clone for CryoMutReadGuard<T, Lock> {
    #[inline]
    fn clone(&self) -> Self {
        unsafe {
            self.state().lock.lock_shared();
        }
        Self { state: self.state }
    }
}

impl<T: ?Sized + fmt::Debug, Lock: crate::Lock> fmt::Debug for CryoMutReadGuard<T, Lock> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CryoMutReadGuard")
            .field("data", &&**self)
            .finish()
    }
}

impl<T: ?Sized, Lock: crate::Lock> Drop for CryoMutReadGuard<T, Lock> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.state().lock.unlock_shared();
            // `self.state()` might be invalid beyond this point
        }
    }
}

impl<T: ?Sized, Lock: crate::Lock> CryoMutWriteGuard<T, Lock> {
    #[inline]
    unsafe fn state(&self) -> &State<T, Lock> {
        self.state.as_ref()
    }
}

impl<T: ?Sized, Lock: crate::Lock> Deref for CryoMutWriteGuard<T, Lock> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.state().data.as_ref() }
    }
}

impl<T: ?Sized, Lock: crate::Lock> DerefMut for CryoMutWriteGuard<T, Lock> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.state().data.as_ptr() }
    }
}

unsafe impl<T: ?Sized, Lock: crate::Lock> StableDeref for CryoMutWriteGuard<T, Lock> {}

impl<T: ?Sized + fmt::Debug, Lock: crate::Lock> fmt::Debug for CryoMutWriteGuard<T, Lock> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CryoMutWriteGuard")
            .field("data", &&**self)
            .finish()
    }
}

impl<T: ?Sized, Lock: crate::Lock> Drop for CryoMutWriteGuard<T, Lock> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.state().lock.unlock_exclusive();
            // `self.state()` might be invalid beyond this point
        }
    }
}

/// Construct a [`Cryo`] or [`CryoMut`] and bind it to a local variable.
///
/// # Safety
///
/// **Don't use. This macro is unsound when used inside an `async fn`.** This
/// macro doesn't require `unsafe { ... }` merely not to cause breakage.
///
/// The unsafety is demonstrated in the following code:
///
/// ```should_panic
/// #![allow(deprecated)]
/// use cryo::cryo;
/// use std::{
///     sync::atomic::{AtomicBool, Ordering},
///     future::Future,
///     task::Context,
/// };
///
/// // For demonstration purposes, we want to stop execution when an undefined
/// // behavior is about occur. To this end, we track the `UserType` object's
/// // aliveness with this flag.
/// static IS_USER_TYPE_ALIVE: AtomicBool = AtomicBool::new(true);
///
/// struct UserType(u32);
///
/// impl Drop for UserType {
///     fn drop(&mut self) {
///         IS_USER_TYPE_ALIVE.store(false, Ordering::Relaxed);
///     }
/// }
///
/// // Let there be a `UserType`.
/// let user_type = UserType(42);
///
/// let mut borrow = None;
///
/// // Apply `cryo!` on it inside an `async` block.
/// let mut fut = Box::pin(async {
///     cryo!(let cryo: Cryo<_, cryo::SyncLock> = &user_type);
///
///     // Leak `borrow` to the outer environment
///     borrow = Some(cryo.borrow());
///
///     // This `Future` will get stuck here. Furthermore, we `forget` this
///     // `Future`, so `cryo`'s destructor will never run.
///     std::future::pending::<()>().await
/// });
///
/// // Run the `Future` until it stalls
/// fut.as_mut().poll(&mut Context::from_waker(&futures::task::noop_waker()));
///
/// // Forget the `Future`. The compiler thinks `user_type` is not borrowed,
/// // but in fact `cryo`, which is borrowing it, is still on memory.
/// std::mem::forget(fut);
///
/// // And `user_type` is gone. Now `cryo` is dangling.
/// drop(user_type);
///
/// // But we can still access the dead `user_type` through `borrow`!
/// let borrow = borrow.unwrap();
/// assert!(
///     IS_USER_TYPE_ALIVE.load(Ordering::Relaxed),
///     "`cryo!` is supposed to keep us safe, isn't it?"  // well, it betrayed us. (panics)
/// );
/// assert_eq!(borrow.0, 42);  // UB
/// ```
///
/// # Examples
///
/// ```
/// #![allow(deprecated)]
/// use cryo::cryo;
/// cryo!(let cryo: Cryo<u8> = &42);
/// assert_eq!(*cryo.borrow(), 42);
/// ```
///
/// ```
/// #![allow(deprecated)]
/// use cryo::cryo;
/// let mut var = 42;
/// {
///     cryo!(let cryo: CryoMut<u8> = &mut var);
///     *cryo.write() = 84;
/// }
/// assert_eq!(var, 84);
/// ```
///
/// The lock implementation can be specified by an extra generic argument. It
/// defaults to [`LocalLock`] when unspecified.
///
/// ```
/// #![allow(deprecated)]
/// use cryo::cryo;
/// use std::thread::spawn;
/// cryo!(let cryo: Cryo<_, cryo::SyncLock> = &42);
/// let borrow = cryo.borrow();
/// spawn(move || {
///     assert_eq!(*borrow, 42);
/// });
/// ```
#[deprecated = "`cryo!` is unsound when used inside `async fn` and will be \
                removed in a future version"]
#[macro_export]
macro_rules! cryo {
    // empty (base case for the recursion)
    () => {};

    // process multiple declarations
    ($(#[$attr:meta])* let $name:ident: $Cryo:ident< $t:ty $(, $Lock:ty)? > = $init:expr; $($rest:tt)*) => (
        $crate::cryo!($(#[$attr])* let $name: $Cryo<$t $(, $Lock)?> = $init);
        $crate::cryo!($($rest)*);
    );

    // handle a single declaration
    ($(#[$attr:meta])* let $name:ident: $Cryo:ident< $t:ty $(, $Lock:ty)? > = $init:expr) => (
        let cryo = unsafe { $crate::$Cryo::<$t, $crate::__LockOrDefault!($(($Lock))?)>::new($init) };
        $crate::pin_mut!(cryo);
        let $name = cryo.into_ref();
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __LockOrDefault {
    // Custom
    (($t:ty)) => {
        $t
    };
    // Default
    () => {
        $crate::LocalLock
    };
}

/// The trait for types that can be wrapped with [`Cryo`] or [`CryoMut`].
pub trait WithCryo: private::Sealed + Sized {
    type Cryo;

    /// Call a given function with a constructed [`Cryo`] or [`CryoMut`].
    ///
    /// This method is also exposed as a global function [`with_cryo`].
    fn with_cryo<R>(self, f: impl FnOnce(Pin<&Self::Cryo>) -> R) -> R;
}

mod private {
    pub trait Sealed {}
    impl<T> Sealed for &T {}
    impl<T> Sealed for &mut T {}
    impl<T, Lock> Sealed for (&T, Lock) {}
    impl<T, Lock> Sealed for (&mut T, Lock) {}
}

/// Constructs [`Cryo`] with [`LocalLock`] as its [`Lock`] type.
impl<'a, T> WithCryo for &'a T {
    type Cryo = Cryo<'a, T, LocalLock>;

    #[inline]
    fn with_cryo<R>(self, f: impl FnOnce(Pin<&Self::Cryo>) -> R) -> R {
        let c = unsafe { Self::Cryo::new(self) };
        pin_mut!(c);
        f(c.as_ref())
    }
}

/// Constructs [`CryoMut`] with [`LocalLock`] as its [`Lock`] type.
impl<'a, T> WithCryo for &'a mut T {
    type Cryo = CryoMut<'a, T, LocalLock>;

    #[inline]
    fn with_cryo<R>(self, f: impl FnOnce(Pin<&Self::Cryo>) -> R) -> R {
        let c = unsafe { Self::Cryo::new(self) };
        pin_mut!(c);
        f(c.as_ref())
    }
}

/// Constructs [`Cryo`] with a specified [`Lock`] type.
impl<'a, T, Lock: crate::Lock> WithCryo for (&'a T, LockTyMarker<Lock>) {
    type Cryo = Cryo<'a, T, Lock>;

    #[inline]
    fn with_cryo<R>(self, f: impl FnOnce(Pin<&Self::Cryo>) -> R) -> R {
        let c = unsafe { Self::Cryo::new(self.0) };
        pin_mut!(c);
        f(c.as_ref())
    }
}

/// Constructs [`CryoMut`] with a specified [`Lock`] type.
impl<'a, T, Lock: crate::Lock> WithCryo for (&'a mut T, LockTyMarker<Lock>) {
    type Cryo = CryoMut<'a, T, Lock>;

    #[inline]
    fn with_cryo<R>(self, f: impl FnOnce(Pin<&Self::Cryo>) -> R) -> R {
        let c = unsafe { Self::Cryo::new(self.0) };
        pin_mut!(c);
        f(c.as_ref())
    }
}

/// Marker type to specify the `Lock` type to use with [`with_cryo`].
pub struct LockTyMarker<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> Default for LockTyMarker<T> {
    #[inline]
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Construct a [`LockTyMarker`].
#[inline]
pub const fn lock_ty<T>() -> LockTyMarker<T> {
    LockTyMarker(PhantomData)
}

/// Call a given function with a constructed [`Cryo`] or [`CryoMut`].
///
/// This function is a thin wrapper of [`WithCryo::with_cryo`].
///
/// See [the crate documentation](crate) for examples.
#[inline]
pub fn with_cryo<T: WithCryo, R>(x: T, f: impl FnOnce(Pin<&T::Cryo>) -> R) -> R {
    x.with_cryo(f)
}
