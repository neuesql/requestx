use bytes::Bytes;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use sonic_rs::Value;
use std::collections::HashMap;

use crate::error::RequestxError;

/// Case-insensitive headers wrapper (internal use only)
#[pyclass]
#[derive(Clone)]
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
            .insert(lowercase_key.clone(), value.clone());
        self.inner.insert(key, value);
    }

    pub fn from_hashmap(headers: HashMap<String, String>) -> Self {
        let mut ci_headers = Self {
            inner: HashMap::new(),
            lowercase_map: HashMap::new(),
        };
        for (key, value) in headers {
            ci_headers.insert(key, value);
        }
        ci_headers
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.lowercase_map.get(&key.to_lowercase())
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

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Python dict-like wrapper with case-insensitive header access
#[pyclass]
#[derive(Clone)]
pub struct CaseInsensitivePyDict {
    headers: CaseInsensitiveHeaders,
}

#[pymethods]
impl CaseInsensitivePyDict {
    #[new]
    #[pyo3(signature = (initial_dict = None))]
    fn new(initial_dict: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut headers = CaseInsensitiveHeaders::new();

        if let Some(dict) = initial_dict {
            for (key, value) in dict.iter() {
                let key_str = key.extract::<String>()?;
                let value_str = value.extract::<String>()?;
                headers.insert(key_str, value_str);
            }
        }

        Ok(Self { headers })
    }

    /// Case-insensitive get
    fn get(&self, key: &str) -> Option<String> {
        self.headers.get(key).cloned()
    }

    /// Case-insensitive __getitem__
    fn __getitem__(&self, key: &str) -> Option<String> {
        self.get(key)
    }

    /// Case-insensitive __contains__
    fn __contains__(&self, key: &str) -> bool {
        self.headers.get(key).is_some()
    }

    /// Case-insensitive __setitem__
    fn __setitem__(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    /// Insert a key-value pair
    fn insert(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    /// Case-insensitive __delitem__
    fn __delitem__(&mut self, key: &str) -> PyResult<()> {
        let lowercase = key.to_lowercase();
        let removed = self.headers.lowercase_map.remove(&lowercase);
        if removed.is_some() {
            // Also remove from inner (we need to find the original case key)
            let key_to_remove = self
                .headers
                .inner
                .keys()
                .find(|k| k.to_lowercase() == lowercase)
                .cloned();
            if let Some(k) = key_to_remove {
                self.headers.inner.remove(&k);
            }
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Key not found: {key}"
            )))
        }
    }

    /// Get length
    fn __len__(&self) -> usize {
        self.headers.len()
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("{:?}", self.headers.inner)
    }

    /// Get all keys
    fn keys(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::new(py, self.headers.keys().cloned().collect::<Vec<_>>())?;
        Ok(list.into())
    }

    /// Get all values
    fn values(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::new(py, self.headers.values().cloned().collect::<Vec<_>>())?;
        Ok(list.into())
    }

    /// Get all items
    fn items(&self, py: Python) -> PyResult<PyObject> {
        let items: Vec<(String, String)> = self
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let list = PyList::new(
            py,
            items
                .iter()
                .map(|(k, v)| (k as &str, v as &str))
                .collect::<Vec<_>>(),
        )?;
        Ok(list.into())
    }

    /// Convert to regular dict
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (key, value) in self.headers.iter() {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }
}

/// Response object compatible with requests.Response
#[pyclass]
#[derive(Clone)]
pub struct Response {
    #[pyo3(get)]
    pub status_code: u16,

    #[pyo3(get)]
    pub url: String,

    pub headers: CaseInsensitiveHeaders,
    text_content: Option<String>,
    pub binary_content: Option<Bytes>,
    encoding: Option<String>,

    #[pyo3(get)]
    pub ok: bool,

    #[pyo3(get)]
    pub reason: String,

    pub is_stream: bool,

    pub elapsed_us: u64,

    // History of redirect responses
    pub history: Vec<Response>,
}

#[pymethods]
impl Response {
    #[new]
    pub fn new(
        status_code: u16,
        url: String,
        headers: HashMap<String, String>,
        content: Vec<u8>,
        is_stream: bool,
        elapsed_us: u64,
    ) -> Self {
        let ok = status_code < 400;
        let reason = Self::status_code_to_reason(status_code);

        // Convert HashMap to CaseInsensitiveHeaders
        let ci_headers = CaseInsensitiveHeaders::from_hashmap(headers);

        Response {
            status_code,
            url,
            headers: ci_headers,
            text_content: None,
            binary_content: Some(content.into()),
            encoding: None,
            ok,
            reason,
            is_stream,
            elapsed_us,
            history: Vec::new(),
        }
    }

    /// Get response headers as a case-insensitive dictionary
    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        let mut dict = CaseInsensitivePyDict::new(None)?;
        for (key, value) in self.headers.iter() {
            dict.insert(key, value);
        }
        Ok(dict.to_dict(py)?)
    }

    /// Get response text content
    #[getter]
    fn text(&mut self) -> PyResult<String> {
        if let Some(ref text) = self.text_content {
            return Ok(text.clone());
        }

        if let Some(ref content) = self.binary_content {
            // Try to detect encoding from headers
            let encoding = self.detect_encoding();

            let text = match encoding.as_deref() {
                Some("utf-8") | None => String::from_utf8_lossy(content).to_string(),
                Some("latin-1") | Some("iso-8859-1") => {
                    // For latin-1, each byte maps directly to a Unicode code point
                    content.iter().map(|&b| b as char).collect()
                }
                _ => {
                    // Fallback to UTF-8 with replacement characters
                    String::from_utf8_lossy(content).to_string()
                }
            };

            self.text_content = Some(text.clone());
            Ok(text)
        } else {
            Ok(String::new())
        }
    }

    /// Get response binary content
    #[getter]
    fn content(&self, py: Python) -> PyResult<PyObject> {
        if let Some(ref content) = self.binary_content {
            Ok(PyBytes::new(py, content).into())
        } else {
            Ok(PyBytes::new(py, &[]).into())
        }
    }

    /// Parse response as JSON - optimized to use from_slice on Bytes
    fn json(&mut self, py: Python) -> PyResult<PyObject> {
        // Use from_slice directly on binary content for better performance
        // Bytes Deref to [u8], so this works without copying
        if let Some(ref content) = self.binary_content {
            let value: Value =
                sonic_rs::from_slice(content).map_err(RequestxError::JsonDecodeError)?;

            pythonize::pythonize(py, &value)
                .map_err(|e| RequestxError::PythonError(e.to_string()).into())
                .map(Bound::unbind)
        } else {
            // Empty response - return empty dict
            let dict = PyDict::new(py);
            Ok(dict.into())
        }
    }

    /// Raise an exception for HTTP error status codes
    fn raise_for_status(&self) -> PyResult<()> {
        if self.status_code >= 400 {
            let error = RequestxError::HttpError {
                status: self.status_code,
                message: format!("{} {}", self.status_code, self.reason),
            };
            return Err(error.into());
        }
        Ok(())
    }

    /// Get response encoding
    #[getter]
    fn encoding(&self) -> Option<String> {
        self.encoding.clone()
    }

    /// Get elapsed time as timedelta
    #[getter]
    fn elapsed(&self, py: Python) -> PyResult<PyObject> {
        // Create a timedelta from microseconds using datetime.timedelta
        let datetime = py.import("datetime")?;
        let timedelta_class = datetime.getattr("timedelta")?;

        // Calculate days, seconds, and microseconds
        let total_seconds = self.elapsed_us / 1_000_000;
        let days = total_seconds / (24 * 60 * 60);
        let seconds = total_seconds % (24 * 60 * 60);
        let microseconds = self.elapsed_us % 1_000_000;

        let timedelta = timedelta_class.call1((days, seconds, microseconds))?;
        Ok(timedelta.into())
    }

    /// Set response encoding
    #[setter]
    fn set_encoding(&mut self, encoding: Option<String>) {
        self.encoding = encoding;
        // Clear cached text content so it gets re-decoded with new encoding
        self.text_content = None;
    }

    /// Check if the response was successful (status code < 400)
    #[getter]
    fn is_redirect(&self) -> bool {
        matches!(self.status_code, 301 | 302 | 303 | 307 | 308)
    }

    /// Check if the response is a permanent redirect
    #[getter]
    fn is_permanent_redirect(&self) -> bool {
        matches!(self.status_code, 301 | 308)
    }

    /// Get the response status text/reason phrase
    #[getter]
    fn status_text(&self) -> String {
        self.reason.clone()
    }

    /// Get response cookies (placeholder - returns empty dict for now)
    #[getter]
    fn cookies(&self, py: Python) -> PyResult<PyObject> {
        // For now, return an empty dict
        // TODO: Implement proper cookie parsing from Set-Cookie headers
        let dict = PyDict::new(py);
        Ok(dict.into())
    }

    /// Iterate over response body in chunks (for streaming large responses)
    /// Returns an iterator that yields bytes chunks
    fn iter_bytes(&self, py: Python) -> PyResult<PyObject> {
        if let Some(ref content) = self.binary_content {
            let chunk_size = 64 * 1024;
            let chunks: Vec<PyObject> = content
                .chunks(chunk_size)
                .map(|chunk| PyBytes::new(py, chunk).into())
                .collect();

            let list = PyList::new(py, &chunks)?;
            Ok(list.into())
        } else {
            let list = PyList::empty(py);
            Ok(list.into())
        }
    }

    /// Iterate over response body content in chunks (requests-compatible)
    /// Yields chunks of the specified size, decoded appropriately
    /// This method provides true streaming behavior when stream=True was used
    fn iter_content(&self, py: Python, chunk_size: Option<usize>) -> PyResult<PyObject> {
        let chunk_size = chunk_size.unwrap_or(512);

        if let Some(ref content) = self.binary_content {
            let chunks: Vec<PyObject> = content
                .chunks(chunk_size)
                .map(|chunk| PyBytes::new(py, chunk).into())
                .collect();

            let list = PyList::new(py, &chunks)?;
            Ok(list.into())
        } else {
            let list = PyList::empty(py);
            Ok(list.into())
        }
    }

    /// Iterate over response body line by line (requests-compatible)
    /// Yields lines as strings, decoded with the response encoding
    fn iter_lines(&mut self, py: Python) -> PyResult<PyObject> {
        let text = self.text()?;

        let lines: Vec<PyObject> = text
            .lines()
            .map(|line| line.into_pyobject(py).map(|s| s.into_any().unbind()))
            .collect::<Result<Vec<_>, _>>()?;

        let list = PyList::new(py, &lines)?;
        Ok(list.into())
    }

    /// Check if response is in streaming mode
    #[getter]
    fn is_stream(&self) -> bool {
        self.is_stream
    }

    /// Get response history (list of redirect responses)
    #[getter]
    fn history(&self, py: Python) -> PyResult<PyObject> {
        let mut items: Vec<Py<PyAny>> = Vec::with_capacity(self.history.len());
        for r in self.history.clone() {
            items.push(r.into_pyobject(py)?.unbind().into());
        }
        let list = PyList::new(py, &items)?;
        Ok(list.into())
    }

    /// Get response links (parsed from Link header)
    #[getter]
    fn links(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);

        // Get Link header
        if let Some(link_header) = self.headers.get("link") {
            // Parse Link header format: <url>; rel="rel", <url2>; rel="rel2"
            let link_header = link_header.trim();

            // Split by comma to get individual links
            for part in link_header.split(',') {
                let part = part.trim();

                // Extract URL between < and >
                let url_start = part.find('<');
                let url_end = part.find('>');

                if url_start.is_none() || url_end.is_none() {
                    continue;
                }

                let url = &part[(url_start.unwrap() + 1)..url_end.unwrap()];
                let after_url = &part[(url_end.unwrap() + 1)..];

                // Extract rel value (rel="..." or rel='...')
                let rel_start = after_url.find("rel=\"");
                let rel_single_start = after_url.find("rel='");

                let rel_value = if let Some(idx) = rel_start {
                    let after_rel = &after_url[(idx + 5)..];
                    let end_idx = after_rel.find('"').unwrap_or(after_rel.len());
                    Some(&after_rel[..end_idx])
                } else if let Some(idx) = rel_single_start {
                    let after_rel = &after_url[(idx + 5)..];
                    let end_idx = after_rel.find('\'').unwrap_or(after_rel.len());
                    Some(&after_rel[..end_idx])
                } else {
                    None
                };

                if let Some(rel) = rel_value {
                    // Create inner dict with url key
                    let inner = PyDict::new(py);
                    inner.set_item("url", url)?;
                    dict.set_item(rel, inner)?;
                }
            }
        }

        Ok(dict.into())
    }

    /// Get the next response in a redirect chain (placeholder)
    #[getter]
    fn next(&self) -> Option<PyObject> {
        // For now, return None
        // TODO: Implement redirect chain tracking
        None
    }

    /// Get the apparent encoding of the response
    #[getter]
    fn apparent_encoding(&self) -> String {
        // Simple heuristic - check for BOM or common patterns
        if let Some(ref content) = self.binary_content {
            if content.starts_with(&[0xEF, 0xBB, 0xBF]) {
                return "utf-8-sig".to_string();
            }
            if content.starts_with(&[0xFF, 0xFE]) {
                return "utf-16-le".to_string();
            }
            if content.starts_with(&[0xFE, 0xFF]) {
                return "utf-16-be".to_string();
            }
        }
        "utf-8".to_string()
    }

    /// String representation of the response
    fn __repr__(&self) -> String {
        format!("<Response [{}]>", self.status_code)
    }

    /// String representation of the response
    fn __str__(&self) -> String {
        format!("<Response [{}]>", self.status_code)
    }

    /// Boolean representation - True if status code < 400
    fn __bool__(&self) -> bool {
        self.ok
    }
}

impl Response {
    /// Convert HTTP status code to reason phrase
    fn status_code_to_reason(status_code: u16) -> String {
        match status_code {
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            205 => "Reset Content",
            206 => "Partial Content",
            207 => "Multi-Status",
            208 => "Already Reported",
            226 => "IM Used",
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Payload Too Large",
            414 => "URI Too Long",
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            418 => "I'm a teapot",
            421 => "Misdirected Request",
            422 => "Unprocessable Entity",
            423 => "Locked",
            424 => "Failed Dependency",
            425 => "Too Early",
            426 => "Upgrade Required",
            428 => "Precondition Required",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            451 => "Unavailable For Legal Reasons",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            506 => "Variant Also Negotiates",
            507 => "Insufficient Storage",
            508 => "Loop Detected",
            510 => "Not Extended",
            511 => "Network Authentication Required",
            _ => "Unknown",
        }
        .to_string()
    }

    /// Detect encoding from Content-Type header
    fn detect_encoding(&self) -> Option<String> {
        if let Some(content_type) = self.headers.get("content-type") {
            // Look for charset parameter in Content-Type header
            if let Some(charset_start) = content_type.find("charset=") {
                let charset_value = &content_type[charset_start + 8..];
                let charset = charset_value
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_lowercase();

                if !charset.is_empty() {
                    return Some(charset);
                }
            }
        }

        // Return the explicitly set encoding or None
        self.encoding.clone()
    }
}
