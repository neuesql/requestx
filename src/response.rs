//! HTTP Response implementation

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};
use std::time::Duration;

use crate::cookies::Cookies;
use crate::headers::Headers;
use crate::request::Request;
use crate::url::URL;

/// HTTP Response object
#[pyclass(name = "Response", subclass)]
pub struct Response {
    status_code: u16,
    headers: Headers,
    content: Vec<u8>,
    url: Option<URL>,
    request: Option<Request>,
    http_version: String,
    /// Whether http_version was set from a real HTTP response (vs default)
    has_real_http_version: bool,
    history: Vec<Response>,
    is_closed: bool,
    is_stream_consumed: bool,
    default_encoding: String,
    explicit_encoding: Option<String>,
    text_accessed: bool,
    elapsed: Duration,
    /// The original stream object (async or sync iterator)
    stream: Option<Py<PyAny>>,
    /// Whether the stream is async (true) or sync (false)
    is_async_stream: bool,
}

impl Clone for Response {
    fn clone(&self) -> Self {
        Self {
            status_code: self.status_code,
            headers: self.headers.clone(),
            content: self.content.clone(),
            url: self.url.clone(),
            request: self.request.clone(),
            http_version: self.http_version.clone(),
            has_real_http_version: self.has_real_http_version,
            history: self.history.clone(),
            is_closed: self.is_closed,
            is_stream_consumed: self.is_stream_consumed,
            default_encoding: self.default_encoding.clone(),
            explicit_encoding: self.explicit_encoding.clone(),
            text_accessed: self.text_accessed,
            elapsed: self.elapsed,
            stream: self
                .stream
                .as_ref()
                .map(|s| Python::attach(|py| s.clone_ref(py))),
            is_async_stream: self.is_async_stream,
        }
    }
}

impl Response {
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: Headers::new(),
            content: Vec::new(),
            url: None,
            request: None,
            http_version: "HTTP/1.1".to_string(),
            has_real_http_version: false,
            history: Vec::new(),
            is_closed: false,
            is_stream_consumed: false,
            default_encoding: "utf-8".to_string(),
            explicit_encoding: None,
            text_accessed: false,
            elapsed: Duration::ZERO,
            stream: None,
            is_async_stream: false,
        }
    }

    /// Set the elapsed time (public Rust API)
    pub fn set_elapsed(&mut self, elapsed: Duration) {
        self.elapsed = elapsed;
    }

    /// Set the request that generated this response (public Rust API)
    pub fn set_request_attr(&mut self, request: Option<Request>) {
        self.request = request;
    }

    pub fn from_reqwest(response: reqwest::blocking::Response, request: Option<Request>) -> PyResult<Self> {
        let status_code = response.status().as_u16();
        let headers = Headers::from_reqwest(response.headers());
        let url = URL::parse(response.url().as_str()).ok();
        let http_version = format!("{:?}", response.version());

        let content = response.bytes().map_err(|e| {
            if e.is_timeout() {
                crate::exceptions::ReadTimeout::new_err(format!("Read timeout: {}", e))
            } else {
                crate::exceptions::ReadError::new_err(format!("Failed to read response: {}", e))
            }
        })?;

        Ok(Self {
            status_code,
            headers,
            content: content.to_vec(),
            url,
            request,
            http_version,
            has_real_http_version: true,
            history: Vec::new(),
            is_closed: true,
            is_stream_consumed: true,
            default_encoding: "utf-8".to_string(),
            explicit_encoding: None,
            text_accessed: false,
            elapsed: Duration::ZERO,
            stream: None,
            is_async_stream: false,
        })
    }

    pub async fn from_reqwest_async(response: reqwest::Response, request: Option<Request>) -> PyResult<Self> {
        Self::from_reqwest_async_with_context(response, request, None).await
    }

    pub async fn from_reqwest_async_with_context(response: reqwest::Response, request: Option<Request>, timeout_context: Option<&str>) -> PyResult<Self> {
        let status_code = response.status().as_u16();
        let headers = Headers::from_reqwest(response.headers());
        let url = URL::parse(response.url().as_str()).ok();
        let http_version = format!("{:?}", response.version());

        let content = response.bytes().await.map_err(|e| {
            if e.is_timeout() {
                // Use timeout context if available, otherwise default to ReadTimeout
                match timeout_context {
                    Some("write") => crate::exceptions::WriteTimeout::new_err(format!("Write timeout: {}", e)),
                    Some("connect") => crate::exceptions::ConnectTimeout::new_err(format!("Connect timeout: {}", e)),
                    Some("pool") => crate::exceptions::PoolTimeout::new_err(format!("Pool timeout: {}", e)),
                    _ => crate::exceptions::ReadTimeout::new_err(format!("Read timeout: {}", e)),
                }
            } else {
                crate::exceptions::ReadError::new_err(format!("Failed to read response: {}", e))
            }
        })?;

        Ok(Self {
            status_code,
            headers,
            content: content.to_vec(),
            url,
            request,
            http_version,
            has_real_http_version: true,
            history: Vec::new(),
            is_closed: true,
            is_stream_consumed: true,
            default_encoding: "utf-8".to_string(),
            explicit_encoding: None,
            text_accessed: false,
            elapsed: Duration::ZERO,
            stream: None,
            is_async_stream: false,
        })
    }
}

#[pymethods]
impl Response {
    #[new]
    #[pyo3(signature = (status_code=200, *, headers=None, content=None, text=None, html=None, json=None, stream=None, request=None, extensions=None, history=None, default_encoding=None))]
    fn py_new(
        status_code: u16,
        headers: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyAny>>,
        text: Option<&str>,
        html: Option<&str>,
        json: Option<&Bound<'_, PyAny>>,
        stream: Option<&Bound<'_, PyAny>>,
        request: Option<Request>,
        extensions: Option<&Bound<'_, PyDict>>,
        history: Option<Vec<Response>>,
        default_encoding: Option<&str>,
    ) -> PyResult<Self> {
        let mut response = Self::new(status_code);
        response.request = request;
        response.default_encoding = default_encoding.unwrap_or("utf-8").to_string();

        if let Some(hist) = history {
            response.history = hist;
        }

        // Set headers
        if let Some(h) = headers {
            if let Ok(headers_obj) = h.extract::<Headers>() {
                response.headers = headers_obj;
            } else if let Ok(dict) = h.cast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    response.headers.set(k, v);
                }
            } else if let Ok(list) = h.cast::<PyList>() {
                // Handle list of tuples [(key, value), ...]
                for item in list.iter() {
                    if let Ok(tuple) = item.cast::<PyTuple>() {
                        if tuple.len() == 2 {
                            // Extract key and value, handling both bytes and string
                            let key_item = tuple.get_item(0)?;
                            let val_item = tuple.get_item(1)?;

                            let k = if let Ok(bytes) = key_item.extract::<Vec<u8>>() {
                                String::from_utf8_lossy(&bytes).into_owned()
                            } else {
                                key_item.extract::<String>()?
                            };

                            let v = if let Ok(bytes) = val_item.extract::<Vec<u8>>() {
                                String::from_utf8_lossy(&bytes).into_owned()
                            } else {
                                val_item.extract::<String>()?
                            };

                            response.headers.append(k, v);
                        }
                    }
                }
            }
        }

        // Handle content
        if let Some(c) = content {
            if let Ok(bytes) = c.extract::<Vec<u8>>() {
                response.content = bytes;
            } else if let Ok(s) = c.extract::<String>() {
                response.content = s.into_bytes();
            } else if let Ok(list) = c.cast::<pyo3::types::PyList>() {
                // Handle list of byte chunks
                let mut content_bytes = Vec::new();
                for item in list.iter() {
                    if let Ok(chunk) = item.extract::<Vec<u8>>() {
                        content_bytes.extend_from_slice(&chunk);
                    } else if let Ok(s) = item.extract::<String>() {
                        content_bytes.extend_from_slice(s.as_bytes());
                    }
                }
                response.content = content_bytes;
            } else if let Ok(tuple) = c.cast::<pyo3::types::PyTuple>() {
                // Handle tuple of byte chunks
                let mut content_bytes = Vec::new();
                for item in tuple.iter() {
                    if let Ok(chunk) = item.extract::<Vec<u8>>() {
                        content_bytes.extend_from_slice(&chunk);
                    } else if let Ok(s) = item.extract::<String>() {
                        content_bytes.extend_from_slice(s.as_bytes());
                    }
                }
                response.content = content_bytes;
            } else if c.hasattr("__aiter__")? {
                // Async iterator - store it for later async iteration
                response.stream = Some(c.clone().unbind());
                response.is_async_stream = true;
                // Don't set content-length for streaming responses
            } else if c.hasattr("__iter__")? {
                // Sync iterator - store it for later iteration
                response.stream = Some(c.clone().unbind());
                response.is_async_stream = false;
                // Don't set content-length for streaming responses
            } else {
                // Invalid content type
                return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                    "'content' must be bytes, str, or iterable, not {}",
                    c.get_type().name()?
                )));
            }
            // Don't set content-length if transfer-encoding is set (chunked transfer)
            if !response.headers.contains("content-length") && !response.headers.contains("transfer-encoding") {
                response
                    .headers
                    .set("Content-Length".to_string(), response.content.len().to_string());
            }
        }

        // Handle text
        if let Some(t) = text {
            response.content = t.as_bytes().to_vec();
            response
                .headers
                .set("Content-Length".to_string(), response.content.len().to_string());
            response
                .headers
                .set("Content-Type".to_string(), "text/plain; charset=utf-8".to_string());
        }

        // Handle HTML
        if let Some(h) = html {
            response.content = h.as_bytes().to_vec();
            response
                .headers
                .set("Content-Length".to_string(), response.content.len().to_string());
            response
                .headers
                .set("Content-Type".to_string(), "text/html; charset=utf-8".to_string());
        }

        // Handle JSON
        if let Some(j) = json {
            let json_str = crate::common::py_to_json_string(j)?;
            response.content = json_str.into_bytes();
            response
                .headers
                .set("Content-Length".to_string(), response.content.len().to_string());
            response
                .headers
                .set("Content-Type".to_string(), "application/json".to_string());
        }

        // For manually constructed responses, they start as not consumed and not closed
        // The stream is only consumed after iterating, and only closed after close() is called
        response.is_stream_consumed = false;
        response.is_closed = false;

        Ok(response)
    }

    #[getter]
    fn status_code(&self) -> u16 {
        self.status_code
    }

    #[getter]
    fn reason_phrase(&self) -> &str {
        status_code_to_reason(self.status_code)
    }

    #[getter]
    fn headers(&self) -> Headers {
        self.headers.clone()
    }

    #[getter]
    fn content<'py>(&mut self, py: Python<'py>) -> Bound<'py, PyBytes> {
        self.is_stream_consumed = true;
        self.is_closed = true;
        PyBytes::new(py, &self.content)
    }

    #[getter]
    fn text(&mut self) -> PyResult<String> {
        let encoding = self.get_encoding();

        // Mark stream as consumed and closed when accessing text
        self.is_stream_consumed = true;
        self.is_closed = true;
        self.text_accessed = true;

        // Decode based on encoding
        let enc_lower = encoding.to_lowercase();
        match enc_lower.as_str() {
            "utf-8" | "utf8" => String::from_utf8(self.content.clone()).map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode response: {}", e))),
            "latin-1" | "latin1" | "iso-8859-1" | "iso_8859_1" => {
                // Latin-1 is a simple 1:1 byte to char mapping
                Ok(self.content.iter().map(|&b| b as char).collect())
            }
            "ascii" | "us-ascii" => {
                // ASCII is UTF-8 compatible for bytes 0-127
                let valid: Result<String, _> = String::from_utf8(
                    self.content
                        .iter()
                        .map(|&b| if b > 127 { b'?' } else { b })
                        .collect(),
                );
                valid.map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode ASCII: {}", e)))
            }
            _ => {
                // For unknown encodings, try UTF-8 first, then fall back to latin-1
                String::from_utf8(self.content.clone()).or_else(|_| Ok(self.content.iter().map(|&b| b as char).collect()))
            }
        }
    }

    fn json(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let text = self.text()?;
        json_to_py(py, &text)
    }

    #[getter]
    fn url(&self) -> Option<URL> {
        // If URL is set, return it; otherwise fall back to request's URL
        if let Some(ref url) = self.url {
            Some(url.clone())
        } else if let Some(ref req) = self.request {
            Some(req.url_ref().clone())
        } else {
            None
        }
    }

    #[getter]
    fn request(&self) -> PyResult<Request> {
        self.request
            .clone()
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("The request instance has not been set on this response."))
    }

    #[setter]
    fn set_request(&mut self, request: Option<Request>) {
        self.request = request;
    }

    #[getter]
    fn http_version(&self) -> &str {
        &self.http_version
    }

    #[getter]
    fn history(&self) -> Vec<Response> {
        self.history.clone()
    }

    #[getter]
    fn cookies(&self) -> Cookies {
        let mut cookies = Cookies::new();
        if let Some(cookie_header) = self.headers.get("set-cookie", None) {
            // Simple cookie parsing
            for part in cookie_header.split(';') {
                let part = part.trim();
                if let Some(eq_idx) = part.find('=') {
                    let (name, value) = part.split_at(eq_idx);
                    let value = &value[1..]; // Skip '='
                    cookies.set(name.trim(), value.trim());
                    break; // Only get first name=value pair
                }
            }
        }
        cookies
    }

    #[getter]
    fn encoding(&self) -> String {
        self.get_encoding()
    }

    #[setter]
    fn set_encoding(&mut self, encoding: &str) -> PyResult<()> {
        if self.text_accessed {
            return Err(pyo3::exceptions::PyValueError::new_err("cannot set encoding after .text has been accessed"));
        }
        self.explicit_encoding = Some(encoding.to_string());
        Ok(())
    }

    #[getter]
    fn is_informational(&self) -> bool {
        (100..200).contains(&self.status_code)
    }

    #[getter]
    fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    #[getter]
    fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status_code)
    }

    #[getter]
    fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    #[getter]
    fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    #[getter]
    fn is_error(&self) -> bool {
        self.status_code >= 400
    }

    #[getter]
    fn is_closed(&self) -> bool {
        self.is_closed
    }

    #[getter]
    fn is_stream_consumed(&self) -> bool {
        self.is_stream_consumed
    }

    #[getter]
    fn num_bytes_downloaded(&self) -> usize {
        self.content.len()
    }

    #[getter]
    fn default_encoding(&self) -> &str {
        &self.default_encoding
    }

    #[getter]
    fn extensions(&self, py: Python<'_>) -> std::collections::HashMap<String, Py<PyAny>> {
        let mut extensions = std::collections::HashMap::new();
        // Only add http_version if it was set from a real HTTP response
        if self.has_real_http_version {
            let version_bytes = self.http_version.as_bytes().to_vec();
            extensions.insert("http_version".to_string(), PyBytes::new(py, &version_bytes).into_any().unbind());
        }
        extensions
    }

    /// Parse Link headers and return a dict of link relations
    #[getter]
    fn links(&self) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
        let mut result = std::collections::HashMap::new();

        if let Some(link_header) = self.headers.get("link", None) {
            // Parse Link header format: <url>; rel=value; type="value", <url2>; rel=value2
            for link in link_header.split(',') {
                let link = link.trim();
                if link.is_empty() {
                    continue;
                }

                let mut link_data = std::collections::HashMap::new();
                let mut parts = link.split(';');

                // First part is the URL in angle brackets
                if let Some(url_part) = parts.next() {
                    let url_part = url_part.trim();
                    if url_part.starts_with('<') && url_part.contains('>') {
                        let end = url_part.find('>').unwrap();
                        let url = &url_part[1..end];
                        link_data.insert("url".to_string(), url.to_string());

                        // Parse remaining parameters
                        for param in parts {
                            let param = param.trim();
                            if param.is_empty() {
                                continue;
                            }
                            if let Some(eq_idx) = param.find('=') {
                                let key = param[..eq_idx].trim().to_lowercase();
                                let value = param[eq_idx + 1..].trim();
                                // Remove quotes if present (both single and double)
                                let value = value.trim_matches('"').trim_matches('\'');
                                link_data.insert(key, value.to_string());
                            }
                        }

                        // Use 'rel' as the key if present, otherwise use URL
                        let key = link_data
                            .get("rel")
                            .cloned()
                            .unwrap_or_else(|| url.to_string());
                        result.insert(key, link_data);
                    }
                }
            }
        }

        result
    }

    #[getter]
    fn elapsed<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Import datetime.timedelta and create an instance
        let datetime = py.import("datetime")?;
        let timedelta = datetime.getattr("timedelta")?;

        // Convert Duration to seconds as float
        let total_secs = self.elapsed.as_secs_f64();

        // Create timedelta(seconds=total_secs)
        let kwargs = PyDict::new(py);
        kwargs.set_item("seconds", total_secs)?;
        timedelta.call((), Some(&kwargs))
    }

    fn raise_for_status(slf: PyRef<'_, Self>) -> PyResult<Py<Self>> {
        // Must have a request associated
        if slf.request.is_none() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Cannot call `raise_for_status` as the request instance has not been set on this response.",
            ));
        }

        // Only 2xx status codes are considered successful
        if slf.is_success() {
            return Ok(slf.into());
        }

        let self_ref = &*slf;

        // Get URL from response or from request if available
        let url_str = self_ref
            .url
            .as_ref()
            .map(|u| u.to_string())
            .or_else(|| self_ref.request.as_ref().map(|r| r.url_ref().to_string()))
            .unwrap_or_default();

        let message_prefix = if self_ref.is_informational() {
            "Informational response"
        } else if self_ref.is_redirect() {
            "Redirect response"
        } else if self_ref.is_client_error() {
            "Client error"
        } else if self_ref.is_server_error() {
            "Server error"
        } else {
            "Error"
        };

        // Build the error message
        let mut message = format!("{} '{} {}' for url '{}'", message_prefix, self_ref.status_code, self_ref.reason_phrase(), url_str);

        // Add redirect location for redirect responses
        if self_ref.is_redirect() {
            if let Some(location) = self_ref.headers.get("location", None) {
                message.push_str(&format!("\nRedirect location: '{}'", location));
            }
        }

        message.push_str(&format!(
            "\nFor more information check: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/{}",
            self_ref.status_code
        ));

        Err(crate::exceptions::HTTPStatusError::new_err(message))
    }

    /// Build the raise_for_status error message, or return None if the response is successful.
    /// Used by the Python wrapper to construct HTTPStatusError with request/response attributes.
    /// Extract charset from the Content-Type header. Returns None if not found.
    /// Used by the Python wrapper to avoid re-parsing Content-Type in Python.
    fn _extract_charset(&self) -> Option<String> {
        self.extract_charset()
    }

    fn _raise_for_status_message(&self) -> Option<String> {
        if self.is_success() {
            return None;
        }

        let url_str = self
            .url
            .as_ref()
            .map(|u| u.to_string())
            .or_else(|| self.request.as_ref().map(|r| r.url_ref().to_string()))
            .unwrap_or_default();

        let message_prefix = if self.is_informational() {
            "Informational response"
        } else if self.is_redirect() {
            "Redirect response"
        } else if self.is_client_error() {
            "Client error"
        } else if self.is_server_error() {
            "Server error"
        } else {
            "Error"
        };

        let mut message = format!("{} '{} {}' for url '{}'", message_prefix, self.status_code, self.reason_phrase(), url_str);

        if self.is_redirect() {
            if let Some(location) = self.headers.get("location", None) {
                message.push_str(&format!("\nRedirect location: '{}'", location));
            }
        }

        message.push_str(&format!("\nFor more information check: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/{}", self.status_code));

        Some(message)
    }

    fn read(&mut self) -> Vec<u8> {
        self.is_stream_consumed = true;
        self.is_closed = true;
        self.content.clone()
    }

    fn close(&mut self) {
        self.is_closed = true;
    }

    #[pyo3(signature = (chunk_size=None))]
    fn iter_raw<'py>(&mut self, py: Python<'py>, chunk_size: Option<usize>) -> PyResult<Py<PyAny>> {
        // Check if this is an async stream - if so, raise RuntimeError
        if self.stream.is_some() && self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call a sync iterator method on an async stream."));
        }

        // Allow iteration if we have content (even if stream was previously consumed)
        // Only block if we have no content AND stream was consumed
        if self.is_stream_consumed && self.content.is_empty() && self.stream.is_none() {
            return Err(crate::exceptions::StreamConsumed::new_err(
                "Attempted to read or stream content, but the content has already been streamed.",
            ));
        }

        // If we have a sync stream, return an iterator that wraps it
        if let Some(ref stream) = self.stream {
            self.is_stream_consumed = true;
            let stream_obj = stream.clone_ref(py);
            self.stream = None; // Consume the stream
            return Ok(SyncStreamRawIterator {
                stream: Some(stream_obj),
                chunk_size: chunk_size.unwrap_or(65536),
                buffer: Vec::new(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind());
        }

        self.is_stream_consumed = true;
        self.is_closed = true;
        Ok(RawIterator {
            content: self.content.clone(),
            position: 0,
            chunk_size: chunk_size.unwrap_or(65536),
        }
        .into_pyobject(py)?
        .into_any()
        .unbind())
    }

    #[pyo3(signature = (chunk_size=None))]
    fn iter_bytes(&mut self, py: Python<'_>, chunk_size: Option<usize>) -> PyResult<Py<PyAny>> {
        // Check if this is an async stream - if so, raise RuntimeError
        if self.stream.is_some() && self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call a sync iterator method on an async stream."));
        }

        // Allow iteration if we have content (even if stream was previously consumed)
        // Only block if we have no content AND stream was consumed
        if self.is_stream_consumed && self.content.is_empty() && self.stream.is_none() {
            return Err(crate::exceptions::StreamConsumed::new_err(
                "Attempted to read or stream content, but the content has already been streamed.",
            ));
        }

        // If we have a sync stream, return an iterator that wraps it
        if let Some(ref stream) = self.stream {
            self.is_stream_consumed = true;
            let stream_obj = stream.clone_ref(py);
            self.stream = None; // Consume the stream
            return Ok(SyncStreamBytesIterator {
                stream: Some(stream_obj),
                chunk_size: chunk_size.unwrap_or(65536),
                buffer: Vec::new(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind());
        }

        self.is_stream_consumed = true;
        self.is_closed = true;
        Ok(BytesIterator {
            content: self.content.clone(),
            position: 0,
            chunk_size: chunk_size.unwrap_or(65536),
        }
        .into_pyobject(py)?
        .into_any()
        .unbind())
    }

    #[pyo3(signature = (chunk_size=None))]
    fn iter_text(&mut self, chunk_size: Option<usize>) -> PyResult<TextIterator> {
        // Check if this is an async stream - if so, raise RuntimeError
        if self.stream.is_some() && self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call a sync iterator method on an async stream."));
        }

        // Allow iteration if we have content (even if stream was previously consumed)
        if self.is_stream_consumed && self.content.is_empty() && self.stream.is_none() {
            return Err(crate::exceptions::StreamConsumed::new_err(
                "Attempted to read or stream content, but the content has already been streamed.",
            ));
        }
        let text = String::from_utf8(self.content.clone()).map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode response: {}", e)))?;
        self.is_stream_consumed = true;
        self.is_closed = true;
        Ok(TextIterator {
            text,
            position: 0,
            chunk_size: chunk_size.unwrap_or(65536),
        })
    }

    fn iter_lines(&mut self) -> PyResult<LinesIterator> {
        // Check if this is an async stream - if so, raise RuntimeError
        if self.stream.is_some() && self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call a sync iterator method on an async stream."));
        }

        // Allow iteration if we have content (even if stream was previously consumed)
        if self.is_stream_consumed && self.content.is_empty() && self.stream.is_none() {
            return Err(crate::exceptions::StreamConsumed::new_err(
                "Attempted to read or stream content, but the content has already been streamed.",
            ));
        }
        let text = String::from_utf8(self.content.clone()).map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode response: {}", e)))?;
        self.is_stream_consumed = true;
        self.is_closed = true;

        // Handle all line endings: \r\n, \n, or \r
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\r' {
                // Check if \r\n
                if chars.peek() == Some(&'\n') {
                    chars.next(); // consume the \n
                }
                lines.push(current_line);
                current_line = String::new();
            } else if c == '\n' {
                lines.push(current_line);
                current_line = String::new();
            } else {
                current_line.push(c);
            }
        }

        // Add any remaining content as the last line
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        Ok(LinesIterator { lines, position: 0 })
    }

    // Async methods
    fn aread<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // aread() always works - it returns cached content and marks stream as consumed
        self.is_stream_consumed = true;
        self.is_closed = true;
        let content = self.content.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(content) })
    }

    #[pyo3(signature = (chunk_size=None))]
    fn aiter_raw(&mut self, py: Python<'_>, chunk_size: Option<usize>) -> PyResult<Py<PyAny>> {
        // Check if this is a sync stream - if so, raise RuntimeError
        if self.stream.is_some() && !self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call an async iterator method on a sync stream."));
        }

        if self.is_stream_consumed && self.stream.is_none() {
            return Err(crate::exceptions::StreamConsumed::new_err(
                "Attempted to read or stream content, but the content has already been streamed.",
            ));
        }

        // If we have an async stream, return an iterator that wraps it
        if let Some(ref stream) = self.stream {
            self.is_stream_consumed = true;
            let stream_obj = stream.clone_ref(py);
            self.stream = None; // Consume the stream
            return Ok(AsyncStreamRawIterator {
                stream: Some(stream_obj),
                aiter: None,
                chunk_size: chunk_size.unwrap_or(65536),
                buffer: Vec::new(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind());
        }

        self.is_stream_consumed = true;
        self.is_closed = true;
        Ok(AsyncRawIterator {
            content: self.content.clone(),
            position: 0,
            chunk_size: chunk_size.unwrap_or(65536),
        }
        .into_pyobject(py)?
        .into_any()
        .unbind())
    }

    #[pyo3(signature = (chunk_size=None))]
    fn aiter_bytes(&mut self, py: Python<'_>, chunk_size: Option<usize>) -> PyResult<Py<PyAny>> {
        // Check if this is a sync stream - if so, raise RuntimeError
        if self.stream.is_some() && !self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call an async iterator method on a sync stream."));
        }

        if self.is_stream_consumed && self.stream.is_none() {
            return Err(crate::exceptions::StreamConsumed::new_err(
                "Attempted to read or stream content, but the content has already been streamed.",
            ));
        }

        // If we have an async stream, return an iterator that wraps it
        if let Some(ref stream) = self.stream {
            self.is_stream_consumed = true;
            let stream_obj = stream.clone_ref(py);
            self.stream = None; // Consume the stream
            return Ok(AsyncStreamBytesIterator {
                stream: Some(stream_obj),
                aiter: None,
                chunk_size: chunk_size.unwrap_or(65536),
                buffer: Vec::new(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind());
        }

        self.is_stream_consumed = true;
        self.is_closed = true;
        Ok(AsyncBytesIterator {
            content: self.content.clone(),
            position: 0,
            chunk_size: chunk_size.unwrap_or(65536),
        }
        .into_pyobject(py)?
        .into_any()
        .unbind())
    }

    #[pyo3(signature = (chunk_size=None))]
    fn aiter_text(&mut self, chunk_size: Option<usize>) -> PyResult<AsyncTextIterator> {
        // Check if this is a sync stream - if so, raise RuntimeError
        if self.stream.is_some() && !self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call an async iterator method on a sync stream."));
        }

        if self.is_stream_consumed && self.stream.is_none() {
            return Err(crate::exceptions::StreamConsumed::new_err(
                "Attempted to read or stream content, but the content has already been streamed.",
            ));
        }
        let text = String::from_utf8(self.content.clone()).map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode response: {}", e)))?;
        self.is_stream_consumed = true;
        self.is_closed = true;
        Ok(AsyncTextIterator {
            text,
            position: 0,
            chunk_size: chunk_size.unwrap_or(65536),
        })
    }

    fn aiter_lines(&mut self) -> PyResult<AsyncLinesIterator> {
        // Check if this is a sync stream - if so, raise RuntimeError
        if self.stream.is_some() && !self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call an async iterator method on a sync stream."));
        }

        if self.is_stream_consumed && self.stream.is_none() {
            return Err(crate::exceptions::StreamConsumed::new_err(
                "Attempted to read or stream content, but the content has already been streamed.",
            ));
        }
        let text = String::from_utf8(self.content.clone()).map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode response: {}", e)))?;
        self.is_stream_consumed = true;
        self.is_closed = true;

        // Handle all line endings
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\r' {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                lines.push(current_line);
                current_line = String::new();
            } else if c == '\n' {
                lines.push(current_line);
                current_line = String::new();
            } else {
                current_line.push(c);
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        Ok(AsyncLinesIterator { lines, position: 0 })
    }

    fn aclose<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Check if this is a sync stream - if so, raise RuntimeError
        if self.stream.is_some() && !self.is_async_stream {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("Attempted to call an async method on a sync stream."));
        }

        self.is_closed = true;
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(()) })
    }

    fn __repr__(&self) -> String {
        format!("<Response [{} {}]>", self.status_code, self.reason_phrase())
    }

    fn __eq__(&self, other: &Response) -> bool {
        self.status_code == other.status_code && self.content == other.content
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(&mut self, _exc_type: Option<&Bound<'_, PyAny>>, _exc_val: Option<&Bound<'_, PyAny>>, _exc_tb: Option<&Bound<'_, PyAny>>) -> bool {
        self.close();
        false
    }

    /// Set content from Python (used by aread wrapper)
    fn _set_content(&mut self, content: Vec<u8>) {
        self.content = content;
        self.is_stream_consumed = true;
        self.is_closed = true;
    }

    /// Set content without closing the response (for iter_bytes)
    fn _set_content_only(&mut self, content: Vec<u8>) {
        self.content = content;
    }
}

impl Response {
    fn get_encoding(&self) -> String {
        // If encoding was explicitly set, use it
        if let Some(ref enc) = self.explicit_encoding {
            return enc.clone();
        }
        // Try to detect from content-type header
        if let Some(charset) = self.extract_charset() {
            return charset;
        }
        self.default_encoding.clone()
    }

    /// Extract charset from Content-Type header, e.g. "text/html; charset=utf-8" -> "utf-8".
    /// Returns None if no charset is specified.
    fn extract_charset(&self) -> Option<String> {
        let content_type = self.headers.get("content-type", None)?;
        parse_charset_from_content_type(&content_type)
    }

    /// Set a header on the response
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.set(name.to_string(), value.to_string());
    }

    /// Set the content (body) of the response
    pub fn set_content(&mut self, content: Vec<u8>) {
        self.content = content;
        self.is_stream_consumed = true;
        self.is_closed = true;
    }

    /// Set all headers on the response
    pub fn set_headers(&mut self, headers: Headers) {
        self.headers = headers;
    }

    /// Set the URL on the response
    pub fn set_url(&mut self, url: URL) {
        self.url = Some(url);
    }

    /// Set the HTTP version string
    pub fn set_http_version(&mut self, version: String) {
        self.http_version = version;
    }
}

/// Iterator for response bytes
#[pyclass]
pub struct BytesIterator {
    content: Vec<u8>,
    position: usize,
    chunk_size: usize,
}

#[pymethods]
impl BytesIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<Vec<u8>> {
        if self.position >= self.content.len() {
            None
        } else {
            let end = std::cmp::min(self.position + self.chunk_size, self.content.len());
            let chunk = self.content[self.position..end].to_vec();
            self.position = end;
            Some(chunk)
        }
    }
}

/// Iterator for response text
#[pyclass]
pub struct TextIterator {
    text: String,
    position: usize,
    chunk_size: usize,
}

#[pymethods]
impl TextIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<String> {
        if self.position >= self.text.len() {
            None
        } else {
            let end = std::cmp::min(self.position + self.chunk_size, self.text.len());
            let chunk = self.text[self.position..end].to_string();
            self.position = end;
            Some(chunk)
        }
    }
}

/// Iterator for response lines
#[pyclass]
pub struct LinesIterator {
    lines: Vec<String>,
    position: usize,
}

#[pymethods]
impl LinesIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<String> {
        if self.position >= self.lines.len() {
            None
        } else {
            let line = self.lines[self.position].clone();
            self.position += 1;
            Some(line)
        }
    }
}

/// Iterator for raw response bytes
#[pyclass]
pub struct RawIterator {
    content: Vec<u8>,
    position: usize,
    chunk_size: usize,
}

#[pymethods]
impl RawIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        if self.position >= self.content.len() {
            None
        } else {
            let end = std::cmp::min(self.position + self.chunk_size, self.content.len());
            let chunk = &self.content[self.position..end];
            self.position = end;
            Some(PyBytes::new(py, chunk))
        }
    }
}

/// Async iterator for raw response bytes
#[pyclass]
pub struct AsyncRawIterator {
    content: Vec<u8>,
    position: usize,
    chunk_size: usize,
}

#[pymethods]
impl AsyncRawIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        if self.position >= self.content.len() {
            Ok(None)
        } else {
            let end = std::cmp::min(self.position + self.chunk_size, self.content.len());
            let chunk = self.content[self.position..end].to_vec();
            self.position = end;
            let fut = pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(chunk) })?;
            Ok(Some(fut))
        }
    }
}

/// Async iterator for decoded response bytes
#[pyclass]
pub struct AsyncBytesIterator {
    content: Vec<u8>,
    position: usize,
    chunk_size: usize,
}

#[pymethods]
impl AsyncBytesIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        if self.position >= self.content.len() {
            Ok(None)
        } else {
            let end = std::cmp::min(self.position + self.chunk_size, self.content.len());
            let chunk = self.content[self.position..end].to_vec();
            self.position = end;
            let fut = pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(chunk) })?;
            Ok(Some(fut))
        }
    }
}

/// Async iterator for response text
#[pyclass]
pub struct AsyncTextIterator {
    text: String,
    position: usize,
    chunk_size: usize,
}

#[pymethods]
impl AsyncTextIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        if self.position >= self.text.len() {
            Ok(None)
        } else {
            let end = std::cmp::min(self.position + self.chunk_size, self.text.len());
            let chunk = self.text[self.position..end].to_string();
            self.position = end;
            let fut = pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(chunk) })?;
            Ok(Some(fut))
        }
    }
}

/// Async iterator for response lines
#[pyclass]
pub struct AsyncLinesIterator {
    lines: Vec<String>,
    position: usize,
}

#[pymethods]
impl AsyncLinesIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        if self.position >= self.lines.len() {
            Ok(None)
        } else {
            let line = self.lines[self.position].clone();
            self.position += 1;
            let fut = pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(line) })?;
            Ok(Some(fut))
        }
    }
}

/// Sync iterator that wraps a Python sync stream for raw bytes
#[pyclass]
pub struct SyncStreamRawIterator {
    stream: Option<Py<PyAny>>,
    chunk_size: usize,
    buffer: Vec<u8>,
}

#[pymethods]
impl SyncStreamRawIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        // If we have buffered data, return a chunk from it
        if !self.buffer.is_empty() {
            let end = std::cmp::min(self.chunk_size, self.buffer.len());
            let chunk: Vec<u8> = self.buffer.drain(..end).collect();
            return Ok(Some(PyBytes::new(py, &chunk)));
        }

        // Get next chunk from the stream
        if let Some(ref stream) = self.stream {
            let iter = stream.call_method0(py, "__iter__")?;
            loop {
                match iter.call_method0(py, "__next__") {
                    Ok(item) => {
                        let chunk: Vec<u8> = item.extract(py)?;
                        if chunk.is_empty() {
                            continue; // Skip empty chunks
                        }
                        if chunk.len() <= self.chunk_size {
                            return Ok(Some(PyBytes::new(py, &chunk)));
                        } else {
                            // Buffer excess and return chunk_size
                            self.buffer.extend_from_slice(&chunk[self.chunk_size..]);
                            return Ok(Some(PyBytes::new(py, &chunk[..self.chunk_size])));
                        }
                    }
                    Err(e) if e.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) => {
                        self.stream = None;
                        return Ok(None);
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(None)
    }
}

/// Sync iterator that wraps a Python sync stream for decoded bytes
#[pyclass]
pub struct SyncStreamBytesIterator {
    stream: Option<Py<PyAny>>,
    chunk_size: usize,
    buffer: Vec<u8>,
}

#[pymethods]
impl SyncStreamBytesIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Vec<u8>>> {
        // If we have buffered data, return a chunk from it
        if !self.buffer.is_empty() {
            let end = std::cmp::min(self.chunk_size, self.buffer.len());
            let chunk: Vec<u8> = self.buffer.drain(..end).collect();
            return Ok(Some(chunk));
        }

        // Get next chunk from the stream
        if let Some(ref stream) = self.stream {
            let iter = stream.call_method0(py, "__iter__")?;
            loop {
                match iter.call_method0(py, "__next__") {
                    Ok(item) => {
                        let chunk: Vec<u8> = item.extract(py)?;
                        if chunk.is_empty() {
                            continue; // Skip empty chunks
                        }
                        if chunk.len() <= self.chunk_size {
                            return Ok(Some(chunk));
                        } else {
                            // Buffer excess and return chunk_size
                            self.buffer.extend_from_slice(&chunk[self.chunk_size..]);
                            return Ok(Some(chunk[..self.chunk_size].to_vec()));
                        }
                    }
                    Err(e) if e.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) => {
                        self.stream = None;
                        return Ok(None);
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(None)
    }
}

/// Async iterator that wraps a Python async stream for raw bytes
#[pyclass]
pub struct AsyncStreamRawIterator {
    stream: Option<Py<PyAny>>, // The original async generator/iterator
    aiter: Option<Py<PyAny>>,  // The __aiter__ result (stored after first call)
    chunk_size: usize,
    buffer: Vec<u8>,
}

#[pymethods]
impl AsyncStreamRawIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        // Initialize aiter if needed
        if self.aiter.is_none() {
            if let Some(ref stream) = self.stream {
                let aiter = stream.call_method0(py, "__aiter__")?;
                self.aiter = Some(aiter);
            }
        }

        // Get next chunk from the async iterator
        if let Some(ref aiter) = self.aiter {
            let anext = aiter.call_method0(py, "__anext__")?;
            return Ok(Some(anext.into_bound(py)));
        }
        Ok(None)
    }
}

/// Async iterator that wraps a Python async stream for decoded bytes
#[pyclass]
pub struct AsyncStreamBytesIterator {
    stream: Option<Py<PyAny>>,
    aiter: Option<Py<PyAny>>,
    chunk_size: usize,
    buffer: Vec<u8>,
}

#[pymethods]
impl AsyncStreamBytesIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        if self.aiter.is_none() {
            if let Some(ref stream) = self.stream {
                let aiter = stream.call_method0(py, "__aiter__")?;
                self.aiter = Some(aiter);
            }
        }

        if let Some(ref aiter) = self.aiter {
            let anext = aiter.call_method0(py, "__anext__")?;
            return Ok(Some(anext.into_bound(py)));
        }
        Ok(None)
    }
}

/// Decompress data based on encoding.
/// Supports: gzip, deflate, br (brotli), zstd.
/// Returns the original data for identity or unknown encodings.
#[pyfunction]
pub fn decompress(py: Python<'_>, data: &[u8], encoding: &str) -> PyResult<Py<PyBytes>> {
    use std::io::Read;

    if data.is_empty() {
        return Ok(PyBytes::new(py, data).unbind());
    }

    let encoding = encoding.to_lowercase();
    let encoding = encoding.trim();

    let decompressed = match encoding {
        "gzip" => {
            let mut decoder = flate2::read::GzDecoder::new(data);
            let mut buf = Vec::new();
            decoder
                .read_to_end(&mut buf)
                .map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode gzip content: {}", e)))?;
            buf
        }
        "deflate" => {
            // Deflate can be raw deflate or zlib-wrapped; try raw first
            let mut decoder = flate2::read::DeflateDecoder::new(data);
            let mut buf = Vec::new();
            match decoder.read_to_end(&mut buf) {
                Ok(_) => buf,
                Err(_) => {
                    // Try zlib-wrapped
                    let mut decoder = flate2::read::ZlibDecoder::new(data);
                    let mut buf2 = Vec::new();
                    decoder
                        .read_to_end(&mut buf2)
                        .map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode deflate content: {}", e)))?;
                    buf2
                }
            }
        }
        "br" => {
            let mut buf = Vec::new();
            let mut decoder = brotli::Decompressor::new(data, 4096);
            decoder
                .read_to_end(&mut buf)
                .map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode brotli content: {}", e)))?;
            buf
        }
        "zstd" => {
            let mut decoder = zstd::Decoder::new(data).map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to create zstd decoder: {}", e)))?;
            let mut buf = Vec::new();
            decoder
                .read_to_end(&mut buf)
                .map_err(|e| crate::exceptions::DecodingError::new_err(format!("Failed to decode zstd content: {}", e)))?;
            buf
        }
        "identity" | "" => {
            return Ok(PyBytes::new(py, data).unbind());
        }
        _ => {
            // Unknown encoding - return as-is
            return Ok(PyBytes::new(py, data).unbind());
        }
    };

    Ok(PyBytes::new(py, &decompressed).unbind())
}

/// Parse charset from a Content-Type header value string.
/// e.g. "text/html; charset=utf-8" -> Some("utf-8")
///      "application/json" -> None
fn parse_charset_from_content_type(content_type: &str) -> Option<String> {
    for part in content_type.split(';') {
        let part = part.trim();
        if part.to_lowercase().starts_with("charset=") {
            let charset = part[8..].trim_matches('"').trim_matches('\'');
            if charset.is_empty() {
                return None;
            }
            return Some(charset.to_string());
        }
    }
    None
}

fn status_code_to_reason(code: u16) -> &'static str {
    match code {
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
        _ => "",
    }
}

/// Parse JSON string to Python object
fn json_to_py(py: Python<'_>, json_str: &str) -> PyResult<Py<PyAny>> {
    let value: sonic_rs::Value = sonic_rs::from_str(json_str).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("JSON parse error: {}", e)))?;
    json_value_to_py(py, &value)
}

/// Detect JSON encoding from BOM or null-byte patterns, decode bytes to string,
/// strip BOM character, and parse JSON using sonic-rs. Returns a Python object.
#[pyfunction]
pub fn json_from_bytes(py: Python<'_>, data: &[u8]) -> PyResult<Py<PyAny>> {
    if data.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err("JSON parse error: empty content"));
    }

    let text = decode_json_bytes(data)?;

    // Strip BOM character if present (U+FEFF)
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);

    json_to_py(py, text)
}

/// Detect JSON encoding from BOM or null byte patterns.
/// Returns the encoding name (e.g., "utf-16-be") or None for plain UTF-8.
#[pyfunction]
pub fn guess_json_utf(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }

    // Check BOMs first (order matters: UTF-32 before UTF-16)
    if data.len() >= 4 {
        if data.starts_with(b"\x00\x00\xfe\xff") {
            return Some("utf-32-be".to_string());
        }
        if data.starts_with(b"\xff\xfe\x00\x00") {
            return Some("utf-32-le".to_string());
        }
    }
    if data.starts_with(b"\xfe\xff") {
        return Some("utf-16-be".to_string());
    }
    if data.starts_with(b"\xff\xfe") {
        return Some("utf-16-le".to_string());
    }
    if data.starts_with(b"\xef\xbb\xbf") {
        return Some("utf-8-sig".to_string());
    }

    // No BOM - detect by null byte patterns
    if data.len() >= 4 {
        let null_count = data[..4].iter().filter(|&&b| b == 0).count();

        // UTF-32: 3 null bytes per character
        if null_count == 3 {
            if data[0] == 0 && data[1] == 0 && data[2] == 0 {
                return Some("utf-32-be".to_string());
            }
            if data[1] == 0 && data[2] == 0 && data[3] == 0 {
                return Some("utf-32-le".to_string());
            }
        }

        // UTF-16: 1 null byte per character (for ASCII range)
        if null_count >= 1 {
            if data[0] == 0 && data[2] == 0 {
                return Some("utf-16-be".to_string());
            }
            if data[1] == 0 && data[3] == 0 {
                return Some("utf-16-le".to_string());
            }
        }
    } else if data.len() >= 2 {
        if data[0] == 0 {
            return Some("utf-16-be".to_string());
        }
        if data[1] == 0 {
            return Some("utf-16-le".to_string());
        }
    }

    // Default: plain UTF-8 (no special encoding)
    None
}

/// Detect encoding of JSON bytes and decode to String.
fn decode_json_bytes(data: &[u8]) -> PyResult<String> {
    // Check BOMs first (order matters: UTF-32 before UTF-16)
    if data.starts_with(b"\x00\x00\xfe\xff") {
        return decode_utf32(data, true);
    }
    if data.starts_with(b"\xff\xfe\x00\x00") {
        return decode_utf32(data, false);
    }
    if data.starts_with(b"\xfe\xff") {
        return decode_utf16(&data[2..], true);
    }
    if data.starts_with(b"\xff\xfe") {
        return decode_utf16(&data[2..], false);
    }
    if data.starts_with(b"\xef\xbb\xbf") {
        // UTF-8 BOM - skip 3 bytes
        return String::from_utf8(data[3..].to_vec()).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("UTF-8 decode error: {}", e)));
    }

    // No BOM - detect by null byte patterns
    if data.len() >= 4 {
        let null_count = data[..4].iter().filter(|&&b| b == 0).count();
        if null_count == 3 {
            if data[0] == 0 && data[1] == 0 && data[2] == 0 {
                return decode_utf32(data, true);
            }
            if data[1] == 0 && data[2] == 0 && data[3] == 0 {
                return decode_utf32(data, false);
            }
        }
        if null_count >= 1 {
            if data[0] == 0 && data[2] == 0 {
                return decode_utf16(data, true);
            }
            if data[1] == 0 && data[3] == 0 {
                return decode_utf16(data, false);
            }
        }
    } else if data.len() >= 2 {
        if data[0] == 0 {
            return decode_utf16(data, true);
        }
        if data[1] == 0 {
            return decode_utf16(data, false);
        }
    }

    // Default: UTF-8
    String::from_utf8(data.to_vec()).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("UTF-8 decode error: {}", e)))
}

fn decode_utf16(data: &[u8], big_endian: bool) -> PyResult<String> {
    if data.len() % 2 != 0 {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid UTF-16 data: odd number of bytes"));
    }
    let u16_iter = data.chunks_exact(2).map(|chunk| {
        if big_endian {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_le_bytes([chunk[0], chunk[1]])
        }
    });
    String::from_utf16(&u16_iter.collect::<Vec<u16>>()).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("UTF-16 decode error: {}", e)))
}

fn decode_utf32(data: &[u8], big_endian: bool) -> PyResult<String> {
    // Skip BOM if present
    let start = if big_endian && data.starts_with(b"\x00\x00\xfe\xff") {
        4
    } else if !big_endian && data.starts_with(b"\xff\xfe\x00\x00") {
        4
    } else {
        0
    };
    let data = &data[start..];
    if data.len() % 4 != 0 {
        return Err(pyo3::exceptions::PyValueError::new_err("Invalid UTF-32 data: not a multiple of 4 bytes"));
    }
    let mut result = String::with_capacity(data.len() / 4);
    for chunk in data.chunks_exact(4) {
        let code_point = if big_endian {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        let c = char::from_u32(code_point).ok_or_else(|| pyo3::exceptions::PyValueError::new_err(format!("Invalid UTF-32 code point: {}", code_point)))?;
        result.push(c);
    }
    Ok(result)
}

/// Convert sonic_rs::Value to Python object
fn json_value_to_py(py: Python<'_>, value: &sonic_rs::Value) -> PyResult<Py<PyAny>> {
    use pyo3::types::{PyDict, PyList};
    use sonic_rs::{JsonContainerTrait, JsonValueTrait};

    if value.is_null() {
        return Ok(py.None());
    }

    if let Some(b) = value.as_bool() {
        return Ok(pyo3::types::PyBool::new(py, b)
            .to_owned()
            .into_any()
            .unbind());
    }

    if let Some(i) = value.as_i64() {
        return Ok(i.into_pyobject(py)?.into_any().unbind());
    }

    if let Some(f) = value.as_f64() {
        return Ok(f.into_pyobject(py)?.into_any().unbind());
    }

    if let Some(s) = value.as_str() {
        return Ok(s.into_pyobject(py)?.into_any().unbind());
    }

    if value.is_array() {
        let list = PyList::empty(py);
        if let Some(arr) = value.as_array() {
            for item in arr.iter() {
                list.append(json_value_to_py(py, item)?)?;
            }
        }
        return Ok(list.into_any().unbind());
    }

    if value.is_object() {
        let dict = PyDict::new(py);
        if let Some(obj) = value.as_object() {
            for (k, v) in obj.iter() {
                dict.set_item(k, json_value_to_py(py, v)?)?;
            }
        }
        return Ok(dict.into_any().unbind());
    }

    Ok(py.None())
}
