//! Synchronous HTTP Client implementation

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

use crate::cookies::Cookies;
use crate::exceptions::convert_reqwest_error;
use crate::headers::Headers;
use crate::multipart::{build_multipart_body, build_multipart_body_with_boundary, extract_boundary_from_content_type};
use crate::request::{Request, py_value_to_form_str};
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
    /// Cached default transport - created lazily and reused
    default_transport: Option<Py<PyAny>>,
    /// Client-level auth
    auth: Option<(String, String)>,
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
            inner: client,
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
                } else if let Ok(list) = h.downcast::<PyList>() {
                    // Handle list of tuples (for repeated headers)
                    for item in list.iter() {
                        let tuple = item.downcast::<pyo3::types::PyTuple>()?;
                        let k: String = tuple.get_item(0)?.extract()?;
                        let v: String = tuple.get_item(1)?.extract()?;
                        // For repeated headers, we need to append not replace
                        request_headers.append(k, v);
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

            // Apply auth - three cases (handled via Python wrapper with sentinels):
            // 1. auth=USE_CLIENT_DEFAULT (_AuthUnset sentinel) → use client auth
            // 2. auth=None explicitly (_AuthDisabled sentinel) → disable auth
            // 3. auth=(user,pass) → use this auth
            let effective_auth: Option<(String, String)> = if let Some(a) = auth {
                // Check type name for sentinels
                if let Ok(type_name) = a.get_type().name() {
                    let type_str = type_name.to_string();
                    // _AuthUnset sentinel - use client auth
                    if type_str == "_AuthUnset" {
                        self.auth.clone()
                    // _AuthDisabled sentinel - disable auth
                    } else if type_str == "_AuthDisabled" {
                        None
                    } else if let Ok(basic) = a.extract::<BasicAuth>() {
                        Some((basic.username, basic.password))
                    } else if let Ok(tuple) = a.extract::<(String, String)>() {
                        Some(tuple)
                    } else {
                        None
                    }
                } else if let Ok(basic) = a.extract::<BasicAuth>() {
                    Some((basic.username, basic.password))
                } else if let Ok(tuple) = a.extract::<(String, String)>() {
                    Some(tuple)
                } else {
                    None
                }
            } else {
                // No per-request auth specified, fall back to client-level auth
                self.auth.clone()
            };

            // Build default headers that httpx sets
            let url_obj = URL::parse(&final_url)?;
            let host_header = Self::get_host_header(&url_obj);
            let version = env!("CARGO_PKG_VERSION");

            // Determine final auth - either from effective_auth, or from URL userinfo
            if let Some((username, password)) = effective_auth {
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    credentials.as_bytes(),
                );
                request_headers.set("authorization".to_string(), format!("Basic {}", encoded));
            } else {
                // Extract auth from URL userinfo if present
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

            // Only add Host header if not already present (required for HTTP)
            // Other headers (accept, accept-encoding, connection, user-agent) come from
            // client.headers which has defaults set at initialization
            if !request_headers.contains("host") {
                request_headers.set("host".to_string(), host_header);
            }

            let mut request = Request::new(method, url_obj);
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

        // Add authentication - three cases (handled via Python wrapper with sentinels):
        // 1. auth=USE_CLIENT_DEFAULT (_AuthUnset sentinel) → use client auth
        // 2. auth=None explicitly (_AuthDisabled sentinel) → disable auth
        // 3. auth=(user,pass) → use this auth
        let effective_auth: Option<(String, String)> = if let Some(a) = auth {
            // Check type name for sentinels
            if let Ok(type_name) = a.get_type().name() {
                let type_str = type_name.to_string();
                // _AuthUnset sentinel - use client auth
                if type_str == "_AuthUnset" {
                    self.auth.clone()
                // _AuthDisabled sentinel - disable auth
                } else if type_str == "_AuthDisabled" {
                    None
                } else if let Ok(basic) = a.extract::<BasicAuth>() {
                    Some((basic.username, basic.password))
                } else if let Ok(tuple) = a.extract::<(String, String)>() {
                    Some(tuple)
                } else {
                    None
                }
            } else if let Ok(basic) = a.extract::<BasicAuth>() {
                Some((basic.username, basic.password))
            } else if let Ok(tuple) = a.extract::<(String, String)>() {
                Some(tuple)
            } else {
                None
            }
        } else {
            // No per-request auth specified, fall back to client-level auth
            self.auth.clone()
        };

        if let Some((username, password)) = effective_auth {
            builder = builder.basic_auth(&username, Some(&password));
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

        // Execute request (release GIL during I/O) and measure elapsed time
        let start = std::time::Instant::now();
        let response = py.allow_threads(|| {
            builder.send()
        }).map_err(convert_reqwest_error)?;
        let elapsed = start.elapsed();

        let mut result = Response::from_reqwest(response, Some(request))?;
        result.set_elapsed(elapsed);
        Ok(result)
    }
}

#[pymethods]
impl Client {
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
            // Try to extract as Cookies first
            if let Ok(cookies_obj) = c.extract::<Cookies>() {
                Some(cookies_obj)
            } else if let Ok(dict) = c.downcast::<PyDict>() {
                // Handle Python dict
                let mut cookies = Cookies::new();
                for (key, value) in dict.iter() {
                    if let (Ok(k), Ok(v)) = (key.extract::<String>(), value.extract::<String>()) {
                        cookies.set(&k, &v);
                    }
                }
                Some(cookies)
            } else {
                // Try iterating over CookieJar (has __iter__ that yields Cookie objects)
                let mut cookies = Cookies::new();
                let mut found_any = false;
                if let Ok(py_iter) = c.try_iter() {
                    for item in py_iter {
                        if let Ok(cookie) = item {
                            // Cookie object has name and value attributes
                            if let Ok(name) = cookie.getattr("name") {
                                if let Ok(value) = cookie.getattr("value") {
                                    if let (Ok(n), Ok(v)) = (name.extract::<String>(), value.extract::<String>()) {
                                        cookies.set(&n, &v);
                                        found_any = true;
                                    }
                                }
                            }
                        }
                    }
                }
                if found_any {
                    Some(cookies)
                } else {
                    None
                }
            }
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
        let http_transport = if proxy.is_some() {
            crate::transport::HTTPTransport::with_proxy(proxy)?
        } else {
            crate::transport::HTTPTransport::default()
        };
        client.default_transport = Some(Py::new(py, http_transport)?.into_any());

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
        // If a custom transport is set, use it directly with the request
        if let Some(ref transport) = self.transport {
            let response = transport.call_method1(py, "handle_request", (request.clone(),))?;
            let mut response = response.extract::<Response>(py)?;
            response.set_request_attr(Some(request.clone()));
            return Ok(response);
        }

        // For regular HTTP, use execute_request but pass the request's headers
        let headers_bound = pyo3::types::PyDict::new(py);
        for (k, v) in request.headers_ref().inner() {
            headers_bound.set_item(k, v)?;
        }

        self.execute_request(
            py,
            request.method(),
            &request.url_ref().to_string(),
            request.content_bytes().map(|b| b.to_vec()),
            None,
            None,
            None,
            None,
            Some(&headers_bound.as_borrowed()),
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
        url: &Bound<'_, PyAny>,
        content: Option<Vec<u8>>,
        data: Option<&Bound<'_, PyDict>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Request> {
        let url_str = Self::url_to_string(url)?;
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

        // Add cookies to headers
        let mut all_cookies = self.cookies.clone();
        if let Some(c) = cookies {
            if let Ok(cookies_obj) = c.extract::<Cookies>() {
                for (k, v) in cookies_obj.inner() {
                    all_cookies.set(k, v);
                }
            } else if let Ok(dict) = c.downcast::<pyo3::types::PyDict>() {
                for (key, value) in dict.iter() {
                    if let (Ok(k), Ok(v)) = (key.extract::<String>(), value.extract::<String>()) {
                        all_cookies.set(&k, &v);
                    }
                }
            }
        }
        let cookie_header = all_cookies.to_header_value();
        if !cookie_header.is_empty() {
            all_headers.set("cookie".to_string(), cookie_header);
        }

        request.set_headers(all_headers);

        // Handle content
        if let Some(c) = content {
            // Set Content-Length header for the content
            let content_len = c.len();
            request.set_content(c);
            let mut headers_mut = request.headers_ref().clone();
            headers_mut.set("content-length".to_string(), content_len.to_string());
            request.set_headers(headers_mut);
        } else if let Some(j) = json {
            // Handle JSON body
            let py = j.py();
            let json_mod = py.import("json")?;
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("ensure_ascii", false)?;
            kwargs.set_item("allow_nan", false)?;
            let separators = pyo3::types::PyTuple::new(py, [",", ":"])?;
            kwargs.set_item("separators", separators)?;
            let json_str: String = json_mod.call_method("dumps", (j,), Some(&kwargs))?.extract()?;
            let json_bytes = json_str.into_bytes();
            let content_len = json_bytes.len();
            request.set_content(json_bytes);
            let mut headers_mut = request.headers_ref().clone();
            headers_mut.set("content-length".to_string(), content_len.to_string());
            if !headers_mut.contains("content-type") {
                headers_mut.set("content-type".to_string(), "application/json".to_string());
            }
            request.set_headers(headers_mut);
        } else if files.is_some() {
            // Check if files is not empty
            let f = files.unwrap();
            let files_not_empty = if let Ok(dict) = f.downcast::<pyo3::types::PyDict>() {
                !dict.is_empty()
            } else if let Ok(list) = f.downcast::<pyo3::types::PyList>() {
                !list.is_empty()
            } else {
                true  // Unknown type, assume not empty
            };

            if files_not_empty {
                // Handle multipart files (and data)
                let py = f.py();
                let mut headers_mut = request.headers_ref().clone();

                // Check if boundary was already set in headers
                let existing_ct = headers_mut.get("content-type", None);
                let (body, content_type) = if let Some(ref ct) = existing_ct {
                    if ct.contains("boundary=") {
                        let boundary = crate::multipart::extract_boundary_from_content_type(ct);
                        if let Some(b) = boundary {
                            let (body, _) = crate::multipart::build_multipart_body_with_boundary(py, data, Some(&f), &b)?;
                            (body, ct.clone())
                        } else {
                            let (body, boundary) = crate::multipart::build_multipart_body(py, data, Some(&f))?;
                            (body, format!("multipart/form-data; boundary={}", boundary))
                        }
                    } else {
                        // Content-Type set but no boundary - preserve the original
                        let (body, _) = crate::multipart::build_multipart_body(py, data, Some(&f))?;
                        (body, ct.clone())
                    }
                } else {
                    let (body, boundary) = crate::multipart::build_multipart_body(py, data, Some(&f))?;
                    (body, format!("multipart/form-data; boundary={}", boundary))
                };

                let content_len = body.len();
                request.set_content(body);
                headers_mut.set("content-length".to_string(), content_len.to_string());
                headers_mut.set("content-type".to_string(), content_type);
                request.set_headers(headers_mut);
            } else if let Some(d) = data {
                // files was empty, but data might not be - handle form data
                if !d.is_empty() {
                    let mut form_data = Vec::new();
                    for (key, value) in d.iter() {
                        let k: String = key.extract()?;
                        if let Ok(list) = value.downcast::<pyo3::types::PyList>() {
                            for item in list.iter() {
                                let v = py_value_to_form_str(&item)?;
                                form_data.push(format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)));
                            }
                        } else {
                            let v = py_value_to_form_str(&value)?;
                            form_data.push(format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)));
                        }
                    }
                    let body = form_data.join("&").into_bytes();
                    let content_len = body.len();
                    request.set_content(body);
                    let mut headers_mut = request.headers_ref().clone();
                    headers_mut.set("content-length".to_string(), content_len.to_string());
                    if !headers_mut.contains("content-type") {
                        headers_mut.set("content-type".to_string(), "application/x-www-form-urlencoded".to_string());
                    }
                    request.set_headers(headers_mut);
                }
            }
        } else if let Some(d) = data {
            // Handle form data (no files) - only if not empty
            if !d.is_empty() {
                let mut form_data = Vec::new();
                for (key, value) in d.iter() {
                    let k: String = key.extract()?;
                    // Handle lists - create multiple key=value pairs
                    if let Ok(list) = value.downcast::<pyo3::types::PyList>() {
                        for item in list.iter() {
                            let v = py_value_to_form_str(&item)?;
                            form_data.push(format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)));
                        }
                    } else {
                        let v = py_value_to_form_str(&value)?;
                        form_data.push(format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)));
                    }
                }
                let body = form_data.join("&").into_bytes();
                let content_len = body.len();
                request.set_content(body);
                let mut headers_mut = request.headers_ref().clone();
                headers_mut.set("content-length".to_string(), content_len.to_string());
                if !headers_mut.contains("content-type") {
                    headers_mut.set("content-type".to_string(), "application/x-www-form-urlencoded".to_string());
                }
                request.set_headers(headers_mut);
            } else {
                // Empty data dict - set Content-Length: 0 for body methods
                let method_upper = method.to_uppercase();
                if method_upper == "POST" || method_upper == "PUT" || method_upper == "PATCH" {
                    let mut headers_mut = request.headers_ref().clone();
                    headers_mut.set("content-length".to_string(), "0".to_string());
                    request.set_headers(headers_mut);
                }
            }
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

    /// Get base_url
    #[getter]
    fn base_url(&self) -> Option<URL> {
        self.base_url.clone()
    }

    /// Set base_url (ensures trailing slash for paths)
    #[setter]
    fn set_base_url(&mut self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        if value.is_none() {
            self.base_url = None;
        } else {
            let url_str = if let Ok(url) = value.extract::<URL>() {
                url.to_string()
            } else if let Ok(s) = value.extract::<String>() {
                s
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "base_url must be a string or URL object",
                ));
            };

            // Normalize base_url: ensure trailing slash for paths
            let normalized = if !url_str.ends_with('/') {
                // Check if URL has a path component (not just domain)
                // If URL has a path, add trailing slash
                format!("{}/", url_str)
            } else {
                url_str
            };

            self.base_url = Some(URL::parse(&normalized)?);
        }
        Ok(())
    }

    /// Get headers
    #[getter]
    fn headers(&self) -> Headers {
        self.headers.clone()
    }

    /// Set headers
    #[setter]
    fn set_headers(&mut self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(headers) = value.extract::<Headers>() {
            self.headers = headers;
        } else if let Ok(dict) = value.downcast::<PyDict>() {
            let mut headers = Headers::default();
            for (key, val) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = val.extract()?;
                headers.set(k, v);
            }
            self.headers = headers;
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "headers must be a Headers object or dict",
            ));
        }
        Ok(())
    }

    /// Get cookies
    #[getter]
    fn cookies(&self) -> Cookies {
        self.cookies.clone()
    }

    /// Set cookies
    #[setter]
    fn set_cookies(&mut self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(cookies) = value.extract::<Cookies>() {
            self.cookies = cookies;
        } else if let Ok(dict) = value.downcast::<PyDict>() {
            let mut cookies = Cookies::default();
            for (key, val) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = val.extract()?;
                cookies.set(&k, &v);
            }
            self.cookies = cookies;
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "cookies must be a Cookies object or dict",
            ));
        }
        Ok(())
    }

    /// Get timeout
    #[getter]
    fn timeout(&self) -> Timeout {
        self.timeout.clone()
    }

    /// Set timeout
    #[setter]
    fn set_timeout(&mut self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(timeout) = value.extract::<Timeout>() {
            self.timeout = timeout;
        } else if let Ok(seconds) = value.extract::<f64>() {
            self.timeout = Timeout::new(Some(seconds), None, None, None, None);
        } else if value.is_none() {
            self.timeout = Timeout::default();
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "timeout must be a Timeout object or number",
            ));
        }
        Ok(())
    }

    /// Mount a transport for a given URL pattern
    fn mount(&mut self, pattern: &str, transport: Py<PyAny>) {
        self.mounts.insert(pattern.to_string(), transport);
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
            let http_transport = transport_module.getattr("HTTPTransport")?;
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
            if self.url_matches_pattern(&url_str, pattern) {
                if let Some(transport) = self.mounts.get(pattern) {
                    return Ok(transport.bind(py).clone());
                }
            }
        }

        // Return default transport
        self._transport(py)
    }

    fn __repr__(&self) -> String {
        "<Client>".to_string()
    }

    /// Compute headers for a redirect request.
    /// This handles cross-origin auth header stripping.
    fn _redirect_headers(&self, request: &Request, url: &URL, method: &str) -> Headers {
        let mut headers = request.headers_ref().clone();

        // Determine if same origin - same scheme, host, port
        let request_url = request.url_ref();
        let same_host = request_url.get_host_str().to_lowercase() == url.get_host_str().to_lowercase();
        let same_scheme = request_url.get_scheme().to_uppercase() == url.get_scheme().to_uppercase();

        // Get ports, defaulting to standard ports for comparison
        let request_port = request_url.get_port().unwrap_or_else(|| {
            if request_url.get_scheme() == "https" { 443 } else { 80 }
        });
        let url_port = url.get_port().unwrap_or_else(|| {
            if url.get_scheme() == "https" { 443 } else { 80 }
        });
        let same_port = request_port == url_port;

        let same_origin = same_scheme && same_host && same_port;

        // Check if this is an HTTPS upgrade (http -> https on same host with default ports)
        let is_https_upgrade = !same_scheme
            && request_url.get_scheme() == "http"
            && url.get_scheme() == "https"
            && same_host
            && request_port == 80
            && url_port == 443;

        // Update Host header for the new URL
        let new_host = Self::get_host_header(url);
        headers.set("Host".to_string(), new_host);

        // Strip Authorization header unless same origin or HTTPS upgrade
        if !same_origin && !is_https_upgrade {
            headers.remove("authorization");
        }

        headers
    }
}

impl Client {
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
    fn url_matches_pattern(&self, url: &str, pattern: &str) -> bool {
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
