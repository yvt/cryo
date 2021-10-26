//
// Copyright 2018 yvt, all rights reserved.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
#![warn(rust_2018_idioms)]

use cryo::*;

use std::{
    thread::{sleep, spawn},
    time::Duration,
};

#[test]
fn new() {
    with_cryo(&42, |_| {});
}

#[test]
fn borrow() {
    with_cryo(&42, |cryo| {
        assert_eq!(*cryo.borrow(), 42);
    });
}

#[test]
fn borrow2() {
    with_cryo(&42, |cryo| {
        let b1 = cryo.borrow();
        let _b2 = cryo.borrow();
        assert_eq!(*b1, 42);
    });
}

#[test]
fn unsize() {
    with_cryo("hello", |cryo| {
        assert_eq!(*cryo.borrow(), *"hello");
    });
}

#[test]
fn get() {
    with_cryo(&42, |cryo| {
        assert_eq!(*cryo.get(), 42);
    });
}

#[test]
fn block_on_drop() {
    with_cryo((&42, lock_ty::<SyncLock>()), |cryo| {
        let borrow = cryo.borrow();
        spawn(move || {
            sleep(Duration::from_millis(50));
            drop(borrow);
        });
    });
}
