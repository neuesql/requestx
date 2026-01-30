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

## Test Status: 392 failed / 1014 passed / 1 skipped (Total: 1407)

### Recent Improvements
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

| ID | Test File | Tests (F/P) | Features | Status | Priority |
|----|-----------|-------------|----------|--------|----------|
| 1 | client/test_auth.py | 13/66 | Basic/Digest auth, custom auth | 🟡 Partial | P0 |
| 2 | models/test_responses.py | 60/46 | Response streaming, encoding | 🟡 Partial | P0 |
| 3 | models/test_url.py | 48/42 | RFC3986 compliance, IDNA | 🔴 Failing | P0 |
| 4 | test_content.py | 18/25 | Stream markers, async iterators | 🟡 Partial | P0 |
| 5 | client/test_proxies.py | 35/34 | Proxy env vars | 🟡 Partial | P1 |
| 6 | client/test_redirects.py | 30/1 | history, next_request | 🔴 Failing | P1 |
| 7 | client/test_async_client.py | 20/32 | Async streaming, build_request | 🟡 Partial | P1 |
| 8 | test_decoders.py | 26/14 | gzip/brotli/zstd/deflate | 🔴 Failing | P1 |
| 9 | test_asgi.py | 24/0 | ASGITransport | 🔴 Failing | P2 |
| 10 | client/test_client.py | 14/21 | build_request, transport | 🟡 Partial | P1 |
| 11 | client/test_headers.py | 15/2 | Header encoding | 🔴 Failing | P1 |
| 12 | models/test_headers.py | 2/25 | parse_header_links | 🟢 Mostly | P1 |
| 13 | test_multipart.py | 15/23 | Key/value validation | 🟡 Partial | P1 |
| 14 | test_utils.py | 14/26 | guess_json_utf, BOM | 🟡 Partial | P2 |
| 15 | models/test_queryparams.py | 0/14 | set(), add(), remove() | ✅ Done | - |
| 16 | models/test_requests.py | 15/9 | Request.stream, pickle | 🟡 Partial | P1 |
| 17 | test_config.py | 1/27 | create_ssl_context | 🟢 Mostly | P0 |
| 18 | test_auth.py | 4/4 | Auth module exports | 🟡 Partial | P1 |
| 19 | test_timeouts.py | 8/2 | Timeout edge cases | 🟡 Partial | P2 |
| 20 | client/test_event_hooks.py | 6/3 | Hooks on redirects | 🟡 Partial | P2 |
| 21 | client/test_cookies.py | 6/1 | Cookie persistence | 🔴 Failing | P2 |
| 22 | models/test_cookies.py | 4/3 | Domain/path support | 🟡 Partial | P2 |
| 23 | client/test_queryparams.py | 3/0 | Client query params | 🔴 Failing | P2 |
| 24 | test_api.py | 2/10 | Iterator content | 🟢 Mostly | P1 |
| 25 | test_exceptions.py | 1/2 | Exception hierarchy | 🟡 Partial | P2 |
| 26 | client/test_properties.py | 0/8 | Client properties | ✅ Done | - |
| 27 | models/test_whatwg.py | 0/563 | WHATWG URL parsing | ✅ Done | - |
| 28 | test_exported_members.py | 0/1 | Module exports | ✅ Done | - |
| 29 | test_status_codes.py | 0/6 | Status codes | ✅ Done | - |
| 30 | test_wsgi.py | 0/12 | WSGI transport | ✅ Done | - |

### Known Issues (Priority Order)
1. **Header case preservation**: Headers are lowercased, tests expect original case
2. **URL scheme handling**: Empty scheme URLs (e.g., "://example.com") not fully supported
3. **Digest auth**: Full RFC 2069/7616 implementation needed
4. **Redirect handling**: Need manual redirect handling for history tracking
5. **UTF-16/32 encoding**: JSON decoding for non-UTF-8 encodings
