# CLAUDE.md - Requestx Project Guide

## Project Overview

Requestx is a high-performance Python HTTP client built on Rust's [reqwest](https://docs.rs/reqwest/) library, using [PyO3](https://pyo3.rs/) for Python bindings. The API is designed to be compatible with [HTTPX](https://www.python-httpx.org/).

## Tech Stack

- **Rust Core**: HTTP client implementation using `reqwest` with `tokio` async runtime
- **Python Bindings**: PyO3 for seamless Rust-Python interop
- **Build System**: Maturin for building Python wheels from Rust
- **JSON**: sonic-rs for high-performance JSON serialization
- **TLS**: rustls for secure connections

## Project Structure

```
requestx/
├── src/                    # Rust source code
│   ├── lib.rs             # Module entry point, PyO3 module definition
│   ├── client.rs          # Client and AsyncClient implementations
│   ├── response.rs        # Response type with JSON/text parsing
│   ├── error.rs           # HTTPX-compatible exception hierarchy
│   ├── types.rs           # Headers, Cookies, Timeout, Proxy, Auth types
│   ├── request.rs         # Module-level convenience functions
│   └── streaming.rs       # Streaming response iterators
├── python/requestx/       # Python package
│   └── __init__.py        # Re-exports from _core Rust module
├── tests/                 # Python tests
│   ├── conftest.py        # Pytest configuration
│   ├── test_sync.py       # Synchronous API tests
│   └── test_async.py      # Asynchronous API tests
├── docs/                  # Sphinx documentation
├── Cargo.toml             # Rust dependencies
├── pyproject.toml         # Python project config (maturin)
└── Makefile               # Development commands
```

## Development Commands

Use numbered make commands for the development workflow:

```bash
make 1-setup           # Setup dev environment with uv
make 2-format          # Format Rust + Python code
make 2-format-check    # Check formatting without changes
make 3-lint            # Run linters (clippy + ruff)
make 4-quality-check   # Combined format check + lint
make 5-build           # Build Rust/Python extension (dev mode)
make 6-test-rust       # Run Rust tests
make 6-test-python     # Run Python tests (requires build)
make 6-test-all        # Run all tests
make 7-doc-build       # Build Sphinx documentation
make 9-clean           # Clean all build artifacts
```

## Building the Project

```bash
# First-time setup
make 1-setup

# Build in development mode
make 5-build
# or directly:
uv run maturin develop

# Build release wheel
maturin build --release
```

## Running Tests

```bash
# Run all tests
make 6-test-all

# Run only Python tests
make 6-test-python

# Run specific test file
uv run python -m unittest tests/test_sync.py -v
```

## Key Architecture Concepts

### Rust Module Structure

The Rust code in `src/lib.rs` registers all Python-visible types:
- **Client classes**: `Client`, `AsyncClient`
- **Response types**: `Response`, `StreamingResponse`, `AsyncStreamingResponse`
- **Configuration types**: `Headers`, `Cookies`, `Timeout`, `Proxy`, `Auth`, `Limits`, `SSLConfig`
- **Exception hierarchy**: HTTPX-compatible exceptions (e.g., `RequestError`, `TimeoutException`, `ConnectError`)
- **Module functions**: `get`, `post`, `put`, `patch`, `delete`, `head`, `options`, `request`

### Client Configuration (`src/client.rs`)

`ClientConfig` holds all client settings:
- `base_url`: Optional base URL for relative requests
- `headers`, `cookies`: Default headers/cookies
- `timeout`: Connection, read, write, pool timeouts
- `follow_redirects`, `max_redirects`: Redirect handling
- `verify_ssl`, `ca_bundle`, `cert_file`: TLS configuration
- `proxy`: HTTP/HTTPS/SOCKS proxy settings
- `auth`: Basic, Bearer, or Digest authentication
- `http2`: Enable HTTP/2 prior knowledge
- `trust_env`: Read proxy/SSL settings from environment

### Response Handling (`src/response.rs`)

The `Response` type provides:
- Status information: `status_code`, `reason_phrase`, `is_success`, `is_error`
- Content access: `content` (bytes), `text` (decoded), `json()` (parsed)
- Metadata: `headers`, `cookies`, `url`, `elapsed`, `http_version`
- Error handling: `raise_for_status()`

### Error Hierarchy (`src/error.rs`)

HTTPX-compatible exception types:
- `RequestError` (base)
  - `TransportError` -> `ConnectError`, `ReadError`, `WriteError`, `ProxyError`
  - `TimeoutException` -> `ConnectTimeout`, `ReadTimeout`, `WriteTimeout`, `PoolTimeout`
  - `HTTPStatusError`
  - `TooManyRedirects`
  - `DecodingError`
  - `InvalidURL`

## Python API Usage

### Synchronous API

```python
import requestx

# Simple request
response = requestx.get("https://api.example.com/data")
print(response.json())

# With client (connection pooling)
with requestx.Client(base_url="https://api.example.com") as client:
    response = client.get("/users")
```

### Asynchronous API

```python
import asyncio
import requestx

async def main():
    async with requestx.AsyncClient() as client:
        response = await client.get("https://api.example.com/data")
        print(response.json())

asyncio.run(main())
```

### Streaming Responses

```python
# Sync streaming
with requestx.Client() as client:
    with client.stream("GET", url) as response:
        for chunk in response.iter_bytes(chunk_size=1024):
            process(chunk)

# Async streaming
async with requestx.AsyncClient() as client:
    async with await client.stream("GET", url) as response:
        async for chunk in response.aiter_bytes(chunk_size=1024):
            process(chunk)
```

## Dependencies

### Rust (Cargo.toml)
- `pyo3` (0.27): Python bindings
- `pyo3-async-runtimes`: Async runtime bridge
- `reqwest` (0.13): HTTP client with many features enabled
- `tokio` (1): Async runtime
- `sonic-rs` (0.5): Fast JSON
- `url` (2): URL parsing

### Python (pyproject.toml)
- Python 3.12+
- Dev: maturin, pytest, pytest-asyncio, httpx (for comparison), black, ruff, mypy

## Code Style

- Rust: `cargo fmt` for formatting, `cargo clippy` for linting
- Python: `black` for formatting, `ruff` for linting
- Run `make 4-quality-check` before committing

## Common Development Tasks

### Adding a New Client Option

1. Add field to `ClientConfig` in `src/client.rs`
2. Update `Client::new()` and `AsyncClient::new()` signatures
3. Apply the config in `build_reqwest_client()` / `build_blocking_client()`
4. Export from `python/requestx/__init__.py` if it's a new type
5. Add tests in `tests/test_sync.py` and `tests/test_async.py`

### Adding a New Exception Type

1. Define in `src/error.rs` using `create_exception!` macro
2. Add variant to `ErrorKind` enum
3. Add constructor method to `Error` impl
4. Map in `From<Error> for PyErr` impl
5. Register in `lib.rs` module init
6. Export from `python/requestx/__init__.py`

### Debugging

- Use `cargo test --verbose` for Rust-level debugging
- Build with `maturin develop` (not `--release`) for debug symbols
- Python exceptions preserve the Rust error chain

---

## HTTPX Compatibility Checkpoints (tests_requestx)

**Current Status**: 698 passed, 708 failed (49.6% passing)

### Test Failures by File

| Test File | Failures | Main Issues |
|-----------|----------|-------------|
| client/test_auth.py | 79 | transport param, DigestAuth.sync_auth_flow |
| models/test_responses.py | 74 | Response streaming methods, encoding |
| client/test_proxies.py | 68 | transport param, _transport_for_url |
| models/test_url.py | 67 | URL(params=), URL(scheme=), URL(path=) |
| client/test_async_client.py | 52 | transport param, async context manager |
| test_content.py | 42 | generator/async_generator content |
| test_decoders.py | 39 | encoding support (utf-16, utf-32) |
| test_multipart.py | 36 | multipart encoding |
| client/test_client.py | 33 | transport param, _redirect_headers |
| client/test_redirects.py | 31 | transport param |
| test_utils.py | 26 | environment proxies, logging |
| test_asgi.py | 20 | transport param |
| models/test_requests.py | 18 | Request(data=), Request(files=), Request(json=) |
| client/test_headers.py | 17 | Headers.encoding, bytes handling |
| models/test_headers.py | 16 | Headers.encoding |
| test_wsgi.py | 12 | transport param |
| test_api.py | 12 | transport param |
| models/test_queryparams.py | 9 | QueryParams ordering |
| client/test_event_hooks.py | 9 | event_hooks param |
| test_timeouts.py | 8 | pool timeout |
| test_auth.py | 8 | DigestAuth methods |
| client/test_properties.py | 8 | base_url writable |
| client/test_cookies.py | 7 | Cookies.set(domain=) |
| models/test_cookies.py | 6 | Cookies.set(domain=) |
| test_config.py | 4 | config issues |
| test_exceptions.py | 3 | exception handling |
| client/test_queryparams.py | 3 | QueryParams |
| test_exported_members.py | 1 | missing exports |

### Failures by Error Type (Priority Order)

| Priority | Error Type | Count | Root Cause | Feature to Implement |
|----------|------------|-------|------------|----------------------|
| **P0** | Client/AsyncClient transport param | 398 | Missing parameter | Add `transport` param (accept & ignore for now) |
| **P0** | HTTPTransport not exported | 59 | Missing class | Create MockTransport/HTTPTransport class |
| **P0** | _transport_for_url missing | 70 | Missing method | Add to Client/AsyncClient |
| **P1** | Request(data=) | 40 | Missing param | Add `data` param to Request |
| **P1** | Request(files=) | 26 | Missing param | Add `files` param to Request |
| **P1** | Request(json=) | 16 | Missing param | Add `json` param to Request |
| **P2** | URL(params=) | 12 | Missing param | Add `params` param to URL |
| **P2** | URL(scheme=) | 10 | Missing param | Add `scheme` param to URL |
| **P2** | URL(path=) | 6 | Missing param | Add `path` param to URL |
| **P3** | Response.stream | 16 | Missing attr | Add stream property |
| **P3** | Response.aiter_bytes | 12 | Missing method | Add async iteration methods |
| **P3** | Response.aiter_raw | 12 | Missing method | Add async iteration methods |
| **P3** | Response.iter_raw | 10 | Missing method | Add sync iteration methods |
| **P3** | Response.aiter_text | 8 | Missing method | Add async text iteration |
| **P3** | Response.iter_text | 6 | Missing method | Add sync text iteration |
| **P3** | Response.num_bytes_downloaded | 6 | Missing attr | Add tracking |
| **P4** | async context manager | 12 | Protocol issue | Fix async stream protocol |
| **P4** | generator content | 18 | Type handling | Accept generators |
| **P4** | async_generator content | 16 | Type handling | Accept async generators |
| **P5** | bytes cast as str | 68 | Encoding | Handle bytes in text operations |
| **P5** | Headers.encoding | 6 | Missing attr | Add encoding property |
| **P6** | Cookies.set(domain=) | 8 | Missing param | Add domain/path params |
| **P7** | DigestAuth.sync_auth_flow | 8 | Missing method | Add auth flow methods |
| **P8** | event_hooks param | 18 | Missing param | Add event_hooks to clients |
| **P9** | base_url writable | 6 | Read-only attr | Make base_url settable |
| **P10** | _redirect_headers | 10 | Missing method | Add redirect helper |

### Implementation Checkpoints

- [ ] **P0-1**: Add `transport` parameter to Client/AsyncClient (accept & store, can be None)
- [ ] **P0-2**: Create HTTPTransport and AsyncHTTPTransport stub classes
- [ ] **P0-3**: Add `_transport_for_url()` method to Client/AsyncClient
- [ ] **P1-1**: Add `data`, `files`, `json` parameters to Request
- [ ] **P2-1**: Add `params`, `scheme`, `path` parameters to URL constructor
- [ ] **P3-1**: Add Response streaming methods (iter_bytes, iter_text, iter_raw)
- [ ] **P3-2**: Add Response async streaming methods (aiter_bytes, aiter_text, aiter_raw)
- [ ] **P3-3**: Add Response.stream and num_bytes_downloaded properties
- [ ] **P4-1**: Fix async context manager protocol for streaming
- [ ] **P4-2**: Handle generator and async_generator content types
- [ ] **P5-1**: Add Headers.encoding property
- [ ] **P5-2**: Fix bytes handling in text operations
- [ ] **P6-1**: Add domain/path params to Cookies.set()
- [ ] **P7-1**: Add sync_auth_flow/async_auth_flow to DigestAuth
- [ ] **P8-1**: Add event_hooks parameter to Client/AsyncClient
- [ ] **P9-1**: Make base_url property writable
- [ ] **P10-1**: Add _redirect_headers method
