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

/// URL type for URL parsing and manipulation (HTTPX compatible)
#[pyclass(name = "URL")]
#[derive(Debug, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub struct URL {
    inner: url::Url,
}

#[pymethods]
impl URL {
    #[new]
    #[pyo3(signature = (url))]
    pub fn new(url: &str) -> PyResult<Self> {
        let inner = url::Url::parse(url).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {e}")))?;
        Ok(Self { inner })
    }

    /// Get the scheme (e.g., "http", "https")
    #[getter]
    pub fn scheme(&self) -> &str {
        self.inner.scheme()
    }

    /// Get the host (e.g., "example.com")
    #[getter]
    pub fn host(&self) -> Option<String> {
        self.inner.host_str().map(|s| s.to_string())
    }

    /// Get the port number
    #[getter]
    pub fn port(&self) -> Option<u16> {
        self.inner.port_or_known_default()
    }

    /// Get the path (e.g., "/api/v1/users")
    #[getter]
    pub fn path(&self) -> &str {
        self.inner.path()
    }

    /// Get the query string (without the leading '?')
    #[getter]
    pub fn query(&self) -> Option<&str> {
        self.inner.query()
    }

    /// Get the query parameters as a QueryParams object (HTTPX compatible)
    #[getter]
    pub fn params(&self) -> QueryParams {
        match self.inner.query() {
            Some(query) => {
                let mut pairs = Vec::new();
                for pair in query.split('&') {
                    if pair.is_empty() {
                        continue;
                    }
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next().unwrap_or("");
                    let value = parts.next().unwrap_or("");
                    // URL decode
                    let key = urlencoding::decode(key)
                        .unwrap_or_else(|_| key.into())
                        .to_string();
                    let value = urlencoding::decode(value)
                        .unwrap_or_else(|_| value.into())
                        .to_string();
                    pairs.push((key, value));
                }
                QueryParams::from_pairs(pairs)
            }
            None => QueryParams::default(),
        }
    }

    /// Get the fragment (without the leading '#')
    #[getter]
    pub fn fragment(&self) -> Option<&str> {
        self.inner.fragment()
    }

    /// Get the raw path and query string as bytes (HTTPX compatible)
    #[getter]
    pub fn raw_path(&self) -> Vec<u8> {
        let path = self.inner.path();
        match self.inner.query() {
            Some(query) => format!("{path}?{query}").into_bytes(),
            None => path.as_bytes().to_vec(),
        }
    }

    /// Check if the URL uses a default port for its scheme
    #[getter]
    pub fn is_default_port(&self) -> bool {
        self.inner.port().is_none()
    }

    /// Get the origin (scheme + host + port)
    #[getter]
    pub fn origin(&self) -> String {
        let scheme = self.inner.scheme();
        let host = self.inner.host_str().unwrap_or("");
        match self.inner.port() {
            Some(port) => format!("{scheme}://{host}:{port}"),
            None => format!("{scheme}://{host}"),
        }
    }

    /// Get username if present
    #[getter]
    pub fn username(&self) -> &str {
        self.inner.username()
    }

    /// Get password if present
    #[getter]
    pub fn password(&self) -> Option<&str> {
        self.inner.password()
    }

    /// Join with another URL or path
    pub fn join(&self, url: &str) -> PyResult<URL> {
        let joined = self
            .inner
            .join(url)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to join URLs: {e}")))?;
        Ok(URL { inner: joined })
    }

    /// Copy the URL with modifications (HTTPX compatible)
    ///
    /// Supports both HTTPX-style `params` parameter (dict, QueryParams, or string)
    /// and the `raw_path` parameter (bytes) for path manipulation.
    #[pyo3(signature = (scheme=None, host=None, port=None, path=None, raw_path=None, query=None, params=None, fragment=None))]
    pub fn copy_with(
        &self,
        py: Python<'_>,
        scheme: Option<&str>,
        host: Option<&str>,
        port: Option<u16>,
        path: Option<&str>,
        raw_path: Option<&Bound<'_, PyAny>>,
        query: Option<&str>,
        params: Option<&Bound<'_, PyAny>>,
        fragment: Option<&str>,
    ) -> PyResult<URL> {
        let mut new_url = self.inner.clone();

        if let Some(s) = scheme {
            new_url
                .set_scheme(s)
                .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid scheme"))?;
        }
        if let Some(h) = host {
            new_url
                .set_host(Some(h))
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid host: {e}")))?;
        }
        if let Some(p) = port {
            new_url
                .set_port(Some(p))
                .map_err(|_| pyo3::exceptions::PyValueError::new_err("Invalid port"))?;
        }

        // Handle raw_path (bytes) - HTTPX compatibility
        // raw_path can contain both path and query, e.g., b"/path?query=value"
        if let Some(raw) = raw_path {
            let raw_bytes: Vec<u8> = if let Ok(bytes) = raw.extract::<Vec<u8>>() {
                bytes
            } else if raw.is_instance_of::<pyo3::types::PyBytes>() {
                raw.cast::<pyo3::types::PyBytes>()
                    .unwrap()
                    .as_bytes()
                    .to_vec()
            } else if let Ok(s) = raw.extract::<String>() {
                s.into_bytes()
            } else {
                return Err(pyo3::exceptions::PyValueError::new_err("raw_path must be bytes or str"));
            };

            let raw_str = String::from_utf8_lossy(&raw_bytes);
            // Split into path and query
            if let Some(query_start) = raw_str.find('?') {
                let (path_part, query_part) = raw_str.split_at(query_start);
                new_url.set_path(path_part);
                // Remove the leading '?' from query
                new_url.set_query(Some(&query_part[1..]));
            } else {
                new_url.set_path(&raw_str);
            }
        } else if let Some(p) = path {
            new_url.set_path(p);
        }

        // Handle params (dict, QueryParams, or string) - HTTPX compatibility
        // params takes precedence over query if both are specified
        if let Some(p) = params {
            let query_str = if let Ok(qp) = p.extract::<QueryParams>() {
                qp.to_query_string()
            } else if let Ok(s) = p.extract::<String>() {
                s
            } else if p.is_instance_of::<PyDict>() {
                let qp = QueryParams::new(Some(p))?;
                qp.to_query_string()
            } else {
                return Err(pyo3::exceptions::PyValueError::new_err("params must be a dict, QueryParams, or string"));
            };

            if query_str.is_empty() {
                new_url.set_query(None);
            } else {
                new_url.set_query(Some(&query_str));
            }
        } else if let Some(q) = query {
            new_url.set_query(Some(q));
        }
        // If neither params nor query specified, keep existing query

        if let Some(f) = fragment {
            new_url.set_fragment(Some(f));
        }

        // Suppress unused variable warning
        let _ = py;

        Ok(URL { inner: new_url })
    }

    /// Compare equality with another URL or string
    pub fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(url) = other.extract::<URL>() {
            Ok(self.inner == url.inner)
        } else if let Ok(s) = other.extract::<String>() {
            Ok(self.inner.as_str() == s)
        } else {
            Ok(false)
        }
    }

    pub fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.as_str().hash(&mut hasher);
        hasher.finish()
    }

    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }

    pub fn __repr__(&self) -> String {
        format!("URL('{}')", self.inner)
    }
}

impl URL {
    /// Create from url::Url
    pub fn from_url(url: url::Url) -> Self {
        Self { inner: url }
    }

    /// Get the inner url::Url
    pub fn as_url(&self) -> &url::Url {
        &self.inner
    }

    /// Get the URL as a string
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

/// QueryParams type for URL query string handling (HTTPX compatible)
///
/// Supports multi-value parameters like HTTPX's QueryParams class.
/// Can be initialized from:
/// - None: empty params
/// - str: raw query string (will be parsed)
/// - dict: key-value pairs (values can be strings or lists)
/// - list of tuples: [(key, value), ...]
/// - another QueryParams object
#[pyclass(name = "QueryParams")]
#[derive(Debug, Clone, Default)]
pub struct QueryParams {
    /// Internal storage: list of (key, value) pairs to preserve order and support multi-values
    inner: Vec<(String, String)>,
}

#[pymethods]
impl QueryParams {
    #[new]
    #[pyo3(signature = (params=None))]
    pub fn new(params: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut inner = Vec::new();

        if let Some(p) = params {
            // Check if it's None
            if p.is_none() {
                return Ok(Self { inner });
            }

            // Check if it's a QueryParams object
            if let Ok(qp) = p.extract::<QueryParams>() {
                return Ok(qp);
            }

            // Check if it's a string (raw query string)
            if let Ok(s) = p.extract::<String>() {
                // Parse the query string
                let query = s.trim_start_matches('?');
                for pair in query.split('&') {
                    if pair.is_empty() {
                        continue;
                    }
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next().unwrap_or("");
                    let value = parts.next().unwrap_or("");
                    // URL decode
                    let key = urlencoding::decode(key)
                        .unwrap_or_else(|_| key.into())
                        .to_string();
                    let value = urlencoding::decode(value)
                        .unwrap_or_else(|_| value.into())
                        .to_string();
                    inner.push((key, value));
                }
                return Ok(Self { inner });
            }

            // Check if it's a list of tuples
            if p.is_instance_of::<pyo3::types::PyList>() {
                let list = p.cast::<pyo3::types::PyList>().unwrap();
                for item in list.iter() {
                    if let Ok(tuple) = item.extract::<(String, String)>() {
                        inner.push(tuple);
                    } else if let Ok(tuple) = item.extract::<(&str, &str)>() {
                        inner.push((tuple.0.to_string(), tuple.1.to_string()));
                    }
                }
                return Ok(Self { inner });
            }

            // Check if it's a dict
            if p.is_instance_of::<PyDict>() {
                let dict = p.cast::<PyDict>().unwrap();
                for (key, value) in dict.iter() {
                    let key: String = key.extract()?;
                    // Handle both single values and lists
                    if let Ok(values) = value.extract::<Vec<String>>() {
                        for v in values {
                            inner.push((key.clone(), v));
                        }
                    } else if let Ok(v) = value.extract::<String>() {
                        inner.push((key, v));
                    } else {
                        // Convert other types to string
                        let v = value.str()?.to_string();
                        inner.push((key, v));
                    }
                }
                return Ok(Self { inner });
            }

            return Err(PyValueError::new_err("QueryParams must be initialized with None, str, dict, list of tuples, or QueryParams"));
        }

        Ok(Self { inner })
    }

    /// Get the first value for a key, or default if not found
    #[pyo3(signature = (key, default=None))]
    pub fn get(&self, key: &str, default: Option<&str>) -> Option<String> {
        for (k, v) in &self.inner {
            if k == key {
                return Some(v.clone());
            }
        }
        default.map(|s| s.to_string())
    }

    /// Get all values for a key as a list
    pub fn get_list(&self, key: &str) -> Vec<String> {
        self.inner
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Get all unique keys
    pub fn keys(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.inner
            .iter()
            .filter_map(|(k, _)| {
                if seen.contains(k) {
                    None
                } else {
                    seen.insert(k.clone());
                    Some(k.clone())
                }
            })
            .collect()
    }

    /// Get all values (one per unique key, first occurrence)
    pub fn values(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.inner
            .iter()
            .filter_map(|(k, v)| {
                if seen.contains(k) {
                    None
                } else {
                    seen.insert(k.clone());
                    Some(v.clone())
                }
            })
            .collect()
    }

    /// Get all unique key-value pairs (first occurrence per key)
    pub fn items(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        let mut seen = std::collections::HashSet::new();
        for (key, value) in &self.inner {
            if !seen.contains(key) {
                seen.insert(key.clone());
                let tuple = PyTuple::new(py, &[key.clone(), value.clone()])?;
                list.append(tuple)?;
            }
        }
        Ok(list.into())
    }

    /// Get all key-value pairs including duplicates
    pub fn multi_items(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        for (key, value) in &self.inner {
            let tuple = PyTuple::new(py, &[key.clone(), value.clone()])?;
            list.append(tuple)?;
        }
        Ok(list.into())
    }

    /// Merge with another QueryParams or dict-like object
    pub fn merge(&self, other: &Bound<'_, PyAny>) -> PyResult<QueryParams> {
        let mut new_params = self.clone();

        if let Ok(qp) = other.extract::<QueryParams>() {
            new_params.inner.extend(qp.inner);
        } else if other.is_instance_of::<PyDict>() {
            let dict = other.cast::<PyDict>().unwrap();
            for (key, value) in dict.iter() {
                let key: String = key.extract()?;
                if let Ok(values) = value.extract::<Vec<String>>() {
                    for v in values {
                        new_params.inner.push((key.clone(), v));
                    }
                } else if let Ok(v) = value.extract::<String>() {
                    new_params.inner.push((key, v));
                } else {
                    let v = value.str()?.to_string();
                    new_params.inner.push((key, v));
                }
            }
        } else {
            return Err(PyValueError::new_err("merge argument must be a QueryParams or dict"));
        }

        Ok(new_params)
    }

    /// Set a value, removing any existing values for that key
    pub fn set(&self, key: &str, value: &str) -> QueryParams {
        let mut new_params = QueryParams {
            inner: self
                .inner
                .iter()
                .filter(|(k, _)| k != key)
                .cloned()
                .collect(),
        };
        new_params.inner.push((key.to_string(), value.to_string()));
        new_params
    }

    /// Add a value for a key (allows duplicates)
    pub fn add(&self, key: &str, value: &str) -> QueryParams {
        let mut new_params = self.clone();
        new_params.inner.push((key.to_string(), value.to_string()));
        new_params
    }

    /// Remove all values for a key
    pub fn remove(&self, key: &str) -> QueryParams {
        QueryParams {
            inner: self
                .inner
                .iter()
                .filter(|(k, _)| k != key)
                .cloned()
                .collect(),
        }
    }

    pub fn __len__(&self) -> usize {
        self.keys().len()
    }

    pub fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.iter().any(|(k, _)| k == key)
    }

    pub fn __getitem__(&self, key: &str) -> PyResult<String> {
        self.get(key, None)
            .ok_or_else(|| PyValueError::new_err(format!("Key '{key}' not found")))
    }

    pub fn __iter__(&self) -> QueryParamsIterator {
        QueryParamsIterator { keys: self.keys(), index: 0 }
    }

    pub fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(qp) = other.extract::<QueryParams>() {
            Ok(self.inner == qp.inner)
        } else {
            Ok(false)
        }
    }

    pub fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (k, v) in &self.inner {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn __str__(&self) -> String {
        self.inner
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    pub fn __repr__(&self) -> String {
        format!("QueryParams('{}')", self.__str__())
    }
}

impl QueryParams {
    /// Create from a vector of key-value pairs
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        Self { inner: pairs }
    }

    /// Get the internal pairs
    pub fn as_pairs(&self) -> &[(String, String)] {
        &self.inner
    }

    /// Convert to URL-encoded query string
    pub fn to_query_string(&self) -> String {
        self.__str__()
    }
}

/// Iterator for QueryParams keys
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

/// Request type for representing HTTP requests (HTTPX compatible)
#[pyclass(name = "Request")]
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method
    #[pyo3(get)]
    pub method: String,

    /// Request URL
    url: URL,

    /// Request headers
    headers: Headers,

    /// Request body content
    content: Option<Vec<u8>>,

    /// Stream flag - whether this request expects a streaming response
    #[pyo3(get)]
    pub stream: bool,
}

#[pymethods]
impl Request {
    #[new]
    #[pyo3(signature = (method, url, headers=None, content=None, stream=false))]
    pub fn new(method: &str, url: &Bound<'_, PyAny>, headers: Option<&Bound<'_, PyAny>>, content: Option<&Bound<'_, pyo3::types::PyBytes>>, stream: bool) -> PyResult<Self> {
        let url = if let Ok(url_obj) = url.extract::<URL>() {
            url_obj
        } else if let Ok(url_str) = url.extract::<String>() {
            URL::new(&url_str)?
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err("url must be a string or URL object"));
        };

        let headers = if let Some(h) = headers {
            extract_headers(h)?
        } else {
            Headers::default()
        };

        let content = content.map(|c| c.as_bytes().to_vec());

        Ok(Self {
            method: method.to_uppercase(),
            url,
            headers,
            content,
            stream,
        })
    }

    /// Get request URL
    #[getter]
    pub fn url(&self) -> URL {
        self.url.clone()
    }

    /// Get request headers
    #[getter]
    pub fn headers(&self) -> Headers {
        self.headers.clone()
    }

    /// Get request content as bytes
    #[getter]
    pub fn content<'py>(&self, py: Python<'py>) -> Option<Bound<'py, pyo3::types::PyBytes>> {
        self.content
            .as_ref()
            .map(|c| pyo3::types::PyBytes::new(py, c))
    }

    pub fn __repr__(&self) -> String {
        format!("<Request('{}', '{}')>", self.method, self.url.as_str())
    }

    pub fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl Request {
    /// Create a new Request with all fields
    pub fn new_internal(method: String, url: URL, headers: Headers, content: Option<Vec<u8>>, stream: bool) -> Self {
        Self {
            method,
            url,
            headers,
            content,
            stream,
        }
    }

    /// Get the URL as a string
    pub fn url_str(&self) -> &str {
        self.url.as_str()
    }

    /// Get the headers reference
    pub fn headers_ref(&self) -> &Headers {
        &self.headers
    }

    /// Get the content reference
    pub fn content_ref(&self) -> Option<&Vec<u8>> {
        self.content.as_ref()
    }
}
