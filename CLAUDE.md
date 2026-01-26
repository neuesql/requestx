# CLAUDE.md - AI Assistant Guide for RequestX

## Project Overview

**RequestX** is a high-performance HTTP client library for Python that provides a drop-in replacement for the `requests` library. It combines Rust's performance and memory safety with Python's ease of use through PyO3 bindings.

- **Current Version**: Python 0.4.3, Rust Core 0.3.0
- **License**: MIT
- **Repository**: https://github.com/neuesql/requestx
- **Documentation**: https://requestx.readthedocs.io

### Key Value Propositions
- Drop-in replacement for `requests` with identical API
- High performance via Rust backend (hyper + tokio)
- Dual sync/async API with automatic context detection
- Cross-platform support (Windows, macOS, Linux)
- Zero runtime Python dependencies

---

## Codebase Structure

```
requestx/
├── src/                          # Rust core implementation
│   ├── lib.rs                    # PyO3 module entry point, Python bindings
│   ├── config.rs                 # Runtime configuration (config.toml parsing)
│   ├── error.rs                  # Error types and Python exception mapping
│   ├── response.rs               # Response object implementation
│   ├── session.rs                # Session class for persistent connections
│   └── core/
│       ├── client.rs             # HTTP client (hyper-based, connection pooling)
│       ├── runtime.rs            # Tokio runtime & sync/async context detection
│       └── mod.rs                # Core module exports
│
├── python/
│   └── requestx/
│       └── __init__.py           # Python wrapper, exception hierarchy, public API
│
├── tests/
│   ├── test_quickstart.py        # Integration tests (unittest-based)
│   └── performance/              # Benchmark payloads (50B - 2MB)
│
├── docs/                         # Sphinx documentation (Furo theme)
│
├── config.toml                   # Runtime settings (HTTP/2, pooling, threads)
├── Cargo.toml                    # Rust package manifest
├── pyproject.toml                # Python project config & tools
├── Makefile                      # Build system (numbered commands)
└── .rustfmt.toml                 # Rust formatter configuration
```

### Key File Locations

| Task | Location | Notes |
|------|----------|-------|
| HTTP Client Logic | `src/core/client.rs` | hyper-based, connection pooling |
| PyO3 Bindings | `src/lib.rs` | Python ↔ Rust interface |
| Session Management | `src/session.rs` | Cookie persistence |
| Python API | `python/requestx/__init__.py` | User-facing interface, exceptions |
| Error Handling | `src/error.rs` | Rust → Python exception mapping |
| Runtime Management | `src/core/runtime.rs` | Tokio, context detection |
| Build System | `Makefile` | Source of truth for all commands |
| Configuration | `config.toml` | HTTP/2, pooling, worker threads |

---

## Development Commands

The Makefile uses numbered commands for clear sequencing:

```bash
# Setup & Dependencies
make 1-setup              # Install uv, sync dependencies

# Code Quality
make 2-format             # Format Rust (cargo fmt) + Python (black)
make 2-format-check       # Check formatting without changes
make 3-lint               # Run clippy + ruff
make 4-quality-check      # Combined format check + lint (CI Stage 1)

# Building
make 5-build              # Build extension with maturin develop

# Testing
make 6-test-rust          # cargo test --verbose + doc tests
make 6-test-python        # unittest discover (requires build)
make 6-test-all           # Run all tests
make 6-test-coverage      # Tests with coverage report

# Documentation
make 7-doc-build          # Build Sphinx docs
make 7-doc-serve          # Serve docs at localhost:8000

# Release
make 8-release-github     # Create GitHub release (requires GIT_TOKEN)
make 8-release-pypi       # Publish to PyPI (requires PYPI_TOKEN)

# Cleanup
make 9-clean              # Remove all build artifacts

# Version Management
make version-patch        # Bump patch version (0.0.x)
make version-minor        # Bump minor version (0.x.0)
make version-major        # Bump major version (x.0.0)

# Benchmarks
make benchmark-get-sync-test   # Sync benchmark vs requests, httpx, etc.
make benchmark-get-async-test  # Async benchmark vs httpx, aiohttp
```

### Quick Development Workflow

```bash
make 1-setup              # First time setup
make 5-build              # Build the extension
make 6-test-python        # Run tests
make 4-quality-check      # Before committing
```

---

## Technology Stack

### Rust Dependencies
- **hyper 0.14** - HTTP client with HTTP/1.1 and HTTP/2 support
- **tokio 1.47.1** - Async runtime
- **PyO3 0.25.0** - Python bindings with ABI3 support
- **sonic-rs 0.5** - High-performance JSON
- **cookie_store 0.21** - Cookie persistence
- **hyper-tls 0.5** - TLS support

### Python Tools
- **maturin** - Build system for PyO3 extensions
- **uv** - Package manager and resolver
- **black** - Formatter (200 char line length)
- **ruff** - Linter (E/W/F/I/B/C4/UP checks)
- **mypy** - Type checker (strict mode)
- **unittest** - Test framework (NOT pytest)

---

## Conventions

### Python Conventions
- **Line length**: 200 characters (both black and ruff)
- **Target version**: Python 3.8+ compatibility, requires 3.12+
- **Testing**: Use built-in `unittest`, NOT pytest
- **Type hints**: Required (mypy strict mode)
- **Exception hierarchy**: Must match `requests` library structure

### Rust Conventions
- **Line length**: 100 characters (rustfmt)
- **Edition**: 2021
- **Imports**: Crate-level granularity with StdExternalCrate grouping
- **Match arms**: No leading pipes (`match_arm_leading_pipes = "Never"`)
- **Error handling**: Use `Result<T, RequestxError>` internally, convert to `PyResult` at boundaries

### Code Style Enforced
```toml
# .rustfmt.toml highlights
max_width = 100
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
match_arm_leading_pipes = "Never"

# pyproject.toml highlights
[tool.black]
line-length = 200

[tool.ruff]
line-length = 200
select = ["E", "W", "F", "I", "B", "C4", "UP"]

[tool.mypy]
disallow_untyped_defs = true
strict_equality = true
```

---

## Anti-Patterns to Avoid

### Rust Anti-Patterns
1. **Creating TLS connectors per request** - Use cached `GLOBAL_CLIENT` or `NOVERIFY_CLIENT`
2. **Data cloning** - Use `Body::from(owned_data)` to move data, not clone
3. **Mixing body parameters** - Never allow `data=` + `json=` together
4. **Blocking async runtime** - Use `tokio::time::sleep`, not `std::thread::sleep`
5. **Unguarded unwraps** - All failures must be `RequestxError`, no `.unwrap()` in request path
6. **Manual redirect implementation** - Use the optimized loop in `execute_request_async`
7. **GIL blocking** - Always use `py.allow_threads()` with `runtime.block_on`

### Python Anti-Patterns
1. **Defining exceptions in Rust** - Define in Python (`__init__.py`), not Rust
2. **Function calls in defaults** - Avoid (lint rule B008)
3. **Using pytest** - Project uses unittest intentionally
4. **Leading match pipes** - Forbidden by `.rustfmt.toml`

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│           Python Application (User API)                     │
│         python/requestx/__init__.py                        │
│  (get, post, put, delete, Session, Response, Exceptions)   │
└──────────────────────┬──────────────────────────────────────┘
                       │ PyO3 Bindings
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              Rust Core (src/lib.rs)                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ HTTP Functions & Request Config Builder              │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Core Module (src/core/)                             │   │
│  │  • Client: hyper HTTP with connection pooling       │   │
│  │  • Runtime: Tokio management, context detection     │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Supporting: Response, Session, Error, Config        │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                       │ Hyper + Tokio
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              Network Layer (HTTP/1.1, HTTP/2, TLS)          │
└─────────────────────────────────────────────────────────────┘
```

### Key Design Patterns
1. **Singleton Pattern** - `OnceLock` for global HTTP client and runtime
2. **Builder Pattern** - `RequestConfigBuilder` for request parameters
3. **Exception Mapping** - Rust errors → PyO3 → Python exception hierarchy
4. **Context-Aware Execution** - Auto-detects sync/async context

---

## Testing

### Test Framework
- Uses **unittest** (explicit choice over pytest)
- Test discovery: `python -m unittest discover tests/ -v`
- Integration tests use `testcontainers` with httpbin Docker image

### Running Tests
```bash
# Build first (required for Python tests)
make 5-build

# Run tests
make 6-test-rust          # Rust unit tests
make 6-test-python        # Python integration tests
make 6-test-all           # Both
make 6-test-coverage      # With coverage report
```

### Test Structure
```
tests/
├── test_quickstart.py    # Main integration tests
│   ├── TestBasicRequests
│   ├── TestResponseStatusCodes
│   ├── TestCookies
│   ├── TestAuthentication
│   ├── TestTimeouts
│   ├── TestSessionManagement
│   ├── TestErrorHandling
│   └── ...
└── performance/          # Benchmark payloads
```

---

## Configuration

### Runtime Configuration (config.toml)
```toml
[client]
pool_idle_timeout_secs = 300
pool_max_idle_per_host = 1024
# HTTP/2 tuning options

[runtime]
worker_threads = 8
max_blocking_threads = 512
thread_stack_size = 1048576  # 1MB
```

### Release Configuration
- **Rust optimizations**: `opt-level = 3`, `lto = true`, `codegen-units = 1`
- **PyO3 ABI3**: Compatible with Python 3.8+
- **Universal2 wheels**: Single macOS artifact for Intel + Apple Silicon
- **OIDC Trusted Publishing**: Secure PyPI releases via GitHub Actions

---

## CI/CD Pipeline

1. **Quality Check** - Format check + lint
2. **Build** - Multi-platform wheel building
3. **Test** - Rust tests + Python integration tests
4. **Release** - GitHub release + PyPI publish

### Supported Platforms
- Linux (x86_64, aarch64) - glibc + musl
- Windows (x64, x86)
- macOS (Intel x86_64, Apple Silicon aarch64)
- Free-threaded Python 3.14t wheels

---

## Quick Reference

### Adding a New HTTP Method
1. Add Rust function in `src/lib.rs`
2. Export via `#[pyfunction]` and add to module
3. Wrap in Python `__init__.py` with `_wrap_request_function`

### Adding a New Exception
1. Define exception class in `python/requestx/__init__.py`
2. Map error in `src/error.rs` `impl From<RequestxError> for PyErr`
3. Add to `_map_exception()` in Python wrapper

### Modifying HTTP Client Behavior
1. Edit `src/core/client.rs` for request execution
2. Update `src/config.rs` for new configuration options
3. Adjust `config.toml` defaults

---

## Important Notes for AI Assistants

1. **Always run quality checks before committing**: `make 4-quality-check`
2. **Build before testing Python**: `make 5-build && make 6-test-python`
3. **Use unittest, not pytest**: This is an explicit project choice
4. **Follow the 200-char line limit for Python**: Configured in black and ruff
5. **Follow the 100-char line limit for Rust**: Configured in rustfmt
6. **Exception hierarchy must match requests library**: For drop-in compatibility
7. **Never create HTTP clients per-request**: Use the singleton pattern
8. **Test with httpbin container**: Integration tests require Docker
