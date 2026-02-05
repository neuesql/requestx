//! URL type implementation

use percent_encoding::percent_decode_str;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use url::Url;

use crate::queryparams::QueryParams;

/// Maximum URL length (same as httpx)
const MAX_URL_LENGTH: usize = 65536;

/// Decode a percent-encoded fragment string
fn decode_fragment(encoded: &str) -> String {
    percent_decode_str(encoded)
        .decode_utf8()
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| encoded.to_string())
}

/// URL parsing and manipulation
#[allow(clippy::upper_case_acronyms)]
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
    /// Store original host for IDNA/IPv6 addresses (before normalization)
    original_host: Option<String>,
    /// Store original relative path for relative URLs (without leading /)
    relative_path: Option<String>,
    /// Store original raw path+query for preserving exact encoding (e.g., single quotes)
    original_raw_path: Option<String>,
}

impl URL {
    pub fn from_url(url: Url) -> Self {
        let fragment = url.fragment().unwrap_or("").to_string();
        // Default to true since url crate always normalizes to have slash
        Self {
            inner: url,
            fragment,
            has_trailing_slash: true,
            empty_scheme: false,
            empty_host: false,
            original_host: None,
            relative_path: None,
            original_raw_path: None,
        }
    }

    pub fn from_url_with_slash(url: Url, has_trailing_slash: bool) -> Self {
        let fragment = url.fragment().unwrap_or("").to_string();
        Self {
            inner: url,
            fragment,
            has_trailing_slash,
            empty_scheme: false,
            empty_host: false,
            original_host: None,
            relative_path: None,
            original_raw_path: None,
        }
    }

    pub fn from_url_with_host(url: Url, has_trailing_slash: bool, original_host: Option<String>) -> Self {
        let fragment = url.fragment().unwrap_or("").to_string();
        Self {
            inner: url,
            fragment,
            has_trailing_slash,
            empty_scheme: false,
            empty_host: false,
            original_host,
            relative_path: None,
            original_raw_path: None,
        }
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
            Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!("Invalid URL for join: {}", e))),
        }
    }

    /// Convert to string (preserving trailing slash based on original input)
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        // For relative URLs, return just the path/query/fragment
        if let Some(ref rel_path) = self.relative_path {
            let mut result = rel_path.clone();
            if let Some(query) = self.inner.query() {
                if !query.is_empty() {
                    result.push('?');
                    result.push_str(query);
                }
            }
            if !self.fragment.is_empty() {
                result.push('#');
                result.push_str(&self.fragment);
            }
            return result;
        }

        // If we have an original_host for IPv6 or percent-encoded hosts, reconstruct the URL
        // For IDNA, use the inner (punycode) format
        let s = if let Some(ref orig_host) = self.original_host {
            // Reconstruct for IPv6 (contains :) or percent-encoded hosts (contains %)
            if orig_host.contains(':') || orig_host.contains('%') {
                // Reconstruct URL with original host format
                let mut result = String::new();

                // Add scheme
                let scheme = self.inner.scheme();
                if scheme != "relative" {
                    result.push_str(scheme);
                    result.push_str("://");
                }

                // Add userinfo if present
                let username = self.inner.username();
                if !username.is_empty() {
                    result.push_str(username);
                    if let Some(password) = self.inner.password() {
                        result.push(':');
                        result.push_str(password);
                    }
                    result.push('@');
                }

                // Add host with original format
                if orig_host.contains(':') {
                    // IPv6 needs brackets
                    result.push('[');
                    result.push_str(orig_host);
                    result.push(']');
                } else {
                    result.push_str(orig_host);
                }

                // Add port if present
                if let Some(port) = self.inner.port() {
                    result.push(':');
                    result.push_str(&port.to_string());
                }

                // Add path
                result.push_str(self.inner.path());

                // Add query if present
                if let Some(query) = self.inner.query() {
                    result.push('?');
                    result.push_str(query);
                }

                // Add fragment if present
                if !self.fragment.is_empty() {
                    result.push('#');
                    result.push_str(&self.fragment);
                }

                result
            } else {
                // For IDNA, use the inner (punycode) format
                self.inner.to_string()
            }
        } else {
            self.inner.to_string()
        };

        // If the original URL didn't have an explicit trailing slash and path is just "/",
        // we need to remove it for compatibility with httpx behavior
        if !self.has_trailing_slash && self.inner.path() == "/" {
            // Handle case: URL ends with / (no query/fragment)
            if s.ends_with('/') && self.inner.query().is_none() && self.inner.fragment().is_none() {
                return s[..s.len() - 1].to_string();
            }

            // Handle case: path is / but followed by query (e.g., "http://example.com/?a=1")
            // Need to find and remove the "/" between host and "?"
            if self.inner.query().is_some() {
                // Find the pattern /?
                if let Some(pos) = s.find("/?") {
                    // Remove the / before ?
                    let mut result = s[..pos].to_string();
                    result.push_str(&s[pos + 1..]); // Skip the /
                    return result;
                }
            }

            // Handle case: path is / but followed by fragment (e.g., "http://example.com/#section")
            if !self.fragment.is_empty() {
                if let Some(pos) = s.find("/#") {
                    let mut result = s[..pos].to_string();
                    result.push_str(&s[pos + 1..]); // Skip the /
                    return result;
                }
            }
        }

        s
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
                &s[1..s.len() - 1]
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
            &host[1..host.len() - 1]
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
        _username: Option<&str>,
        _password: Option<&str>,
        params: Option<&Bound<'_, PyAny>>,
        _netloc: Option<&[u8]>,
        _raw_path: Option<&[u8]>,
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
                if !authority.starts_with('[') {
                    // Not IPv6
                    if let Some(colon_pos) = authority.rfind(':') {
                        // Check if there's an @ (userinfo) after this colon
                        let after_colon = &authority[colon_pos + 1..];
                        if !after_colon.contains('@') {
                            // This should be a port
                            if !after_colon.is_empty() && !after_colon.chars().all(|c| c.is_ascii_digit()) {
                                return Err(crate::exceptions::InvalidURL::new_err(format!("Invalid port: '{}'", after_colon)));
                            }
                        }
                    }
                }
            }

            // Check for invalid host addresses before parsing
            if let Some(authority_start) = url_str.find("://") {
                let after_scheme = &url_str[authority_start + 3..];
                // Find the host portion
                let host_start = if let Some(at_pos) = after_scheme.find('@') {
                    at_pos + 1
                } else {
                    0
                };
                let host_part = &after_scheme[host_start..];

                // Check for IPv6 address
                if host_part.starts_with('[') {
                    if let Some(bracket_end) = host_part.find(']') {
                        let ipv6_addr = &host_part[..bracket_end + 1];
                        let inner_addr = &host_part[1..bracket_end];
                        // Check if it's a valid IPv6 address (basic validation)
                        if !is_valid_ipv6(inner_addr) {
                            return Err(crate::exceptions::InvalidURL::new_err(format!("Invalid IPv6 address: '{}'", ipv6_addr)));
                        }
                    }
                } else {
                    // Find end of host
                    let host_end = host_part
                        .find(&[':', '/', '?', '#'][..])
                        .unwrap_or(host_part.len());
                    let host = &host_part[..host_end];

                    // Check if it looks like an IPv4 address
                    if looks_like_ipv4(host) && !is_valid_ipv4(host) {
                        return Err(crate::exceptions::InvalidURL::new_err(format!("Invalid IPv4 address: '{}'", host)));
                    }

                    // Check for invalid IDNA characters
                    if !host.is_empty() && !host.is_ascii() && !is_valid_idna(host) {
                        return Err(crate::exceptions::InvalidURL::new_err(format!("Invalid IDNA hostname: '{}'", host)));
                    }
                }
            }

            // Handle special cases that the url crate doesn't support well

            // Case 1: Empty scheme like "://example.com"
            if let Some(rest) = url_str.strip_prefix("://") {
                                          // Parse the rest as if it had http scheme, then mark as empty scheme
                let temp_url = format!("http://{}", rest);
                match Url::parse(&temp_url) {
                    Ok(mut parsed_url) => {
                        // Apply params if provided
                        if let Some(params_obj) = params {
                            let query_params = QueryParams::from_py(params_obj)?;
                            parsed_url.set_query(Some(&query_params.to_query_string()));
                        }
                        let has_trailing_slash = url_str
                            .split('?')
                            .next()
                            .unwrap_or(url_str)
                            .split('#')
                            .next()
                            .unwrap_or(url_str)
                            .ends_with('/');
                        let frag = decode_fragment(parsed_url.fragment().unwrap_or(""));
                        return Ok(Self {
                            inner: parsed_url,
                            fragment: frag,
                            has_trailing_slash,
                            empty_scheme: true, // Mark as empty scheme
                            empty_host: false,
                            original_host: None,
                            relative_path: None,
                            original_raw_path: None,
                        });
                    }
                    Err(e) => {
                        return Err(crate::exceptions::InvalidURL::new_err(format!("Invalid URL: {}", e)));
                    }
                }
            }

            // Case 2: Scheme with empty authority like "http://"
            if url_str.ends_with("://")
                || (url_str.contains("://") && {
                    let after = url_str.split("://").nth(1).unwrap_or("");
                    after.is_empty() || after == "/"
                })
            {
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
                        let frag = decode_fragment(parsed_url.fragment().unwrap_or(""));
                        return Ok(Self {
                            inner: parsed_url,
                            fragment: frag,
                            has_trailing_slash,
                            empty_scheme: false,
                            empty_host: true, // Mark as empty host
                            original_host: None,
                            relative_path: None,
                            original_raw_path: None,
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
                                original_host: None,
                                relative_path: None,
                                original_raw_path: None,
                            });
                        }
                    }
                }
            }

            // Pre-process URL to handle spaces in the host
            // URLs like "https://exam le.com/" should create a URL with host="exam%20le.com"
            // The url crate rejects percent-encoded hosts, so we use a placeholder and store the encoded host
            let (url_str_processed, space_encoded_host) = if let Some(authority_start) = url_str.find("://") {
                let scheme_part = &url_str[..authority_start + 3];
                let after_scheme = &url_str[authority_start + 3..];

                // Find the authority portion (before first / ? or #)
                let authority_end = after_scheme
                    .find(&['/', '?', '#'][..])
                    .unwrap_or(after_scheme.len());
                let authority_part = &after_scheme[..authority_end];
                let rest_part = &after_scheme[authority_end..];

                // Skip userinfo: find last @ to get the actual host portion
                let host_start_in_authority = if let Some(at_pos) = authority_part.rfind('@') {
                    at_pos + 1
                } else {
                    0
                };
                let host_and_port = &authority_part[host_start_in_authority..];
                let userinfo_part = &authority_part[..host_start_in_authority]; // includes trailing @

                // Separate host from port
                let host_only = if let Some(colon_pos) = host_and_port.rfind(':') {
                    let potential_port = &host_and_port[colon_pos + 1..];
                    if !potential_port.is_empty() && potential_port.chars().all(|c| c.is_ascii_digit()) {
                        &host_and_port[..colon_pos]
                    } else {
                        host_and_port
                    }
                } else {
                    host_and_port
                };

                // Check if host (not userinfo, not port) contains spaces
                if host_only.contains(' ') {
                    let encoded_host = host_only.replace(' ', "%20");
                    // Reconstruct authority with placeholder host but preserve userinfo and port
                    let port_part = &host_and_port[host_only.len()..]; // e.g., ":8080" or ""
                    let processed = format!("{}{}placeholder-space-host.invalid{}{}", scheme_part, userinfo_part, port_part, rest_part);
                    (processed, Some(encoded_host))
                } else {
                    (url_str.to_string(), None)
                }
            } else {
                (url_str.to_string(), None)
            };
            let url_str = url_str_processed.as_str();

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
                    // Apply params if provided and not empty
                    let params_applied = if let Some(params_obj) = params {
                        let query_params = QueryParams::from_py(params_obj)?;
                        let query_string = query_params.to_query_string();
                        // Only set query if params is not empty
                        if !query_string.is_empty() {
                            parsed_url.set_query(Some(&query_string));
                        } else {
                            // If empty params, also clear any existing query from URL
                            parsed_url.set_query(None);
                        }
                        true
                    } else {
                        false
                    };

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

                    let frag = decode_fragment(parsed_url.fragment().unwrap_or(""));
                    // If host had spaces, use the percent-encoded host as original_host
                    // Otherwise extract original host from URL string for IDNA/IPv6
                    let original_host = if let Some(ref encoded) = space_encoded_host {
                        Some(encoded.clone())
                    } else {
                        extract_original_host(url_str)
                    };
                    // Extract original raw_path to preserve exact encoding (e.g., unencoded single quotes)
                    // But if params were applied, they override the query, so don't use original raw_path
                    let original_raw_path = if params_applied {
                        None
                    } else {
                        extract_original_raw_path(url_str)
                    };
                    return Ok(Self {
                        inner: parsed_url,
                        fragment: frag,
                        has_trailing_slash,
                        empty_scheme: false,
                        empty_host: false,
                        original_host,
                        relative_path: None,
                        original_raw_path,
                    });
                }
                Err(e) => {
                    return Err(crate::exceptions::InvalidURL::new_err(format!("Invalid URL: {}", e)));
                }
            }
        }

        // Build URL from components
        // Only default to "http" scheme if a host is provided
        let host = host.unwrap_or("");
        let scheme = if host.is_empty() {
            scheme.unwrap_or("")
        } else {
            scheme.unwrap_or("http")
        };

        // Validate component lengths (max 65536 characters for any component)
        const MAX_COMPONENT_LENGTH: usize = 65536;
        if let Some(p) = path {
            if p.len() > MAX_COMPONENT_LENGTH {
                return Err(crate::exceptions::InvalidURL::new_err("URL component 'path' too long"));
            }
            // Check for non-printable characters in path
            for (i, c) in p.chars().enumerate() {
                if c.is_control() && c != '\t' {
                    return Err(crate::exceptions::InvalidURL::new_err(format!(
                        "Invalid non-printable ASCII character in URL path component, {:?} at position {}.",
                        c, i
                    )));
                }
            }
        }
        if let Some(q) = query {
            if q.len() > MAX_COMPONENT_LENGTH {
                return Err(crate::exceptions::InvalidURL::new_err("URL component 'query' too long"));
            }
        }
        if let Some(f) = fragment {
            if f.len() > MAX_COMPONENT_LENGTH {
                return Err(crate::exceptions::InvalidURL::new_err("URL component 'fragment' too long"));
            }
        }

        // Validate scheme
        if !scheme.is_empty()
            && !scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        {
            return Err(crate::exceptions::InvalidURL::new_err("Invalid URL component 'scheme'"));
        }

        // Check if host is IPv6 (contains : but is not a domain with port)
        // Strip brackets if present
        let host_clean = if host.starts_with('[') && host.ends_with(']') {
            &host[1..host.len() - 1]
        } else {
            host
        };
        let is_ipv6 = !host_clean.is_empty() && host_clean.contains(':');
        let host_for_url = if is_ipv6 {
            format!("[{}]", host_clean)
        } else {
            host.to_string()
        };

        let mut url_string = if host.is_empty() && scheme.is_empty() {
            String::new()
        } else {
            format!("{}://{}", scheme, host_for_url)
        };

        if let Some(p) = port {
            url_string.push_str(&format!(":{}", p));
        }

        let path = path.unwrap_or("/");

        // Validate path for absolute URLs
        if !host.is_empty() && !path.is_empty() && !path.starts_with('/') {
            return Err(crate::exceptions::InvalidURL::new_err("For absolute URLs, path must be empty or begin with '/'"));
        }

        // Validate path for relative URLs
        if host.is_empty() && scheme.is_empty() {
            if path.starts_with("//") {
                return Err(crate::exceptions::InvalidURL::new_err("Relative URLs cannot have a path starting with '//'"));
            }
            if path.starts_with(':') {
                return Err(crate::exceptions::InvalidURL::new_err("Relative URLs cannot have a path starting with ':'"));
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
                    // Store the original relative path (without leading /)
                    let rel_path = Some(path.to_string());
                    Ok(Self {
                        inner: u,
                        fragment: frag,
                        has_trailing_slash: has_slash,
                        empty_scheme: false,
                        empty_host: false,
                        original_host: None,
                        relative_path: rel_path,
                        original_raw_path: None,
                    })
                }
                Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!("Invalid URL: {}", e))),
            }
        } else {
            // Store original host if it's an IDNA or IPv6 address (use cleaned version without brackets)
            let orig_host = if is_ipv6 || !host.is_ascii() {
                Some(host_clean.to_string())
            } else {
                None
            };
            match Url::parse(&url_string) {
                Ok(u) => {
                    let has_slash = u.path() != "/" || url_string.ends_with('/');
                    Ok(Self {
                        inner: u,
                        fragment: frag,
                        has_trailing_slash: has_slash,
                        empty_scheme: false,
                        empty_host: false,
                        original_host: orig_host,
                        relative_path: None,
                        original_raw_path: None,
                    })
                }
                Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!("Invalid URL: {}", e))),
            }
        }
    }
}

/// Extract original host from URL string (for IDNA and IPv6 addresses)
fn extract_original_host(url_str: &str) -> Option<String> {
    // Find the host portion of the URL
    if let Some(authority_start) = url_str.find("://") {
        let after_scheme = &url_str[authority_start + 3..];

        // Skip userinfo if present
        let host_start = if let Some(at_pos) = after_scheme.find('@') {
            at_pos + 1
        } else {
            0
        };
        let host_part = &after_scheme[host_start..];

        // Find end of host (port, path, query, or fragment)
        let host_end = if host_part.starts_with('[') {
            // IPv6 address - find closing bracket
            if let Some(bracket_end) = host_part.find(']') {
                bracket_end + 1
            } else {
                host_part.len()
            }
        } else {
            // Regular host - find first delimiter
            host_part
                .find(&[':', '/', '?', '#'][..])
                .unwrap_or(host_part.len())
        };

        let host = &host_part[..host_end];

        // Strip brackets from IPv6
        let host = if host.starts_with('[') && host.ends_with(']') {
            &host[1..host.len() - 1]
        } else {
            host
        };

        // Only store if it contains non-ASCII (IDNA) or is IPv6
        if !host.is_ascii() || host.contains(':') {
            return Some(host.to_string());
        }
    }
    None
}

/// Extract original raw path (path + query) from URL string to preserve exact encoding
/// This is needed because the url crate may encode characters like single quotes
/// that shouldn't be encoded in query/path strings according to RFC 3986.
fn extract_original_raw_path(url_str: &str) -> Option<String> {
    // Find the path portion of the URL (after authority, before fragment)
    if let Some(authority_start) = url_str.find("://") {
        let after_scheme = &url_str[authority_start + 3..];

        // Find the start of the path (first /)
        if let Some(path_start) = after_scheme.find('/') {
            let path_and_rest = &after_scheme[path_start..];

            // Remove the fragment if present
            let raw_path = if let Some(frag_start) = path_and_rest.find('#') {
                &path_and_rest[..frag_start]
            } else {
                path_and_rest
            };

            // Normalize: encode spaces and non-ASCII while preserving
            // already-encoded %XX sequences and safe chars (like single quotes)
            return Some(normalize_raw_path(raw_path));
        }
    }
    None
}

/// Normalize a raw path string: percent-encode spaces and non-ASCII chars,
/// preserve already-encoded %XX sequences and all other characters.
fn normalize_raw_path(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len() * 2);
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 2 < bytes.len() && bytes[i + 1].is_ascii_hexdigit() && bytes[i + 2].is_ascii_hexdigit() {
            // Already-encoded sequence - preserve as-is (keep original case)
            result.push('%');
            result.push(bytes[i + 1] as char);
            result.push(bytes[i + 2] as char);
            i += 3;
        } else if b == b' ' {
            result.push_str("%20");
            i += 1;
        } else if b > 127 {
            // Non-ASCII byte - percent encode
            result.push_str(&format!("%{:02X}", b));
            i += 1;
        } else {
            result.push(b as char);
            i += 1;
        }
    }
    result
}

/// Check if a string looks like an IPv4 address (all digits and dots)
fn looks_like_ipv4(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Check if a string is a valid IPv4 address
fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for part in parts {
        if part.is_empty() {
            return false;
        }
        match part.parse::<u32>() {
            Ok(n) if n <= 255 => {}
            _ => return false,
        }
    }
    true
}

/// Check if a string is a valid IPv6 address (basic validation)
fn is_valid_ipv6(s: &str) -> bool {
    // Very basic IPv6 validation - check if it contains colons and valid hex digits
    if s.is_empty() {
        return false;
    }

    // IPv6 addresses must contain at least one colon (unless it's ::)
    if !s.contains(':') {
        return false;
    }

    // Check for valid characters: hex digits, colons, dots (for IPv4-mapped addresses)
    for c in s.chars() {
        if !c.is_ascii_hexdigit() && c != ':' && c != '.' {
            return false;
        }
    }

    // Check each group (simple validation)
    let groups: Vec<&str> = s.split(':').collect();

    for group in &groups {
        if group.is_empty() {
            continue;
        }
        // Check if it's an IPv4 suffix (for IPv4-mapped addresses)
        if group.contains('.') {
            if !is_valid_ipv4(group) {
                return false;
            }
        } else {
            // IPv6 groups should be at most 4 hex digits
            if group.len() > 4 {
                return false;
            }
        }
    }

    // :: can only appear once (represented by more than one consecutive empty group)
    // But we need to handle cases like "::1" (2 empty groups at start) and "1::" (2 at end)
    // and "::" (3 empty groups)
    true
}

/// Encode userinfo (username/password) for URL
/// This encodes special characters but NOT percent signs (to avoid double-encoding)
fn encode_userinfo(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '@' => result.push_str("%40"),
            ' ' => result.push_str("%20"),
            ':' => result.push_str("%3A"),
            '/' => result.push_str("%2F"),
            '?' => result.push_str("%3F"),
            '#' => result.push_str("%23"),
            '[' => result.push_str("%5B"),
            ']' => result.push_str("%5D"),
            // Don't encode % - assume it's already encoded
            '%' => result.push('%'),
            // Allow unreserved characters
            c if c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~' => {
                result.push(c);
            }
            // Encode other characters
            c => {
                for b in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    result
}

/// Check if a hostname is a valid IDNA (basic validation)
fn is_valid_idna(s: &str) -> bool {
    // Check each label in the hostname
    for label in s.split('.') {
        if label.is_empty() {
            continue;
        }
        // Check for invalid Unicode categories
        for c in label.chars() {
            // Disallow certain characters that are invalid in IDNA 2008
            // This includes symbols, emojis (most), and certain combining marks
            let cat = c as u32;

            // Common invalid characters in IDNA:
            // - Emoji (most in range 0x1F000-0x1FFFF or specific characters)
            // - Symbols like ☃ (U+2603)
            if (0x2600..=0x26FF).contains(&cat) {
                // Miscellaneous Symbols block - includes snowman (☃)
                return false;
            }
            if (0x1F300..=0x1FFFF).contains(&cat) {
                // Emoji and symbols
                return false;
            }
        }
    }
    true
}

#[pymethods]
impl URL {
    #[new]
    #[pyo3(signature = (url=None, **kwargs))]
    fn py_new(url: Option<&Bound<'_, PyAny>>, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        // Validate and extract url argument
        let url_str: Option<String> = match url {
            None => None,
            Some(obj) => {
                if obj.is_none() {
                    None
                } else {
                    match obj.extract::<String>() {
                        Ok(s) => Some(s),
                        Err(_) => {
                            let type_name = obj.get_type().qualname()?;
                            return Err(PyTypeError::new_err(format!("Invalid type for url. Expected str but got {}", type_name)));
                        }
                    }
                }
            }
        };

        // Valid keyword arguments
        const VALID_KWARGS: &[&str] = &["scheme", "host", "port", "path", "query", "fragment", "username", "password", "params", "netloc", "raw_path"];

        let mut scheme_owned: Option<String> = None;
        let mut host_owned: Option<String> = None;
        let mut port: Option<u16> = None;
        let mut path_owned: Option<String> = None;
        let mut query_owned: Option<Vec<u8>> = None;
        let mut fragment_owned: Option<String> = None;
        let mut username_owned: Option<String> = None;
        let mut password_owned: Option<String> = None;
        let mut params_obj: Option<Bound<'_, PyAny>> = None;
        let mut netloc_owned: Option<Vec<u8>> = None;
        let mut raw_path_owned: Option<Vec<u8>> = None;

        if let Some(kw) = kwargs {
            for (key, value) in kw.iter() {
                let key_str: String = key.extract()?;
                if !VALID_KWARGS.contains(&key_str.as_str()) {
                    return Err(PyTypeError::new_err(format!("'{}' is an invalid keyword argument for URL()", key_str)));
                }
                match key_str.as_str() {
                    "scheme" => scheme_owned = Some(value.extract()?),
                    "host" => host_owned = Some(value.extract()?),
                    "port" => {
                        if value.is_none() {
                            port = None;
                        } else {
                            port = Some(value.extract()?);
                        }
                    }
                    "path" => path_owned = Some(value.extract()?),
                    "query" => query_owned = Some(value.extract()?),
                    "fragment" => fragment_owned = Some(value.extract()?),
                    "username" => username_owned = Some(value.extract()?),
                    "password" => password_owned = Some(value.extract()?),
                    "params" => params_obj = Some(value.clone()),
                    "netloc" => netloc_owned = Some(value.extract()?),
                    "raw_path" => raw_path_owned = Some(value.extract()?),
                    _ => unreachable!(),
                }
            }
        }

        // Early validation of component kwargs (even when url string is provided)
        if let Some(ref p) = path_owned {
            if p.len() > MAX_URL_LENGTH {
                return Err(crate::exceptions::InvalidURL::new_err("URL component 'path' too long"));
            }
            for (i, c) in p.chars().enumerate() {
                if c.is_control() && c != '\t' {
                    return Err(crate::exceptions::InvalidURL::new_err(format!(
                        "Invalid non-printable ASCII character in URL path component, {:?} at position {}.",
                        c, i
                    )));
                }
            }
        }
        if let Some(ref q) = query_owned {
            if q.len() > MAX_URL_LENGTH {
                return Err(crate::exceptions::InvalidURL::new_err("URL component 'query' too long"));
            }
        }
        if let Some(ref f) = fragment_owned {
            if f.len() > MAX_URL_LENGTH {
                return Err(crate::exceptions::InvalidURL::new_err("URL component 'fragment' too long"));
            }
        }

        Self::new_impl(
            url_str.as_deref(),
            scheme_owned.as_deref(),
            host_owned.as_deref(),
            port,
            path_owned.as_deref(),
            query_owned.as_deref(),
            fragment_owned.as_deref(),
            username_owned.as_deref(),
            password_owned.as_deref(),
            params_obj.as_ref(),
            netloc_owned.as_deref(),
            raw_path_owned.as_deref(),
        )
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
        // Return original host if available (for IDNA/IPv6 addresses)
        if let Some(ref orig) = self.original_host {
            // Strip brackets from IPv6 if present
            let host = if orig.starts_with('[') && orig.ends_with(']') {
                &orig[1..orig.len() - 1]
            } else {
                orig.as_str()
            };
            return host.to_lowercase();
        }
        let host = self.inner.host_str().unwrap_or("");
        // Strip brackets for IPv6 addresses - httpx returns host without brackets
        let host = if host.starts_with('[') && host.ends_with(']') {
            &host[1..host.len() - 1]
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
    fn path(&self) -> String {
        // For relative URLs, return the original relative path
        if let Some(ref rel_path) = self.relative_path {
            return urlencoding::decode(rel_path)
                .unwrap_or_else(|_| rel_path.as_str().into())
                .into_owned();
        }
        // Return decoded path (percent-decode)
        let raw_path = self.inner.path();
        urlencoding::decode(raw_path)
            .unwrap_or_else(|_| raw_path.into())
            .into_owned()
    }

    #[getter]
    fn query<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        // Use original_raw_path to preserve exact query encoding (e.g., unencoded single quotes)
        if let Some(ref orig_raw) = self.original_raw_path {
            if let Some(query_pos) = orig_raw.find('?') {
                let q = &orig_raw[query_pos + 1..];
                return PyBytes::new(py, q.as_bytes());
            }
            // original_raw_path exists but no query
            return PyBytes::new(py, b"");
        }
        let q = self.inner.query().unwrap_or("");
        PyBytes::new(py, q.as_bytes())
    }

    #[getter]
    fn fragment(&self) -> &str {
        &self.fragment
    }

    #[getter]
    fn raw_path<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        // If we have the original raw_path stored, use it to preserve exact encoding
        if let Some(ref orig_raw) = self.original_raw_path {
            return PyBytes::new(py, orig_raw.as_bytes());
        }

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
        // For IPv6 addresses or percent-encoded hosts with original_host, return the original format
        // For IDNA, use the punycode-encoded form from inner
        if let Some(ref orig) = self.original_host {
            // Use original_host for IPv6 (contains :) or percent-encoded hosts (contains %)
            if orig.contains(':') || orig.contains('%') {
                return PyBytes::new(py, orig.as_bytes());
            }
        }
        let host = self.inner.host_str().unwrap_or("");
        // Strip brackets for IPv6 addresses - httpcore expects host without brackets
        let host = if host.starts_with('[') && host.ends_with(']') {
            &host[1..host.len() - 1]
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
        // Use original host for IPv6 or percent-encoded hosts, use inner (punycode) for IDNA
        let raw_host = self.inner.host_str().unwrap_or("");
        let host = if let Some(ref orig) = self.original_host {
            if orig.contains(':') {
                // IPv6 needs brackets
                format!("[{}]", orig)
            } else if orig.contains('%') {
                // Percent-encoded host (e.g., spaces encoded as %20)
                orig.clone()
            } else {
                // For IDNA, use the punycode-encoded form from inner
                raw_host.to_string()
            }
        } else {
            raw_host.to_string()
        };
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
            Ok(joined) => {
                // Check if the joined URL should have a trailing slash
                // Only preserve slash if the input URL had one at the end
                let input_has_slash = url.ends_with('/');
                let has_slash = if joined.path() == "/" {
                    // For root path, check if original input ended with /
                    input_has_slash || url == "/"
                } else {
                    input_has_slash
                };

                // If base URL is relative (has relative_path), result should also be relative
                let rel_path = if self.relative_path.is_some() || self.inner.scheme() == "relative" {
                    // For relative URLs, the path from joined is the relative path
                    let path = joined.path();
                    Some(path.to_string())
                } else {
                    None
                };

                let frag = joined.fragment().unwrap_or("").to_string();
                Ok(Self {
                    inner: joined,
                    fragment: frag,
                    has_trailing_slash: has_slash,
                    empty_scheme: false,
                    empty_host: false,
                    original_host: None,
                    relative_path: rel_path,
                    original_raw_path: None,
                })
            }
            Err(e) => Err(crate::exceptions::InvalidURL::new_err(format!("Invalid URL for join: {}", e))),
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
                        new_url
                            .inner
                            .set_scheme(&scheme)
                            .map_err(|_| crate::exceptions::InvalidURL::new_err("Invalid scheme"))?;
                    }
                    "host" => {
                        let host: String = value.extract()?;
                        // Strip brackets if present (user might pass [::1] or ::1)
                        let host_clean = if host.starts_with('[') && host.ends_with(']') {
                            &host[1..host.len() - 1]
                        } else {
                            &host
                        };
                        // Check if this is an IPv6 address (contains : but not as port separator)
                        let is_ipv6 = host_clean.contains(':') && !host_clean.contains('/');
                        let host_to_set = if is_ipv6 {
                            format!("[{}]", host_clean)
                        } else {
                            host_clean.to_string()
                        };
                        new_url
                            .inner
                            .set_host(Some(&host_to_set))
                            .map_err(|e| crate::exceptions::InvalidURL::new_err(format!("Invalid host: {}", e)))?;
                        // Store original host for IDNA/IPv6
                        if is_ipv6 || !host.is_ascii() {
                            new_url.original_host = Some(host_clean.to_string());
                        } else {
                            new_url.original_host = None;
                        }
                    }
                    "port" => {
                        // Handle port - allow large values in URL (will fail at connection time)
                        if value.is_none() {
                            new_url
                                .inner
                                .set_port(None)
                                .map_err(|_| crate::exceptions::InvalidURL::new_err("Invalid port"))?;
                        } else {
                            let port_value: i64 = value.extract()?;
                            // Store as u16 by taking modulo - the connection will fail if truly invalid
                            // This matches httpx behavior which allows "impossible" ports in URLs
                            if port_value < 0 {
                                return Err(crate::exceptions::InvalidURL::new_err("Invalid port: negative values not allowed"));
                            }
                            // Convert large port numbers by truncating to u16 range
                            // The URL will be invalid for actual connections
                            let port_u16 = (port_value % 65536) as u16;
                            new_url
                                .inner
                                .set_port(Some(port_u16))
                                .map_err(|_| crate::exceptions::InvalidURL::new_err("Invalid port"))?;
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
                        new_url
                            .inner
                            .set_fragment(if frag.is_empty() { None } else { Some(&frag) });
                    }
                    "netloc" => {
                        let netloc: &[u8] = value.extract()?;
                        let netloc_str = String::from_utf8_lossy(netloc);
                        // Parse netloc (may contain host:port)
                        if let Some(idx) = netloc_str.rfind(':') {
                            let (host, port_str) = netloc_str.split_at(idx);
                            let port_str = &port_str[1..];
                            if let Ok(port) = port_str.parse::<u16>() {
                                new_url
                                    .inner
                                    .set_host(Some(host))
                                    .map_err(|e| crate::exceptions::InvalidURL::new_err(format!("Invalid host: {}", e)))?;
                                new_url
                                    .inner
                                    .set_port(Some(port))
                                    .map_err(|_| crate::exceptions::InvalidURL::new_err("Invalid port"))?;
                            } else {
                                new_url
                                    .inner
                                    .set_host(Some(&netloc_str))
                                    .map_err(|e| crate::exceptions::InvalidURL::new_err(format!("Invalid host: {}", e)))?;
                            }
                        } else {
                            new_url
                                .inner
                                .set_host(Some(&netloc_str))
                                .map_err(|e| crate::exceptions::InvalidURL::new_err(format!("Invalid host: {}", e)))?;
                        }
                    }
                    "username" => {
                        let username: String = value.extract()?;
                        let encoded = encode_userinfo(&username);
                        new_url
                            .inner
                            .set_username(&encoded)
                            .map_err(|_| crate::exceptions::InvalidURL::new_err("Cannot set username"))?;
                    }
                    "password" => {
                        let password: String = value.extract()?;
                        let encoded = encode_userinfo(&password);
                        new_url
                            .inner
                            .set_password(Some(&encoded))
                            .map_err(|_| crate::exceptions::InvalidURL::new_err("Cannot set password"))?;
                    }
                    other => {
                        return Err(PyTypeError::new_err(format!("'{}' is an invalid keyword argument for URL()", other)));
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
