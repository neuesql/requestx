//! Additional types: streams, auth, status codes

use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Synchronous byte stream base class
#[pyclass(name = "SyncByteStream", subclass)]
#[derive(Clone, Debug, Default)]
pub struct SyncByteStream {
    data: Vec<u8>,
}

#[pymethods]
impl SyncByteStream {
    #[new]
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<Vec<u8>> {
        if self.data.is_empty() {
            None
        } else {
            let data = std::mem::take(&mut self.data);
            Some(data)
        }
    }

    fn read(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn close(&mut self) {
        self.data.clear();
    }
}

/// Asynchronous byte stream base class
#[pyclass(name = "AsyncByteStream", subclass)]
#[derive(Clone, Debug, Default)]
pub struct AsyncByteStream {
    data: Vec<u8>,
}

#[pymethods]
impl AsyncByteStream {
    #[new]
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        if self.data.is_empty() {
            Ok(None)
        } else {
            let data = std::mem::take(&mut self.data);
            Ok(Some(PyBytes::new(py, &data)))
        }
    }

    fn aread<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    fn aclose(&mut self) {
        self.data.clear();
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

    fn __repr__(&self) -> String {
        format!("BasicAuth(username={:?}, password=***)", self.username)
    }

    fn __eq__(&self, other: &BasicAuth) -> bool {
        self.username == other.username && self.password == other.password
    }
}

/// Digest authentication (placeholder)
#[pyclass(name = "DigestAuth")]
#[derive(Clone, Debug)]
pub struct DigestAuth {
    #[pyo3(get)]
    pub username: String,
    #[pyo3(get)]
    pub password: String,
}

#[pymethods]
impl DigestAuth {
    #[new]
    fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    fn __repr__(&self) -> String {
        format!("DigestAuth(username={:?}, password=***)", self.username)
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

/// HTTP status codes
#[pyclass(name = "codes")]
pub struct codes;

#[pymethods]
impl codes {
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
}
