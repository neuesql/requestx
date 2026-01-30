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

## Test Status: 527 failed / 880 passed / 1 skipped (Total: 1407)

| ID | Test File | Tests (F/T) | Features | Dependencies | Status | Priority |
|----|-----------|-------------|----------|--------------|--------|----------|
| 1 | client/test_auth.py | 77/79 | Basic/Digest auth, custom auth callables | MockTransport | 🔴 Failing | P0 |
| 2 | models/test_responses.py | 64/106 | Response streaming, encoding, links | Response model | 🔴 Failing | P0 |
| 3 | models/test_url.py | 48/90 | RFC3986 compliance, percent encoding, IDNA | URL model | 🔴 Failing | P0 |
| 4 | test_content.py | 42/43 | Stream markers, async iterators, multipart | Content handling | 🔴 Failing | P0 |
| 5 | client/test_proxies.py | 35/69 | Proxy env vars (HTTP_PROXY, NO_PROXY) | Transport | 🟡 Partial | P1 |
| 6 | client/test_redirects.py | 30/31 | history, next_request, cross-domain auth | Response | 🔴 Failing | P1 |
| 7 | client/test_async_client.py | 28/52 | Async streaming, build_request | AsyncClient | 🟡 Partial | P1 |
| 8 | test_decoders.py | 26/40 | gzip/brotli/zstd/deflate decoders | Decoders | 🔴 Failing | P1 |
| 9 | test_asgi.py | 24/24 | ASGITransport, app lifecycle | Transport | 🔴 Failing | P2 |
| 10 | client/test_client.py | 18/35 | build_request, transport management | Client | 🟡 Partial | P1 |
| 11 | client/test_headers.py | 15/17 | Header encoding, sensitive masking | Headers | 🔴 Failing | P1 |
| 12 | models/test_headers.py | 15/27 | parse_header_links, encoding | Headers | 🔴 Failing | P1 |
| 13 | test_multipart.py | 15/38 | Key/value validation, HTML5 escaping | Multipart | 🟡 Partial | P1 |
| 14 | test_utils.py | 14/40 | guess_json_utf, BOM detection | Utils | 🟡 Partial | P2 |
| 15 | models/test_queryparams.py | 13/14 | set(), add(), remove(), __hash__ | QueryParams | 🔴 Failing | P1 |
| 16 | models/test_requests.py | 13/24 | Request.stream, pickle support | Request | 🟡 Partial | P1 |
| 17 | test_config.py | 12/28 | create_ssl_context, verify, cert | SSL | 🟡 Partial | P0 |
| 18 | test_auth.py | 8/8 | Auth module exports | Auth | 🔴 Failing | P1 |
| 19 | test_timeouts.py | 8/10 | Timeout edge cases | Timeout | 🟡 Partial | P2 |
| 20 | client/test_event_hooks.py | 6/9 | Hooks on redirects | Hooks | 🟡 Partial | P2 |
| 21 | client/test_cookies.py | 6/7 | Cookie persistence | Cookies | 🔴 Failing | P2 |
| 22 | models/test_cookies.py | 4/7 | Domain/path support | Cookies | 🟡 Partial | P2 |
| 23 | client/test_queryparams.py | 3/3 | Client query params | QueryParams | 🔴 Failing | P2 |
| 24 | test_api.py | 2/12 | Iterator content in post/put | API | 🟡 Partial | P1 |
| 25 | test_exceptions.py | 1/3 | Exception hierarchy | Exceptions | 🟡 Partial | P2 |
| 26 | client/test_properties.py | 0/8 | Client properties | Client | ✅ Done | - |
| 27 | models/test_whatwg.py | 0/563 | WHATWG URL parsing | URL | ✅ Done | - |
| 28 | test_exported_members.py | 0/1 | Module exports | Exports | ✅ Done | - |
| 29 | test_status_codes.py | 0/6 | Status codes | Status | ✅ Done | - |
| 30 | test_wsgi.py | 0/12 | WSGI transport | Transport | ✅ Done | - |
