//! Common types for requestx

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::HashMap;
use std::time::Duration;

/// HTTP Headers wrapper
#[pyclass(name = "Headers")]
#[derive(Debug, Clone, Default)]
pub struct Headers {
    pub inner: HashMap<String, Vec<String>>,
}

#[pymethods]
impl Headers {
    #[new]
    #[pyo3(signature = (headers=None))]
    pub fn new(headers: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut inner = HashMap::new();
        if let Some(dict) = headers {
            for (key, value) in dict.iter() {
                let key: String = key.extract()?;
                let key_lower = key.to_lowercase();
                let value: String = value.extract()?;
                inner.entry(key_lower).or_insert_with(Vec::new).push(value);
            }
        }
        Ok(Self { inner })
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.inner
            .get(&key.to_lowercase())
            .and_then(|v| v.first().cloned())
    }

    pub fn get_list(&self, key: &str) -> Vec<String> {
        self.inner
            .get(&key.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.inner
            .insert(key.to_lowercase(), vec![value.to_string()]);
    }

    pub fn add(&mut self, key: &str, value: &str) {
        self.inner
            .entry(key.to_lowercase())
            .or_default()
            .push(value.to_string());
    }

    pub fn remove(&mut self, key: &str) {
        self.inner.remove(&key.to_lowercase());
    }

    pub fn keys(&self) -> Vec<String> {
        self.inner.keys().cloned().collect()
    }

    pub fn values(&self) -> Vec<String> {
        self.inner
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect()
    }

    pub fn items(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        for (key, values) in &self.inner {
            for value in values {
                let tuple = PyTuple::new(py, &[key.clone(), value.clone()])?;
                list.append(tuple)?;
            }
        }
        Ok(list.into())
    }

    pub fn __len__(&self) -> usize {
        self.inner.values().map(|v| v.len()).sum()
    }

    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.contains_key(&key.to_lowercase())
    }

    pub fn __getitem__(&self, key: &str) -> PyResult<String> {
        self.get(key)
            .ok_or_else(|| PyValueError::new_err(format!("Header '{key}' not found")))
    }

    pub fn __setitem__(&mut self, key: &str, value: &str) {
        self.set(key, value);
    }

    pub fn __delitem__(&mut self, key: &str) {
        self.remove(key);
    }

    pub fn __repr__(&self) -> String {
        format!("Headers({:?})", self.inner)
    }

    pub fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl Headers {
    pub fn to_reqwest_headers(&self) -> reqwest::header::HeaderMap {
        let mut map = reqwest::header::HeaderMap::new();
        for (key, values) in &self.inner {
            if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                for value in values {
                    if let Ok(val) = reqwest::header::HeaderValue::from_str(value) {
                        map.append(name.clone(), val);
                    }
                }
            }
        }
        map
    }

    pub fn from_reqwest_headers(headers: &reqwest::header::HeaderMap) -> Self {
        let mut inner = HashMap::new();
        for (key, value) in headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if let Ok(value_str) = value.to_str() {
                inner
                    .entry(key_str)
                    .or_insert_with(Vec::new)
                    .push(value_str.to_string());
            }
        }
        Self { inner }
    }
}

/// Cookie storage wrapper
#[pyclass(name = "Cookies")]
#[derive(Debug, Clone, Default)]
pub struct Cookies {
    pub inner: HashMap<String, String>,
}

#[pymethods]
impl Cookies {
    #[new]
    #[pyo3(signature = (cookies=None))]
    pub fn new(cookies: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut inner = HashMap::new();
        if let Some(dict) = cookies {
            for (key, value) in dict.iter() {
                let key: String = key.extract()?;
                let value: String = value.extract()?;
                inner.insert(key, value);
            }
        }
        Ok(Self { inner })
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.inner.get(name).cloned()
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.inner.insert(name.to_string(), value.to_string());
    }

    pub fn delete(&mut self, name: &str) {
        self.inner.remove(name);
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn keys(&self) -> Vec<String> {
        self.inner.keys().cloned().collect()
    }

    pub fn values(&self) -> Vec<String> {
        self.inner.values().cloned().collect()
    }

    pub fn items(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        for (key, value) in &self.inner {
            let tuple = PyTuple::new(py, &[key.clone(), value.clone()])?;
            list.append(tuple)?;
        }
        Ok(list.into())
    }

    pub fn __len__(&self) -> usize {
        self.inner.len()
    }

    pub fn __contains__(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }

    pub fn __getitem__(&self, name: &str) -> PyResult<String> {
        self.get(name)
            .ok_or_else(|| PyValueError::new_err(format!("Cookie '{name}' not found")))
    }

    pub fn __setitem__(&mut self, name: &str, value: &str) {
        self.set(name, value);
    }

    pub fn __delitem__(&mut self, name: &str) {
        self.delete(name);
    }

    pub fn __iter__(&self) -> CookiesIterator {
        CookiesIterator {
            keys: self.inner.keys().cloned().collect(),
            index: 0,
        }
    }

    pub fn __repr__(&self) -> String {
        format!("Cookies({:?})", self.inner)
    }

    pub fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Iterator for Cookies keys
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

/// Timeout configuration
#[pyclass(name = "Timeout")]
#[derive(Debug, Clone)]
pub struct Timeout {
    pub connect: Option<Duration>,
    pub read: Option<Duration>,
    pub write: Option<Duration>,
    pub pool: Option<Duration>,
    pub total: Option<Duration>,
}

#[pymethods]
impl Timeout {
    #[new]
    #[pyo3(signature = (timeout=None, connect=None, read=None, write=None, pool=None))]
    pub fn new(timeout: Option<f64>, connect: Option<f64>, read: Option<f64>, write: Option<f64>, pool: Option<f64>) -> Self {
        Self {
            total: timeout.map(Duration::from_secs_f64),
            connect: connect.map(Duration::from_secs_f64),
            read: read.map(Duration::from_secs_f64),
            write: write.map(Duration::from_secs_f64),
            pool: pool.map(Duration::from_secs_f64),
        }
    }

    #[getter]
    pub fn connect_timeout(&self) -> Option<f64> {
        self.connect.map(|d| d.as_secs_f64())
    }

    #[getter]
    pub fn read_timeout(&self) -> Option<f64> {
        self.read.map(|d| d.as_secs_f64())
    }

    #[getter]
    pub fn write_timeout(&self) -> Option<f64> {
        self.write.map(|d| d.as_secs_f64())
    }

    #[getter]
    pub fn pool_timeout(&self) -> Option<f64> {
        self.pool.map(|d| d.as_secs_f64())
    }

    #[getter]
    pub fn total_timeout(&self) -> Option<f64> {
        self.total.map(|d| d.as_secs_f64())
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Timeout(total={:?}, connect={:?}, read={:?}, write={:?}, pool={:?})",
            self.total, self.connect, self.read, self.write, self.pool
        )
    }
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            connect: Some(Duration::from_secs(5)),
            read: Some(Duration::from_secs(5)),
            write: Some(Duration::from_secs(5)),
            pool: Some(Duration::from_secs(5)),
            total: Some(Duration::from_secs(30)),
        }
    }
}

/// Proxy configuration
#[pyclass(name = "Proxy")]
#[derive(Debug, Clone)]
pub struct Proxy {
    pub http: Option<String>,
    pub https: Option<String>,
    pub all: Option<String>,
    pub no_proxy: Option<String>,
}

#[pymethods]
impl Proxy {
    #[new]
    #[pyo3(signature = (url=None, http=None, https=None, all=None, no_proxy=None))]
    pub fn new(url: Option<String>, http: Option<String>, https: Option<String>, all: Option<String>, no_proxy: Option<String>) -> Self {
        // If a single url is provided, use it for all protocols
        let all_proxy = all.or(url);
        Self {
            http: http.or_else(|| all_proxy.clone()),
            https: https.or_else(|| all_proxy.clone()),
            all: all_proxy,
            no_proxy,
        }
    }

    #[getter]
    pub fn http_proxy(&self) -> Option<String> {
        self.http.clone()
    }

    #[getter]
    pub fn https_proxy(&self) -> Option<String> {
        self.https.clone()
    }

    pub fn __repr__(&self) -> String {
        format!("Proxy(http={:?}, https={:?}, no_proxy={:?})", self.http, self.https, self.no_proxy)
    }
}

/// Resource limits configuration (like HTTPX Limits)
#[pyclass(name = "Limits")]
#[derive(Debug, Clone)]
pub struct Limits {
    pub max_connections: Option<usize>,
    pub max_keepalive_connections: Option<usize>,
    pub keepalive_expiry: Option<Duration>,
}

#[pymethods]
impl Limits {
    #[new]
    #[pyo3(signature = (max_connections=None, max_keepalive_connections=None, keepalive_expiry=None))]
    pub fn new(max_connections: Option<usize>, max_keepalive_connections: Option<usize>, keepalive_expiry: Option<f64>) -> Self {
        Self {
            max_connections,
            max_keepalive_connections,
            keepalive_expiry: keepalive_expiry.map(Duration::from_secs_f64),
        }
    }

    #[getter]
    pub fn get_max_connections(&self) -> Option<usize> {
        self.max_connections
    }

    #[getter]
    pub fn get_max_keepalive_connections(&self) -> Option<usize> {
        self.max_keepalive_connections
    }

    #[getter]
    pub fn get_keepalive_expiry(&self) -> Option<f64> {
        self.keepalive_expiry.map(|d| d.as_secs_f64())
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Limits(max_connections={:?}, max_keepalive_connections={:?}, keepalive_expiry={:?})",
            self.max_connections, self.max_keepalive_connections, self.keepalive_expiry
        )
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_connections: Some(100),
            max_keepalive_connections: Some(20),
            keepalive_expiry: Some(Duration::from_secs(5)),
        }
    }
}

/// SSL/TLS configuration
#[pyclass(name = "SSLConfig")]
#[derive(Debug, Clone, Default)]
pub struct SSLConfig {
    /// Path to CA bundle file for verification
    pub ca_bundle: Option<String>,
    /// Path to client certificate file
    pub cert_file: Option<String>,
    /// Path to client certificate key file
    pub key_file: Option<String>,
    /// Password for encrypted key file
    pub key_password: Option<String>,
    /// Whether to verify SSL certificates
    pub verify: bool,
}

#[pymethods]
impl SSLConfig {
    #[new]
    #[pyo3(signature = (verify=true, ca_bundle=None, cert=None, key=None, key_password=None))]
    pub fn new(verify: bool, ca_bundle: Option<String>, cert: Option<String>, key: Option<String>, key_password: Option<String>) -> Self {
        Self {
            verify,
            ca_bundle,
            cert_file: cert,
            key_file: key,
            key_password,
        }
    }

    #[getter]
    pub fn get_verify(&self) -> bool {
        self.verify
    }

    #[getter]
    pub fn get_ca_bundle(&self) -> Option<String> {
        self.ca_bundle.clone()
    }

    #[getter]
    pub fn get_cert_file(&self) -> Option<String> {
        self.cert_file.clone()
    }

    #[getter]
    pub fn get_key_file(&self) -> Option<String> {
        self.key_file.clone()
    }

    pub fn __repr__(&self) -> String {
        format!("SSLConfig(verify={}, ca_bundle={:?}, cert={:?}, key={:?})", self.verify, self.ca_bundle, self.cert_file, self.key_file)
    }
}

/// Authentication configuration
#[pyclass(name = "Auth")]
#[derive(Debug, Clone)]
pub struct Auth {
    pub auth_type: AuthType,
}

#[derive(Debug, Clone)]
pub enum AuthType {
    Basic { username: String, password: String },
    Bearer { token: String },
    Digest { username: String, password: String },
}

#[pymethods]
impl Auth {
    /// Create basic authentication
    #[staticmethod]
    pub fn basic(username: String, password: String) -> Self {
        Self {
            auth_type: AuthType::Basic { username, password },
        }
    }

    /// Create bearer token authentication
    #[staticmethod]
    pub fn bearer(token: String) -> Self {
        Self {
            auth_type: AuthType::Bearer { token },
        }
    }

    /// Create digest authentication (falls back to basic in reqwest)
    #[staticmethod]
    pub fn digest(username: String, password: String) -> Self {
        Self {
            auth_type: AuthType::Digest { username, password },
        }
    }

    pub fn __repr__(&self) -> String {
        match &self.auth_type {
            AuthType::Basic { username, .. } => format!("Auth.basic('{username}', '***')"),
            AuthType::Bearer { .. } => "Auth.bearer('***')".to_string(),
            AuthType::Digest { username, .. } => format!("Auth.digest('{username}', '***')"),
        }
    }
}

/// Query parameters helper
pub fn extract_params(params: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<(String, String)>> {
    let mut result = Vec::new();
    if let Some(dict) = params {
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            // Handle both single values and lists
            if let Ok(values) = value.extract::<Vec<String>>() {
                for v in values {
                    result.push((key.clone(), v));
                }
            } else {
                let value: String = value.extract()?;
                result.push((key, value));
            }
        }
    }
    Ok(result)
}

/// Extract cookies from PyDict or Cookies object
pub fn extract_cookies(cookies: &Bound<'_, PyAny>) -> PyResult<HashMap<String, String>> {
    if let Ok(cookies_obj) = cookies.extract::<Cookies>() {
        Ok(cookies_obj.inner)
    } else if cookies.is_instance_of::<PyDict>() {
        let dict = cookies.extract::<Bound<'_, PyDict>>()?;
        let mut result = HashMap::new();
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            let value: String = value.extract()?;
            result.insert(key, value);
        }
        Ok(result)
    } else {
        Err(PyValueError::new_err("cookies must be a dict or Cookies object"))
    }
}

/// Extract headers from PyDict or Headers object
pub fn extract_headers(headers: &Bound<'_, PyAny>) -> PyResult<Headers> {
    if let Ok(headers_obj) = headers.extract::<Headers>() {
        Ok(headers_obj)
    } else if headers.is_instance_of::<PyDict>() {
        let dict = headers.extract::<Bound<'_, PyDict>>()?;
        Headers::new(Some(&dict))
    } else {
        Err(PyValueError::new_err("headers must be a dict or Headers object"))
    }
}

/// Extract timeout from various input types
pub fn extract_timeout(timeout: &Bound<'_, PyAny>) -> PyResult<Timeout> {
    if let Ok(timeout_obj) = timeout.extract::<Timeout>() {
        Ok(timeout_obj)
    } else if let Ok(secs) = timeout.extract::<f64>() {
        Ok(Timeout::new(Some(secs), None, None, None, None))
    } else if let Ok(tuple) = timeout.extract::<(f64, f64)>() {
        Ok(Timeout::new(None, Some(tuple.0), Some(tuple.1), None, None))
    } else {
        Err(PyValueError::new_err("timeout must be a float, tuple, or Timeout object"))
    }
}

/// Extract verify parameter (bool or path string)
pub fn extract_verify(verify: &Bound<'_, PyAny>) -> PyResult<(bool, Option<String>)> {
    if let Ok(b) = verify.extract::<bool>() {
        Ok((b, None))
    } else if let Ok(path) = verify.extract::<String>() {
        // If it's a string, it's a path to a CA bundle
        Ok((true, Some(path)))
    } else {
        Err(PyValueError::new_err("verify must be a bool or a path string"))
    }
}

/// Extract cert parameter (path string or tuple of (cert, key) or (cert, key, password))
pub fn extract_cert(cert: &Bound<'_, PyAny>) -> PyResult<(Option<String>, Option<String>, Option<String>)> {
    if let Ok(path) = cert.extract::<String>() {
        // Single path - cert file only (key might be in same file)
        Ok((Some(path), None, None))
    } else if let Ok((cert_path, key_path)) = cert.extract::<(String, String)>() {
        // Tuple of (cert, key)
        Ok((Some(cert_path), Some(key_path), None))
    } else if let Ok((cert_path, key_path, password)) = cert.extract::<(String, String, String)>() {
        // Tuple of (cert, key, password)
        Ok((Some(cert_path), Some(key_path), Some(password)))
    } else {
        Err(PyValueError::new_err("cert must be a path string or tuple of (cert, key) or (cert, key, password)"))
    }
}

/// Extract limits from Limits object or dict
pub fn extract_limits(limits: &Bound<'_, PyAny>) -> PyResult<Limits> {
    if let Ok(limits_obj) = limits.extract::<Limits>() {
        Ok(limits_obj)
    } else if limits.is_instance_of::<PyDict>() {
        let dict = limits.extract::<Bound<'_, PyDict>>()?;
        let max_connections = dict
            .get_item("max_connections")?
            .and_then(|v| v.extract().ok());
        let max_keepalive = dict
            .get_item("max_keepalive_connections")?
            .and_then(|v| v.extract().ok());
        let keepalive_expiry = dict
            .get_item("keepalive_expiry")?
            .and_then(|v| v.extract().ok());
        Ok(Limits::new(max_connections, max_keepalive, keepalive_expiry))
    } else {
        Err(PyValueError::new_err("limits must be a Limits object or dict"))
    }
}

/// Get proxy from environment variables
pub fn get_env_proxy() -> Option<Proxy> {
    let http_proxy = std::env::var("HTTP_PROXY")
        .or_else(|_| std::env::var("http_proxy"))
        .ok();
    let https_proxy = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .ok();
    let all_proxy = std::env::var("ALL_PROXY")
        .or_else(|_| std::env::var("all_proxy"))
        .ok();
    let no_proxy = std::env::var("NO_PROXY")
        .or_else(|_| std::env::var("no_proxy"))
        .ok();

    if http_proxy.is_some() || https_proxy.is_some() || all_proxy.is_some() {
        Some(Proxy {
            http: http_proxy.or_else(|| all_proxy.clone()),
            https: https_proxy.or_else(|| all_proxy.clone()),
            all: all_proxy,
            no_proxy,
        })
    } else {
        None
    }
}

/// Get SSL cert paths from environment variables
pub fn get_env_ssl_cert() -> Option<String> {
    std::env::var("SSL_CERT_FILE")
        .or_else(|_| std::env::var("REQUESTS_CA_BUNDLE"))
        .or_else(|_| std::env::var("CURL_CA_BUNDLE"))
        .ok()
}

/// Get SSL cert directory from environment variables
#[allow(dead_code)]
pub fn get_env_ssl_cert_dir() -> Option<String> {
    std::env::var("SSL_CERT_DIR").ok()
}
