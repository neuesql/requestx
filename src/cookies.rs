//! Cookies implementation with domain and path support

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::HashMap;

use crate::exceptions::CookieConflict;

/// A single cookie with name, value, domain, and path
#[derive(Clone, Debug, PartialEq)]
struct Cookie {
    name: String,
    value: String,
    domain: String,
    path: String,
}

impl Cookie {
    fn new(name: &str, value: &str, domain: Option<&str>, path: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.unwrap_or("").to_string(),
            path: path.unwrap_or("/").to_string(),
        }
    }

    fn key(&self) -> (String, String, String) {
        (self.name.clone(), self.domain.clone(), self.path.clone())
    }
}

/// HTTP Cookies jar with domain and path support
#[pyclass(name = "Cookies")]
#[derive(Clone, Debug, Default)]
pub struct Cookies {
    cookies: Vec<Cookie>,
}

impl Cookies {
    pub fn new() -> Self {
        Self {
            cookies: Vec::new(),
        }
    }

    pub fn from_reqwest(jar: &reqwest::cookie::Jar, url: &url::Url) -> Self {
        let mut cookies = Self::new();
        // Note: reqwest's Jar doesn't expose cookies directly
        // We'll need to track cookies ourselves
        cookies
    }

    pub fn to_header_value(&self) -> String {
        self.cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn inner(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for cookie in &self.cookies {
            map.insert(cookie.name.clone(), cookie.value.clone());
        }
        map
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.set_with_domain_path(name, value, None, None);
    }

    fn set_with_domain_path(&mut self, name: &str, value: &str, domain: Option<&str>, path: Option<&str>) {
        let domain_str = domain.unwrap_or("");
        let path_str = path.unwrap_or("/");

        // Check if we already have a cookie with this name/domain/path
        if let Some(pos) = self.cookies.iter().position(|c| {
            c.name == name && c.domain == domain_str && c.path == path_str
        }) {
            self.cookies[pos].value = value.to_string();
        } else {
            self.cookies.push(Cookie::new(name, value, domain, path));
        }
    }

    fn get_with_domain(&self, name: &str, domain: Option<&str>) -> Option<&Cookie> {
        if let Some(d) = domain {
            self.cookies.iter().find(|c| c.name == name && c.domain == d)
        } else {
            // Find any cookie with this name
            self.cookies.iter().find(|c| c.name == name)
        }
    }

    fn count_matching(&self, name: &str) -> usize {
        self.cookies.iter().filter(|c| c.name == name).count()
    }

    /// Set the Cookie header on a request from this cookie jar
    pub fn set_cookie_header(&self, request: &mut crate::request::Request) {
        if !self.cookies.is_empty() {
            let header_value = self.to_header_value();
            request.headers_mut().set("Cookie".to_string(), header_value);
        }
    }
}

#[pymethods]
impl Cookies {
    #[new]
    #[pyo3(signature = (cookies=None))]
    fn py_new(py: Python<'_>, cookies: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut c = Self::new();

        if let Some(obj) = cookies {
            // Try dict
            if let Ok(dict) = obj.downcast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    c.set_with_domain_path(&k, &v, None, None);
                }
            }
            // Try Cookies object
            else if let Ok(other_cookies) = obj.extract::<Cookies>() {
                c.cookies = other_cookies.cookies;
            }
            // Try list of tuples
            else if let Ok(list) = obj.downcast::<PyList>() {
                for item in list.iter() {
                    if let Ok(tuple) = item.downcast::<PyTuple>() {
                        if tuple.len() >= 2 {
                            let k: String = tuple.get_item(0)?.extract()?;
                            let v: String = tuple.get_item(1)?.extract()?;
                            c.set_with_domain_path(&k, &v, None, None);
                        }
                    } else if let Ok(pair) = item.extract::<(String, String)>() {
                        c.set_with_domain_path(&pair.0, &pair.1, None, None);
                    }
                }
            }
            // Try http.cookiejar.CookieJar or other iterable
            else if obj.hasattr("__iter__")? {
                // CookieJar is iterable
                let iter = obj.call_method0("__iter__")?;
                loop {
                    match iter.call_method0("__next__") {
                        Ok(cookie) => {
                            if cookie.hasattr("name")? && cookie.hasattr("value")? {
                                let name: String = cookie.getattr("name")?.extract()?;
                                let value: String = cookie.getattr("value")?.extract()?;
                                let domain: Option<String> = cookie.getattr("domain").ok().and_then(|d| d.extract::<String>().ok());
                                let path: Option<String> = cookie.getattr("path").ok().and_then(|p| p.extract::<String>().ok());
                                c.set_with_domain_path(&name, &value, domain.as_deref(), path.as_deref());
                            }
                        }
                        Err(_) => break, // StopIteration
                    }
                }
            }
        }

        Ok(c)
    }

    #[pyo3(signature = (name, default=None, domain=None))]
    fn get(&self, name: &str, default: Option<&str>, domain: Option<&str>) -> Option<String> {
        self.get_with_domain(name, domain)
            .map(|c| c.value.clone())
            .or_else(|| default.map(|s| s.to_string()))
    }

    #[pyo3(signature = (name, value, domain=None, path=None))]
    fn set_cookie(&mut self, name: &str, value: &str, domain: Option<&str>, path: Option<&str>) {
        self.set_with_domain_path(name, value, domain, path);
    }

    #[pyo3(signature = (name, value, domain=None, path=None))]
    #[pyo3(name = "set")]
    fn py_set(&mut self, name: &str, value: &str, domain: Option<&str>, path: Option<&str>) {
        self.set_with_domain_path(name, value, domain, path);
    }

    #[pyo3(signature = (name, domain=None, path=None))]
    fn delete(&mut self, name: &str, domain: Option<&str>, path: Option<&str>) {
        if let (Some(d), Some(p)) = (domain, path) {
            self.cookies.retain(|c| !(c.name == name && c.domain == d && c.path == p));
        } else if let Some(d) = domain {
            self.cookies.retain(|c| !(c.name == name && c.domain == d));
        } else {
            self.cookies.retain(|c| c.name != name);
        }
    }

    #[pyo3(signature = (domain=None, path=None))]
    fn clear(&mut self, domain: Option<&str>, path: Option<&str>) {
        if let (Some(d), Some(p)) = (domain, path) {
            self.cookies.retain(|c| !(c.domain == d && c.path == p));
        } else if let Some(d) = domain {
            self.cookies.retain(|c| c.domain != d);
        } else {
            self.cookies.clear();
        }
    }

    fn keys(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.cookies
            .iter()
            .filter_map(|c| {
                if seen.insert(c.name.clone()) {
                    Some(c.name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn values(&self) -> Vec<String> {
        // Return values for unique names (first occurrence)
        let mut seen = std::collections::HashSet::new();
        self.cookies
            .iter()
            .filter_map(|c| {
                if seen.insert(c.name.clone()) {
                    Some(c.value.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn items(&self) -> Vec<(String, String)> {
        // Return items for unique names (first occurrence)
        let mut seen = std::collections::HashSet::new();
        self.cookies
            .iter()
            .filter_map(|c| {
                if seen.insert(c.name.clone()) {
                    Some((c.name.clone(), c.value.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn __getitem__(&self, name: &str) -> PyResult<String> {
        let matching: Vec<_> = self.cookies.iter().filter(|c| c.name == name).collect();

        if matching.is_empty() {
            Err(PyKeyError::new_err(name.to_string()))
        } else if matching.len() > 1 {
            // Check if all matching cookies have different domains
            let domains: std::collections::HashSet<_> = matching.iter().map(|c| &c.domain).collect();
            if domains.len() > 1 {
                Err(CookieConflict::new_err(format!(
                    "Cookies with name '{}' exist for multiple domains",
                    name
                )))
            } else {
                Ok(matching[0].value.clone())
            }
        } else {
            Ok(matching[0].value.clone())
        }
    }

    fn __setitem__(&mut self, name: String, value: String) {
        self.set_with_domain_path(&name, &value, None, None);
    }

    fn __delitem__(&mut self, name: &str) -> PyResult<()> {
        let count = self.cookies.iter().filter(|c| c.name == name).count();
        if count > 0 {
            self.cookies.retain(|c| c.name != name);
            Ok(())
        } else {
            Err(PyKeyError::new_err(name.to_string()))
        }
    }

    fn __contains__(&self, name: &str) -> bool {
        self.cookies.iter().any(|c| c.name == name)
    }

    fn __iter__(&self) -> CookiesIterator {
        CookiesIterator {
            keys: self.keys(),
            index: 0,
        }
    }

    fn __len__(&self) -> usize {
        self.cookies.len()
    }

    fn __bool__(&self) -> bool {
        !self.cookies.is_empty()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_cookies) = other.extract::<Cookies>() {
            // Compare by name/value only for compatibility
            let self_map = self.inner();
            let other_map = other_cookies.inner();
            Ok(self_map == other_map)
        } else if let Ok(dict) = other.downcast::<PyDict>() {
            let self_map = self.inner();
            let mut other_map = HashMap::new();
            for (k, v) in dict.iter() {
                let key: String = k.extract()?;
                let value: String = v.extract()?;
                other_map.insert(key, value);
            }
            Ok(self_map == other_map)
        } else {
            Ok(false)
        }
    }

    fn __repr__(&self) -> String {
        let items: Vec<String> = self
            .cookies
            .iter()
            .map(|c| {
                let domain = if c.domain.is_empty() {
                    "/".to_string()
                } else {
                    format!("{} /", c.domain)
                };
                format!("<Cookie {}={} for {}>", c.name, c.value, domain)
            })
            .collect();
        format!("<Cookies[{}]>", items.join(", "))
    }

    fn update(&mut self, other: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(dict) = other.downcast::<PyDict>() {
            for (key, value) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                self.set_with_domain_path(&k, &v, None, None);
            }
        } else if let Ok(cookies) = other.extract::<Cookies>() {
            for cookie in cookies.cookies {
                self.set_with_domain_path(
                    &cookie.name,
                    &cookie.value,
                    Some(&cookie.domain),
                    Some(&cookie.path),
                );
            }
        }
        Ok(())
    }

    /// Extract cookies from a response
    fn extract_cookies(&mut self, response: &Bound<'_, PyAny>) -> PyResult<()> {
        // Get headers from response
        let headers = response.getattr("headers")?;

        // Get the request URL for the domain
        let request = response.getattr("request")?;
        let url = request.getattr("url")?;
        let host: String = if url.hasattr("host")? {
            url.getattr("host")?.extract().unwrap_or_default()
        } else {
            String::new()
        };

        // Iterate through all Set-Cookie headers
        if headers.hasattr("get_list")? {
            let cookie_headers: Vec<String> = headers.call_method1("get_list", ("set-cookie",))?.extract()?;
            for cookie_str in cookie_headers {
                self.parse_set_cookie(&cookie_str, &host);
            }
        } else if headers.hasattr("multi_items")? {
            // Fall back to multi_items for Headers
            let multi_items: Vec<(String, String)> = headers.call_method0("multi_items")?.extract()?;
            for (name, value) in multi_items {
                if name.to_lowercase() == "set-cookie" {
                    self.parse_set_cookie(&value, &host);
                }
            }
        }

        Ok(())
    }
}

impl Cookies {
    fn parse_set_cookie(&mut self, cookie_str: &str, default_domain: &str) {
        // Parse a Set-Cookie header
        // Format: name=value; attr1=val1; attr2=val2; ...
        let mut parts = cookie_str.splitn(2, ';');
        if let Some(name_value) = parts.next() {
            if let Some((name, value)) = name_value.split_once('=') {
                let name = name.trim().to_string();
                let value = value.trim().to_string();

                // Parse attributes
                let mut domain = String::new();
                let mut path = "/".to_string();

                if let Some(attrs) = parts.next() {
                    for attr in attrs.split(';') {
                        let attr = attr.trim();
                        if let Some((key, val)) = attr.split_once('=') {
                            let key_lower = key.trim().to_lowercase();
                            let val = val.trim();
                            match key_lower.as_str() {
                                "domain" => {
                                    domain = val.strip_prefix('.').unwrap_or(val).to_string();
                                }
                                "path" => {
                                    path = val.to_string();
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Use default domain if not specified
                if domain.is_empty() {
                    domain = default_domain.to_string();
                }

                self.set_with_domain_path(&name, &value, Some(&domain), Some(&path));
            }
        }
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
