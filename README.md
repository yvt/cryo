<h1 align="center">
<img src="doc/banner.svg" alt="Cryo — Extend the lifetime of a reference. Safely.">
</h1>

<p align="center">
<a href="https://docs.rs/cryo/"><img src="https://docs.rs/cryo/badge.svg" alt="docs.rs"></a> <a href="https://crates.io/crates/cryo"><img src="https://img.shields.io/crates/v/cryo"></a> <img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue">
</p>

Requires Rust 1.34.0 or later.

This crate provides a cell-like type `Cryo` that is similar to `RefCell`
except that it constrains the lifetime of its borrowed value
through a runtime check mechanism, erasing the compile-time lifetime
information. The lock guard `CryoRef` created from `Cryo` is
`'static` and therefore can be used in various situations that require
`'static` types, including:

 - Storing `CryoRef` temporarily in a `std::any::Any`-compatible container.
 - Capturing a reference to create a [Objective-C block](https://crates.io/crates/block).

This works by, when a `Cryo` is dropped, not letting the current thread's
execution move forward (at least¹) until all references to the expiring
`Cryo` are dropped so that none of them can outlive the `Cryo`.
This is implemented by [readers-writer locks] under the hood.

[readers-writer locks]: https://en.wikipedia.org/wiki/Readers–writer_lock

<sub>¹ `SyncLock` blocks the current thread's execution on lock failure.
`LocalLock`, on the other hand, panics because it's designed for
single-thread use cases and would deadlock otherwise.</sub>

## Examples

`cryo!`, `Cryo`, and `LocalLock` (single-thread lock
implementation, used by default):

```rust
use std::{thread::spawn, pin::Pin};

let cell: usize = 42;

{
    // `cryo!` uses `LocalLock` by default
    cryo!(let cryo: Cryo<usize> = &cell);

    // Borrow `cryo` and move it into a `'static` closure.
    let borrow: CryoRef<usize, _> = cryo.borrow();
    let closure: Box<dyn Fn()> =
        Box::new(move || { assert_eq!(*borrow, 42); });
    closure();

    // Compile-time lifetime works as well.
    assert_eq!(*cryo.get(), 42);

    // When `cryo` is dropped, it will block until there are no other
    // references to `cryo`. In this case, the program will not leave
    // this block until the thread we just spawned completes execution.
}
```

`cryo!`, `Cryo`, and `SyncLock` (thread-safe lock implementation):

```rust
use std::{thread::spawn, pin::Pin};

let cell: usize = 42;

{
    // This this we are specifying the lock implementation
    cryo!(let cryo: Cryo<usize, SyncLock> = &cell);

    // Borrow `cryo` and move it into a `'static` closure.
    // `CryoRef` can be sent to another thread because
    // `SyncLock` is thread-safe.
    let borrow: CryoRef<usize, _> = cryo.borrow();
    spawn(move || { assert_eq!(*borrow, 42); });

    // Compile-time lifetime works as well.
    assert_eq!(*cryo.get(), 42);

    // When `cryo` is dropped, it will block until there are no other
    // references to `cryo`. In this case, the program will not leave
    // this block until the thread we just spawned completes execution.
}
```

`cryo!`, `CryoMut`, and `SyncLock`:

```rust
{
    cryo!(let cryo_mut: CryoMut<usize, SyncLock> = &mut cell);

    // Borrow `cryo_mut` and move it into a `'static` closure.
    let mut borrow: CryoMutWriteGuard<usize, _> = cryo_mut.write();
    spawn(move || { *borrow = 1; });

    // When `cryo_mut` is dropped, it will block until there are no other
    // references to `cryo_mut`. In this case, the program will not leave
    // this block until the thread we just spawned completes execution
}
assert_eq!(cell, 1);
```

**Don't** do these:

```rust
// The following statement will DEADLOCK because it attempts to drop
// `Cryo` while a `CryoRef` is still referencing it, and `Cryo`'s
// destructor will wait for the `CryoRef` to be dropped first (which
// will never happen)
let borrow = {
    cryo!(let cryo: Cryo<_, SyncLock> = &cell);
    cryo.borrow()
};
```

```rust
// The following statement will ABORT because it attempts to drop
// `Cryo` while a `CryoRef` is still referencing it, and `Cryo`'s
// destructor will panic, knowing no amount of waiting would cause
// the `CryoRef` to be dropped
let borrow = {
    cryo!(let cryo: Cryo<_> = &cell);
    cryo.borrow()
};
```

## Caveats

- While it's capable of extending the effective lifetime of a reference,
  it does not apply to nested references. For example, when
  `&'a NonStaticType<'b>` is supplied to `Cryo`'s constructor, the
  borrowed type is `CryoRef<NonStaticType<'b>>`, which is still partially
  bound to the original lifetime.

## Details

### Feature flags

 - `std` (enabled by default) enables `SyncLock`.

 - `lock_api` enables the blanket implementation of `Lock` on
   all types implementing `lock_api::RawRwLock`, such as
   `spin::RawRwLock` and `parking_lot::RawRwLock`.

`spin::RawRwLock`: https://docs.rs/spin/0.9.0/spin/type.RwLock.html
`parking_lot::RawRwLock`: https://docs.rs/parking_lot/0.11.1/parking_lot/struct.RawRwLock.html

### Overhead

`Cryo<T, SyncLock>`'s creation, destruction, borrowing, and unborrowing
each take one or two atomic operations in the best cases.

Neither of `SyncLock` and `LocalLock` require dynamic memory allocation.

### Nomenclature

From [cryopreservation](https://en.wikipedia.org/wiki/Cryopreservation).


License: MIT/Apache-2.0
