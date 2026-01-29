//! HTTP Request implementation

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use crate::cookies::Cookies;
use crate::headers::Headers;
use crate::multipart::{build_multipart_body, build_multipart_body_with_boundary, extract_boundary_from_content_type};
use crate::types::SyncByteStream;
use crate::url::URL;

/// HTTP Request object
#[pyclass(name = "Request")]
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

    pub fn headers_mut(&mut self) -> &mut Headers {
        &mut self.headers
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
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "Content must be bytes or str",
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

        // Handle multipart (files provided and non-empty)
        let files_is_empty = files.as_ref().map_or(true, |f| {
            if let Ok(dict) = f.downcast::<PyDict>() {
                dict.is_empty()
            } else if let Ok(list) = f.downcast::<pyo3::types::PyList>() {
                list.is_empty()
            } else {
                false
            }
        });
        let data_is_empty = data.as_ref().map_or(true, |d| {
            if let Ok(dict) = d.downcast::<PyDict>() {
                dict.is_empty()
            } else {
                false
            }
        });

        if let Some(f) = files {
            // Skip multipart if both files and data are empty, but set Content-Length: 0
            if files_is_empty && data_is_empty {
                request.content = Some(Vec::new());
            } else {
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
            }
        } else if let Some(d) = data {
            // Handle form data (no files) or bytes with deprecation warning
            if let Ok(bytes) = d.extract::<Vec<u8>>() {
                // Emit deprecation warning for using data= with bytes
                let warnings = _py.import("warnings")?;
                warnings.call_method1(
                    "warn",
                    (
                        "Use 'content=<...>' to upload raw bytes/text content.",
                        _py.get_type::<pyo3::exceptions::PyDeprecationWarning>(),
                    ),
                )?;
                request.content = Some(bytes);
            } else if let Ok(dict) = d.downcast::<PyDict>() {
                let mut form_data = Vec::new();
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    encode_form_value(&mut form_data, &k, &value)?;
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

        // Set Content-Length header only when content is provided
        if let Some(ref content) = request.content {
            request.headers.set("Content-Length".to_string(), content.len().to_string());
        }

        // Set Host header if not already provided
        if !request.headers.contains("host") {
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

    #[getter]
    fn headers(&self) -> Headers {
        self.headers.clone()
    }

    #[getter]
    fn content<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        match &self.content {
            Some(c) => PyBytes::new(py, c),
            None => PyBytes::new(py, b""),
        }
    }

    #[getter]
    fn stream(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = self.content.clone().unwrap_or_default();
        let (async_stream, sync_stream) = crate::types::AsyncByteStream::from_data(data);
        let obj = Py::new(py, (async_stream, sync_stream))?;
        Ok(obj.into_any())
    }

    #[getter]
    fn extensions(&self) -> std::collections::HashMap<String, PyObject> {
        std::collections::HashMap::new()
    }

    fn read(&mut self) -> Vec<u8> {
        self.content.clone().unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!("<Request('{}', '{}')>", self.method, self.url.to_string())
    }

    fn __eq__(&self, other: &Request) -> bool {
        self.method == other.method && self.url.to_string() == other.url.to_string()
    }
}

/// Convert Python object to JSON string using sonic-rs
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

/// Encode a form value (handles bool, None, list, string, int)
fn encode_form_value(form_data: &mut Vec<String>, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
    use pyo3::types::{PyBool, PyFloat, PyInt, PyList, PyString, PyTuple};

    // Handle None
    if value.is_none() {
        form_data.push(format!("{}=", urlencoding::encode(key)));
        return Ok(());
    }

    // Handle bool (must check before int since bool is a subclass of int)
    if let Ok(b) = value.downcast::<PyBool>() {
        let val_str = if b.is_true() { "true" } else { "false" };
        form_data.push(format!("{}={}", urlencoding::encode(key), val_str));
        return Ok(());
    }

    // Handle int
    if let Ok(i) = value.downcast::<PyInt>() {
        let val: i64 = i.extract()?;
        form_data.push(format!("{}={}", urlencoding::encode(key), val));
        return Ok(());
    }

    // Handle float
    if let Ok(f) = value.downcast::<PyFloat>() {
        let val: f64 = f.extract()?;
        form_data.push(format!("{}={}", urlencoding::encode(key), val));
        return Ok(());
    }

    // Handle string
    if let Ok(s) = value.downcast::<PyString>() {
        let val: String = s.extract()?;
        form_data.push(format!("{}={}", urlencoding::encode(key), urlencoding::encode(&val)));
        return Ok(());
    }

    // Handle list (each item becomes a separate key=value pair)
    if let Ok(list) = value.downcast::<PyList>() {
        for item in list.iter() {
            encode_form_value(form_data, key, &item)?;
        }
        return Ok(());
    }

    // Handle tuple
    if let Ok(tuple) = value.downcast::<PyTuple>() {
        for item in tuple.iter() {
            encode_form_value(form_data, key, &item)?;
        }
        return Ok(());
    }

    // Fallback: try to convert to string
    let s = value.str()?.to_string();
    form_data.push(format!("{}={}", urlencoding::encode(key), urlencoding::encode(&s)));
    Ok(())
}
