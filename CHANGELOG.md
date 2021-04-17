# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/yvt/cryo/compare/0.1.6...HEAD
[0.1.6]: https://github.com/yvt/cryo/compare/0.1.5...0.1.6
[0.1.5]: https://github.com/yvt/cryo/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/yvt/cryo/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/yvt/cryo/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/yvt/cryo/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/yvt/cryo/compare/0.1.0...0.1.1
