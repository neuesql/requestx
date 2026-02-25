# RequestX: Refactoring with reqwest

`reqwest` is built on top of `hyper` and `tokio`, so you get all the performance benefits of Rust-based HTTP handling but with a much more ergonomic API.

## Architecture Overview

```

(reqwest):
  PyO3 ← reqwest (wraps hyper + connection pool + TLS + cookies + redirects)
```

`reqwest` already handles connection pooling, redirects, cookies, TLS, and timeouts — so you can delete a lot of manual code.

---

## 1. Core Client Wrapper

```rust
// src/client.rs
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyBytes, PyList};
use reqwest::{Client, ClientBuilder, Method, header};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

#[pyclass]
pub struct RustClient {
    client: Arc<Client>,
    runtime: Arc<Runtime>,
}

#[pymethods]
impl RustClient {
    #[new]
    #[pyo3(signature = (
        max_connections = 100,
        max_connections_per_host = 10,
        timeout = 30.0,
        follow_redirects = true,
        max_redirects = 10,
        verify_ssl = true,
        http2 = false,
        proxy = None,
        user_agent = None,
    ))]
    fn new(
        max_connections: usize,
        max_connections_per_host: usize,
        timeout: f64,
        follow_redirects: bool,
        max_redirects: usize,
        verify_ssl: bool,
        http2: bool,
        proxy: Option<&str>,
        user_agent: Option<&str>,
    ) -> PyResult<Self> {
        let mut builder = ClientBuilder::new()
            .pool_max_idle_per_host(max_connections_per_host)
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(Duration::from_secs_f64(timeout))
            .danger_accept_invalid_certs(!verify_ssl);

        if follow_redirects {
            builder = builder.redirect(reqwest::redirect::Policy::limited(max_redirects));
        } else {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }

        if http2 {
            builder = builder.http2_prior_knowledge();
        }

        if let Some(p) = proxy {
            let proxy = reqwest::Proxy::all(p)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
            builder = builder.proxy(proxy);
        }

        if let Some(ua) = user_agent {
            builder = builder.user_agent(ua);
        }

        let client = builder
            .build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self {
            client: Arc::new(client),
            runtime: Arc::new(runtime),
        })
    }
}
```

**What you removed:** Manual `hyper::Client`, manual `HttpConnector`, manual TLS setup, manual connection pool struct — `reqwest` handles all of it.

---

## 2. Request Execution — GIL-free

```rust
// src/request.rs
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyBytes};
use reqwest::{Method, header::HeaderMap, header::HeaderName, header::HeaderValue};
use std::str::FromStr;
use std::sync::Arc;
use bytes::Bytes;

/// Intermediate result that lives in Rust (no Python objects)
struct RawResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Bytes,
    url: String,
}

#[pymethods]
impl super::client::RustClient {
    /// Main request method — releases GIL for the entire HTTP lifecycle
    fn request<'py>(
        &self,
        py: Python<'py>,
        method: &str,
        url: &str,
        headers: Option<Vec<(&str, &str)>>,
        body: Option<&[u8]>,
        params: Option<Vec<(&str, &str)>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let method = Method::from_str(method)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let url = url.to_string();
        let headers_owned: Option<Vec<(String, String)>> = headers.map(|h| {
            h.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
        });
        let body_owned: Option<Bytes> = body.map(|b| Bytes::copy_from_slice(b));
        let params_owned: Option<Vec<(String, String)>> = params.map(|p| {
            p.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
        });

        // ⚡ Everything after this point runs WITHOUT the GIL
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut req = client.request(method, &url);

            // Set query params
            if let Some(p) = params_owned {
                req = req.query(&p);
            }

            // Set headers
            if let Some(h) = headers_owned {
                let mut header_map = HeaderMap::new();
                for (k, v) in h {
                    let name = HeaderName::from_str(&k).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
                    })?;
                    let value = HeaderValue::from_str(&v).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
                    })?;
                    header_map.insert(name, value);
                }
                req = req.headers(header_map);
            }

            // Set body
            if let Some(b) = body_owned {
                req = req.body(b);
            }

            // 🚀 Send request — connection pool, DNS, TLS, HTTP parse all in Rust
            let response = req.send().await.map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string())
            })?;

            let status = response.status().as_u16();
            let url = response.url().to_string();
            let headers: Vec<(String, String)> = response.headers().iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            // Read body — streaming happens in Rust, zero-copy with Bytes
            let body = response.bytes().await.map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string())
            })?;

            // Return to Python — GIL re-acquired here automatically
            Ok(RawResponse { status, headers, body, url })
        })
    }
}
```

---

## 3. Response Object — Lazy, Minimal Copies

```rust
// src/response.rs
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use bytes::Bytes;

#[pyclass]
pub struct RustResponse {
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub url: String,
    headers: Vec<(String, String)>,
    body: Bytes,  // Reference-counted, no copy on clone
}

#[pymethods]
impl RustResponse {
    /// Headers as Python dict — created on demand
    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.headers {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Raw bytes — single copy into Python bytes object
    #[getter]
    fn content<'py>(&self, py: Python<'py>) -> &Bound<'py, PyBytes> {
        PyBytes::new(py, &self.body)
    }

    /// Decode text in Rust (faster than Python .decode())
    #[getter]
    fn text(&self) -> PyResult<String> {
        match std::str::from_utf8(&self.body) {
            Ok(s) => Ok(s.to_string()),
            Err(_) => Ok(String::from_utf8_lossy(&self.body).to_string()),
        }
    }

    /// Parse JSON in Rust using serde_json (~3x faster than Python json.loads)
    fn json(&self, py: Python) -> PyResult<PyObject> {
        let value: serde_json::Value = serde_json::from_slice(&self.body)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        serde_json_value_to_py(py, &value)
    }

    fn __repr__(&self) -> String {
        format!("<Response [{}]>", self.status_code)
    }

    fn __bool__(&self) -> bool {
        self.status_code >= 200 && self.status_code < 400
    }
}

/// Convert serde_json::Value to Python objects
fn serde_json_value_to_py(py: Python, value: &serde_json::Value) -> PyResult<PyObject> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.into_pyobject(py)?.into()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into())
            } else {
                Ok(n.as_f64().unwrap().into_pyobject(py)?.into())
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into()),
        serde_json::Value::Array(arr) => {
            let list = pyo3::types::PyList::new(
                py,
                arr.iter()
                    .map(|v| serde_json_value_to_py(py, v))
                    .collect::<PyResult<Vec<_>>>()?,
            )?;
            Ok(list.into())
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, serde_json_value_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}
```

---

## 4. Streaming Response — For LLM APIs

This is critical for AI use cases (SSE streams from OpenAI, Anthropic, etc.):

```rust
// src/stream.rs
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use bytes::Bytes;
use tokio::sync::mpsc;

#[pyclass]
pub struct RustResponseStream {
    status_code: u16,
    headers: Vec<(String, String)>,
    receiver: Option<mpsc::Receiver<Result<Bytes, String>>>,
}

impl super::client::RustClient {
    /// Streaming request — returns headers immediately, body streams lazily
    fn stream<'py>(
        &self,
        py: Python<'py>,
        method: &str,
        url: &str,
        headers: Option<Vec<(&str, &str)>>,
        body: Option<&[u8]>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        // ... (same setup as request())

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let response = client.get(&url).send().await.map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string())
            })?;

            let status_code = response.status().as_u16();
            let headers: Vec<(String, String)> = response.headers().iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            // Spawn a Tokio task to stream body chunks into a channel
            let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(32);

            tokio::spawn(async move {
                let mut stream = response.bytes_stream();
                use futures_util::StreamExt;
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            if tx.send(Ok(bytes)).await.is_err() {
                                break; // Python side dropped the stream
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e.to_string())).await;
                            break;
                        }
                    }
                }
            });

            Ok(RustResponseStream {
                status_code,
                headers,
                receiver: Some(rx),
            })
        })
    }
}

#[pymethods]
impl RustResponseStream {
    #[getter]
    fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Python: `async for chunk in stream:`
    fn __aiter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        let rx = self.receiver.as_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyStopAsyncIteration, _>("")
        })?;

        let fut = async move {
            match rx.recv().await {
                Some(Ok(bytes)) => Ok(Some(bytes)),
                Some(Err(e)) => Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(e)),
                None => Ok(None),  // Stream complete
            }
        };

        // This releases GIL while waiting for next chunk
        Ok(Some(pyo3_async_runtimes::tokio::future_into_py(py, async move {
            match fut.await? {
                Some(bytes) => Python::with_gil(|py| {
                    Ok(PyBytes::new(py, &bytes).into())
                }),
                None => Err(PyErr::new::<pyo3::exceptions::PyStopAsyncIteration, _>("")),
            }
        })?))
    }
}
```

---

## 5. Python Wrapper — Stays Thin

```python
# python/requestx/_client.py
from ._rust import RustClient, RustResponse, RustResponseStream


class AsyncClient:
    """httpx-compatible async client powered by Rust/reqwest."""

    def __init__(self, **kwargs):
        self._inner = RustClient(**kwargs)

    async def request(self, method, url, **kwargs):
        raw = await self._inner.request(
            method=method, url=str(url),
            headers=list(kwargs.get("headers", {}).items())
                if kwargs.get("headers") else None,
            body=kwargs.get("content"),
            params=list(kwargs.get("params", {}).items())
                if kwargs.get("params") else None,
        )
        return Response(raw)

    async def stream(self, method, url, **kwargs):
        raw_stream = await self._inner.stream(
            method=method, url=str(url), **kwargs
        )
        return StreamResponse(raw_stream)

    async def get(self, url, **kwargs):
        return await self.request("GET", url, **kwargs)

    async def post(self, url, **kwargs):
        return await self.request("POST", url, **kwargs)

    async def put(self, url, **kwargs):
        return await self.request("PUT", url, **kwargs)

    async def delete(self, url, **kwargs):
        return await self.request("DELETE", url, **kwargs)

    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        pass  # reqwest::Client handles cleanup via Rust Drop


class Response:
    """httpx.Response-compatible wrapper."""
    __slots__ = ("_raw",)

    def __init__(self, raw: RustResponse):
        self._raw = raw

    @property
    def status_code(self):
        return self._raw.status_code

    @property
    def headers(self):
        return self._raw.headers

    @property
    def text(self):
        return self._raw.text

    @property
    def content(self):
        return self._raw.content

    def json(self):
        return self._raw.json()

    @property
    def url(self):
        return self._raw.url

    def raise_for_status(self):
        if self.status_code >= 400:
            raise HTTPStatusError(self.status_code, response=self)

    def __repr__(self):
        return f"<Response [{self.status_code}]>"

    def __bool__(self):
        return 200 <= self.status_code < 400


class StreamResponse:
    """Async iterator for streaming responses."""

    def __init__(self, raw_stream: RustResponseStream):
        self._raw = raw_stream

    @property
    def status_code(self):
        return self._raw.status_code

    async def __aiter__(self):
        async for chunk in self._raw:
            yield chunk

    async def aiter_lines(self):
        """For SSE/LLM streaming — split chunks on newlines."""
        buffer = b""
        async for chunk in self._raw:
            buffer += chunk
            while b"\n" in buffer:
                line, buffer = buffer.split(b"\n", 1)
                yield line.decode("utf-8")

    async def aiter_text(self):
        async for chunk in self._raw:
            yield chunk.decode("utf-8")
```

---

## 6. Cargo.toml

```toml
[package]
name = "requestx"
edition = "2021"

[lib]
name = "_rust"
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.22", features = ["extension-module"] }
pyo3-async-runtimes = { version = "0.22", features = ["tokio-runtime"] }
reqwest = { version = "0.12", features = [
    "json",
    "cookies",
    "gzip",
    "brotli",
    "zstd",
    "deflate",
    "stream",
    "rustls-tls",    # Use rustls instead of OpenSSL (easier cross-compile)
    "http2",
    "socks",
] }
tokio = { version = "1", features = ["full"] }
bytes = "1"
serde_json = "1"
futures-util = "0.3"
```

---

## Key Refactoring Wins with reqwest

| What you had to build manually with hyper | What reqwest gives you for free |
|---|---|
| Connection pool + idle timeout | ✅ Built-in `pool_max_idle_per_host`, `pool_idle_timeout` |
| TLS connector setup | ✅ `rustls-tls` or `native-tls` feature flag |
| Redirect following | ✅ `redirect::Policy` |
| Cookie jar | ✅ `cookie_store(true)` |
| Gzip/Brotli/Zstd decompression | ✅ Feature flags |
| Proxy support | ✅ `Proxy::all()`, `Proxy::http()` |
| Timeout handling | ✅ `timeout()`, `connect_timeout()` |
| HTTP/2 | ✅ `http2_prior_knowledge()` or ALPN negotiation |
| Streaming body | ✅ `bytes_stream()` |

You go from ~2000 lines of manual hyper plumbing to ~500 lines of reqwest + PyO3 glue, with the same (or better) performance since reqwest uses hyper under the hood anyway.
