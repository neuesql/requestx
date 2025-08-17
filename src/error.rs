use pyo3::exceptions::{PyConnectionError, PyRuntimeError, PyTimeoutError, PyValueError};
use pyo3::prelude::*;
use thiserror::Error;

/// Custom error types for RequestX
#[derive(Error, Debug)]
pub enum RequestxError {
    #[error("Network error: {0}")]
    NetworkError(#[from] hyper::Error),

    #[error("Request timeout: {0}")]
    TimeoutError(#[from] tokio::time::error::Elapsed),

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("JSON decode error: {0}")]
    JsonDecodeError(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] hyper::http::uri::InvalidUri),

    #[error("HTTP request error: {0}")]
    HttpRequestError(#[from] hyper::http::Error),

    #[error("SSL error: {0}")]
    SslError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Python error: {0}")]
    PythonError(String),
}

/// Convert Rust errors to Python exceptions
impl From<RequestxError> for PyErr {
    fn from(error: RequestxError) -> Self {
        match error {
            RequestxError::NetworkError(e) => {
                // Use more efficient string formatting for common cases
                PyConnectionError::new_err(format!("Network error: {}", e))
            }
            RequestxError::TimeoutError(_) => {
                // Use static string for timeout errors to avoid allocation
                PyTimeoutError::new_err("Request timeout")
            }
            RequestxError::HttpError { status, message } => {
                // Pre-format common HTTP errors
                match status {
                    400 => PyRuntimeError::new_err("HTTP 400: Bad Request"),
                    401 => PyRuntimeError::new_err("HTTP 401: Unauthorized"),
                    403 => PyRuntimeError::new_err("HTTP 403: Forbidden"),
                    404 => PyRuntimeError::new_err("HTTP 404: Not Found"),
                    500 => PyRuntimeError::new_err("HTTP 500: Internal Server Error"),
                    502 => PyRuntimeError::new_err("HTTP 502: Bad Gateway"),
                    503 => PyRuntimeError::new_err("HTTP 503: Service Unavailable"),
                    _ => PyRuntimeError::new_err(format!("HTTP {}: {}", status, message)),
                }
            }
            RequestxError::JsonDecodeError(e) => {
                PyValueError::new_err(format!("JSON decode error: {}", e))
            }
            RequestxError::InvalidUrl(e) => {
                PyValueError::new_err(format!("Invalid URL: {}", e))
            }
            RequestxError::HttpRequestError(e) => {
                PyRuntimeError::new_err(format!("HTTP request error: {}", e))
            }
            RequestxError::SslError(msg) => {
                PyConnectionError::new_err(format!("SSL error: {}", msg))
            }
            RequestxError::RuntimeError(msg) => {
                // Check if this is an invalid URL error and map it to ValueError
                if msg.contains("Invalid URL:") {
                    PyValueError::new_err(msg)
                } else {
                    PyRuntimeError::new_err(format!("Runtime error: {}", msg))
                }
            }
            RequestxError::PythonError(msg) => {
                PyRuntimeError::new_err(format!("Python error: {}", msg))
            }
        }
    }
}
