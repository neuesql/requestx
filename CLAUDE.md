# RequestX

High-performance Python HTTP client, API-compatible with httpx, powered by Rust's reqwest via PyO3.

## Quick Commands
```bash
# Build (always use release for accurate perf testing)
maturin develop --release

# Test - reference tests (DO NOT MODIFY)
pytest tests_httpx/ -v

# Test - target tests (must all pass)
pytest tests_requestx/ -v

# Both (verify compatibility)
pytest tests_httpx/ tests_requestx/ -v

# Lint & format
cargo clippy && cargo fmt
ruff check python/ && ruff format python/
```

## Project Structure
```
src/                      # Rust implementation (ALL business logic here)
python/requestx/
└── __init__.py           # ONLY exports from Rust, NO business logic

tests_httpx/              # Reference tests (DO NOT MODIFY)
tests_requestx/           # Target tests (must all pass)
```

## Core Dependencies (Cargo.toml)
```toml
[dependencies]
pyo3 = { version = "0.27", features = ["extension-module"] }
pyo3-async-runtimes = { version = "0.27", features = ["tokio-runtime"] }
reqwest = { version = "0.13", features = ["blocking", "json", "cookies", "gzip", "brotli", "deflate", "zstd", "multipart", "stream", "rustls", "socks", "http2"] }
tokio = { version = "1", features = ["full"] }
sonic-rs = "0.5"
serde = { version = "1.0", features = ["derive"] }
url = "2"
bytes = "1"
http = "1"
```

## Critical Rules

### 1. Rust-First Architecture
- **ALL** business logic in Rust
- `python/requestx/__init__.py` contains ONLY re-exports
- Never call Python libraries from Rust (use Rust equivalents)

### 2. PyO3 Patterns
```rust
// ✅ Use Python::attach(), not deprecated with_gil()
Python::attach(|py| { ... })

// ✅ Strong type signatures (compile-time checking)
fn process(url: &str, data: Vec<i64>) -> PyResult<String>

// ❌ Avoid PyAny (runtime overhead)
fn process(data: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>>
```

### 3. GIL Management
```rust
// ✅ Extract data FIRST, then release GIL for I/O
#[pyfunction]
fn fetch(py: Python, url: String) -> PyResult<String> {
    py.allow_threads(|| {
        // Network I/O here - GIL released
        blocking_fetch(&url)
    })
}
```

Release GIL for: network I/O, file I/O, CPU work >1ms
Keep GIL for: Python object access, operations <1ms

### 4. Async Pattern
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

### 5. JSON: Always sonic-rs
```rust
// ✅ sonic-rs (SIMD-accelerated, 50-300x faster than Python json)
let parsed: Value = sonic_rs::from_str(&json_str)?;
let output = sonic_rs::to_string(&value)?;

// ❌ Never call Python's json module
```

### 6. Memory Efficiency
```rust
// ✅ Return references, not clones
#[getter]
fn url(&self) -> &str { &self.url }

// ✅ Zero-copy for bytes
#[getter]
fn content(&self, py: Python) -> Bound<'_, PyBytes> {
    PyBytes::new_bound(py, &self.content)
}

// ✅ Pre-allocate when size known
let mut headers = Vec::with_capacity(response.headers().len());
```

## Don't

- ❌ Modify `tests_httpx/` (reference tests)
- ❌ Put business logic in Python
- ❌ Use `panic!` (crashes Python)
- ❌ Convert types inside loops (convert once at boundary)
- ❌ Use deprecated `Python::with_gil()`

## API Compatibility

Must implement all public APIs from [httpx](https://github.com/encode/httpx/tree/master/httpx), excluding CLI.

Check `httpx/__init__.py` for the complete public API surface. Goal: `import requestx as httpx` works as drop-in replacement.

## Success Criteria
```bash
pytest tests_requestx/ -v  # ALL PASSED
```

- Drop-in compatible: `import requestx as httpx` works
- Performance ≥ httpx
- Zero Python business logic

## References

- httpx source: https://github.com/encode/httpx/tree/master/httpx
- pyreqwest: https://github.com/MarkusSintonen/pyreqwest

---

## Test Status: 64 failed / 1342 passed / 1 skipped (Total: 1407)

### Recent Improvements
- **Client params**: Client now supports `params` constructor argument with proper QueryParams merging
- **Module exports**: Fixed `__all__` to be case-insensitively sorted, hidden internal imports
- **DigestAuth** (8/8 tests passing): Full RFC 2069/7616 compliance, nonce counting, cookie preservation
- **Response constructor**: Properly unwraps `_WrappedRequest` to pass to Rust `_Response`
- **Client/AsyncClient exception conversion**: All HTTP methods now properly convert Rust exceptions to Python
- **URL validation**: Empty scheme (`://example.org`) and empty host (`http://`) now raise UnsupportedProtocol
- **Iterator type checking**: Sync Client rejects async iterators, AsyncClient rejects sync iterators with RuntimeError
- **Content streaming** (43/43 tests passing): BytesIO, iterators, async iterators, stream mode detection
- **Request.stream**: Proper sync/async/dual mode detection with StreamConsumed handling
- **Transport lifecycle**: Mounted transports properly enter/exit with context manager
- Proxy support: `_transport_for_url`, `_transport`, `_mounts` dictionary, proxy env vars
- Auth generator protocol: `sync_auth_flow` and `async_auth_flow` work with custom auth classes

| ID | Test File | Tests (F/P) | Features | Status | Priority | Effort |
|----|-----------|-------------|----------|--------|----------|--------|
| 1 | client/test_auth.py | 13/66 | Basic auth URL, custom auth, netrc, digest trio | 🟡 Partial | P0 | H |
| 2 | client/test_async_client.py | 8/44 | ResponseNotRead, async iterator, http_version | 🟡 Partial | P0 | M |
| 3 | models/test_url.py | 7/83 | Query/fragment encoding, percent escape, validation | 🟢 Mostly | P1 | M |
| 4 | test_timeouts.py | 6/4 | Write/connect/pool timeout exception types | 🟡 Partial | P1 | L |
| 5 | client/test_event_hooks.py | 6/3 | Hooks not firing on redirects | 🟡 Partial | P2 | M |
| 6 | client/test_redirects.py | 5/26 | Streaming body, malformed, cookies | 🟢 Mostly | P1 | M |
| 7 | client/test_client.py | 4/31 | Raw header, server extensions, autodetect encoding | 🟡 Partial | P0 | M |
| 8 | models/test_cookies.py | 4/3 | Domain/path support, repr | 🟡 Partial | P2 | M |
| 9 | test_api.py | 2/10 | Iterator content in top-level API | 🟢 Mostly | P2 | L |
| 10 | models/test_headers.py | 2/25 | Encoding in repr, explicit decode | 🟢 Mostly | P2 | L |
| 11 | client/test_headers.py | 2/15 | Host header with port | 🟢 Mostly | P2 | L |
| 12 | test_multipart.py | 1/37 | Non-seekable file-like | 🟢 Mostly | P2 | M |
| 13 | models/test_responses.py | 1/105 | Response pickling | 🟢 Mostly | P2 | M |
| 14 | test_config.py | 1/27 | SSLContext with request | 🟢 Mostly | P2 | M |
| 15 | client/test_properties.py | 1/7 | Client headers case | 🟢 Mostly | P2 | L |
| 16 | test_exceptions.py | 1/2 | Request attribute on exception | 🟢 Mostly | P2 | L |
| 17 | test_auth.py | 0/8 | Digest auth nonce, RFC 7616, cookies | ✅ Done | - | - |
| 18 | client/test_queryparams.py | 0/3 | Client query params | ✅ Done | - | - |
| 19 | test_exported_members.py | 0/1 | Module exports | ✅ Done | - | - |
| 20 | test_content.py | 0/43 | Stream markers, async iterators, bytesio | ✅ Done | - | - |
| 21 | models/test_requests.py | 0/24 | Request.stream, pickle, generators | ✅ Done | - | - |
| 22 | client/test_proxies.py | 0/69 | Proxy env vars | ✅ Done | - | - |
| 23 | models/test_whatwg.py | 0/563 | WHATWG URL parsing | ✅ Done | - | - |
| 24 | test_decoders.py | 0/40 | gzip/brotli/zstd/deflate | ✅ Done | - | - |
| 25 | test_utils.py | 0/40 | guess_json_utf, BOM | ✅ Done | - | - |
| 26 | test_asgi.py | 0/24 | ASGITransport | ✅ Done | - | - |
| 27 | models/test_queryparams.py | 0/14 | set(), add(), remove() | ✅ Done | - | - |
| 28 | test_wsgi.py | 0/12 | WSGI transport | ✅ Done | - | - |
| 29 | client/test_cookies.py | 0/7 | Cookie jar, persistence | ✅ Done | - | - |
| 30 | test_status_codes.py | 0/6 | Status codes | ✅ Done | - | - |

**Effort Legend:** L = Low (localized fix), M = Medium (multiple components), H = High (architectural)

### Top Failing Categories
1. **Client auth** (13 failures): Basic auth in URL, custom auth, netrc, digest trio edge cases
2. **Async client** (8 failures): ResponseNotRead on streamed, async iterator streaming, http_version
3. **URL edge cases** (7 failures): Query/fragment encoding, percent escaping, component validation
4. **Timeouts** (6 failures): Write/connect/pool timeout exception type mapping
5. **Event hooks** (6 failures): Hooks not firing on redirect responses
6. **Redirects** (5 failures): Streaming body redirect, malformed redirect, cookie behavior

### Known Issues (Priority Order)
1. **ResponseNotRead**: Need to raise when accessing `.content` on streamed response (M)
2. **Async iterator streaming**: Support async iterator content in requests (M)
3. **Server extensions**: `http_version` extension missing from response (L)
4. **Timeout exceptions**: Map Rust timeout errors to ConnectTimeout/WriteTimeout/PoolTimeout (L)
5. **Event hooks on redirect**: Hooks need to fire for each redirect response (M)
6. **Encoding detection**: `default_encoding` callable not being used for autodetection (M)
7. **URL auth extraction**: Parse and strip basic auth credentials from URL (M)
8. **Netrc support**: Parse netrc file for auth credentials (M)
9. **Custom auth**: Auth generator protocol needs proper response body access (M)
10. **Header case**: Preserve original header case in some contexts (L)
