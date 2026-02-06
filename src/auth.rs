//! Authentication implementations

use base64::Engine;
use digest::Digest;
use pyo3::prelude::*;
use pyo3::types::PyList;
use rand::RngCore;

use crate::request::Request;

/// Build a Basic auth header value: "Basic <base64(username:password)>".
#[pyfunction]
pub fn basic_auth_header(username: &str, password: &str) -> String {
    let credentials = format!("{}:{}", username, password);
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
    format!("Basic {}", encoded)
}

/// Generate a client nonce for digest auth.
/// Returns a 16-character hex string.
#[pyfunction]
pub fn generate_cnonce() -> String {
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    // SHA1 hash of random bytes, take first 16 hex chars
    let mut hasher = sha1::Sha1::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

/// Compute a digest hash using the specified algorithm.
/// Supported algorithms: MD5, SHA, SHA-256, SHA-512 (and their -SESS variants).
#[pyfunction]
pub fn digest_hash(data: &str, algorithm: &str) -> String {
    let algo = algorithm.to_uppercase();
    let algo = algo.trim_end_matches("-SESS");

    match algo {
        "MD5" => {
            let mut hasher = md5::Md5::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        }
        "SHA" => {
            let mut hasher = sha1::Sha1::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        }
        "SHA-256" => {
            let mut hasher = sha2::Sha256::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        }
        "SHA-512" => {
            let mut hasher = sha2::Sha512::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        }
        _ => {
            // Default to MD5
            let mut hasher = md5::Md5::new();
            hasher.update(data.as_bytes());
            hex::encode(hasher.finalize())
        }
    }
}

/// Build the Digest auth response value.
/// Returns the response hash and the qop value used (if any).
#[pyfunction]
#[pyo3(signature = (username, password, realm, nonce, nc, cnonce, qop, method, uri, algorithm))]
pub fn compute_digest_response(
    username: &str,
    password: &str,
    realm: &str,
    nonce: &str,
    nc: &str,
    cnonce: &str,
    qop: &str,
    method: &str,
    uri: &str,
    algorithm: &str,
) -> PyResult<(String, Option<String>)> {
    // Calculate A1
    let a1_base = format!("{}:{}:{}", username, realm, password);
    let ha1 = if algorithm.to_uppercase().ends_with("-SESS") {
        let ha1_base = digest_hash(&a1_base, algorithm);
        digest_hash(&format!("{}:{}:{}", ha1_base, nonce, cnonce), algorithm)
    } else {
        digest_hash(&a1_base, algorithm)
    };

    // Calculate A2
    let a2 = format!("{}:{}", method, uri);
    let ha2 = digest_hash(&a2, algorithm);

    // Calculate response
    let (response, qop_value) = if !qop.is_empty() {
        // Parse qop options
        let qop_options: Vec<&str> = qop.split(',').map(|s| s.trim()).collect();
        if qop_options.contains(&"auth") {
            let qop_value = "auth".to_string();
            let response_data = format!("{}:{}:{}:{}:{}:{}", ha1, nonce, nc, cnonce, qop_value, ha2);
            (digest_hash(&response_data, algorithm), Some(qop_value))
        } else if qop_options.contains(&"auth-int") {
            return Err(pyo3::exceptions::PyNotImplementedError::new_err("Digest auth qop=auth-int is not implemented"));
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(format!("Unsupported Digest auth qop value: {}", qop)));
        }
    } else {
        // RFC 2069 style
        let response_data = format!("{}:{}:{}", ha1, nonce, ha2);
        (digest_hash(&response_data, algorithm), None)
    };

    Ok((response, qop_value))
}

/// Base Auth class that can be subclassed in Python
#[pyclass(name = "Auth", subclass)]
#[derive(Clone, Default)]
pub struct Auth {
    requires_request_body: bool,
    requires_response_body: bool,
}

#[pymethods]
impl Auth {
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(_args: &Bound<'_, pyo3::types::PyTuple>, _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>) -> Self {
        Self::default()
    }

    /// Called to get authentication flow generator
    /// Returns an iterator that yields requests
    #[pyo3(signature = (request))]
    fn auth_flow<'py>(&self, py: Python<'py>, request: &Request) -> PyResult<Bound<'py, PyList>> {
        // Return a list that can be iterated
        // Subclasses can override this
        let request = request.clone();
        let list = PyList::new(py, vec![request.into_pyobject(py)?])?;
        Ok(list)
    }

    /// Sync auth flow - calls auth_flow and iterates
    fn sync_auth_flow<'py>(&self, py: Python<'py>, request: &Request) -> PyResult<Bound<'py, PyList>> {
        self.auth_flow(py, request)
    }

    /// Async auth flow - calls auth_flow and iterates asynchronously
    fn async_auth_flow<'py>(&self, py: Python<'py>, request: &Request) -> PyResult<Bound<'py, PyList>> {
        self.auth_flow(py, request)
    }

    #[getter]
    fn requires_request_body(&self) -> bool {
        self.requires_request_body
    }

    #[getter]
    fn requires_response_body(&self) -> bool {
        self.requires_response_body
    }

    fn __repr__(&self) -> String {
        "<Auth>".to_string()
    }
}

/// Function-based auth that wraps a callable
#[pyclass(name = "FunctionAuth", extends = Auth)]
pub struct FunctionAuth {
    func: Py<PyAny>,
}

#[pymethods]
impl FunctionAuth {
    #[new]
    fn new(func: Py<PyAny>) -> (Self, Auth) {
        (Self { func }, Auth::default())
    }

    #[pyo3(signature = (request))]
    fn auth_flow<'py>(&self, py: Python<'py>, request: &Request) -> PyResult<Bound<'py, PyList>> {
        // Call the function with the request
        let result = self.func.call1(py, (request.clone(),))?;

        // If it returns a Request, wrap it in a list
        if let Ok(req) = result.extract::<Request>(py) {
            let list = PyList::new(py, vec![req.into_pyobject(py)?])?;
            return Ok(list);
        }

        // Otherwise assume it's already a list/iterable and convert to list
        let bound = result.bind(py);
        if let Ok(list) = bound.cast::<PyList>() {
            return Ok(list.clone());
        }

        // Use Python's list() builtin to convert any iterable to list
        let builtins = py.import("builtins")?;
        let list_func = builtins.getattr("list")?;
        let py_list = list_func.call1((bound,))?;
        Ok(py_list.cast::<PyList>()?.clone())
    }
}
