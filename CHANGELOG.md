# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Breaking: `Cryo` is now generalized over lock implementations. The default implementation was rewritten from scratch. The new implementation constrains the access of `Cryo` to a single thread but provides an improved performance.
- Breaking: `parking-lot` feature was removed.

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
