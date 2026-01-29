//! HTTP Response implementation

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use crate::cookies::Cookies;
use crate::headers::Headers;
use crate::request::Request;
use crate::url::URL;

/// HTTP Response object
#[pyclass(name = "Response")]
#[derive(Clone)]
pub struct Response {
    status_code: u16,
    headers: Headers,
    content: Vec<u8>,
    url: Option<URL>,
    request: Option<Request>,
    http_version: String,
    history: Vec<Response>,
    is_closed: bool,
    is_stream_consumed: bool,
    default_encoding: String,
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
            history: Vec::new(),
            is_closed: false,
            is_stream_consumed: false,
            default_encoding: "utf-8".to_string(),
        }
    }

    /// Set the request that generated this response (public Rust API)
    pub fn set_request_attr(&mut self, request: Option<Request>) {
        self.request = request;
    }

    pub fn from_reqwest(
        response: reqwest::blocking::Response,
        request: Option<Request>,
    ) -> PyResult<Self> {
        let status_code = response.status().as_u16();
        let headers = Headers::from_reqwest(response.headers());
        let url = URL::parse(response.url().as_str()).ok();
        let http_version = format!("{:?}", response.version());

        let content = response.bytes().map_err(|e| {
            crate::exceptions::ReadError::new_err(format!("Failed to read response: {}", e))
        })?;

        Ok(Self {
            status_code,
            headers,
            content: content.to_vec(),
            url,
            request,
            http_version,
            history: Vec::new(),
            is_closed: true,
            is_stream_consumed: true,
            default_encoding: "utf-8".to_string(),
        })
    }

    pub async fn from_reqwest_async(
        response: reqwest::Response,
        request: Option<Request>,
    ) -> PyResult<Self> {
        let status_code = response.status().as_u16();
        let headers = Headers::from_reqwest(response.headers());
        let url = URL::parse(response.url().as_str()).ok();
        let http_version = format!("{:?}", response.version());

        let content = response.bytes().await.map_err(|e| {
            crate::exceptions::ReadError::new_err(format!("Failed to read response: {}", e))
        })?;

        Ok(Self {
            status_code,
            headers,
            content: content.to_vec(),
            url,
            request,
            http_version,
            history: Vec::new(),
            is_closed: true,
            is_stream_consumed: true,
            default_encoding: "utf-8".to_string(),
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
            } else if let Ok(dict) = h.downcast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    response.headers.set(k, v);
                }
            }
        }

        // Handle content
        if let Some(c) = content {
            if let Ok(bytes) = c.extract::<Vec<u8>>() {
                response.content = bytes;
            } else if let Ok(s) = c.extract::<String>() {
                response.content = s.into_bytes();
            } else if let Ok(list) = c.downcast::<pyo3::types::PyList>() {
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
            } else if let Ok(tuple) = c.downcast::<pyo3::types::PyTuple>() {
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
            }
            if !response.headers.contains("content-length") {
                response.headers.set(
                    "Content-Length".to_string(),
                    response.content.len().to_string(),
                );
            }
        }

        // Handle text
        if let Some(t) = text {
            response.content = t.as_bytes().to_vec();
            response.headers.set(
                "Content-Length".to_string(),
                response.content.len().to_string(),
            );
            response.headers.set(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            );
        }

        // Handle HTML
        if let Some(h) = html {
            response.content = h.as_bytes().to_vec();
            response.headers.set(
                "Content-Length".to_string(),
                response.content.len().to_string(),
            );
            response.headers.set(
                "Content-Type".to_string(),
                "text/html; charset=utf-8".to_string(),
            );
        }

        // Handle JSON
        if let Some(j) = json {
            let json_str = py_to_json_string(j)?;
            response.content = json_str.into_bytes();
            response.headers.set(
                "Content-Length".to_string(),
                response.content.len().to_string(),
            );
            response.headers.set(
                "Content-Type".to_string(),
                "application/json".to_string(),
            );
        }

        response.is_stream_consumed = true;
        response.is_closed = true;

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
    fn content<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.content)
    }

    #[getter]
    fn text(&self) -> PyResult<String> {
        // Try to get encoding from content-type header
        let encoding = self.get_encoding();

        // For now, just use UTF-8 (proper encoding detection would need more work)
        String::from_utf8(self.content.clone()).map_err(|e| {
            crate::exceptions::DecodingError::new_err(format!("Failed to decode response: {}", e))
        })
    }

    fn json(&self, py: Python<'_>) -> PyResult<PyObject> {
        let text = self.text()?;
        json_to_py(py, &text)
    }

    #[getter]
    fn url(&self) -> Option<URL> {
        self.url.clone()
    }

    #[getter]
    fn request(&self) -> Option<Request> {
        self.request.clone()
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
    fn extensions(&self) -> std::collections::HashMap<String, PyObject> {
        std::collections::HashMap::new()
    }

    fn raise_for_status(&self) -> PyResult<()> {
        if self.is_error() {
            let message = format!(
                "{} {} for url {}",
                self.status_code,
                self.reason_phrase(),
                self.url.as_ref().map(|u| u.to_string()).unwrap_or_default()
            );
            Err(crate::exceptions::HTTPStatusError::new_err(message))
        } else {
            Ok(())
        }
    }

    fn read(&mut self) -> Vec<u8> {
        self.is_stream_consumed = true;
        self.content.clone()
    }

    fn close(&mut self) {
        self.is_closed = true;
    }

    fn iter_bytes(&self) -> BytesIterator {
        BytesIterator {
            content: self.content.clone(),
            position: 0,
            chunk_size: 4096,
        }
    }

    fn iter_text(&self) -> PyResult<TextIterator> {
        let text = self.text()?;
        Ok(TextIterator {
            text,
            position: 0,
            chunk_size: 4096,
        })
    }

    fn iter_lines(&self) -> PyResult<LinesIterator> {
        let text = self.text()?;
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

        Ok(LinesIterator {
            lines,
            position: 0,
        })
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

    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        self.close();
        false
    }
}

impl Response {
    fn get_encoding(&self) -> String {
        if let Some(content_type) = self.headers.get("content-type", None) {
            // Look for charset in content-type
            for part in content_type.split(';') {
                let part = part.trim();
                if part.to_lowercase().starts_with("charset=") {
                    return part[8..].trim_matches('"').to_string();
                }
            }
        }
        self.default_encoding.clone()
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
        _ => "Unknown",
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

/// Parse JSON string to Python object
fn json_to_py(py: Python<'_>, json_str: &str) -> PyResult<PyObject> {
    let value: sonic_rs::Value = sonic_rs::from_str(json_str).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("JSON parse error: {}", e))
    })?;
    json_value_to_py(py, &value)
}

/// Convert sonic_rs::Value to Python object
fn json_value_to_py(py: Python<'_>, value: &sonic_rs::Value) -> PyResult<PyObject> {
    use pyo3::types::{PyDict, PyList};
    use sonic_rs::{JsonValueTrait, JsonContainerTrait};

    if value.is_null() {
        return Ok(py.None());
    }

    if let Some(b) = value.as_bool() {
        return Ok(pyo3::types::PyBool::new(py, b).to_owned().into_any().unbind());
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
