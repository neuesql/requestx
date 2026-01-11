# PROJECT KNOWLEDGE BASE

**Generated:** 2026-01-10 20:03:14 UTC  
**Commit:** c3a3917  
**Branch:** feature/lean-client

## OVERVIEW
High-performance HTTP client for Python with requests-compatible API. Rust core (hyper + tokio) bridged to Python via PyO3.

## STRUCTURE
```
requestx/
├── src/                # Rust core (HTTP engine, PyO3 bindings)
├── src/core/          # HTTP client implementation  
├── python/requestx/    # Python wrapper + public API
├── tests/             # Python integration tests
├── docs/              # Sphinx documentation
└── config.toml        # Runtime configuration
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| **HTTP Client Logic** | `src/core/client.rs` | hyper-based implementation |
| **PyO3 Bindings** | `src/lib.rs` | Python ↔ Rust interface |
| **Session Management** | `src/session.rs` | Cookie persistence |
| **Python API** | `python/requestx/__init__.py` | User-facing interface |
| **Build System** | `Makefile` | Source of truth for CI |
| **Performance** | `python/requestx/benchmark.py` | Performance testing |
| **Configuration** | `config.toml` | Runtime settings |

## CONVENTIONS
- **Python 3.13+**: Early adoption of free-threaded builds (`python3.13t`)
- **unittest over pytest**: Explicit choice for built-in framework
- **Maturin bridge**: Rust core with Python bindings
- **Context-aware execution**: Automatic sync/async detection
- **Global client pattern**: `OnceLock` for connection pooling

## ANTI-PATTERNS (THIS PROJECT)
- **TLS per request**: Must use cached `NOVERIFY_CLIENT`
- **Data cloning**: Use `Body::from(text)` instead of clone
- **Parameter mixing**: Never `data=` + `json=` together
- **Exception registration**: Define exceptions in Python, not Rust
- **Leading match pipes**: Forbidden by `.rustfmt.toml`
- **Function calls in defaults**: Lint rule B008 violation

## UNIQUE STYLES
- **OIDC Trusted Publishing**: GitHub Actions OIDC to PyPI
- **Universal2 wheels**: Single artifact for Intel + Apple Silicon
- **Three CI workflows**: `ci.yml`, `publish.yml`, `release.yml`
- **Dynamic versioning**: Makefile extracts version from Cargo.toml
- **Performance profiler**: Built-in metrics collection

## COMMANDS
```bash
# Development
make dev-setup           # Install dependencies
make build               # Build extension
make test               # Full test suite

# CI Pipeline
make quality-check      # Stage 1: Linting
make verify-import      # Stage 2: Import tests  
make test-rust          # Stage 3: Rust tests
make test-python        # Stage 4: Python tests

# Release
make tag-release        # Tag and publish
make publish-release   # Publish to PyPI
```

## NOTES
- **Performance critical**: Zero-copy operations, pre-allocated strings
- **HTTP/2 support**: Native through hyper
- **Cookie management**: Uses `cookie_store` crate
- **Error mapping**: Rust errors → Python exceptions
- **Documentation**: Auto-generated via Sphinx
- **Current branch**: `feature/lean-client`正在进行重大重构