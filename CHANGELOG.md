# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Breaking (**soundness fix**):
    - `Cryo<T, _>: Send` requires `T: Sync` instead of `T: Send`.
    - `CryoMut<T, _>: Send` now requires `T: Sync` in addition to `T: Send`.
    - `CryoMutReadGuard<T, _>: Send` requires `T: Sync` instead of `T: Send`.
- `Cryo<T, _>: Sync` no longer requires `T: Send`.
- `CryoMutReadGuard<T, _>: Sync` no longer requires `T: Send`.
- `CryoMutWriteGuard<T, _>: Sync` if `T: Sync`.
- Breaking (**soundness fix**): Remove `cryo!`

## [0.2.7] - 2021-10-25

- Deprecate 0.2.x because of numerous soundness bugs that require breaking fixes

## [0.2.6] - 2021-09-11

- Update `README.md`

## [0.2.4] - 2021-09-11

- Bring `with_cryo` back with an overloaded interface (one function can produce both `Cryo` and `CryoMut` depending on a given type).
- Deprecate (**soundness fix**): `cryo!`

## [0.2.3] - 2021-09-02

- Breaking (**soundness fix**): `{Cryo, CryoMut}::new` is now `unsafe fn`.

## [0.2.2] - 2021-05-08

- `cryo` no longer enables the default features of `stable_deref_trait`, fixing builds on `core`-only targets.

## [0.2.1] - 2021-04-19

- Added `AtomicLock`

## [0.2.0] - 2021-04-18

- Breaking: `Cryo` is now generalized over lock implementations. Two implementations are provided: `LocalLock` (single-threaded) and `SyncLock` (borrows can be sent to other threads). You need to specify in `cryo!` to use the latter.
- Breaking: `parking-lot` feature was removed.
- Breaking: `Cryo` now utilizes `std::pin::Pin` (instead of making the constructor `unsafe fn`) for pinning. Most methods now take `self: Pin<&Cryo<_, _>>` as the receiver.
- Breaking: `with_cryo[_mut]` was superseded by `cryo!`.
- Breaking: `std` feature was added.

## [0.1.6] - 20xx-xx-xx
## [0.1.5] - 20xx-xx-xx
## [0.1.4] - 20xx-xx-xx
## [0.1.3] - 20xx-xx-xx
## [0.1.2] - 20xx-xx-xx
## [0.1.1] - 20xx-xx-xx
## 0.1.0 - 20xx-xx-xx

- Initial release.

[Unreleased]: https://github.com/yvt/cryo/compare/0.2.7...HEAD
[0.2.7]: https://github.com/yvt/cryo/compare/0.2.6...0.2.7
[0.2.6]: https://github.com/yvt/cryo/compare/0.2.4...0.2.6
[0.2.4]: https://github.com/yvt/cryo/compare/0.2.3...0.2.4
[0.2.3]: https://github.com/yvt/cryo/compare/0.2.2...0.2.3
[0.2.2]: https://github.com/yvt/cryo/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/yvt/cryo/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/yvt/cryo/compare/0.1.6...0.2.0
[0.1.6]: https://github.com/yvt/cryo/compare/0.1.5...0.1.6
[0.1.5]: https://github.com/yvt/cryo/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/yvt/cryo/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/yvt/cryo/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/yvt/cryo/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/yvt/cryo/compare/0.1.0...0.1.1
