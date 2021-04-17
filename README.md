# cryo

[<img src="https://docs.rs/cryo/badge.svg" alt="docs.rs">](https://docs.rs/cryo/)

*Extend the lifetime of a reference. Safely.*

Requires Rust 1.34.0 or later.

This crate provides a cell-like type [`Cryo`] that is similar to `RefCell`
except that it constrains the lifetime of its borrowed value
through a runtime check mechanism, erasing the compile-time lifetime
information. The lock guard [`CryoRef`] created from `Cryo` is
`'static` and therefore can be used in various situations that require
`'static` types, including:

 - Storing [`CryoRef`] temporarily in a `std::any::Any`-compatible container.
 - Capturing a reference to create a [Objective-C block](https://crates.io/crates/block).

This works by, when a `Cryo` is dropped, blocking the current thread until
all references to the contained value are dropped so that none of them can
outlive the cell.

## Examples

[`cryo!`] and [`Cryo`]:

```rust
use std::{thread::spawn, pin::Pin};

let cell: usize = 42;

{
    cryo!(let cryo: Cryo<usize> = &cell);

    // Borrow `cryo` and move it into a `'static` closure.
    let borrow: CryoRef<usize, _> = cryo.borrow();
    spawn(move || { assert_eq!(*borrow, 42); });

    // Compile-time lifetime works as well.
    assert_eq!(*cryo.get(), 42);

    // When `cryo` is dropped, it will block until there are no other
    // references to `cryo`. In this case, the program will not leave
    // this block until the thread we just spawned completes execution.
}
```

[`cryo!`] and [`CryoMut`]:

```rust
{
    cryo!(let cryo_mut: CryoMut<usize> = &mut cell);

    // Borrow `cryo_mut` and move it into a `'static` closure.
    let mut borrow: CryoMutWriteGuard<usize, _> = cryo_mut.write();
    spawn(move || { *borrow = 1; });

    // When `cryo_mut` is dropped, it will block until there are no other
    // references to `cryo_mut`. In this case, the program will not leave
    // this block until the thread we just spawned completes execution
}
assert_eq!(cell, 1);
```

**Don't** do this:

```rust
// The following statement will deadlock because it attempts to drop
// `Cryo` while a `CryoRef` is still referencing it
let borrow = {
    cryo!(let cryo: Cryo<_> = &cell);
    cryo.borrow()
};
```

## Caveats

- While it's capable of extending the effective lifetime of a reference,
  it does not apply to nested references. For example, when
  `&'a NonStaticType<'b>` is supplied to the `Cryo`'s constructor, the
  borrowed type is `CryoRef<NonStaticType<'b>>`, which is still partially
  bound to the original lifetime.

## Details

### Feature flags

 - `std` (enabled by default) enables [`SyncLock`].

 - `lock_api` enables the blanket implementation of [`RawRwLock`] on
   all types implementing [`lock_api::RawRwLock`], such as
   [`parking_lot::RawRwLock`].

[`parking_lot::RawRwLock`]: https://docs.rs/parking_lot/0.11.1/parking_lot/struct.RawRwLock.html

### Overhead

`Cryo<T, SyncLock>`'s creation, destruction, borrowing, and unborrowing
each take one or two atomic operations in the best cases.

### Nomenclature

From [cryopreservation](https://en.wikipedia.org/wiki/Cryopreservation).


License: MIT/Apache-2.0
