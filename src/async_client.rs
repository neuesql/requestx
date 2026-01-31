//! Asynchronous HTTP Client implementation

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;
use std::collections::HashMap;
use std::sync::Arc;

use crate::cookies::Cookies;
use crate::exceptions::convert_reqwest_error;
use crate::headers::Headers;
use crate::request::Request;
use crate::response::Response;
use crate::timeout::Timeout;
use crate::types::BasicAuth;
use crate::url::URL;

/// Helper to extract URL string from either String or URL object
fn extract_url_string(url: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = url.extract::<String>() {
        Ok(s)
    } else if let Ok(u) = url.extract::<URL>() {
        Ok(u.to_string())
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "URL must be a string or URL object",
        ))
    }
}

/// Event hooks storage
#[derive(Default)]
struct EventHooks {
    request: Vec<Py<PyAny>>,
    response: Vec<Py<PyAny>>,
}

/// Asynchronous HTTP Client
#[pyclass(name = "AsyncClient")]
pub struct AsyncClient {
    inner: Arc<reqwest::Client>,
    base_url: Option<URL>,
    headers: Headers,
    cookies: Cookies,
    timeout: Timeout,
    follow_redirects: bool,
    max_redirects: usize,
    event_hooks: EventHooks,
    trust_env: bool,
    mounts: HashMap<String, Py<PyAny>>,
    transport: Option<Py<PyAny>>,
    /// Cached default transport - created lazily and reused
    default_transport: Option<Py<PyAny>>,
    /// Client-level auth
    auth: Option<(String, String)>,
}

impl Default for AsyncClient {
    fn default() -> Self {
        Self::new_impl(None, None, None, None, None, None, None).unwrap()
    }
}

impl AsyncClient {
    fn new_impl(
        auth: Option<(String, String)>,
        headers: Option<Headers>,
        cookies: Option<Cookies>,
        timeout: Option<Timeout>,
        follow_redirects: Option<bool>,
        max_redirects: Option<usize>,
        base_url: Option<URL>,
    ) -> PyResult<Self> {
        let timeout = timeout.unwrap_or_default();
        let follow_redirects = follow_redirects.unwrap_or(true);
        let max_redirects = max_redirects.unwrap_or(20);

        let mut builder = reqwest::Client::builder()
            .redirect(if follow_redirects {
                reqwest::redirect::Policy::limited(max_redirects)
            } else {
                reqwest::redirect::Policy::none()
            });

        if let Some(dur) = timeout.to_duration() {
            builder = builder.timeout(dur);
        }

        if let Some(connect_dur) = timeout.connect_duration() {
            builder = builder.connect_timeout(connect_dur);
        }

        let client = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create client: {}", e))
        })?;

        // Create default headers if none provided
        let version = env!("CARGO_PKG_VERSION");
        let mut default_headers = Headers::default();
        default_headers.set("accept".to_string(), "*/*".to_string());
        default_headers.set("accept-encoding".to_string(), "gzip, deflate, br, zstd".to_string());
        default_headers.set("connection".to_string(), "keep-alive".to_string());
        default_headers.set("user-agent".to_string(), format!("python-httpx/{}", version));

        // Merge user-provided headers over defaults
        let final_headers = if let Some(user_headers) = headers {
            // Start with defaults, then overlay user headers
            for (k, v) in user_headers.inner() {
                default_headers.set(k.clone(), v.clone());
            }
            default_headers
        } else {
            default_headers
        };

        Ok(Self {
            inner: Arc::new(client),
            base_url,
            headers: final_headers,
            cookies: cookies.unwrap_or_default(),
            timeout,
            follow_redirects,
            max_redirects,
            event_hooks: EventHooks::default(),
            trust_env: true,
            mounts: HashMap::new(),
            transport: None,
            default_transport: None,
            auth,
        })
    }

    fn resolve_url(&self, url: &str) -> PyResult<String> {
        if let Some(base) = &self.base_url {
            if !url.contains("://") {
                return Ok(base.join_url(url)?.to_string());
            }
        }
        Ok(url.to_string())
    }
}

#[pymethods]
impl AsyncClient {
    #[new]
    #[pyo3(signature = (*, auth=None, cookies=None, headers=None, timeout=None, follow_redirects=None, max_redirects=None, base_url=None, event_hooks=None, trust_env=None, transport=None, mounts=None, proxy=None, **_kwargs))]
    fn new(
        py: Python<'_>,
        auth: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        max_redirects: Option<usize>,
        base_url: Option<&Bound<'_, PyAny>>,
        event_hooks: Option<&Bound<'_, PyDict>>,
        trust_env: Option<bool>,
        transport: Option<Py<PyAny>>,
        mounts: Option<&Bound<'_, PyDict>>,
        proxy: Option<&str>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let auth_tuple = if let Some(a) = auth {
            if let Ok(basic) = a.extract::<BasicAuth>() {
                Some((basic.username, basic.password))
            } else if let Ok(tuple) = a.extract::<(String, String)>() {
                Some(tuple)
            } else {
                None
            }
        } else {
            None
        };

        let headers_obj = if let Some(h) = headers {
            if let Ok(headers_obj) = h.extract::<Headers>() {
                Some(headers_obj)
            } else if let Ok(dict) = h.downcast::<PyDict>() {
                let mut hdr = Headers::new();
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    hdr.set(k, v);
                }
                Some(hdr)
            } else {
                None
            }
        } else {
            None
        };

        let cookies_obj = if let Some(c) = cookies {
            c.extract::<Cookies>().ok()
        } else {
            None
        };

        let timeout_obj = if let Some(t) = timeout {
            if let Ok(timeout_obj) = t.extract::<Timeout>() {
                Some(timeout_obj)
            } else if let Ok(secs) = t.extract::<f64>() {
                Some(Timeout::new(Some(secs), None, None, None, None))
            } else {
                None
            }
        } else {
            None
        };

        let base_url_obj = if let Some(url) = base_url {
            if let Ok(url_obj) = url.extract::<URL>() {
                Some(url_obj)
            } else if let Ok(url_str) = url.extract::<String>() {
                Some(URL::parse(&url_str)?)
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "base_url must be a string or URL object",
                ));
            }
        } else {
            None
        };

        let mut client = Self::new_impl(
            auth_tuple,
            headers_obj,
            cookies_obj,
            timeout_obj,
            follow_redirects,
            max_redirects,
            base_url_obj,
        )?;

        // Set trust_env
        if let Some(trust) = trust_env {
            client.trust_env = trust;
        }

        // Parse event_hooks dict if provided
        if let Some(hooks_dict) = event_hooks {
            if let Some(request_hooks) = hooks_dict.get_item("request")? {
                if let Ok(list) = request_hooks.downcast::<PyList>() {
                    for item in list.iter() {
                        client.event_hooks.request.push(item.unbind());
                    }
                }
            }
            if let Some(response_hooks) = hooks_dict.get_item("response")? {
                if let Ok(list) = response_hooks.downcast::<PyList>() {
                    for item in list.iter() {
                        client.event_hooks.response.push(item.unbind());
                    }
                }
            }
        }

        // Set transport if provided
        client.transport = transport;

        // Initialize default transport (with proxy if specified)
        let async_transport = if proxy.is_some() {
            crate::transport::AsyncHTTPTransport::with_proxy(proxy)?
        } else {
            crate::transport::AsyncHTTPTransport::default()
        };
        client.default_transport = Some(Py::new(py, async_transport)?.into_any());

        // Handle mounts with validation
        if let Some(mounts_dict) = mounts {
            for (key, value) in mounts_dict.iter() {
                let pattern: String = key.extract()?;
                // Validate mount key format - must contain "://"
                if !pattern.contains("://") {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Mount pattern '{}' is invalid. Did you mean '{}://'?",
                        pattern, pattern
                    )));
                }
                client.mounts.insert(pattern, value.unbind());
            }
        }

        Ok(client)
    }

    /// HTTP GET request
    /// auth parameter: Rust None = use client auth, Python None = disable auth, (user,pass) = use this auth
    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url_str = extract_url_string(url)?;
        self.async_request(py, "GET".to_string(), url_str, None, None, None, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn post<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<PyObject>,
        files: Option<PyObject>,
        json: Option<PyObject>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url_str = extract_url_string(url)?;
        self.async_request(py, "POST".to_string(), url_str, content, data, json, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn put<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<PyObject>,
        files: Option<PyObject>,
        json: Option<PyObject>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url_str = extract_url_string(url)?;
        self.async_request(py, "PUT".to_string(), url_str, content, data, json, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn patch<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<PyObject>,
        files: Option<PyObject>,
        json: Option<PyObject>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url_str = extract_url_string(url)?;
        self.async_request(py, "PATCH".to_string(), url_str, content, data, json, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn delete<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url_str = extract_url_string(url)?;
        self.async_request(py, "DELETE".to_string(), url_str, None, None, None, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn head<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url_str = extract_url_string(url)?;
        self.async_request(py, "HEAD".to_string(), url_str, None, None, None, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn options<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url_str = extract_url_string(url)?;
        self.async_request(py, "OPTIONS".to_string(), url_str, None, None, None, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn request<'py>(
        &self,
        py: Python<'py>,
        method: String,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<PyObject>,
        files: Option<PyObject>,
        json: Option<PyObject>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url_str = extract_url_string(url)?;
        self.async_request(py, method, url_str, content, data, json, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn stream<'py>(
        &self,
        py: Python<'py>,
        method: String,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<PyObject>,
        files: Option<PyObject>,
        json: Option<PyObject>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<AsyncStreamContextManager> {
        let url_str = extract_url_string(url)?;

        // Prepare all the request parameters for the async context manager
        Ok(AsyncStreamContextManager {
            client: self.clone_for_stream(py)?,
            method,
            url: url_str,
            content,
            data,
            json,
            params,
            headers,
            cookies,
            auth,
            follow_redirects,
            timeout,
            response: None,
        })
    }

    fn aclose<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            Ok(())
        })
    }

    #[pyo3(signature = (method, url, *, content=None, params=None, headers=None))]
    fn build_request(
        &self,
        method: &str,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Request> {
        let url_str = extract_url_string(url)?;
        let resolved_url = self.resolve_url(&url_str)?;
        let parsed_url = URL::new_impl(Some(&resolved_url), None, None, None, None, None, None, None, None, params, None, None)?;

        // Extract Host header info before moving parsed_url
        let host_header_value: Option<String> = if let Some(host) = parsed_url.inner().host_str() {
            let host_value = if let Some(port) = parsed_url.inner().port() {
                // Include non-default port in Host header
                let scheme = parsed_url.inner().scheme();
                let default_port: u16 = match scheme {
                    "http" => 80,
                    "https" => 443,
                    _ => 0,
                };
                if port != default_port {
                    format!("{}:{}", host, port)
                } else {
                    host.to_string()
                }
            } else {
                host.to_string()
            };
            Some(host_value)
        } else {
            None
        };

        let mut request = Request::new(method, parsed_url);

        // Add headers
        let mut all_headers = self.headers.clone();
        if let Some(h) = headers {
            if let Ok(headers_obj) = h.extract::<Headers>() {
                for (k, v) in headers_obj.inner() {
                    all_headers.set(k.clone(), v.clone());
                }
            } else if let Ok(dict) = h.downcast::<pyo3::types::PyDict>() {
                for (key, value) in dict.iter() {
                    if let (Ok(k), Ok(v)) = (key.extract::<String>(), value.extract::<String>()) {
                        all_headers.set(k, v);
                    }
                }
            } else if let Ok(list) = h.downcast::<pyo3::types::PyList>() {
                for item in list.iter() {
                    if let Ok(tuple) = item.downcast::<pyo3::types::PyTuple>() {
                        if tuple.len() == 2 {
                            if let (Ok(k), Ok(v)) = (
                                tuple.get_item(0).and_then(|i| i.extract::<String>()),
                                tuple.get_item(1).and_then(|i| i.extract::<String>())
                            ) {
                                all_headers.append(k, v);
                            }
                        }
                    }
                }
            }
        }

        // Add Host header from URL if not already set
        if !all_headers.contains("host") && !all_headers.contains("Host") {
            if let Some(host_value) = host_header_value {
                all_headers.set("host".to_string(), host_value);
            }
        }

        request.set_headers(all_headers);

        // Add content
        if let Some(c) = content {
            // Set Content-Length header for the content
            let content_len = c.len();
            request.set_content(c);
            let mut headers_mut = request.headers_ref().clone();
            headers_mut.set("content-length".to_string(), content_len.to_string());
            request.set_headers(headers_mut);
        } else {
            // For methods that expect a body (POST, PUT, PATCH), add Content-length: 0
            let method_upper = method.to_uppercase();
            if method_upper == "POST" || method_upper == "PUT" || method_upper == "PATCH" {
                let mut headers_mut = request.headers_ref().clone();
                headers_mut.set("content-length".to_string(), "0".to_string());
                request.set_headers(headers_mut);
            }
        }

        Ok(request)
    }

    /// Send a pre-built request
    fn send<'py>(&self, py: Python<'py>, request: Request) -> PyResult<Bound<'py, PyAny>> {
        // If a custom transport is set, use it
        if let Some(ref transport) = self.transport {
            let transport = transport.clone_ref(py);
            let request_clone = request.clone();
            return future_into_py(py, async move {
                Python::with_gil(|py| -> PyResult<Response> {
                    let result = transport.call_method1(py, "handle_async_request", (request_clone.clone(),))?;
                    // Check if it's a coroutine
                    let inspect = py.import("inspect")?;
                    let is_coro = inspect.call_method1("iscoroutine", (result.bind(py),))?.extract::<bool>()?;
                    if is_coro {
                        // If coroutine, we need to await it - but we can't easily do that here
                        // For now, extract directly
                        let mut response = result.extract::<Response>(py)?;
                        response.set_request_attr(Some(request_clone));
                        Ok(response)
                    } else {
                        let mut response = result.extract::<Response>(py)?;
                        response.set_request_attr(Some(request_clone));
                        Ok(response)
                    }
                })
            });
        }

        // For regular HTTP, use async_request
        let method = request.method().to_string();
        let url = request.url_ref().to_string();
        let inner = self.inner.clone();
        let headers = request.headers_ref().clone();
        let content = request.content_bytes().map(|b| b.to_vec());

        future_into_py(py, async move {
            // Build the reqwest request
            let req_method = match method.as_str() {
                "GET" => reqwest::Method::GET,
                "POST" => reqwest::Method::POST,
                "PUT" => reqwest::Method::PUT,
                "DELETE" => reqwest::Method::DELETE,
                "HEAD" => reqwest::Method::HEAD,
                "OPTIONS" => reqwest::Method::OPTIONS,
                "PATCH" => reqwest::Method::PATCH,
                _ => reqwest::Method::GET,
            };

            let mut req_builder = inner.request(req_method, &url);

            // Add headers
            for (k, v) in headers.inner() {
                req_builder = req_builder.header(k.as_str(), v.as_str());
            }

            // Add content if present
            if let Some(body) = content {
                req_builder = req_builder.body(body);
            }

            let response = req_builder.send().await.map_err(convert_reqwest_error)?;
            let (status, response_headers, version) = (
                response.status().as_u16(),
                response.headers().clone(),
                format!("{:?}", response.version()),
            );
            let url_str = response.url().to_string();
            let content = response.bytes().await.map_err(convert_reqwest_error)?;

            // Build response
            let mut resp = Response::new(status);
            resp.set_content(content.to_vec());
            // Convert headers
            let mut resp_headers = Headers::new();
            for (k, v) in response_headers.iter() {
                if let Ok(v_str) = v.to_str() {
                    resp_headers.set(k.as_str().to_string(), v_str.to_string());
                }
            }
            resp.set_headers(resp_headers);
            resp.set_url(URL::new_impl(Some(&url_str), None, None, None, None, None, None, None, None, None, None, None)?);
            resp.set_http_version(version);
            resp.set_request_attr(Some(request));
            Ok(resp)
        })
    }

    fn __aenter__<'py>(slf: PyRef<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let slf_obj = slf.into_pyobject(py)?.unbind();
        future_into_py(py, async move {
            Ok(slf_obj)
        })
    }

    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            Ok(false)
        })
    }

    /// Get event_hooks as a dict
    #[getter]
    fn event_hooks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);

        let request_list = PyList::new(py, self.event_hooks.request.iter().map(|h| h.bind(py)))?;
        let response_list = PyList::new(py, self.event_hooks.response.iter().map(|h| h.bind(py)))?;

        dict.set_item("request", request_list)?;
        dict.set_item("response", response_list)?;

        Ok(dict)
    }

    /// Set event_hooks from a dict
    #[setter]
    fn set_event_hooks(&mut self, hooks: &Bound<'_, PyDict>) -> PyResult<()> {
        self.event_hooks = EventHooks::default();

        if let Some(request_hooks) = hooks.get_item("request")? {
            if let Ok(list) = request_hooks.downcast::<PyList>() {
                for item in list.iter() {
                    self.event_hooks.request.push(item.unbind());
                }
            }
        }
        if let Some(response_hooks) = hooks.get_item("response")? {
            if let Ok(list) = response_hooks.downcast::<PyList>() {
                for item in list.iter() {
                    self.event_hooks.response.push(item.unbind());
                }
            }
        }

        Ok(())
    }

    #[getter]
    fn trust_env(&self) -> bool {
        self.trust_env
    }

    #[setter]
    fn set_trust_env(&mut self, value: bool) {
        self.trust_env = value;
    }

    /// Get client-level auth
    #[getter]
    fn auth(&self) -> Option<BasicAuth> {
        self.auth.as_ref().map(|(user, pass)| {
            BasicAuth {
                username: user.clone(),
                password: pass.clone(),
            }
        })
    }

    /// Set client-level auth
    #[setter]
    fn set_auth(&mut self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        if value.is_none() {
            self.auth = None;
        } else if let Ok(basic) = value.extract::<BasicAuth>() {
            self.auth = Some((basic.username, basic.password));
        } else if let Ok(tuple) = value.extract::<(String, String)>() {
            self.auth = Some(tuple);
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "auth must be a tuple (username, password) or BasicAuth object",
            ));
        }
        Ok(())
    }

    /// Mount a transport for a given URL pattern
    fn mount(&mut self, pattern: &str, transport: Py<PyAny>) {
        self.mounts.insert(pattern.to_string(), transport);
    }

    fn __repr__(&self) -> String {
        "<AsyncClient>".to_string()
    }

    /// Get the default transport
    #[getter]
    fn _transport<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        if let Some(ref t) = self.transport {
            Ok(t.bind(py).clone())
        } else if let Some(ref t) = self.default_transport {
            Ok(t.bind(py).clone())
        } else {
            // This shouldn't happen if initialized properly
            let transport_module = py.import("requestx")?;
            let http_transport = transport_module.getattr("AsyncHTTPTransport")?;
            let transport = http_transport.call0()?;
            Ok(transport)
        }
    }

    /// Get the transport for a given URL, considering mounts
    fn _transport_for_url<'py>(&self, py: Python<'py>, url: &URL) -> PyResult<Bound<'py, PyAny>> {
        let url_str = url.to_string();

        // Check mounts in order of specificity (longer patterns first)
        let mut sorted_patterns: Vec<_> = self.mounts.keys().collect();
        sorted_patterns.sort_by(|a, b| b.len().cmp(&a.len()));

        for pattern in sorted_patterns {
            if Self::url_matches_pattern_static(&url_str, pattern) {
                if let Some(transport) = self.mounts.get(pattern) {
                    return Ok(transport.bind(py).clone());
                }
            }
        }

        // Return default transport
        self._transport(py)
    }
}

impl AsyncClient {
    fn async_request<'py>(
        &self,
        py: Python<'py>,
        method: String,
        url: String,
        content: Option<Vec<u8>>,
        data: Option<PyObject>,
        json: Option<PyObject>,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let default_headers = self.headers.clone();
        let default_cookies = self.cookies.clone();
        let base_url = self.base_url.clone();

        // Resolve URL
        let resolved_url = if let Some(base) = &base_url {
            if !url.contains("://") {
                base.join_url(&url)?.to_string()
            } else {
                url.clone()
            }
        } else {
            url.clone()
        };

        // Process params
        let final_url = if let Some(p) = &params {
            Python::with_gil(|py| {
                let p_bound = p.bind(py);
                let qp = crate::queryparams::QueryParams::from_py(p_bound)?;
                let qs = qp.to_query_string();
                if qs.is_empty() {
                    Ok::<String, PyErr>(resolved_url.clone())
                } else if resolved_url.contains('?') {
                    Ok(format!("{}&{}", resolved_url, qs))
                } else {
                    Ok(format!("{}?{}", resolved_url, qs))
                }
            })?
        } else {
            resolved_url.clone()
        };

        // Build headers for request
        let mut request_headers = default_headers.clone();
        if let Some(h) = &headers {
            Python::with_gil(|py| {
                let h_bound = h.bind(py);
                if let Ok(headers_obj) = h_bound.extract::<Headers>() {
                    for (k, v) in headers_obj.inner() {
                        request_headers.set(k.clone(), v.clone());
                    }
                } else if let Ok(dict) = h_bound.downcast::<PyDict>() {
                    for (key, value) in dict.iter() {
                        if let (Ok(k), Ok(v)) = (key.extract::<String>(), value.extract::<String>()) {
                            request_headers.set(k, v);
                        }
                    }
                }
            });
        }

        // Add cookies to headers
        let cookie_header = default_cookies.to_header_value();
        if !cookie_header.is_empty() {
            request_headers.set("Cookie".to_string(), cookie_header);
        }

        // Process body
        let body_content = if let Some(c) = content {
            Some(c)
        } else if let Some(j) = &json {
            let json_str = Python::with_gil(|py| {
                let j_bound = j.bind(py);
                py_to_json_string(j_bound)
            })?;
            if !request_headers.contains("content-type") {
                request_headers.set("Content-Type".to_string(), "application/json".to_string());
            }
            Some(json_str.into_bytes())
        } else if let Some(d) = &data {
            Python::with_gil(|py| {
                let d_bound = d.bind(py);
                if let Ok(dict) = d_bound.downcast::<PyDict>() {
                    let mut form_data = Vec::new();
                    for (key, value) in dict.iter() {
                        if let (Ok(k), Ok(v)) = (key.extract::<String>(), value.extract::<String>()) {
                            form_data.push(format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)));
                        }
                    }
                    if !request_headers.contains("content-type") {
                        request_headers.set("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
                    }
                    Ok::<Option<Vec<u8>>, PyErr>(Some(form_data.join("&").into_bytes()))
                } else {
                    Ok(None)
                }
            })?
        } else {
            None
        };

        // Process auth - add Authorization header (per-request auth takes precedence over client-level auth)
        // Auth handling - four cases (handled via Python wrapper with sentinels):
        // 1. auth=USE_CLIENT_DEFAULT (_AuthUnset sentinel) → use client auth
        // 2. auth=None explicitly (_AuthDisabled sentinel) → disable auth
        // 3. auth=(user,pass) or BasicAuth → use Basic auth
        // 4. auth=callable → call it with Request to modify headers
        enum AuthAction {
            UseClientAuth,
            DisableAuth,
            BasicAuth(String, String),
            CallableAuth(Py<PyAny>),
        }

        let auth_action = if let Some(a) = &auth {
            Python::with_gil(|py| {
                let a_bound = a.bind(py);
                // Check type name for sentinels
                if let Ok(type_name) = a_bound.get_type().name() {
                    let type_str = type_name.to_string();
                    // _AuthUnset sentinel - use client auth
                    if type_str == "_AuthUnset" {
                        return AuthAction::UseClientAuth;
                    }
                    // _AuthDisabled sentinel - disable auth
                    if type_str == "_AuthDisabled" {
                        return AuthAction::DisableAuth;
                    }
                }
                // Check if it's Python's None
                if a_bound.is_none() {
                    AuthAction::DisableAuth
                } else if let Ok(basic) = a_bound.extract::<BasicAuth>() {
                    AuthAction::BasicAuth(basic.username, basic.password)
                } else if let Ok(tuple) = a_bound.extract::<(String, String)>() {
                    AuthAction::BasicAuth(tuple.0, tuple.1)
                } else if a_bound.is_callable() {
                    // Callable auth - will call it with Request later
                    AuthAction::CallableAuth(a.clone_ref(py))
                } else {
                    // Unknown auth type, disable auth
                    AuthAction::DisableAuth
                }
            })
        } else {
            // No per-request auth specified (Rust None), fall back to client-level auth
            AuthAction::UseClientAuth
        };

        // Apply auth based on action
        let callable_auth: Option<Py<PyAny>> = match auth_action {
            AuthAction::UseClientAuth => {
                if let Some((username, password)) = &self.auth {
                    let credentials = format!("{}:{}", username, password);
                    let encoded = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        credentials.as_bytes(),
                    );
                    request_headers.set("Authorization".to_string(), format!("Basic {}", encoded));
                }
                None
            }
            AuthAction::DisableAuth => None,
            AuthAction::BasicAuth(username, password) => {
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    credentials.as_bytes(),
                );
                request_headers.set("Authorization".to_string(), format!("Basic {}", encoded));
                None
            }
            AuthAction::CallableAuth(auth_fn) => Some(auth_fn),
        };

        // Clone transport outside the borrow so the clone lives beyond &self
        let transport_opt: Option<Py<PyAny>> = self.transport.as_ref().map(|t| t.clone_ref(py));

        // If a custom transport is set, use it instead of making HTTP requests
        if let Some(transport) = transport_opt {
            // Parse URL for host header and userinfo extraction
            let url_obj = URL::parse(&final_url)?;
            let host_header = Self::get_host_header(&url_obj);

            // Extract auth from URL userinfo if no auth was already set
            if !request_headers.contains("authorization") {
                let url_username = url_obj.get_username();
                if !url_username.is_empty() {
                    let url_password = url_obj.get_password().unwrap_or_default();
                    let credentials = format!("{}:{}", url_username, url_password);
                    let encoded = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        credentials.as_bytes(),
                    );
                    request_headers.set("authorization".to_string(), format!("Basic {}", encoded));
                }
            }

            // Add Host header if not already present
            if !request_headers.contains("host") {
                request_headers.set("host".to_string(), host_header);
            }

            // Build the Request object
            let mut request = Request::new(&method, url_obj);
            request.set_headers(request_headers);
            if let Some(ref body) = body_content {
                request.set_content(body.clone());
            }

            // Apply callable auth if provided - it modifies the request in place
            if let Some(ref auth_fn) = callable_auth {
                let auth_fn_bound = auth_fn.bind(py);
                let modified_request = auth_fn_bound.call1((request.clone(),))?;
                // The auth function returns a modified Request
                if let Ok(req) = modified_request.extract::<Request>() {
                    request = req;
                }
            }

            // Call the transport's handle_async_request method (for async handlers)
            // or handle_request method (for sync handlers)
            let request_clone = request.clone();

            // Check if transport has handle_async_request (works with async handlers)
            let has_async_handler = transport.bind(py).hasattr("handle_async_request")?;

            if has_async_handler {
                // Use handle_async_request which can handle both sync and async handlers
                let transport_bound = transport.bind(py);
                let coro = transport_bound.call_method1("handle_async_request", (request_clone.clone(),))?;

                // Convert the coroutine to a Rust future and await it
                return pyo3_async_runtimes::tokio::into_future(coro).map(|fut| {
                    pyo3_async_runtimes::tokio::future_into_py(py, async move {
                        let response = fut.await?;
                        Python::with_gil(|py| {
                            let mut resp = response.extract::<Response>(py)?;
                            resp.set_request_attr(Some(request_clone));
                            Ok(resp)
                        })
                    })
                })?;
            }

            // Fall back to handle_request for sync-only transports
            return future_into_py(py, async move {
                Python::with_gil(|py| -> PyResult<Response> {
                    let transport_bound: &Bound<'_, PyAny> = transport.bind(py);

                    // Try handle_request (for MockTransport with sync handlers)
                    if transport_bound.hasattr("handle_request")? {
                        let result = transport_bound.call_method1("handle_request", (request_clone.clone(),))?;
                        let mut response = result.extract::<Response>()?;
                        response.set_request_attr(Some(request_clone));
                        return Ok(response);
                    }

                    // If it's a callable (Python function), call it directly
                    if transport_bound.is_callable() {
                        let result = transport_bound.call1((request_clone.clone(),))?;
                        let mut response = result.extract::<Response>()?;
                        response.set_request_attr(Some(request_clone));
                        return Ok(response);
                    }

                    Err(pyo3::exceptions::PyTypeError::new_err(
                        "Transport must have handle_request method or be callable",
                    ))
                })
            });
        }

        // Standard HTTP request path using reqwest
        let client = self.inner.clone();
        let method_clone = method.clone();
        let url_clone = final_url.clone();

        // Convert Headers to reqwest::header::HeaderMap
        let mut all_headers = reqwest::header::HeaderMap::new();
        for (k, v) in request_headers.inner() {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                all_headers.insert(name, val);
            }
        }

        future_into_py(py, async move {
            let method = reqwest::Method::from_bytes(method_clone.as_bytes())
                .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid HTTP method"))?;

            let mut builder = client.request(method.clone(), &url_clone);
            builder = builder.headers(all_headers);

            if let Some(b) = body_content {
                builder = builder.body(b);
            }

            let start = std::time::Instant::now();
            let response = builder.send().await.map_err(convert_reqwest_error)?;
            let elapsed = start.elapsed();

            let request = Request::new(method.as_str(), URL::parse(&url_clone)?);
            let mut result = Response::from_reqwest_async(response, Some(request)).await?;
            result.set_elapsed(elapsed);
            Ok(result)
        })
    }
}

impl AsyncClient {
    /// Get the host header value for a URL (without userinfo, port only if non-default)
    fn get_host_header(url: &URL) -> String {
        let host = url.get_host_str();
        let port = url.get_port();
        let scheme = url.get_scheme();

        // Only include port if non-default
        let default_port = match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => 0,
        };

        if let Some(p) = port {
            if p != default_port {
                return format!("{}:{}", host, p);
            }
        }
        host
    }

    /// Check if a URL matches a mount pattern
    fn url_matches_pattern_static(url: &str, pattern: &str) -> bool {
        // Mount patterns can be:
        // - "all://" - matches all URLs
        // - "http://" - matches all HTTP URLs
        // - "https://" - matches all HTTPS URLs
        // - "http://example.com" - matches specific domain (any port)
        // - "http://example.com:8080" - matches specific domain and port
        // - "http://*.example.com" - matches subdomains only (not example.com itself)
        // - "http://*example.com" - matches domain suffix (example.com and www.example.com)
        // - "http://*" - matches any domain with http scheme
        // - "all://example.com" - matches domain on any scheme

        if pattern == "all://" {
            return true;
        }

        // Parse the URL scheme
        let url_scheme = url.split("://").next().unwrap_or("");
        let pattern_scheme = pattern.split("://").next().unwrap_or("");

        // Check scheme match (unless pattern scheme is "all")
        if pattern_scheme != "all" && pattern_scheme != url_scheme {
            return false;
        }

        // Get the URL host (with port)
        let url_host = if let Some(rest) = url.strip_prefix(&format!("{}://", url_scheme)) {
            rest.split('/').next().unwrap_or("")
        } else {
            ""
        };

        // Get the pattern host (with port if specified)
        let pattern_host = if let Some(rest) = pattern.strip_prefix(&format!("{}://", pattern_scheme)) {
            rest.split('/').next().unwrap_or("")
        } else {
            ""
        };

        // If pattern is just scheme://, match all hosts
        if pattern_host.is_empty() {
            return true;
        }

        // Handle "*" pattern - matches any host
        if pattern_host == "*" {
            return true;
        }

        // Split into host and port
        let url_host_no_port = url_host.split(':').next().unwrap_or(url_host);
        let url_port = url_host.split(':').nth(1);
        let pattern_host_no_port = pattern_host.split(':').next().unwrap_or(pattern_host);
        let pattern_port = pattern_host.split(':').nth(1);

        // Handle "*.example.com" pattern - matches subdomains ONLY (NOT example.com itself)
        if pattern_host_no_port.starts_with("*.") {
            let suffix = &pattern_host_no_port[2..]; // Remove "*."
            // Must have a dot before the suffix (i.e., must be a subdomain)
            // "*.example.com" matches "www.example.com" but NOT "example.com"
            if url_host_no_port.ends_with(&format!(".{}", suffix)) {
                return Self::port_matches(url_port, pattern_port);
            }
            return false;
        }

        // Handle "*example.com" pattern (no dot) - matches suffix
        // e.g., "*example.com" matches "example.com" and "www.example.com" but NOT "wwwexample.com"
        if pattern_host_no_port.starts_with('*') && !pattern_host_no_port.starts_with("*.") {
            let suffix = &pattern_host_no_port[1..]; // Remove "*"
            // Must either be exact match or have a dot before suffix
            if url_host_no_port == suffix {
                return Self::port_matches(url_port, pattern_port);
            }
            if url_host_no_port.ends_with(&format!(".{}", suffix)) {
                return Self::port_matches(url_port, pattern_port);
            }
            return false;
        }

        // Exact host match
        if url_host_no_port != pattern_host_no_port {
            return false;
        }

        // If pattern has a port, URL must have matching port
        // If pattern has no port, any port matches
        Self::port_matches(url_port, pattern_port)
    }

    /// Check if URL port matches pattern port
    fn port_matches(url_port: Option<&str>, pattern_port: Option<&str>) -> bool {
        match pattern_port {
            None => true,  // Pattern has no port requirement
            Some(pp) => url_port == Some(pp),  // Port must match exactly
        }
    }
}

/// Convert Python object to JSON string
/// Uses Python's json module for serialization to preserve dict insertion order
/// and match httpx's default behavior (ensure_ascii=False, allow_nan=False, compact)
fn py_to_json_string(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let py = obj.py();
    let json_mod = py.import("json")?;

    // Use httpx's default JSON settings:
    // - ensure_ascii=False (allows non-ASCII characters)
    // - allow_nan=False (raises ValueError for NaN/Inf)
    // - separators=(',', ':') (compact representation)
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("ensure_ascii", false)?;
    kwargs.set_item("allow_nan", false)?;
    let separators = pyo3::types::PyTuple::new(py, [",", ":"])?;
    kwargs.set_item("separators", separators)?;

    let result = json_mod.call_method("dumps", (obj,), Some(&kwargs))?;
    result.extract::<String>()
}

/// Convert Python object to sonic_rs::Value
fn py_to_json_value(obj: &Bound<'_, PyAny>) -> PyResult<sonic_rs::Value> {
    use pyo3::types::{PyBool, PyFloat, PyInt, PyList, PyString};

    if obj.is_none() {
        return Ok(sonic_rs::Value::default());
    }

    if let Ok(b) = obj.downcast::<PyBool>() {
        return Ok(sonic_rs::json!(b.is_true()));
    }

    if let Ok(i) = obj.downcast::<PyInt>() {
        let val: i64 = i.extract()?;
        return Ok(sonic_rs::json!(val));
    }

    if let Ok(f) = obj.downcast::<PyFloat>() {
        let val: f64 = f.extract()?;
        return Ok(sonic_rs::json!(val));
    }

    if let Ok(s) = obj.downcast::<PyString>() {
        let val: String = s.extract()?;
        return Ok(sonic_rs::json!(val));
    }

    if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(py_to_json_value(&item)?);
        }
        return Ok(sonic_rs::Value::from(arr));
    }

    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut obj_map = sonic_rs::Object::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            let value = py_to_json_value(&v)?;
            obj_map.insert(&key, value);
        }
        return Ok(sonic_rs::Value::from(obj_map));
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "Unsupported type for JSON serialization",
    ))
}

/// Async stream context manager for client.stream()
#[pyclass(name = "AsyncStreamContextManager")]
pub struct AsyncStreamContextManager {
    client: Py<AsyncClient>,
    method: String,
    url: String,
    content: Option<Vec<u8>>,
    data: Option<PyObject>,
    json: Option<PyObject>,
    params: Option<PyObject>,
    headers: Option<PyObject>,
    cookies: Option<PyObject>,
    auth: Option<PyObject>,
    follow_redirects: Option<bool>,
    timeout: Option<PyObject>,
    response: Option<Response>,
}

#[pymethods]
impl AsyncStreamContextManager {
    fn __aenter__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();

        // Extract all values first before borrowing the client
        let method = slf.method.clone();
        let url = slf.url.clone();
        let content = slf.content.take();
        let data = slf.data.take();
        let json = slf.json.take();
        let params = slf.params.take();
        let headers = slf.headers.take();
        let cookies = slf.cookies.take();
        let auth = slf.auth.take();
        let follow_redirects = slf.follow_redirects;
        let timeout = slf.timeout.take();

        // Now get client reference
        let client = slf.client.bind(py);

        // Call the Python-level request method
        let kwargs = PyDict::new(py);
        if let Some(c) = content {
            kwargs.set_item("content", c)?;
        }
        if let Some(d) = data {
            kwargs.set_item("data", d)?;
        }
        if let Some(j) = json {
            kwargs.set_item("json", j)?;
        }
        if let Some(p) = params {
            kwargs.set_item("params", p)?;
        }
        if let Some(h) = headers {
            kwargs.set_item("headers", h)?;
        }
        if let Some(c) = cookies {
            kwargs.set_item("cookies", c)?;
        }
        if let Some(a) = auth {
            kwargs.set_item("auth", a)?;
        }
        if let Some(f) = follow_redirects {
            kwargs.set_item("follow_redirects", f)?;
        }
        if let Some(t) = timeout {
            kwargs.set_item("timeout", t)?;
        }

        // Call client.request(method, url, **kwargs)
        client.call_method("request", (method, url), Some(&kwargs))
    }

    fn __aexit__<'py>(
        &mut self,
        py: Python<'py>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            Ok(false)
        })
    }
}

impl AsyncClient {
    /// Clone the client for use in stream context manager
    fn clone_for_stream(&self, py: Python<'_>) -> PyResult<Py<AsyncClient>> {
        // Clone mounts manually since Py<PyAny> requires clone_ref
        let mut mounts_clone = HashMap::new();
        for (k, v) in &self.mounts {
            mounts_clone.insert(k.clone(), v.clone_ref(py));
        }

        let client = AsyncClient {
            inner: self.inner.clone(),
            base_url: self.base_url.clone(),
            headers: self.headers.clone(),
            cookies: self.cookies.clone(),
            timeout: self.timeout.clone(),
            follow_redirects: self.follow_redirects,
            max_redirects: self.max_redirects,
            event_hooks: EventHooks::default(),
            trust_env: self.trust_env,
            mounts: mounts_clone,
            transport: self.transport.as_ref().map(|t| t.clone_ref(py)),
            default_transport: self.default_transport.as_ref().map(|t| t.clone_ref(py)),
            auth: self.auth.clone(),
        };
        Py::new(py, client)
    }
}
