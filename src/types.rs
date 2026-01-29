//! Additional types: streams, auth, status codes

use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Synchronous byte stream base class
/// Implements both sync (__iter__/__next__) and async (__aiter__/__anext__) iteration
#[pyclass(name = "SyncByteStream", subclass)]
#[derive(Clone, Debug, Default)]
pub struct SyncByteStream {
    data: Vec<u8>,
    position: usize,
}

impl SyncByteStream {
    /// Create a new SyncByteStream with the given data
    pub fn from_data(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }
}

#[pymethods]
impl SyncByteStream {
    #[new]
    fn new() -> Self {
        Self { data: Vec::new(), position: 0 }
    }

    // Sync iteration
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<Vec<u8>> {
        if self.position >= self.data.len() {
            None
        } else {
            let chunk = self.data[self.position..].to_vec();
            self.position = self.data.len();
            Some(chunk)
        }
    }

    // Async iteration - uses coroutine to return awaitable
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Import asyncio and create a completed Future with the result
        let asyncio = py.import("asyncio")?;
        let loop_fn = asyncio.getattr("get_running_loop")?;
        let event_loop = loop_fn.call0()?;
        let future = event_loop.call_method0("create_future")?;

        if self.position >= self.data.len() {
            // Signal StopAsyncIteration by setting exception on the future
            let stop_async_iter = py.import("builtins")?.getattr("StopAsyncIteration")?.call0()?;
            future.call_method1("set_exception", (stop_async_iter,))?;
        } else {
            let chunk = PyBytes::new(py, &self.data[self.position..]);
            self.position = self.data.len();
            future.call_method1("set_result", (chunk,))?;
        }

        Ok(future)
    }

    fn read(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn close(&mut self) {
        self.data.clear();
        self.position = 0;
    }

    fn aread<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    fn aclose(&mut self) {
        self.data.clear();
        self.position = 0;
    }
}

/// Asynchronous byte stream base class - subclass of SyncByteStream
#[pyclass(name = "AsyncByteStream", extends = SyncByteStream, subclass)]
#[derive(Clone, Debug, Default)]
pub struct AsyncByteStream;

impl AsyncByteStream {
    /// Create an AsyncByteStream with data
    pub fn from_data(data: Vec<u8>) -> (Self, SyncByteStream) {
        (AsyncByteStream, SyncByteStream::from_data(data))
    }
}

#[pymethods]
impl AsyncByteStream {
    #[new]
    fn new() -> (Self, SyncByteStream) {
        (AsyncByteStream, SyncByteStream::new())
    }
}

/// Basic authentication
#[pyclass(name = "BasicAuth")]
#[derive(Clone, Debug)]
pub struct BasicAuth {
    #[pyo3(get)]
    pub username: String,
    #[pyo3(get)]
    pub password: String,
}

#[pymethods]
impl BasicAuth {
    #[new]
    #[pyo3(signature = (username, password=""))]
    fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    /// Get the Authorization header value for Basic auth
    fn build_auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64_encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// Sync auth flow - returns a generator that yields the authenticated request
    fn sync_auth_flow(&self, request: crate::request::Request) -> BasicAuthFlow {
        BasicAuthFlow {
            auth_header: self.build_auth_header(),
            request: Some(request),
        }
    }

    /// Async auth flow - same as sync for Basic auth
    fn async_auth_flow(&self, request: crate::request::Request) -> BasicAuthFlow {
        BasicAuthFlow {
            auth_header: self.build_auth_header(),
            request: Some(request),
        }
    }

    fn __repr__(&self) -> String {
        format!("BasicAuth(username={:?}, password=***)", self.username)
    }

    fn __eq__(&self, other: &BasicAuth) -> bool {
        self.username == other.username && self.password == other.password
    }
}

/// Generator for Basic auth flow
#[pyclass]
pub struct BasicAuthFlow {
    auth_header: String,
    request: Option<crate::request::Request>,
}

#[pymethods]
impl BasicAuthFlow {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<crate::request::Request> {
        if let Some(mut request) = self.request.take() {
            request.headers_mut().set("Authorization".to_string(), self.auth_header.clone());
            Some(request)
        } else {
            None
        }
    }

    fn send(&mut self, _response: &Bound<'_, PyAny>) -> PyResult<crate::request::Request> {
        // For Basic auth, we don't need to handle responses
        // The request is already done - raise StopIteration
        Err(pyo3::exceptions::PyStopIteration::new_err(()))
    }
}

/// Simple base64 encoding for auth
fn base64_encode(input: &[u8]) -> String {
    use std::fmt::Write;
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        result.push(ALPHABET[(b0 >> 2) as usize] as char);
        result.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if i + 1 < input.len() {
            result.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if i + 2 < input.len() {
            result.push(ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }
    result
}

/// Digest auth challenge parsed from WWW-Authenticate header
#[derive(Clone, Debug)]
pub struct DigestAuthChallenge {
    pub realm: Vec<u8>,
    pub nonce: Vec<u8>,
    pub algorithm: String,
    pub opaque: Option<Vec<u8>>,
    pub qop: Option<Vec<u8>>,
}

/// Digest authentication
#[pyclass(name = "DigestAuth")]
pub struct DigestAuth {
    #[pyo3(get)]
    pub username: String,
    #[pyo3(get)]
    pub password: String,
    last_challenge: Option<DigestAuthChallenge>,
    nonce_count: u32,
}

impl Clone for DigestAuth {
    fn clone(&self) -> Self {
        Self {
            username: self.username.clone(),
            password: self.password.clone(),
            last_challenge: self.last_challenge.clone(),
            nonce_count: self.nonce_count,
        }
    }
}

impl std::fmt::Debug for DigestAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DigestAuth")
            .field("username", &self.username)
            .field("password", &"***")
            .finish()
    }
}

#[pymethods]
impl DigestAuth {
    #[new]
    fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
            last_challenge: None,
            nonce_count: 1,
        }
    }

    /// Sync auth flow - returns a generator that handles digest auth
    fn sync_auth_flow(&mut self, request: crate::request::Request) -> DigestAuthFlow {
        DigestAuthFlow::new(
            self.username.as_bytes().to_vec(),
            self.password.as_bytes().to_vec(),
            self.last_challenge.take(),
            self.nonce_count,
            request,
        )
    }

    /// Async auth flow - same implementation for DigestAuth
    fn async_auth_flow(&mut self, request: crate::request::Request) -> DigestAuthFlow {
        self.sync_auth_flow(request)
    }

    /// Get client nonce - exposed for testing
    fn _get_client_nonce(&self, nonce_count: u32, nonce: &[u8]) -> Vec<u8> {
        DigestAuthFlow::generate_client_nonce(nonce_count, nonce)
    }

    fn __repr__(&self) -> String {
        format!("DigestAuth(username={:?}, password=***)", self.username)
    }
}

/// Generator for Digest auth flow
#[pyclass]
pub struct DigestAuthFlow {
    username: Vec<u8>,
    password: Vec<u8>,
    last_challenge: Option<DigestAuthChallenge>,
    nonce_count: u32,
    request: Option<crate::request::Request>,
    state: DigestFlowState,
    auth_header: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum DigestFlowState {
    Initial,
    WaitingForResponse,
    SentAuthRequest,
    Done,
}

impl DigestAuthFlow {
    fn new(
        username: Vec<u8>,
        password: Vec<u8>,
        last_challenge: Option<DigestAuthChallenge>,
        nonce_count: u32,
        request: crate::request::Request,
    ) -> Self {
        Self {
            username,
            password,
            last_challenge,
            nonce_count,
            request: Some(request),
            state: DigestFlowState::Initial,
            auth_header: None,
        }
    }

    fn generate_client_nonce(nonce_count: u32, nonce: &[u8]) -> Vec<u8> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut data = Vec::new();
        data.extend_from_slice(nonce_count.to_string().as_bytes());
        data.extend_from_slice(nonce);

        // Add timestamp
        if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
            data.extend_from_slice(duration.as_secs().to_string().as_bytes());
        }

        // Add some random bytes (simplified - use timestamp as seed)
        let random_bytes: [u8; 8] = [
            ((nonce_count * 17) % 256) as u8,
            ((nonce_count * 31) % 256) as u8,
            ((nonce_count * 47) % 256) as u8,
            ((nonce_count * 61) % 256) as u8,
            ((nonce_count * 79) % 256) as u8,
            ((nonce_count * 97) % 256) as u8,
            ((nonce_count * 113) % 256) as u8,
            ((nonce_count * 127) % 256) as u8,
        ];
        data.extend_from_slice(&random_bytes);

        // SHA1 hash and take first 16 hex chars
        let hash = sha1_hash(&data);
        hash[..16].as_bytes().to_vec()
    }

    fn parse_challenge(auth_header: &str) -> Option<DigestAuthChallenge> {
        // Parse "Digest realm="xxx", nonce="yyy", ..."
        let header = auth_header.strip_prefix("Digest ")
            .or_else(|| auth_header.strip_prefix("digest "))?;

        let mut realm: Option<Vec<u8>> = None;
        let mut nonce: Option<Vec<u8>> = None;
        let mut algorithm = "MD5".to_string();
        let mut opaque: Option<Vec<u8>> = None;
        let mut qop: Option<Vec<u8>> = None;

        // Simple parser for key=value pairs
        for part in parse_http_list(header) {
            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim();
                let value = unquote(value.trim());
                match key.to_lowercase().as_str() {
                    "realm" => realm = Some(value.as_bytes().to_vec()),
                    "nonce" => nonce = Some(value.as_bytes().to_vec()),
                    "algorithm" => algorithm = value,
                    "opaque" => opaque = Some(value.as_bytes().to_vec()),
                    "qop" => qop = Some(value.as_bytes().to_vec()),
                    _ => {}
                }
            }
        }

        Some(DigestAuthChallenge {
            realm: realm?,
            nonce: nonce?,
            algorithm,
            opaque,
            qop,
        })
    }

    fn build_auth_header(&mut self, request: &crate::request::Request, challenge: &DigestAuthChallenge) -> String {
        let hash_func: fn(&[u8]) -> String = match challenge.algorithm.to_uppercase().as_str() {
            "MD5" | "MD5-SESS" => md5_hash,
            "SHA" | "SHA-SESS" => sha1_hash,
            "SHA-256" | "SHA-256-SESS" => sha256_hash,
            "SHA-512" | "SHA-512-SESS" => sha512_hash,
            _ => md5_hash,
        };

        // A1 = username:realm:password
        let mut a1 = Vec::new();
        a1.extend_from_slice(&self.username);
        a1.push(b':');
        a1.extend_from_slice(&challenge.realm);
        a1.push(b':');
        a1.extend_from_slice(&self.password);

        // Get path from request URL
        let path = request.url_ref().raw_path();

        // A2 = method:uri
        let mut a2 = Vec::new();
        a2.extend_from_slice(request.method().as_bytes());
        a2.push(b':');
        a2.extend_from_slice(path.as_bytes());

        let ha2 = hash_func(&a2);

        let nc_value = format!("{:08x}", self.nonce_count);
        let cnonce = Self::generate_client_nonce(self.nonce_count, &challenge.nonce);
        self.nonce_count += 1;

        let mut ha1 = hash_func(&a1);

        // Handle -SESS algorithms
        if challenge.algorithm.to_uppercase().ends_with("-SESS") {
            let mut sess_data = Vec::new();
            sess_data.extend_from_slice(ha1.as_bytes());
            sess_data.push(b':');
            sess_data.extend_from_slice(&challenge.nonce);
            sess_data.push(b':');
            sess_data.extend_from_slice(&cnonce);
            ha1 = hash_func(&sess_data);
        }

        // Resolve QOP
        let qop = self.resolve_qop(challenge.qop.as_deref());

        // Build response digest
        let response = if qop.is_none() {
            // RFC 2069
            let mut digest_data = Vec::new();
            digest_data.extend_from_slice(ha1.as_bytes());
            digest_data.push(b':');
            digest_data.extend_from_slice(&challenge.nonce);
            digest_data.push(b':');
            digest_data.extend_from_slice(ha2.as_bytes());
            hash_func(&digest_data)
        } else {
            // RFC 2617/7616
            let mut digest_data = Vec::new();
            digest_data.extend_from_slice(ha1.as_bytes());
            digest_data.push(b':');
            digest_data.extend_from_slice(&challenge.nonce);
            digest_data.push(b':');
            digest_data.extend_from_slice(nc_value.as_bytes());
            digest_data.push(b':');
            digest_data.extend_from_slice(&cnonce);
            digest_data.push(b':');
            digest_data.extend_from_slice(b"auth");
            digest_data.push(b':');
            digest_data.extend_from_slice(ha2.as_bytes());
            hash_func(&digest_data)
        };

        // Build header value
        let mut parts = vec![
            format!("username=\"{}\"", String::from_utf8_lossy(&self.username)),
            format!("realm=\"{}\"", String::from_utf8_lossy(&challenge.realm)),
            format!("nonce=\"{}\"", String::from_utf8_lossy(&challenge.nonce)),
            format!("uri=\"{}\"", path),
            format!("response=\"{}\"", response),
            format!("algorithm={}", challenge.algorithm),
        ];

        if let Some(ref opaque) = challenge.opaque {
            parts.push(format!("opaque=\"{}\"", String::from_utf8_lossy(opaque)));
        }

        if qop.is_some() {
            parts.push("qop=auth".to_string());
            parts.push(format!("nc={}", nc_value));
            parts.push(format!("cnonce=\"{}\"", String::from_utf8_lossy(&cnonce)));
        }

        format!("Digest {}", parts.join(", "))
    }

    fn resolve_qop(&self, qop: Option<&[u8]>) -> Option<Vec<u8>> {
        match qop {
            None => None,
            Some(q) => {
                // Split on comma and check for "auth"
                let qop_str = String::from_utf8_lossy(q);
                for part in qop_str.split(',') {
                    if part.trim() == "auth" {
                        return Some(b"auth".to_vec());
                    }
                }
                // If only auth-int is available, we don't support it yet
                None
            }
        }
    }
}

#[pymethods]
impl DigestAuthFlow {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<crate::request::Request> {
        match self.state {
            DigestFlowState::Initial => {
                if let Some(mut request) = self.request.take() {
                    // If we have a last challenge, add auth header
                    if let Some(ref challenge) = self.last_challenge.clone() {
                        let auth_header = self.build_auth_header(&request, challenge);
                        request.headers_mut().set("Authorization".to_string(), auth_header);
                    }
                    self.state = DigestFlowState::WaitingForResponse;
                    self.request = Some(request.clone());
                    Some(request)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn send(&mut self, response: &Bound<'_, PyAny>) -> PyResult<crate::request::Request> {
        match self.state {
            DigestFlowState::WaitingForResponse => {
                // Get status code from response
                let status_code: u16 = response.getattr("status_code")?.extract()?;

                if status_code != 401 {
                    // Not a 401, we're done
                    self.state = DigestFlowState::Done;
                    return Err(pyo3::exceptions::PyStopIteration::new_err(()));
                }

                // Check for WWW-Authenticate header
                let headers = response.getattr("headers")?;

                // Try to get www-authenticate header
                let auth_header: Option<String> = if let Ok(h) = headers.call_method1("get", ("www-authenticate",)) {
                    h.extract().ok()
                } else {
                    None
                };

                let auth_header = match auth_header {
                    Some(h) if h.to_lowercase().starts_with("digest ") => h,
                    _ => {
                        // No digest auth header, we're done
                        self.state = DigestFlowState::Done;
                        return Err(pyo3::exceptions::PyStopIteration::new_err(()));
                    }
                };

                // Parse the challenge
                let challenge = match Self::parse_challenge(&auth_header) {
                    Some(c) => c,
                    None => {
                        self.state = DigestFlowState::Done;
                        return Err(pyo3::exceptions::PyRuntimeError::new_err(
                            "Failed to parse Digest WWW-Authenticate header"
                        ));
                    }
                };

                // Reset nonce count for new challenge
                self.nonce_count = 1;
                self.last_challenge = Some(challenge.clone());

                // Build authenticated request
                if let Some(mut request) = self.request.take() {
                    let auth_header = self.build_auth_header(&request, &challenge);
                    request.headers_mut().set("Authorization".to_string(), auth_header);

                    // Copy cookies from response if present
                    if let Ok(cookies) = response.getattr("cookies") {
                        if let Ok(cookie_jar) = cookies.extract::<crate::cookies::Cookies>() {
                            cookie_jar.set_cookie_header(&mut request);
                        }
                    }

                    self.state = DigestFlowState::SentAuthRequest;
                    self.request = Some(request.clone());
                    Ok(request)
                } else {
                    self.state = DigestFlowState::Done;
                    Err(pyo3::exceptions::PyStopIteration::new_err(()))
                }
            }
            DigestFlowState::SentAuthRequest => {
                self.state = DigestFlowState::Done;
                Err(pyo3::exceptions::PyStopIteration::new_err(()))
            }
            _ => Err(pyo3::exceptions::PyStopIteration::new_err(())),
        }
    }
}

// Hash functions
fn md5_hash(data: &[u8]) -> String {
    let digest = md5::compute(data);
    format!("{:x}", digest)
}

fn sha1_hash(data: &[u8]) -> String {
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn sha256_hash(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn sha512_hash(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha512::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Parse HTTP header list (simplified version)
fn parse_http_list(header: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in header.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ',' if !in_quotes => {
                if !current.trim().is_empty() {
                    result.push(current.trim().to_string());
                }
                current = String::new();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

/// Remove quotes from a string
fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len()-1].to_string()
    } else {
        s.to_string()
    }
}

/// NetRC authentication (placeholder)
#[pyclass(name = "NetRCAuth")]
#[derive(Clone, Debug)]
pub struct NetRCAuth {
    #[pyo3(get)]
    pub file: Option<String>,
}

#[pymethods]
impl NetRCAuth {
    #[new]
    #[pyo3(signature = (file=None))]
    fn new(file: Option<&str>) -> Self {
        Self {
            file: file.map(|s| s.to_string()),
        }
    }

    fn __repr__(&self) -> String {
        format!("NetRCAuth(file={:?})", self.file)
    }
}

/// HTTP status codes - provides flexible access patterns
#[pyclass(name = "codes", subclass)]
pub struct codes;

impl codes {
    fn name_to_code(name: &str) -> Option<u16> {
        match name.to_uppercase().as_str() {
            "CONTINUE" => Some(100),
            "SWITCHING_PROTOCOLS" => Some(101),
            "PROCESSING" => Some(102),
            "EARLY_HINTS" => Some(103),
            "OK" => Some(200),
            "CREATED" => Some(201),
            "ACCEPTED" => Some(202),
            "NON_AUTHORITATIVE_INFORMATION" => Some(203),
            "NO_CONTENT" => Some(204),
            "RESET_CONTENT" => Some(205),
            "PARTIAL_CONTENT" => Some(206),
            "MULTI_STATUS" => Some(207),
            "ALREADY_REPORTED" => Some(208),
            "IM_USED" => Some(226),
            "MULTIPLE_CHOICES" => Some(300),
            "MOVED_PERMANENTLY" => Some(301),
            "FOUND" => Some(302),
            "SEE_OTHER" => Some(303),
            "NOT_MODIFIED" => Some(304),
            "USE_PROXY" => Some(305),
            "TEMPORARY_REDIRECT" => Some(307),
            "PERMANENT_REDIRECT" => Some(308),
            "BAD_REQUEST" => Some(400),
            "UNAUTHORIZED" => Some(401),
            "PAYMENT_REQUIRED" => Some(402),
            "FORBIDDEN" => Some(403),
            "NOT_FOUND" => Some(404),
            "METHOD_NOT_ALLOWED" => Some(405),
            "NOT_ACCEPTABLE" => Some(406),
            "PROXY_AUTHENTICATION_REQUIRED" => Some(407),
            "REQUEST_TIMEOUT" => Some(408),
            "CONFLICT" => Some(409),
            "GONE" => Some(410),
            "LENGTH_REQUIRED" => Some(411),
            "PRECONDITION_FAILED" => Some(412),
            "PAYLOAD_TOO_LARGE" => Some(413),
            "URI_TOO_LONG" => Some(414),
            "UNSUPPORTED_MEDIA_TYPE" => Some(415),
            "RANGE_NOT_SATISFIABLE" => Some(416),
            "EXPECTATION_FAILED" => Some(417),
            "IM_A_TEAPOT" => Some(418),
            "MISDIRECTED_REQUEST" => Some(421),
            "UNPROCESSABLE_ENTITY" => Some(422),
            "LOCKED" => Some(423),
            "FAILED_DEPENDENCY" => Some(424),
            "TOO_EARLY" => Some(425),
            "UPGRADE_REQUIRED" => Some(426),
            "PRECONDITION_REQUIRED" => Some(428),
            "TOO_MANY_REQUESTS" => Some(429),
            "REQUEST_HEADER_FIELDS_TOO_LARGE" => Some(431),
            "UNAVAILABLE_FOR_LEGAL_REASONS" => Some(451),
            "INTERNAL_SERVER_ERROR" => Some(500),
            "NOT_IMPLEMENTED" => Some(501),
            "BAD_GATEWAY" => Some(502),
            "SERVICE_UNAVAILABLE" => Some(503),
            "GATEWAY_TIMEOUT" => Some(504),
            "HTTP_VERSION_NOT_SUPPORTED" => Some(505),
            "VARIANT_ALSO_NEGOTIATES" => Some(506),
            "INSUFFICIENT_STORAGE" => Some(507),
            "LOOP_DETECTED" => Some(508),
            "NOT_EXTENDED" => Some(510),
            "NETWORK_AUTHENTICATION_REQUIRED" => Some(511),
            _ => None,
        }
    }

    fn code_to_phrase(code: u16) -> &'static str {
        match code {
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            103 => "Early Hints",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            205 => "Reset Content",
            206 => "Partial Content",
            207 => "Multi-Status",
            208 => "Already Reported",
            226 => "IM Used",
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Payload Too Large",
            414 => "URI Too Long",
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            418 => "I'm a teapot",
            421 => "Misdirected Request",
            422 => "Unprocessable Entity",
            423 => "Locked",
            424 => "Failed Dependency",
            425 => "Too Early",
            426 => "Upgrade Required",
            428 => "Precondition Required",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            451 => "Unavailable For Legal Reasons",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            506 => "Variant Also Negotiates",
            507 => "Insufficient Storage",
            508 => "Loop Detected",
            510 => "Not Extended",
            511 => "Network Authentication Required",
            _ => "",
        }
    }
}

#[pymethods]
impl codes {
    /// Allow codes["NOT_FOUND"] access
    #[classmethod]
    fn __class_getitem__(_cls: &Bound<'_, pyo3::types::PyType>, name: &str) -> PyResult<u16> {
        Self::name_to_code(name).ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(name.to_string())
        })
    }

    /// Get reason phrase for a status code
    #[staticmethod]
    fn get_reason_phrase(code: u16) -> &'static str {
        Self::code_to_phrase(code)
    }

    // 1xx Informational
    #[classattr]
    const CONTINUE: u16 = 100;
    #[classattr]
    const SWITCHING_PROTOCOLS: u16 = 101;
    #[classattr]
    const PROCESSING: u16 = 102;
    #[classattr]
    const EARLY_HINTS: u16 = 103;

    // 2xx Success
    #[classattr]
    const OK: u16 = 200;
    #[classattr]
    const CREATED: u16 = 201;
    #[classattr]
    const ACCEPTED: u16 = 202;
    #[classattr]
    const NON_AUTHORITATIVE_INFORMATION: u16 = 203;
    #[classattr]
    const NO_CONTENT: u16 = 204;
    #[classattr]
    const RESET_CONTENT: u16 = 205;
    #[classattr]
    const PARTIAL_CONTENT: u16 = 206;
    #[classattr]
    const MULTI_STATUS: u16 = 207;
    #[classattr]
    const ALREADY_REPORTED: u16 = 208;
    #[classattr]
    const IM_USED: u16 = 226;

    // 3xx Redirection
    #[classattr]
    const MULTIPLE_CHOICES: u16 = 300;
    #[classattr]
    const MOVED_PERMANENTLY: u16 = 301;
    #[classattr]
    const FOUND: u16 = 302;
    #[classattr]
    const SEE_OTHER: u16 = 303;
    #[classattr]
    const NOT_MODIFIED: u16 = 304;
    #[classattr]
    const USE_PROXY: u16 = 305;
    #[classattr]
    const TEMPORARY_REDIRECT: u16 = 307;
    #[classattr]
    const PERMANENT_REDIRECT: u16 = 308;

    // 4xx Client Error
    #[classattr]
    const BAD_REQUEST: u16 = 400;
    #[classattr]
    const UNAUTHORIZED: u16 = 401;
    #[classattr]
    const PAYMENT_REQUIRED: u16 = 402;
    #[classattr]
    const FORBIDDEN: u16 = 403;
    #[classattr]
    const NOT_FOUND: u16 = 404;
    #[classattr]
    const METHOD_NOT_ALLOWED: u16 = 405;
    #[classattr]
    const NOT_ACCEPTABLE: u16 = 406;
    #[classattr]
    const PROXY_AUTHENTICATION_REQUIRED: u16 = 407;
    #[classattr]
    const REQUEST_TIMEOUT: u16 = 408;
    #[classattr]
    const CONFLICT: u16 = 409;
    #[classattr]
    const GONE: u16 = 410;
    #[classattr]
    const LENGTH_REQUIRED: u16 = 411;
    #[classattr]
    const PRECONDITION_FAILED: u16 = 412;
    #[classattr]
    const PAYLOAD_TOO_LARGE: u16 = 413;
    #[classattr]
    const URI_TOO_LONG: u16 = 414;
    #[classattr]
    const UNSUPPORTED_MEDIA_TYPE: u16 = 415;
    #[classattr]
    const RANGE_NOT_SATISFIABLE: u16 = 416;
    #[classattr]
    const EXPECTATION_FAILED: u16 = 417;
    #[classattr]
    const IM_A_TEAPOT: u16 = 418;
    #[classattr]
    const MISDIRECTED_REQUEST: u16 = 421;
    #[classattr]
    const UNPROCESSABLE_ENTITY: u16 = 422;
    #[classattr]
    const LOCKED: u16 = 423;
    #[classattr]
    const FAILED_DEPENDENCY: u16 = 424;
    #[classattr]
    const TOO_EARLY: u16 = 425;
    #[classattr]
    const UPGRADE_REQUIRED: u16 = 426;
    #[classattr]
    const PRECONDITION_REQUIRED: u16 = 428;
    #[classattr]
    const TOO_MANY_REQUESTS: u16 = 429;
    #[classattr]
    const REQUEST_HEADER_FIELDS_TOO_LARGE: u16 = 431;
    #[classattr]
    const UNAVAILABLE_FOR_LEGAL_REASONS: u16 = 451;

    // 5xx Server Error
    #[classattr]
    const INTERNAL_SERVER_ERROR: u16 = 500;
    #[classattr]
    const NOT_IMPLEMENTED: u16 = 501;
    #[classattr]
    const BAD_GATEWAY: u16 = 502;
    #[classattr]
    const SERVICE_UNAVAILABLE: u16 = 503;
    #[classattr]
    const GATEWAY_TIMEOUT: u16 = 504;
    #[classattr]
    const HTTP_VERSION_NOT_SUPPORTED: u16 = 505;
    #[classattr]
    const VARIANT_ALSO_NEGOTIATES: u16 = 506;
    #[classattr]
    const INSUFFICIENT_STORAGE: u16 = 507;
    #[classattr]
    const LOOP_DETECTED: u16 = 508;
    #[classattr]
    const NOT_EXTENDED: u16 = 510;
    #[classattr]
    const NETWORK_AUTHENTICATION_REQUIRED: u16 = 511;

    // Lowercase aliases for all status codes
    #[classattr]
    fn r#continue() -> u16 { 100 }
    #[classattr]
    fn switching_protocols() -> u16 { 101 }
    #[classattr]
    fn processing() -> u16 { 102 }
    #[classattr]
    fn early_hints() -> u16 { 103 }
    #[classattr]
    fn ok() -> u16 { 200 }
    #[classattr]
    fn created() -> u16 { 201 }
    #[classattr]
    fn accepted() -> u16 { 202 }
    #[classattr]
    fn non_authoritative_information() -> u16 { 203 }
    #[classattr]
    fn no_content() -> u16 { 204 }
    #[classattr]
    fn reset_content() -> u16 { 205 }
    #[classattr]
    fn partial_content() -> u16 { 206 }
    #[classattr]
    fn multi_status() -> u16 { 207 }
    #[classattr]
    fn already_reported() -> u16 { 208 }
    #[classattr]
    fn im_used() -> u16 { 226 }
    #[classattr]
    fn multiple_choices() -> u16 { 300 }
    #[classattr]
    fn moved_permanently() -> u16 { 301 }
    #[classattr]
    fn found() -> u16 { 302 }
    #[classattr]
    fn see_other() -> u16 { 303 }
    #[classattr]
    fn not_modified() -> u16 { 304 }
    #[classattr]
    fn use_proxy() -> u16 { 305 }
    #[classattr]
    fn temporary_redirect() -> u16 { 307 }
    #[classattr]
    fn permanent_redirect() -> u16 { 308 }
    #[classattr]
    fn bad_request() -> u16 { 400 }
    #[classattr]
    fn unauthorized() -> u16 { 401 }
    #[classattr]
    fn payment_required() -> u16 { 402 }
    #[classattr]
    fn forbidden() -> u16 { 403 }
    #[classattr]
    fn not_found() -> u16 { 404 }
    #[classattr]
    fn method_not_allowed() -> u16 { 405 }
    #[classattr]
    fn not_acceptable() -> u16 { 406 }
    #[classattr]
    fn proxy_authentication_required() -> u16 { 407 }
    #[classattr]
    fn request_timeout() -> u16 { 408 }
    #[classattr]
    fn conflict() -> u16 { 409 }
    #[classattr]
    fn gone() -> u16 { 410 }
    #[classattr]
    fn length_required() -> u16 { 411 }
    #[classattr]
    fn precondition_failed() -> u16 { 412 }
    #[classattr]
    fn payload_too_large() -> u16 { 413 }
    #[classattr]
    fn uri_too_long() -> u16 { 414 }
    #[classattr]
    fn unsupported_media_type() -> u16 { 415 }
    #[classattr]
    fn range_not_satisfiable() -> u16 { 416 }
    #[classattr]
    fn expectation_failed() -> u16 { 417 }
    #[classattr]
    fn im_a_teapot() -> u16 { 418 }
    #[classattr]
    fn misdirected_request() -> u16 { 421 }
    #[classattr]
    fn unprocessable_entity() -> u16 { 422 }
    #[classattr]
    fn locked() -> u16 { 423 }
    #[classattr]
    fn failed_dependency() -> u16 { 424 }
    #[classattr]
    fn too_early() -> u16 { 425 }
    #[classattr]
    fn upgrade_required() -> u16 { 426 }
    #[classattr]
    fn precondition_required() -> u16 { 428 }
    #[classattr]
    fn too_many_requests() -> u16 { 429 }
    #[classattr]
    fn request_header_fields_too_large() -> u16 { 431 }
    #[classattr]
    fn unavailable_for_legal_reasons() -> u16 { 451 }
    #[classattr]
    fn internal_server_error() -> u16 { 500 }
    #[classattr]
    fn not_implemented() -> u16 { 501 }
    #[classattr]
    fn bad_gateway() -> u16 { 502 }
    #[classattr]
    fn service_unavailable() -> u16 { 503 }
    #[classattr]
    fn gateway_timeout() -> u16 { 504 }
    #[classattr]
    fn http_version_not_supported() -> u16 { 505 }
    #[classattr]
    fn variant_also_negotiates() -> u16 { 506 }
    #[classattr]
    fn insufficient_storage() -> u16 { 507 }
    #[classattr]
    fn loop_detected() -> u16 { 508 }
    #[classattr]
    fn not_extended() -> u16 { 510 }
    #[classattr]
    fn network_authentication_required() -> u16 { 511 }
}
