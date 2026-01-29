//! HTTP Transport implementations including MockTransport

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use parking_lot::Mutex;

use crate::request::Request;
use crate::response::Response;

/// Base transport trait for HTTP requests
pub trait Transport: Send + Sync {
    fn handle_request(&self, request: &Request) -> PyResult<Response>;
}

/// Mock transport for testing - returns predefined responses
#[pyclass(name = "MockTransport", subclass)]
pub struct MockTransport {
    handler: Arc<Mutex<Option<Py<PyAny>>>>,
}

impl Default for MockTransport {
    fn default() -> Self {
        Self {
            handler: Arc::new(Mutex::new(None)),
        }
    }
}

#[pymethods]
impl MockTransport {
    #[new]
    #[pyo3(signature = (handler=None))]
    fn new(handler: Option<Py<PyAny>>) -> Self {
        Self {
            handler: Arc::new(Mutex::new(handler)),
        }
    }

    fn handle_request(&self, py: Python<'_>, request: &Request) -> PyResult<Response> {
        let handler = self.handler.lock();
        if let Some(ref h) = *handler {
            // Call the Python handler function
            let result = h.call1(py, (request.clone(),))?;

            // If it returns a Response, use it directly
            if let Ok(response) = result.extract::<Response>(py) {
                return Ok(response);
            }

            // If it's a callable that needs to be awaited (async), handle that
            // For now, we expect sync handlers
            Err(pyo3::exceptions::PyTypeError::new_err(
                "MockTransport handler must return a Response object",
            ))
        } else {
            // Return a default 200 response
            Ok(Response::new(200))
        }
    }

    fn __repr__(&self) -> String {
        "<MockTransport>".to_string()
    }
}

/// Async mock transport for testing async clients
#[pyclass(name = "AsyncMockTransport", subclass)]
pub struct AsyncMockTransport {
    handler: Arc<Mutex<Option<Py<PyAny>>>>,
}

impl Default for AsyncMockTransport {
    fn default() -> Self {
        Self {
            handler: Arc::new(Mutex::new(None)),
        }
    }
}

#[pymethods]
impl AsyncMockTransport {
    #[new]
    #[pyo3(signature = (handler=None))]
    fn new(handler: Option<Py<PyAny>>) -> Self {
        Self {
            handler: Arc::new(Mutex::new(handler)),
        }
    }

    fn handle_async_request<'py>(
        &self,
        py: Python<'py>,
        request: &Request,
    ) -> PyResult<Bound<'py, PyAny>> {
        use pyo3_async_runtimes::tokio::future_into_py;

        // Clone the handler Arc to move into the future
        let handler_arc = self.handler.clone();
        let request = request.clone();

        future_into_py(py, async move {
            Python::with_gil(|py| -> PyResult<Response> {
                let handler = handler_arc.lock();
                if let Some(ref h) = *handler {
                    let result = h.call1(py, (request,))?;
                    result.extract::<Response>(py).map_err(|e| e.into())
                } else {
                    Ok(Response::new(200))
                }
            })
        })
    }

    fn __repr__(&self) -> String {
        "<AsyncMockTransport>".to_string()
    }
}

/// HTTP transport using reqwest (the default transport)
#[pyclass(name = "HTTPTransport")]
#[derive(Clone)]
pub struct HTTPTransport {
    inner: Arc<reqwest::blocking::Client>,
    verify: bool,
    cert: Option<String>,
    http2: bool,
}

impl Default for HTTPTransport {
    fn default() -> Self {
        Self {
            inner: Arc::new(reqwest::blocking::Client::new()),
            verify: true,
            cert: None,
            http2: false,
        }
    }
}

#[pymethods]
impl HTTPTransport {
    #[new]
    #[pyo3(signature = (*, verify=true, cert=None, http2=false, retries=0, **_kwargs))]
    fn new(
        verify: bool,
        cert: Option<String>,
        http2: bool,
        retries: usize,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let _ = retries; // TODO: implement retries

        let mut builder = reqwest::blocking::Client::builder();

        if !verify {
            builder = builder.danger_accept_invalid_certs(true);
        }

        // TODO: Add cert support

        let client = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create transport: {}", e))
        })?;

        Ok(Self {
            inner: Arc::new(client),
            verify,
            cert,
            http2,
        })
    }

    fn __repr__(&self) -> String {
        format!("<HTTPTransport(verify={})>", self.verify)
    }

    fn close(&self) {
        // reqwest client doesn't need explicit close
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        self.close();
        false
    }
}

/// Async HTTP transport using reqwest
#[pyclass(name = "AsyncHTTPTransport")]
#[derive(Clone)]
pub struct AsyncHTTPTransport {
    inner: Arc<reqwest::Client>,
    verify: bool,
    cert: Option<String>,
    http2: bool,
}

impl Default for AsyncHTTPTransport {
    fn default() -> Self {
        Self {
            inner: Arc::new(reqwest::Client::new()),
            verify: true,
            cert: None,
            http2: false,
        }
    }
}

#[pymethods]
impl AsyncHTTPTransport {
    #[new]
    #[pyo3(signature = (*, verify=true, cert=None, http2=false, retries=0, **_kwargs))]
    fn new(
        verify: bool,
        cert: Option<String>,
        http2: bool,
        retries: usize,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let _ = retries;

        let mut builder = reqwest::Client::builder();

        if !verify {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create transport: {}", e))
        })?;

        Ok(Self {
            inner: Arc::new(client),
            verify,
            cert,
            http2,
        })
    }

    fn __repr__(&self) -> String {
        format!("<AsyncHTTPTransport(verify={})>", self.verify)
    }

    fn aclose<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use pyo3_async_runtimes::tokio::future_into_py;
        future_into_py(py, async move { Ok(()) })
    }

    fn __aenter__<'py>(slf: PyRef<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let slf_obj = slf.into_pyobject(py)?.unbind();
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(slf_obj) })
    }

    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.aclose(py)
    }
}
