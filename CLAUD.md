# Project: RequestX - High-Performance Python HTTP Client

## Objective
Build a high-performance Python HTTP client that is fully API-compatible with httpx, powered by Rust's reqwest library via PyO3 bindings.

## Architecture Requirements

### Core Principles
1. **Rust-First Implementation**: ALL business logic must be implemented in Rust
2. **Minimal Python Layer**: `python/requestx/__init__.py` should ONLY contain:
   - Type exports from Rust
   - Class exports from Rust
   - No Python business logic
3. **Performance Priority**: Optimize PyO3 bridge for minimal overhead

### Technology Stack
- **HTTP Engine**: Rust `reqwest` crate
- **Python Bindings**: PyO3 (use `Python::attach()` API, not deprecated `with_gil()`)
- **Target API**: httpx-compatible (excluding `httpx.__main__` and CLI features)

## Reference Materials

### Source Code to Understand
1. **httpx source**: https://github.com/encode/httpx/tree/master/httpx
   - Study: Client, AsyncClient, Request, Response, URL, Headers, Cookies, Timeout, Limits
   - Ignore: `__main__.py`, CLI-related code

2. **Current project structure**:
   - `python/requestx/__init__.py` - Clean this file, export Rust types only
   - `src/` - Rust implementation (reqwest + PyO3)
   - `test_httpx/` - Reference tests (100% working, do not modify)
   - `test_requestx/` - Target tests (must all pass)

## Implementation Tasks

### Phase 1: Clean Python Layer
```python
# python/requestx/__init__.py - TARGET STATE
# Only exports, no logic

from .requestx import (
    # Classes
    Client,
    AsyncClient,
    Request,
    Response,
    # Types
    URL,
    Headers,
    Cookies,
    QueryParams,
    Timeout,
    Limits,
    # Exceptions
    HTTPError,
    RequestError,
    TimeoutException,
    # Functions
    get,
    post,
    put,
    patch,
    delete,
    head,
    options,
    request,
)

__all__ = [...]
__version__ = "..."
```

### Phase 2: Rust Implementation Checklist
Implement in Rust (`src/lib.rs` or modular structure):

- [ ] `Client` - Sync HTTP client
- [ ] `AsyncClient` - Async HTTP client
- [ ] `Request` - HTTP request object
- [ ] `Response` - HTTP response object
- [ ] `URL` - URL parsing and manipulation
- [ ] `Headers` - HTTP headers (dict-like interface)
- [ ] `Cookies` - Cookie jar
- [ ] `QueryParams` - Query string parameters
- [ ] `Timeout` - Timeout configuration
- [ ] `Limits` - Connection limits
- [ ] Top-level functions: `get()`, `post()`, `put()`, `patch()`, `delete()`, `head()`, `options()`, `request()`
- [ ] Exception hierarchy matching httpx

### Phase 3: PyO3 Performance Considerations
```rust
// Use these patterns for performance:

// 1. Release GIL during blocking I/O
fn sync_request(py: Python<'_>, ...) -> PyResult<Response> {
    py.allow_threads(|| {
        // reqwest blocking call here
    })
}

// 2. For async, use pyo3-asyncio or manual future handling
#[pyo3(signature = (...))]
fn async_request<'py>(py: Python<'py>, ...) -> PyResult<Bound<'py, PyAny>> {
    pyo3_asyncio::tokio::future_into_py(py, async move {
        // reqwest async call here
    })
}

// 3. Efficient type conversions - avoid unnecessary copies
// 4. Use Cow<str> where possible
// 5. Implement __repr__, __str__, __eq__ for Python compatibility
```

## Testing Strategy

### Test Execution Order
1. First, verify reference tests work:
```bash
   pytest test_httpx/ -v  # Must be 100% passing
```

2. Then run target tests iteratively:
```bash
   pytest test_requestx/ -v --tb=short
```

3. Compare behavior:
```bash
   # Run both to ensure compatibility
   pytest test_httpx/ test_requestx/ -v
```

### Success Criteria
- [ ] ALL tests in `test_requestx/` pass
- [ ] API is drop-in compatible with httpx (import requestx as httpx should work)
- [ ] No Python business logic in `__init__.py`
- [ ] Performance equal or better than httpx

## Constraints
- Do NOT implement: `httpx.__main__`, CLI features, `httpx.main()`
- Do NOT modify: `test_httpx/` folder
- MUST use: Rust reqwest for all HTTP operations
- MUST use: PyO3 `Python::attach()` (not deprecated `with_gil()`)

## Completion Definition
Task is DONE when:
```bash
pytest test_requestx/ -v
# Result: ALL PASSED
```

## Reference Implementation
- https://github.com/MarkusSintonen/pyreqwest
