//! Cookies implementation

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

/// HTTP Cookies jar
#[pyclass(name = "Cookies")]
#[derive(Clone, Debug, Default)]
pub struct Cookies {
    inner: HashMap<String, String>,
}

impl Cookies {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn from_reqwest(jar: &reqwest::cookie::Jar, url: &url::Url) -> Self {
        let mut cookies = Self::new();
        // Note: reqwest's Jar doesn't expose cookies directly
        // We'll need to track cookies ourselves
        cookies
    }

    pub fn to_header_value(&self) -> String {
        self.inner
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn inner(&self) -> &HashMap<String, String> {
        &self.inner
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.inner.insert(name.to_string(), value.to_string());
    }
}

#[pymethods]
impl Cookies {
    #[new]
    #[pyo3(signature = (cookies=None))]
    fn py_new(cookies: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut c = Self::new();

        if let Some(obj) = cookies {
            if let Ok(dict) = obj.downcast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    c.inner.insert(k, v);
                }
            } else if let Ok(other_cookies) = obj.extract::<Cookies>() {
                c.inner = other_cookies.inner;
            }
        }

        Ok(c)
    }

    fn get(&self, name: &str, default: Option<&str>) -> Option<String> {
        self.inner
            .get(name)
            .cloned()
            .or_else(|| default.map(|s| s.to_string()))
    }

    #[pyo3(signature = (name, value, domain=None, path=None))]
    fn set_cookie(&mut self, name: &str, value: &str, domain: Option<&str>, path: Option<&str>) {
        // For simplicity, we just store name=value
        // In a full implementation, we'd handle domain/path
        self.inner.insert(name.to_string(), value.to_string());
    }

    fn delete(&mut self, name: &str) {
        self.inner.remove(name);
    }

    fn clear(&mut self) {
        self.inner.clear();
    }

    fn keys(&self) -> Vec<String> {
        self.inner.keys().cloned().collect()
    }

    fn values(&self) -> Vec<String> {
        self.inner.values().cloned().collect()
    }

    fn items(&self) -> Vec<(String, String)> {
        self.inner.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    fn __getitem__(&self, name: &str) -> PyResult<String> {
        self.inner
            .get(name)
            .cloned()
            .ok_or_else(|| PyKeyError::new_err(name.to_string()))
    }

    fn __setitem__(&mut self, name: String, value: String) {
        self.inner.insert(name, value);
    }

    fn __delitem__(&mut self, name: &str) -> PyResult<()> {
        if self.inner.remove(name).is_some() {
            Ok(())
        } else {
            Err(PyKeyError::new_err(name.to_string()))
        }
    }

    fn __contains__(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }

    fn __iter__(&self) -> CookiesIterator {
        CookiesIterator {
            keys: self.keys(),
            index: 0,
        }
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_cookies) = other.extract::<Cookies>() {
            Ok(self.inner == other_cookies.inner)
        } else if let Ok(dict) = other.downcast::<PyDict>() {
            let mut other_map = HashMap::new();
            for (k, v) in dict.iter() {
                let key: String = k.extract()?;
                let value: String = v.extract()?;
                other_map.insert(key, value);
            }
            Ok(self.inner == other_map)
        } else {
            Ok(false)
        }
    }

    fn __repr__(&self) -> String {
        let items: Vec<String> = self
            .inner
            .iter()
            .map(|(k, v)| format!("<Cookie {}={} for />", k, v))
            .collect();
        format!("Cookies([{}])", items.join(", "))
    }

    fn update(&mut self, other: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(dict) = other.downcast::<PyDict>() {
            for (key, value) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                self.inner.insert(k, v);
            }
        } else if let Ok(cookies) = other.extract::<Cookies>() {
            for (k, v) in cookies.inner {
                self.inner.insert(k, v);
            }
        }
        Ok(())
    }
}

#[pyclass]
pub struct CookiesIterator {
    keys: Vec<String>,
    index: usize,
}

#[pymethods]
impl CookiesIterator {
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
