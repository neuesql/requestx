//! Cookies implementation with proper domain/path support (httpx-compatible)

use crate::exceptions::CookieConflict;
use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

/// Internal cookie entry storing name, value, domain, and path
#[derive(Clone, Debug, PartialEq, Eq)]
struct CookieEntry {
    name: String,
    value: String,
    domain: String,
    path: String,
}

/// HTTP Cookies jar with domain/path support
#[pyclass(name = "Cookies")]
#[derive(Clone, Debug, Default)]
pub struct Cookies {
    entries: Vec<CookieEntry>,
}

impl Cookies {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn from_reqwest(_jar: &reqwest::cookie::Jar, _url: &url::Url) -> Self {
        // Note: reqwest's Jar doesn't expose cookies directly
        // We'll need to track cookies ourselves
        Self::new()
    }

    pub fn to_header_value(&self) -> String {
        self.entries
            .iter()
            .map(|e| format!("{}={}", e.name, e.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn inner(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        for entry in &self.entries {
            map.insert(entry.name.clone(), entry.value.clone());
        }
        map
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.set_with_domain_path(name, value, "", "/");
    }

    fn set_with_domain_path(&mut self, name: &str, value: &str, domain: &str, path: &str) {
        // Find and update existing cookie with same name, domain, path
        for entry in &mut self.entries {
            if entry.name == name && entry.domain == domain && entry.path == path {
                entry.value = value.to_string();
                return;
            }
        }
        // Add new entry
        self.entries.push(CookieEntry {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.to_string(),
            path: path.to_string(),
        });
    }

    /// Find cookies matching name with optional domain/path filter
    fn find_cookies(&self, name: &str, domain: Option<&str>, path: Option<&str>) -> Vec<&CookieEntry> {
        self.entries
            .iter()
            .filter(|e| {
                if e.name != name {
                    return false;
                }
                if let Some(d) = domain {
                    if e.domain != d {
                        return false;
                    }
                }
                if let Some(p) = path {
                    if e.path != p {
                        return false;
                    }
                }
                true
            })
            .collect()
    }
}

#[pymethods]
impl Cookies {
    #[new]
    #[pyo3(signature = (cookies=None))]
    fn py_new(cookies: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let mut c = Self::new();

        if let Some(obj) = cookies {
            // Try to extract as our own Cookies type first
            if let Ok(other_cookies) = obj.extract::<Cookies>() {
                c.entries = other_cookies.entries;
                return Ok(c);
            }

            // Handle dict
            if let Ok(dict) = obj.downcast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let k: String = key.extract()?;
                    let v: String = value.extract()?;
                    c.set_with_domain_path(&k, &v, "", "/");
                }
                return Ok(c);
            }

            // Handle list of tuples
            if let Ok(list) = obj.downcast::<PyList>() {
                for item in list.iter() {
                    let tuple = item.downcast::<PyTuple>()?;
                    let k: String = tuple.get_item(0)?.extract()?;
                    let v: String = tuple.get_item(1)?.extract()?;
                    c.set_with_domain_path(&k, &v, "", "/");
                }
                return Ok(c);
            }

            // Check if it's a CookieJar from http.cookiejar (iterable with Cookie objects)
            if let Ok(py_iter) = obj.try_iter() {
                // Try to iterate over CookieJar (Python http.cookiejar.CookieJar)
                let mut handled_as_jar = false;
                for item_result in py_iter {
                    let item: Bound<'_, PyAny> = item_result?;
                    // Check if item has 'name', 'value', 'domain', 'path' attributes (Cookie object)
                    if let (Ok(name), Ok(value)) = (item.getattr("name"), item.getattr("value")) {
                        handled_as_jar = true;
                        let name_str: String = name.extract()?;
                        let value_str: String = value.extract()?;
                        let domain_str: String = item
                            .getattr("domain")
                            .and_then(|d| d.extract::<String>())
                            .unwrap_or_default();
                        let path_str: String = item
                            .getattr("path")
                            .and_then(|p| p.extract::<String>())
                            .unwrap_or_else(|_| "/".to_string());
                        c.set_with_domain_path(&name_str, &value_str, &domain_str, &path_str);
                    } else {
                        // Not a Cookie object, this might be a different iterable
                        break;
                    }
                }
                if handled_as_jar && !c.entries.is_empty() {
                    return Ok(c);
                }
            }
        }

        Ok(c)
    }

    #[pyo3(signature = (name, default=None, domain=None, path=None))]
    fn get(&self, name: &str, default: Option<&str>, domain: Option<&str>, path: Option<&str>) -> PyResult<Option<String>> {
        let matches = self.find_cookies(name, domain, path);
        match matches.len() {
            0 => Ok(default.map(|s| s.to_string())),
            1 => Ok(Some(matches[0].value.clone())),
            _ => {
                // Multiple matches without domain/path filter - error
                if domain.is_none() && path.is_none() {
                    Err(CookieConflict::new_err(format!("Multiple cookies with name '{}' exist for different domains/paths", name)))
                } else {
                    // With filters, just return first match
                    Ok(Some(matches[0].value.clone()))
                }
            }
        }
    }

    #[pyo3(name = "set", signature = (name, value, domain=None, path=None))]
    fn set_py(&mut self, name: &str, value: &str, domain: Option<&str>, path: Option<&str>) {
        let domain = domain.unwrap_or("");
        let path = path.unwrap_or("/");
        self.set_with_domain_path(name, value, domain, path);
    }

    #[pyo3(signature = (name, domain=None, path=None))]
    fn delete(&mut self, name: &str, domain: Option<&str>, path: Option<&str>) {
        self.entries.retain(|e| {
            if e.name != name {
                return true;
            }
            if let Some(d) = domain {
                if e.domain != d {
                    return true;
                }
            }
            if let Some(p) = path {
                if e.path != p {
                    return true;
                }
            }
            false
        });
    }

    #[pyo3(signature = (domain=None, path=None))]
    fn clear(&mut self, domain: Option<&str>, path: Option<&str>) {
        if domain.is_none() && path.is_none() {
            self.entries.clear();
        } else {
            self.entries.retain(|e| {
                if let Some(d) = domain {
                    if e.domain != d {
                        return true;
                    }
                }
                if let Some(p) = path {
                    if e.path != p {
                        return true;
                    }
                }
                // Matches domain/path criteria - remove it
                false
            });
        }
    }

    fn keys(&self) -> Vec<String> {
        // Return unique names
        let mut seen = std::collections::HashSet::new();
        self.entries
            .iter()
            .filter_map(|e| {
                if seen.insert(e.name.clone()) {
                    Some(e.name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn values(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.value.clone()).collect()
    }

    fn items(&self) -> Vec<(String, String)> {
        self.entries
            .iter()
            .map(|e| (e.name.clone(), e.value.clone()))
            .collect()
    }

    fn __getitem__(&self, name: &str) -> PyResult<String> {
        let matches: Vec<_> = self.entries.iter().filter(|e| e.name == name).collect();
        match matches.len() {
            0 => Err(PyKeyError::new_err(name.to_string())),
            1 => Ok(matches[0].value.clone()),
            _ => Err(CookieConflict::new_err(format!("Multiple cookies with name '{}' exist for different domains/paths", name))),
        }
    }

    fn __setitem__(&mut self, name: String, value: String) {
        // Set without domain/path (defaults)
        self.set_with_domain_path(&name, &value, "", "/");
    }

    fn __delitem__(&mut self, name: &str) -> PyResult<()> {
        let before_len = self.entries.len();
        self.entries.retain(|e| e.name != name);
        if self.entries.len() < before_len {
            Ok(())
        } else {
            Err(PyKeyError::new_err(name.to_string()))
        }
    }

    fn __contains__(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name == name)
    }

    fn __iter__(&self) -> CookiesIterator {
        CookiesIterator::new(self.keys())
    }

    fn __len__(&self) -> usize {
        self.entries.len()
    }

    fn __bool__(&self) -> bool {
        !self.entries.is_empty()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_cookies) = other.extract::<Cookies>() {
            // Compare entries - order might differ
            if self.entries.len() != other_cookies.entries.len() {
                return Ok(false);
            }
            // Check all entries exist in other
            for entry in &self.entries {
                if !other_cookies.entries.iter().any(|e| e == entry) {
                    return Ok(false);
                }
            }
            Ok(true)
        } else if let Ok(dict) = other.downcast::<PyDict>() {
            // Compare as simple name->value dict (ignoring domain/path)
            let self_map = self.inner();
            let mut other_map = std::collections::HashMap::new();
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
            .entries
            .iter()
            .map(|e| {
                let domain_display = if e.domain.is_empty() {
                    String::new()
                } else {
                    format!("{} ", e.domain)
                };
                format!("<Cookie {}={} for {}/>", e.name, e.value, domain_display)
            })
            .collect();
        format!("<Cookies[{}]>", items.join(", "))
    }

    fn update(&mut self, other: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(dict) = other.downcast::<PyDict>() {
            for (key, value) in dict.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                self.set_with_domain_path(&k, &v, "", "/");
            }
        } else if let Ok(cookies) = other.extract::<Cookies>() {
            for entry in cookies.entries {
                self.set_with_domain_path(&entry.name, &entry.value, &entry.domain, &entry.path);
            }
        }
        Ok(())
    }

    /// Get the jar property (returns CookieJar for iteration over Cookie objects)
    #[getter]
    fn jar(&self) -> CookieJar {
        let cookies = self
            .entries
            .iter()
            .map(|e| Cookie {
                name: e.name.clone(),
                value: e.value.clone(),
                domain: e.domain.clone(),
                path: e.path.clone(),
            })
            .collect();
        CookieJar { cookies }
    }

    /// Extract cookies from a response (httpx compatibility)
    fn extract_cookies(&mut self, response: &Bound<'_, PyAny>) -> PyResult<()> {
        // Get headers from response
        let headers = response.getattr("headers")?;

        // Get request URL for domain defaulting
        let request = response.getattr("request")?;
        let url = request.getattr("url")?;
        let host: String = url
            .getattr("host")
            .and_then(|h| h.extract::<String>())
            .unwrap_or_default();

        // Get all Set-Cookie headers
        let set_cookie_headers: Vec<String> = if let Ok(multi_items) = headers.call_method0("multi_items") {
            let mut cookies = Vec::new();
            if let Ok(py_iter) = multi_items.try_iter() {
                for item_result in py_iter {
                    let item: Bound<'_, PyAny> = item_result?;
                    let tuple = item.downcast::<PyTuple>()?;
                    let key: String = tuple.get_item(0)?.extract()?;
                    if key.to_lowercase() == "set-cookie" {
                        let value: String = tuple.get_item(1)?.extract()?;
                        cookies.push(value);
                    }
                }
            }
            cookies
        } else if let Ok(get_list) = headers.call_method1("get_list", ("set-cookie",)) {
            get_list.extract()?
        } else if let Ok(single) = headers.call_method1("get", ("set-cookie",)) {
            if !single.is_none() {
                vec![single.extract()?]
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        // Parse each Set-Cookie header
        for cookie_str in set_cookie_headers {
            self.do_parse_set_cookie(&cookie_str, &host);
        }

        Ok(())
    }

    /// Parse a Set-Cookie header string (internal)
    fn do_parse_set_cookie(&mut self, cookie_str: &str, default_domain: &str) {
        let parts: Vec<&str> = cookie_str.split(';').collect();
        if parts.is_empty() {
            return;
        }

        // First part is name=value
        let name_value = parts[0].trim();
        let (name, value) = if let Some(eq_pos) = name_value.find('=') {
            let n = name_value[..eq_pos].trim();
            let v = name_value[eq_pos + 1..].trim();
            (n.to_string(), v.to_string())
        } else {
            return;
        };

        // Parse attributes
        let mut domain = default_domain.to_string();
        let mut path = "/".to_string();

        for part in parts.iter().skip(1) {
            let part = part.trim();
            let (attr_name, attr_value) = if let Some(eq_pos) = part.find('=') {
                (part[..eq_pos].trim().to_lowercase(), part[eq_pos + 1..].trim().to_string())
            } else {
                (part.to_lowercase(), String::new())
            };

            match attr_name.as_str() {
                "domain" => {
                    // Remove leading dot if present
                    domain = attr_value.trim_start_matches('.').to_string();
                }
                "path" => {
                    path = attr_value;
                }
                _ => {}
            }
        }

        self.set_with_domain_path(&name, &value, &domain, &path);
    }
}

/// A single Cookie object (for jar iteration)
#[pyclass(name = "Cookie")]
#[derive(Clone)]
pub struct Cookie {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    value: String,
    #[pyo3(get)]
    domain: String,
    #[pyo3(get)]
    path: String,
}

#[pymethods]
impl Cookie {
    fn __repr__(&self) -> String {
        let domain_display = if self.domain.is_empty() {
            String::new()
        } else {
            format!("{} ", self.domain)
        };
        format!("<Cookie {}={} for {}/>", self.name, self.value, domain_display)
    }
}

/// Cookie jar that holds Cookie objects
#[pyclass(name = "CookieJar")]
pub struct CookieJar {
    cookies: Vec<Cookie>,
}

#[pymethods]
impl CookieJar {
    fn __iter__(&self) -> CookieJarIterator {
        CookieJarIterator::new(self.cookies.clone())
    }

    fn __len__(&self) -> usize {
        self.cookies.len()
    }
}

crate::common::impl_py_iterator!(CookieJarIterator, Cookie, cookies, "CookieJarIterator");
crate::common::impl_py_iterator!(CookiesIterator, String, keys, "CookiesIterator");

/// Parse a Set-Cookie header string and return (name, value, is_expired).
/// Returns None if the header is malformed (no name=value).
#[pyfunction]
pub fn parse_set_cookie(cookie_str: &str) -> Option<(String, String, bool)> {
    let parts: Vec<&str> = cookie_str.split(';').collect();
    if parts.is_empty() {
        return None;
    }

    // First part is name=value
    let name_value = parts[0].trim();
    let eq_pos = name_value.find('=')?;
    let name = name_value[..eq_pos].trim().to_string();
    let value = name_value[eq_pos + 1..].trim().to_string();

    if name.is_empty() {
        return None;
    }

    // Check for expires attribute
    let mut is_expired = false;
    for part in parts.iter().skip(1) {
        let part = part.trim();
        if let Some(eq_pos) = part.find('=') {
            let attr_name = part[..eq_pos].trim().to_lowercase();
            if attr_name == "expires" {
                let expires_str = part[eq_pos + 1..].trim();
                is_expired = is_cookie_expired(expires_str);
                break;
            }
        }
    }

    Some((name, value, is_expired))
}

/// Check if an expires date string represents an expired cookie.
/// Parses HTTP date formats (RFC 2616 / RFC 7231).
fn is_cookie_expired(expires_str: &str) -> bool {
    // Try parsing common HTTP date formats
    // Format 1: "Sun, 06 Nov 1994 08:49:37 GMT" (RFC 1123)
    // Format 2: "Sunday, 06-Nov-94 08:49:37 GMT" (RFC 850)
    // Format 3: "Sun Nov  6 08:49:37 1994" (ANSI C asctime())
    use std::time::SystemTime;

    // Helper: parse a month name to 1-12
    fn parse_month(s: &str) -> Option<u32> {
        match s.to_lowercase().as_str() {
            "jan" => Some(1),
            "feb" => Some(2),
            "mar" => Some(3),
            "apr" => Some(4),
            "may" => Some(5),
            "jun" => Some(6),
            "jul" => Some(7),
            "aug" => Some(8),
            "sep" => Some(9),
            "oct" => Some(10),
            "nov" => Some(11),
            "dec" => Some(12),
            _ => None,
        }
    }

    // Try to parse RFC 1123 format: "Sun, 06 Nov 1994 08:49:37 GMT"
    // or RFC 850 format: "Sunday, 06-Nov-94 08:49:37 GMT"
    let parts: Vec<&str> = expires_str.split_whitespace().collect();

    if parts.len() >= 4 {
        // Try extracting day, month, year, time
        let (day_str, month_str, year_str, time_str) = if parts[0].ends_with(',') {
            // RFC 1123/850: "Sun, 06 Nov 1994 08:49:37 GMT" or "Sunday, 06-Nov-94 08:49:37 GMT"
            if parts.len() >= 5 {
                // Handle "06-Nov-94" format
                if parts[1].contains('-') {
                    let date_parts: Vec<&str> = parts[1].split('-').collect();
                    if date_parts.len() == 3 {
                        (date_parts[0], date_parts[1], date_parts[2], parts[2])
                    } else {
                        return false;
                    }
                } else {
                    (parts[1], parts[2], parts[3], parts[4])
                }
            } else {
                return false;
            }
        } else {
            // Might be asctime format: "Sun Nov  6 08:49:37 1994"
            // Skip weekday, then month, day, time, year
            if parts.len() >= 5 {
                (parts[2], parts[1], parts[4], parts[3])
            } else {
                return false;
            }
        };

        let day: u32 = day_str.parse().ok().unwrap_or(1);
        let month = parse_month(month_str).unwrap_or(1);
        let year: i32 = {
            let y: i32 = year_str.parse().ok().unwrap_or(1970);
            // Handle 2-digit years (RFC 850)
            if y < 100 {
                if y >= 70 {
                    1900 + y
                } else {
                    2000 + y
                }
            } else {
                y
            }
        };

        // Parse time "HH:MM:SS"
        let time_parts: Vec<&str> = time_str.split(':').collect();
        let hour: u32 = time_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minute: u32 = time_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let second: u32 = time_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        // Calculate Unix timestamp for the parsed date
        // Days from epoch to start of year
        fn days_from_epoch_to_year(year: i32) -> i64 {
            let y = year as i64;
            365 * (y - 1970) + (y - 1969) / 4 - (y - 1901) / 100 + (y - 1601) / 400
        }

        fn is_leap_year(year: i32) -> bool {
            (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
        }

        fn days_in_month(month: u32, year: i32) -> u32 {
            match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if is_leap_year(year) {
                        29
                    } else {
                        28
                    }
                }
                _ => 30,
            }
        }

        let mut days = days_from_epoch_to_year(year);
        for m in 1..month {
            days += days_in_month(m, year) as i64;
        }
        days += (day as i64) - 1;

        let expires_secs = days * 86400 + (hour as i64) * 3600 + (minute as i64) * 60 + (second as i64);

        // Compare with current time
        if let Ok(now) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            return expires_secs < now.as_secs() as i64;
        }
    }

    false
}
