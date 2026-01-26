//! Response types for requestx

use crate::error::{Error, Result};
use crate::types::{Cookies, Headers};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use std::collections::HashMap;

/// HTTP Response wrapper
#[pyclass(name = "Response")]
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code
    #[pyo3(get)]
    pub status_code: u16,

    /// Response headers
    headers: Headers,

    /// Response body as bytes
    content: Vec<u8>,

    /// Final URL after redirects
    #[pyo3(get)]
    pub url: String,

    /// HTTP version
    #[pyo3(get)]
    pub http_version: String,

    /// Response cookies
    cookies: Cookies,

    /// Elapsed time in seconds
    #[pyo3(get)]
    pub elapsed: f64,

    /// Request method
    #[pyo3(get)]
    pub request_method: String,

    /// History of redirect responses
    history: Vec<Response>,

    /// Encoding (detected or specified)
    encoding: Option<String>,

    /// Reason phrase
    #[pyo3(get)]
    pub reason_phrase: String,
}

#[pymethods]
impl Response {
    /// Get response headers
    #[getter]
    pub fn headers(&self) -> Headers {
        self.headers.clone()
    }

    /// Get response cookies
    #[getter]
    pub fn cookies(&self) -> Cookies {
        self.cookies.clone()
    }

    /// Get response content as bytes
    #[getter]
    pub fn content<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.content)
    }

    /// Get response text (decoded content)
    #[getter]
    pub fn text(&self) -> PyResult<String> {
        let encoding = self.detect_encoding();
        self.decode_content(&encoding)
            .map_err(|e| Error::decode(e.to_string()).into())
    }

    /// Get encoding
    #[getter]
    pub fn encoding(&self) -> Option<String> {
        self.encoding
            .clone()
            .or_else(|| Some(self.detect_encoding()))
    }

    /// Set encoding
    #[setter]
    pub fn set_encoding(&mut self, encoding: Option<String>) {
        self.encoding = encoding;
    }

    /// Parse response as JSON
    pub fn json<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let text = self.text()?;
        let value: sonic_rs::Value = sonic_rs::from_str(&text).map_err(|e| Error::decode(format!("JSON decode error: {e}")))?;
        json_to_py(py, &value)
    }

    /// Get redirect history
    #[getter]
    pub fn history(&self) -> Vec<Response> {
        self.history.clone()
    }

    /// Check if request was successful (2xx status)
    #[getter]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Check if response is a redirect (3xx status)
    #[getter]
    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status_code)
    }

    /// Check if response is a client error (4xx status)
    #[getter]
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    /// Check if response is a server error (5xx status)
    #[getter]
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    /// Check if response indicates an error (4xx or 5xx)
    #[getter]
    pub fn is_error(&self) -> bool {
        self.status_code >= 400
    }

    /// Check if response has a redirect location header
    #[getter]
    pub fn has_redirect_location(&self) -> bool {
        self.headers.inner.contains_key("location")
    }

    /// Get next redirect URL if present
    #[getter]
    pub fn next_url(&self) -> Option<String> {
        self.headers.get("location")
    }

    /// Get content length if present
    #[getter]
    pub fn content_length(&self) -> Option<usize> {
        self.headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
    }

    /// Get content type if present
    #[getter]
    pub fn content_type(&self) -> Option<String> {
        self.headers.get("content-type")
    }

    /// Raise an exception if the response indicates an error
    pub fn raise_for_status(&self) -> PyResult<()> {
        if self.is_error() {
            Err(Error::status(self.status_code, format!("{} {} for url {}", self.status_code, self.reason_phrase, self.url)).into())
        } else {
            Ok(())
        }
    }

    /// Read response content (compatibility method)
    pub fn read<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        self.content(py)
    }

    /// Iterate over response content in chunks
    pub fn iter_bytes<'py>(&self, py: Python<'py>, chunk_size: Option<usize>) -> PyResult<Bound<'py, PyList>> {
        let chunk_size = chunk_size.unwrap_or(8192);
        let chunks: Vec<Bound<'py, PyBytes>> = self
            .content
            .chunks(chunk_size)
            .map(|chunk| PyBytes::new(py, chunk))
            .collect();
        PyList::new(py, chunks)
    }

    /// Iterate over response lines
    pub fn iter_lines(&self) -> PyResult<Vec<String>> {
        let text = self.text()?;
        Ok(text.lines().map(|s| s.to_string()).collect())
    }

    /// Get response links from Link header
    pub fn links<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        if let Some(link_header) = self.headers.get("link") {
            // Parse Link header format: <url>; rel="name", ...
            for link in link_header.split(',') {
                let parts: Vec<&str> = link.split(';').collect();
                if let Some(url_part) = parts.first() {
                    let url = url_part
                        .trim()
                        .trim_start_matches('<')
                        .trim_end_matches('>');
                    for part in parts.iter().skip(1) {
                        let part = part.trim();
                        if let Some(rel) = part.strip_prefix("rel=") {
                            let rel = rel.trim_matches('"').trim_matches('\'');
                            let link_dict = PyDict::new(py);
                            link_dict.set_item("url", url)?;
                            dict.set_item(rel, link_dict)?;
                        }
                    }
                }
            }
        }
        Ok(dict)
    }

    /// Close the response (no-op for now, included for compatibility)
    pub fn close(&self) {}

    pub fn __repr__(&self) -> String {
        format!("<Response [{} {}]>", self.status_code, self.reason_phrase)
    }

    pub fn __str__(&self) -> String {
        self.__repr__()
    }

    pub fn __bool__(&self) -> bool {
        self.is_success()
    }

    pub fn __len__(&self) -> usize {
        self.content.len()
    }
}

impl Response {
    /// Create a new Response from reqwest response data
    pub fn new(status_code: u16, headers: Headers, content: Vec<u8>, url: String, http_version: String, cookies: Cookies, elapsed: f64, request_method: String, reason_phrase: String) -> Self {
        Self {
            status_code,
            headers,
            content,
            url,
            http_version,
            cookies,
            elapsed,
            request_method,
            history: Vec::new(),
            encoding: None,
            reason_phrase,
        }
    }

    /// Set redirect history
    pub fn with_history(mut self, history: Vec<Response>) -> Self {
        self.history = history;
        self
    }

    /// Set default encoding (used by client when default_encoding is configured)
    pub fn set_default_encoding(&mut self, encoding: String) {
        // Only set if not already explicitly set
        if self.encoding.is_none() {
            self.encoding = Some(encoding);
        }
    }

    /// Detect encoding from Content-Type header or content
    fn detect_encoding(&self) -> String {
        // First, check Content-Type header for charset
        if let Some(content_type) = self.headers.get("content-type") {
            if let Some(charset_pos) = content_type.to_lowercase().find("charset=") {
                let charset_start = charset_pos + 8;
                let charset: String = content_type[charset_start..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !charset.is_empty() {
                    return charset.to_lowercase();
                }
            }
        }

        // Check for BOM
        if self.content.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return "utf-8".to_string();
        }
        if self.content.starts_with(&[0xFE, 0xFF]) {
            return "utf-16-be".to_string();
        }
        if self.content.starts_with(&[0xFF, 0xFE]) {
            return "utf-16-le".to_string();
        }

        // Default to UTF-8
        "utf-8".to_string()
    }

    /// Decode content using the specified encoding
    fn decode_content(&self, encoding: &str) -> Result<String> {
        match encoding.to_lowercase().as_str() {
            "utf-8" | "utf8" => String::from_utf8(self.content.clone()).or_else(|_| Ok(String::from_utf8_lossy(&self.content).to_string())),
            "ascii" | "us-ascii" => Ok(self.content.iter().map(|&b| b as char).collect()),
            "iso-8859-1" | "latin-1" | "latin1" => Ok(self.content.iter().map(|&b| b as char).collect()),
            _ => {
                // Fall back to UTF-8 with lossy conversion
                Ok(String::from_utf8_lossy(&self.content).to_string())
            }
        }
    }

    /// Create response from reqwest response (async)
    pub async fn from_reqwest(response: reqwest::Response, start_time: std::time::Instant, request_method: &str) -> Result<Self> {
        let status_code = response.status().as_u16();
        let reason_phrase = response
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();
        let url = response.url().to_string();
        let http_version = format!("{:?}", response.version());

        // Extract headers
        let headers = Headers::from_reqwest_headers(response.headers());

        // Extract cookies
        let mut cookies_map = HashMap::new();
        for cookie in response.cookies() {
            cookies_map.insert(cookie.name().to_string(), cookie.value().to_string());
        }
        let cookies = Cookies { inner: cookies_map };

        // Get body
        let content = response.bytes().await?.to_vec();
        let elapsed = start_time.elapsed().as_secs_f64();

        Ok(Self::new(status_code, headers, content, url, http_version, cookies, elapsed, request_method.to_string(), reason_phrase))
    }
}

/// Convert sonic_rs::Value to Python object
fn json_to_py<'py>(py: Python<'py>, value: &sonic_rs::Value) -> PyResult<Bound<'py, PyAny>> {
    use pyo3::types::{PyBool, PyFloat, PyString};
    use sonic_rs::{JsonContainerTrait, JsonValueTrait};

    // Use as_* methods which return Option to check and extract in one step
    if let Some(b) = value.as_bool() {
        Ok(PyBool::new(py, b).to_owned().into_any())
    } else if let Some(i) = value.as_i64() {
        Ok(i.into_pyobject(py)?.to_owned().into_any())
    } else if let Some(u) = value.as_u64() {
        // Only use u64 if it doesn't fit in i64
        if u > i64::MAX as u64 {
            Ok(u.into_pyobject(py)?.to_owned().into_any())
        } else {
            Ok((u as i64).into_pyobject(py)?.to_owned().into_any())
        }
    } else if let Some(f) = value.as_f64() {
        Ok(PyFloat::new(py, f).into_any())
    } else if let Some(s) = value.as_str() {
        Ok(PyString::new(py, s).into_any())
    } else if let Some(arr) = value.as_array() {
        let list: Vec<Bound<'py, PyAny>> = arr
            .iter()
            .map(|v| json_to_py(py, v))
            .collect::<PyResult<_>>()?;
        Ok(PyList::new(py, list)?.into_any())
    } else if let Some(obj) = value.as_object() {
        let dict = PyDict::new(py);
        for (k, v) in obj.iter() {
            dict.set_item(k, json_to_py(py, v)?)?;
        }
        Ok(dict.into_any())
    } else {
        // null or unknown type
        Ok(py.None().into_bound(py))
    }
}
