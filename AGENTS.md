# AGENTS.md - RequestX Development Guide

## Project Overview

RequestX is a high-performance Python HTTP client library, API-compatible with httpx, powered by Rust's reqwest via PyO3. The project follows a Rust-first architecture where **all business logic resides in Rust**, with Python providing only re-exports.

**Goal**: `import requestx as httpx` must work as a drop-in replacement.

---

## Build, Lint & Test Commands

### Development Build
```bash
# Always use release for accurate performance testing
maturin develop --release

# Development build (faster, for debugging)
maturin develop
```

### Running Tests
```bash
# Run target tests (must ALL pass)
pytest tests_requestx/ -v

# Run reference tests (DO NOT MODIFY, for API compatibility)
pytest tests_httpx/ -v

# Run both (verify compatibility)
pytest tests_httpx/ tests_requestx/ -v

# Run a single test
pytest tests_requestx/test_api.py::test_get -v
pytest tests_httpx/test_content.py::test_text -v

# Run by marker (network tests require connectivity)
pytest -m network -v
```

### Code Quality
```bash
# Rust linting and formatting
cargo clippy
cargo fmt

# Python linting and formatting
ruff check python/
ruff format python/

# Full quality check (all in sequence)
cargo clippy && cargo fmt && ruff check python/ && ruff format python/
```

---

## Code Style Guidelines

### Rust Patterns

#### PyO3 Usage
- **Use `Python::attach()`** instead of deprecated `with_gil()`
- **Prefer strong type signatures** over `PyAny` for compile-time checking
- Use `Bound<'_, T>` for Python object references

```rust
// GOOD
#[pyfunction]
fn fetch(py: Python, url: String) -> PyResult<String> {
    py.allow_threads(|| {
        // GIL released for network I/O
        blocking_fetch(&url)
    })
}

// AVOID
fn fetch(data: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>>
```

#### GIL Management
- **Release GIL for**: network I/O, file I/O, CPU work >1ms
- **Keep GIL for**: Python object access, operations <1ms

```rust
py.allow_threads(|| {
    // Network I/O here - GIL released
})
```

#### Async Pattern
```rust
use pyo3_async_runtimes::tokio::future_into_py;

#[pymethods]
impl AsyncClient {
    fn get<'py>(&self, py: Python<'py>, url: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let resp = client.get(&url).send().await?;
            Ok(Response::from_reqwest(resp).await?)
        })
    }
}
```

#### JSON Handling
- **Always use `sonic-rs`** (SIMD-accelerated, 50-300x faster than Python json)
- Never call Python's json module from Rust

```rust
let parsed: Value = sonic_rs::from_str(&json_str)?;
let output = sonic_rs::to_string(&value)?;
```

#### Memory Efficiency
- Return references, not clones
- Pre-allocate vectors when size is known
- Use `Vec::with_capacity()` when size is predictable

```rust
#[getter]
fn url(&self) -> &str { &self.url }

let mut headers = Vec::with_capacity(response.headers().len());
```

### Naming Conventions

| Category | Convention | Example |
|----------|------------|---------|
| Functions | snake_case | `execute_request`, `get_timeout` |
| Variables | snake_case | `follow_redirects`, `content_length` |
| Structs | PascalCase | `AsyncClient`, `URL` |
| Traits | PascalCase | `Auth` |
| Constants | SCREAMING_SNAKE_CASE | `DEFAULT_TIMEOUT` |
| Module imports | Lowercase | `use crate::client::Client;` |
| Python kwargs | snake_case | `follow_redirects`, `trust_env` |

### Error Handling

- Use `PyResult<T>` for all Python-facing functions
- Convert reqwest errors to Python exceptions using `convert_reqwest_error()`
- Follow httpx exception hierarchy (see `src/exceptions.rs`)
- **Never use `panic!`** - it crashes the Python interpreter

```rust
pub fn convert_reqwest_error(e: reqwest::Error) -> PyErr {
    // Check for specific error types and return appropriate Python exception
    if e.is_builder() {
        return UnsupportedProtocol::new_err(format!("{}", e));
    }
    // ... more conditions
    TransportError::new_err(format!("{}", e))
}
```

### Import Organization

```rust
// Standard library
use std::collections::HashMap;

// Crate imports
use pyo3::prelude::*;
use pyo3::types::PyDict;

// Local imports (use `crate::` for absolute paths)
use crate::client::Client;
use crate::response::Response;
```

### Documentation

- Use `//!` for module-level docs
- Use `///` for function/struct docs
- Document all `#[pyfunction]` and `#[pymethods]` with docstrings
- Include `#[pyo3(signature = (...))]` for Python signature

```rust
/// Perform a GET request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None))]
pub fn get(
    py: Python<'_>,
    url: &str,
    params: Option<&Bound<'_, PyAny>>,
    // ...
) -> PyResult<Response> {
    // ...
}
```

---

## Project Structure

```
requestx/
├── src/                      # Rust implementation (ALL business logic)
│   ├── lib.rs               # Module entry point, exports
│   ├── api.rs               # Top-level functions (get, post, etc.)
│   ├── async_client.rs      # AsyncClient implementation
│   ├── client.rs            # Sync Client implementation
│   ├── response.rs          # Response type
│   ├── request.rs           # Request type
│   ├── headers.rs           # Headers type
│   ├── queryparams.rs       # QueryParams type
│   ├── cookies.rs           # Cookies type
│   ├── url.rs               # URL type
│   ├── auth.rs              # Auth types
│   ├── timeout.rs           # Timeout, Limits types
│   ├── multipart.rs         # Multipart handling
│   ├── transport.rs         # Transport types
│   ├── types.rs             # Streams, status codes
│   └── exceptions.rs        # Exception hierarchy
│
├── python/requestx/
│   ├── __init__.py          # ONLY re-exports from Rust (NO business logic)
│   └── _utils.py            # Utility functions
│
├── tests_httpx/             # Reference tests (DO NOT MODIFY)
├── tests_requestx/          # Target tests (must pass)
├── Cargo.toml               # Rust dependencies
├── pyproject.toml           # Python project config
└── CLAUDE.md                # Claude Code guidance (READ THIS)
```

---

## Critical Rules

1. **Never modify `tests_httpx/`** - these are reference tests for API compatibility
2. **All business logic in Rust** - Python files should only re-export
3. **Never use `panic!`** - crashes Python
4. **Never suppress type errors** (`as any`, `@ts-ignore`, etc.)
5. **Convert types once at boundaries** - not inside loops
6. **Always run tests after changes**: `pytest tests_requestx/ -v`

---

## Dependencies Reference

Key Rust crates (check `Cargo.toml` for versions):
- `pyo3` 0.27 - Python bindings
- `reqwest` 0.13 - HTTP client
- `tokio` 1 - Async runtime
- `sonic-rs` 0.5 - SIMD JSON
- `url` 2 - URL parsing

---

## References

- httpx source: https://github.com/encode/httpx/tree/master/httpx
- PyO3 guide: https://pyo3.rs/
- reqwest docs: https://docs.rs/reqwest/
