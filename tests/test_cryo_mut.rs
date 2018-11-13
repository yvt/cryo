//
// Copyright 2018 yvt, all rights reserved.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
extern crate cryo;
use cryo::*;

use std::{
    thread::{sleep, spawn},
    time::Duration,
};

#[test]
fn new() {
    with_cryo_mut(&mut 42, |_| {});
}

#[test]
fn read() {
    with_cryo_mut(&mut 42, |cryo_mut| {
        assert_eq!(*cryo_mut.read(), 42);
    });
}

#[test]
fn read2() {
    with_cryo_mut(&mut 42, |cryo_mut| {
        let b1 = cryo_mut.read();
        let _b2 = cryo_mut.read();
        assert_eq!(*b1, 42);
    });
}

#[test]
fn write() {
    with_cryo_mut(&mut 42, |cryo_mut| {
        assert_eq!(*cryo_mut.write(), 42);
    });
}

#[test]
fn try_get_mut() {
    with_cryo_mut(&mut 42, |cryo_mut| {
        assert_eq!(cryo_mut.try_get_mut(), Some(&mut 42));
    });
}

#[test]
fn try_get_mut_fail() {
    with_cryo_mut(&mut 42, |cryo_mut| {
        let _b = cryo_mut.read();
        assert_eq!(cryo_mut.try_get_mut(), None);
    });
}

#[test]
fn block_on_drop() {
    with_cryo_mut(&mut 42, |cryo_mut| {
        let borrow = cryo_mut.read();
        spawn(move || {
            sleep(Duration::from_millis(50));
            drop(borrow);
        });
    });
}
