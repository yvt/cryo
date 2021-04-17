//
// Copyright 2018–2021 yvt, all rights reserved.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
//! *Extend the lifetime of a reference. Safely.*
//!
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
//! This works by, when a `Cryo` is dropped, blocking the current thread until
//! all references to the contained value are dropped so that none of them can
//! outlive the cell.
//!
//! # Examples
//!
//! [`cryo!`] and [`Cryo`]:
//!
//! ```
//! # use cryo::*;
//! use std::{thread::spawn, pin::Pin};
//!
//! let cell: usize = 42;
//!
//! {
//!     cryo!(let cryo: Cryo<usize> = &cell);
//!
//!     // Borrow `cryo` and move it into a `'static` closure.
//!     let borrow: CryoRef<usize, _> = cryo.borrow();
//!     spawn(move || { assert_eq!(*borrow, 42); });
//!
//!     // Compile-time lifetime works as well.
//!     assert_eq!(*cryo.get(), 42);
//!
//!     // When `cryo` is dropped, it will block until there are no other
//!     // references to `cryo`. In this case, the program will not leave
//!     // this block until the thread we just spawned completes execution.
//! }
//! ```
//!
//! [`cryo!`] and [`CryoMut`]:
//!
//! ```
//! # use cryo::*;
//! # use std::{thread::spawn, pin::Pin};
//! # let mut cell: usize = 0;
//! {
//!     cryo!(let cryo_mut: CryoMut<usize> = &mut cell);
//!
//!     // Borrow `cryo_mut` and move it into a `'static` closure.
//!     let mut borrow: CryoMutWriteGuard<usize, _> = cryo_mut.write();
//!     spawn(move || { *borrow = 1; });
//!
//!     // When `cryo_mut` is dropped, it will block until there are no other
//!     // references to `cryo_mut`. In this case, the program will not leave
//!     // this block until the thread we just spawned completes execution
//! }
//! assert_eq!(cell, 1);
//! ```
//!
//! **Don't** do this:
//!
//! ```no_run
//! # use cryo::*;
//! # let cell = 0usize;
//! // The following statement will deadlock because it attempts to drop
//! // `Cryo` while a `CryoRef` is still referencing it
//! let borrow = {
//!     cryo!(let cryo: Cryo<_> = &cell);
//!     cryo.borrow()
//! };
//! ```
//!
//! # Caveats
//!
//! - While it's capable of extending the effective lifetime of a reference,
//!   it does not apply to nested references. For example, when
//!   `&'a NonStaticType<'b>` is supplied to the `Cryo`'s constructor, the
//!   borrowed type is `CryoRef<NonStaticType<'b>>`, which is still partially
//!   bound to the original lifetime.
//!
//! # Details
//!
//! ## Feature flags
//!
//!  - `std` (enabled by default) enables [`StdRawRwLock`].
//!
//!  - `lock_api` enables the blanket implementation of [`RawRwLock`] on
//!    all types implementing [`lock_api::RawRwLock`], such as
//!    [`parking_lot::RawRwLock`].
//!
//! [`parking_lot::RawRwLock`]: https://docs.rs/parking_lot/0.11.1/parking_lot/struct.RawRwLock.html
//!
//! ## Overhead
//!
//! `Cryo<T, StdRawRwLock>`'s creation, destruction, borrowing, and unborrowing
//! each take one or two atomic operations in the best cases.
//!
//! ## Nomenclature
//!
//! From [cryopreservation](https://en.wikipedia.org/wiki/Cryopreservation).
//!
#![warn(rust_2018_idioms)]
#![no_std]

use core::{
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

mod raw_rwlock;
pub use self::raw_rwlock::*;

/// A cell-like type that enforces the lifetime restriction of its borrowed
/// value at runtime.
///
/// `Cryo` is a variation of [`CryoMut`] that only can be immutably borrowed.
///
/// When a `Cryo` is dropped, the current thread will be blocked until all
/// references to the contained value are dropped. This ensures that none of
/// the references can outlive the referent.
///
/// See the [module-level documentation] for more details.
///
/// [module-level documentation]: index.html
pub struct Cryo<'a, T: ?Sized, RwLock: RawRwLock> {
    state: State<T, RwLock>,
    _phantom: (PhantomData<&'a T>, PhantomPinned),
}

unsafe impl<'a, T: ?Sized + Send, RwLock: RawRwLock> Send for Cryo<'a, T, RwLock> where
    RwLock::LockMarker: Send
{
}
unsafe impl<'a, T: ?Sized + Send + Sync, RwLock: RawRwLock> Sync for Cryo<'a, T, RwLock> where
    RwLock::LockMarker: Send
{
}

/// A cell-like type that enforces the lifetime restriction of its borrowed
/// value at runtime.
///
/// `CryoMut` is a variation of [`Cryo`] that can be mutably borrowed.
///
/// When a `CryoMut` is dropped, the current thread will be blocked until all
/// references to the contained value are dropped. This ensures that none of
/// the references can outlive the referent.
///
/// See the [module-level documentation] for more details.
///
/// [module-level documentation]: index.html
pub struct CryoMut<'a, T: ?Sized, RwLock: RawRwLock> {
    state: State<T, RwLock>,
    _phantom: (PhantomData<&'a mut T>, PhantomPinned),
}

unsafe impl<'a, T: ?Sized + Send, RwLock: RawRwLock> Send for CryoMut<'a, T, RwLock> where
    RwLock::LockMarker: Send
{
}
unsafe impl<'a, T: ?Sized + Send + Sync, RwLock: RawRwLock> Sync for CryoMut<'a, T, RwLock> where
    RwLock::LockMarker: Send
{
}

struct State<T: ?Sized, RwLock> {
    data: NonNull<T>,
    lock: RwLock,
}

/// The lock guard type of [`Cryo`]. This is currently a type alias but might
/// change in a future.
pub type CryoRef<T, RwLock> = CryoMutReadGuard<T, RwLock>;

/// The read lock guard type of [`CryoMut`].
pub struct CryoMutReadGuard<T: ?Sized, RwLock: RawRwLock> {
    state: NonNull<State<T, RwLock>>,
}

unsafe impl<T: ?Sized + Send, RwLock: RawRwLock> Send for CryoMutReadGuard<T, RwLock> where
    RwLock::UnlockMarker: Send
{
}
unsafe impl<T: ?Sized + Send + Sync, RwLock: RawRwLock> Sync for CryoMutReadGuard<T, RwLock> where
    RwLock::UnlockMarker: Send
{
}

/// The write lock guard type of [`CryoMut`].
pub struct CryoMutWriteGuard<T: ?Sized, RwLock: RawRwLock> {
    state: NonNull<State<T, RwLock>>,
}

unsafe impl<T: ?Sized + Send, RwLock: RawRwLock> Send for CryoMutWriteGuard<T, RwLock> where
    RwLock::UnlockMarker: Send
{
}

impl<'a, T: ?Sized + 'a, RwLock: RawRwLock> Cryo<'a, T, RwLock> {
    /// Construct a new `Cryo`.
    #[inline]
    pub fn new(x: &'a T) -> Self {
        Self {
            state: State {
                data: NonNull::from(x),
                lock: RwLock::new(),
            },
            _phantom: (PhantomData, PhantomPinned),
        }
    }

    /// Borrow a cell using runtime lifetime rules.
    #[inline]
    pub fn borrow(self: Pin<&Self>) -> CryoRef<T, RwLock> {
        // Safety: `Cryo`'s `Send`-ness is constrained by that of `RwLock::LockMarker`
        unsafe { self.state.lock.lock_shared() };
        CryoRef {
            state: NonNull::from(&self.state),
        }
    }

    /// Borrow a cell using compile-time lifetime rules.
    ///
    /// This operation is no-op since `Cryo` only can be immutably borrowed.
    #[inline]
    pub fn get(&self) -> &'a T {
        unsafe { &*self.state.data.as_ptr() }
    }
}

impl<'a, T: ?Sized + fmt::Debug, RwLock: RawRwLock> fmt::Debug for Cryo<'a, T, RwLock> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cryo").field("data", &self.get()).finish()
    }
}

impl<'a, T: ?Sized + 'a, RwLock: RawRwLock> Drop for Cryo<'a, T, RwLock> {
    #[inline]
    fn drop(&mut self) {
        // Safety: `Cryo`'s `Send`-ness is constrained by that of `RwLock::LockMarker`
        unsafe { self.state.lock.lock_exclusive() };
        // A write lock ensures there are no other references to
        // the contents
    }
}

impl<'a, T: ?Sized + 'a, RwLock: RawRwLock> CryoMut<'a, T, RwLock> {
    /// Construct a new `CryoMut`.
    #[inline]
    pub fn new(x: &'a mut T) -> Self {
        Self {
            state: State {
                data: NonNull::from(x),
                lock: RawRwLock::new(),
            },
            _phantom: (PhantomData, PhantomPinned),
        }
    }

    /// Acquire a read (shared) lock on a `CryoMut`.
    #[inline]
    pub fn read(self: Pin<&Self>) -> CryoMutReadGuard<T, RwLock> {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `RwLock::LockMarker`
        unsafe { self.state.lock.lock_shared() };
        CryoMutReadGuard {
            state: NonNull::from(&self.state),
        }
    }

    /// Attempt to acquire a read (shared) lock on a `CryoMut`.
    #[inline]
    pub fn try_read(self: Pin<&Self>) -> Option<CryoMutReadGuard<T, RwLock>> {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `RwLock::LockMarker`
        if unsafe { self.state.lock.try_lock_shared() } {
            Some(CryoMutReadGuard {
                state: NonNull::from(&self.state),
            })
        } else {
            None
        }
    }

    /// Acquire a write (exclusive) lock on a `CryoMut`.
    #[inline]
    pub fn write(self: Pin<&Self>) -> CryoMutWriteGuard<T, RwLock> {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `RwLock::LockMarker`
        unsafe { self.state.lock.lock_exclusive() };
        CryoMutWriteGuard {
            state: NonNull::from(&self.state),
        }
    }

    /// Attempt to acquire a write (exclusive) lock on a `CryoMut`.
    #[inline]
    pub fn try_write(self: Pin<&Self>) -> Option<CryoMutWriteGuard<T, RwLock>> {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `RwLock::LockMarker`
        if unsafe { self.state.lock.try_lock_exclusive() } {
            Some(CryoMutWriteGuard {
                state: NonNull::from(&self.state),
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
            Some(unsafe { &mut *self.state.data.as_ptr() })
        } else {
            None
        }
    }
}

impl<'a, T: ?Sized + fmt::Debug, RwLock: RawRwLock> fmt::Debug for CryoMut<'a, T, RwLock> {
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

impl<'a, T: ?Sized + 'a, RwLock: RawRwLock> Drop for CryoMut<'a, T, RwLock> {
    #[inline]
    fn drop(&mut self) {
        // Safety: `CryoMut`'s `Send`-ness is constrained by that of `RwLock::LockMarker`
        unsafe { self.state.lock.lock_exclusive() };
        // A write lock ensures there are no other references to
        // the contents
    }
}

impl<T: ?Sized, RwLock: RawRwLock> CryoMutReadGuard<T, RwLock> {
    #[inline]
    unsafe fn state(&self) -> &State<T, RwLock> {
        self.state.as_ref()
    }
}

impl<T: ?Sized, RwLock: RawRwLock> Deref for CryoMutReadGuard<T, RwLock> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.state().data.as_ref() }
    }
}

unsafe impl<T: ?Sized, RwLock: RawRwLock> StableDeref for CryoMutReadGuard<T, RwLock> {}
unsafe impl<T: ?Sized, RwLock: RawRwLock> CloneStableDeref for CryoMutReadGuard<T, RwLock> {}

impl<T: ?Sized, RwLock: RawRwLock> Clone for CryoMutReadGuard<T, RwLock> {
    #[inline]
    fn clone(&self) -> Self {
        unsafe {
            self.state().lock.lock_shared();
        }
        Self { state: self.state }
    }
}

impl<T: ?Sized + fmt::Debug, RwLock: RawRwLock> fmt::Debug for CryoMutReadGuard<T, RwLock> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CryoMutReadGuard")
            .field("data", &&**self)
            .finish()
    }
}

impl<T: ?Sized, RwLock: RawRwLock> Drop for CryoMutReadGuard<T, RwLock> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.state().lock.unlock_shared();
            // `self.state()` might be invalid beyond this point
        }
    }
}

impl<T: ?Sized, RwLock: RawRwLock> CryoMutWriteGuard<T, RwLock> {
    #[inline]
    unsafe fn state(&self) -> &State<T, RwLock> {
        self.state.as_ref()
    }
}

impl<T: ?Sized, RwLock: RawRwLock> Deref for CryoMutWriteGuard<T, RwLock> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.state().data.as_ref() }
    }
}

impl<T: ?Sized, RwLock: RawRwLock> DerefMut for CryoMutWriteGuard<T, RwLock> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.state().data.as_ptr() }
    }
}

unsafe impl<T: ?Sized, RwLock: RawRwLock> StableDeref for CryoMutWriteGuard<T, RwLock> {}

impl<T: ?Sized + fmt::Debug, RwLock: RawRwLock> fmt::Debug for CryoMutWriteGuard<T, RwLock> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CryoMutWriteGuard")
            .field("data", &&**self)
            .finish()
    }
}

impl<T: ?Sized, RwLock: RawRwLock> Drop for CryoMutWriteGuard<T, RwLock> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.state().lock.unlock_exclusive();
            // `self.state()` might be invalid beyond this point
        }
    }
}

/// Construct a [`Cryo`] or [`CryoMut`] and bind it to a local variable.
#[macro_export]
macro_rules! cryo {
    // empty (base case for the recursion)
    () => {};

    // process multiple declarations
    ($(#[$attr:meta])* let $name:ident: $Cryo:ident< $t:ty $(, $RwLock:ty)? > = $init:expr; $($rest:tt)*) => (
        $crate::cryo!($(#[$attr])* let $name: $Cryo<$t $(, $RwLock)?> = $init);
        $crate::cryo!($($rest)*);
    );

    // handle a single declaration
    ($(#[$attr:meta])* let $name:ident: $Cryo:ident< $t:ty $(, $RwLock:ty)? > = $init:expr) => (
        let cryo = $crate::$Cryo::<$t, $crate::__RwLockOrDefault!($(($RwLock))?)>::new($init);
        $crate::pin_mut!(cryo);
        let $name = cryo.into_ref();
    );
}

#[doc(hidden)]
#[cfg(feature = "std")]
#[macro_export]
macro_rules! __RwLockOrDefault {
    // Custom
    (($t:ty)) => {
        $t
    };
    // Default
    () => {
        $crate::StdRawRwLock
    };
}

#[doc(hidden)]
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! __RwLockOrDefault {
    // Custom
    (($t:ty)) => {
        $t
    };
    // Default
    () => {
        compile_error!(
            "`std` feature is disabled; the default RwLock implementation \
            (`StdRawRwLock`) is unavailable. please specify one (e.g., \
            `Cryo<_, MyRawRwLock>`)
            "
        )
    };
}
