//! Query Parameters implementation

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

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
                let k: String = key.extract()?;
                // Handle both single values and lists
                if let Ok(list) = value.downcast::<PyList>() {
                    for item in list.iter() {
                        let v: String = item.extract()?;
                        params.inner.push((k.clone(), v));
                    }
                } else {
                    let v: String = value.extract()?;
                    params.inner.push((k, v));
                }
            }
        } else if let Ok(list) = obj.downcast::<PyList>() {
            for item in list.iter() {
                let tuple = item.downcast::<PyTuple>()?;
                let k: String = tuple.get_item(0)?.extract()?;
                let v: String = tuple.get_item(1)?.extract()?;
                params.inner.push((k, v));
            }
        } else if let Ok(qp) = obj.extract::<QueryParams>() {
            params.inner = qp.inner;
        } else if let Ok(s) = obj.extract::<String>() {
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
            // Remove existing keys first, then add
            self.inner.retain(|(existing_k, _)| existing_k != k);
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
        // Return first value for each unique key
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
            // Sort both for comparison (order-independent equality)
            let mut self_sorted = self.inner.clone();
            self_sorted.sort();
            let mut other_sorted = other_qp.inner.clone();
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
        let mut hasher = DefaultHasher::new();
        for (k, v) in &self.inner {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Return a new QueryParams with the key set to value (replacing any existing)
    #[pyo3(name = "set")]
    fn py_set(&self, key: &str, value: &str) -> Self {
        let mut new_qp = self.clone();
        new_qp.set(key, value);
        new_qp
    }

    /// Return a new QueryParams with an additional key-value pair (allowing duplicates)
    #[pyo3(name = "add")]
    fn py_add(&self, key: &str, value: &str) -> Self {
        let mut new_qp = self.clone();
        new_qp.add(key, value);
        new_qp
    }

    /// Return a new QueryParams with all values for the given key removed
    #[pyo3(name = "remove")]
    fn py_remove(&self, key: &str) -> Self {
        let mut new_qp = self.clone();
        new_qp.remove(key);
        new_qp
    }

    /// Return a new QueryParams with additional params merged
    #[pyo3(name = "merge")]
    fn py_merge(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut new_qp = self.clone();
        let other_qp = Self::from_py(other)?;
        new_qp.merge(&other_qp);
        Ok(new_qp)
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
