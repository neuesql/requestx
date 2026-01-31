//! HTTP Request implementation

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString};

use crate::cookies::Cookies;
use crate::headers::Headers;
use crate::multipart::{build_multipart_body, build_multipart_body_with_boundary, extract_boundary_from_content_type};
use crate::types::SyncByteStream;
use crate::url::URL;

/// Convert a Python value to a string for form encoding (handles int, float, bool, str, None)
pub fn py_value_to_form_str(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    if obj.is_none() {
        return Ok(String::new());
    }
    // Check bool before int (since bool is subclass of int in Python)
    if let Ok(b) = obj.downcast::<PyBool>() {
        return Ok(if b.is_true() { "true" } else { "false" }.to_string());
    }
    if let Ok(i) = obj.downcast::<PyInt>() {
        let val: i64 = i.extract()?;
        return Ok(val.to_string());
    }
    if let Ok(f) = obj.downcast::<PyFloat>() {
        let val: f64 = f.extract()?;
        return Ok(val.to_string());
    }
    if let Ok(s) = obj.downcast::<PyString>() {
        return Ok(s.extract::<String>()?);
    }
    // Fall back to str() representation
    Ok(obj.str()?.to_string())
}

/// Mutable headers wrapper for Request.headers
/// This allows modifying headers in place and assigning back to Request
#[pyclass(name = "MutableHeaders")]
#[derive(Clone)]
pub struct MutableHeaders {
    pub headers: Headers,
}

#[pymethods]
impl MutableHeaders {
    fn __getitem__(&self, key: &str) -> PyResult<String> {
        self.headers.get(key, None).ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(key.to_string())
        })
    }

    fn __setitem__(&mut self, key: &str, value: &str) {
        self.headers.set(key.to_string(), value.to_string());
    }

    fn __delitem__(&mut self, key: &str) {
        // Remove all entries with this key
        let key_lower = key.to_lowercase();
        let new_inner: Vec<_> = self.headers.inner()
            .iter()
            .filter(|(k, _)| k.to_lowercase() != key_lower)
            .cloned()
            .collect();
        self.headers = Headers::from_vec(new_inner);
    }

    fn __contains__(&self, key: &str) -> bool {
        self.headers.get(key, None).is_some()
    }

    fn __iter__(&self) -> MutableHeadersIter {
        // Get unique keys
        let mut seen = std::collections::HashSet::new();
        let keys: Vec<String> = self.headers.inner()
            .iter()
            .filter_map(|(k, _)| {
                let k_lower = k.to_lowercase();
                if seen.insert(k_lower) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();
        MutableHeadersIter { keys, index: 0 }
    }

    #[pyo3(signature = (key, default=None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        self.headers.get(key, default.as_deref())
    }

    fn keys(&self) -> Vec<String> {
        // Return unique keys
        let mut seen = std::collections::HashSet::new();
        self.headers.inner()
            .iter()
            .filter_map(|(k, _)| {
                let k_lower = k.to_lowercase();
                if seen.insert(k_lower) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn values(&self) -> Vec<String> {
        self.headers.inner().iter().map(|(_, v)| v.clone()).collect()
    }

    fn items(&self) -> Vec<(String, String)> {
        // Return merged values for duplicate keys (httpx behavior)
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for (key, _) in self.headers.inner() {
            let key_lower = key.to_lowercase();
            if seen.insert(key_lower.clone()) {
                let values: Vec<&str> = self.headers.inner()
                    .iter()
                    .filter(|(k, _)| k.to_lowercase() == key_lower)
                    .map(|(_, v)| v.as_str())
                    .collect();
                result.push((key.clone(), values.join(", ")));
            }
        }
        result
    }

    fn multi_items(&self) -> Vec<(String, String)> {
        self.headers.inner().clone()
    }

    /// Returns the raw headers as a list of (name, value) tuples of bytes
    #[getter]
    fn raw<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        use pyo3::types::PyBytes;
        let items: Vec<_> = self.headers.inner()
            .iter()
            .map(|(k, v)| {
                let key_bytes = PyBytes::new(py, k.as_bytes());
                let value_bytes = PyBytes::new(py, v.as_bytes());
                (key_bytes, value_bytes)
            })
            .collect();
        PyList::new(py, items)
    }

    fn update(&mut self, other: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(h) = other.extract::<Headers>() {
            for (k, v) in h.inner() {
                self.headers.set(k.clone(), v.clone());
            }
        } else if let Ok(mh) = other.extract::<MutableHeaders>() {
            for (k, v) in mh.headers.inner() {
                self.headers.set(k.clone(), v.clone());
            }
        } else if let Ok(dict) = other.downcast::<PyDict>() {
            for (key, value) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                self.headers.set(k, v);
            }
        }
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("MutableHeaders({:?})", self.headers.inner())
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        use pyo3::types::PyDict;
        // Compare with dict
        if let Ok(dict) = other.downcast::<PyDict>() {
            // Build dict from our headers
            let our_items: Vec<(String, String)> = self.headers.inner().clone();
            // Convert to lowercase-keyed map for comparison
            let mut our_map = std::collections::HashMap::new();
            for (k, v) in &our_items {
                our_map.insert(k.to_lowercase(), v.clone());
            }
            // Compare
            for (key, value) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                if our_map.get(&k.to_lowercase()) != Some(&v) {
                    return Ok(false);
                }
            }
            // Check same number of keys
            // Count unique keys in our headers
            let our_unique_keys: std::collections::HashSet<String> = our_items.iter().map(|(k, _)| k.to_lowercase()).collect();
            if our_unique_keys.len() != dict.len() {
                return Ok(false);
            }
            return Ok(true);
        }
        // Compare with Headers
        if let Ok(h) = other.extract::<Headers>() {
            // Compare inner vectors - both have same structure
            return Ok(self.headers.inner() == h.inner());
        }
        // Compare with MutableHeaders
        if let Ok(mh) = other.extract::<MutableHeaders>() {
            return Ok(self.headers.inner() == mh.headers.inner());
        }
        Ok(false)
    }
}

#[pyclass]
pub struct MutableHeadersIter {
    keys: Vec<String>,
    index: usize,
}

#[pymethods]
impl MutableHeadersIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<String> {
        if self.index < self.keys.len() {
            let key = self.keys[self.index].clone();
            self.index += 1;
            Some(key)
        } else {
            None
        }
    }
}

/// HTTP Request object
#[pyclass(name = "Request", subclass)]
#[derive(Clone)]
pub struct Request {
    method: String,
    url: URL,
    headers: Headers,
    content: Option<Vec<u8>>,
}

impl Request {
    pub fn new(method: &str, url: URL) -> Self {
        Self {
            method: method.to_uppercase(),
            url,
            headers: Headers::new(),
            content: None,
        }
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn url_ref(&self) -> &URL {
        &self.url
    }

    pub fn headers_ref(&self) -> &Headers {
        &self.headers
    }

    pub fn content_bytes(&self) -> Option<&[u8]> {
        self.content.as_deref()
    }

    pub fn set_content(&mut self, content: Vec<u8>) {
        self.content = Some(content);
    }

    pub fn set_headers(&mut self, headers: Headers) {
        self.headers = headers;
    }
}

#[pymethods]
impl Request {
    #[new]
    #[pyo3(signature = (method, url, *, params=None, headers=None, cookies=None, content=None, data=None, files=None, json=None, stream=None, extensions=None))]
    fn py_new(
        _py: Python<'_>,
        method: &str,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyAny>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyAny>>,
        data: Option<&Bound<'_, PyAny>>,
        files: Option<&Bound<'_, PyAny>>,
        json: Option<&Bound<'_, PyAny>>,
        #[allow(unused)] stream: Option<&Bound<'_, PyAny>>,
        #[allow(unused)] extensions: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        // Parse URL
        let parsed_url = if let Ok(url_obj) = url.extract::<URL>() {
            url_obj
        } else if let Ok(url_str) = url.extract::<String>() {
            URL::new_impl(Some(&url_str), None, None, None, None, None, None, None, None, params, None, None)?
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "URL must be a string or URL object",
            ));
        };

        let mut request = Self {
            method: method.to_uppercase(),
            url: parsed_url,
            headers: Headers::new(),
            content: None,
        };

        // Set headers
        if let Some(h) = headers {
            if let Ok(headers_obj) = h.extract::<Headers>() {
                request.headers = headers_obj;
            } else if let Ok(dict) = h.downcast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    request.headers.set(k, v);
                }
            }
        }

        // Set cookies as header
        if let Some(c) = cookies {
            if let Ok(cookies_obj) = c.extract::<Cookies>() {
                let cookie_header = cookies_obj.to_header_value();
                if !cookie_header.is_empty() {
                    request.headers.set("Cookie".to_string(), cookie_header);
                }
            }
        }

        // Handle content
        if let Some(c) = content {
            if let Ok(bytes) = c.extract::<Vec<u8>>() {
                request.content = Some(bytes);
            } else if let Ok(s) = c.extract::<String>() {
                request.content = Some(s.into_bytes());
            } else {
                // Invalid content type - must be bytes or str
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    format!("'content' must be bytes or str, not {}", c.get_type().name()?)
                ));
            }
        }

        // Handle JSON
        if let Some(j) = json {
            let json_str = py_to_json_string(j)?;
            request.content = Some(json_str.into_bytes());
            if !request.headers.contains("content-type") {
                request.headers.set("Content-Type".to_string(), "application/json".to_string());
            }
        }

        // Handle multipart (files provided)
        // Check if files is not empty (dict or list)
        let files_not_empty = files.map(|f| {
            if let Ok(dict) = f.downcast::<PyDict>() {
                !dict.is_empty()
            } else if let Ok(list) = f.downcast::<PyList>() {
                !list.is_empty()
            } else {
                true  // Unknown type, assume not empty
            }
        }).unwrap_or(false);

        if files_not_empty {
            let f = files.unwrap();
            // Check if boundary was already set in headers BEFORE reading files
            let existing_ct = request.headers.get("content-type", None);
            // Get data dict if provided
            let data_dict: Option<&Bound<'_, PyDict>> = data.and_then(|d| d.downcast::<PyDict>().ok());

            let (body, content_type) = if let Some(ref ct) = existing_ct {
                if ct.contains("boundary=") {
                    // Extract boundary from existing header and use it
                    let boundary_str = extract_boundary_from_content_type(ct);
                    if let Some(b) = boundary_str {
                        let (body, _) = build_multipart_body_with_boundary(_py, data_dict, Some(f), &b)?;
                        (body, ct.clone())
                    } else {
                        // Invalid boundary format, use auto-generated
                        let (body, boundary) = build_multipart_body(_py, data_dict, Some(f))?;
                        (body, format!("multipart/form-data; boundary={}", boundary))
                    }
                } else {
                    // Content-Type set but no boundary
                    let (body, boundary) = build_multipart_body(_py, data_dict, Some(f))?;
                    // Keep the existing content-type
                    (body, ct.clone())
                }
            } else {
                // No Content-Type set, use auto-generated boundary
                let (body, boundary) = build_multipart_body(_py, data_dict, Some(f))?;
                (body, format!("multipart/form-data; boundary={}", boundary))
            };

            request.content = Some(body);
            request.headers.set("Content-Type".to_string(), content_type);
        } else if let Some(d) = data {
            // Handle form data (no files)
            if let Ok(dict) = d.downcast::<PyDict>() {
                // Only process if dict is not empty
                if !dict.is_empty() {
                    let mut form_data = Vec::new();
                    for (key, value) in dict.iter() {
                        let k: String = key.extract()?;
                        // Handle lists - create multiple key=value pairs
                        if let Ok(list) = value.downcast::<PyList>() {
                            for item in list.iter() {
                                let v = py_value_to_form_str(&item)?;
                                form_data.push(format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)));
                            }
                        } else {
                            let v = py_value_to_form_str(&value)?;
                            form_data.push(format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)));
                        }
                    }
                    request.content = Some(form_data.join("&").into_bytes());
                    if !request.headers.contains("content-type") {
                        request.headers.set(
                            "Content-Type".to_string(),
                            "application/x-www-form-urlencoded".to_string(),
                        );
                    }
                }
            }
        }

        // Set Content-Length header
        // - If content was provided, set to actual length
        // - For methods with body (POST, PUT, PATCH), set to 0 if no content
        // - For other methods (GET, HEAD, etc.), don't set if no content
        if let Some(ref content) = request.content {
            request.headers.set("Content-Length".to_string(), content.len().to_string());
        } else if matches!(request.method.as_str(), "POST" | "PUT" | "PATCH") {
            request.headers.set("Content-Length".to_string(), "0".to_string());
        }

        // Set Host header
        if let Some(host) = request.url.get_host() {
            request.headers.set("Host".to_string(), host);
        }

        Ok(request)
    }

    #[getter(method)]
    fn py_method(&self) -> &str {
        &self.method
    }

    #[getter]
    fn url(&self) -> URL {
        self.url.clone()
    }

    #[getter]
    fn headers(&self) -> MutableHeaders {
        // Return a MutableHeaders wrapper that holds a reference-like proxy
        MutableHeaders { headers: self.headers.clone() }
    }

    #[setter(headers)]
    fn py_set_headers(&mut self, headers: &Bound<'_, PyAny>) -> PyResult<()> {
        use pyo3::types::PyDict;
        if let Ok(h) = headers.extract::<Headers>() {
            self.headers = h;
        } else if let Ok(mh) = headers.extract::<MutableHeaders>() {
            self.headers = mh.headers;
        } else if let Ok(dict) = headers.downcast::<PyDict>() {
            self.headers = Headers::new();
            for (key, value) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                self.headers.set(k, v);
            }
        }
        Ok(())
    }

    #[getter]
    fn content<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        match &self.content {
            Some(c) => PyBytes::new(py, c),
            None => PyBytes::new(py, b""),
        }
    }

    #[getter]
    fn stream(&self) -> SyncByteStream {
        match &self.content {
            Some(data) => SyncByteStream::from_data(data.clone()),
            None => SyncByteStream::from_data(Vec::new()),
        }
    }

    #[getter]
    fn extensions(&self) -> std::collections::HashMap<String, PyObject> {
        std::collections::HashMap::new()
    }

    fn read(&mut self) -> Vec<u8> {
        self.content.clone().unwrap_or_default()
    }

    /// Set a single header on the request
    fn set_header(&mut self, name: &str, value: &str) {
        self.headers.set(name.to_string(), value.to_string());
    }

    /// Get a single header from the request
    fn get_header(&self, name: &str, default: Option<&str>) -> Option<String> {
        self.headers.get(name, default)
    }

    fn __repr__(&self) -> String {
        format!("<Request('{}', '{}')>", self.method, self.url.to_string())
    }

    fn __eq__(&self, other: &Request) -> bool {
        self.method == other.method && self.url.to_string() == other.url.to_string()
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
        let mut obj = sonic_rs::Object::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            let value = py_to_json_value(&v)?;
            obj.insert(&key, value);
        }
        return Ok(sonic_rs::Value::from(obj));
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "Unsupported type for JSON serialization",
    ))
}
