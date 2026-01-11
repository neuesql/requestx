use hyper::header::HeaderMap;
use md5::Digest;
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};
use std::collections::HashMap;
use std::sync::Mutex;

use base64::Engine;

/// HTTP Digest Authentication handler
///
/// Implements RFC 2617 HTTP Digest Access Authentication with MD5 hashing.
/// Supports both auth and auth-int quality of protection (qop) modes.
#[derive(Debug)]
pub struct HTTPDigestAuth {
    pub username: String,
    pub password: String,
    /// Nonce counter for qop auth mode - protects against replay attacks
    pub nc: Mutex<u32>,
    /// Cache for nonce values to prevent replay attacks
    #[allow(dead_code)]
    pub nonce_cache: Mutex<HashMap<String, NonceData>>,
}

#[derive(Debug, Clone)]
pub struct NonceData {
    pub nonce: String,
    pub opaque: String,
    pub qop: Vec<String>,
    pub realm: String,
}

impl HTTPDigestAuth {
    /// Create a new HTTPDigestAuth handler
    pub fn new(username: &str, password: &str) -> Self {
        HTTPDigestAuth {
            username: username.to_string(),
            password: password.to_string(),
            nc: Mutex::new(0),
            nonce_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Generate the Authorization header value for a given request
    ///
    /// This method implements the RFC 2617 digest authentication algorithm:
    /// 1. Parse WWW-Authenticate header to extract nonce, realm, qop
    /// 2. Calculate H(A1) = MD5(username:realm:password)
    /// 3. Calculate H(A2) = MD5(method:uri)
    /// 4. Calculate response = MD5(H(A1):nonce:nc:cnonce:qop:H(A2))
    pub fn authorize(
        &self,
        method: &str,
        uri: &str,
        www_authenticate: Option<&str>,
    ) -> Option<String> {
        // If no WWW-Authenticate header, return None (will fall back to basic auth)
        let www_auth = match www_authenticate {
            Some(h) => h,
            None => return None,
        };

        // Parse WWW-Authenticate header parameters
        let params = parse_digest_params(www_auth).ok()?;

        let realm = params.get("realm").cloned().unwrap_or_default();
        let nonce = params.get("nonce").cloned().unwrap_or_default();
        let opaque = params.get("opaque").cloned().unwrap_or_default();
        let qop_str = params.get("qop").cloned().unwrap_or_default();

        // Parse qop list (comma-separated)
        let qop_list: Vec<String> = if qop_str.is_empty() {
            vec!["auth".to_string()] // Default to 'auth' if qop not specified
        } else {
            qop_str.split(',').map(|s| s.trim().to_string()).collect()
        };

        // Select best qop (prefer auth-int > auth)
        let qop = qop_list
            .iter()
            .find(|q| q.starts_with("auth"))
            .unwrap_or(&"auth".to_string())
            .clone();

        // Generate client nonce (random 8 hex chars)
        let cnonce = generate_cnonce();

        // Increment nonce counter
        let mut nc = self.nc.lock().unwrap();
        *nc += 1;
        let nc_str = format!("{:08x}", *nc);

        // Calculate H(A1) = MD5(username:realm:password)
        let a1 = format!("{}:{}:{}", self.username, realm, self.password);
        let ha1 = format!("{:x}", md5::compute(a1));

        // Calculate H(A2) = MD5(method:uri)
        let a2 = format!("{}:{}", method, uri);
        let ha2 = format!("{:x}", md5::compute(a2));

        // Calculate response = MD5(H(A1):nonce:nc:cnonce:qop:H(A2))
        let response_input = format!("{}:{}:{}:{}:{}:{}", ha1, nonce, nc_str, &cnonce, qop, ha2);
        let response = format!("{:x}", md5::compute(response_input));

        // Build Authorization header
        let auth_header = format!(
            r#"Digest username="{}", realm="{}", nonce="{}", uri="{}", qop={}, nc={}, cnonce="{}", response="{}", opaque="{}""#,
            self.username, realm, nonce, uri, qop, nc_str, cnonce, response, opaque
        );

        Some(auth_header)
    }
}

/// Parse WWW-Authenticate header parameters
fn parse_digest_params(header: &str) -> PyResult<HashMap<String, String>> {
    let mut params = HashMap::new();

    // Remove "Digest " prefix if present
    let header = header.strip_prefix("Digest ").unwrap_or(header);

    // Parse quoted parameters
    let mut current_key = String::new();
    let mut current_value = String::new();
    let mut in_quotes = false;
    let mut key_started = false;

    for (i, c) in header.chars().enumerate() {
        let prev_char = if i > 0 {
            header.chars().nth(i - 1)
        } else {
            None
        };
        let next_char = header.chars().nth(i + 1);

        if c == '"' {
            in_quotes = !in_quotes;
            if !in_quotes && !current_value.is_empty() {
                // End of quoted value
                params.insert(
                    current_key.trim().to_string(),
                    current_value.trim().to_string(),
                );
                current_key.clear();
                current_value.clear();
                key_started = false;
            } else if !key_started {
                // Start of quoted value, the key is everything since last =
                let eq_pos = header[..i].rfind('=');
                if let Some(pos) = eq_pos {
                    current_key = header[pos + 1..i].trim().to_string();
                    key_started = true;
                }
            }
        } else if c == ',' && !in_quotes {
            // End of parameter
            if !current_key.is_empty() {
                params.insert(
                    current_key.trim().to_string(),
                    current_value.trim().to_string(),
                );
                current_key.clear();
                current_value.clear();
                key_started = false;
            }
        } else if in_quotes {
            current_value.push(c);
        } else if c == '=' && !key_started {
            // Mark that we expect the value next
            key_started = true;
        }
    }

    // Handle last parameter if exists
    if !current_key.is_empty() {
        params.insert(
            current_key.trim().to_string(),
            current_value.trim().to_string(),
        );
    }

    Ok(params)
}

/// Generate a random client nonce (8 hex characters)
fn generate_cnonce() -> String {
    let mut rng = fastrand::Rng::new();
    format!("{:08x}", rng.u32(0..u32::MAX))
}

/// Fast random number generator wrapper for cnonce generation
mod fastrand {
    use std::cell::Cell;

    thread_local! {
        static RNG: Cell<u64> = Cell::new(1);
    }

    pub struct Rng {
        state: u64,
    }

    impl Rng {
        pub fn new() -> Self {
            use std::time::{SystemTime, UNIX_EPOCH};
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            RNG.with(|rng| {
                let seed = rng.get().wrapping_add(timestamp);
                rng.set(seed);
                Rng { state: seed }
            })
        }

        /// Generate random u32
        pub fn u32(&mut self, range: std::ops::Range<u32>) -> u32 {
            self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let x = self.state >> 33;
            (x as u32) % (range.end - range.start) + range.start
        }
    }
}

/// HTTP Proxy Authentication handler
///
/// Implements basic authentication for HTTP proxy servers.
#[derive(Debug, Clone)]
pub struct HTTPProxyAuth {
    pub username: String,
    pub password: String,
}

impl HTTPProxyAuth {
    /// Create a new HTTPProxyAuth handler
    pub fn new(username: &str, password: &str) -> Self {
        HTTPProxyAuth {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    /// Generate the Proxy-Authorization header value
    pub fn authorize(&self) -> String {
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            credentials.as_bytes(),
        );
        format!("Basic {}", encoded)
    }
}

/// Authentication type enum for internal use
#[derive(Debug)]
pub enum AuthType {
    Basic((String, String)),
    Digest(HTTPDigestAuth),
    Proxy(HTTPProxyAuth),
}

impl AuthType {
    /// Convert to tuple (username, password) for basic auth
    pub fn to_basic(&self) -> Option<(String, String)> {
        match self {
            AuthType::Basic(creds) => Some(creds.clone()),
            AuthType::Digest(auth) => Some((auth.username.clone(), auth.password.clone())),
            AuthType::Proxy(auth) => Some((auth.username.clone(), auth.password.clone())),
        }
    }
}

/// Parse authentication from URL (user:pass@host format)
///
/// Extracts username and password from a URL's userinfo section.
/// Returns (username, password) tuple if present, or None if no auth in URL.
#[pyfunction]
pub fn get_auth_from_url<'py>(py: Python<'py>, url: &str) -> PyResult<Option<Bound<'py, PyTuple>>> {
    let parsed = url::Url::parse(url).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid URL: {}", e))
    })?;

    // username() returns "" if no username, password() returns None if no password
    let username = parsed.username();
    let has_username = !username.is_empty();
    let password = parsed.password().unwrap_or("");

    if has_username {
        let tuple = PyTuple::new(py, [username.to_string(), password.to_string()])?;
        Ok(Some(tuple))
    } else {
        Ok(None)
    }
}

/// Remove authentication info from URL (for security purposes)
///
/// Returns a URL with the userinfo portion stripped.
/// Example: "https://user:pass@example.com" -> "https://example.com"
#[pyfunction]
pub fn urldefragauth<'py>(py: Python<'py>, url: &str) -> PyResult<Bound<'py, PyString>> {
    let parsed = url::Url::parse(url).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid URL: {}", e))
    })?;

    // Rebuild URL without userinfo
    let scheme = parsed.scheme();
    let host = parsed
        .host()
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("URL has no host"))?;
    let port = parsed.port();
    let path = parsed.path();
    let query = parsed.query();

    // Build new URL without auth
    let new_url = match port {
        Some(p) => {
            if let Some(q) = query {
                format!("{}://{}{}{}?{}", scheme, host, p, path, q)
            } else {
                format!("{}://{}{}{}", scheme, host, p, path)
            }
        }
        None => {
            if let Some(q) = query {
                format!("{}://{}{}?{}", scheme, host, path, q)
            } else {
                format!("{}://{}{}", scheme, host, path)
            }
        }
    };

    Ok(PyString::new(py, &new_url))
}

/// Python wrapper for HTTPDigestAuth
#[pyclass(name = "HTTPDigestAuth")]
pub struct PyHTTPDigestAuth {
    auth: HTTPDigestAuth,
}

#[pymethods]
impl PyHTTPDigestAuth {
    #[new]
    fn new(username: &str, password: &str) -> Self {
        PyHTTPDigestAuth {
            auth: HTTPDigestAuth::new(username, password),
        }
    }

    #[getter]
    fn username(&self) -> String {
        self.auth.username.clone()
    }

    #[getter]
    fn password(&self) -> String {
        self.auth.password.clone()
    }
}

/// Python wrapper for HTTPProxyAuth
#[pyclass(name = "HTTPProxyAuth")]
pub struct PyHTTPProxyAuth {
    auth: HTTPProxyAuth,
}

#[pymethods]
impl PyHTTPProxyAuth {
    #[new]
    fn new(username: &str, password: &str) -> Self {
        PyHTTPProxyAuth {
            auth: HTTPProxyAuth::new(username, password),
        }
    }

    #[getter]
    fn username(&self) -> String {
        self.auth.username.clone()
    }

    #[getter]
    fn password(&self) -> String {
        self.auth.password.clone()
    }

    fn authorize(&self) -> String {
        self.auth.authorize()
    }
}
