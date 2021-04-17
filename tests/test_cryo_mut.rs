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
use pin_utils::pin_mut;

use std::{
    thread::{sleep, spawn},
    time::Duration,
};

#[test]
fn new() {
    let mut var = 42;
    cryo!(let _cryo: CryoMut<_> = &mut var);
}

#[test]
fn read() {
    let mut var = 42;
    cryo!(let cryo_mut: CryoMut<_> = &mut var);
    assert_eq!(*cryo_mut.read(), 42);
}

#[test]
fn read2() {
    let mut var = 42;
    cryo!(let cryo_mut: CryoMut<_> = &mut var);
    let b1 = cryo_mut.read();
    let _b2 = cryo_mut.read();
    assert_eq!(*b1, 42);
}

#[test]
fn write() {
    let mut var = 42;
    cryo!(let cryo_mut: CryoMut<_> = &mut var);
    assert_eq!(*cryo_mut.write(), 42);
}

#[test]
fn try_get_mut() {
    let mut cell = 42;
    let cryo_mut = CryoMut::<_, SyncLock>::new(&mut cell);
    pin_mut!(cryo_mut);
    assert_eq!(cryo_mut.try_get_mut(), Some(&mut 42));
}

#[test]
fn try_get_mut_fail() {
    let mut cell = 42;
    let cryo_mut = CryoMut::<_, SyncLock>::new(&mut cell);
    pin_mut!(cryo_mut);
    let _b = cryo_mut.as_ref().read();
    assert_eq!(cryo_mut.try_get_mut(), None);
}

#[test]
fn block_on_drop() {
    let mut var = 42;
    cryo!(let cryo_mut: CryoMut<_> = &mut var);
    let borrow = cryo_mut.read();
    spawn(move || {
        sleep(Duration::from_millis(50));
        drop(borrow);
    });
}

#[test]
fn block_by_exclusive_access() {
    let mut var = 42;
    cryo!(let cryo_mut: CryoMut<_> = &mut var);
    let borrow = cryo_mut.read();
    spawn(move || {
        sleep(Duration::from_millis(100));
        assert_eq!(*borrow, 42);
        drop(borrow);
    });
    assert_eq!(std::mem::replace(&mut *cryo_mut.write(), 56), 42);

    let mut borrow = cryo_mut.write();
    spawn(move || {
        sleep(Duration::from_millis(100));
        assert_eq!(std::mem::replace(&mut *borrow, 72), 56);
        drop(borrow);
    });
    assert_eq!(std::mem::replace(&mut *cryo_mut.write(), 100), 72);
}
