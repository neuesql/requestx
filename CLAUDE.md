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

## Test Status: 238 failed / 1168 passed / 1 skipped (Total: 1407)

### Recent Improvements
- **Response async streaming** (33 more tests passing): `aiter_raw`, `aiter_bytes`, `aiter_lines` implemented in Python wrapper
- Proxy support: `_transport_for_url`, `_transport`, `_mounts` dictionary, proxy env vars (HTTP_PROXY, HTTPS_PROXY, ALL_PROXY, NO_PROXY)
- URL: Added `raw_scheme` property, fixed `raw_host` IPv6 bracket handling
- Auth generator protocol: `sync_auth_flow` and `async_auth_flow` work with custom auth classes
- DigestAuth implementation with MD5, SHA, SHA-256, SHA-512 algorithm support
- AsyncClient and Client auth type validation (raises TypeError for invalid auth)
- AsyncClient and Client stream() context manager with auth support
- Transport routing in auth flows (_send_single_request pattern)
- HTTPStatusError now has `request` and `response` attributes
- Response history tracking during auth flows
- AsyncClient properly handles custom transports with auth flows
- Response.request setter now works
- Request.headers proxy properly syncs with Rust headers
- AsyncClient/Client context manager calls transport lifecycle methods
- MutableHeaders.raw property for raw header bytes
- Content-length: 0 header for POST/PUT/PATCH without body
- ASGI transport working (24/24 tests passing)
- Decoders working (40/40 tests passing)
- Utils working (40/40 tests passing)
- Redirects mostly working (26/31 tests passing)

| ID | Test File | Tests (F/P) | Features | Status | Priority |
|----|-----------|-------------|----------|--------|----------|
| 1 | models/test_responses.py | 27/79 | Response streaming, encoding, async iter | 🟡 Partial | P0 |
| 2 | models/test_url.py | 48/42 | RFC3986 compliance, IDNA, IPv6 | 🟡 Partial | P0 |
| 3 | test_multipart.py | 28/10 | Boundary parsing, file tuples, validation | 🔴 Failing | P0 |
| 4 | client/test_async_client.py | 22/30 | Async streaming, build_request, transport | 🟡 Partial | P0 |
| 5 | client/test_auth.py | 21/58 | Basic/Digest auth, custom auth, netrc | 🟡 Partial | P0 |
| 6 | test_content.py | 18/25 | Stream markers, async iterators, bytesio | 🟡 Partial | P0 |
| 7 | models/test_requests.py | 15/9 | Request.stream, pickle, generators | 🟡 Partial | P1 |
| 8 | client/test_client.py | 14/21 | build_request, transport, URL merge | 🟡 Partial | P1 |
| 9 | test_timeouts.py | 10/0 | Read/write/connect/pool timeout | 🔴 Failing | P1 |
| 10 | client/test_cookies.py | 7/0 | Cookie jar, persistence | 🔴 Failing | P1 |
| 11 | client/test_event_hooks.py | 6/3 | Hooks on redirects | 🟡 Partial | P2 |
| 12 | client/test_redirects.py | 5/26 | history, next_request, streaming body | 🟢 Mostly | P1 |
| 13 | models/test_cookies.py | 4/3 | Domain/path support, repr | 🟡 Partial | P2 |
| 14 | test_auth.py | 4/4 | Digest auth nonce, RFC 7616 | 🟡 Partial | P1 |
| 15 | client/test_queryparams.py | 3/0 | Client query params | 🔴 Failing | P2 |
| 16 | models/test_headers.py | 2/25 | Header encoding, repr | 🟢 Mostly | P2 |
| 17 | client/test_headers.py | 2/15 | Host header with port | 🟢 Mostly | P2 |
| 18 | test_api.py | 2/10 | Iterator content | 🟢 Mostly | P2 |
| 19 | test_config.py | 1/27 | SSLContext with request | 🟢 Mostly | P2 |
| 20 | client/test_properties.py | 1/7 | Client headers | 🟢 Mostly | P2 |
| 21 | test_exported_members.py | 1/0 | Module exports | 🔴 Failing | P2 |
| 22 | test_exceptions.py | 0/3 | Exception hierarchy | ✅ Done | - |
| 23 | client/test_proxies.py | 0/69 | Proxy env vars | ✅ Done | - |
| 24 | models/test_whatwg.py | 0/563 | WHATWG URL parsing | ✅ Done | - |
| 25 | test_decoders.py | 0/40 | gzip/brotli/zstd/deflate | ✅ Done | - |
| 26 | test_utils.py | 0/40 | guess_json_utf, BOM | ✅ Done | - |
| 27 | test_asgi.py | 0/24 | ASGITransport | ✅ Done | - |
| 28 | models/test_queryparams.py | 0/14 | set(), add(), remove() | ✅ Done | - |
| 29 | test_wsgi.py | 0/12 | WSGI transport | ✅ Done | - |
| 30 | test_status_codes.py | 0/6 | Status codes | ✅ Done | - |

### Top Failing Categories
1. **URL edge cases** (48 failures): Empty scheme, IPv6, IDNA encoding, path encoding
2. **Multipart** (28 failures): Boundary parsing, file tuples, content-type handling
3. **Response streaming** (27 failures): Sync streaming, encoding fallback, pickling
4. **Async client** (22 failures): Build request, streaming, transport mounting
5. **Auth flows** (21 failures): Basic auth assertion, digest nonce counting, netrc

### Known Issues (Priority Order)
1. **URL scheme handling**: Empty scheme URLs (e.g., "://example.com") not fully supported
2. **Multipart boundary**: Boundary extraction from content-type header
3. **Response encoding**: Fallback encoding detection, explicit encoding setting
4. **Timeout exceptions**: Need to raise correct exception types (ReadTimeout, ConnectTimeout, etc.)
5. **Cookie jar integration**: Cookie persistence across requests
