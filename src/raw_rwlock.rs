//
// Copyright 2018 yvt, all rights reserved.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//

#[cfg(feature = "parking_lot")]
use parking_lot::{Condvar, Mutex};

#[cfg(not(feature = "parking_lot"))]
use std::sync::{Condvar, Mutex};

/// Provides a functionality similar to `RwLock<()>`, its logical lock guard is
/// `Send` (which applies to neither of `std::sync::RwLock` and
/// `parking_lot::RwLock`).
pub struct RawRwLock(Mutex<usize>, Condvar);

const COUNT_EXCLUSIVE: usize = !0;

#[cfg(feature = "parking_lot")]
macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock()
    };
}

#[cfg(feature = "parking_lot")]
macro_rules! wait {
    ($condvar:expr, $guard:expr) => {
        $condvar.wait(&mut $guard)
    };
}

#[cfg(not(feature = "parking_lot"))]
macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock().unwrap()
    };
}

#[cfg(not(feature = "parking_lot"))]
macro_rules! wait {
    ($condvar:expr, $guard:expr) => {
        $guard = $condvar.wait($guard).unwrap()
    };
}

impl RawRwLock {
    pub fn new() -> Self {
        RawRwLock(Mutex::new(0), Condvar::new())
    }

    pub fn raw_read(&self) {
        let mut count = lock!(self.0);
        while *count == COUNT_EXCLUSIVE {
            wait!(self.1, count);
        }
        *count += 1;
    }

    pub fn raw_try_read(&self) -> bool {
        let mut count = lock!(self.0);
        if *count == COUNT_EXCLUSIVE {
            false
        } else {
            *count += 1;
            true
        }
    }

    pub fn raw_write(&self) {
        let mut count = lock!(self.0);
        while *count != 0 {
            wait!(self.1, count);
        }
        *count = COUNT_EXCLUSIVE;
    }

    pub fn raw_try_write(&self) -> bool {
        let mut count = lock!(self.0);
        if *count != 0 {
            false
        } else {
            *count = COUNT_EXCLUSIVE;
            true
        }
    }

    pub unsafe fn raw_unlock_read(&self) {
        let mut count = lock!(self.0);

        debug_assert_ne!(*count, 0);
        *count -= 1;

        self.1.notify_all();
    }

    pub unsafe fn raw_unlock_write(&self) {
        let mut count = lock!(self.0);

        debug_assert_eq!(*count, COUNT_EXCLUSIVE);
        *count = 0;

        self.1.notify_all();
    }
}
