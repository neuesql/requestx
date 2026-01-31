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
    /// Track if the original URL had an explicit trailing slash for root path
    has_trailing_slash: bool,
    /// Track if the URL has an empty scheme (like "://example.com")
    empty_scheme: bool,
    /// Track if the URL has an empty host (like "http://")
    empty_host: bool,
}

impl URL {
    pub fn from_url(url: Url) -> Self {
        let fragment = url.fragment().unwrap_or("").to_string();
        // Default to true since url crate always normalizes to have slash
        Self { inner: url, fragment, has_trailing_slash: true, empty_scheme: false, empty_host: false }
    }

    pub fn from_url_with_slash(url: Url, has_trailing_slash: bool) -> Self {
        let fragment = url.fragment().unwrap_or("").to_string();
        Self { inner: url, fragment, has_trailing_slash, empty_scheme: false, empty_host: false }
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

    /// Convert to string (preserving trailing slash based on original input)
    pub fn to_string(&self) -> String {
        let s = self.inner.to_string();
        // Only strip trailing slash if:
        // 1. The URL ends with /
        // 2. The path is exactly "/" (root path)
        // 3. There's no query or fragment
        // 4. The original URL did NOT have a trailing slash
        if s.ends_with('/')
            && self.inner.path() == "/"
            && self.inner.query().is_none()
            && self.inner.fragment().is_none()
            && !self.has_trailing_slash
        {
            s[..s.len() - 1].to_string()
        } else {
            s
        }
    }

    /// Convert to string with trailing slash (raw representation)
    pub fn to_string_raw(&self) -> String {
        self.inner.to_string()
    }

    /// Get the host (public Rust API)
    pub fn get_host(&self) -> Option<String> {
        self.inner.host_str().map(|s| {
            // Strip brackets for IPv6 addresses
            let host = if s.starts_with('[') && s.ends_with(']') {
                &s[1..s.len()-1]
            } else {
                s
            };
            host.to_lowercase()
        })
    }

    /// Get the scheme (public Rust API)
    pub fn get_scheme(&self) -> String {
        let s = self.inner.scheme();
        if s == "relative" {
            String::new()
        } else {
            s.to_string()
        }
    }

    /// Get the host as string (public Rust API)
    pub fn get_host_str(&self) -> String {
        let host = self.inner.host_str().unwrap_or("");
        // Strip brackets for IPv6 addresses
        let host = if host.starts_with('[') && host.ends_with(']') {
            &host[1..host.len()-1]
        } else {
            host
        };
        host.to_lowercase()
    }

    /// Get the port (public Rust API)
    pub fn get_port(&self) -> Option<u16> {
        self.inner.port()
    }

    /// Get the username (public Rust API)
    pub fn get_username(&self) -> String {
        urlencoding::decode(self.inner.username())
            .unwrap_or_else(|_| self.inner.username().into())
            .into_owned()
    }

    /// Get the password (public Rust API)
    pub fn get_password(&self) -> Option<String> {
        self.inner.password().map(|p| {
            urlencoding::decode(p)
                .unwrap_or_else(|_| p.into())
                .into_owned()
        })
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

            // Check for invalid port before parsing
            // Look for pattern like :abc/ or :abc? or :abc# or :abc at end
            if let Some(authority_start) = url_str.find("://") {
                let after_scheme = &url_str[authority_start + 3..];
                // Find the end of authority (first / ? or #, or end of string)
                let authority_end = after_scheme.find('/').unwrap_or(after_scheme.len());
                let authority_end = authority_end.min(after_scheme.find('?').unwrap_or(after_scheme.len()));
                let authority_end = authority_end.min(after_scheme.find('#').unwrap_or(after_scheme.len()));
                let authority = &after_scheme[..authority_end];

                // Check for port in authority (after last : that's not part of IPv6)
                if !authority.starts_with('[') {  // Not IPv6
                    if let Some(colon_pos) = authority.rfind(':') {
                        // Check if there's an @ (userinfo) after this colon
                        let after_colon = &authority[colon_pos + 1..];
                        if !after_colon.contains('@') {
                            // This should be a port
                            if !after_colon.is_empty() && !after_colon.chars().all(|c| c.is_ascii_digit()) {
                                return Err(crate::exceptions::InvalidURL::new_err(format!(
                                    "Invalid port: '{}'", after_colon
                                )));
                            }
                        }
                    }
                }
            }

            // Handle special cases that the url crate doesn't support well

            // Case 1: Empty scheme like "://example.com"
            if url_str.starts_with("://") {
                let rest = &url_str[3..];  // Remove "://"
                // Parse the rest as if it had http scheme, then mark as empty scheme
                let temp_url = format!("http://{}", rest);
                match Url::parse(&temp_url) {
                    Ok(mut parsed_url) => {
                        // Apply params if provided
                        if let Some(params_obj) = params {
                            let query_params = QueryParams::from_py(params_obj)?;
                            parsed_url.set_query(Some(&query_params.to_query_string()));
                        }
                        let has_trailing_slash = url_str.split('?').next().unwrap_or(url_str)
                            .split('#').next().unwrap_or(url_str).ends_with('/');
                        let frag = parsed_url.fragment().unwrap_or("").to_string();
                        return Ok(Self {
                            inner: parsed_url,
                            fragment: frag,
                            has_trailing_slash,
                            empty_scheme: true,  // Mark as empty scheme
                            empty_host: false,
                        });
                    }
                    Err(e) => {
                        return Err(crate::exceptions::InvalidURL::new_err(format!(
                            "Invalid URL: {}", e
                        )));
                    }
                }
            }

            // Case 2: Scheme with empty authority like "http://"
            if url_str.ends_with("://") || (url_str.contains("://") && {
                let after = url_str.split("://").nth(1).unwrap_or("");
                after.is_empty() || after == "/"
            }) {
                // Extract the scheme
                let scheme_end = url_str.find("://").unwrap();
                let scheme = &url_str[..scheme_end];
                let rest = &url_str[scheme_end + 3..];
                // Build a URL with dummy host
                let temp_url = format!("{}://placeholder.invalid/{}", scheme, rest.trim_start_matches('/'));
                match Url::parse(&temp_url) {
                    Ok(mut parsed_url) => {
                        // Apply params if provided
                        if let Some(params_obj) = params {
                            let query_params = QueryParams::from_py(params_obj)?;
                            parsed_url.set_query(Some(&query_params.to_query_string()));
                        }
                        let has_trailing_slash = rest.ends_with('/') || rest.is_empty();
                        let frag = parsed_url.fragment().unwrap_or("").to_string();
                        return Ok(Self {
                            inner: parsed_url,
                            fragment: frag,
                            has_trailing_slash,
                            empty_scheme: false,
                            empty_host: true,  // Mark as empty host
                        });
                    }
                    Err(_) => {
                        // Fallback: create minimal URL
                        let base = format!("{}://placeholder.invalid/", scheme);
                        if let Ok(parsed_url) = Url::parse(&base) {
                            return Ok(Self {
                                inner: parsed_url,
                                fragment: String::new(),
                                has_trailing_slash: true,
                                empty_scheme: false,
                                empty_host: true,
                            });
                        }
                    }
                }
            }

            // Normal URL parsing
            let parsed = Url::parse(url_str).or_else(|_| {
                // Try as relative URL with a base
                if !url_str.contains("://") {
                    // This is a relative URL
                    Url::parse(&format!("relative:{}", url_str))
                } else {
                    Err(url::ParseError::InvalidDomainCharacter)
                }
            });

            match parsed {
                Ok(mut parsed_url) => {
                    // Apply params if provided
                    if let Some(params_obj) = params {
                        let query_params = QueryParams::from_py(params_obj)?;
                        parsed_url.set_query(Some(&query_params.to_query_string()));
                    }

                    // Track if original URL had a trailing slash
                    // For root paths, check if original ended with /
                    let has_trailing_slash = if parsed_url.path() == "/" {
                        // Check if original string ended with / (before query/fragment)
                        let base = url_str.split('?').next().unwrap_or(url_str);
                        let base = base.split('#').next().unwrap_or(base);
                        base.ends_with('/')
                    } else {
                        // For non-root paths, preserve as-is
                        true
                    };

                    let frag = parsed_url.fragment().unwrap_or("").to_string();
                    return Ok(Self {
                        inner: parsed_url,
                        fragment: frag,
                        has_trailing_slash,
                        empty_scheme: false,
                        empty_host: false,
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
                Ok(u) => {
                    let has_slash = u.path() != "/" || url_string.ends_with('/');
                    Ok(Self {
                        inner: u,
                        fragment: frag,
                        has_trailing_slash: has_slash,
                        empty_scheme: false,
                        empty_host: false,
                    })
                }
                Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!(
                    "Invalid URL: {}",
                    e
                ))),
            }
        } else {
            match Url::parse(&url_string) {
                Ok(u) => {
                    let has_slash = u.path() != "/" || url_string.ends_with('/');
                    Ok(Self {
                        inner: u,
                        fragment: frag,
                        has_trailing_slash: has_slash,
                        empty_scheme: false,
                        empty_host: false,
                    })
                }
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
        if self.empty_scheme {
            return "";
        }
        let s = self.inner.scheme();
        if s == "relative" {
            ""
        } else {
            s
        }
    }

    #[getter]
    fn host(&self) -> String {
        if self.empty_host {
            return String::new();
        }
        let host = self.inner.host_str().unwrap_or("");
        // Strip brackets for IPv6 addresses - httpx returns host without brackets
        let host = if host.starts_with('[') && host.ends_with(']') {
            &host[1..host.len()-1]
        } else {
            host
        };
        host.to_lowercase()
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
        // Strip brackets for IPv6 addresses - httpcore expects host without brackets
        let host = if host.starts_with('[') && host.ends_with(']') {
            &host[1..host.len()-1]
        } else {
            host
        };
        PyBytes::new(py, host.as_bytes())
    }

    #[getter]
    fn raw_scheme<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let scheme = self.inner.scheme();
        if scheme == "relative" {
            PyBytes::new(py, b"")
        } else {
            PyBytes::new(py, scheme.as_bytes())
        }
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
                        // Handle port - allow large values in URL (will fail at connection time)
                        if value.is_none() {
                            new_url.inner.set_port(None).map_err(|_| {
                                crate::exceptions::InvalidURL::new_err("Invalid port")
                            })?;
                        } else {
                            let port_value: i64 = value.extract()?;
                            // Store as u16 by taking modulo - the connection will fail if truly invalid
                            // This matches httpx behavior which allows "impossible" ports in URLs
                            if port_value < 0 {
                                return Err(crate::exceptions::InvalidURL::new_err(
                                    "Invalid port: negative values not allowed"
                                ));
                            }
                            // Convert large port numbers by truncating to u16 range
                            // The URL will be invalid for actual connections
                            let port_u16 = (port_value % 65536) as u16;
                            new_url.inner.set_port(Some(port_u16)).map_err(|_| {
                                crate::exceptions::InvalidURL::new_err("Invalid port")
                            })?;
                        }
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
        self.to_string()
    }

    fn __repr__(&self) -> String {
        // Mask password in repr for security
        if self.inner.password().is_some() {
            // Build URL string with [secure] instead of actual password
            let mut url_str = String::new();
            url_str.push_str(self.inner.scheme());
            url_str.push_str("://");

            let username = self.inner.username();
            if !username.is_empty() {
                url_str.push_str(username);
                url_str.push_str(":[secure]@");
            }

            if let Some(host) = self.inner.host_str() {
                url_str.push_str(host);
            }

            if let Some(port) = self.inner.port() {
                url_str.push_str(&format!(":{}", port));
            }

            url_str.push_str(self.inner.path());

            if let Some(query) = self.inner.query() {
                url_str.push('?');
                url_str.push_str(query);
            }

            if let Some(fragment) = self.inner.fragment() {
                url_str.push('#');
                url_str.push_str(fragment);
            }

            format!("URL('{}')", url_str)
        } else {
            format!("URL('{}')", self.inner)
        }
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_url) = other.extract::<URL>() {
            // Compare internal URLs (both normalized)
            Ok(self.inner.as_str() == other_url.inner.as_str())
        } else if let Ok(other_str) = other.extract::<String>() {
            // For string comparison, try both with and without trailing slash
            // to match user expectations
            let self_str = self.inner.to_string();
            if self_str == other_str {
                return Ok(true);
            }
            // Also compare after normalizing both (strip or add trailing slash)
            let self_normalized = self.to_string();
            let other_normalized = other_str.trim_end_matches('/');
            if self_normalized == other_normalized || self_normalized == other_str {
                return Ok(true);
            }
            // Final check: if other has trailing slash, check against inner
            if other_str.ends_with('/') && self_str == other_str {
                return Ok(true);
            }
            Ok(false)
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
