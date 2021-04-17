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
    cryo!(let _unused: Cryo<_> = &42);
}

#[test]
fn borrow() {
    cryo!(let cryo: Cryo<_> = &42);
    assert_eq!(*cryo.borrow(), 42);
}

#[test]
fn borrow2() {
    cryo!(let cryo: Cryo<_> = &42);
    let b1 = cryo.borrow();
    let _b2 = cryo.borrow();
    assert_eq!(*b1, 42);
}

#[test]
fn get() {
    cryo!(let cryo: Cryo<_> = &42);
    assert_eq!(*cryo.get(), 42);
}

#[test]
fn block_on_drop() {
    cryo!(let cryo: Cryo<_> = &42);
    let borrow = cryo.borrow();
    spawn(move || {
        sleep(Duration::from_millis(50));
        drop(borrow);
    });
}
