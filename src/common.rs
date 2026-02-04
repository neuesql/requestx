//! Shared utility functions used across multiple modules.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::headers::Headers;
use crate::url::URL;

/// Convert Python object to JSON string.
/// Uses Python's json module for serialization to preserve dict insertion order
/// and match httpx's default behavior (ensure_ascii=False, allow_nan=False, compact).
pub(crate) fn py_to_json_string(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let py = obj.py();
    let json_mod = py.import("json")?;

    // Use httpx's default JSON settings:
    // - ensure_ascii=False (allows non-ASCII characters)
    // - allow_nan=False (raises ValueError for NaN/Inf)
    // - separators=(',', ':') (compact representation)
    let kwargs = PyDict::new(py);
    kwargs.set_item("ensure_ascii", false)?;
    kwargs.set_item("allow_nan", false)?;
    let separators = pyo3::types::PyTuple::new(py, [",", ":"])?;
    kwargs.set_item("separators", separators)?;

    let result = json_mod.call_method("dumps", (obj,), Some(&kwargs))?;
    result.extract::<String>()
}

/// Convert Python object to sonic_rs::Value.
pub(crate) fn py_to_json_value(obj: &Bound<'_, PyAny>) -> PyResult<sonic_rs::Value> {
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
        // Check for NaN and Inf - not allowed by default in JSON
        if val.is_nan() || val.is_infinite() {
            return Err(pyo3::exceptions::PyValueError::new_err("Out of range float values are not JSON compliant"));
        }
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

    Err(pyo3::exceptions::PyTypeError::new_err("Unsupported type for JSON serialization"))
}

/// Build the Host header value from a URL.
/// Only includes port if it's non-default for the scheme.
pub(crate) fn get_host_header(url: &URL) -> String {
    let host = url.get_host_str();
    let port = url.get_port();
    let scheme = url.get_scheme();

    let default_port = match scheme.as_str() {
        "http" => 80,
        "https" => 443,
        _ => 0,
    };

    if let Some(p) = port {
        if p != default_port {
            return format!("{}:{}", host, p);
        }
    }
    host
}

/// Check if a URL matches a mount pattern.
///
/// Mount patterns can be:
/// - "all://" - matches all URLs
/// - "http://" - matches all HTTP URLs
/// - "https://" - matches all HTTPS URLs
/// - "http://example.com" - matches specific domain (any port)
/// - "http://example.com:8080" - matches specific domain and port
/// - "http://*.example.com" - matches subdomains only (not example.com itself)
/// - "http://*example.com" - matches domain suffix (example.com and www.example.com)
/// - "http://*" - matches any domain with http scheme
/// - "all://example.com" - matches domain on any scheme
pub(crate) fn url_matches_pattern(url: &str, pattern: &str) -> bool {
    if pattern == "all://" {
        return true;
    }

    // Parse the URL scheme
    let url_scheme = url.split("://").next().unwrap_or("");
    let pattern_scheme = pattern.split("://").next().unwrap_or("");

    // Check scheme match (unless pattern scheme is "all")
    if pattern_scheme != "all" && pattern_scheme != url_scheme {
        return false;
    }

    // Get the URL host (with port)
    let url_host = if let Some(rest) = url.strip_prefix(&format!("{}://", url_scheme)) {
        rest.split('/').next().unwrap_or("")
    } else {
        ""
    };

    // Get the pattern host (with port if specified)
    let pattern_host = if let Some(rest) = pattern.strip_prefix(&format!("{}://", pattern_scheme)) {
        rest.split('/').next().unwrap_or("")
    } else {
        ""
    };

    // If pattern is just scheme://, match all hosts
    if pattern_host.is_empty() {
        return true;
    }

    // Handle "*" pattern - matches any host
    if pattern_host == "*" {
        return true;
    }

    // Split into host and port
    let url_host_no_port = url_host.split(':').next().unwrap_or(url_host);
    let url_port = url_host.split(':').nth(1);
    let pattern_host_no_port = pattern_host.split(':').next().unwrap_or(pattern_host);
    let pattern_port = pattern_host.split(':').nth(1);

    // Handle "*.example.com" pattern - matches subdomains ONLY (NOT example.com itself)
    if pattern_host_no_port.starts_with("*.") {
        let suffix = &pattern_host_no_port[2..]; // Remove "*."
        if url_host_no_port.ends_with(&format!(".{}", suffix)) {
            return port_matches(url_port, pattern_port);
        }
        return false;
    }

    // Handle "*example.com" pattern (no dot) - matches suffix
    if pattern_host_no_port.starts_with('*') && !pattern_host_no_port.starts_with("*.") {
        let suffix = &pattern_host_no_port[1..]; // Remove "*"
        if url_host_no_port == suffix {
            return port_matches(url_port, pattern_port);
        }
        if url_host_no_port.ends_with(&format!(".{}", suffix)) {
            return port_matches(url_port, pattern_port);
        }
        return false;
    }

    // Exact host match
    if url_host_no_port != pattern_host_no_port {
        return false;
    }

    // If pattern has a port, URL must have matching port
    // If pattern has no port, any port matches
    port_matches(url_port, pattern_port)
}

/// Check if URL port matches pattern port.
fn port_matches(url_port: Option<&str>, pattern_port: Option<&str>) -> bool {
    match pattern_port {
        None => true,                     // Pattern has no port requirement
        Some(pp) => url_port == Some(pp), // Port must match exactly
    }
}

/// Generate a PyO3 iterator class with `__iter__` and `__next__`.
///
/// Usage: `impl_py_iterator!(StructName, ItemType, field_name, "PythonClassName");`
macro_rules! impl_py_iterator {
    ($name:ident, $item_type:ty, $field:ident, $pyname:literal) => {
        #[pyo3::pyclass(name = $pyname)]
        pub struct $name {
            pub $field: Vec<$item_type>,
            index: usize,
        }

        #[pyo3::pymethods]
        impl $name {
            fn __iter__(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyRef<'_, Self> {
                slf
            }

            fn __next__(&mut self) -> Option<$item_type> {
                if self.index < self.$field.len() {
                    let item = self.$field[self.index].clone();
                    self.index += 1;
                    Some(item)
                } else {
                    None
                }
            }
        }

        impl $name {
            pub fn new($field: Vec<$item_type>) -> Self {
                Self { $field, index: 0 }
            }
        }
    };
}
pub(crate) use impl_py_iterator;

/// Generate a PyO3 dual-mode byte stream class (supports both sync and async iteration).
///
/// Usage: `impl_byte_stream!(StructName, "PythonClassName");`
macro_rules! impl_byte_stream {
    ($name:ident, $pyname:literal) => {
        #[pyo3::pyclass(name = $pyname, subclass)]
        #[derive(Clone, Debug, Default)]
        pub struct $name {
            data: Vec<u8>,
            sync_consumed: bool,
            async_consumed: bool,
        }

        #[pyo3::pymethods]
        impl $name {
            #[new]
            fn new() -> Self {
                Self {
                    data: Vec::new(),
                    sync_consumed: false,
                    async_consumed: false,
                }
            }

            fn __iter__(mut slf: pyo3::PyRefMut<'_, Self>) -> pyo3::PyRefMut<'_, Self> {
                slf.sync_consumed = false;
                slf
            }

            fn __next__(&mut self) -> Option<Vec<u8>> {
                if self.sync_consumed || self.data.is_empty() {
                    None
                } else {
                    self.sync_consumed = true;
                    Some(self.data.clone())
                }
            }

            fn __aiter__(mut slf: pyo3::PyRefMut<'_, Self>) -> pyo3::PyRefMut<'_, Self> {
                slf.async_consumed = false;
                slf
            }

            fn __anext__<'py>(&mut self, py: pyo3::Python<'py>) -> pyo3::PyResult<Option<pyo3::Bound<'py, pyo3::types::PyBytes>>> {
                if self.async_consumed || self.data.is_empty() {
                    Ok(None)
                } else {
                    self.async_consumed = true;
                    Ok(Some(pyo3::types::PyBytes::new(py, &self.data)))
                }
            }

            fn read(&self) -> Vec<u8> {
                self.data.clone()
            }

            fn close(&mut self) {
                self.data.clear();
                self.sync_consumed = true;
                self.async_consumed = true;
            }

            fn aread<'py>(&self, py: pyo3::Python<'py>) -> pyo3::Bound<'py, pyo3::types::PyBytes> {
                pyo3::types::PyBytes::new(py, &self.data)
            }

            fn aclose(&mut self) {
                self.data.clear();
                self.sync_consumed = true;
                self.async_consumed = true;
            }

            fn __repr__(&self) -> String {
                format!("<{} [{} bytes]>", $pyname, self.data.len())
            }
        }

        impl $name {
            pub fn from_data(data: Vec<u8>) -> Self {
                Self {
                    data,
                    sync_consumed: false,
                    async_consumed: false,
                }
            }

            pub fn data(&self) -> &[u8] {
                &self.data
            }
        }
    };
}
pub(crate) use impl_byte_stream;

/// Create default headers, optionally merging user-provided headers on top.
pub(crate) fn make_default_headers(user_headers: Option<&Headers>) -> Headers {
    let version = env!("CARGO_PKG_VERSION");
    let mut headers = Headers::default();
    headers.set("Accept".to_string(), "*/*".to_string());
    headers.set("Accept-Encoding".to_string(), "gzip, deflate, br, zstd".to_string());
    headers.set("Connection".to_string(), "keep-alive".to_string());
    headers.set("User-Agent".to_string(), format!("python-httpx/{}", version));

    if let Some(user_headers) = user_headers {
        for (k, v) in user_headers.inner() {
            headers.set(k.clone(), v.clone());
        }
    }

    headers
}
