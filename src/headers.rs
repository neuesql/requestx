//! HTTP Headers implementation

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyString, PyTuple};
use std::collections::HashMap;

/// Extract string from either str or bytes, returning (string, encoding)
fn extract_string_or_bytes(obj: &Bound<'_, PyAny>) -> PyResult<(String, String)> {
    // Check for None first
    if obj.is_none() {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            format!("Header value must be str or bytes, not {}", obj.get_type())
        ));
    }
    // Try string first
    if let Ok(s) = obj.downcast::<PyString>() {
        return Ok((s.to_string(), "ascii".to_string()));
    }
    // Try bytes
    if let Ok(b) = obj.downcast::<PyBytes>() {
        let bytes = b.as_bytes();
        // Try to detect encoding
        // First try ASCII (all bytes < 128)
        if bytes.iter().all(|&byte| byte < 128) {
            return Ok((String::from_utf8_lossy(bytes).to_string(), "ascii".to_string()));
        }
        // Try UTF-8
        if let Ok(s) = std::str::from_utf8(bytes) {
            return Ok((s.to_string(), "utf-8".to_string()));
        }
        // Fall back to ISO-8859-1 (Latin-1) - direct byte to char mapping
        let s: String = bytes.iter().map(|&b| b as char).collect();
        return Ok((s, "iso-8859-1".to_string()));
    }
    // Try extracting as string - if this fails, give a better error
    obj.extract::<String>().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err(
            format!("Header value must be str or bytes, not {}", obj.get_type())
        )
    }).map(|s| (s, "ascii".to_string()))
}

/// Extract key (lowercased) from either str or bytes, returning (string, encoding)
fn extract_key_or_bytes(obj: &Bound<'_, PyAny>) -> PyResult<(String, String)> {
    let (s, enc) = extract_string_or_bytes(obj)?;
    Ok((s.to_lowercase(), enc))
}

/// HTTP Headers with case-insensitive keys
#[pyclass(name = "Headers")]
#[derive(Clone, Debug, Default)]
pub struct Headers {
    /// Store headers as list of (name, value) tuples to preserve order and duplicates
    inner: Vec<(String, String)>,
    /// Whether headers were created from a dict (affects repr format)
    from_dict: bool,
    /// Encoding used to decode bytes (ascii, utf-8, iso-8859-1)
    encoding: String,
}

impl Headers {
    pub fn new() -> Self {
        Self { inner: Vec::new(), from_dict: false, encoding: "ascii".to_string() }
    }

    pub fn from_vec(headers: Vec<(String, String)>) -> Self {
        Self { inner: headers, from_dict: false, encoding: "ascii".to_string() }
    }

    pub fn get_all(&self, key: &str) -> Vec<&str> {
        let key_lower = key.to_lowercase();
        self.inner
            .iter()
            .filter(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    pub fn to_reqwest(&self) -> reqwest::header::HeaderMap {
        let mut map = reqwest::header::HeaderMap::new();
        for (key, value) in &self.inner {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                map.append(name, val);
            }
        }
        map
    }

    pub fn from_reqwest(headers: &reqwest::header::HeaderMap) -> Self {
        let inner: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or("").to_string(),
                )
            })
            .collect();
        Self { inner, from_dict: false, encoding: "ascii".to_string() }
    }

    pub fn inner(&self) -> &Vec<(String, String)> {
        &self.inner
    }

    /// Iterate over header (key, value) pairs
    pub fn iter_pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Set a header value (removes existing headers with same key)
    /// Keys are normalized to lowercase to match httpx behavior
    pub fn set(&mut self, key: String, value: String) {
        let key_lower = key.to_lowercase();
        self.inner.retain(|(k, _)| k.to_lowercase() != key_lower);
        self.inner.push((key_lower, value));
    }

    /// Check if a header exists
    pub fn contains(&self, key: &str) -> bool {
        let key_lower = key.to_lowercase();
        self.inner.iter().any(|(k, _)| k.to_lowercase() == key_lower)
    }

    /// Get a header value (returns comma-separated if multiple values exist)
    pub fn get(&self, key: &str, default: Option<&str>) -> Option<String> {
        let key_lower = key.to_lowercase();
        let values: Vec<&str> = self.inner
            .iter()
            .filter(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.as_str())
            .collect();

        if values.is_empty() {
            default.map(|s| s.to_string())
        } else {
            Some(values.join(", "))
        }
    }

    /// Remove a header by key (case-insensitive)
    pub fn remove(&mut self, key: &str) {
        let key_lower = key.to_lowercase();
        self.inner.retain(|(k, _)| k.to_lowercase() != key_lower);
    }

    /// Append a header value (allows duplicate keys)
    pub fn append(&mut self, key: String, value: String) {
        let key_lower = key.to_lowercase();
        self.inner.push((key_lower, value));
    }
}

#[pymethods]
impl Headers {
    #[new]
    #[pyo3(signature = (headers=None))]
    fn py_new(headers: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        use pyo3::types::PyBytes;

        let mut h = Self::new();

        if let Some(obj) = headers {
            if let Ok(dict) = obj.downcast::<PyDict>() {
                h.from_dict = true;
                for (key, value) in dict.iter() {
                    // Handle both string and bytes keys/values (keys are lowercased)
                    let (k, k_encoding) = extract_key_or_bytes(&key)?;
                    let (v, v_encoding) = extract_string_or_bytes(&value)?;
                    h.inner.push((k, v));
                    // Update encoding if non-ascii detected
                    if k_encoding != "ascii" || v_encoding != "ascii" {
                        if k_encoding == "utf-8" || v_encoding == "utf-8" {
                            h.encoding = "utf-8".to_string();
                        } else if k_encoding == "iso-8859-1" || v_encoding == "iso-8859-1" {
                            h.encoding = "iso-8859-1".to_string();
                        }
                    }
                }
            } else if let Ok(list) = obj.downcast::<PyList>() {
                for item in list.iter() {
                    let tuple = item.downcast::<PyTuple>()?;
                    let (k, k_encoding) = extract_key_or_bytes(&tuple.get_item(0)?)?;
                    let (v, v_encoding) = extract_string_or_bytes(&tuple.get_item(1)?)?;
                    h.inner.push((k, v));
                    // Update encoding if non-ascii detected
                    if k_encoding != "ascii" || v_encoding != "ascii" {
                        if k_encoding == "utf-8" || v_encoding == "utf-8" {
                            h.encoding = "utf-8".to_string();
                        } else if k_encoding == "iso-8859-1" || v_encoding == "iso-8859-1" {
                            h.encoding = "iso-8859-1".to_string();
                        }
                    }
                }
            } else if let Ok(other_headers) = obj.extract::<Headers>() {
                h.inner = other_headers.inner;
                h.from_dict = other_headers.from_dict;
                h.encoding = other_headers.encoding;
            }
        }

        Ok(h)
    }

    #[pyo3(name = "get", signature = (key, default=None))]
    fn py_get(&self, key: &str, default: Option<&str>) -> Option<String> {
        self.get(key, default)
    }

    #[pyo3(signature = (key, split_commas=false))]
    fn get_list(&self, key: &str, split_commas: bool) -> Vec<String> {
        let key_lower = key.to_lowercase();
        let values: Vec<String> = self.inner
            .iter()
            .filter(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.clone())
            .collect();

        if split_commas {
            values
                .iter()
                .flat_map(|v| v.split(',').map(|s| s.trim().to_string()))
                .collect()
        } else {
            values
        }
    }

    fn keys(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.inner
            .iter()
            .filter_map(|(k, _)| {
                let lower = k.to_lowercase();
                if seen.insert(lower.clone()) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn values(&self) -> Vec<String> {
        // Return merged values for duplicate keys, maintaining key order
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for key in self.keys() {
            let key_lower = key.to_lowercase();
            if seen.insert(key_lower.clone()) {
                let values: Vec<&str> = self.inner
                    .iter()
                    .filter(|(k, _)| k.to_lowercase() == key_lower)
                    .map(|(_, v)| v.as_str())
                    .collect();
                result.push(values.join(", "));
            }
        }
        result
    }

    fn setdefault(&mut self, key: String, default: Option<String>) -> String {
        let key_lower = key.to_lowercase();
        if let Some(existing) = self.inner
            .iter()
            .find(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.clone())
        {
            existing
        } else {
            let value = default.unwrap_or_default();
            self.inner.push((key_lower, value.clone()));
            value
        }
    }

    fn items(&self) -> Vec<(String, String)> {
        // Return merged values for duplicate keys, maintaining key order
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for (key, _) in &self.inner {
            let key_lower = key.to_lowercase();
            if seen.insert(key_lower.clone()) {
                let values: Vec<&str> = self.inner
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
        self.inner.clone()
    }

    #[getter]
    fn raw(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.inner
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect()
    }

    fn __getitem__(&self, key: &str) -> PyResult<String> {
        let key_lower = key.to_lowercase();
        let values: Vec<&str> = self.inner
            .iter()
            .filter(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.as_str())
            .collect();

        if values.is_empty() {
            Err(PyKeyError::new_err(key.to_string()))
        } else {
            Ok(values.join(", "))
        }
    }

    fn __setitem__(&mut self, key: String, value: String) {
        let key_lower = key.to_lowercase();
        // Find first occurrence of this key to preserve ordering
        let mut first_found = false;
        let mut insert_pos = None;
        let mut new_inner = Vec::with_capacity(self.inner.len());

        for (i, (k, v)) in self.inner.iter().enumerate() {
            if k.to_lowercase() == key_lower {
                if !first_found {
                    // Replace at first occurrence
                    insert_pos = Some(new_inner.len());
                    first_found = true;
                }
                // Skip all occurrences of this key
            } else {
                new_inner.push((k.clone(), v.clone()));
            }
        }

        if let Some(pos) = insert_pos {
            new_inner.insert(pos, (key_lower.clone(), value));
        } else {
            new_inner.push((key_lower, value));
        }

        self.inner = new_inner;
    }

    fn __delitem__(&mut self, key: &str) -> PyResult<()> {
        let key_lower = key.to_lowercase();
        let orig_len = self.inner.len();
        self.inner.retain(|(k, _)| k.to_lowercase() != key_lower);
        if self.inner.len() == orig_len {
            Err(PyKeyError::new_err(key.to_string()))
        } else {
            Ok(())
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        let key_lower = key.to_lowercase();
        self.inner.iter().any(|(k, _)| k.to_lowercase() == key_lower)
    }

    fn __iter__(&self) -> HeadersIterator {
        HeadersIterator {
            keys: self.keys(),
            index: 0,
        }
    }

    fn __len__(&self) -> usize {
        self.keys().len()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_headers) = other.extract::<Headers>() {
            // Compare multi_items as sets (order independent, case-insensitive keys)
            let mut self_items: Vec<(String, String)> = self
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            let mut other_items: Vec<(String, String)> = other_headers
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            self_items.sort();
            other_items.sort();
            Ok(self_items == other_items)
        } else if let Ok(dict) = other.downcast::<PyDict>() {
            let self_map: HashMap<String, String> = self
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            let mut other_map = HashMap::new();
            for (k, v) in dict.iter() {
                let key: String = k.extract()?;
                let value: String = v.extract()?;
                other_map.insert(key.to_lowercase(), value);
            }
            Ok(self_map == other_map)
        } else if let Ok(list) = other.downcast::<PyList>() {
            // Compare with list of tuples
            let mut self_items: Vec<(String, String)> = self
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            let mut other_items: Vec<(String, String)> = Vec::new();
            for item in list.iter() {
                let tuple = item.downcast::<PyTuple>()?;
                let k: String = tuple.get_item(0)?.extract()?;
                let v: String = tuple.get_item(1)?.extract()?;
                other_items.push((k.to_lowercase(), v));
            }
            self_items.sort();
            other_items.sort();
            Ok(self_items == other_items)
        } else {
            Ok(false)
        }
    }

    fn __repr__(&self) -> String {
        // Sensitive headers that should be masked
        let sensitive_headers = ["authorization", "proxy-authorization"];

        let mask_value = |k: &str, v: &str| -> String {
            if sensitive_headers.contains(&k.to_lowercase().as_str()) {
                "[secure]".to_string()
            } else {
                v.to_string()
            }
        };

        if self.from_dict {
            let items: Vec<String> = self
                .inner
                .iter()
                .map(|(k, v)| format!("'{}': '{}'", k, mask_value(k, v)))
                .collect();
            format!("Headers({{{}}})", items.join(", "))
        } else {
            // Check if we have duplicate keys - if so, use list format
            let mut seen = std::collections::HashSet::new();
            let has_duplicates = self.inner.iter().any(|(k, _)| !seen.insert(k.to_lowercase()));

            if has_duplicates {
                let items: Vec<String> = self
                    .inner
                    .iter()
                    .map(|(k, v)| format!("('{}', '{}')", k, mask_value(k, v)))
                    .collect();
                format!("Headers([{}])", items.join(", "))
            } else {
                // Single values per key - use dict format
                let items: Vec<String> = self
                    .inner
                    .iter()
                    .map(|(k, v)| format!("'{}': '{}'", k, mask_value(k, v)))
                    .collect();
                format!("Headers({{{}}})", items.join(", "))
            }
        }
    }

    #[getter]
    fn encoding(&self) -> &str {
        &self.encoding
    }

    #[setter]
    fn set_encoding(&mut self, encoding: &str) {
        self.encoding = encoding.to_string();
    }

    fn copy(&self) -> Self {
        self.clone()
    }

    fn update(&mut self, other: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(dict) = other.downcast::<PyDict>() {
            for (key, value) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                self.__setitem__(k, v);
            }
        } else if let Ok(headers) = other.extract::<Headers>() {
            for (k, v) in headers.inner {
                self.__setitem__(k, v);
            }
        }
        Ok(())
    }
}

#[pyclass]
pub struct HeadersIterator {
    keys: Vec<String>,
    index: usize,
}

#[pymethods]
impl HeadersIterator {
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
