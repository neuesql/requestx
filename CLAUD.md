# Project: RequestX - High-Performance Python HTTP Client

## Objective
Build a high-performance Python HTTP client that is fully API-compatible with httpx, powered by Rust's reqwest library via PyO3 bindings.

## Architecture Requirements

### Core Principles
1. **Rust-First Implementation**: ALL business logic must be implemented in Rust
2. **Minimal Python Layer**: `python/requestx/__init__.py` should ONLY contain:
   - Type exports from Rust
   - Class exports from Rust
   - No Python business logic
3. **Performance Priority**: Optimize PyO3 bridge for minimal overhead

### Technology Stack
- **HTTP Engine**: Rust `reqwest` crate
- **Python Bindings**: PyO3 (use `Python::attach()` API, not deprecated `with_gil()`)
- **Async Runtime**: `pyo3-async-runtimes` with tokio feature
- **Target API**: httpx-compatible (excluding `httpx.__main__` and CLI features)

### Core Dependencies (Cargo.toml)
```toml
[package]
name = "requestx"
version = "1.0.8"
edition = "2021"

[lib]
name = "requestx"
crate-type = ["cdylib"]

[dependencies]
# PyO3 for Python bindings
pyo3 = { version = "0.27", features = ["extension-module"] }
pyo3-async-runtimes = { version = "0.27", features = ["tokio-runtime"] }

# Reqwest for HTTP
reqwest = { version = "0.13", features = [
    "blocking",
    "json",
    "query",
    "form",
    "cookies",
    "gzip",
    "brotli",
    "deflate",
    "zstd",
    "multipart",
    "stream",
    "rustls",
    "socks",
    "http2",
] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization (SIMD-accelerated JSON)
serde = { version = "1.0", features = ["derive"] }
sonic-rs = "0.5"

# URL handling
url = "2"
urlencoding = "2"

# Bytes
bytes = "1"

# HTTP types
http = "1"

# For multipart
mime = "0.3"
mime_guess = "2"

# Futures
futures = "0.3"

[profile.release]
lto = true
codegen-units = 1
opt-level = 3
```

## Reference Materials

### Source Code to Understand
1. **httpx source**: https://github.com/encode/httpx/tree/master/httpx
   - Study: Client, AsyncClient, Request, Response, URL, Headers, Cookies, Timeout, Limits
   - Ignore: `__main__.py`, CLI-related code

2. **Current project structure**:
   - `python/requestx/__init__.py` - Clean this file, export Rust types only
   - `src/` - Rust implementation (reqwest + PyO3)
   - `test_httpx/` - Reference tests (100% working, do not modify)
   - `test_requestx/` - Target tests (must all pass)

## Implementation Tasks

### Phase 1: Clean Python Layer
```python
# python/requestx/__init__.py - TARGET STATE
# Only exports, no logic

from .requestx import (
    # Classes
    Client,
    AsyncClient,
    Request,
    Response,
    # Types
    URL,
    Headers,
    Cookies,
    QueryParams,
    Timeout,
    Limits,
    # Exceptions
    HTTPError,
    RequestError,
    TimeoutException,
    # Functions
    get,
    post,
    put,
    patch,
    delete,
    head,
    options,
    request,
)

__all__ = [...]
__version__ = "..."
```

### Phase 2: Rust Implementation Checklist
Implement in Rust (`src/lib.rs` or modular structure):

- [ ] `Client` - Sync HTTP client
- [ ] `AsyncClient` - Async HTTP client
- [ ] `Request` - HTTP request object
- [ ] `Response` - HTTP response object
- [ ] `URL` - URL parsing and manipulation
- [ ] `Headers` - HTTP headers (dict-like interface)
- [ ] `Cookies` - Cookie jar
- [ ] `QueryParams` - Query string parameters
- [ ] `Timeout` - Timeout configuration
- [ ] `Limits` - Connection limits
- [ ] Top-level functions: `get()`, `post()`, `put()`, `patch()`, `delete()`, `head()`, `options()`, `request()`
- [ ] Exception hierarchy matching httpx

### Phase 3: PyO3 Performance Rules

> **Golden Rule**: The fastest Python code is code that doesn't call Python

#### Performance Hierarchy (Slow to Fast)
```
Python interpreted execution
    ↓ 10-100x faster
PyO3 calling Python code
    ↓ 5-10x faster
PyO3 + frequent Python ↔ Rust conversion
    ↓ 2-3x faster
PyO3 + one-time conversion + Rust processing
    ↓ 1.5-2x faster
Pure Rust + zero-copy optimization
```

#### Priority Rules (Must Follow)

| Priority | Rule | Impact |
|----------|------|--------|
| ⭐⭐⭐⭐⭐ | Use Rust native libraries (sonic-rs, not Python json) | 10-100x |
| ⭐⭐⭐⭐⭐ | Minimize Python ↔ Rust boundary crossings | 5-10x |
| ⭐⭐⭐⭐ | Convert data ONCE at function boundaries | 2-5x |
| ⭐⭐⭐⭐ | Release GIL for I/O and CPU-intensive operations | 2-10x |
| ⭐⭐⭐ | Pre-allocate containers with `Vec::with_capacity()` | 10-30% |
| ⭐⭐⭐ | Return references (`&str`) instead of clones (`String`) | 5-15% |
| ⭐⭐ | Use batch operations instead of individual ones | 5-10% |

---

## PyO3 Best Practices

### 1. Type Conversion Rules

**ALWAYS use strong type signatures:**
```rust
// ✅ Good: Compile-time type checking
#[pyfunction]
fn process(url: &str, data: Vec<i64>) -> PyResult<String> { ... }

// ❌ Bad: Runtime type checking overhead
#[pyfunction]
fn process(url: &Bound<'_, PyAny>, data: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> { ... }
```

**ALWAYS convert at boundaries, not in loops:**
```rust
// ✅ Good: Convert once at function boundary
#[pyfunction]
fn analyze_data(data: Vec<f64>) -> Vec<f64> {
    data.iter().map(|x| x * 2.0).filter(|x| *x > 0.0).collect()
}

// ❌ Bad: Convert every iteration
#[pyfunction]
fn analyze_data_bad(py: Python, data: &PyList) -> PyResult<Py<PyList>> {
    let result = PyList::empty_bound(py);
    for item in data.iter() {
        let val: f64 = item.extract()?;  // ❌ Convert every iteration
        result.append((val * 2.0).into_py(py))?;  // ❌ Convert back
    }
    Ok(result.unbind())
}
```

### 2. GIL Management Rules

**Release GIL for:**
- File I/O operations
- Network requests (reqwest calls)
- CPU-intensive computation (>1ms)
- Database queries

**Do NOT release GIL for:**
- Simple operations (<1ms)
- Operations requiring Python object access

```rust
// ✅ Correct pattern: Extract first, then release GIL
#[pyfunction]
fn process(py: Python, data: &PyList) -> PyResult<Vec<i64>> {
    // Step 1: Extract data while holding GIL
    let rust_data: Vec<i64> = data.extract()?;

    // Step 2: Release GIL for computation
    let result = py.allow_threads(|| {
        rust_data.iter().map(|x| x * 2).collect()
    });

    Ok(result)
}
```

**GIL Decision Tree:**
```
Should I release GIL?
├─ Operation < 1ms? → No (overhead > benefit)
├─ Need Python objects? → No (requires GIL)
├─ I/O operation? → Yes ✓
├─ CPU-intensive? → Yes ✓
└─ Parallel processing? → Yes ✓
```

### 3. Memory Management Rules

**Use zero-copy returns:**
```rust
// ✅ Good: Zero-copy with PyBytes
#[getter]
fn content(&self, py: Python) -> Bound<'_, PyBytes> {
    PyBytes::new_bound(py, &self.content)
}

// ❌ Bad: Unnecessary copy
#[getter]
fn content(&self) -> Vec<u8> {
    self.content.clone()
}
```

**Return references instead of clones:**
```rust
// ✅ Good: Return reference
#[getter]
fn url(&self) -> &str { &self.url }

// ❌ Bad: Clone every access
#[getter]
fn url(&self) -> String { self.url.clone() }
```

**Pre-allocate when capacity is known:**
```rust
// ✅ Good
let mut headers = Vec::with_capacity(response.headers().len());

// ❌ Bad: Multiple reallocations
let mut headers = Vec::new();
```

### 4. JSON Processing Rules

**ALWAYS use sonic-rs, NEVER Python json module:**

sonic-rs is a SIMD-accelerated JSON library, significantly faster than serde_json.

```rust
// ✅ Best: sonic-rs with SIMD acceleration (10-100x faster than Python)
let json_str = sonic_rs::to_string(&value)?;
let parsed: Value = sonic_rs::from_str(&json_str)?;

// ✅ Good: serde_json as fallback (10-50x faster than Python)
let json_str = serde_json::to_string(&value)?;

// ❌ Bad: Calls Python
let json_mod = PyModule::import(py, "json")?;
json_mod.getattr("dumps")?.call1((data,))?;
```

**Cargo.toml:**
```toml
[dependencies]
sonic-rs = "0.5"        # Primary: SIMD-accelerated JSON
serde = { version = "1.0", features = ["derive"] }
```

| JSON Size | Python json | serde_json | sonic-rs | Speedup (sonic-rs) |
|-----------|-------------|------------|----------|-------------------|
| < 1KB | 0.05ms | 0.005ms | 0.001ms | **50x** |
| 10KB | 0.5ms | 0.03ms | 0.005ms | **100x** |
| 100KB | 5ms | 0.1ms | 0.02ms | **250x** |
| 1MB | 50ms | 1ms | 0.15ms | **330x** |

### 5. Error Handling Rules

**Use `?` operator with proper error types:**
```rust
// ✅ Good: Clean and informative
#[pyfunction]
fn read_file(path: &str) -> PyResult<String> {
    std::fs::read_to_string(path)
        .map_err(|e| PyIOError::new_err(format!("Cannot read {}: {}", path, e)))
}

// ❌ Bad: Silent failure
fn bad(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

// ❌ Bad: Crashes Python
fn bad_panic(value: i64) -> i64 {
    if value < 0 { panic!("Negative!"); }
    value
}
```

### 6. Async Programming Rules

**Use `pyo3-async-runtimes` for Python asyncio integration:**

```rust
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

// ✅ Async HTTP request pattern with pyo3-async-runtimes
#[pyfunction]
fn async_fetch<'py>(py: Python<'py>, url: String) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let response = reqwest::get(&url).await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))?;
        let text = response.text().await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))?;
        Ok(text)
    })
}

// ✅ Async client method pattern
#[pymethods]
impl AsyncClient {
    fn get<'py>(&self, py: Python<'py>, url: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let response = client.get(&url).send().await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))?;
            // Convert to Response object
            Ok(Response::from_reqwest(response).await?)
        })
    }
}
```

| Scenario | Use | Reason |
|----------|-----|--------|
| I/O intensive | Async ✓ | High concurrency, low overhead |
| CPU intensive | Threading + GIL release | True parallelism |
| Mixed | Async + spawn_blocking | Flexible |
| Simple tasks | Sync | Avoid complexity |

### 7. Python Protocol Implementation

**Implement these for Python compatibility:**
- `__repr__` - Developer string representation
- `__str__` - User-friendly string
- `__eq__` - Equality comparison
- `__hash__` - For use in sets/dicts
- `__len__` - For sized objects
- `__iter__` / `__next__` - For iterables
- `__enter__` / `__exit__` - For context managers

### 8. Free-Threaded Python (PyO3 0.28+)

For Python 3.14+ without GIL:
```rust
// Use Python::attach() instead of with_gil()
#[pyfunction]
fn operation(path: &str) -> PyResult<String> {
    Python::attach(|py| {
        // Thread is now attached to Python runtime
        std::fs::read_to_string(path)
            .map_err(|e| PyIOError::new_err(format!("{}", e)))
    })
}

// Use Mutex for thread-safe shared state (replaces GILProtected)
static COUNTER: Mutex<usize> = Mutex::new(0);
```

---

## Type Conversion Quick Reference

| Rust Type | Python Type | Notes |
|-----------|-------------|-------|
| `i64`, `u64` | `int` | Integer |
| `f64` | `float` | Float |
| `bool` | `bool` | Boolean |
| `String`, `&str` | `str` | String |
| `Vec<T>` | `list` | List |
| `HashMap<K, V>` | `dict` | Dictionary |
| `Option<T>` | `T` or `None` | Optional |
| `PyResult<T>` | `T` or raises | May fail |
| `Vec<u8>`, `&[u8]` | `bytes` | Binary data |

---

## Anti-Patterns to Avoid

1. **Overusing `PyAny`** - Loses type safety, high runtime overhead
2. **Converting in loops** - Extract once, process in Rust
3. **Calling Python libraries from Rust** - Use Rust equivalents
4. **Swallowing errors** - Always return `PyResult`
5. **Using `panic!`** - Crashes Python process
6. **Nested `with_gil`** - May cause deadlock
7. **Cloning when references work** - Wasteful memory usage
8. **Forgetting to release GIL** - Blocks other Python threads

## Testing Strategy

### Test Execution Order
1. First, verify reference tests work:
```bash
   pytest test_httpx/ -v  # Must be 100% passing
```

2. Then run target tests iteratively:
```bash
   pytest test_requestx/ -v --tb=short
```

3. Compare behavior:
```bash
   # Run both to ensure compatibility
   pytest test_httpx/ test_requestx/ -v
```

### Success Criteria
- [ ] ALL tests in `test_requestx/` pass
- [ ] API is drop-in compatible with httpx (import requestx as httpx should work)
- [ ] No Python business logic in `__init__.py`
- [ ] Performance equal or better than httpx

## Constraints
- Do NOT implement: `httpx.__main__`, CLI features, `httpx.main()`
- Do NOT modify: `test_httpx/` folder
- MUST use: Rust reqwest for all HTTP operations
- MUST use: PyO3 `Python::attach()` (not deprecated `with_gil()`)

## Completion Definition
Task is DONE when:
```bash
pytest test_requestx/ -v
# Result: ALL PASSED
```

## Reference Implementation
- https://github.com/MarkusSintonen/pyreqwest
