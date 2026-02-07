# PyO3 0.28 Performance Best Practices for Python Libraries in Rust

> Updated for **PyO3 v0.28.0** (released February 2026), covering the latest API changes including `Python::detach`, `cast` vs `extract`, `vectorcall` protocol, free-threaded Python support, and the `pyo3_disable_reference_pool` compilation flag.

---

## 1. Detach from the Interpreter for Long-Running Rust Work (Highest Impact)

In PyO3 0.28, `Python::allow_threads` has been renamed to **`Python::detach`**. This is the single most important optimization — it allows the Python interpreter to proceed without waiting for the current thread.

On **GIL-enabled builds**, this is crucial as only one thread may be attached at a time. On **free-threaded builds** (Python 3.13t/3.14t), this is still essential because "stop the world" events (like garbage collection) force all attached threads to wait.

**Rule of thumb:** Attaching/detaching takes <1ms, so any work expected to take multiple milliseconds benefits from detaching.

```rust
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyfunction]
fn parse_response<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyAny>> {
    // ✅ Detach from interpreter during pure-Rust work
    let parsed = py.detach(|| {
        serde_json::from_slice::<serde_json::Value>(data)
    }).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    // Re-attach automatically; convert to Python only here
    pythonize::pythonize(py, &parsed)
}
```

**Batch pattern — minimize attached time:**

```rust
use rayon::prelude::*;

#[pyfunction]
fn process_batch<'py>(py: Python<'py>, items: Vec<String>) -> PyResult<Vec<String>> {
    // Phase 1: Detach and do heavy Rust work in parallel
    let results = py.detach(|| {
        items.into_par_iter()
            .map(|item| item.to_uppercase()) // example transform
            .collect::<Vec<_>>()
    });

    // Phase 2: Return — PyO3 handles Vec<String> → list[str] conversion
    Ok(results)
}
```

---

## 2. Use `cast` Instead of `extract` for Type Checks

This comes directly from the [PyO3 0.28 performance guide](https://pyo3.rs/v0.28.0/performance.html). When you're doing polymorphic dispatch and **ignoring the error**, use `cast` instead of `extract` to avoid the costly `PyDowncastError` → `PyErr` conversion.

```rust
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};
use pyo3::exceptions::PyTypeError;

#[pyfunction]
fn process<'py>(value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    // ✅ Use `cast` — avoids costly PyDowncastError → PyErr conversion
    if let Ok(list) = value.cast::<PyList>() {
        process_list(list)
    } else if let Ok(dict) = value.cast::<PyDict>() {
        process_dict(dict)
    } else if let Ok(s) = value.cast::<PyString>() {
        process_string(s)
    } else {
        // Only pay error conversion cost on the final fallback
        Err(PyTypeError::new_err("Unsupported type"))
    }
}
```

**When to use which:**

| Method | Use When | Cost |
|--------|----------|------|
| `cast::<T>()` | Type-checking native Python types, error is ignored | Cheap — no `PyErr` allocation |
| `extract::<T>()` | You need the Rust value, or need the `PyErr` | More expensive due to error conversion |

---

## 3. Zero-Cost Python Token Access via `Bound::py()`

Another tip from the official performance page: if you already have a `Bound<'py, T>` reference, use `.py()` to get the `Python<'py>` token instead of calling `Python::attach`. `Python::attach` has a small but measurable cost from checking if the thread is already attached.

```rust
use pyo3::prelude::*;
use pyo3::types::PyList;

struct Inner(Py<PyList>);

struct InnerBound<'py>(Bound<'py, PyList>);

impl PartialEq<Inner> for InnerBound<'_> {
    fn eq(&self, other: &Inner) -> bool {
        // ✅ Zero-cost token access from existing Bound reference
        let py = self.0.py();
        let other_len = other.0.bind(py).len();
        self.0.len() == other_len
    }
}

// ❌ Avoid: unnecessary Python::attach when you already have a Bound
// Python::attach(|py| { ... }) // has overhead from attachment check
```

---

## 4. Use Vectorcall Protocol for Calling Python

PyO3 0.28 will use the more efficient `vectorcall` protocol (PEP 590) when you pass **Rust tuples** as call arguments. `Bound<'_, PyTuple>` and `Py<PyTuple>` can only use the older, slower `tp_call` protocol.

```rust
use pyo3::prelude::*;

#[pyfunction]
fn call_callback(py: Python<'_>, callback: &Bound<'_, PyAny>) -> PyResult<PyObject> {
    // ✅ Rust tuple → vectorcall (fast path)
    let result = callback.call1((42, "hello", true))?;

    // ❌ Avoid: PyTuple → tp_call (slower path)
    // let args = PyTuple::new(py, &[42.into_pyobject(py)?, ...])?;
    // let result = callback.call1(args)?;

    Ok(result.unbind())
}
```

**Key rule:** Prefer Rust tuples `(arg1, arg2, ...)` over constructing `PyTuple` for all `.call()`, `.call1()`, and `.call_method1()` invocations.

---

## 5. Disable the Global Reference Pool

PyO3 maintains a global mutable reference pool for deferred reference count updates when `Py<T>` is dropped without being attached to the interpreter. The synchronization overhead can become significant at the Python-Rust boundary.

Add to your `.cargo/config.toml`:

```toml
[build]
rustflags = ["--cfg", "pyo3_disable_reference_pool"]
```

**Tradeoff:** With this flag, dropping a `Py<T>` (or types containing it like `PyErr`, `PyBackedStr`, `PyBackedBytes`) without being attached will **abort**. So you must ensure all Python objects are dropped while attached:

```rust
use pyo3::prelude::*;
use pyo3::types::PyList;

// ✅ Correct: drop within an attached context
let numbers: Py<PyList> = Python::attach(|py| PyList::empty(py).unbind());

Python::attach(|py| {
    numbers.bind(py).append(42).unwrap();
});

// Explicitly drop while attached
Python::attach(move |py| {
    drop(numbers);  // Safe — we're attached
});
```

Optionally add `pyo3_leak_on_drop_without_reference_pool` to leak instead of abort (prevents crashes but may cause resource exhaustion long-term).

---

## 6. Avoid Unnecessary Data Copies

### Zero-copy buffer access

```rust
use pyo3::prelude::*;

#[pyfunction]
fn compute_checksum(data: &[u8]) -> u32 {
    // `data` borrows directly from Python's buffer — zero copy
    data.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32))
}
```

### Keep data Rust-side in `#[pyclass]`

```rust
use pyo3::prelude::*;

#[pyclass]
struct Response {
    // Store as Rust types — no Python overhead
    status: u16,
    body: Option<bytes::Bytes>,  // requires `bytes` feature in PyO3 0.28!
    headers: std::collections::HashMap<String, String>,
}

#[pymethods]
impl Response {
    #[getter]
    fn status_code(&self) -> u16 {
        self.status  // Cheap: primitive copy
    }

    fn json(&self, py: Python<'_>) -> PyResult<PyObject> {
        let bytes = self.body.as_ref()
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("no body"))?;
        // Convert to Python only on explicit request
        py.detach(|| serde_json::from_slice::<serde_json::Value>(bytes))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
            .and_then(|v| Ok(pythonize::pythonize(py, &v)?))
    }
}
```

### New in 0.28: `bytes` crate integration

PyO3 0.28 adds optional `bytes` crate support for zero-copy `bytes::Bytes` ↔ Python conversion:

```toml
[dependencies]
pyo3 = { version = "0.28", features = ["bytes"] }
```

---

## 7. Smart Type Conversions with `IntoPyObject`

PyO3 0.28 has fully removed the deprecated `ToPyObject` and `IntoPy` traits. Use **`IntoPyObject`** exclusively:

```rust
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyfunction]
fn process_config(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<PyObject> {
    // Extract once into Rust types, work in Rust
    let map: HashMap<String, String> = data.extract()?;

    let result = py.detach(|| {
        map.into_iter()
            .filter(|(k, _)| !k.starts_with("_"))
            .collect::<HashMap<_, _>>()
    });

    // Single conversion back to Python using IntoPyObject
    Ok(result.into_pyobject(py)?.into_any().unbind())
}
```

**Use `Cow<str>` to avoid allocation when possible:**

```rust
use std::borrow::Cow;

#[pyfunction]
fn normalize_url(url: &str) -> Cow<'_, str> {
    if url.ends_with('/') {
        Cow::Borrowed(url)  // No allocation
    } else {
        Cow::Owned(format!("{}/", url))
    }
}
```

---

## 8. `#[pyclass]` Optimization

```rust
use std::sync::Arc;

#[pyclass(frozen)]          // Immutable → no locking overhead on field access
#[pyclass(freelist = 256)]  // Object pool for frequently created/destroyed objects
struct Headers {
    inner: Arc<reqwest::header::HeaderMap>,  // Arc for cheap sharing across Rust threads
}
```

### Free-threaded Python support (0.28)

PyO3 0.28 requires `#[pyclass]` types to implement `Sync` (for free-threaded builds). Free-threaded support is now **opt-out** rather than opt-in:

```rust
// ✅ Works on both GIL-enabled and free-threaded builds
#[pyclass(frozen)]  // Frozen makes Sync trivial for most types
struct Config {
    timeout: u64,
    base_url: String,
}

// For mutable state, use interior mutability with Sync-safe primitives
use std::sync::Mutex;

#[pyclass]
struct ConnectionPool {
    connections: Mutex<Vec<Connection>>,  // Mutex is Sync
}
```

---

## 9. Async Integration

For HTTP clients like RequestX, use a **shared, long-lived Tokio runtime**:

```rust
use std::sync::OnceLock;
use tokio::runtime::Runtime;

fn get_runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        Runtime::new().expect("Failed to create Tokio runtime")
    })
}
```

Bridge Rust futures to Python awaitables:

```rust
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyfunction]
fn fetch<'py>(py: Python<'py>, url: String) -> PyResult<Bound<'py, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        // Runs on Tokio runtime — automatically detached from interpreter
        let resp = reqwest::get(&url).await
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        let bytes = resp.bytes().await
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(Python::attach(|py| {
            PyBytes::new(py, &bytes).unbind()
        }))
    })
}
```

---

## 10. Efficient String Handling

```rust
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;

// ✅ Accept &str — zero-copy borrow from Python string
#[pyfunction]
fn validate_url(url: &str) -> bool {
    url.starts_with("https://")
}

// ✅ Use PyBackedStr (0.28) when you need owned string data
// without keeping the Python object alive
#[pyfunction]
fn extract_host(url: &Bound<'_, pyo3::types::PyString>) -> PyResult<String> {
    let backed: PyBackedStr = url.extract()?;
    // PyBackedStr::as_str() is new in 0.28
    Ok(backed.as_str().split('/').nth(2).unwrap_or("").to_string())
}
```

---

## 11. Error Handling in Hot Paths

Avoid `unwrap()` — panics across FFI are expensive and can cause undefined behavior:

```rust
use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError};

#[pyfunction]
fn parse_json(data: &[u8]) -> PyResult<String> {
    // ✅ Use ? with proper error mapping
    let value: serde_json::Value = serde_json::from_slice(data)
        .map_err(|e| PyValueError::new_err(format!("JSON parse error: {e}")))?;
    Ok(value.to_string())
}
```

---

## 12. Build Configuration

### Cargo.toml

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.28.0", features = ["extension-module"] }

[profile.release]
lto = "fat"           # Link-time optimization (slower build, faster binary)
codegen-units = 1     # Better optimization (slower build)
opt-level = 3
strip = true          # Smaller .so/.pyd file

[profile.release.build-override]
opt-level = 3
```

### .cargo/config.toml (optional, for maximum performance)

```toml
[build]
rustflags = [
    "--cfg", "pyo3_disable_reference_pool",  # Remove global ref pool overhead
]
```

### pyproject.toml (maturin)

```toml
[build-system]
requires = ["maturin>=1.7,<2.0"]
build-backend = "maturin"

[tool.maturin]
features = ["pyo3/extension-module"]
strip = true
```

---

## 13. PEP 489 Multi-Phase Module Initialization (0.28)

PyO3 0.28 switches `#[pymodule]` to use PEP 489 multi-phase initialization internally. No code changes needed, but this prepares for future subinterpreter support and is slightly more efficient:

```rust
use pyo3::prelude::*;

#[pymodule]
fn requestx(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Response>()?;
    m.add_function(wrap_pyfunction!(fetch, m)?)?;
    Ok(())
}
```

---

## Quick Reference Summary

| Technique | Impact | Effort | PyO3 Version |
|---|---|---|---|
| `py.detach()` for CPU/IO work | 🔥🔥🔥 | Low | 0.28+ (`allow_threads` before) |
| `cast` over `extract` for type checks | 🔥🔥🔥 | Low | 0.28+ |
| `pyo3_disable_reference_pool` flag | 🔥🔥🔥 | Low | 0.28+ |
| Zero-copy buffer / `bytes` feature | 🔥🔥🔥 | Medium | 0.28+ |
| Rust tuples for vectorcall | 🔥🔥 | Low | 0.28+ |
| `Bound::py()` over `Python::attach` | 🔥🔥 | Low | 0.28+ |
| Shared Tokio runtime | 🔥🔥🔥 | Low | Any |
| Keep data Rust-side in `#[pyclass]` | 🔥🔥 | Medium | Any |
| `#[pyclass(frozen)]` | 🔥🔥 | Low | 0.23+ |
| `IntoPyObject` (replaces `ToPyObject`) | 🔥 | Medium | 0.28+ |
| LTO + `codegen-units = 1` | 🔥 | Trivial | Any |
| `freelist` for hot objects | 🔥 | Trivial | Any |

---

## Migration Notes for 0.28

| Old API | New API (0.28) |
|---------|---------------|
| `py.allow_threads(\|\| { ... })` | `py.detach(\|\| { ... })` |
| `value.extract::<PyList>()` (when ignoring error) | `value.cast::<PyList>()` |
| `Python::with_gil(\|py\| { ... })` | `Python::attach(\|py\| { ... })` |
| `ToPyObject` / `IntoPy` traits | `IntoPyObject` trait |
| `PyObject` type alias | `Py<PyAny>` (PyObject is deprecated) |
| `AsPyPointer` trait | Use `Py<T>`, `Bound<T>`, `Borrowed<T>` methods |
| Free-threaded opt-in | Free-threaded opt-out (support is default) |
