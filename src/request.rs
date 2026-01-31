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

/// Stream mode for content
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamMode {
    /// Bytes content - supports both sync and async iteration
    Dual,
    /// Sync-only content (BytesIO, sync iterator)
    SyncOnly,
    /// Async-only content (async iterator, async file-like)
    AsyncOnly,
}

/// HTTP Request object
#[pyclass(name = "Request", subclass, module = "requestx._core")]
pub struct Request {
    method: String,
    url: URL,
    headers: Headers,
    content: Option<Vec<u8>>,
    /// Whether content is from a stream (iterator/generator)
    is_streaming: bool,
    /// Whether the stream has been read (for streaming content)
    is_stream_consumed: bool,
    /// Whether aread() was called (for returning async stream)
    was_async_read: bool,
    /// Python stream object (for pickle/stream tracking)
    stream_ref: Option<PyObject>,
    /// Stream mode (dual, sync-only, or async-only)
    stream_mode: StreamMode,
}

impl Clone for Request {
    fn clone(&self) -> Self {
        Python::with_gil(|py| {
            Self {
                method: self.method.clone(),
                url: self.url.clone(),
                headers: self.headers.clone(),
                content: self.content.clone(),
                is_streaming: self.is_streaming,
                is_stream_consumed: self.is_stream_consumed,
                was_async_read: self.was_async_read,
                stream_ref: self.stream_ref.as_ref().map(|obj| obj.clone_ref(py)),
                stream_mode: self.stream_mode,
            }
        })
    }
}

impl Request {
    pub fn new(method: &str, url: URL) -> Self {
        Self {
            method: method.to_uppercase(),
            url,
            headers: Headers::new(),
            content: None,
            is_streaming: false,
            is_stream_consumed: false,
            was_async_read: false,
            stream_ref: None,
            stream_mode: StreamMode::Dual,
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
        py: Python<'_>,
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
            is_streaming: false,
            is_stream_consumed: false,
            was_async_read: false,
            stream_ref: None,
            stream_mode: StreamMode::Dual,
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
                request.stream_mode = StreamMode::Dual;  // bytes supports both sync and async
            } else if let Ok(s) = c.extract::<String>() {
                request.content = Some(s.into_bytes());
                request.stream_mode = StreamMode::Dual;  // str supports both sync and async
            } else {
                // Check for invalid types first - int, float, dict should be rejected
                let type_name = c.get_type().name()?.to_string();
                if type_name == "int" || type_name == "float" || type_name == "dict" {
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        format!("Invalid type for content: {}", type_name)
                    ));
                }

                // Check if it's an async iterator/generator (has __aiter__ and __anext__)
                let has_aiter = c.hasattr("__aiter__")?;
                let has_anext = c.hasattr("__anext__")?;
                let is_async = has_aiter && has_anext;

                // Check if it's a sync iterator (has __iter__ but not async)
                let has_iter = c.hasattr("__iter__")?;
                let has_next = c.hasattr("__next__")?;

                // Check if it's a file-like object (has read and seek methods)
                let has_read = c.hasattr("read")?;
                let has_seek = c.hasattr("seek")?;
                let has_aread = c.hasattr("aread")?;

                // Also check for generator type or async generator type
                let is_gen_type = type_name == "generator";
                let is_async_gen_type = type_name == "async_generator";

                // Check if it's a sync file-like object (has read() AND seek() - distinguishes from generators)
                // BytesIO, file objects, etc. - we can read content immediately
                // Use seek() as discriminator since file-like objects have it but generators don't
                let is_sync_file_like = has_read && has_seek && !is_gen_type;

                if is_async || is_async_gen_type {
                    // Async iterator/generator - treat as streaming
                    request.is_streaming = true;
                    request.stream_ref = Some(c.clone().unbind());
                    request.stream_mode = StreamMode::AsyncOnly;
                } else if has_aread && !has_anext && !is_async_gen_type {
                    // Async file-like object (has aread but not __anext__)
                    // Treat as async streaming
                    request.is_streaming = true;
                    request.stream_ref = Some(c.clone().unbind());
                    request.stream_mode = StreamMode::AsyncOnly;
                } else if is_sync_file_like {
                    // Sync file-like object (BytesIO, etc.) - read content immediately
                    let read_method = c.getattr("read")?;
                    let content_obj = read_method.call0()?;
                    if let Ok(bytes) = content_obj.extract::<Vec<u8>>() {
                        request.content = Some(bytes);
                        request.stream_mode = StreamMode::SyncOnly;
                    } else if let Ok(s) = content_obj.extract::<String>() {
                        request.content = Some(s.into_bytes());
                        request.stream_mode = StreamMode::SyncOnly;
                    } else {
                        return Err(pyo3::exceptions::PyTypeError::new_err(
                            "File-like object read() must return bytes or str"
                        ));
                    }
                } else if has_next || is_gen_type {
                    // Sync iterator/generator - treat as streaming
                    request.is_streaming = true;
                    request.stream_ref = Some(c.clone().unbind());
                    request.stream_mode = StreamMode::SyncOnly;
                } else if has_iter {
                    // Generic iterable - wrap and treat as streaming
                    request.is_streaming = true;
                    request.stream_ref = Some(c.clone().unbind());
                    request.stream_mode = StreamMode::SyncOnly;
                } else {
                    // Invalid content type - must be bytes, str, or iterator
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        format!("Invalid type for content: {}", type_name)
                    ));
                }
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
                        let (body, _) = build_multipart_body_with_boundary(py, data_dict, Some(f), &b)?;
                        (body, ct.clone())
                    } else {
                        // Invalid boundary format, use auto-generated
                        let (body, boundary) = build_multipart_body(py, data_dict, Some(f))?;
                        (body, format!("multipart/form-data; boundary={}", boundary))
                    }
                } else {
                    // Content-Type set but no boundary
                    let (body, boundary) = build_multipart_body(py, data_dict, Some(f))?;
                    // Keep the existing content-type
                    (body, ct.clone())
                }
            } else {
                // No Content-Type set, use auto-generated boundary
                let (body, boundary) = build_multipart_body(py, data_dict, Some(f))?;
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
            } else {
                // data is not a dict - treat as content with DeprecationWarning
                // This is for compatibility with requests library
                emit_deprecation_warning(py, "Use 'content=...' instead of 'data=...' for raw bytes or iterator content.")?;

                // Handle the same way as content parameter
                if let Ok(bytes) = d.extract::<Vec<u8>>() {
                    request.content = Some(bytes);
                    request.stream_mode = StreamMode::Dual;
                } else if let Ok(s) = d.extract::<String>() {
                    request.content = Some(s.into_bytes());
                    request.stream_mode = StreamMode::Dual;
                } else {
                    // Check for iterator/generator/async iterator
                    let type_name = d.get_type().name()?.to_string();

                    let has_aiter = d.hasattr("__aiter__")?;
                    let has_anext = d.hasattr("__anext__")?;
                    let is_async = has_aiter && has_anext;

                    let has_iter = d.hasattr("__iter__")?;
                    let has_next = d.hasattr("__next__")?;
                    let has_read = d.hasattr("read")?;
                    let has_aread = d.hasattr("aread")?;

                    let is_gen_type = type_name == "generator";
                    let is_async_gen_type = type_name == "async_generator";

                    if is_async || is_async_gen_type || has_aread {
                        request.is_streaming = true;
                        request.stream_ref = Some(d.clone().unbind());
                        request.stream_mode = StreamMode::AsyncOnly;
                    } else if has_iter || has_next || is_gen_type || has_read {
                        request.is_streaming = true;
                        request.stream_ref = Some(d.clone().unbind());
                        request.stream_mode = StreamMode::SyncOnly;
                    }
                }
            }
        }

        // Set Content-Length or Transfer-Encoding header
        // - If content was provided (non-streaming), set Content-Length to actual length
        // - For streaming content, set Transfer-Encoding: chunked (unless Content-Length already set)
        // - For methods with body (POST, PUT, PATCH) and no content, set Content-Length: 0
        if request.is_streaming {
            // Streaming content - set Transfer-Encoding: chunked unless Content-Length is already set
            if !request.headers.contains("content-length") && !request.headers.contains("Content-Length") {
                request.headers.set("Transfer-Encoding".to_string(), "chunked".to_string());
            }
        } else if let Some(ref content) = request.content {
            request.headers.set("Content-Length".to_string(), content.len().to_string());
        } else if matches!(request.method.as_str(), "POST" | "PUT" | "PATCH") {
            request.headers.set("Content-Length".to_string(), "0".to_string());
        }

        // Set Host header only if not already set by user
        if !request.headers.contains("host") && !request.headers.contains("Host") {
            if let Some(host) = request.url.get_host() {
                request.headers.set("Host".to_string(), host);
            }
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

    /// Get the stream mode: "dual", "sync", or "async"
    #[getter]
    fn stream_mode(&self) -> &str {
        match self.stream_mode {
            StreamMode::Dual => "dual",
            StreamMode::SyncOnly => "sync",
            StreamMode::AsyncOnly => "async",
        }
    }

    /// Get the stream reference (for iterators/generators)
    #[getter]
    fn stream_ref(&self, py: Python<'_>) -> Option<PyObject> {
        self.stream_ref.as_ref().map(|obj| obj.clone_ref(py))
    }

    /// Check if this is a streaming request
    #[getter]
    fn is_streaming(&self) -> bool {
        self.is_streaming
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
    fn content<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        if self.is_streaming && !self.is_stream_consumed {
            // Raise RequestNotRead for unread streaming content
            let requestx = py.import("requestx")?;
            let exc_type = requestx.getattr("RequestNotRead")?;
            return Err(PyErr::from_value(exc_type.call0()?));
        }
        match &self.content {
            Some(c) => Ok(PyBytes::new(py, c)),
            None => Ok(PyBytes::new(py, b"")),
        }
    }

    #[getter]
    fn stream<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::types::AsyncByteStream;

        // If content has been read, return a stream from the content
        // The stream needs to support both sync and async iteration based on how it was read
        if self.is_stream_consumed || !self.is_streaming {
            let data = self.content.clone().unwrap_or_default();
            // Return AsyncByteStream if aread was called, SyncByteStream otherwise
            // Both types support both sync and async iteration, so this works either way
            let stream = SyncByteStream::from_data(data);
            let stream_obj = Py::new(py, stream)?;
            Ok(stream_obj.into_bound(py).into_any())
        } else {
            // Return the original stream reference if not consumed
            if let Some(ref stream_ref) = self.stream_ref {
                Ok(stream_ref.bind(py).clone())
            } else {
                let stream = SyncByteStream::from_data(Vec::new());
                let stream_obj = Py::new(py, stream)?;
                Ok(stream_obj.into_bound(py).into_any())
            }
        }
    }

    #[getter]
    fn extensions(&self) -> std::collections::HashMap<String, PyObject> {
        std::collections::HashMap::new()
    }

    fn read(&mut self, py: Python<'_>) -> PyResult<Vec<u8>> {
        if self.is_streaming && !self.is_stream_consumed {
            // Check if stream is closed (None after unpickling without read)
            if self.stream_ref.is_none() {
                let requestx = py.import("requestx")?;
                let exc_type = requestx.getattr("StreamClosed")?;
                return Err(PyErr::from_value(exc_type.call0()?));
            }

            // Consume the stream
            let stream_obj = self.stream_ref.as_ref().unwrap().bind(py);
            let mut result: Vec<u8> = Vec::new();

            // Check if it's async iterator (has __anext__) - can't consume sync
            if stream_obj.hasattr("__anext__")? {
                // For async iterators, we can't consume them in sync read
                // This is a special case - mark as consumed but leave empty
                self.is_stream_consumed = true;
                self.content = Some(result.clone());
                return Ok(result);
            }

            // Try to iterate over the stream using Python iteration protocol
            let iter_obj = stream_obj.call_method0("__iter__")?;
            loop {
                match iter_obj.call_method0("__next__") {
                    Ok(chunk) => {
                        if let Ok(bytes) = chunk.extract::<Vec<u8>>() {
                            result.extend(bytes);
                        } else if let Ok(s) = chunk.extract::<String>() {
                            result.extend(s.into_bytes());
                        }
                    }
                    Err(e) => {
                        if e.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                            break;
                        }
                        return Err(e);
                    }
                }
            }

            self.content = Some(result.clone());
            self.is_stream_consumed = true;
            self.stream_ref = None;  // Clear the stream reference
            Ok(result)
        } else {
            Ok(self.content.clone().unwrap_or_default())
        }
    }

    /// Async read method - reads streaming content asynchronously
    fn aread<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Mark that async read was called - affects stream getter
        self.was_async_read = true;

        // Create an async coroutine that reads the stream
        let is_streaming = self.is_streaming;
        let is_stream_consumed = self.is_stream_consumed;
        let stream_ref = self.stream_ref.as_ref().map(|s| s.clone_ref(py));
        let content = self.content.clone();

        if is_streaming && !is_stream_consumed {
            // Check if stream is closed
            if stream_ref.is_none() {
                let requestx = py.import("requestx")?;
                let exc_type = requestx.getattr("StreamClosed")?;
                return Err(PyErr::from_value(exc_type.call0()?));
            }

            // We need to consume the async iterator
            // Create a coroutine that does this
            let code = r#"
async def _aread(stream):
    result = b""
    async for chunk in stream:
        if isinstance(chunk, bytes):
            result += chunk
        else:
            result += chunk.encode()
    return result
"#;
            let builtins = py.import("builtins")?;
            let exec_fn = builtins.getattr("exec")?;
            let globals = PyDict::new(py);
            exec_fn.call1((code, &globals))?;
            let aread_func = globals.get_item("_aread")?.unwrap();
            let stream = stream_ref.unwrap();
            let coro = aread_func.call1((stream,))?;

            // Mark as consumed
            self.is_stream_consumed = true;
            self.stream_ref = None;

            Ok(coro)
        } else {
            // Return completed future with content
            let content_bytes = content.unwrap_or_default();

            // Create a coroutine that returns the content immediately
            let code = r#"
async def _return_bytes(data):
    return data
"#;
            let builtins = py.import("builtins")?;
            let exec_fn = builtins.getattr("exec")?;
            let globals = PyDict::new(py);
            exec_fn.call1((code, &globals))?;
            let return_func = globals.get_item("_return_bytes")?.unwrap();
            let coro = return_func.call1((PyBytes::new(py, &content_bytes),))?;
            Ok(coro)
        }
    }

    /// Set the content from Python (used by aread wrapper)
    fn _set_content_from_aread(&mut self, content: Vec<u8>) {
        self.content = Some(content);
        self.is_stream_consumed = true;
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

    /// Pickle support - get state
    fn __getstate__(&self, py: Python<'_>) -> PyResult<PyObject> {
        let state = PyDict::new(py);
        state.set_item("method", &self.method)?;
        state.set_item("url", self.url.to_string())?;
        state.set_item("headers", self.headers.inner())?;
        state.set_item("content", self.content.as_ref().map(|c| PyBytes::new(py, c)))?;
        state.set_item("is_streaming", self.is_streaming)?;
        state.set_item("is_stream_consumed", self.is_stream_consumed)?;
        state.set_item("was_async_read", self.was_async_read)?;
        // Don't pickle the actual stream, just mark that there was one
        state.set_item("had_stream", self.stream_ref.is_some())?;
        Ok(state.into())
    }

    /// Pickle support - restore state
    fn __setstate__(&mut self, py: Python<'_>, state: &Bound<'_, PyDict>) -> PyResult<()> {
        self.method = state.get_item("method")?.unwrap().extract()?;
        let url_str: String = state.get_item("url")?.unwrap().extract()?;
        self.url = URL::new_impl(Some(&url_str), None, None, None, None, None, None, None, None, None, None, None)?;

        // Restore headers
        self.headers = Headers::new();
        let headers_list: Vec<(String, String)> = state.get_item("headers")?.unwrap().extract()?;
        for (k, v) in headers_list {
            self.headers.set(k, v);
        }

        // Restore content
        self.content = if let Some(content_item) = state.get_item("content")? {
            if content_item.is_none() {
                None
            } else if let Ok(bytes) = content_item.extract::<Vec<u8>>() {
                Some(bytes)
            } else {
                None
            }
        } else {
            None
        };

        self.is_streaming = state.get_item("is_streaming")?.unwrap().extract()?;
        self.is_stream_consumed = state.get_item("is_stream_consumed")?.unwrap().extract()?;
        self.was_async_read = state.get_item("was_async_read")?.map(|v| v.extract().unwrap_or(false)).unwrap_or(false);

        // Stream reference is not pickled - it's gone after unpickling
        // If it was streaming and not consumed, it will raise StreamClosed on read attempts
        self.stream_ref = None;

        Ok(())
    }

    /// Reduce for pickle - use __getnewargs__ to provide required args
    fn __getnewargs__(&self) -> (&str, String) {
        (&self.method, self.url.to_string())
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

/// Emit a DeprecationWarning from Python
fn emit_deprecation_warning(py: Python<'_>, message: &str) -> PyResult<()> {
    let warnings = py.import("warnings")?;
    let deprecation_warning = py.get_type::<pyo3::exceptions::PyDeprecationWarning>();
    warnings.call_method1("warn", (message, deprecation_warning, 2i32))?;
    Ok(())
}
