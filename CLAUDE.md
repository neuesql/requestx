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

## Test Status: 74 failed / 1332 passed / 1 skipped (Total: 1407)

### Recent Improvements
- **Client/AsyncClient exception conversion**: All HTTP methods now properly convert Rust exceptions to Python
- **URL validation**: Empty scheme (`://example.org`) and empty host (`http://`) now raise UnsupportedProtocol
- **Iterator type checking**: Sync Client rejects async iterators, AsyncClient rejects sync iterators with RuntimeError
- **Content streaming** (43/43 tests passing): BytesIO, iterators, async iterators, stream mode detection
- **Request.stream**: Proper sync/async/dual mode detection with StreamConsumed handling
- **DeprecationWarning**: Emitted when using `data=` with bytes/iterator content
- **URL fixes**: IPv6 preservation, IDNA encoding, relative paths, userinfo encoding
- **Transport lifecycle**: Mounted transports properly enter/exit with context manager
- Proxy support: `_transport_for_url`, `_transport`, `_mounts` dictionary, proxy env vars
- Auth generator protocol: `sync_auth_flow` and `async_auth_flow` work with custom auth classes
- DigestAuth implementation with MD5, SHA, SHA-256, SHA-512 algorithm support

| ID | Test File | Tests (F/P) | Features | Status | Priority |
|----|-----------|-------------|----------|--------|----------|
| 1 | client/test_async_client.py | 8/44 | Async streaming, build_request, transport | 🟡 Partial | P0 |
| 2 | client/test_auth.py | 15/64 | Basic/Digest auth, custom auth, netrc | 🟡 Partial | P0 |
| 3 | client/test_client.py | 4/31 | build_request, transport, URL merge | 🟡 Partial | P0 |
| 4 | models/test_url.py | 7/83 | RFC3986 compliance, IDNA, IPv6 | 🟢 Mostly | P1 |
| 5 | test_timeouts.py | 6/4 | Read/write/connect/pool timeout | 🟡 Partial | P1 |
| 6 | client/test_event_hooks.py | 6/3 | Hooks on redirects | 🟡 Partial | P2 |
| 7 | client/test_redirects.py | 5/26 | history, next_request, streaming body | 🟢 Mostly | P1 |
| 8 | models/test_cookies.py | 4/3 | Domain/path support, repr | 🟡 Partial | P2 |
| 9 | test_auth.py | 4/4 | Digest auth nonce, RFC 7616 | 🟡 Partial | P1 |
| 10 | client/test_queryparams.py | 3/0 | Client query params | 🔴 Failing | P2 |
| 11 | test_api.py | 2/10 | Iterator content | 🟢 Mostly | P2 |
| 12 | models/test_headers.py | 2/25 | Header encoding, repr | 🟢 Mostly | P2 |
| 13 | client/test_headers.py | 2/15 | Host header with port | 🟢 Mostly | P2 |
| 14 | test_multipart.py | 1/37 | Non-seekable file-like | 🟢 Mostly | P2 |
| 15 | models/test_responses.py | 1/105 | Response pickling | 🟢 Mostly | P2 |
| 16 | test_config.py | 1/27 | SSLContext with request | 🟢 Mostly | P2 |
| 17 | client/test_properties.py | 1/7 | Client headers | 🟢 Mostly | P2 |
| 18 | test_exported_members.py | 1/0 | Module exports | 🔴 Failing | P2 |
| 19 | test_exceptions.py | 1/2 | Request attribute | 🟢 Mostly | P2 |
| 20 | test_content.py | 0/43 | Stream markers, async iterators, bytesio | ✅ Done | - |
| 21 | models/test_requests.py | 0/24 | Request.stream, pickle, generators | ✅ Done | - |
| 22 | client/test_proxies.py | 0/69 | Proxy env vars | ✅ Done | - |
| 23 | models/test_whatwg.py | 0/563 | WHATWG URL parsing | ✅ Done | - |
| 24 | test_decoders.py | 0/40 | gzip/brotli/zstd/deflate | ✅ Done | - |
| 25 | test_utils.py | 0/40 | guess_json_utf, BOM | ✅ Done | - |
| 26 | test_asgi.py | 0/24 | ASGITransport | ✅ Done | - |
| 27 | models/test_queryparams.py | 0/14 | set(), add(), remove() | ✅ Done | - |
| 28 | test_wsgi.py | 0/12 | WSGI transport | ✅ Done | - |
| 29 | client/test_cookies.py | 0/7 | Cookie jar, persistence | ✅ Done | - |
| 30 | test_status_codes.py | 0/6 | Status codes | ✅ Done | - |

### Top Failing Categories
1. **Async client** (20 failures): Cancellation, server extensions, streaming
2. **Client auth** (15 failures): Basic auth in URL, custom auth, digest auth edge cases
3. **Client** (15 failures): Invalid URL handling, URL merging, transport mounting
4. **URL edge cases** (7 failures): Path encoding, percent escaping, invalid components
5. **Timeouts** (6 failures): Connect/write/pool timeout exception types

### Known Issues (Priority Order)
1. **Timeout exceptions**: Need to raise correct exception types (ReadTimeout, ConnectTimeout, etc.)
2. **URL path encoding**: Special characters in path/query/fragment
3. **Client URL merging**: Relative URL handling with base URL
4. **Auth in URL**: Basic auth credentials in URL not being extracted
5. **Event hooks on redirects**: Hooks not firing properly during redirect chains
