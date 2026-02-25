# RequestX

High-performance Python HTTP client, API-compatible with httpx, powered by Rust's reqwest via PyO3.

## Features

- **httpx API compatibility** — Drop-in replacement: `import requestx as httpx` works
- **AI SDK compatible** — Works with OpenAI, Anthropic SDKs via `http_client=requestx.Client()`
- **High performance** — Rust-powered with GIL-free I/O, SIMD JSON (sonic-rs), zero-copy bytes
- **Full async support** — Tokio runtime for true concurrent multiplexing
- **Standards compliant** — WHATWG URL, RFC 2388 (multipart), RFC 7616 (digest auth), HTTP/2

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

## Architecture

### Rust-First Design (12,021 LOC across 18 modules)

All business logic lives in Rust. The Python layer contains only thin wrappers for auth protocol, exception conversion, and re-exports.

```
src/                           # Rust implementation (ALL business logic)
├── lib.rs             (121)   # PyModule definition & exports
├── response.rs       (1866)   # Response handling, 8 iterator types (sync/async)
├── url.rs            (1618)   # WHATWG-compliant URL parser
├── client.rs         (1228)   # Sync HTTP client with event hooks
├── async_client.rs   (1139)   # Async client, Tokio runtime
├── request.rs         (936)   # Request building, MutableHeaders
├── transport.rs       (706)   # Mock, HTTP, WSGI transports
├── cookies.rs         (672)   # Domain/path-aware cookie jar
├── headers.rs         (627)   # Case-preserving, encoding-aware headers
├── types.rs           (626)   # Auth types, status codes
├── common.rs          (488)   # JSON (sonic-rs), decompression, utilities
├── timeout.rs         (409)   # Timeout, Limits, Proxy configuration
├── multipart.rs       (387)   # RFC 2388 multipart encoding
├── queryparams.rs     (338)   # Query string parser & builder
├── client_common.rs   (252)   # Shared auth, headers, cookies merging
├── api.rs             (237)   # Top-level module functions
├── auth.rs            (208)   # DigestAuth (RFC 2069/7616)
└── exceptions.rs      (163)   # httpx-compatible exception hierarchy

python/requestx/               # Thin Python wrappers (re-exports only)
├── __init__.py                # 67 public symbols, drop-in for httpx
├── _client.py                 # Sync Client wrapper (auth, mounts, proxy)
├── _async_client.py           # Async Client wrapper
├── _request.py                # Request wrapper (_WrappedRequest for auth)
├── _response.py               # Response wrapper with .stream property
├── _client_common.py          # Shared proxy/transport utilities
├── _api.py                    # Top-level get/post/put/patch/delete/head/options
├── _auth.py                   # BasicAuth, DigestAuth, NetRCAuth, FunctionAuth
├── _transports.py             # BaseTransport, MockTransport, ASGITransport
├── _compat.py                 # Sentinels, SSL context, codes wrapper
├── _exceptions.py             # Exception hierarchy with request attribute
├── _streams.py                # ByteStream adapters, streaming wrappers
└── _utils.py                  # Utility functions

tests_httpx/                   # Reference tests — DO NOT MODIFY (30 files)
tests_requestx/                # Target tests — must all pass (30 files)
tests_performance/             # Benchmarks (3 files)
```

### Rust Exports: 65 types, 17 functions

**Core types:** Client, AsyncClient, Request, Response, URL, Headers, QueryParams, Cookies, Timeout, Limits, Proxy

**Auth:** Auth, BasicAuth, DigestAuth, NetRCAuth, FunctionAuth

**Streaming (8 iterator types):** BytesIterator, TextIterator, LinesIterator, RawIterator + async variants

**Transports:** MockTransport, AsyncMockTransport, HTTPTransport, AsyncHTTPTransport, WSGITransport

**Exceptions (20+):** Full httpx exception hierarchy — HTTPError, TimeoutException, ConnectTimeout, ReadTimeout, WriteTimeout, PoolTimeout, ConnectError, TooManyRedirects, StreamConsumed, etc.

### Performance Architecture

- **GIL-free I/O**: All network operations release the GIL via `py.allow_threads()` — enables true parallelism
- **Tokio async runtime**: Async requests multiplex entirely outside Python's GIL
- **sonic-rs JSON**: SIMD-accelerated parsing/serialization (gains scale with payload size)
- **Zero-copy bytes**: `PyBytes` for response content, reference-returning getters
- **Freelist caching**: Headers (256), Cookies (64), URL (128) — avoids repeated allocation
- **Rust-native decompression**: gzip/brotli/deflate/zstd via flate2, brotli, zstd crates
- **Connection pooling**: reqwest-level pool with HTTP/2 multiplexing via rustls
- **Pre-allocation**: `Vec::with_capacity()` when sizes are known

## Core Dependencies (Cargo.toml)
```toml
[dependencies]
pyo3 = { version = "0.28", features = ["extension-module"] }
pyo3-async-runtimes = { version = "0.28", features = ["tokio-runtime"] }
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
// ✅ sonic-rs (SIMD-accelerated)
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

### 7. SDK Compatibility
- requestx patches `type.__instancecheck__` at import to pass httpx.Client isinstance checks
- This enables AI SDK compatibility (OpenAI, Anthropic accept requestx.Client)
- Patch is global but detection is narrow (class + module name matching)

## Don't

- ❌ Modify `tests_httpx/` (reference tests)
- ❌ Put business logic in Python
- ❌ Use `panic!` (crashes Python)
- ❌ Convert types inside loops (convert once at boundary)
- ❌ Use deprecated `Python::with_gil()`

## API Compatibility

98.5% coverage of httpx public API (65/66 symbols). Only `main` (CLI entry point) is excluded by design.

Drop-in replacement: `import requestx as httpx` works.

### Standards Compliance
- WHATWG URL parsing
- RFC 2388 (multipart)
- RFC 2069/7616 (digest auth)
- HTTP/2 support

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

## Test Status: 0 failed / 1406 passed / 1 skipped (Total: 1407)

All 30 httpx compatibility test files pass. Key coverage areas:

| Area | Tests | Features |
|------|-------|----------|
| Auth | 79+ | Basic, Digest (RFC 7616), NetRC, custom callables, streaming body |
| Async Client | 52+ | ResponseNotRead, async iterators, http_version, MockTransport |
| URL | 90+ | WHATWG parsing, percent-encoding, fragment decoding, validation |
| Redirects | 31 | Malformed URLs, streaming body, cookie persistence |
| Responses | 106+ | Pickling, streaming, content decoding |
| Headers | 27+ | Case preservation, encoding-aware, repr |
| Content | 43+ | BytesIO, sync/async iterators, stream mode detection |
| Timeouts | 10+ | Pool, connect, read, write timeout classification |
| Decoders | — | gzip, brotli, deflate, zstd |
| Transports | — | Mock, HTTP, WSGI, ASGI |
| Cookies | — | Domain/path, jar persistence, conflict handling |
