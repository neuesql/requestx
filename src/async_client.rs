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

        Ok(Self {
            inner: Arc::new(client),
            base_url,
            headers: headers.unwrap_or_default(),
            cookies: cookies.unwrap_or_default(),
            timeout,
            follow_redirects,
            max_redirects,
            event_hooks: EventHooks::default(),
            trust_env: true,
            mounts: HashMap::new(),
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
    #[pyo3(signature = (*, auth=None, cookies=None, headers=None, timeout=None, follow_redirects=None, max_redirects=None, base_url=None, event_hooks=None, trust_env=None, **_kwargs))]
    fn new(
        auth: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
        max_redirects: Option<usize>,
        base_url: Option<&str>,
        event_hooks: Option<&Bound<'_, PyDict>>,
        trust_env: Option<bool>,
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

        Ok(client)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        url: String,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.async_request(py, "GET".to_string(), url, None, None, None, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn post<'py>(
        &self,
        py: Python<'py>,
        url: String,
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
        self.async_request(py, "POST".to_string(), url, content, data, json, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn put<'py>(
        &self,
        py: Python<'py>,
        url: String,
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
        self.async_request(py, "PUT".to_string(), url, content, data, json, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn patch<'py>(
        &self,
        py: Python<'py>,
        url: String,
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
        self.async_request(py, "PATCH".to_string(), url, content, data, json, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn delete<'py>(
        &self,
        py: Python<'py>,
        url: String,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.async_request(py, "DELETE".to_string(), url, None, None, None, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn head<'py>(
        &self,
        py: Python<'py>,
        url: String,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.async_request(py, "HEAD".to_string(), url, None, None, None, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn options<'py>(
        &self,
        py: Python<'py>,
        url: String,
        params: Option<PyObject>,
        headers: Option<PyObject>,
        cookies: Option<PyObject>,
        auth: Option<PyObject>,
        follow_redirects: Option<bool>,
        timeout: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.async_request(py, "OPTIONS".to_string(), url, None, None, None, params, headers, cookies, auth, follow_redirects, timeout)
    }

    #[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None))]
    fn request<'py>(
        &self,
        py: Python<'py>,
        method: String,
        url: String,
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
        self.async_request(py, method, url, content, data, json, params, headers, cookies, auth, follow_redirects, timeout)
    }

    fn aclose<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            Ok(())
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

    /// Mount a transport for a given URL pattern
    fn mount(&mut self, pattern: &str, transport: Py<PyAny>) {
        self.mounts.insert(pattern.to_string(), transport);
    }

    fn __repr__(&self) -> String {
        "<AsyncClient>".to_string()
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
        let client = self.inner.clone();
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

        // Build headers
        let mut all_headers = reqwest::header::HeaderMap::new();
        for (k, v) in default_headers.inner() {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                all_headers.insert(name, val);
            }
        }

        if let Some(h) = &headers {
            Python::with_gil(|py| {
                let h_bound = h.bind(py);
                if let Ok(headers_obj) = h_bound.extract::<Headers>() {
                    for (k, v) in headers_obj.inner() {
                        if let (Ok(name), Ok(val)) = (
                            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                            reqwest::header::HeaderValue::from_str(v),
                        ) {
                            all_headers.insert(name, val);
                        }
                    }
                }
            });
        }

        // Process cookies
        let cookie_header = default_cookies.to_header_value();
        if !cookie_header.is_empty() {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&cookie_header) {
                all_headers.insert(reqwest::header::COOKIE, val);
            }
        }

        // Process body
        let body = if let Some(c) = content {
            Some(c)
        } else if let Some(j) = &json {
            let json_str = Python::with_gil(|py| {
                let j_bound = j.bind(py);
                py_to_json_string(j_bound)
            })?;
            all_headers.insert(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            );
            Some(json_str.into_bytes())
        } else {
            None
        };

        // Process auth
        let auth_header = if let Some(a) = &auth {
            Python::with_gil(|py| {
                let a_bound = a.bind(py);
                if let Ok(basic) = a_bound.extract::<BasicAuth>() {
                    let credentials = format!("{}:{}", basic.username, basic.password);
                    let encoded = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        credentials.as_bytes(),
                    );
                    Some(format!("Basic {}", encoded))
                } else if let Ok(tuple) = a_bound.extract::<(String, String)>() {
                    let credentials = format!("{}:{}", tuple.0, tuple.1);
                    let encoded = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        credentials.as_bytes(),
                    );
                    Some(format!("Basic {}", encoded))
                } else {
                    None
                }
            })
        } else {
            None
        };

        if let Some(auth_val) = auth_header {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&auth_val) {
                all_headers.insert(reqwest::header::AUTHORIZATION, val);
            }
        }

        let method_clone = method.clone();
        let url_clone = final_url.clone();

        future_into_py(py, async move {
            let method = reqwest::Method::from_bytes(method_clone.as_bytes())
                .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid HTTP method"))?;

            let mut builder = client.request(method.clone(), &url_clone);
            builder = builder.headers(all_headers);

            if let Some(b) = body {
                builder = builder.body(b);
            }

            let response = builder.send().await.map_err(convert_reqwest_error)?;

            let request = Request::new(method.as_str(), URL::parse(&url_clone)?);
            Response::from_reqwest_async(response, Some(request)).await
        })
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
