# Changelog

All notable changes to RequestX will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial public release
- Synchronous HTTP client (`Client`)
- Asynchronous HTTP client (`AsyncClient`)
- Module-level convenience functions (`get`, `post`, `put`, `patch`, `delete`, `head`, `options`)
- Streaming response support
- HTTPX-compatible exception hierarchy
- HTTP/2 support
- Connection pooling
- Timeout configuration
- Proxy support
- Basic and Bearer authentication
- SSL/TLS configuration

## [0.1.0] - 2024-01-01

### Added
- Initial release
- Core HTTP client functionality
- Python 3.12+ support
- PyO3 bindings for Rust reqwest
- Basic documentation

---

## Version History

### Versioning Scheme

RequestX follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for incompatible API changes
- **MINOR** version for new functionality in a backward-compatible manner
- **PATCH** version for backward-compatible bug fixes

### Support Policy

- **Latest version**: Full support with bug fixes and new features
- **Previous minor version**: Security fixes only
- **Older versions**: No support

### Deprecation Policy

Features are deprecated in a minor release before removal in a major release:

1. Feature is marked as deprecated with a warning
2. Documentation is updated to indicate deprecation
3. Feature is removed in the next major version

### Reporting Issues

Found a bug or have a feature request? Please open an issue on [GitHub](https://github.com/neuesql/requestx/issues).
