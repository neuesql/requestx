// url.rs - HTTPX-compatible URL implementation for RequestX
//
// This module provides a complete URL parsing and manipulation implementation
// that is fully compatible with httpx.URL, including:
// - IDNA hostname support (internationalized domain names)
// - Proper percent-encoding/decoding for all URL components
// - Path normalization (resolving . and ..)
// - IPv4/IPv6 address handling
// - Query parameter manipulation
// - URL joining (RFC 3986 compliant)
// - copy_with() for URL modifications

use pyo3::prelude::*;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::types::{PyBytes, PyDict, PyString};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{Ipv4Addr, Ipv6Addr};

/// Maximum URL length to prevent DoS
const MAX_URL_LENGTH: usize = 65536;
/// Maximum component length
const MAX_COMPONENT_LENGTH: usize = 65536;

/// Default ports for common schemes
fn default_port_for_scheme(scheme: &str) -> Option<u16> {
    match scheme.to_lowercase().as_str() {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}

/// Custom error type for invalid URLs
#[derive(Debug, Clone)]
pub struct InvalidURL {
    pub message: String,
}

impl InvalidURL {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for InvalidURL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for InvalidURL {}

impl From<InvalidURL> for PyErr {
    fn from(err: InvalidURL) -> PyErr {
        // Create an InvalidURL exception in Python
        // This should map to httpx.InvalidURL
        PyValueError::new_err(err.message)
    }
}

/// Internal URL representation
#[derive(Debug, Clone)]
struct UrlComponents {
    scheme: String,
    username: String,
    password: Option<String>,
    host: String,
    raw_host: Vec<u8>,
    port: Option<u16>,
    path: String,
    raw_path: Vec<u8>,
    query: Vec<u8>,
    fragment: String,
    /// Whether the URL had an explicit '?' with empty query
    has_trailing_question: bool,
}

impl Default for UrlComponents {
    fn default() -> Self {
        Self {
            scheme: String::new(),
            username: String::new(),
            password: None,
            host: String::new(),
            raw_host: Vec::new(),
            port: None,
            path: String::new(),
            raw_path: Vec::new(),
            query: Vec::new(),
            fragment: String::new(),
            has_trailing_question: false,
        }
    }
}

/// Python-exposed URL class
#[pyclass(name = "URL")]
#[derive(Debug, Clone)]
pub struct URL {
    components: UrlComponents,
    /// Original string representation (normalized)
    url_string: String,
}

// ============================================================================
// Percent Encoding/Decoding Utilities
// ============================================================================

/// Characters that are safe in path component (RFC 3986 pchar without pct-encoded)
fn is_path_safe(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, '-' | '.' | '_' | '~' | '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' | ':' | '@' | '/' | '[' | ']')
}

/// Characters that are safe in query component
fn is_query_safe(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, '-' | '.' | '_' | '~' | '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' | ':' | '@' | '/' | '?' | '[' | ']')
}

/// Characters that are safe in userinfo component
fn is_userinfo_safe(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, '-' | '.' | '_' | '~' | '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' | '%')
}

/// Percent-encode a string with a custom safety predicate
fn percent_encode<F>(input: &str, is_safe: F) -> String
where
    F: Fn(char) -> bool,
{
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        if is_safe(c) {
            result.push(c);
        } else if c.is_ascii() {
            result.push_str(&format!("%{:02X}", c as u8));
        } else {
            // Encode UTF-8 bytes
            for b in c.to_string().as_bytes() {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}

/// Percent-encode bytes
fn percent_encode_bytes<F>(input: &[u8], is_safe: F) -> Vec<u8>
where
    F: Fn(u8) -> bool,
{
    let mut result = Vec::with_capacity(input.len());
    for &b in input {
        if is_safe(b) {
            result.push(b);
        } else {
            result.extend_from_slice(format!("%{:02X}", b).as_bytes());
        }
    }
    result
}

/// Decode percent-encoded string
fn percent_decode(input: &str) -> Result<String, InvalidURL> {
    let bytes = percent_decode_bytes(input.as_bytes())?;
    String::from_utf8(bytes).map_err(|_| InvalidURL::new("Invalid UTF-8 in URL"))
}

/// Decode percent-encoded bytes
fn percent_decode_bytes(input: &[u8]) -> Result<Vec<u8>, InvalidURL> {
    let mut result = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' && i + 2 < input.len() {
            let hex = std::str::from_utf8(&input[i + 1..i + 3])
                .map_err(|_| InvalidURL::new("Invalid percent encoding"))?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| InvalidURL::new("Invalid percent encoding"))?;
            result.push(byte);
            i += 3;
        } else {
            result.push(input[i]);
            i += 1;
        }
    }
    Ok(result)
}

/// Normalize percent encoding - decode safe chars, encode unsafe ones
fn normalize_percent_encoding<F>(input: &str, is_safe: F) -> String
where
    F: Fn(char) -> bool + Copy,
{
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            // Try to decode
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    let c = byte as char;
                    if c.is_ascii() && is_safe(c) {
                        // Safe char - keep decoded
                        result.push(c);
                    } else {
                        // Keep encoded (uppercase)
                        result.push('%');
                        result.push_str(&hex.to_uppercase());
                    }
                    i += 3;
                    continue;
                }
            }
        }
        
        let c = bytes[i] as char;
        if c.is_ascii() {
            if is_safe(c) || c == '%' {
                result.push(c);
            } else {
                result.push_str(&format!("%{:02X}", bytes[i]));
            }
        } else {
            // Non-ASCII - encode
            result.push_str(&format!("%{:02X}", bytes[i]));
        }
        i += 1;
    }
    
    result
}

// ============================================================================
// IDNA Support
// ============================================================================

/// Convert Unicode hostname to ASCII (punycode)
fn idna_encode(host: &str) -> Result<String, InvalidURL> {
    // Check if already ASCII
    if host.is_ascii() {
        return Ok(host.to_lowercase());
    }
    
    let mut result = String::new();
    for (i, label) in host.split('.').enumerate() {
        if i > 0 {
            result.push('.');
        }
        
        if label.is_ascii() {
            result.push_str(&label.to_lowercase());
        } else {
            // Encode using punycode
            match punycode_encode(label) {
                Ok(encoded) => {
                    result.push_str("xn--");
                    result.push_str(&encoded);
                }
                Err(_) => {
                    return Err(InvalidURL::new(format!("Invalid IDNA hostname: '{}'", host)));
                }
            }
        }
    }
    
    Ok(result)
}

/// Simple punycode encoder
fn punycode_encode(input: &str) -> Result<String, InvalidURL> {
    const BASE: u32 = 36;
    const TMIN: u32 = 1;
    const TMAX: u32 = 26;
    const SKEW: u32 = 38;
    const DAMP: u32 = 700;
    const INITIAL_BIAS: u32 = 72;
    const INITIAL_N: u32 = 128;
    
    let input: Vec<char> = input.chars().collect();
    let mut output = String::new();
    
    // Copy basic code points
    let mut basic_count = 0u32;
    for &c in &input {
        if (c as u32) < 128 {
            output.push(c.to_ascii_lowercase());
            basic_count += 1;
        }
    }
    
    let mut handled = basic_count;
    if basic_count > 0 {
        output.push('-');
    }
    
    let mut n = INITIAL_N;
    let mut delta = 0u32;
    let mut bias = INITIAL_BIAS;
    
    let input_len = input.len() as u32;
    
    while handled < input_len {
        // Find minimum code point >= n
        let mut m = u32::MAX;
        for &c in &input {
            let cp = c as u32;
            if cp >= n && cp < m {
                m = cp;
            }
        }
        
        delta = delta.saturating_add((m - n).saturating_mul(handled + 1));
        n = m;
        
        for &c in &input {
            let cp = c as u32;
            if cp < n {
                delta = delta.saturating_add(1);
            } else if cp == n {
                let mut q = delta;
                let mut k = BASE;
                
                loop {
                    let t = if k <= bias {
                        TMIN
                    } else if k >= bias + TMAX {
                        TMAX
                    } else {
                        k - bias
                    };
                    
                    if q < t {
                        break;
                    }
                    
                    let digit = t + (q - t) % (BASE - t);
                    output.push(encode_digit(digit));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                
                output.push(encode_digit(q));
                bias = adapt(delta, handled + 1, handled == basic_count);
                delta = 0;
                handled += 1;
            }
        }
        
        delta += 1;
        n += 1;
    }
    
    Ok(output)
}

fn encode_digit(d: u32) -> char {
    if d < 26 {
        (b'a' + d as u8) as char
    } else {
        (b'0' + (d - 26) as u8) as char
    }
}

fn adapt(mut delta: u32, num_points: u32, first_time: bool) -> u32 {
    const BASE: u32 = 36;
    const TMIN: u32 = 1;
    const TMAX: u32 = 26;
    const SKEW: u32 = 38;
    const DAMP: u32 = 700;
    
    delta = if first_time {
        delta / DAMP
    } else {
        delta / 2
    };
    delta += delta / num_points;
    
    let mut k = 0;
    while delta > ((BASE - TMIN) * TMAX) / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }
    
    k + (BASE - TMIN + 1) * delta / (delta + SKEW)
}

// ============================================================================
// IP Address Validation
// ============================================================================

fn parse_ipv4(host: &str) -> Result<Ipv4Addr, InvalidURL> {
    host.parse::<Ipv4Addr>()
        .map_err(|_| InvalidURL::new(format!("Invalid IPv4 address: '{}'", host)))
}

fn parse_ipv6(host: &str) -> Result<Ipv6Addr, InvalidURL> {
    // Remove brackets if present
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.parse::<Ipv6Addr>()
        .map_err(|_| InvalidURL::new(format!("Invalid IPv6 address: '[{}]'", host)))
}

fn is_ipv4_address(host: &str) -> bool {
    host.parse::<Ipv4Addr>().is_ok()
}

fn is_ipv6_address(host: &str) -> bool {
    let h = host.trim_start_matches('[').trim_end_matches(']');
    h.parse::<Ipv6Addr>().is_ok()
}

// ============================================================================
// Path Normalization
// ============================================================================

/// Normalize path by resolving . and .. segments (RFC 3986 Section 5.2.4)
fn normalize_path(path: &str, is_absolute: bool) -> String {
    let mut segments: Vec<&str> = Vec::new();
    
    for segment in path.split('/') {
        match segment {
            "." => {
                // Skip current directory
            }
            ".." => {
                // Go up one directory (but don't go above root for absolute URLs)
                if !segments.is_empty() && segments.last() != Some(&"..") {
                    segments.pop();
                } else if !is_absolute {
                    segments.push("..");
                }
            }
            s => {
                if !s.is_empty() || segments.is_empty() {
                    segments.push(s);
                }
            }
        }
    }
    
    let mut result = segments.join("/");
    
    // Preserve trailing slash
    if path.ends_with('/') && !result.ends_with('/') {
        result.push('/');
    }
    
    // Ensure absolute paths start with /
    if is_absolute && !result.starts_with('/') {
        result.insert(0, '/');
    }
    
    if result.is_empty() && is_absolute {
        return "/".to_string();
    }
    
    result
}

// ============================================================================
// URL Parsing
// ============================================================================

/// Check for non-printable ASCII characters
fn check_non_printable(input: &str, component_name: Option<&str>) -> Result<(), InvalidURL> {
    for (i, c) in input.chars().enumerate() {
        if c.is_ascii_control() {
            let char_repr = match c {
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                '\t' => "\\t".to_string(),
                _ => format!("\\x{:02x}", c as u8),
            };
            
            let msg = if let Some(name) = component_name {
                format!(
                    "Invalid non-printable ASCII character in URL {} component, '{}' at position {}.",
                    name, char_repr, i
                )
            } else {
                format!(
                    "Invalid non-printable ASCII character in URL, '{}' at position {}.",
                    char_repr, i
                )
            };
            return Err(InvalidURL::new(msg));
        }
    }
    Ok(())
}

/// Parse a URL string into components
fn parse_url(url: &str) -> Result<UrlComponents, InvalidURL> {
    // Check length
    if url.len() > MAX_URL_LENGTH {
        return Err(InvalidURL::new("URL too long"));
    }
    
    // Check for non-printable characters
    check_non_printable(url, None)?;
    
    let mut components = UrlComponents::default();
    let mut remaining = url;
    
    // Parse fragment (from the end)
    if let Some(hash_pos) = remaining.find('#') {
        components.fragment = remaining[hash_pos + 1..].to_string();
        remaining = &remaining[..hash_pos];
    }
    
    // Parse scheme
    if let Some(colon_pos) = remaining.find(':') {
        let potential_scheme = &remaining[..colon_pos];
        if is_valid_scheme(potential_scheme) {
            components.scheme = potential_scheme.to_lowercase();
            remaining = &remaining[colon_pos + 1..];
        }
    }
    
    // Parse authority (if present)
    if remaining.starts_with("//") {
        remaining = &remaining[2..];
        
        // Find end of authority
        let auth_end = remaining.find('/').unwrap_or(remaining.len());
        let auth_end = auth_end.min(remaining.find('?').unwrap_or(remaining.len()));
        
        let authority = &remaining[..auth_end];
        remaining = &remaining[auth_end..];
        
        // Parse userinfo
        if let Some(at_pos) = authority.rfind('@') {
            let userinfo = &authority[..at_pos];
            let host_part = &authority[at_pos + 1..];
            
            // Parse username:password
            if let Some(colon_pos) = userinfo.find(':') {
                components.username = percent_decode(&userinfo[..colon_pos])?;
                components.password = Some(percent_decode(&userinfo[colon_pos + 1..])?);
            } else {
                components.username = percent_decode(userinfo)?;
            }
            
            parse_host_port(host_part, &mut components)?;
        } else {
            parse_host_port(authority, &mut components)?;
        }
        
        // Ensure path starts with / for absolute URLs
        if remaining.is_empty() {
            remaining = "/";
        }
    }
    
    // Parse query
    if let Some(query_pos) = remaining.find('?') {
        let query_str = &remaining[query_pos + 1..];
        components.has_trailing_question = true;
        
        // Normalize query encoding
        let normalized = normalize_percent_encoding(query_str, is_query_safe);
        components.query = normalized.into_bytes();
        
        remaining = &remaining[..query_pos];
    }
    
    // The rest is the path
    let is_absolute = !components.scheme.is_empty() || !components.host.is_empty();
    
    // Normalize path encoding
    let path_str = normalize_percent_encoding(remaining, is_path_safe);
    
    // Normalize the path (resolve . and ..)
    let normalized_path = normalize_path(&path_str, is_absolute);
    
    // Decode for the decoded path property
    components.path = percent_decode(&normalized_path)?;
    
    // Build raw_path (encoded path + query)
    let encoded_path = encode_path(&components.path);
    let mut raw_path = encoded_path.into_bytes();
    if !components.query.is_empty() || components.has_trailing_question {
        raw_path.push(b'?');
        raw_path.extend_from_slice(&components.query);
    }
    components.raw_path = raw_path;
    
    Ok(components)
}

fn is_valid_scheme(s: &str) -> bool {
    if s.is_empty() {
        return true; // Empty scheme is valid for relative URLs
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_alphabetic() {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

fn parse_host_port(input: &str, components: &mut UrlComponents) -> Result<(), InvalidURL> {
    let input = input.trim();
    
    if input.is_empty() {
        components.host = String::new();
        components.raw_host = Vec::new();
        return Ok(());
    }
    
    // Handle IPv6 addresses [...]
    if input.starts_with('[') {
        if let Some(bracket_end) = input.find(']') {
            let ipv6_str = &input[1..bracket_end];
            let _ = parse_ipv6(ipv6_str)?;
            
            components.host = ipv6_str.to_lowercase();
            components.raw_host = format!("[{}]", ipv6_str.to_lowercase()).into_bytes();
            
            // Parse port after ]
            if bracket_end + 1 < input.len() {
                let after_bracket = &input[bracket_end + 1..];
                if let Some(port_str) = after_bracket.strip_prefix(':') {
                    if !port_str.is_empty() {
                        components.port = parse_port(port_str)?;
                    }
                }
            }
            
            return Ok(());
        } else {
            return Err(InvalidURL::new(format!("Invalid IPv6 address: '{}'", input)));
        }
    }
    
    // Regular host:port parsing
    let (host_str, port_str) = if let Some(colon_pos) = input.rfind(':') {
        let potential_port = &input[colon_pos + 1..];
        // Make sure it's a port and not part of the host
        if potential_port.chars().all(|c| c.is_ascii_digit()) {
            (&input[..colon_pos], Some(potential_port))
        } else {
            (input, None)
        }
    } else {
        (input, None)
    };
    
    // Parse port
    if let Some(ps) = port_str {
        if !ps.is_empty() {
            components.port = parse_port(ps)?;
        }
    }
    
    // Process host
    let host = host_str.to_string();
    
    // Check if it looks like an IPv4 address
    if host.chars().all(|c| c.is_ascii_digit() || c == '.') && host.contains('.') {
        // Validate IPv4
        let parts: Vec<&str> = host.split('.').collect();
        if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
            // It's an IPv4 address - validate it
            let _ = parse_ipv4(&host)?;
            components.host = host.clone();
            components.raw_host = host.into_bytes();
            return Ok(());
        }
    }
    
    // Check if host needs percent encoding for spaces
    if host.contains(' ') || host.chars().any(|c| !c.is_ascii()) {
        // Percent-encode spaces in host
        if host.contains(' ') {
            let encoded_host = host.replace(' ', "%20");
            components.host = encoded_host.clone();
            components.raw_host = encoded_host.into_bytes();
            return Ok(());
        }
        
        // Handle IDNA
        let ascii_host = idna_encode(&host)?;
        components.host = host.to_lowercase();
        components.raw_host = ascii_host.into_bytes();
    } else {
        // Regular ASCII hostname
        components.host = host.to_lowercase();
        components.raw_host = components.host.clone().into_bytes();
    }
    
    Ok(())
}

fn parse_port(port_str: &str) -> Result<Option<u16>, InvalidURL> {
    if port_str.is_empty() {
        return Ok(None);
    }
    
    port_str.parse::<u16>()
        .map(Some)
        .map_err(|_| InvalidURL::new(format!("Invalid port: '{}'", port_str)))
}

fn encode_path(path: &str) -> String {
    percent_encode(path, is_path_safe)
}

// ============================================================================
// URL Building
// ============================================================================

fn build_url_string(components: &UrlComponents) -> String {
    let mut result = String::new();
    
    // Scheme
    if !components.scheme.is_empty() {
        result.push_str(&components.scheme);
        result.push(':');
    }
    
    // Authority
    let has_authority = !components.host.is_empty() 
        || !components.username.is_empty() 
        || !components.scheme.is_empty();
    
    if has_authority {
        result.push_str("//");
        
        // Userinfo
        if !components.username.is_empty() || components.password.is_some() {
            result.push_str(&percent_encode(&components.username, is_userinfo_safe));
            if let Some(ref password) = components.password {
                result.push(':');
                result.push_str(&percent_encode(password, is_userinfo_safe));
            }
            result.push('@');
        }
        
        // Host
        if is_ipv6_address(&components.host) && !components.host.starts_with('[') {
            result.push('[');
            result.push_str(&components.host);
            result.push(']');
        } else if !components.raw_host.is_empty() {
            // Use raw_host for the URL string (ASCII/punycode)
            let host_str = if is_ipv6_address(&components.host) && !components.host.starts_with('[') {
                format!("[{}]", components.host)
            } else {
                String::from_utf8_lossy(&components.raw_host).to_string()
            };
            result.push_str(&host_str);
        }
        
        // Port (only if not default)
        if let Some(port) = components.port {
            let default_port = default_port_for_scheme(&components.scheme);
            if default_port != Some(port) {
                result.push(':');
                result.push_str(&port.to_string());
            }
        }
    }
    
    // Path
    let encoded_path = encode_path(&components.path);
    result.push_str(&encoded_path);
    
    // Query
    if !components.query.is_empty() {
        result.push('?');
        result.push_str(&String::from_utf8_lossy(&components.query));
    } else if components.has_trailing_question {
        result.push('?');
    }
    
    // Fragment
    if !components.fragment.is_empty() {
        result.push('#');
        result.push_str(&components.fragment);
    }
    
    result
}

// ============================================================================
// QueryParams Support
// ============================================================================

/// Encode query parameters in form-urlencoded format
fn encode_query_params(params: &[(String, String)]) -> String {
    params.iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                form_urlencode(k),
                form_urlencode(v)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Form URL encoding (spaces become +, etc.)
fn form_urlencode(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '*' {
            result.push(c);
        } else if c == ' ' {
            result.push('+');
        } else {
            for b in c.to_string().as_bytes() {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}

// ============================================================================
// PyO3 Implementation
// ============================================================================

#[pymethods]
impl URL {
    /// Create a new URL from a string or components
    #[new]
    #[pyo3(signature = (url=None, **kwargs))]
    fn new(url: Option<&Bound<'_, PyAny>>, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        // Handle component-based construction
        if let Some(kw) = kwargs {
            if !kw.is_empty() {
                return Self::from_components(url, kw);
            }
        }
        
        // Handle URL string or URL object
        if let Some(url_arg) = url {
            if let Ok(url_str) = url_arg.extract::<String>() {
                return Self::from_string(&url_str);
            }
            if let Ok(existing_url) = url_arg.extract::<URL>() {
                return Ok(existing_url);
            }
            return Err(PyTypeError::new_err(
                "URL() argument must be a string or URL instance"
            ));
        }
        
        // No arguments - create empty relative URL
        Ok(Self {
            components: UrlComponents::default(),
            url_string: String::new(),
        })
    }
    
    /// Get the scheme (e.g., "https")
    #[getter]
    fn scheme(&self) -> &str {
        &self.components.scheme
    }
    
    /// Get the host (decoded, e.g., "中国.icom.museum")
    #[getter]
    fn host(&self) -> &str {
        &self.components.host
    }
    
    /// Get the raw host (ASCII/punycode encoded)
    #[getter]
    fn raw_host<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.components.raw_host)
    }
    
    /// Get the port (None if default port for scheme)
    #[getter]
    fn port(&self) -> Option<u16> {
        self.components.port
    }
    
    /// Get the path (decoded)
    #[getter]
    fn path(&self) -> &str {
        &self.components.path
    }
    
    /// Get the raw path (encoded path + query as bytes)
    #[getter]
    fn raw_path<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.components.raw_path)
    }
    
    /// Get the query string as bytes (without leading '?')
    #[getter]
    fn query<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.components.query)
    }
    
    /// Get the fragment (without leading '#')
    #[getter]
    fn fragment(&self) -> &str {
        &self.components.fragment
    }
    
    /// Get userinfo (username:password) as bytes
    #[getter]
    fn userinfo<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let mut userinfo = String::new();
        if !self.components.username.is_empty() || self.components.password.is_some() {
            userinfo.push_str(&percent_encode(&self.components.username, is_userinfo_safe));
            if let Some(ref password) = self.components.password {
                userinfo.push(':');
                userinfo.push_str(&percent_encode(password, is_userinfo_safe));
            }
        }
        PyBytes::new(py, userinfo.as_bytes())
    }
    
    /// Get username (decoded)
    #[getter]
    fn username(&self) -> &str {
        &self.components.username
    }
    
    /// Get password (decoded)
    #[getter]
    fn password(&self) -> Option<&str> {
        self.components.password.as_deref()
    }
    
    /// Get netloc (host:port) as bytes
    #[getter]
    fn netloc<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let mut netloc = String::new();
        
        if is_ipv6_address(&self.components.host) && !self.components.host.starts_with('[') {
            netloc.push('[');
            netloc.push_str(&self.components.host);
            netloc.push(']');
        } else {
            netloc.push_str(&String::from_utf8_lossy(&self.components.raw_host));
        }
        
        if let Some(port) = self.components.port {
            netloc.push(':');
            netloc.push_str(&port.to_string());
        }
        
        PyBytes::new(py, netloc.as_bytes())
    }
    
    /// Get the origin (scheme + host + port)
    #[getter]
    fn origin(&self) -> String {
        let mut result = String::new();
        result.push_str(&self.components.scheme);
        result.push_str("://");
        
        if is_ipv6_address(&self.components.host) && !self.components.host.starts_with('[') {
            result.push('[');
            result.push_str(&self.components.host);
            result.push(']');
        } else {
            result.push_str(&String::from_utf8_lossy(&self.components.raw_host));
        }
        
        if let Some(port) = self.components.port {
            result.push(':');
            result.push_str(&port.to_string());
        }
        
        result
    }
    
    /// Check if URL is relative (no scheme)
    #[getter]
    fn is_relative_url(&self) -> bool {
        self.components.scheme.is_empty()
    }
    
    /// Check if URL is absolute (has scheme)
    #[getter]
    fn is_absolute_url(&self) -> bool {
        !self.components.scheme.is_empty()
    }
    
    /// Check if using default port for scheme
    #[getter]
    fn is_default_port(&self) -> bool {
        match default_port_for_scheme(&self.components.scheme) {
            Some(default) => self.components.port.map_or(true, |p| p == default),
            None => self.components.port.is_none(),
        }
    }
    
    /// Get query parameters as QueryParams object
    #[getter]
    fn params(&self, py: Python<'_>) -> PyResult<PyObject> {
        // Import QueryParams from the module
        let module = py.import("requestx")?;
        let query_params_class = module.getattr("QueryParams")?;
        
        let query_str = String::from_utf8_lossy(&self.components.query);
        query_params_class.call1((query_str.to_string(),))
            .map(|obj| obj.into())
    }
    
    /// Copy the URL with modifications
    #[pyo3(signature = (**kwargs))]
    fn copy_with(&self, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut new_components = self.components.clone();
        
        if let Some(kw) = kwargs {
            let valid_keys = [
                "scheme", "netloc", "path", "query", "fragment",
                "username", "password", "host", "port", "raw_path", "params"
            ];
            
            // Check for invalid keys
            for key in kw.keys() {
                let key_str: String = key.extract()?;
                if !valid_keys.contains(&key_str.as_str()) {
                    return Err(PyTypeError::new_err(format!(
                        "'{}' is an invalid keyword argument for copy_with()",
                        key_str
                    )));
                }
            }
            
            // Validate userinfo type
            if let Ok(Some(userinfo)) = kw.get_item("userinfo") {
                if userinfo.extract::<&PyBytes>().is_err() {
                    return Err(PyTypeError::new_err(
                        "'userinfo' is an invalid keyword argument for URL()"
                    ));
                }
            }
            
            // Apply scheme
            if let Ok(Some(scheme)) = kw.get_item("scheme") {
                let scheme_str: String = scheme.extract()?;
                // Validate scheme doesn't contain unexpected characters
                if scheme_str.contains("://") {
                    return Err(PyValueError::new_err("Invalid URL component 'scheme'"));
                }
                new_components.scheme = scheme_str.to_lowercase();
            }
            
            // Apply netloc (overrides host/port)
            if let Ok(Some(netloc)) = kw.get_item("netloc") {
                let netloc_bytes: &[u8] = netloc.extract()?;
                let netloc_str = std::str::from_utf8(netloc_bytes)
                    .map_err(|_| InvalidURL::new("Invalid netloc encoding"))?;
                parse_host_port(netloc_str, &mut new_components)
                    .map_err(|e| PyValueError::new_err(e.message))?;
            } else {
                // Apply individual components
                if let Ok(Some(host)) = kw.get_item("host") {
                    let host_str: String = host.extract()?;
                    // Handle IPv6 addresses
                    let host_str = host_str.trim_start_matches('[').trim_end_matches(']');
                    
                    if is_ipv6_address(host_str) {
                        new_components.host = host_str.to_lowercase();
                        new_components.raw_host = format!("[{}]", host_str.to_lowercase()).into_bytes();
                    } else {
                        let ascii_host = idna_encode(host_str)
                            .map_err(|e| PyValueError::new_err(e.message))?;
                        new_components.host = host_str.to_lowercase();
                        new_components.raw_host = ascii_host.into_bytes();
                    }
                }
                
                if let Ok(Some(port)) = kw.get_item("port") {
                    let port_val: Option<u16> = if port.is_none() {
                        None
                    } else {
                        Some(port.extract()?)
                    };
                    new_components.port = port_val;
                }
                
                if let Ok(Some(username)) = kw.get_item("username") {
                    new_components.username = username.extract()?;
                }
                
                if let Ok(Some(password)) = kw.get_item("password") {
                    new_components.password = Some(password.extract()?);
                }
            }
            
            // Apply raw_path (overrides path and query)
            if let Ok(Some(raw_path)) = kw.get_item("raw_path") {
                let raw_path_bytes: &[u8] = raw_path.extract()?;
                let raw_path_str = std::str::from_utf8(raw_path_bytes)
                    .map_err(|_| InvalidURL::new("Invalid raw_path encoding"))?;
                
                // Split into path and query
                if let Some(query_pos) = raw_path_str.find('?') {
                    let path_part = &raw_path_str[..query_pos];
                    let query_part = &raw_path_str[query_pos + 1..];
                    
                    new_components.path = percent_decode(path_part)
                        .map_err(|e| PyValueError::new_err(e.message))?;
                    new_components.query = query_part.as_bytes().to_vec();
                    new_components.has_trailing_question = true;
                } else {
                    new_components.path = percent_decode(raw_path_str)
                        .map_err(|e| PyValueError::new_err(e.message))?;
                    new_components.query = Vec::new();
                    new_components.has_trailing_question = false;
                }
                
                new_components.raw_path = raw_path_bytes.to_vec();
            } else {
                // Apply path
                if let Ok(Some(path)) = kw.get_item("path") {
                    let path_str: String = path.extract()?;
                    check_non_printable(&path_str, Some("path"))
                        .map_err(|e| PyValueError::new_err(e.message))?;
                    
                    if path_str.len() > MAX_COMPONENT_LENGTH {
                        return Err(PyValueError::new_err("URL component 'path' too long"));
                    }
                    
                    // Validate path for absolute URLs
                    let is_absolute = !new_components.scheme.is_empty() || !new_components.host.is_empty();
                    if is_absolute && !path_str.is_empty() && !path_str.starts_with('/') {
                        return Err(PyValueError::new_err(
                            "For absolute URLs, path must be empty or begin with '/'"
                        ));
                    }
                    
                    new_components.path = path_str;
                }
                
                // Apply query
                if let Ok(Some(query)) = kw.get_item("query") {
                    let query_bytes: &[u8] = query.extract()?;
                    new_components.query = query_bytes.to_vec();
                    new_components.has_trailing_question = true;
                }
                
                // Apply params (overrides query)
                if let Ok(Some(params)) = kw.get_item("params") {
                    let params_list = extract_params(params)?;
                    let query_str = encode_query_params(&params_list);
                    new_components.query = query_str.into_bytes();
                    new_components.has_trailing_question = !params_list.is_empty();
                }
            }
            
            // Apply fragment
            if let Ok(Some(fragment)) = kw.get_item("fragment") {
                new_components.fragment = fragment.extract()?;
            }
        }
        
        // Rebuild raw_path
        let encoded_path = encode_path(&new_components.path);
        let mut raw_path = encoded_path.into_bytes();
        if !new_components.query.is_empty() || new_components.has_trailing_question {
            raw_path.push(b'?');
            raw_path.extend_from_slice(&new_components.query);
        }
        new_components.raw_path = raw_path;
        
        let url_string = build_url_string(&new_components);
        
        Ok(Self {
            components: new_components,
            url_string,
        })
    }
    
    /// Join with another URL or path (RFC 3986 compliant)
    fn join(&self, url: &str) -> PyResult<Self> {
        // Parse the reference URL
        let reference = parse_url(url)
            .map_err(|e| PyValueError::new_err(e.message))?;
        
        let mut result = UrlComponents::default();
        
        if !reference.scheme.is_empty() {
            // Reference has scheme - use it directly
            result.scheme = reference.scheme;
            result.host = reference.host;
            result.raw_host = reference.raw_host;
            result.port = reference.port;
            result.username = reference.username;
            result.password = reference.password;
            result.path = remove_dot_segments(&reference.path);
            result.query = reference.query;
            result.has_trailing_question = reference.has_trailing_question;
        } else if !reference.host.is_empty() {
            // Reference has authority
            result.scheme = self.components.scheme.clone();
            result.host = reference.host;
            result.raw_host = reference.raw_host;
            result.port = reference.port;
            result.username = reference.username;
            result.password = reference.password;
            result.path = remove_dot_segments(&reference.path);
            result.query = reference.query;
            result.has_trailing_question = reference.has_trailing_question;
        } else if reference.path.is_empty() {
            // Reference has empty path
            result.scheme = self.components.scheme.clone();
            result.host = self.components.host.clone();
            result.raw_host = self.components.raw_host.clone();
            result.port = self.components.port;
            result.username = self.components.username.clone();
            result.password = self.components.password.clone();
            result.path = self.components.path.clone();
            
            if !reference.query.is_empty() || reference.has_trailing_question {
                result.query = reference.query;
                result.has_trailing_question = reference.has_trailing_question;
            } else {
                result.query = self.components.query.clone();
                result.has_trailing_question = self.components.has_trailing_question;
            }
        } else {
            result.scheme = self.components.scheme.clone();
            result.host = self.components.host.clone();
            result.raw_host = self.components.raw_host.clone();
            result.port = self.components.port;
            result.username = self.components.username.clone();
            result.password = self.components.password.clone();
            
            if reference.path.starts_with('/') {
                result.path = remove_dot_segments(&reference.path);
            } else {
                // Merge paths
                let merged = merge_paths(&self.components.path, &reference.path, !self.components.host.is_empty());
                result.path = remove_dot_segments(&merged);
            }
            
            result.query = reference.query;
            result.has_trailing_question = reference.has_trailing_question;
        }
        
        result.fragment = reference.fragment;
        
        // Rebuild raw_path
        let encoded_path = encode_path(&result.path);
        let mut raw_path = encoded_path.into_bytes();
        if !result.query.is_empty() || result.has_trailing_question {
            raw_path.push(b'?');
            raw_path.extend_from_slice(&result.query);
        }
        result.raw_path = raw_path;
        
        let url_string = build_url_string(&result);
        
        Ok(Self {
            components: result,
            url_string,
        })
    }
    
    /// Set a query parameter (returns new URL)
    fn copy_set_param(&self, key: &str, value: &str) -> PyResult<Self> {
        let mut params = self.parse_query_params();
        
        // Remove existing keys
        params.retain(|(k, _)| k != key);
        // Add new key-value
        params.push((key.to_string(), value.to_string()));
        
        let mut new_components = self.components.clone();
        let query_str = encode_query_params(&params);
        new_components.query = query_str.into_bytes();
        new_components.has_trailing_question = !params.is_empty();
        
        // Rebuild raw_path
        let encoded_path = encode_path(&new_components.path);
        let mut raw_path = encoded_path.into_bytes();
        if !new_components.query.is_empty() {
            raw_path.push(b'?');
            raw_path.extend_from_slice(&new_components.query);
        }
        new_components.raw_path = raw_path;
        
        let url_string = build_url_string(&new_components);
        
        Ok(Self {
            components: new_components,
            url_string,
        })
    }
    
    /// Add a query parameter (returns new URL)
    fn copy_add_param(&self, key: &str, value: &str) -> PyResult<Self> {
        let mut params = self.parse_query_params();
        params.push((key.to_string(), value.to_string()));
        
        let mut new_components = self.components.clone();
        let query_str = encode_query_params(&params);
        new_components.query = query_str.into_bytes();
        new_components.has_trailing_question = true;
        
        // Rebuild raw_path
        let encoded_path = encode_path(&new_components.path);
        let mut raw_path = encoded_path.into_bytes();
        if !new_components.query.is_empty() {
            raw_path.push(b'?');
            raw_path.extend_from_slice(&new_components.query);
        }
        new_components.raw_path = raw_path;
        
        let url_string = build_url_string(&new_components);
        
        Ok(Self {
            components: new_components,
            url_string,
        })
    }
    
    /// Remove a query parameter (returns new URL)
    fn copy_remove_param(&self, key: &str) -> PyResult<Self> {
        let mut params = self.parse_query_params();
        params.retain(|(k, _)| k != key);
        
        let mut new_components = self.components.clone();
        let query_str = encode_query_params(&params);
        new_components.query = query_str.into_bytes();
        new_components.has_trailing_question = false;
        
        // Rebuild raw_path
        let encoded_path = encode_path(&new_components.path);
        let mut raw_path = encoded_path.into_bytes();
        if !new_components.query.is_empty() {
            raw_path.push(b'?');
            raw_path.extend_from_slice(&new_components.query);
        }
        new_components.raw_path = raw_path;
        
        let url_string = build_url_string(&new_components);
        
        Ok(Self {
            components: new_components,
            url_string,
        })
    }
    
    /// Merge query parameters (returns new URL)
    fn copy_merge_params(&self, params: &Bound<'_, PyDict>) -> PyResult<Self> {
        let mut existing_params = self.parse_query_params();
        
        for (key, value) in params.iter() {
            let key_str: String = key.extract()?;
            let value_str: String = value.extract()?;
            existing_params.push((key_str, value_str));
        }
        
        let mut new_components = self.components.clone();
        let query_str = encode_query_params(&existing_params);
        new_components.query = query_str.into_bytes();
        new_components.has_trailing_question = !existing_params.is_empty();
        
        // Rebuild raw_path
        let encoded_path = encode_path(&new_components.path);
        let mut raw_path = encoded_path.into_bytes();
        if !new_components.query.is_empty() {
            raw_path.push(b'?');
            raw_path.extend_from_slice(&new_components.query);
        }
        new_components.raw_path = raw_path;
        
        let url_string = build_url_string(&new_components);
        
        Ok(Self {
            components: new_components,
            url_string,
        })
    }
    
    fn __str__(&self) -> &str {
        &self.url_string
    }
    
    fn __repr__(&self) -> String {
        format!("URL('{}')", self.url_string)
    }
    
    fn __hash__(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.url_string.hash(&mut hasher);
        hasher.finish()
    }
    
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other_url) = other.extract::<URL>() {
            self.url_string == other_url.url_string
        } else if let Ok(other_str) = other.extract::<String>() {
            self.url_string == other_str
        } else {
            false
        }
    }
    
    fn __ne__(&self, other: &Bound<'_, PyAny>) -> bool {
        !self.__eq__(other)
    }
    
    fn __lt__(&self, other: &URL) -> bool {
        self.url_string < other.url_string
    }
    
    fn __le__(&self, other: &URL) -> bool {
        self.url_string <= other.url_string
    }
    
    fn __gt__(&self, other: &URL) -> bool {
        self.url_string > other.url_string
    }
    
    fn __ge__(&self, other: &URL) -> bool {
        self.url_string >= other.url_string
    }
}

impl URL {
    /// Create URL from string
    fn from_string(url: &str) -> PyResult<Self> {
        let components = parse_url(url)
            .map_err(|e| PyValueError::new_err(e.message))?;
        let url_string = build_url_string(&components);
        
        Ok(Self {
            components,
            url_string,
        })
    }
    
    /// Create URL from components
    fn from_components(url: Option<&Bound<'_, PyAny>>, kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let valid_keys = [
            "scheme", "host", "port", "path", "query", "fragment",
            "username", "password", "params"
        ];
        
        // Check for invalid keys
        for key in kwargs.keys() {
            let key_str: String = key.extract()?;
            if !valid_keys.contains(&key_str.as_str()) {
                return Err(PyTypeError::new_err(format!(
                    "'{}' is an invalid keyword argument for URL()",
                    key_str
                )));
            }
        }
        
        // Start with base URL if provided
        let mut components = if let Some(url_arg) = url {
            let url_str: String = url_arg.extract()?;
            parse_url(&url_str)
                .map_err(|e| PyValueError::new_err(e.message))?
        } else {
            UrlComponents::default()
        };
        
        // Apply components from kwargs
        if let Ok(Some(scheme)) = kwargs.get_item("scheme") {
            let scheme_str: String = scheme.extract()?;
            if !scheme_str.is_empty() && !is_valid_scheme(&scheme_str) {
                return Err(PyValueError::new_err("Invalid URL component 'scheme'"));
            }
            components.scheme = scheme_str.to_lowercase();
        }
        
        if let Ok(Some(host)) = kwargs.get_item("host") {
            let host_str: String = host.extract()?;
            let host_str = host_str.trim_start_matches('[').trim_end_matches(']');
            
            if is_ipv6_address(host_str) {
                let _ = parse_ipv6(host_str)
                    .map_err(|e| PyValueError::new_err(e.message))?;
                components.host = host_str.to_lowercase();
                components.raw_host = format!("[{}]", host_str.to_lowercase()).into_bytes();
            } else {
                let ascii_host = idna_encode(host_str)
                    .map_err(|e| PyValueError::new_err(e.message))?;
                components.host = host_str.to_lowercase();
                components.raw_host = ascii_host.into_bytes();
            }
        }
        
        if let Ok(Some(port)) = kwargs.get_item("port") {
            let port_val: Option<u16> = if port.is_none() {
                None
            } else {
                Some(port.extract()?)
            };
            components.port = port_val;
        }
        
        if let Ok(Some(path)) = kwargs.get_item("path") {
            let path_str: String = path.extract()?;
            
            check_non_printable(&path_str, Some("path"))
                .map_err(|e| PyValueError::new_err(e.message))?;
            
            if path_str.len() > MAX_COMPONENT_LENGTH {
                return Err(PyValueError::new_err("URL component 'path' too long"));
            }
            
            // Validate path
            let is_absolute = !components.scheme.is_empty() || !components.host.is_empty();
            
            if is_absolute && !path_str.is_empty() && !path_str.starts_with('/') {
                return Err(PyValueError::new_err(
                    "For absolute URLs, path must be empty or begin with '/'"
                ));
            }
            
            if !is_absolute {
                if path_str.starts_with("//") {
                    return Err(PyValueError::new_err(
                        "Relative URLs cannot have a path starting with '//'"
                    ));
                }
                if path_str.starts_with(':') {
                    return Err(PyValueError::new_err(
                        "Relative URLs cannot have a path starting with ':'"
                    ));
                }
            }
            
            components.path = path_str;
        }
        
        if let Ok(Some(query)) = kwargs.get_item("query") {
            let query_bytes: &[u8] = query.extract()?;
            components.query = query_bytes.to_vec();
            components.has_trailing_question = true;
        }
        
        if let Ok(Some(params)) = kwargs.get_item("params") {
            let params_list = extract_params(&params)?;
            let query_str = encode_query_params(&params_list);
            components.query = query_str.into_bytes();
            components.has_trailing_question = !params_list.is_empty();
        }
        
        if let Ok(Some(fragment)) = kwargs.get_item("fragment") {
            components.fragment = fragment.extract()?;
        }
        
        if let Ok(Some(username)) = kwargs.get_item("username") {
            components.username = username.extract()?;
        }
        
        if let Ok(Some(password)) = kwargs.get_item("password") {
            components.password = Some(password.extract()?);
        }
        
        // Ensure path defaults to / for absolute URLs
        if (!components.scheme.is_empty() || !components.host.is_empty()) && components.path.is_empty() {
            components.path = "/".to_string();
        }
        
        // Build raw_path
        let encoded_path = encode_path(&components.path);
        let mut raw_path = encoded_path.into_bytes();
        if !components.query.is_empty() || components.has_trailing_question {
            raw_path.push(b'?');
            raw_path.extend_from_slice(&components.query);
        }
        components.raw_path = raw_path;
        
        let url_string = build_url_string(&components);
        
        Ok(Self {
            components,
            url_string,
        })
    }
    
    /// Parse query string into key-value pairs
    fn parse_query_params(&self) -> Vec<(String, String)> {
        let query_str = String::from_utf8_lossy(&self.components.query);
        if query_str.is_empty() {
            return Vec::new();
        }
        
        query_str
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or("");
                Some((
                    form_urldecode(key),
                    form_urldecode(value),
                ))
            })
            .collect()
    }
}

/// Decode form-urlencoded string
fn form_urldecode(s: &str) -> String {
    let s = s.replace('+', " ");
    percent_decode(&s).unwrap_or(s)
}

/// Extract params from various Python types
fn extract_params(params: &Bound<'_, PyAny>) -> PyResult<Vec<(String, String)>> {
    let mut result = Vec::new();
    
    if let Ok(dict) = params.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            result.push((key.extract()?, value.extract()?));
        }
    } else if let Ok(query_params) = params.getattr("items") {
        // QueryParams-like object
        let items = query_params.call0()?;
        for item in items.iter()? {
            let item = item?;
            let tuple: (&str, &str) = item.extract()?;
            result.push((tuple.0.to_string(), tuple.1.to_string()));
        }
    } else if let Ok(s) = params.extract::<String>() {
        // Parse query string
        for pair in s.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let Some(key) = parts.next() {
                let value = parts.next().unwrap_or("");
                result.push((key.to_string(), value.to_string()));
            }
        }
    }
    
    Ok(result)
}

/// Remove dot segments from path (RFC 3986)
fn remove_dot_segments(path: &str) -> String {
    let mut output: Vec<&str> = Vec::new();
    
    for segment in path.split('/') {
        match segment {
            "." => {}
            ".." => {
                output.pop();
            }
            s => {
                output.push(s);
            }
        }
    }
    
    let mut result = output.join("/");
    
    if path.starts_with('/') && !result.starts_with('/') {
        result.insert(0, '/');
    }
    
    if path.ends_with('/') && !result.ends_with('/') {
        result.push('/');
    }
    
    result
}

/// Merge base and reference paths (RFC 3986)
fn merge_paths(base: &str, reference: &str, has_authority: bool) -> String {
    if has_authority && base.is_empty() {
        format!("/{}", reference)
    } else if let Some(last_slash) = base.rfind('/') {
        format!("{}{}", &base[..=last_slash], reference)
    } else {
        reference.to_string()
    }
}

// ============================================================================
// InvalidURL Exception
// ============================================================================

/// Python exception for invalid URLs
#[pyclass(extends=pyo3::exceptions::PyValueError)]
pub struct InvalidURLError {
    #[pyo3(get)]
    message: String,
}

#[pymethods]
impl InvalidURLError {
    #[new]
    fn new(message: String) -> (Self, pyo3::exceptions::PyValueError) {
        let err = pyo3::exceptions::PyValueError::new_err(message.clone());
        (Self { message }, err.into())
    }
    
    fn __str__(&self) -> &str {
        &self.message
    }
    
    fn __repr__(&self) -> String {
        format!("InvalidURL('{}')", self.message)
    }
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register the URL module
pub fn register_url_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<URL>()?;
    
    // Create InvalidURL as a subclass of ValueError
    let py = m.py();
    let invalid_url = py.get_type::<InvalidURLError>();
    m.add("InvalidURL", invalid_url)?;
    
    Ok(())
}
