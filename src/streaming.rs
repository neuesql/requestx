//! Streaming response types for requestx

use crate::error::Error;
use crate::types::{Cookies, Headers};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

/// Sync streaming response - reads body incrementally
#[pyclass(name = "StreamingResponse")]
pub struct StreamingResponse {
    /// HTTP status code
    #[pyo3(get)]
    pub status_code: u16,

    /// Response headers
    headers: Headers,

    /// Final URL after redirects
    #[pyo3(get)]
    pub url: String,

    /// HTTP version
    #[pyo3(get)]
    pub http_version: String,

    /// Response cookies
    cookies: Cookies,

    /// Elapsed time in seconds (time to first byte)
    #[pyo3(get)]
    pub elapsed: f64,

    /// Request method
    #[pyo3(get)]
    pub request_method: String,

    /// Reason phrase
    #[pyo3(get)]
    pub reason_phrase: String,

    /// The underlying blocking response for streaming
    inner: Arc<Mutex<Option<reqwest::blocking::Response>>>,

    /// Default chunk size
    chunk_size: usize,

    /// Whether the stream is closed
    closed: Arc<Mutex<bool>>,
}

#[pymethods]
impl StreamingResponse {
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
            Err(Error::status(
                self.status_code,
                format!(
                    "{} {} for url {}",
                    self.status_code, self.reason_phrase, self.url
                ),
            )
            .into())
        } else {
            Ok(())
        }
    }

    /// Read all remaining content and return as bytes
    pub fn read<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let mut inner = self.inner.lock().map_err(|e| Error::request(e.to_string()))?;
        if let Some(response) = inner.take() {
            let bytes = response.bytes().map_err(Error::from)?;
            *self.closed.lock().map_err(|e| Error::request(e.to_string()))? = true;
            Ok(PyBytes::new(py, &bytes))
        } else {
            Err(Error::request("Response body already consumed").into())
        }
    }

    /// Read all remaining content as text
    pub fn text(&self) -> PyResult<String> {
        let mut inner = self.inner.lock().map_err(|e| Error::request(e.to_string()))?;
        if let Some(response) = inner.take() {
            let text = response.text().map_err(Error::from)?;
            *self.closed.lock().map_err(|e| Error::request(e.to_string()))? = true;
            Ok(text)
        } else {
            Err(Error::request("Response body already consumed").into())
        }
    }

    /// Iterate over response bytes in chunks
    /// Returns a BytesIterator
    #[pyo3(signature = (chunk_size=None))]
    pub fn iter_bytes(&self, chunk_size: Option<usize>) -> PyResult<BytesIterator> {
        let chunk_size = chunk_size.unwrap_or(self.chunk_size);
        Ok(BytesIterator {
            inner: self.inner.clone(),
            closed: self.closed.clone(),
            chunk_size,
            buffer: Vec::new(),
        })
    }

    /// Iterate over response text in chunks
    #[pyo3(signature = (chunk_size=None))]
    pub fn iter_text(&self, chunk_size: Option<usize>) -> PyResult<TextIterator> {
        let chunk_size = chunk_size.unwrap_or(self.chunk_size);
        Ok(TextIterator {
            inner: self.inner.clone(),
            closed: self.closed.clone(),
            chunk_size,
            buffer: Vec::new(),
            encoding: self.detect_encoding(),
        })
    }

    /// Iterate over response lines
    pub fn iter_lines(&self) -> PyResult<LinesIterator> {
        Ok(LinesIterator {
            inner: self.inner.clone(),
            closed: self.closed.clone(),
            buffer: String::new(),
            encoding: self.detect_encoding(),
        })
    }

    /// Iterate over raw bytes (alias for iter_bytes)
    #[pyo3(signature = (chunk_size=None))]
    pub fn iter_raw(&self, chunk_size: Option<usize>) -> PyResult<BytesIterator> {
        self.iter_bytes(chunk_size)
    }

    /// Close the streaming response
    pub fn close(&self) -> PyResult<()> {
        let mut inner = self.inner.lock().map_err(|e| Error::request(e.to_string()))?;
        *inner = None;
        *self.closed.lock().map_err(|e| Error::request(e.to_string()))? = true;
        Ok(())
    }

    /// Context manager enter
    pub fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager exit
    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    pub fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        self.close()
    }

    pub fn __repr__(&self) -> String {
        format!("<StreamingResponse [{} {}]>", self.status_code, self.reason_phrase)
    }
}

impl StreamingResponse {
    /// Create a new StreamingResponse from reqwest blocking response
    pub fn from_blocking(
        response: reqwest::blocking::Response,
        elapsed: f64,
        request_method: &str,
    ) -> Self {
        let status_code = response.status().as_u16();
        let reason_phrase = response
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();
        let url = response.url().to_string();
        let http_version = format!("{:?}", response.version());

        let headers = Headers::from_reqwest_headers(response.headers());

        let mut cookies_map = HashMap::new();
        for cookie in response.cookies() {
            cookies_map.insert(cookie.name().to_string(), cookie.value().to_string());
        }
        let cookies = Cookies { inner: cookies_map };

        Self {
            status_code,
            headers,
            url,
            http_version,
            cookies,
            elapsed,
            request_method: request_method.to_string(),
            reason_phrase,
            inner: Arc::new(Mutex::new(Some(response))),
            chunk_size: 4096,
            closed: Arc::new(Mutex::new(false)),
        }
    }

    fn detect_encoding(&self) -> String {
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
        "utf-8".to_string()
    }
}

/// Iterator for streaming bytes
#[pyclass]
pub struct BytesIterator {
    inner: Arc<Mutex<Option<reqwest::blocking::Response>>>,
    closed: Arc<Mutex<bool>>,
    chunk_size: usize,
    buffer: Vec<u8>,
}

#[pymethods]
impl BytesIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        use std::io::Read;

        let mut inner = self.inner.lock().map_err(|e| Error::request(e.to_string()))?;
        if let Some(ref mut response) = *inner {
            self.buffer.resize(self.chunk_size, 0);
            match response.read(&mut self.buffer) {
                Ok(0) => {
                    // EOF
                    *self.closed.lock().map_err(|e| Error::request(e.to_string()))? = true;
                    Ok(None)
                }
                Ok(n) => {
                    Ok(Some(PyBytes::new(py, &self.buffer[..n])))
                }
                Err(e) => Err(Error::request(e.to_string()).into()),
            }
        } else {
            Ok(None)
        }
    }
}

/// Iterator for streaming text
#[pyclass]
pub struct TextIterator {
    inner: Arc<Mutex<Option<reqwest::blocking::Response>>>,
    closed: Arc<Mutex<bool>>,
    chunk_size: usize,
    buffer: Vec<u8>,
    #[allow(dead_code)]
    encoding: String,
}

#[pymethods]
impl TextIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyString>>> {
        use std::io::Read;

        let mut inner = self.inner.lock().map_err(|e| Error::request(e.to_string()))?;
        if let Some(ref mut response) = *inner {
            self.buffer.resize(self.chunk_size, 0);
            match response.read(&mut self.buffer) {
                Ok(0) => {
                    *self.closed.lock().map_err(|e| Error::request(e.to_string()))? = true;
                    Ok(None)
                }
                Ok(n) => {
                    let text = String::from_utf8_lossy(&self.buffer[..n]).to_string();
                    Ok(Some(PyString::new(py, &text)))
                }
                Err(e) => Err(Error::request(e.to_string()).into()),
            }
        } else {
            Ok(None)
        }
    }
}

/// Iterator for streaming lines
#[pyclass]
pub struct LinesIterator {
    inner: Arc<Mutex<Option<reqwest::blocking::Response>>>,
    closed: Arc<Mutex<bool>>,
    buffer: String,
    #[allow(dead_code)]
    encoding: String,
}

#[pymethods]
impl LinesIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyString>>> {
        use std::io::Read;

        // First check if we have a complete line in the buffer
        if let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 1..].to_string();
            return Ok(Some(PyString::new(py, &line)));
        }

        // Read more data
        let mut inner = self.inner.lock().map_err(|e| Error::request(e.to_string()))?;
        if let Some(ref mut response) = *inner {
            let mut chunk = vec![0u8; 4096];
            loop {
                match response.read(&mut chunk) {
                    Ok(0) => {
                        // EOF - return remaining buffer if any
                        *self.closed.lock().map_err(|e| Error::request(e.to_string()))? = true;
                        if !self.buffer.is_empty() {
                            let line = std::mem::take(&mut self.buffer);
                            return Ok(Some(PyString::new(py, &line)));
                        }
                        return Ok(None);
                    }
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&chunk[..n]);
                        self.buffer.push_str(&text);

                        // Check for complete line
                        if let Some(pos) = self.buffer.find('\n') {
                            let line = self.buffer[..pos].to_string();
                            self.buffer = self.buffer[pos + 1..].to_string();
                            return Ok(Some(PyString::new(py, &line)));
                        }
                    }
                    Err(e) => return Err(Error::request(e.to_string()).into()),
                }
            }
        } else {
            Ok(None)
        }
    }
}

/// Async streaming response - reads body incrementally
#[pyclass(name = "AsyncStreamingResponse")]
pub struct AsyncStreamingResponse {
    /// HTTP status code
    #[pyo3(get)]
    pub status_code: u16,

    /// Response headers
    headers: Headers,

    /// Final URL after redirects
    #[pyo3(get)]
    pub url: String,

    /// HTTP version
    #[pyo3(get)]
    pub http_version: String,

    /// Response cookies
    cookies: Cookies,

    /// Elapsed time in seconds (time to first byte)
    #[pyo3(get)]
    pub elapsed: f64,

    /// Request method
    #[pyo3(get)]
    pub request_method: String,

    /// Reason phrase
    #[pyo3(get)]
    pub reason_phrase: String,

    /// The underlying async response for streaming
    inner: Arc<TokioMutex<Option<reqwest::Response>>>,

    /// Default chunk size
    chunk_size: usize,

    /// Whether the stream is closed
    closed: Arc<TokioMutex<bool>>,
}

#[pymethods]
impl AsyncStreamingResponse {
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
            Err(Error::status(
                self.status_code,
                format!(
                    "{} {} for url {}",
                    self.status_code, self.reason_phrase, self.url
                ),
            )
            .into())
        } else {
            Ok(())
        }
    }

    /// Async read all remaining content as bytes
    pub fn aread<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let closed = self.closed.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            if let Some(response) = guard.take() {
                let bytes = response.bytes().await.map_err(Error::from)?;
                *closed.lock().await = true;
                Ok(bytes.to_vec())
            } else {
                Err(Error::request("Response body already consumed").into())
            }
        })
    }

    /// Async read all remaining content as text
    pub fn atext<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let closed = self.closed.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            if let Some(response) = guard.take() {
                let text = response.text().await.map_err(Error::from)?;
                *closed.lock().await = true;
                Ok(text)
            } else {
                Err(Error::request("Response body already consumed").into())
            }
        })
    }

    /// Async iterate over response bytes - returns an async iterator
    #[pyo3(signature = (chunk_size=None))]
    pub fn aiter_bytes(&self, chunk_size: Option<usize>) -> PyResult<AsyncBytesIterator> {
        let chunk_size = chunk_size.unwrap_or(self.chunk_size);
        Ok(AsyncBytesIterator {
            inner: self.inner.clone(),
            closed: self.closed.clone(),
            chunk_size,
        })
    }

    /// Async iterate over response text
    #[pyo3(signature = (chunk_size=None))]
    pub fn aiter_text(&self, chunk_size: Option<usize>) -> PyResult<AsyncTextIterator> {
        let chunk_size = chunk_size.unwrap_or(self.chunk_size);
        Ok(AsyncTextIterator {
            inner: self.inner.clone(),
            closed: self.closed.clone(),
            chunk_size,
            encoding: self.detect_encoding(),
        })
    }

    /// Async iterate over response lines
    pub fn aiter_lines(&self) -> PyResult<AsyncLinesIterator> {
        Ok(AsyncLinesIterator {
            inner: self.inner.clone(),
            closed: self.closed.clone(),
            buffer: Arc::new(TokioMutex::new(String::new())),
            encoding: self.detect_encoding(),
        })
    }

    /// Async iterate over raw bytes (alias for aiter_bytes)
    #[pyo3(signature = (chunk_size=None))]
    pub fn aiter_raw(&self, chunk_size: Option<usize>) -> PyResult<AsyncBytesIterator> {
        self.aiter_bytes(chunk_size)
    }

    /// Async close the streaming response
    pub fn aclose<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let closed = self.closed.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            *guard = None;
            *closed.lock().await = true;
            Ok(())
        })
    }

    /// Async context manager enter
    pub fn __aenter__<'py>(slf: Py<Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let slf_clone = slf.clone_ref(py);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(slf_clone)
        })
    }

    /// Async context manager exit
    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    pub fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let closed = self.closed.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            *guard = None;
            *closed.lock().await = true;
            Ok(())
        })
    }

    pub fn __repr__(&self) -> String {
        format!("<AsyncStreamingResponse [{} {}]>", self.status_code, self.reason_phrase)
    }
}

impl AsyncStreamingResponse {
    /// Create a new AsyncStreamingResponse from reqwest async response
    pub fn from_async(
        response: reqwest::Response,
        elapsed: f64,
        request_method: &str,
    ) -> Self {
        let status_code = response.status().as_u16();
        let reason_phrase = response
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();
        let url = response.url().to_string();
        let http_version = format!("{:?}", response.version());

        let headers = Headers::from_reqwest_headers(response.headers());

        let mut cookies_map = HashMap::new();
        for cookie in response.cookies() {
            cookies_map.insert(cookie.name().to_string(), cookie.value().to_string());
        }
        let cookies = Cookies { inner: cookies_map };

        Self {
            status_code,
            headers,
            url,
            http_version,
            cookies,
            elapsed,
            request_method: request_method.to_string(),
            reason_phrase,
            inner: Arc::new(TokioMutex::new(Some(response))),
            chunk_size: 4096,
            closed: Arc::new(TokioMutex::new(false)),
        }
    }

    fn detect_encoding(&self) -> String {
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
        "utf-8".to_string()
    }
}

/// Async iterator for streaming bytes
#[pyclass]
pub struct AsyncBytesIterator {
    inner: Arc<TokioMutex<Option<reqwest::Response>>>,
    closed: Arc<TokioMutex<bool>>,
    chunk_size: usize,
}

#[pymethods]
impl AsyncBytesIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let closed = self.closed.clone();
        let chunk_size = self.chunk_size;

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            if let Some(ref mut response) = *guard {
                // Use chunk() to get the next chunk from the response body
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        // Return the chunk, potentially limiting to chunk_size
                        let data = if chunk.len() > chunk_size {
                            chunk[..chunk_size].to_vec()
                        } else {
                            chunk.to_vec()
                        };
                        Ok(Some(data))
                    }
                    Ok(None) => {
                        // End of stream
                        *closed.lock().await = true;
                        Ok(None)
                    }
                    Err(e) => Err(Error::request(e.to_string()).into()),
                }
            } else {
                Ok(None)
            }
        })
    }
}

/// Async iterator for streaming text
#[pyclass]
pub struct AsyncTextIterator {
    inner: Arc<TokioMutex<Option<reqwest::Response>>>,
    closed: Arc<TokioMutex<bool>>,
    chunk_size: usize,
    #[allow(dead_code)]
    encoding: String,
}

#[pymethods]
impl AsyncTextIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let closed = self.closed.clone();
        let chunk_size = self.chunk_size;

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = inner.lock().await;
            if let Some(ref mut response) = *guard {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let data = if chunk.len() > chunk_size {
                            &chunk[..chunk_size]
                        } else {
                            &chunk[..]
                        };
                        let text = String::from_utf8_lossy(data).to_string();
                        Ok(Some(text))
                    }
                    Ok(None) => {
                        *closed.lock().await = true;
                        Ok(None)
                    }
                    Err(e) => Err(Error::request(e.to_string()).into()),
                }
            } else {
                Ok(None)
            }
        })
    }
}

/// Async iterator for streaming lines
#[pyclass]
pub struct AsyncLinesIterator {
    inner: Arc<TokioMutex<Option<reqwest::Response>>>,
    closed: Arc<TokioMutex<bool>>,
    buffer: Arc<TokioMutex<String>>,
    #[allow(dead_code)]
    encoding: String,
}

#[pymethods]
impl AsyncLinesIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let closed = self.closed.clone();
        let buffer = self.buffer.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            // First check if we have a complete line in the buffer
            {
                let mut buf = buffer.lock().await;
                if let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].to_string();
                    *buf = buf[pos + 1..].to_string();
                    return Ok(Some(line));
                }
            }

            // Read more data
            let mut guard = inner.lock().await;
            if let Some(ref mut response) = *guard {
                loop {
                    match response.chunk().await {
                        Ok(Some(chunk)) => {
                            let text = String::from_utf8_lossy(&chunk);
                            let mut buf = buffer.lock().await;
                            buf.push_str(&text);

                            // Check for complete line
                            if let Some(pos) = buf.find('\n') {
                                let line = buf[..pos].to_string();
                                *buf = buf[pos + 1..].to_string();
                                return Ok(Some(line));
                            }
                        }
                        Ok(None) => {
                            // EOF - return remaining buffer if any
                            *closed.lock().await = true;
                            let mut buf = buffer.lock().await;
                            if !buf.is_empty() {
                                let line = std::mem::take(&mut *buf);
                                return Ok(Some(line));
                            }
                            return Ok(None);
                        }
                        Err(e) => return Err(Error::request(e.to_string()).into()),
                    }
                }
            } else {
                Ok(None)
            }
        })
    }
}
