# RequestX - High-Performance Python HTTP Client

## Objective
Build an httpx-compatible Python HTTP client powered by Rust's reqwest via PyO3.

## Core Dependencies
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

## Architecture
- **Rust**: ALL business logic (reqwest + PyO3)
- **Python**: ONLY exports from Rust module
- **Reference**: https://github.com/encode/httpx/tree/master/httpx

---

## Iterative Development Strategy

### Iteration 1: Core Types (Foundation)
**Goal**: Pass `tests_requestx/models/` tests

| Component | Tests | Key Methods |
|-----------|-------|-------------|
| `URL` | `test_url.py` | `scheme`, `host`, `port`, `path`, `query`, `fragment`, `join()`, `copy_with()` |
| `Headers` | `test_headers.py` | `__getitem__`, `__setitem__`, `keys()`, `values()`, `items()`, `raw` |
| `QueryParams` | `test_queryparams.py` | `__getitem__`, `get()`, `keys()`, `values()`, `items()` |
| `Cookies` | `test_cookies.py` | `__getitem__`, `get()`, `set()`, `delete()` |

**Run**: `pytest tests_requestx/models/ -v`

---

### Iteration 2: Request & Response
**Goal**: Pass `tests_requestx/models/test_requests.py` and `test_responses.py`

| Component | Key Properties |
|-----------|----------------|
| `Request` | `method`, `url`, `headers`, `content`, `stream` |
| `Response` | `status_code`, `reason_phrase`, `headers`, `content`, `text`, `json()`, `raise_for_status()` |

**Run**: `pytest tests_requestx/models/test_requests.py tests_requestx/models/test_responses.py -v`

---

### Iteration 3: Sync Client
**Goal**: Pass `tests_requestx/client/test_client.py`

| Component | Key Methods |
|-----------|-------------|
| `Client` | `get()`, `post()`, `put()`, `patch()`, `delete()`, `head()`, `options()`, `request()`, `stream()`, `send()`, `build_request()` |

**Context Manager**: `__enter__`, `__exit__`

**Run**: `pytest tests_requestx/client/test_client.py -v`

---

### Iteration 4: Async Client
**Goal**: Pass `tests_requestx/client/test_async_client.py`

| Component | Key Methods |
|-----------|-------------|
| `AsyncClient` | Same as Client but async: `await client.get()`, etc. |

**Async Context Manager**: `__aenter__`, `__aexit__`

**Run**: `pytest tests_requestx/client/test_async_client.py -v`

---

### Iteration 5: Client Features
**Goal**: Pass remaining client tests

| Feature | Test File |
|---------|-----------|
| Headers | `test_headers.py` |
| Cookies | `test_cookies.py` |
| Auth | `test_auth.py` |
| Redirects | `test_redirects.py` |
| Proxies | `test_proxies.py` |
| Query Params | `test_queryparams.py` |
| Event Hooks | `test_event_hooks.py` |

**Run**: `pytest tests_requestx/client/ -v`

---

### Iteration 6: Top-Level API & Exceptions
**Goal**: Pass all remaining tests

| Component | Test File |
|-----------|-----------|
| `get()`, `post()`, etc. | `test_api.py` |
| `Timeout`, `Limits` | `test_timeouts.py`, `test_config.py` |
| Exception hierarchy | `test_exceptions.py` |
| Exports | `test_exported_members.py` |

**Run**: `pytest tests_requestx/ -v`

---

## Test Commands

```bash
# Reference tests (must pass - do not modify)
pytest tests_httpx/ -v

# Target tests by iteration
pytest tests_requestx/models/ -v                    # Iteration 1-2
pytest tests_requestx/client/test_client.py -v      # Iteration 3
pytest tests_requestx/client/test_async_client.py -v # Iteration 4
pytest tests_requestx/client/ -v                     # Iteration 5
pytest tests_requestx/ -v                            # Full suite

# Compare behavior
pytest tests_httpx/ tests_requestx/ -v
```

---

## PyO3 Rules (Quick Reference)

1. **Convert once at boundaries** - not in loops
2. **Release GIL** for I/O (`py.allow_threads`)
3. **Use sonic-rs** for JSON (not Python json)
4. **Return `&str`** instead of `String.clone()`
5. **Pre-allocate** with `Vec::with_capacity()`

---

## Completion
```bash
pytest tests_requestx/ -v
# Result: ALL PASSED
```
