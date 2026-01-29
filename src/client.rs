//! Synchronous HTTP Client implementation

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

use crate::cookies::Cookies;
use crate::exceptions::convert_reqwest_error;
use crate::headers::Headers;
use crate::multipart::{build_multipart_body, build_multipart_body_with_boundary, extract_boundary_from_content_type};
use crate::request::Request;
use crate::response::Response;
use crate::timeout::Timeout;
use crate::types::BasicAuth;
use crate::url::URL;

/// Event hooks storage
#[derive(Default)]
struct EventHooks {
    request: Vec<Py<PyAny>>,
    response: Vec<Py<PyAny>>,
}

/// Synchronous HTTP Client
#[pyclass(name = "Client")]
pub struct Client {
    inner: reqwest::blocking::Client,
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
}

impl Default for Client {
    fn default() -> Self {
        Self::new_impl(None, None, None, None, None, None, None).unwrap()
    }
}

impl Client {
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

        let mut builder = reqwest::blocking::Client::builder()
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

        Ok(Self {
            inner: client,
            base_url,
            headers: headers.unwrap_or_default(),
            cookies: cookies.unwrap_or_default(),
            timeout,
            follow_redirects,
            max_redirects,
            event_hooks: EventHooks::default(),
            trust_env: true,
            mounts: HashMap::new(),
            transport: None,
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

    /// Extract a string URL from a &str or URL object
    fn url_to_string(url: &Bound<'_, PyAny>) -> PyResult<String> {
        // Try to extract as string first
        if let Ok(s) = url.extract::<String>() {
            return Ok(s);
        }
        // Try to extract as URL object
        if let Ok(url_obj) = url.extract::<URL>() {
            return Ok(url_obj.to_string());
        }
        // Try calling str() on the object
        let s = url.str()?.to_string();
        Ok(s)
    }

    pub fn execute_request(
        &self,
        py: Python<'_>,
        method: &str,
        url: &str,
        content: Option<Vec<u8>>,
        data: Option<&Bound<'_, PyDict>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        let resolved_url = self.resolve_url(url)?;

        // Build URL with params
        let final_url = if let Some(p) = params {
            let qp = crate::queryparams::QueryParams::from_py(p)?;
            let qs = qp.to_query_string();
            if qs.is_empty() {
                resolved_url
            } else if resolved_url.contains('?') {
                format!("{}&{}", resolved_url, qs)
            } else {
                format!("{}?{}", resolved_url, qs)
            }
        } else {
            resolved_url
        };

        // If a custom transport is set, use it instead of making HTTP requests
        if let Some(ref transport) = self.transport {
            // Build the Request object with all the headers and body
            let mut request_headers = self.headers.clone();
            if let Some(h) = headers {
                if let Ok(headers_obj) = h.extract::<Headers>() {
                    for (k, v) in headers_obj.inner() {
                        request_headers.set(k.clone(), v.clone());
                    }
                } else if let Ok(dict) = h.downcast::<PyDict>() {
                    for (key, value) in dict.iter() {
                        let k: String = key.extract()?;
                        let v: String = value.extract()?;
                        request_headers.set(k, v);
                    }
                }
            }

            // Add cookies to headers
            let mut all_cookies = self.cookies.clone();
            if let Some(c) = cookies {
                if let Ok(cookies_obj) = c.extract::<Cookies>() {
                    for (k, v) in cookies_obj.inner() {
                        all_cookies.set(k, v);
                    }
                }
            }
            let cookie_header = all_cookies.to_header_value();
            if !cookie_header.is_empty() {
                request_headers.set("Cookie".to_string(), cookie_header);
            }

            // Check if we need multipart encoding (files provided)
            let (body_content, content_type) = if files.is_some() {
                // Check if boundary was already set in headers BEFORE reading files
                let existing_ct = request_headers.get("content-type", None);

                let (body, content_type) = if let Some(ref ct) = existing_ct {
                    if ct.contains("boundary=") {
                        // Extract boundary from existing header and use it
                        let boundary_str = extract_boundary_from_content_type(ct);
                        if let Some(b) = boundary_str {
                            let (body, _) = build_multipart_body_with_boundary(py, data, files, &b)?;
                            (body, ct.clone())
                        } else {
                            // Invalid boundary format, use auto-generated
                            let (body, boundary) = build_multipart_body(py, data, files)?;
                            (body, format!("multipart/form-data; boundary={}", boundary))
                        }
                    } else {
                        // Content-Type set but no boundary - use content-type as is (will auto-generate boundary in body)
                        let (body, boundary) = build_multipart_body(py, data, files)?;
                        // Keep the existing content-type but we generated body with auto boundary
                        // This case is when user sets content-type without boundary - we keep their content-type
                        (body, ct.clone())
                    }
                } else {
                    // No Content-Type set, use auto-generated boundary
                    let (body, boundary) = build_multipart_body(py, data, files)?;
                    (body, format!("multipart/form-data; boundary={}", boundary))
                };

                (Some(body), Some(content_type))
            } else if let Some(c) = content {
                (Some(c), None)
            } else if let Some(d) = data {
                let mut form_data = Vec::new();
                for (key, value) in d.iter() {
                    let k: String = key.extract()?;
                    // Handle both string and bytes values
                    let v: String = if let Ok(s) = value.extract::<String>() {
                        s
                    } else if let Ok(b) = value.extract::<Vec<u8>>() {
                        String::from_utf8_lossy(&b).to_string()
                    } else {
                        value.str()?.to_string()
                    };
                    form_data.push(format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)));
                }
                let ct = if !request_headers.contains("content-type") {
                    Some("application/x-www-form-urlencoded".to_string())
                } else {
                    None
                };
                (Some(form_data.join("&").into_bytes()), ct)
            } else if let Some(j) = json {
                let json_str = py_to_json_string(j)?;
                let ct = if !request_headers.contains("content-type") {
                    Some("application/json".to_string())
                } else {
                    None
                };
                (Some(json_str.into_bytes()), ct)
            } else {
                (None, None)
            };

            if let Some(ct) = content_type {
                request_headers.set("Content-Type".to_string(), ct);
            }

            let mut request = Request::new(method, URL::parse(&final_url)?);
            request.set_headers(request_headers);
            if let Some(body) = body_content {
                request.set_content(body);
            }

            // Call the transport's handle_request method
            let response = transport.call_method1(py, "handle_request", (request.clone(),))?;
            let mut response = response.extract::<Response>(py)?;
            // Set the request on the response
            response.set_request_attr(Some(request));
            return Ok(response);
        }

        // Standard HTTP request path
        let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid HTTP method: {}", method))
        })?;

        let mut builder = self.inner.request(method.clone(), &final_url);

        // Add default headers
        for (k, v) in self.headers.inner() {
            builder = builder.header(k.as_str(), v.as_str());
        }

        // Add request-specific headers
        if let Some(h) = headers {
            if let Ok(headers_obj) = h.extract::<Headers>() {
                for (k, v) in headers_obj.inner() {
                    builder = builder.header(k.as_str(), v.as_str());
                }
            } else if let Ok(dict) = h.downcast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    builder = builder.header(k.as_str(), v.as_str());
                }
            }
        }

        // Add cookies
        let mut all_cookies = self.cookies.clone();
        if let Some(c) = cookies {
            if let Ok(cookies_obj) = c.extract::<Cookies>() {
                for (k, v) in cookies_obj.inner() {
                    all_cookies.set(k, v);
                }
            }
        }
        let cookie_header = all_cookies.to_header_value();
        if !cookie_header.is_empty() {
            builder = builder.header("cookie", cookie_header);
        }

        // Add authentication
        if let Some(a) = auth {
            if let Ok(basic) = a.extract::<BasicAuth>() {
                builder = builder.basic_auth(&basic.username, Some(&basic.password));
            } else if let Ok(tuple) = a.extract::<(String, String)>() {
                builder = builder.basic_auth(&tuple.0, Some(&tuple.1));
            }
        }

        // Add body
        if let Some(c) = content {
            builder = builder.body(c);
        } else if let Some(d) = data {
            // Form data
            let mut form_data = Vec::new();
            for (key, value) in d.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                form_data.push((k, v));
            }
            builder = builder.form(&form_data);
        } else if let Some(j) = json {
            let json_str = py_to_json_string(j)?;
            builder = builder
                .header("content-type", "application/json")
                .body(json_str);
        }

        // Create request object for response
        let request = Request::new(method.as_str(), URL::parse(&final_url)?);

        // Execute request (release GIL during I/O)
        let response = py.allow_threads(|| {
            builder.send()
        }).map_err(convert_reqwest_error)?;

        Response::from_reqwest(response, Some(request))
    }
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (*, auth=None, cookies=None, headers=None, timeout=None, follow_redirects=None, max_redirects=None, base_url=None, event_hooks=None, trust_env=None, transport=None, **_kwargs))]
    fn new(
        py: Python<'_>,
        auth: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        max_redirects: Option<usize>,
        base_url: Option<&str>,
        event_hooks: Option<&Bound<'_, PyDict>>,
        trust_env: Option<bool>,
        transport: Option<Py<PyAny>>,
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
            Some(URL::parse(url)?)
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

        Ok(client)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn get(
        &self,
        py: Python<'_>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, "GET", &url_str, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn post(
        &self,
        py: Python<'_>,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<&Bound<'_, PyDict>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, "POST", &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn put(
        &self,
        py: Python<'_>,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<&Bound<'_, PyDict>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, "PUT", &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn patch(
        &self,
        py: Python<'_>,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<&Bound<'_, PyDict>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, "PATCH", &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn delete(
        &self,
        py: Python<'_>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, "DELETE", &url_str, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn head(
        &self,
        py: Python<'_>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, "HEAD", &url_str, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn options(
        &self,
        py: Python<'_>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, "OPTIONS", &url_str, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
    }

    #[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn request(
        &self,
        py: Python<'_>,
        method: &str,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<&Bound<'_, PyDict>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, method, &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
    }

    #[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn stream(
        &self,
        py: Python<'_>,
        method: &str,
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<&Bound<'_, PyDict>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Response> {
        // For now, stream behaves the same as request
        let url_str = Self::url_to_string(url)?;
        self.execute_request(py, method, &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
    }

    fn send(&self, py: Python<'_>, request: &Request) -> PyResult<Response> {
        self.execute_request(
            py,
            request.method(),
            &request.url_ref().to_string(),
            request.content_bytes().map(|b| b.to_vec()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None))]
    fn build_request(
        &self,
        method: &str,
        url: &str,
        content: Option<Vec<u8>>,
        data: Option<&Bound<'_, PyDict>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Request> {
        let resolved_url = self.resolve_url(url)?;
        let parsed_url = URL::new_impl(Some(&resolved_url), None, None, None, None, None, None, None, None, params, None, None)?;
        let mut request = Request::new(method, parsed_url);

        // Add headers
        let mut all_headers = self.headers.clone();
        if let Some(h) = headers {
            if let Ok(headers_obj) = h.extract::<Headers>() {
                for (k, v) in headers_obj.inner() {
                    all_headers.set(k.clone(), v.clone());
                }
            }
        }
        request.set_headers(all_headers);

        // Add content
        if let Some(c) = content {
            request.set_content(c);
        }

        Ok(request)
    }

    fn close(&self) {
        // Client doesn't need explicit close in reqwest
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

    /// Mount a transport for a given URL pattern
    fn mount(&mut self, pattern: &str, transport: Py<PyAny>) {
        self.mounts.insert(pattern.to_string(), transport);
    }

    fn __repr__(&self) -> String {
        "<Client>".to_string()
    }
}

/// Convert Python object to JSON string
fn py_to_json_string(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = py_to_json_value(obj)?;
    sonic_rs::to_string(&value).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("JSON serialization error: {}", e))
    })
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
