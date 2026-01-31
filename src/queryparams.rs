//! Query Parameters implementation

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};

/// Convert a Python value to a string (handles int, float, bool, str)
fn py_to_str(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    if obj.is_none() {
        return Ok(String::new());
    }
    // Check bool before int (since bool is subclass of int in Python)
    if let Ok(b) = obj.downcast::<PyBool>() {
        return Ok(if b.is_true() { "true" } else { "false" }.to_string());
    }
    if let Ok(i) = obj.downcast::<PyInt>() {
        let val: i64 = i.extract()?;
        return Ok(val.to_string());
    }
    if let Ok(f) = obj.downcast::<PyFloat>() {
        let val: f64 = f.extract()?;
        return Ok(val.to_string());
    }
    if let Ok(s) = obj.downcast::<PyString>() {
        return Ok(s.extract::<String>()?);
    }
    // Fall back to str() representation
    Ok(obj.str()?.to_string())
}

/// Query Parameters with support for multiple values per key
#[pyclass(name = "QueryParams")]
#[derive(Clone, Debug, Default)]
pub struct QueryParams {
    inner: Vec<(String, String)>,
}

impl QueryParams {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn from_query_string(query: &str) -> Self {
        let inner: Vec<(String, String)> = query
            .split('&')
            .filter(|s| !s.is_empty())
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or("");
                Some((
                    urlencoding::decode(key).unwrap_or_else(|_| key.into()).into_owned(),
                    urlencoding::decode(value).unwrap_or_else(|_| value.into()).into_owned(),
                ))
            })
            .collect();
        Self { inner }
    }

    pub fn from_py(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut params = Self::new();

        if let Ok(dict) = obj.downcast::<PyDict>() {
            for (key, value) in dict.iter() {
                let k = py_to_str(&key)?;
                // Handle both single values and lists/tuples
                if let Ok(list) = value.downcast::<PyList>() {
                    for item in list.iter() {
                        let v = py_to_str(&item)?;
                        params.inner.push((k.clone(), v));
                    }
                } else if let Ok(tuple) = value.downcast::<PyTuple>() {
                    for item in tuple.iter() {
                        let v = py_to_str(&item)?;
                        params.inner.push((k.clone(), v));
                    }
                } else {
                    let v = py_to_str(&value)?;
                    params.inner.push((k, v));
                }
            }
        } else if let Ok(list) = obj.downcast::<PyList>() {
            for item in list.iter() {
                let tuple = item.downcast::<PyTuple>()?;
                let k = py_to_str(&tuple.get_item(0)?)?;
                let v = py_to_str(&tuple.get_item(1)?)?;
                params.inner.push((k, v));
            }
        } else if let Ok(tuple) = obj.downcast::<PyTuple>() {
            // Handle tuple of tuples
            for item in tuple.iter() {
                let inner_tuple = item.downcast::<PyTuple>()?;
                let k = py_to_str(&inner_tuple.get_item(0)?)?;
                let v = py_to_str(&inner_tuple.get_item(1)?)?;
                params.inner.push((k, v));
            }
        } else if let Ok(qp) = obj.extract::<QueryParams>() {
            params.inner = qp.inner;
        } else if let Ok(s) = obj.extract::<String>() {
            params = Self::from_query_string(&s);
        } else if let Ok(bytes) = obj.downcast::<pyo3::types::PyBytes>() {
            // Handle bytes input - decode as UTF-8
            let s = String::from_utf8_lossy(bytes.as_bytes());
            params = Self::from_query_string(&s);
        }

        Ok(params)
    }

    pub fn to_query_string(&self) -> String {
        self.inner
            .iter()
            .map(|(k, v)| {
                let encoded_key = urlencoding::encode(k).replace("%20", "+");
                let encoded_value = urlencoding::encode(v).replace("%20", "+");
                format!("{}={}", encoded_key, encoded_value)
            })
            .collect::<Vec<_>>()
            .join("&")
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.inner.retain(|(k, _)| k != key);
        self.inner.push((key.to_string(), value.to_string()));
    }

    pub fn add(&mut self, key: &str, value: &str) {
        self.inner.push((key.to_string(), value.to_string()));
    }

    pub fn remove(&mut self, key: &str) {
        self.inner.retain(|(k, _)| k != key);
    }

    pub fn merge(&mut self, other: &QueryParams) {
        for (k, v) in &other.inner {
            self.inner.push((k.clone(), v.clone()));
        }
    }
}

#[pymethods]
impl QueryParams {
    #[new]
    #[pyo3(signature = (params=None))]
    fn py_new(params: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        if let Some(obj) = params {
            Self::from_py(obj)
        } else {
            Ok(Self::new())
        }
    }

    #[pyo3(signature = (key, default=None))]
    fn get(&self, key: &str, default: Option<&str>) -> Option<String> {
        self.inner
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .or_else(|| default.map(|s| s.to_string()))
    }

    /// Returns a new QueryParams with the key set to value (replaces existing)
    #[pyo3(name = "set")]
    fn py_set(&self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut new = self.clone();
        let v = py_to_str(value)?;
        new.set(key, &v);
        Ok(new)
    }

    /// Returns a new QueryParams with the key-value pair added (keeps existing)
    #[pyo3(name = "add")]
    fn py_add(&self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut new = self.clone();
        let v = py_to_str(value)?;
        new.add(key, &v);
        Ok(new)
    }

    /// Returns a new QueryParams with the key removed
    #[pyo3(name = "remove")]
    fn py_remove(&self, key: &str) -> Self {
        let mut new = self.clone();
        new.remove(key);
        new
    }

    /// Returns a new QueryParams merged with another mapping (replaces existing keys)
    #[pyo3(name = "merge")]
    fn py_merge(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut new = self.clone();
        let other_qp = Self::from_py(other)?;
        // Replace existing keys from other_qp
        for (k, v) in &other_qp.inner {
            // Remove existing entries for this key
            new.inner.retain(|(existing_k, _)| existing_k != k);
        }
        // Then add all from other_qp
        for (k, v) in &other_qp.inner {
            new.inner.push((k.clone(), v.clone()));
        }
        Ok(new)
    }

    /// Deprecated: use set/add/remove instead
    fn update(&self, _other: &Bound<'_, PyAny>) -> PyResult<()> {
        Err(pyo3::exceptions::PyRuntimeError::new_err(
            "QueryParams are immutable. Use `q = q.set(...)` instead of `q.update(...)`."
        ))
    }

    fn get_list(&self, key: &str) -> Vec<String> {
        self.inner
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .collect()
    }

    fn keys(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.inner
            .iter()
            .filter_map(|(k, _)| {
                if seen.insert(k.clone()) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn values(&self) -> Vec<String> {
        // Return first value per unique key (matching items() behavior)
        let mut seen = std::collections::HashSet::new();
        self.inner
            .iter()
            .filter_map(|(k, v)| {
                if seen.insert(k.clone()) {
                    Some(v.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn items(&self) -> Vec<(String, String)> {
        // Return unique keys with first value
        let mut seen = std::collections::HashSet::new();
        self.inner
            .iter()
            .filter_map(|(k, v)| {
                if seen.insert(k.clone()) {
                    Some((k.clone(), v.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn multi_items(&self) -> Vec<(String, String)> {
        self.inner.clone()
    }

    fn __getitem__(&self, key: &str) -> PyResult<String> {
        self.inner
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| PyKeyError::new_err(key.to_string()))
    }

    fn __setitem__(&self, _key: &str, _value: &str) -> PyResult<()> {
        Err(pyo3::exceptions::PyRuntimeError::new_err(
            "QueryParams are immutable. Use `q = q.set(...)` instead of `q[\"a\"] = \"value\"`."
        ))
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.iter().any(|(k, _)| k == key)
    }

    fn __iter__(&self) -> QueryParamsIterator {
        QueryParamsIterator {
            keys: self.keys(),
            index: 0,
        }
    }

    fn __len__(&self) -> usize {
        self.keys().len()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_qp) = other.extract::<QueryParams>() {
            // Order-independent comparison: same key-value pairs regardless of order
            // But duplicates must match exactly
            if self.inner.len() != other_qp.inner.len() {
                return Ok(false);
            }
            // Sort both and compare
            let mut self_sorted = self.inner.clone();
            let mut other_sorted = other_qp.inner.clone();
            self_sorted.sort();
            other_sorted.sort();
            Ok(self_sorted == other_sorted)
        } else {
            Ok(false)
        }
    }

    fn __str__(&self) -> String {
        self.to_query_string()
    }

    fn __repr__(&self) -> String {
        format!("QueryParams('{}')", self.to_query_string())
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        // Order-independent hash: sort entries first
        let mut sorted = self.inner.clone();
        sorted.sort();
        let mut hasher = DefaultHasher::new();
        for (k, v) in &sorted {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }
        hasher.finish()
    }
}

#[pyclass]
pub struct QueryParamsIterator {
    keys: Vec<String>,
    index: usize,
}

#[pymethods]
impl QueryParamsIterator {
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
