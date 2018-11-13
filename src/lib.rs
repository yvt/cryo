//
// Copyright 2018 yvt, all rights reserved.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
//! *Extend the lifetime of a reference. Safely.*
//!
//! This crate provides a cell-like type [`Cryo`] that is similar to `RefCell`
//! except that it constrains the lifetime of its borrowed value
//! through a runtime check mechanism. Intuitively, it effectively extends the
//! lifetime of a reference to `'static`.
//!
//! This works by, when a `Cryo` is dropped, blocking the current thread until
//! all references to the contained value are dropped so that none of them can
//! outlive the cell. Since it's possible to skip `drop`, the constructor of
//! `Cryo` is marked as `unsafe`. Safe utility functions [`with_cryo`] and
//! [`with_cryo_mut`] ensure that cells are dropped properly.
//!
//! # Examples
//!
//! [`with_cryo`] and [`Cryo`]:
//!
//! ```
//! # use cryo::*;
//! use std::thread::spawn;
//!
//! let cell: usize = 42;
//!
//! with_cryo(&cell, |cryo: &Cryo<usize>| {
//!     // Borrow `cryo` and move it into a `'static` closure.
//!     let borrow: CryoRef<usize> = cryo.borrow();
//!     spawn(move || { assert_eq!(*borrow, 42); });
//!
//!     // Compile-time lifetime works as well.
//!     assert_eq!(*cryo.get(), 42);
//!
//!     // When `cryo` is dropped, it will block until there are no other
//!     // references to `cryo`. In this case, `with_cryo` will not return
//!     // until the thread we just spawned completes execution.
//! });
//! ```
//!
//! [`with_cryo_mut`] and [`CryoMut`]:
//!
//! ```
//! # use cryo::*;
//! # use std::thread::spawn;
//! # let mut cell: usize = 0;
//! with_cryo_mut(&mut cell, |cryo_mut: &mut CryoMut<usize>| {
//!     // Borrow `cryo_mut` and move it into a `'static` closure.
//!     let mut borrow: CryoMutWriteGuard<usize> = cryo_mut.write();
//!     spawn(move || { *borrow = 1; });
//!
//!     // When `cryo_mut` is dropped, it will block until there are no other
//!     // references to `cryo_mut`.  In this case, `with_cryo_mut` will not
//!     // return until the thread we just spawned completes execution.
//! });
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
//! let borrow = with_cryo(&cell, |cryo| cryo.borrow());
//! ```
//!
//! # Caveats
//!
//! - While it's capable of extending the effective lifetime of a reference,
//!   it does not apply to nested references. For example, when
//!   `&'a NonStaticType<'b>` is given, the borrowed type is
//!   `CryoRef<NonStaticType<'b>>`, which is still bound to the original
//!   lifetime.
//!
//! # Details
//!
//! ## Feature flags
//!
//!  - `use_parking_lot` — Specifies to use `parking_lot` instead of `std::sync`.
//!
//! ## Overhead
//!
//! `Cryo<T>` incurs moderate overhead due to the uses of `Mutex` and
//! `Condvar`. This can be alleviated somewhat by using the `use_parking_lot`
//! feature flag.
//!
//! ## Nomenclature
//!
//! From [cryopreservation](https://en.wikipedia.org/wiki/Cryopreservation).
//!

use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "parking_lot")]
extern crate parking_lot;

mod raw_rwlock;
use self::raw_rwlock::RawRwLock;

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
pub struct Cryo<'a, T: ?Sized + 'a> {
    state: State<T>,
    _phantom: PhantomData<&'a T>,
}

unsafe impl<'a, T: ?Sized + Send> Send for Cryo<'a, T> {}
unsafe impl<'a, T: ?Sized + Send + Sync> Sync for Cryo<'a, T> {}

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
pub struct CryoMut<'a, T: ?Sized + 'a> {
    state: State<T>,
    _phantom: PhantomData<&'a mut T>,
}

unsafe impl<'a, T: ?Sized + Send> Send for CryoMut<'a, T> {}
unsafe impl<'a, T: ?Sized + Send + Sync> Sync for CryoMut<'a, T> {}

struct State<T: ?Sized> {
    data: *mut T,
    lock: RawRwLock,
}

/// The lock guard type of [`Cryo`]. This is currently a type alias but might
/// change in a future.
pub type CryoRef<T> = CryoMutReadGuard<T>;

/// The read lock guard type of [`CryoMut`].
pub struct CryoMutReadGuard<T: ?Sized> {
    state: *const State<T>,
}

unsafe impl<T: ?Sized + Send> Send for CryoMutReadGuard<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for CryoMutReadGuard<T> {}

/// The write lock guard type of [`CryoMut`].
pub struct CryoMutWriteGuard<T: ?Sized> {
    state: *const State<T>,
}

unsafe impl<T: ?Sized + Send> Send for CryoMutWriteGuard<T> {}

impl<'a, T: ?Sized + 'a> Cryo<'a, T> {
    /// Construct a new `Cryo`.
    ///
    /// # Safety
    ///
    ///  - The constructed `Cryo` must not be moved around. This might result
    ///    in a dangling pointer in `CryoRef`.
    ///
    ///  - The constructed `Cryo` must not be disposed without dropping
    ///    (e.g., passed to `std::mem::forget`). This might result
    ///    in a dangling pointer in `CryoRef`.
    ///
    pub unsafe fn new(x: &'a T) -> Self {
        Self {
            state: State {
                data: x as *const T as *mut T,
                lock: RawRwLock::new(),
            },
            _phantom: PhantomData,
        }
    }

    /// Borrow a cell using runtime lifetime rules.
    pub fn borrow(&self) -> CryoRef<T> {
        self.state.lock.raw_read();
        CryoRef { state: &self.state }
    }

    /// Borrow a cell using compile-time lifetime rules.
    ///
    /// This operation is no-op since `Cryo` only can be immutably borrowed.
    pub fn get(&self) -> &'a T {
        unsafe { &*(self.state.data as *const T) }
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for Cryo<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Cryo").field("data", &self.get()).finish()
    }
}

impl<'a, T: ?Sized + 'a> Drop for Cryo<'a, T> {
    fn drop(&mut self) {
        self.state.lock.raw_write();
        // A write lock ensures there are no other references to
        // the contents
    }
}

impl<'a, T: ?Sized + 'a> CryoMut<'a, T> {
    /// Construct a new `CryoMut`.
    ///
    /// # Safety
    ///
    ///  - The constructed `CryoMut` must not be moved around. This might result
    ///    in a dangling pointer in `CryoMut*Guard`.
    ///
    ///  - The constructed `CryoMut` must not be disposed without dropping
    ///    (e.g., passed to `std::mem::forget`). This might result
    ///    in a dangling pointer in `CryoMut*Guard`.
    ///
    pub unsafe fn new(x: &'a mut T) -> Self {
        Self {
            state: State {
                data: x,
                lock: RawRwLock::new(),
            },
            _phantom: PhantomData,
        }
    }

    /// Acquire a read (shared) lock on a `CryoMut`.
    pub fn read(&self) -> CryoMutReadGuard<T> {
        self.state.lock.raw_read();
        CryoMutReadGuard { state: &self.state }
    }

    /// Attempt to acquire a read (shared) lock on a `CryoMut`.
    pub fn try_read(&self) -> Option<CryoMutReadGuard<T>> {
        if self.state.lock.raw_try_read() {
            Some(CryoMutReadGuard { state: &self.state })
        } else {
            None
        }
    }

    /// Acquire a write (exclusive) lock on a `CryoMut`.
    pub fn write(&self) -> CryoMutWriteGuard<T> {
        self.state.lock.raw_write();
        CryoMutWriteGuard { state: &self.state }
    }

    /// Attempt to acquire a write (exclusive) lock on a `CryoMut`.
    pub fn try_write(&self) -> Option<CryoMutWriteGuard<T>> {
        if self.state.lock.raw_try_write() {
            Some(CryoMutWriteGuard { state: &self.state })
        } else {
            None
        }
    }

    /// Attempt to mutably borrow a `CryoMut` using compile-time lifetime rules.
    ///
    /// Returns `None` if the `CryoMut` is already borrowed via
    /// [`CryoMutReadGuard`] or [`CryoMutWriteGuard`].
    pub fn try_get_mut(&mut self) -> Option<&mut T> {
        if self.state.lock.raw_try_write() {
            unsafe {
                self.state.lock.raw_unlock_write();
            }
            Some(unsafe { &mut *self.state.data })
        } else {
            None
        }
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for CryoMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(x) = self.try_read() {
            f.debug_struct("CryoMut").field("data", &&*x).finish()
        } else {
            struct LockedPlaceholder;
            impl fmt::Debug for LockedPlaceholder {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("<locked>")
                }
            }
            f.debug_struct("CryoMut")
                .field("data", &LockedPlaceholder)
                .finish()
        }
    }
}

impl<'a, T: ?Sized + 'a> Drop for CryoMut<'a, T> {
    fn drop(&mut self) {
        self.state.lock.raw_write();
        // A write lock ensures there are no other references to
        // the contents
    }
}

impl<T: ?Sized> CryoMutReadGuard<T> {
    unsafe fn state(&self) -> &State<T> {
        &*self.state
    }
}

impl<T: ?Sized> Deref for CryoMutReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.state().data as *const T) }
    }
}

impl<T: ?Sized> Clone for CryoMutReadGuard<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.state().lock.raw_read();
        }
        Self { state: self.state }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for CryoMutReadGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CryoMutReadGuard")
            .field("data", &&**self)
            .finish()
    }
}

impl<T: ?Sized> Drop for CryoMutReadGuard<T> {
    fn drop(&mut self) {
        unsafe {
            self.state().lock.raw_unlock_read();
            // `self.state()` might be invalid beyond this point
        }
    }
}

impl<T: ?Sized> CryoMutWriteGuard<T> {
    unsafe fn state(&self) -> &State<T> {
        &*self.state
    }
}

impl<T: ?Sized> Deref for CryoMutWriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.state().data as *const T) }
    }
}

impl<T: ?Sized> DerefMut for CryoMutWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.state().data }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for CryoMutWriteGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CryoMutWriteGuard")
            .field("data", &&**self)
            .finish()
    }
}

impl<T: ?Sized> Drop for CryoMutWriteGuard<T> {
    fn drop(&mut self) {
        unsafe {
            self.state().lock.raw_unlock_write();
            // `self.state()` might be invalid beyond this point
        }
    }
}

/// Call a given function with a constructed [`Cryo`].
pub fn with_cryo<T, R>(x: &T, f: impl FnOnce(&Cryo<T>) -> R) -> R {
    f(&unsafe { Cryo::new(x) })
}

/// Call a given function with a constructed [`CryoMut`].
pub fn with_cryo_mut<T, R>(x: &mut T, f: impl FnOnce(&mut CryoMut<T>) -> R) -> R {
    f(&mut unsafe { CryoMut::new(x) })
}
