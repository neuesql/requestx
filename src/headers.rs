//! HTTP Headers implementation

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};
use std::collections::HashMap;

/// Extract a string from either a String or bytes Python object
fn extract_string_or_bytes(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = obj.extract::<String>() {
        Ok(s)
    } else if let Ok(bytes) = obj.downcast::<PyBytes>() {
        let bytes_slice = bytes.as_bytes();
        String::from_utf8(bytes_slice.to_vec())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid UTF-8: {}", e)))
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "Expected str or bytes",
        ))
    }
}

/// HTTP Headers with case-insensitive keys
#[pyclass(name = "Headers")]
#[derive(Clone, Debug, Default)]
pub struct Headers {
    /// Store headers as list of (name, value) tuples to preserve order and duplicates
    inner: Vec<(String, String)>,
}

impl Headers {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn from_vec(headers: Vec<(String, String)>) -> Self {
        Self { inner: headers }
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
        Self { inner }
    }

    pub fn inner(&self) -> &Vec<(String, String)> {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut Vec<(String, String)> {
        &mut self.inner
    }

    /// Iterate over header (key, value) pairs
    pub fn iter_pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Set a header value (removes existing headers with same key)
    pub fn set(&mut self, key: String, value: String) {
        let key_lower = key.to_lowercase();
        self.inner.retain(|(k, _)| k.to_lowercase() != key_lower);
        self.inner.push((key, value));
    }

    /// Check if a header exists
    pub fn contains(&self, key: &str) -> bool {
        let key_lower = key.to_lowercase();
        self.inner.iter().any(|(k, _)| k.to_lowercase() == key_lower)
    }

    /// Get a header value (concatenates multiple values with ", ")
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
}

#[pymethods]
impl Headers {
    #[new]
    #[pyo3(signature = (headers=None))]
    fn py_new(headers: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut h = Self::new();

        if let Some(obj) = headers {
            if let Ok(dict) = obj.downcast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let k = extract_string_or_bytes(&key)?;
                    let v = extract_string_or_bytes(&value)?;
                    h.inner.push((k, v));
                }
            } else if let Ok(list) = obj.downcast::<PyList>() {
                for item in list.iter() {
                    let tuple = item.downcast::<PyTuple>()?;
                    let k = extract_string_or_bytes(&tuple.get_item(0)?)?;
                    let v = extract_string_or_bytes(&tuple.get_item(1)?)?;
                    h.inner.push((k, v));
                }
            } else if let Ok(other_headers) = obj.extract::<Headers>() {
                h.inner = other_headers.inner;
            }
        }

        Ok(h)
    }

    #[pyo3(name = "get", signature = (key, default=None))]
    fn py_get(&self, key: &str, default: Option<&str>) -> Option<String> {
        self.get(key, default)
    }

    fn get_list(&self, key: &str) -> Vec<String> {
        let key_lower = key.to_lowercase();
        self.inner
            .iter()
            .filter(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.clone())
            .collect()
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
        // Return concatenated values for unique keys
        self.keys()
            .iter()
            .map(|k| self.get(k, None).unwrap_or_default())
            .collect()
    }

    fn items(&self) -> Vec<(String, String)> {
        // Return unique keys with concatenated values
        self.keys()
            .iter()
            .map(|k| (k.clone(), self.get(k, None).unwrap_or_default()))
            .collect()
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
        self.get(key, None)
            .ok_or_else(|| PyKeyError::new_err(key.to_string()))
    }

    fn __setitem__(&mut self, key: String, value: String) {
        let key_lower = key.to_lowercase();
        // Remove existing headers with same key
        self.inner.retain(|(k, _)| k.to_lowercase() != key_lower);
        self.inner.push((key, value));
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
            // Compare as case-insensitive multimap (sorted)
            let mut self_items: Vec<(String, String)> = self
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            self_items.sort();
            let mut other_items: Vec<(String, String)> = other_headers
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            other_items.sort();
            Ok(self_items == other_items)
        } else if let Ok(list) = other.downcast::<PyList>() {
            // Compare with list of tuples
            let mut self_items: Vec<(String, String)> = self
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            self_items.sort();
            let mut other_items: Vec<(String, String)> = Vec::new();
            for item in list.iter() {
                let tuple = item.downcast::<PyTuple>()?;
                let k: String = tuple.get_item(0)?.extract()?;
                let v: String = tuple.get_item(1)?.extract()?;
                other_items.push((k.to_lowercase(), v));
            }
            other_items.sort();
            Ok(self_items == other_items)
        } else if let Ok(dict) = other.downcast::<PyDict>() {
            // Compare with dict (unique keys, first value)
            let mut self_items: Vec<(String, String)> = self
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            self_items.sort();
            let mut other_items: Vec<(String, String)> = Vec::new();
            for (k, v) in dict.iter() {
                let key: String = k.extract()?;
                let value: String = v.extract()?;
                other_items.push((key.to_lowercase(), value));
            }
            other_items.sort();
            Ok(self_items == other_items)
        } else {
            Ok(false)
        }
    }

    fn __repr__(&self) -> String {
        // Check if there are duplicate keys
        let unique_keys: std::collections::HashSet<String> = self
            .inner
            .iter()
            .map(|(k, _)| k.to_lowercase())
            .collect();

        if unique_keys.len() == self.inner.len() {
            // No duplicates - use dict format
            let items: Vec<String> = self
                .inner
                .iter()
                .map(|(k, v)| format!("'{}': '{}'", k, v))
                .collect();
            format!("Headers({{{}}})", items.join(", "))
        } else {
            // Has duplicates - use list format
            let items: Vec<String> = self
                .inner
                .iter()
                .map(|(k, v)| format!("('{}', '{}')", k, v))
                .collect();
            format!("Headers([{}])", items.join(", "))
        }
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
