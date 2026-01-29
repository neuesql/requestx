//! HTTP Transport implementations including MockTransport

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};
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

/// WSGI Transport - allows making requests to WSGI applications
#[pyclass(name = "WSGITransport")]
pub struct WSGITransport {
    app: Py<PyAny>,
    wsgi_errors: Option<Py<PyAny>>,
    script_name: String,
    root_path: String,
}

#[pymethods]
impl WSGITransport {
    #[new]
    #[pyo3(signature = (app, *, raise_app_exceptions=true, script_name="", root_path="", wsgi_errors=None))]
    fn new(
        app: Py<PyAny>,
        raise_app_exceptions: bool,
        script_name: &str,
        root_path: &str,
        wsgi_errors: Option<Py<PyAny>>,
    ) -> Self {
        let _ = raise_app_exceptions; // We always raise exceptions
        Self {
            app,
            wsgi_errors,
            script_name: script_name.to_string(),
            root_path: root_path.to_string(),
        }
    }

    fn handle_request(&self, py: Python<'_>, request: &Request) -> PyResult<Response> {
        let io_module = py.import("io")?;

        // Get request details using public Rust methods
        let url = request.url_ref();
        let method = request.method();
        let headers = request.headers_ref();
        let body = request.content_bytes();

        // Build wsgi.input from request body
        let wsgi_input = if let Some(body_bytes) = body {
            let bytes_io = io_module.getattr("BytesIO")?;
            bytes_io.call1((PyBytes::new(py, body_bytes),))?
        } else {
            let bytes_io = io_module.getattr("BytesIO")?;
            bytes_io.call1((PyBytes::new(py, b""),))?
        };

        // Build wsgi.errors
        let wsgi_errors_obj = if let Some(ref errors) = self.wsgi_errors {
            errors.clone_ref(py).into_bound(py)
        } else {
            let string_io = io_module.getattr("StringIO")?;
            string_io.call0()?
        };

        // Parse URL components
        let url_str = url.to_string();
        let parsed_url = reqwest::Url::parse(&url_str).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e))
        })?;

        let host = parsed_url.host_str().unwrap_or("localhost");
        let port = parsed_url.port_or_known_default().unwrap_or(80);
        let path = parsed_url.path();
        let query_string = parsed_url.query().unwrap_or("");
        let scheme = parsed_url.scheme();

        // Build environ dict
        let environ = PyDict::new(py);
        environ.set_item("REQUEST_METHOD", method)?;
        environ.set_item("SCRIPT_NAME", &self.script_name)?;
        environ.set_item("PATH_INFO", path)?;
        environ.set_item("QUERY_STRING", query_string)?;
        environ.set_item("SERVER_NAME", host)?;
        environ.set_item("SERVER_PORT", port.to_string())?;
        environ.set_item("SERVER_PROTOCOL", "HTTP/1.1")?;
        environ.set_item("wsgi.version", (1, 0))?;
        environ.set_item("wsgi.url_scheme", scheme)?;
        environ.set_item("wsgi.input", &wsgi_input)?;
        environ.set_item("wsgi.errors", &wsgi_errors_obj)?;
        environ.set_item("wsgi.multithread", true)?;
        environ.set_item("wsgi.multiprocess", true)?;
        environ.set_item("wsgi.run_once", false)?;

        // Add headers to environ (using the Rust headers_ref method)
        for (key, value) in headers.iter_pairs() {
            // Convert header name to WSGI format
            let key_upper = key.to_uppercase().replace('-', "_");
            if key_upper == "CONTENT_TYPE" {
                environ.set_item("CONTENT_TYPE", &value)?;
            } else if key_upper == "CONTENT_LENGTH" {
                environ.set_item("CONTENT_LENGTH", &value)?;
            } else {
                environ.set_item(format!("HTTP_{}", key_upper), &value)?;
            }
        }

        // Add content-length if we have a body
        if let Some(body_bytes) = body {
            if !environ.contains("CONTENT_LENGTH")? {
                environ.set_item("CONTENT_LENGTH", body_bytes.len().to_string())?;
            }
        }

        // Create start_response callable using a class-based approach
        let status_holder: Py<PyList> = PyList::empty(py).unbind();
        let headers_holder: Py<PyList> = PyList::empty(py).unbind();
        let exc_info_holder: Py<PyList> = PyList::empty(py).unbind();

        // Create a callable class instance
        let locals = PyDict::new(py);
        locals.set_item("status_holder", &status_holder)?;
        locals.set_item("headers_holder", &headers_holder)?;
        locals.set_item("exc_info_holder", &exc_info_holder)?;

        py.run(
            c"
class StartResponse:
    def __init__(self, status_h, headers_h, exc_h):
        self.status_h = status_h
        self.headers_h = headers_h
        self.exc_h = exc_h
    def __call__(self, status, response_headers, exc_info=None):
        if exc_info:
            self.exc_h.append(exc_info)
        self.status_h.append(status)
        for h in response_headers:
            self.headers_h.append(h)
        return lambda x: None  # write() callable

start_response = StartResponse(status_holder, headers_holder, exc_info_holder)
",
            None,
            Some(&locals),
        )?;

        let start_response = locals.get_item("start_response")?.unwrap();

        // Call the WSGI app
        let result = self.app.call1(py, (environ, start_response))?;

        // Collect response body by manually iterating
        // NOTE: For generators, start_response is called during iteration!
        let result_bound = result.bind(py);
        let mut body_parts: Vec<u8> = Vec::new();

        // Get the iterator from the result
        let iter = result_bound.call_method0("__iter__")?;

        // Iterate until StopIteration
        loop {
            match iter.call_method0("__next__") {
                Ok(chunk) => {
                    let bytes: Vec<u8> = chunk.extract()?;
                    body_parts.extend_from_slice(&bytes);
                }
                Err(e) if e.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) => {
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        // Close the iterator if it has a close method (WSGI protocol)
        if result_bound.hasattr("close")? {
            result_bound.call_method0("close")?;
        }

        // Check for exc_info (after iteration since start_response may be called during iteration)
        let exc_info_bound = exc_info_holder.bind(py);
        if exc_info_bound.len() > 0 {
            // Re-raise the exception
            let exc_tuple = exc_info_bound.get_item(0)?;
            let exc_tuple = exc_tuple.downcast::<PyTuple>()?;
            let exc_value = exc_tuple.get_item(1)?;
            // Raise the exception
            return Err(PyErr::from_value(exc_value.unbind().into_bound(py)));
        }

        // Parse status (after iteration since start_response may be called during iteration for generators)
        let status_bound = status_holder.bind(py);
        if status_bound.len() == 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "start_response was not called",
            ));
        }
        let status_str: String = status_bound.get_item(0)?.extract()?;
        let status_code: u16 = status_str
            .split_whitespace()
            .next()
            .unwrap_or("200")
            .parse()
            .unwrap_or(200);

        // Build response
        let mut response = Response::new(status_code);

        // Set headers
        let headers_bound = headers_holder.bind(py);
        for header in headers_bound.iter() {
            let tuple = header.downcast::<PyTuple>()?;
            let name: String = tuple.get_item(0)?.extract()?;
            let value: String = tuple.get_item(1)?.extract()?;
            response.set_header(&name, &value);
        }

        // Set body
        response.set_content(body_parts);

        Ok(response)
    }

    fn __repr__(&self) -> String {
        "<WSGITransport>".to_string()
    }

    fn close(&self) {
        // No-op
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
