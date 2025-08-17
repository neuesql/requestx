use cookie_store::CookieStore;
use hyper::{Client, HeaderMap, Method, Uri};
use hyper_tls::HttpsConnector;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::core::client::{RequestxClient, ResponseData};
use crate::core::runtime::get_global_runtime_manager;
use crate::error::RequestxError;
use crate::{parse_kwargs, response_data_to_py_response};

/// Session object for persistent HTTP connections with cookie and header management
#[pyclass]
pub struct Session {
    client: RequestxClient,
    cookies: Arc<Mutex<CookieStore>>,
    headers: Arc<Mutex<HeaderMap>>,
}

#[pymethods]
impl Session {
    #[new]
    fn new() -> PyResult<Self> {
        // Create a custom hyper client for the session with optimized settings
        let https = HttpsConnector::new();
        let hyper_client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(90)) // Longer idle timeout for session reuse
            .pool_max_idle_per_host(50) // More connections per host for sessions
            .http2_only(false) // Allow HTTP/1.1 fallback
            .http2_initial_stream_window_size(Some(65536)) // Optimize HTTP/2 streams
            .http2_initial_connection_window_size(Some(1048576)) // 1MB connection window
            .build::<_, hyper::Body>(https);

        let client = RequestxClient::with_custom_client(hyper_client).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create session client: {}",
                e
            ))
        })?;

        let cookies = Arc::new(Mutex::new(CookieStore::default()));
        let headers = Arc::new(Mutex::new(HeaderMap::new()));

        Ok(Session {
            client,
            cookies,
            headers,
        })
    }

    /// HTTP GET request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn get(&self, py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request(py, "GET".to_string(), url, kwargs)
    }

    /// HTTP POST request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn post(&self, py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request(py, "POST".to_string(), url, kwargs)
    }

    /// HTTP PUT request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn put(&self, py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request(py, "PUT".to_string(), url, kwargs)
    }

    /// HTTP DELETE request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn delete(&self, py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request(py, "DELETE".to_string(), url, kwargs)
    }

    /// HTTP HEAD request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn head(&self, py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request(py, "HEAD".to_string(), url, kwargs)
    }

    /// HTTP OPTIONS request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn options(&self, py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request(py, "OPTIONS".to_string(), url, kwargs)
    }

    /// HTTP PATCH request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn patch(&self, py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
        self.request(py, "PATCH".to_string(), url, kwargs)
    }

    /// Generic HTTP request using session with state persistence
    #[pyo3(signature = (method, url, /, **kwargs))]
    fn request(
        &self,
        py: Python,
        method: String,
        url: String,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyObject> {
        // Validate HTTP method
        let method_upper = method.to_uppercase();
        let method: Method = match method_upper.as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "HEAD" => Method::HEAD,
            "OPTIONS" => Method::OPTIONS,
            "PATCH" => Method::PATCH,
            "TRACE" => Method::TRACE,
            "CONNECT" => Method::CONNECT,
            _ => {
                return Err(
                    RequestxError::RuntimeError(format!("Invalid HTTP method: {}", method)).into(),
                )
            }
        };

        let uri: Uri = url.parse().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid URL: {}", e))
        })?;

        // Parse kwargs and merge with session headers
        let mut config_builder = parse_kwargs(py, kwargs)?;

        // Merge session headers with request headers
        let session_headers = self.headers.lock().unwrap();
        if !session_headers.is_empty() {
            let mut merged_headers = session_headers.clone();

            // If request has headers, merge them (request headers take precedence)
            if let Some(ref request_headers) = config_builder.headers {
                for (name, value) in request_headers.iter() {
                    merged_headers.insert(name, value.clone());
                }
            }

            config_builder.headers = Some(merged_headers);
        }

        let config = config_builder.build(method, uri);

        // Clone necessary data for the async closure
        let client = self.client.clone();
        let cookies = Arc::clone(&self.cookies);
        let session_headers = Arc::clone(&self.headers);

        // Use enhanced runtime management for context detection and execution
        let runtime_manager = get_global_runtime_manager();

        let future = async move {
            // Execute the request
            let response_data = client.request_async(config).await?;

            // Process cookies from response
            Self::process_response_cookies(&cookies, &response_data).await;

            // Update session headers if needed (e.g., from authentication responses)
            Self::update_session_headers(&session_headers, &response_data).await;

            response_data_to_py_response(response_data)
        };

        runtime_manager.execute_future(py, future)
    }

    /// Get session headers as a dictionary
    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        let headers = self.headers.lock().unwrap();
        let dict = pyo3::types::PyDict::new(py);

        for (name, value) in headers.iter() {
            let name_str = name.to_string();
            let value_str = value.to_str().unwrap_or("").to_string();
            dict.set_item(name_str, value_str)?;
        }

        Ok(dict.into())
    }

    /// Set session headers from a dictionary
    #[setter]
    fn set_headers(&self, headers_dict: &PyDict) -> PyResult<()> {
        let mut headers = self.headers.lock().unwrap();
        headers.clear();

        for (key, value) in headers_dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_str = value.extract::<String>()?;

            let header_name = key_str.parse::<hyper::header::HeaderName>().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid header name '{}': {}",
                    key_str, e
                ))
            })?;

            let header_value = value_str
                .parse::<hyper::header::HeaderValue>()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid header value '{}': {}",
                        value_str, e
                    ))
                })?;

            headers.insert(header_name, header_value);
        }

        Ok(())
    }

    /// Get session cookies as a dictionary (simplified representation)
    #[getter]
    fn cookies(&self, py: Python) -> PyResult<PyObject> {
        let cookies = self.cookies.lock().unwrap();
        let dict = pyo3::types::PyDict::new(py);

        // Convert cookie store to a simple name-value dictionary
        for cookie in cookies.iter_any() {
            dict.set_item(cookie.name(), cookie.value())?;
        }

        Ok(dict.into())
    }

    /// Update a session header
    fn update_header(&self, name: String, value: String) -> PyResult<()> {
        let mut headers = self.headers.lock().unwrap();

        let header_name = name.parse::<hyper::header::HeaderName>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid header name '{}': {}",
                name, e
            ))
        })?;

        let header_value = value.parse::<hyper::header::HeaderValue>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid header value '{}': {}",
                value, e
            ))
        })?;

        headers.insert(header_name, header_value);
        Ok(())
    }

    /// Remove a session header
    fn remove_header(&self, name: String) -> PyResult<()> {
        let mut headers = self.headers.lock().unwrap();

        let header_name = name.parse::<hyper::header::HeaderName>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid header name '{}': {}",
                name, e
            ))
        })?;

        headers.remove(&header_name);
        Ok(())
    }

    /// Clear all session headers
    fn clear_headers(&self) -> PyResult<()> {
        let mut headers = self.headers.lock().unwrap();
        headers.clear();
        Ok(())
    }

    /// Clear all session cookies
    fn clear_cookies(&self) -> PyResult<()> {
        let mut cookies = self.cookies.lock().unwrap();
        cookies.clear();
        Ok(())
    }

    /// Close the session (cleanup resources)
    fn close(&self) -> PyResult<()> {
        // Clear cookies and headers
        self.clear_cookies()?;
        self.clear_headers()?;
        Ok(())
    }

    /// Context manager support - enter
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Context manager support - exit
    fn __exit__(
        &self,
        _exc_type: Option<&PyAny>,
        _exc_value: Option<&PyAny>,
        _traceback: Option<&PyAny>,
    ) -> PyResult<bool> {
        self.close()?;
        Ok(false) // Don't suppress exceptions
    }

    /// String representation of the session
    fn __repr__(&self) -> String {
        let headers_count = self.headers.lock().unwrap().len();
        let cookies_count = self.cookies.lock().unwrap().iter_any().count();
        format!(
            "<Session headers={} cookies={}>",
            headers_count, cookies_count
        )
    }
}

impl Session {
    /// Process cookies from HTTP response and store them in the session
    async fn process_response_cookies(
        _cookies: &Arc<Mutex<CookieStore>>,
        _response_data: &ResponseData,
    ) {
        // TODO: Implement proper cookie parsing and storage
        // For now, we'll skip cookie processing due to lifetime complexities
        // This can be enhanced in a future iteration to properly handle cookies
        // The cookie store is available and ready for implementation
    }

    /// Update session headers based on response (e.g., authentication tokens)
    async fn update_session_headers(
        _session_headers: &Arc<Mutex<HeaderMap>>,
        _response_data: &ResponseData,
    ) {
        // For now, we don't automatically update session headers from responses
        // This could be extended to handle authentication tokens, etc.
        // Future enhancement: parse WWW-Authenticate headers, update Authorization, etc.
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        // Create a new session with the same configuration
        let session = Session::new().expect("Failed to create cloned session");

        // Copy headers
        {
            let source_headers = self.headers.lock().unwrap();
            let mut dest_headers = session.headers.lock().unwrap();
            *dest_headers = source_headers.clone();
        }

        // Copy cookies
        {
            let source_cookies = self.cookies.lock().unwrap();
            let mut dest_cookies = session.cookies.lock().unwrap();
            *dest_cookies = source_cookies.clone();
        }

        session
    }
}
