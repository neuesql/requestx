//! URL type implementation

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::collections::HashMap;
use url::Url;

use crate::queryparams::QueryParams;

/// Maximum URL length (same as httpx)
const MAX_URL_LENGTH: usize = 65536;

/// URL parsing and manipulation
#[pyclass(name = "URL")]
#[derive(Clone, Debug)]
pub struct URL {
    inner: Url,
    fragment: String,
}

impl URL {
    pub fn from_url(url: Url) -> Self {
        let fragment = url.fragment().unwrap_or("").to_string();
        Self { inner: url, fragment }
    }

    pub fn inner(&self) -> &Url {
        &self.inner
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// Parse a URL string
    pub fn parse(url_str: &str) -> PyResult<Self> {
        Self::new_impl(Some(url_str), None, None, None, None, None, None, None, None, None, None, None)
    }

    /// Join with another URL
    pub fn join_url(&self, url: &str) -> PyResult<Self> {
        match self.inner.join(url) {
            Ok(joined) => Ok(Self::from_url(joined)),
            Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!(
                "Invalid URL for join: {}",
                e
            ))),
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    /// Constructor with Python params
    pub fn new_impl(
        url: Option<&str>,
        scheme: Option<&str>,
        host: Option<&str>,
        port: Option<u16>,
        path: Option<&str>,
        query: Option<&[u8]>,
        fragment: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        params: Option<&Bound<'_, PyAny>>,
        netloc: Option<&[u8]>,
        raw_path: Option<&[u8]>,
    ) -> PyResult<Self> {
        // If URL string is provided, parse it
        if let Some(url_str) = url {
            if url_str.len() > MAX_URL_LENGTH {
                return Err(crate::exceptions::InvalidURL::new_err("URL too long"));
            }

            // Check for non-printable characters
            for (i, c) in url_str.chars().enumerate() {
                if c.is_control() && c != '\t' {
                    return Err(crate::exceptions::InvalidURL::new_err(format!(
                        "Invalid non-printable ASCII character in URL, {:?} at position {}.",
                        c, i
                    )));
                }
            }

            let parsed = Url::parse(url_str).or_else(|_| {
                // Try as relative URL
                Url::parse(&format!("http://example.com{}", url_str))
                    .map(|mut u| {
                        u.set_scheme("").ok();
                        u
                    })
                    .or_else(|_| {
                        // Handle scheme-relative URLs like "://example.com"
                        if url_str.starts_with("://") {
                            Url::parse(&format!("http{}", url_str)).map(|mut u| {
                                u.set_scheme("").ok();
                                u
                            })
                        } else {
                            Url::parse(&format!("relative:{}", url_str))
                        }
                    })
            });

            match parsed {
                Ok(mut parsed_url) => {
                    // Apply params if provided
                    if let Some(params_obj) = params {
                        let query_params = QueryParams::from_py(params_obj)?;
                        parsed_url.set_query(Some(&query_params.to_query_string()));
                    }

                    let frag = parsed_url.fragment().unwrap_or("").to_string();
                    return Ok(Self {
                        inner: parsed_url,
                        fragment: frag,
                    });
                }
                Err(e) => {
                    return Err(crate::exceptions::InvalidURL::new_err(format!(
                        "Invalid URL: {}",
                        e
                    )));
                }
            }
        }

        // Build URL from components
        let scheme = scheme.unwrap_or("http");
        let host = host.unwrap_or("");

        // Validate scheme
        if !scheme.is_empty() && !scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return Err(crate::exceptions::InvalidURL::new_err(
                "Invalid URL component 'scheme'",
            ));
        }

        let mut url_string = if host.is_empty() && scheme.is_empty() {
            String::new()
        } else {
            format!("{}://{}", scheme, host)
        };

        if let Some(p) = port {
            url_string.push_str(&format!(":{}", p));
        }

        let path = path.unwrap_or("/");

        // Validate path for absolute URLs
        if !host.is_empty() && !path.is_empty() && !path.starts_with('/') {
            return Err(crate::exceptions::InvalidURL::new_err(
                "For absolute URLs, path must be empty or begin with '/'",
            ));
        }

        // Validate path for relative URLs
        if host.is_empty() && scheme.is_empty() {
            if path.starts_with("//") {
                return Err(crate::exceptions::InvalidURL::new_err(
                    "Relative URLs cannot have a path starting with '//'",
                ));
            }
            if path.starts_with(':') {
                return Err(crate::exceptions::InvalidURL::new_err(
                    "Relative URLs cannot have a path starting with ':'",
                ));
            }
        }

        url_string.push_str(path);

        if let Some(q) = query {
            let q_str = String::from_utf8_lossy(q);
            if !q_str.is_empty() {
                url_string.push('?');
                url_string.push_str(&q_str);
            }
        }

        let frag = fragment.unwrap_or("").to_string();
        if !frag.is_empty() {
            url_string.push('#');
            url_string.push_str(&frag);
        }

        // Handle relative URLs
        if host.is_empty() && scheme.is_empty() {
            let dummy_base = Url::parse("relative://dummy").unwrap();
            match dummy_base.join(&url_string) {
                Ok(u) => Ok(Self {
                    inner: u,
                    fragment: frag,
                }),
                Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!(
                    "Invalid URL: {}",
                    e
                ))),
            }
        } else {
            match Url::parse(&url_string) {
                Ok(u) => Ok(Self {
                    inner: u,
                    fragment: frag,
                }),
                Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!(
                    "Invalid URL: {}",
                    e
                ))),
            }
        }
    }
}

#[pymethods]
impl URL {
    #[new]
    #[pyo3(signature = (url=None, *, scheme=None, host=None, port=None, path=None, query=None, fragment=None, username=None, password=None, params=None, netloc=None, raw_path=None))]
    fn py_new(
        url: Option<&str>,
        scheme: Option<&str>,
        host: Option<&str>,
        port: Option<u16>,
        path: Option<&str>,
        query: Option<&[u8]>,
        fragment: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        params: Option<&Bound<'_, PyAny>>,
        netloc: Option<&[u8]>,
        raw_path: Option<&[u8]>,
    ) -> PyResult<Self> {
        Self::new_impl(url, scheme, host, port, path, query, fragment, username, password, params, netloc, raw_path)
    }

    #[getter]
    fn scheme(&self) -> &str {
        let s = self.inner.scheme();
        if s == "relative" {
            ""
        } else {
            s
        }
    }

    #[getter]
    fn host(&self) -> String {
        self.inner.host_str().unwrap_or("").to_lowercase()
    }

    #[getter]
    fn port(&self) -> Option<u16> {
        self.inner.port()
    }

    #[getter]
    fn path(&self) -> &str {
        self.inner.path()
    }

    #[getter]
    fn query<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let q = self.inner.query().unwrap_or("");
        PyBytes::new(py, q.as_bytes())
    }

    #[getter]
    fn fragment(&self) -> &str {
        &self.fragment
    }

    #[getter]
    fn raw_path<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let path = self.inner.path();
        let query = self.inner.query();

        let raw = if let Some(q) = query {
            if q.is_empty() {
                format!("{}?", path)
            } else {
                format!("{}?{}", path, q)
            }
        } else {
            path.to_string()
        };

        PyBytes::new(py, raw.as_bytes())
    }

    #[getter]
    fn raw_host<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let host = self.inner.host_str().unwrap_or("");
        PyBytes::new(py, host.as_bytes())
    }

    #[getter]
    fn netloc<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let host = self.inner.host_str().unwrap_or("");
        let port = self.inner.port();

        let netloc = if let Some(p) = port {
            format!("{}:{}", host, p)
        } else {
            host.to_string()
        };

        // Add userinfo if present
        let userinfo = self.userinfo(py);
        let userinfo_bytes: &[u8] = userinfo.as_bytes();
        if !userinfo_bytes.is_empty() {
            let full = format!("{}@{}", String::from_utf8_lossy(userinfo_bytes), netloc);
            PyBytes::new(py, full.as_bytes())
        } else {
            PyBytes::new(py, netloc.as_bytes())
        }
    }

    #[getter]
    fn userinfo<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let username = self.inner.username();
        let password = self.inner.password().unwrap_or("");

        if username.is_empty() && password.is_empty() {
            PyBytes::new(py, b"")
        } else if password.is_empty() {
            PyBytes::new(py, username.as_bytes())
        } else {
            let userinfo = format!("{}:{}", username, password);
            PyBytes::new(py, userinfo.as_bytes())
        }
    }

    #[getter]
    fn username(&self) -> String {
        urlencoding::decode(self.inner.username())
            .unwrap_or_else(|_| self.inner.username().into())
            .into_owned()
    }

    #[getter]
    fn password(&self) -> Option<String> {
        self.inner.password().map(|p| {
            urlencoding::decode(p)
                .unwrap_or_else(|_| p.into())
                .into_owned()
        })
    }

    #[getter]
    fn params(&self) -> QueryParams {
        let query = self.inner.query().unwrap_or("");
        QueryParams::from_query_string(query)
    }

    fn join(&self, url: &str) -> PyResult<Self> {
        match self.inner.join(url) {
            Ok(joined) => Ok(Self::from_url(joined)),
            Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!(
                "Invalid URL for join: {}",
                e
            ))),
        }
    }

    #[pyo3(signature = (**kwargs))]
    fn copy_with(&self, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut new_url = self.clone();

        if let Some(kw) = kwargs {
            for (key, value) in kw.iter() {
                let key_str: String = key.extract()?;
                match key_str.as_str() {
                    "scheme" => {
                        let scheme: String = value.extract()?;
                        new_url.inner.set_scheme(&scheme).map_err(|_| {
                            crate::exceptions::InvalidURL::new_err("Invalid scheme")
                        })?;
                    }
                    "host" => {
                        let host: String = value.extract()?;
                        new_url.inner.set_host(Some(&host)).map_err(|e| {
                            crate::exceptions::InvalidURL::new_err(format!("Invalid host: {}", e))
                        })?;
                    }
                    "port" => {
                        let port: Option<u16> = value.extract()?;
                        new_url.inner.set_port(port).map_err(|_| {
                            crate::exceptions::InvalidURL::new_err("Invalid port")
                        })?;
                    }
                    "path" => {
                        let path: String = value.extract()?;
                        new_url.inner.set_path(&path);
                    }
                    "query" => {
                        let query: &[u8] = value.extract()?;
                        let q_str = String::from_utf8_lossy(query);
                        if q_str.is_empty() {
                            new_url.inner.set_query(None);
                        } else {
                            new_url.inner.set_query(Some(&q_str));
                        }
                    }
                    "raw_path" => {
                        let raw_path: &[u8] = value.extract()?;
                        let raw_str = String::from_utf8_lossy(raw_path);
                        if let Some(idx) = raw_str.find('?') {
                            let (path, query) = raw_str.split_at(idx);
                            new_url.inner.set_path(path);
                            let q = &query[1..]; // Skip the '?'
                            if q.is_empty() {
                                // Keep the trailing '?' indicator
                                new_url.inner.set_query(Some(""));
                            } else {
                                new_url.inner.set_query(Some(q));
                            }
                        } else {
                            new_url.inner.set_path(&raw_str);
                            new_url.inner.set_query(None);
                        }
                    }
                    "fragment" => {
                        let frag: String = value.extract()?;
                        new_url.fragment = frag.clone();
                        new_url.inner.set_fragment(if frag.is_empty() {
                            None
                        } else {
                            Some(&frag)
                        });
                    }
                    "netloc" => {
                        let netloc: &[u8] = value.extract()?;
                        let netloc_str = String::from_utf8_lossy(netloc);
                        // Parse netloc (may contain host:port)
                        if let Some(idx) = netloc_str.rfind(':') {
                            let (host, port_str) = netloc_str.split_at(idx);
                            let port_str = &port_str[1..];
                            if let Ok(port) = port_str.parse::<u16>() {
                                new_url.inner.set_host(Some(host)).map_err(|e| {
                                    crate::exceptions::InvalidURL::new_err(format!("Invalid host: {}", e))
                                })?;
                                new_url.inner.set_port(Some(port)).map_err(|_| {
                                    crate::exceptions::InvalidURL::new_err("Invalid port")
                                })?;
                            } else {
                                new_url.inner.set_host(Some(&netloc_str)).map_err(|e| {
                                    crate::exceptions::InvalidURL::new_err(format!("Invalid host: {}", e))
                                })?;
                            }
                        } else {
                            new_url.inner.set_host(Some(&netloc_str)).map_err(|e| {
                                crate::exceptions::InvalidURL::new_err(format!("Invalid host: {}", e))
                            })?;
                        }
                    }
                    "username" => {
                        let username: String = value.extract()?;
                        let encoded = urlencoding::encode(&username);
                        new_url.inner.set_username(&encoded).map_err(|_| {
                            crate::exceptions::InvalidURL::new_err("Cannot set username")
                        })?;
                    }
                    "password" => {
                        let password: String = value.extract()?;
                        let encoded = urlencoding::encode(&password);
                        new_url.inner.set_password(Some(&encoded)).map_err(|_| {
                            crate::exceptions::InvalidURL::new_err("Cannot set password")
                        })?;
                    }
                    other => {
                        return Err(PyTypeError::new_err(format!(
                            "'{}' is an invalid keyword argument for URL()",
                            other
                        )));
                    }
                }
            }
        }

        Ok(new_url)
    }

    fn copy_set_param(&self, key: &str, value: &str) -> Self {
        let mut params = self.params();
        params.set(key, value);
        let mut new_url = self.clone();
        new_url.inner.set_query(Some(&params.to_query_string()));
        new_url
    }

    fn copy_add_param(&self, key: &str, value: &str) -> Self {
        let mut params = self.params();
        params.add(key, value);
        let mut new_url = self.clone();
        new_url.inner.set_query(Some(&params.to_query_string()));
        new_url
    }

    fn copy_remove_param(&self, key: &str) -> Self {
        let mut params = self.params();
        params.remove(key);
        let mut new_url = self.clone();
        let qs = params.to_query_string();
        if qs.is_empty() {
            new_url.inner.set_query(None);
        } else {
            new_url.inner.set_query(Some(&qs));
        }
        new_url
    }

    fn copy_merge_params(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut params = self.params();
        let other_params = QueryParams::from_py(other)?;
        params.merge(&other_params);
        let mut new_url = self.clone();
        new_url.inner.set_query(Some(&params.to_query_string()));
        Ok(new_url)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("URL('{}')", self.inner)
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_url) = other.extract::<URL>() {
            Ok(self.inner.as_str() == other_url.inner.as_str())
        } else if let Ok(other_str) = other.extract::<String>() {
            Ok(self.inner.as_str() == other_str)
        } else {
            Ok(false)
        }
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.as_str().hash(&mut hasher);
        hasher.finish()
    }
}
