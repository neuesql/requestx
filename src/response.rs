//! Response types for requestx

use crate::error::{Error, Result};
use crate::types::{Cookies, Headers, Request};
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use std::collections::HashMap;

/// HTTP Response wrapper
#[pyclass(name = "Response")]
#[derive(Debug)]
pub struct Response {
    /// HTTP status code
    #[pyo3(get)]
    pub status_code: u16,

    /// Response headers
    headers: Headers,

    /// Response body as bytes
    content: Vec<u8>,

    /// Final URL after redirects
    url_str: String,

    /// HTTP version
    #[pyo3(get)]
    pub http_version: String,

    /// Response cookies
    cookies: Cookies,

    /// Elapsed time in seconds
    #[pyo3(get)]
    pub elapsed: f64,

    /// Request method (kept for backward compatibility)
    request_method: String,

    /// History of redirect responses
    history: Vec<Response>,

    /// Encoding (detected or specified)
    encoding: Option<String>,

    /// Default encoding (used when charset not in Content-Type)
    default_encoding: Option<String>,

    /// Reason phrase
    #[pyo3(get)]
    pub reason_phrase: String,

    /// The original request that generated this response
    request: Option<Request>,

    /// Whether the response is closed
    is_closed: bool,

    /// Whether the stream has been consumed
    is_stream_consumed: bool,

    /// Whether text has been accessed (locks encoding)
    text_accessed: std::sync::atomic::AtomicBool,
}

impl Clone for Response {
    fn clone(&self) -> Self {
        Self {
            status_code: self.status_code,
            headers: self.headers.clone(),
            content: self.content.clone(),
            url_str: self.url_str.clone(),
            http_version: self.http_version.clone(),
            cookies: self.cookies.clone(),
            elapsed: self.elapsed,
            request_method: self.request_method.clone(),
            history: self.history.clone(),
            encoding: self.encoding.clone(),
            default_encoding: self.default_encoding.clone(),
            reason_phrase: self.reason_phrase.clone(),
            request: self.request.clone(),
            is_closed: self.is_closed,
            is_stream_consumed: self.is_stream_consumed,
            text_accessed: std::sync::atomic::AtomicBool::new(
                self.text_accessed.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
}

#[pymethods]
impl Response {
    /// Create a Response for testing purposes (HTTPX compatibility)
    #[new]
    #[pyo3(signature = (
        status_code=200,
        headers=None,
        content=None,
        text=None,
        html=None,
        json=None,
        stream=None,
        request=None,
        extensions=None,
        default_encoding=None
    ))]
    pub fn py_new(
        py: Python<'_>,
        status_code: u16,
        headers: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyAny>>,
        text: Option<&str>,
        html: Option<&str>,
        json: Option<&Bound<'_, PyAny>>,
        #[allow(unused)] stream: Option<&Bound<'_, PyAny>>,
        request: Option<Request>,
        #[allow(unused)] extensions: Option<&Bound<'_, PyDict>>,
        default_encoding: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        // Parse headers
        let response_headers = if let Some(h) = headers {
            if let Ok(headers_obj) = h.extract::<Headers>() {
                headers_obj
            } else if h.is_instance_of::<PyDict>() {
                let dict = h.downcast::<PyDict>()?;
                let mut header_map = IndexMap::new();
                for (key, value) in dict.iter() {
                    let key_str: String = key.extract()?;
                    let key_lower = key_str.to_lowercase();
                    let value_str: String = value.extract()?;
                    header_map.entry(key_lower).or_insert_with(Vec::new).push(value_str);
                }
                Headers { inner: header_map }
            } else if let Ok(list) = h.downcast::<PyList>() {
                let mut header_map = IndexMap::new();
                for item in list.iter() {
                    let tuple: (String, String) = item.extract()?;
                    let key_lower = tuple.0.to_lowercase();
                    header_map.entry(key_lower).or_insert_with(Vec::new).push(tuple.1);
                }
                Headers { inner: header_map }
            } else {
                Headers::default()
            }
        } else {
            Headers::default()
        };

        // Determine content bytes and whether to add Content-Type
        // HTTPX behavior: Content-Type is added for text/html/json params but NOT for content param
        let mut add_content_type: Option<&str> = None;
        let content_bytes: Vec<u8> = if let Some(c) = content {
            if let Ok(bytes) = c.extract::<Vec<u8>>() {
                bytes
            } else if let Ok(s) = c.extract::<String>() {
                s.into_bytes()
            } else {
                Vec::new()
            }
        } else if let Some(t) = text {
            add_content_type = Some("text/plain; charset=utf-8");
            t.as_bytes().to_vec()
        } else if let Some(h) = html {
            add_content_type = Some("text/html; charset=utf-8");
            h.as_bytes().to_vec()
        } else if let Some(j) = json {
            add_content_type = Some("application/json");
            // Serialize JSON with compact separators (HTTPX compatibility)
            let json_mod = py.import("json")?;
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("separators", (",", ":"))?;
            let json_str: String = json_mod.call_method("dumps", (j,), Some(&kwargs))?.extract()?;
            json_str.into_bytes()
        } else {
            Vec::new()
        };

        // Add Content-Length header if content is present (HTTPX compatibility)
        let mut final_headers = response_headers;
        if !content_bytes.is_empty() {
            final_headers.inner.entry("content-length".to_string())
                .or_insert_with(Vec::new)
                .push(content_bytes.len().to_string());
        }
        // Add Content-Type header only for text/html/json params (HTTPX compatibility)
        if let Some(ct) = add_content_type {
            final_headers.inner.entry("content-type".to_string())
                .or_insert_with(Vec::new)
                .push(ct.to_string());
        }

        // Get reason phrase
        let reason = get_reason_phrase(status_code);

        // Process default_encoding - can be a string or callable
        let default_enc = if let Some(de) = default_encoding {
            if let Ok(s) = de.extract::<String>() {
                Some(s)
            } else if de.is_callable() {
                // Call the function with content bytes
                let content_bytes_py = pyo3::types::PyBytes::new(py, &content_bytes);
                if let Ok(result) = de.call1((content_bytes_py,)) {
                    result.extract::<Option<String>>().ok().flatten()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            status_code,
            headers: final_headers,
            content: content_bytes,
            url_str: String::new(),
            http_version: "HTTP/1.1".to_string(),
            cookies: Cookies::default(),
            elapsed: 0.0,
            request_method: "GET".to_string(),
            history: Vec::new(),
            encoding: None,
            default_encoding: default_enc,
            reason_phrase: reason,
            request,
            is_closed: true,
            is_stream_consumed: true,
            text_accessed: std::sync::atomic::AtomicBool::new(false),
        })
    }

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
        // Mark text as accessed (locks encoding changes)
        self.text_accessed.store(true, std::sync::atomic::Ordering::Relaxed);
        // Use explicit encoding if set, otherwise detect
        let encoding = self.encoding.clone().unwrap_or_else(|| self.detect_encoding());
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

    /// Get charset encoding from Content-Type header (without validation)
    #[getter]
    pub fn charset_encoding(&self) -> Option<String> {
        self.extract_charset_from_content_type()
    }

    /// Set encoding
    #[setter]
    pub fn set_encoding(&mut self, encoding: Option<String>) -> PyResult<()> {
        if self.text_accessed.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Cannot set encoding after accessing text"
            ));
        }
        self.encoding = encoding;
        Ok(())
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

    /// Get the final URL after redirects (as string for backward compatibility)
    #[getter]
    pub fn url(&self) -> String {
        self.url_str.clone()
    }

    /// Get the original request that generated this response
    #[getter]
    pub fn request(&self) -> Option<Request> {
        self.request.clone()
    }

    /// Get the request method (for backward compatibility)
    #[getter]
    pub fn request_method(&self) -> String {
        self.request_method.clone()
    }

    /// Whether the response is closed
    #[getter]
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// Whether the stream has been consumed
    #[getter]
    pub fn is_stream_consumed(&self) -> bool {
        self.is_stream_consumed
    }

    /// Check if response is informational (1xx status)
    #[getter]
    pub fn is_informational(&self) -> bool {
        (100..200).contains(&self.status_code)
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
        self.headers.get_value("location")
    }

    /// Get content length if present
    #[getter]
    pub fn content_length(&self) -> Option<usize> {
        self.headers
            .get_value("content-length")
            .and_then(|v| v.parse().ok())
    }

    /// Get content type if present
    #[getter]
    pub fn content_type(&self) -> Option<String> {
        self.headers.get_value("content-type")
    }

    /// Raise an exception if the response indicates an error
    /// HTTPX behavior: raises for 1xx (informational), 3xx (redirect), 4xx (client error), 5xx (server error)
    pub fn raise_for_status(&self) -> PyResult<()> {
        // HTTPX requires a request to be set
        let request_url = match &self.request {
            Some(req) => req.url_str(),
            None => return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Cannot call `raise_for_status` as the request instance has not been set on this response."
            )),
        };

        // 2xx responses are success - don't raise
        if self.is_success() {
            return Ok(());
        }

        let (error_type, redirect_location) = if self.is_informational() {
            ("Informational response", None)
        } else if self.is_redirect() {
            ("Redirect response", self.headers.get_value("location"))
        } else if self.is_client_error() {
            ("Client error", None)
        } else if self.is_server_error() {
            ("Server error", None)
        } else {
            // Unknown status range
            ("Error", None)
        };

        let mut message = format!(
            "{} '{} {}' for url '{}'\n",
            error_type, self.status_code, self.reason_phrase, request_url
        );

        if let Some(location) = redirect_location {
            message.push_str(&format!("Redirect location: '{}'\n", location));
        }

        message.push_str(&format!(
            "For more information check: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/{}",
            self.status_code
        ));

        Err(Error::status(self.status_code, message).into())
    }

    /// Read response content (compatibility method)
    pub fn read<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        self.content(py)
    }

    /// Async read response content (HTTPX compatibility)
    /// For non-streaming responses, the body is already read, so this just returns the content
    pub fn aread<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let content = self.content.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(content) })
    }

    /// Async close the response (HTTPX compatibility)
    /// For non-streaming responses, this is a no-op
    pub fn aclose<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(()) })
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
        if let Some(link_header) = self.headers.get_value("link") {
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
            url_str: url,
            http_version,
            cookies,
            elapsed,
            request_method,
            history: Vec::new(),
            encoding: None,
            default_encoding: None,
            reason_phrase,
            request: None,
            is_closed: true,          // For non-streaming responses, body is already read
            is_stream_consumed: true, // Body is already consumed
            text_accessed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Set redirect history
    pub fn with_history(mut self, history: Vec<Response>) -> Self {
        self.history = history;
        self
    }

    /// Set the request that generated this response
    pub fn with_request(mut self, request: Request) -> Self {
        self.request = Some(request);
        self
    }

    /// Set default encoding (used by client when default_encoding is configured)
    pub fn set_default_encoding(&mut self, encoding: String) {
        // Only set if not already explicitly set
        if self.encoding.is_none() {
            self.encoding = Some(encoding);
        }
    }

    /// Set the request that generated this response (mutable version)
    pub fn set_request(&mut self, request: Request) {
        self.request = Some(request);
    }

    /// Check if encoding is valid
    fn is_valid_encoding(encoding: &str) -> bool {
        // List of common valid encodings
        matches!(encoding.to_lowercase().as_str(),
            "utf-8" | "utf8" | "utf-16" | "utf-16-be" | "utf-16-le" |
            "utf-32" | "utf-32-be" | "utf-32-le" |
            "ascii" | "us-ascii" | "iso-8859-1" | "latin-1" | "latin1" |
            "iso-8859-2" | "iso-8859-15" | "windows-1252" | "cp1252" |
            "shift_jis" | "shift-jis" | "euc-jp" | "euc-kr" | "gb2312" | "gbk" | "gb18030" |
            "big5" | "koi8-r" | "koi8-u"
        )
    }

    /// Extract charset from Content-Type header without validation
    fn extract_charset_from_content_type(&self) -> Option<String> {
        if let Some(content_type) = self.headers.get_value("content-type") {
            if let Some(charset_pos) = content_type.to_lowercase().find("charset=") {
                let charset_start = charset_pos + 8;
                let charset: String = content_type[charset_start..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !charset.is_empty() {
                    return Some(charset.to_lowercase());
                }
            }
        }
        None
    }

    /// Detect encoding from Content-Type header or content
    fn detect_encoding(&self) -> String {
        // First, check Content-Type header for charset
        if let Some(charset) = self.extract_charset_from_content_type() {
            // Only return the charset if it's a valid encoding
            if Self::is_valid_encoding(&charset) {
                return charset;
            }
            // Invalid charset, fall through to other detection methods
        }

        // Use default_encoding if provided
        if let Some(ref default_enc) = self.default_encoding {
            return default_enc.clone();
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
            "windows-1252" | "cp1252" => Ok(self.decode_cp1252()),
            _ => {
                // Fall back to UTF-8 with lossy conversion
                Ok(String::from_utf8_lossy(&self.content).to_string())
            }
        }
    }

    /// Decode Windows-1252 (cp1252) content
    fn decode_cp1252(&self) -> String {
        // Windows-1252 to Unicode mapping for 0x80-0x9F range
        const CP1252_MAP: [char; 32] = [
            '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
            '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
            '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
            '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
        ];

        self.content.iter().map(|&b| {
            if b >= 0x80 && b <= 0x9F {
                CP1252_MAP[(b - 0x80) as usize]
            } else {
                b as char
            }
        }).collect()
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

/// Get reason phrase for a status code
fn get_reason_phrase(status_code: u16) -> String {
    match status_code {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",
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
        413 => "Request Entity Too Large",
        414 => "Request-URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Requested Range Not Satisfiable",
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
        _ => "",
    }.to_string()
}
