use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use serde_json::Value;
use std::collections::HashMap;

use crate::error::RequestxError;

/// Response object compatible with requests.Response
#[pyclass]
pub struct Response {
    #[pyo3(get)]
    status_code: u16,

    #[pyo3(get)]
    url: String,

    headers: HashMap<String, String>,
    text_content: Option<String>,
    binary_content: Option<Vec<u8>>,
    encoding: Option<String>,

    // Additional fields for requests compatibility
    #[pyo3(get)]
    ok: bool,

    #[pyo3(get)]
    reason: String,
}

#[pymethods]
impl Response {
    #[new]
    pub fn new(
        status_code: u16,
        url: String,
        headers: HashMap<String, String>,
        content: Vec<u8>,
    ) -> Self {
        let ok = status_code < 400;
        let reason = Self::status_code_to_reason(status_code);

        Response {
            status_code,
            url,
            headers,
            text_content: None,
            binary_content: Some(content),
            encoding: None,
            ok,
            reason,
        }
    }

    /// Get response headers as a dictionary
    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (key, value) in &self.headers {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
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

    /// Parse response as JSON
    fn json(&mut self, py: Python) -> PyResult<PyObject> {
        let text = self.text()?;
        let value: Value =
            serde_json::from_str(&text).map_err(|e| RequestxError::JsonDecodeError(e))?;

        pythonize::pythonize(py, &value)
            .map_err(|e| RequestxError::PythonError(e.to_string()).into())
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

    /// Get response history (placeholder - returns empty list for now)
    #[getter]
    fn history(&self, py: Python) -> PyResult<PyObject> {
        // For now, return an empty list
        // TODO: Implement redirect history tracking
        Ok(py.eval("[]", None, None)?.into())
    }

    /// Get response links (placeholder - returns empty dict for now)
    #[getter]
    fn links(&self, py: Python) -> PyResult<PyObject> {
        // For now, return an empty dict
        // TODO: Parse Link headers
        let dict = PyDict::new(py);
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
        if let Some(content_type) = self
            .headers
            .get("content-type")
            .or_else(|| self.headers.get("Content-Type"))
        {
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
