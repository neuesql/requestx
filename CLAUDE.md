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

## Test Status: 31 failed / 1375 passed / 1 skipped (Total: 1407)

### Recent Improvements
- **Auth improvements** (79/79 tests passing): Basic auth in URL, custom auth callables, NetRCAuth, RepeatAuth generator flow, ResponseBodyAuth, streaming body digest auth, MockTransport handler property
- **Timeout exception types** (10/10 tests passing): ConnectTimeout, WriteTimeout, ReadTimeout now properly classified using timeout context
- **URL fragment decoding**: Fragments are now properly percent-decoded when returned
- **Limits support**: AsyncClient now accepts `limits` parameter for connection pool configuration
- **Exception request attribute**: All exceptions now have `request` property that raises RuntimeError when not set
- **Client headers isinstance**: `_HeadersProxy` now inherits from Headers, passing isinstance checks
- **Top-level API iterators**: `post()`, `put()`, `patch()` now consume generators/iterators before passing to Rust
- **Headers repr encoding**: Repr now includes encoding suffix when not 'ascii'
- **AsyncClient streaming** (52/52 tests passing): ResponseNotRead, StreamClosed, async iterator content, MockTransport, http_version extensions
- **Response pickling** (106/106 tests passing): Streaming responses correctly raise StreamClosed after unpickling
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

| ID | Test File | Failed | Features | Status | Priority | Effort |
|----|-----------|--------|----------|--------|----------|--------|
| 1 | client/test_auth.py | 0 | Basic auth URL, custom auth, netrc, digest, streaming | ✅ Done | - | - |
| 2 | client/test_async_client.py | 0 | ResponseNotRead, async iterator, http_version | ✅ Done | - | - |
| 3 | models/test_url.py | 6 | Query/fragment encoding, percent escape, validation | 🟢 Mostly | P1 | M |
| 4 | test_timeouts.py | 2 | Pool timeout not firing | 🟢 Mostly | P2 | M |
| 5 | client/test_event_hooks.py | 6 | Hooks not firing on redirects | 🟡 Partial | P2 | M |
| 6 | client/test_redirects.py | 5 | Streaming body, malformed, cookies | 🟢 Mostly | P1 | M |
| 7 | client/test_client.py | 3 | Raw header, autodetect encoding | 🟢 Mostly | P1 | M |
| 8 | models/test_cookies.py | 4 | Domain/path support, repr | 🟡 Partial | P2 | M |
| 9 | test_api.py | 0 | Iterator content in top-level API | ✅ Done | - | - |
| 10 | models/test_headers.py | 1 | Explicit encoding decode | 🟢 Mostly | P2 | M |
| 11 | client/test_headers.py | 0 | Auth extraction from URL | ✅ Done | - | - |
| 12 | test_multipart.py | 1 | Non-seekable file-like | 🟢 Mostly | P2 | M |
| 13 | models/test_responses.py | 0 | Response pickling | ✅ Done | - | - |
| 14 | test_config.py | 1 | SSLContext with request | 🟢 Mostly | P2 | M |
| 15 | client/test_properties.py | 0 | Client headers case | ✅ Done | - | - |
| 16 | test_exceptions.py | 0 | Request attribute on exception | ✅ Done | - | - |
| 17 | test_auth.py | 2 | Digest auth RFC 7616 cnonce format | 🟢 Mostly | P2 | M |
| 18 | client/test_queryparams.py | 0 | Client query params | ✅ Done | - | - |
| 19 | test_exported_members.py | 0 | Module exports | ✅ Done | - | - |
| 20 | test_content.py | 0 | Stream markers, async iterators, bytesio | ✅ Done | - | - |
| 21 | models/test_requests.py | 0 | Request.stream, pickle, generators | ✅ Done | - | - |
| 22 | client/test_proxies.py | 0 | Proxy env vars | ✅ Done | - | - |
| 23 | models/test_whatwg.py | 0 | WHATWG URL parsing | ✅ Done | - | - |
| 24 | test_decoders.py | 0 | gzip/brotli/zstd/deflate | ✅ Done | - | - |
| 25 | test_utils.py | 0 | guess_json_utf, BOM | ✅ Done | - | - |
| 26 | test_asgi.py | 0 | ASGITransport | ✅ Done | - | - |
| 27 | models/test_queryparams.py | 0 | set(), add(), remove() | ✅ Done | - | - |
| 28 | test_wsgi.py | 0 | WSGI transport | ✅ Done | - | - |
| 29 | client/test_cookies.py | 0 | Cookie jar, persistence | ✅ Done | - | - |
| 30 | test_status_codes.py | 0 | Status codes | ✅ Done | - | - |

**Effort Legend:** L = Low (localized fix), M = Medium (multiple components), H = High (architectural)

### Top Failing Categories
1. **URL edge cases** (6 failures): Query encoding, percent escape host, validation
2. **Event hooks** (6 failures): Hooks not firing on redirect responses
3. **Redirects** (5 failures): Streaming body redirect, malformed redirect, cookie behavior
4. **Cookies** (4 failures): Domain/path support, repr formatting
5. **Client encoding** (3 failures): Raw header, autodetect encoding, explicit encoding

### Known Issues (Priority Order)
1. **Event hooks on redirect**: Hooks need to fire for each redirect response (M)
2. **Encoding detection**: `default_encoding` callable not being used for autodetection (M)
3. **Cookie domain/path**: Cookie matching with domain and path constraints (M)
5. **Netrc support**: Parse netrc file for auth credentials (M)
6. **Custom auth**: Auth generator protocol needs proper response body access (M)
7. **Headers explicit encoding**: Lazy re-decode when encoding property is changed (M)
