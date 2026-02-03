```markdown
# RequestX Event Hooks Implementation

## Overview

A Rust/PyO3 implementation providing httpx-compatible event hooks for sync and async HTTP clients.

## Core Components

### 1. Request Model (`models.rs`)

```rust
#[pyclass]
#[derive(Clone)]
pub struct Request {
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub url: String,
    headers: HashMap<String, String>,
    content: Option<Vec<u8>>,
}
```

### 2. Response Model (`models.rs`)

```rust
#[pyclass]
#[derive(Clone)]
pub struct Response {
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub request: Request,  // httpx-style: response.request
    headers: HashMap<String, String>,
    content: Option<Vec<u8>>,
}
```

### 3. Hook System (`hooks.rs`)

```rust
pub struct Hook {
    callback: PyObject,
    is_async: bool,  // Auto-detected via inspect.iscoroutinefunction
}

pub struct EventHooks {
    pub request: Vec<Hook>,
    pub response: Vec<Hook>,
}

impl EventHooks {
    // Parse from Python dict: {'request': [...], 'response': [...]}
    pub fn from_py_dict(py: Python<'_>, dict: Option<&Bound<'_, PyDict>>) -> PyResult<Self>;
}
```

### 4. Client API (`client.rs`)

```rust
#[pyclass]
pub struct Client {
    inner: ReqwestClient,
    hooks: EventHooks,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (*, event_hooks=None, timeout=None))]
    pub fn new(py: Python<'_>, event_hooks: Option<&Bound<'_, PyDict>>, timeout: Option<f64>) -> PyResult<Self>;
    
    pub fn get(&self, py: Python<'_>, url: String, ...) -> PyResult<Response>;
    pub fn post(&self, py: Python<'_>, url: String, ...) -> PyResult<Response>;
    // + put, delete, request methods
}

#[pyclass]
pub struct AsyncClient { /* similar structure */ }
```

## Request Flow

```
1. Build Request object
2. Execute request hooks: for hook in hooks.request { hook(request) }
3. Send HTTP request via reqwest
4. Build Response with embedded Request
5. Execute response hooks: for hook in hooks.response { hook(response) }
6. Return Response
```

## Python Usage

```python
import requestx

def log_request(request):
    print(f"Request: {request.method} {request.url}")

def log_response(response):
    print(f"Response: {response.request.method} {response.request.url} -> {response.status_code}")

# Sync client
client = requestx.Client(event_hooks={'request': [log_request], 'response': [log_response]})
response = client.get("https://httpbin.org/get")

# Async client
async def main():
    async with requestx.AsyncClient(event_hooks={'request': [log_request]}) as client:
        response = await client.get("https://httpbin.org/get")
```

## Dependencies (Cargo.toml)

```toml
[dependencies]
pyo3 = { version = "0.21", features = ["extension-module"] }
pyo3-asyncio = { version = "0.21", features = ["tokio-runtime"] }
reqwest = { version = "0.12", features = ["json", "cookies"] }
tokio = { version = "1", features = ["full"] }
```

## Key Features

| Feature | Support |
|---------|---------|
| `event_hooks={'request': [], 'response': []}` | ✅ |
| `response.request` access | ✅ |
| Sync + async hooks auto-detection | ✅ |
| Multiple hooks per event | ✅ |
| Context manager (`with`/`async with`) | ✅ |
```