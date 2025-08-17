use pyo3::exceptions::{PyConnectionError, PyRuntimeError, PyTimeoutError, PyValueError};
use pyo3::prelude::*;
use thiserror::Error;

/// Custom error types for RequestX that map to requests-compatible exceptions
#[derive(Error, Debug)]
pub enum RequestxError {
    #[error("Network error: {0}")]
    NetworkError(#[from] hyper::Error),

    #[error("Request timeout: {0}")]
    TimeoutError(#[from] tokio::time::error::Elapsed),

    #[error("Connect timeout")]
    ConnectTimeout,

    #[error("Read timeout")]
    ReadTimeout,

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("JSON decode error: {0}")]
    JsonDecodeError(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] hyper::http::uri::InvalidUri),

    #[error("URL required")]
    UrlRequired,

    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    #[error("Missing schema")]
    MissingSchema,

    #[error("HTTP request error: {0}")]
    HttpRequestError(#[from] hyper::http::Error),

    #[error("SSL error: {0}")]
    SslError(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Too many redirects")]
    TooManyRedirects,

    #[error("Proxy error: {0}")]
    ProxyError(String),

    #[error("Chunked encoding error: {0}")]
    ChunkedEncodingError(String),

    #[error("Content decoding error: {0}")]
    ContentDecodingError(String),

    #[error("Stream consumed error")]
    StreamConsumedError,

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Python error: {0}")]
    PythonError(String),
}

/// Convert Rust errors to Python exceptions with requests-compatible mapping
impl From<RequestxError> for PyErr {
    fn from(error: RequestxError) -> Self {
        match error {
            RequestxError::NetworkError(e) => {
                // Map hyper errors to appropriate connection errors
                let error_str = e.to_string();
                if error_str.contains("dns") || error_str.contains("resolve") {
                    PyConnectionError::new_err(format!("Failed to resolve hostname: {}", e))
                } else if error_str.contains("connect") || error_str.contains("connection") {
                    PyConnectionError::new_err(format!("Connection error: {}", e))
                } else if error_str.contains("timeout") {
                    PyTimeoutError::new_err(format!("Connection timeout: {}", e))
                } else {
                    PyConnectionError::new_err(format!("Network error: {}", e))
                }
            }
            RequestxError::TimeoutError(_) => PyTimeoutError::new_err(
                "The server did not send any data in the allotted amount of time",
            ),
            RequestxError::ConnectTimeout => PyTimeoutError::new_err(
                "The request timed out while trying to connect to the remote server",
            ),
            RequestxError::ReadTimeout => PyTimeoutError::new_err(
                "The server did not send any data in the allotted amount of time",
            ),
            RequestxError::HttpError { status, message } => {
                // Create HTTPError with status code information
                PyRuntimeError::new_err(format!("{} Client Error: {} for url", status, message))
            }
            RequestxError::JsonDecodeError(e) => {
                PyValueError::new_err(format!("Failed to decode JSON response: {}", e))
            }
            RequestxError::InvalidUrl(e) => PyValueError::new_err(format!("Invalid URL: {}", e)),
            RequestxError::UrlRequired => {
                PyValueError::new_err("A valid URL is required to make a request")
            }
            RequestxError::InvalidSchema(schema) => {
                PyValueError::new_err(format!("Invalid URL schema: {}", schema))
            }
            RequestxError::MissingSchema => {
                PyValueError::new_err("No connection adapters were found for the URL")
            }
            RequestxError::HttpRequestError(e) => {
                // Map HTTP request building errors
                let error_str = e.to_string();
                if error_str.contains("header") {
                    PyValueError::new_err(format!("Invalid header: {}", e))
                } else {
                    PyRuntimeError::new_err(format!("HTTP request error: {}", e))
                }
            }
            RequestxError::SslError(msg) => {
                PyConnectionError::new_err(format!("SSL error: {}", msg))
            }
            RequestxError::InvalidHeader(msg) => {
                PyValueError::new_err(format!("Invalid header: {}", msg))
            }
            RequestxError::TooManyRedirects => {
                PyRuntimeError::new_err("Exceeded maximum number of redirects")
            }
            RequestxError::ProxyError(msg) => {
                PyConnectionError::new_err(format!("Proxy error: {}", msg))
            }
            RequestxError::ChunkedEncodingError(msg) => {
                PyConnectionError::new_err(format!("Chunked encoding error: {}", msg))
            }
            RequestxError::ContentDecodingError(msg) => {
                PyRuntimeError::new_err(format!("Content decoding error: {}", msg))
            }
            RequestxError::StreamConsumedError => {
                PyRuntimeError::new_err("The content for this response was already consumed")
            }
            RequestxError::RuntimeError(msg) => {
                // Check for specific error patterns and map appropriately
                if msg.contains("Invalid URL:") {
                    PyValueError::new_err(msg)
                } else if msg.contains("Invalid HTTP method:") {
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

/// Helper function to register all custom exceptions with the Python module
pub fn register_exceptions(_py: Python, _m: &PyModule) -> PyResult<()> {
    // Don't register the exceptions here - they will be defined in Python
    // The Rust exceptions are only used for conversion to Python exceptions
    Ok(())
}
