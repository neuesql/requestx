use cookie_store::CookieStore;
use hyper::header::{HeaderValue, SET_COOKIE};
use hyper::{Method, Uri};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::client::{create_client, RequestxClient, ResponseData};
use crate::core::runtime::get_global_runtime_manager;
use crate::error::RequestxError;
use crate::{parse_kwargs, response_data_to_py_response};

/// Simple retry configuration extracted from Python Retry object
#[derive(Clone, Debug)]
struct RetryConfig {
    total: u32,
    backoff_factor: f64,
    status_forcelist: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            total: 0,
            backoff_factor: 0.1,
            status_forcelist: Vec::new(),
        }
    }
}

/// Case-insensitive header wrapper for session headers
#[derive(Clone, Debug)]
pub struct CaseInsensitiveHeaders {
    inner: HashMap<String, String>,
    lowercase_map: HashMap<String, String>,
}

impl CaseInsensitiveHeaders {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            lowercase_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: String) {
        let lowercase_key = key.to_lowercase();
        self.lowercase_map
            .insert(lowercase_key.clone(), key.clone());
        self.inner.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        let lowercase_key = key.to_lowercase();
        self.lowercase_map
            .get(&lowercase_key)
            .and_then(|original_key| self.inner.get(original_key))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut String> {
        let lowercase_key = key.to_lowercase();
        self.lowercase_map
            .get(&lowercase_key)
            .and_then(|original_key| self.inner.get_mut(original_key))
    }

    pub fn remove(&mut self, key: &str) {
        let lowercase_key = key.to_lowercase();
        if let Some(original_key) = self.lowercase_map.get(&lowercase_key) {
            self.inner.remove(original_key);
        }
        self.lowercase_map.remove(&lowercase_key);
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.lowercase_map.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.inner.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.inner.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &String> {
        self.inner.values()
    }
}

impl Default for CaseInsensitiveHeaders {
    fn default() -> Self {
        Self::new()
    }
}

/// Session object for persistent HTTP connections with cookie and header management
#[pyclass]
pub struct Session {
    client: RequestxClient,
    cookies: Arc<Mutex<CookieStore>>,
    headers: Arc<Mutex<CaseInsensitiveHeaders>>,
    trust_env: bool,
    max_redirects: u32,
    /// Maximum number of retries for failed requests (default: 0)
    max_retries: u32,
    /// Retry backoff factor in seconds (default: 0.1)
    backoff_factor: f64,
    /// Event hooks dictionary - each key is a hook name, value is a list of callable hooks
    hooks: Arc<Mutex<HashMap<String, Vec<PyObject>>>>,
    /// Mounted adapters for URL prefixes (adapter class name -> (prefix, adapter config))
    adapters: Arc<Mutex<HashMap<String, (String, PyObject)>>>,
}

#[pymethods]
impl Session {
    #[new]
    fn new() -> PyResult<Self> {
        let hyper_client = create_client();

        let client = RequestxClient::with_custom_client(hyper_client).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create session client: {e}"
            ))
        })?;

        let cookies = Arc::new(Mutex::new(CookieStore::default()));
        let headers = Arc::new(Mutex::new(CaseInsensitiveHeaders::new()));

        Ok(Session {
            client,
            cookies,
            headers,
            trust_env: true,
            max_redirects: 30,
            max_retries: 0,
            backoff_factor: 0.1,
            hooks: Arc::new(Mutex::new(HashMap::new())),
            adapters: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get hooks dictionary
    #[getter]
    fn get_hooks(&self, py: Python) -> PyResult<PyObject> {
        let hooks_dict = PyDict::new(py);
        let hooks = self.hooks.lock().unwrap();
        for (name, hook_list) in hooks.iter() {
            let py_list: PyObject = hook_list
                .iter()
                .map(|obj| obj.clone_ref(py))
                .collect::<Vec<_>>()
                .into_pyobject(py)?
                .into();
            hooks_dict.set_item(name, py_list)?;
        }
        Ok(hooks_dict.into())
    }

    /// Set hooks dictionary
    #[setter]
    fn set_hooks(&mut self, hooks_dict: &Bound<'_, PyDict>) -> PyResult<()> {
        let mut hooks = self.hooks.lock().unwrap();
        hooks.clear();

        for (key, value) in hooks_dict.iter() {
            let key_str = key.extract::<String>()?;
            // Value should be a list of callables
            let py_list = value.downcast::<pyo3::types::PyList>()?;
            let hook_list: Vec<PyObject> = py_list.iter().map(|obj| obj.unbind().into()).collect();
            hooks.insert(key_str, hook_list);
        }
        Ok(())
    }

    /// Register a hook callback for a specific event
    #[pyo3(signature = (event, callback))]
    fn register_hook(&mut self, event: String, callback: PyObject) -> PyResult<()> {
        let mut hooks = self.hooks.lock().unwrap();
        let hook_list = hooks.entry(event).or_insert_with(Vec::new);
        hook_list.push(callback);
        Ok(())
    }

    /// Get max_retries
    #[getter]
    fn get_max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Set max_retries
    #[setter]
    fn set_max_retries(&mut self, value: u32) {
        self.max_retries = value;
    }

    /// Get backoff_factor
    #[getter]
    fn get_backoff_factor(&self) -> f64 {
        self.backoff_factor
    }

    /// Set backoff_factor
    #[setter]
    fn set_backoff_factor(&mut self, value: f64) {
        self.backoff_factor = value;
    }

    /// Mount an adapter with retry configuration for a URL prefix.
    ///
    /// This allows different retry configurations for different URL prefixes.
    ///
    /// Examples:
    ///     >>> import requestx
    ///     >>> from requestx import Retry, HTTPAdapter
    ///     >>> s = requestx.Session()
    ///     >>> retry = Retry(total=3, status_forcelist=[502, 503, 504])
    ///     >>> adapter = HTTPAdapter(max_retries=retry)
    ///     >>> s.mount('https://', adapter)
    #[pyo3(signature = (prefix, adapter))]
    fn mount(&mut self, _py: Python, prefix: String, adapter: PyObject) -> PyResult<()> {
        let mut adapters = self.adapters.lock().unwrap();
        // Use a fixed key for now - all adapters go under "HTTPAdapter"
        adapters.insert("HTTPAdapter".to_string(), (prefix, adapter));
        Ok(())
    }

    /// Get adapter configuration for a given URL.
    fn get_adapter(&self, py: Python, url: &str) -> PyResult<PyObject> {
        let adapters = self.adapters.lock().unwrap();

        // Check each mounted adapter to see if it matches the URL
        for (prefix, adapter) in adapters.values() {
            if url.starts_with(prefix) {
                return Ok(adapter.clone_ref(py));
            }
        }

        // Return None if no adapter matches
        Ok(py.None().into())
    }

    /// Clone the session (copy headers and cookies)
    fn clone(&self) -> Self {
        // Create a new session with the same configuration
        let mut session = Session::new().expect("Failed to create cloned session");

        // Copy retry configuration
        session.max_retries = self.max_retries;
        session.backoff_factor = self.backoff_factor;

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

    /// HTTP GET request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn get(
        &self,
        py: Python,
        url: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        self.request(py, "GET".to_string(), url, kwargs)
    }

    /// HTTP POST request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn post(
        &self,
        py: Python,
        url: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        self.request(py, "POST".to_string(), url, kwargs)
    }

    /// HTTP PUT request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn put(
        &self,
        py: Python,
        url: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        self.request(py, "PUT".to_string(), url, kwargs)
    }

    /// HTTP DELETE request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn delete(
        &self,
        py: Python,
        url: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        self.request(py, "DELETE".to_string(), url, kwargs)
    }

    /// HTTP HEAD request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn head(
        &self,
        py: Python,
        url: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        self.request(py, "HEAD".to_string(), url, kwargs)
    }

    /// HTTP OPTIONS request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn options(
        &self,
        py: Python,
        url: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        self.request(py, "OPTIONS".to_string(), url, kwargs)
    }

    /// HTTP PATCH request using session
    #[pyo3(signature = (url, /, **kwargs))]
    fn patch(
        &self,
        py: Python,
        url: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        self.request(py, "PATCH".to_string(), url, kwargs)
    }

    /// Generic HTTP request using session with state persistence
    #[pyo3(signature = (method, url, /, **kwargs))]
    fn request(
        &self,
        py: Python,
        method: String,
        url: String,
        kwargs: Option<&Bound<'_, PyDict>>,
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
                    RequestxError::RuntimeError(format!("Invalid HTTP method: {method}")).into(),
                )
            }
        };

        let uri: Uri = url.parse().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid URL: {e}"))
        })?;

        // Parse kwargs and merge with session headers
        let mut config_builder = parse_kwargs(py, kwargs)?;

        // Merge session headers with request headers (request headers take precedence)
        let session_headers = self.headers.lock().unwrap();
        if !session_headers.is_empty() {
            // Start with session headers
            let mut merged_headers = hyper::HeaderMap::new();

            // First, add session headers
            for (name, value) in session_headers.iter() {
                if let (Ok(header_name), Ok(header_value)) = (
                    name.parse::<hyper::header::HeaderName>(),
                    value.parse::<hyper::header::HeaderValue>(),
                ) {
                    merged_headers.insert(header_name, header_value);
                }
            }

            // Then overlay with request headers (these take precedence)
            if let Some(request_headers) = config_builder.headers.take() {
                for (header_name, header_value) in request_headers {
                    if let Some(name) = header_name {
                        merged_headers.insert(name, header_value);
                    }
                }
            }

            config_builder.headers = Some(merged_headers);
        }

        // Get cookies for this URL from the session cookie store
        let cookies_for_url = {
            let cookies = self.cookies.lock().unwrap();
            Self::get_cookies_for_url(&cookies, &uri)
        };

        // Add cookies to request headers
        if let Some(cookie_header) = cookies_for_url {
            if let Ok(cookie_value) = hyper::header::HeaderValue::from_str(&cookie_header) {
                if let Some(ref mut headers) = config_builder.headers {
                    headers.insert(hyper::header::COOKIE, cookie_value);
                } else {
                    let mut headers = hyper::HeaderMap::new();
                    headers.insert(hyper::header::COOKIE, cookie_value);
                    config_builder.headers = Some(headers);
                }
            }
        }

        // Apply max_redirects from session if not explicitly set
        if config_builder.max_redirects.is_none() {
            config_builder.max_redirects = Some(self.max_redirects);
        }

        let config = config_builder.build(method, uri);

        // Clone necessary data for the async closure
        let client = self.client.clone();
        let cookies = Arc::clone(&self.cookies);
        let session_headers = Arc::clone(&self.headers);
        let hooks = Arc::clone(&self.hooks);
        let adapters = Arc::clone(&self.adapters);

        // Extract retry configuration from mounted adapter BEFORE the async block
        let retry_config: RetryConfig = {
            let adapters_lock = adapters.lock().unwrap();
            if let Some((prefix, adapter)) = adapters_lock.get("HTTPAdapter") {
                if url.starts_with(prefix) {
                    // Extract retry config from Python Retry object
                    let max_retries_obj = adapter.getattr(py, "max_retries")?;

                    // Get total retries
                    let total: u32 = if let Ok(total_attr) = max_retries_obj.getattr(py, "total") {
                        total_attr.extract::<u32>(py).unwrap_or(0)
                    } else {
                        max_retries_obj.extract::<u32>(py).unwrap_or(0)
                    };

                    // Get backoff_factor
                    let backoff_factor: f64 = max_retries_obj
                        .getattr(py, "backoff_factor")
                        .ok()
                        .and_then(|bf| bf.extract::<f64>(py).ok())
                        .unwrap_or(0.1);

                    // Get status_forcelist
                    let status_forcelist: Vec<u16> = max_retries_obj
                        .getattr(py, "status_forcelist")
                        .ok()
                        .and_then(|sf| sf.extract::<Vec<u16>>(py).ok())
                        .unwrap_or_default();

                    RetryConfig {
                        total,
                        backoff_factor,
                        status_forcelist,
                    }
                } else {
                    RetryConfig::default()
                }
            } else {
                RetryConfig::default()
            }
        };

        // Use enhanced runtime management for context detection and execution
        let runtime_manager = get_global_runtime_manager();

        let future = async move {
            let mut attempt = 0;

            loop {
                attempt += 1;

                match client.request_async(config.clone()).await {
                    Ok(response_data) => {
                        let status_code = response_data.status_code;

                        // Check if we should retry based on status code
                        let should_retry = retry_config.total > 0
                            && retry_config.status_forcelist.contains(&status_code)
                            && attempt <= retry_config.total as usize;

                        if should_retry {
                            // Exponential backoff
                            let delay_ms = (retry_config.backoff_factor
                                * 1000.0
                                * (2.0_f64.powi(attempt as i32 - 1)))
                                as u64;
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        }

                        // Process cookies from response
                        Self::process_response_cookies(&cookies, &response_data).await;

                        // Update session headers if needed
                        Self::update_session_headers(&session_headers, &response_data).await;

                        // Convert to PyObject
                        break response_data_to_py_response(response_data);
                    }
                    Err(_) => {
                        // Connection error - check if we should retry
                        if retry_config.total > 0 && attempt <= retry_config.total as usize {
                            // Exponential backoff for connection errors
                            let delay_ms = (retry_config.backoff_factor
                                * 1000.0
                                * (2.0_f64.powi(attempt as i32 - 1)))
                                as u64;
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        }

                        // No more retries or retry disabled - return error
                        break Err(RequestxError::RuntimeError(format!(
                            "Connection failed after {} attempts",
                            attempt
                        ))
                        .into());
                    }
                }
            }
        };

        let py_response = runtime_manager.execute_future(py, future)?;

        // Execute response hooks (outside of async to avoid thread issues)
        {
            let hooks_lock = hooks.lock().unwrap();
            if let Some(response_hooks) = hooks_lock.get("response") {
                for hook in response_hooks {
                    // Call the hook with the response
                    // Clone the PyObject using clone_ref which is the correct method
                    let py_response_clone = py_response.clone_ref(py);
                    let _ = hook.call1(py, (py_response_clone,));
                }
            }
        }

        Ok(py_response)
    }

    /// Get session headers as a dictionary
    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        let headers = self.headers.lock().unwrap();
        let dict = pyo3::types::PyDict::new(py);

        for (name, value) in headers.iter() {
            dict.set_item(name, value)?;
        }

        Ok(dict.into())
    }

    /// Set session headers from a dictionary
    #[setter]
    fn set_headers(&self, headers_dict: &Bound<'_, PyDict>) -> PyResult<()> {
        let mut headers = self.headers.lock().unwrap();
        headers.clear();

        for (key, value) in headers_dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_str = value.extract::<String>()?;
            headers.insert(key_str, value_str);
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
        headers.insert(name, value);
        Ok(())
    }

    /// Remove a session header
    fn remove_header(&self, name: String) -> PyResult<()> {
        let mut headers = self.headers.lock().unwrap();
        headers.remove(&name);
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

    /// Get trust_env setting
    #[getter]
    fn trust_env(&self) -> bool {
        self.trust_env
    }

    /// Set trust_env setting
    #[setter]
    fn set_trust_env(&mut self, value: bool) {
        self.trust_env = value;
    }

    /// Get max_redirects setting
    #[getter]
    fn max_redirects(&self) -> u32 {
        self.max_redirects
    }

    /// Set max_redirects setting
    #[setter]
    fn set_max_redirects(&mut self, value: u32) {
        self.max_redirects = value;
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
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        self.close()?;
        Ok(false) // Don't suppress exceptions
    }

    /// String representation of the session
    fn __repr__(&self) -> String {
        let headers_count = self.headers.lock().unwrap().len();
        let cookies_count = self.cookies.lock().unwrap().iter_any().count();
        format!(
            "<Session headers={} cookies={} trust_env={} max_redirects={}>",
            headers_count, cookies_count, self.trust_env, self.max_redirects
        )
    }
}

impl Session {
    /// Get cookies for a specific URL from the cookie store
    fn get_cookies_for_url(cookie_store: &CookieStore, uri: &Uri) -> Option<String> {
        // Create a url::Url from the URI for cookie matching
        let request_url = url::Url::parse(uri.to_string().as_str()).ok()?;
        let cookies: Vec<String> = cookie_store
            .iter_any()
            .filter(|cookie| cookie.matches(&request_url))
            .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
            .collect();

        if cookies.is_empty() {
            None
        } else {
            Some(cookies.join("; "))
        }
    }

    /// Process cookies from HTTP response and store them in the session
    async fn process_response_cookies(
        cookies: &Arc<Mutex<CookieStore>>,
        response_data: &ResponseData,
    ) {
        // Get all Set-Cookie headers from the response
        let set_cookie_headers: Vec<HeaderValue> = response_data
            .headers
            .get_all(SET_COOKIE)
            .into_iter()
            .cloned()
            .collect();

        if set_cookie_headers.is_empty() {
            return;
        }

        // Parse each Set-Cookie header and add to the store
        let mut cookie_store = cookies.lock().unwrap();

        // Create request URL for cookie domain validation
        let request_url = url::Url::parse(response_data.url.to_string().as_str());

        for header_value in set_cookie_headers {
            if let Ok(header_str) = header_value.to_str() {
                // Parse the cookie from the Set-Cookie header
                if let Ok(ref url) = request_url {
                    let _ = cookie_store.parse(header_str, url);
                }
            }
        }
    }

    /// Update session headers based on response (e.g., authentication tokens)
    async fn update_session_headers(
        _session_headers: &Arc<Mutex<CaseInsensitiveHeaders>>,
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
