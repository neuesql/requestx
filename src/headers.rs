//! HTTP Headers implementation

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::HashMap;

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

    /// Iterate over header (key, value) pairs
    pub fn iter_pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Set a header value (removes existing headers with same key, retains position)
    pub fn set(&mut self, key: String, value: String) {
        let key_lower = key.to_lowercase();
        // Find the position of the first existing header with the same key
        let first_pos = self.inner.iter().position(|(k, _)| k.to_lowercase() == key_lower);

        // Remove all existing headers with same key
        self.inner.retain(|(k, _)| k.to_lowercase() != key_lower);

        // Insert at original position if existed, otherwise append
        if let Some(pos) = first_pos {
            self.inner.insert(pos, (key, value));
        } else {
            self.inner.push((key, value));
        }
    }

    /// Check if a header exists
    pub fn contains(&self, key: &str) -> bool {
        let key_lower = key.to_lowercase();
        self.inner.iter().any(|(k, _)| k.to_lowercase() == key_lower)
    }

    /// Get a header value
    pub fn get(&self, key: &str, default: Option<&str>) -> Option<String> {
        let key_lower = key.to_lowercase();
        self.inner
            .iter()
            .find(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.clone())
            .or_else(|| default.map(|s| s.to_string()))
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
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    h.inner.push((k, v));
                }
            } else if let Ok(list) = obj.downcast::<PyList>() {
                for item in list.iter() {
                    let tuple = item.downcast::<PyTuple>()?;
                    let k: String = tuple.get_item(0)?.extract()?;
                    let v: String = tuple.get_item(1)?.extract()?;
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
        // Return comma-joined values for duplicate keys, in order of first appearance
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for (k, _) in &self.inner {
            let lower = k.to_lowercase();
            if seen.insert(lower.clone()) {
                // First time seeing this key, collect all values
                let values: Vec<&str> = self.inner
                    .iter()
                    .filter(|(k2, _)| k2.to_lowercase() == lower)
                    .map(|(_, v)| v.as_str())
                    .collect();
                result.push(values.join(", "));
            }
        }
        result
    }

    fn items(&self) -> Vec<(String, String)> {
        // Return items with comma-joined values for duplicate keys
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for (k, _) in &self.inner {
            let lower = k.to_lowercase();
            if seen.insert(lower.clone()) {
                // First time seeing this key, collect all values
                let values: Vec<&str> = self.inner
                    .iter()
                    .filter(|(k2, _)| k2.to_lowercase() == lower)
                    .map(|(_, v)| v.as_str())
                    .collect();
                result.push((k.clone(), values.join(", ")));
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
        // Find the position of the first existing header with the same key
        let first_pos = self.inner.iter().position(|(k, _)| k.to_lowercase() == key_lower);

        // Remove all existing headers with same key
        self.inner.retain(|(k, _)| k.to_lowercase() != key_lower);

        // Insert at original position if existed, otherwise append
        if let Some(pos) = first_pos {
            self.inner.insert(pos, (key, value));
        } else {
            self.inner.push((key, value));
        }
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
            // Compare multi_items with case-insensitive keys
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
        } else if let Ok(list) = other.downcast::<PyList>() {
            // Compare with list of tuples (case-insensitive keys)
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
        } else if let Ok(dict) = other.downcast::<PyDict>() {
            // Compare with dict (case-insensitive keys, combine values)
            let mut self_items: Vec<(String, String)> = self
                .inner
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
            let mut other_items: Vec<(String, String)> = Vec::new();
            for (k, v) in dict.iter() {
                let key: String = k.extract()?;
                let value: String = v.extract()?;
                other_items.push((key.to_lowercase(), value));
            }
            self_items.sort();
            other_items.sort();
            Ok(self_items == other_items)
        } else {
            Ok(false)
        }
    }

    fn __repr__(&self) -> String {
        // Check if there are duplicate keys (case-insensitive)
        let mut seen = std::collections::HashSet::new();
        let mut has_duplicates = false;
        for (k, _) in &self.inner {
            if !seen.insert(k.to_lowercase()) {
                has_duplicates = true;
                break;
            }
        }

        if has_duplicates {
            // Use list format for multi-value headers
            let items: Vec<String> = self
                .inner
                .iter()
                .map(|(k, v)| format!("('{}', '{}')", k, v))
                .collect();
            format!("Headers([{}])", items.join(", "))
        } else {
            // Use dict format for single-value headers
            let items: Vec<String> = self
                .inner
                .iter()
                .map(|(k, v)| format!("'{}': '{}'", k, v))
                .collect();
            format!("Headers({{{}}})", items.join(", "))
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

    fn setdefault(&mut self, key: String, default: String) -> String {
        let key_lower = key.to_lowercase();
        // Check if key exists
        if let Some(existing) = self.inner
            .iter()
            .find(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v.clone())
        {
            existing
        } else {
            // Key doesn't exist, set default value
            self.inner.push((key, default.clone()));
            default
        }
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
