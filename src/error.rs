use pyo3::prelude::*;
use pyo3::exceptions::{PyConnectionError, PyTimeoutError, PyValueError, PyRuntimeError};
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
                PyConnectionError::new_err(format!("Network error: {}", e))
            }
            RequestxError::TimeoutError(e) => {
                PyTimeoutError::new_err(format!("Request timeout: {}", e))
            }
            RequestxError::HttpError { status, message } => {
                PyRuntimeError::new_err(format!("HTTP {}: {}", status, message))
            }
            RequestxError::JsonDecodeError(e) => {
                PyValueError::new_err(format!("JSON decode error: {}", e))
            }
            RequestxError::InvalidUrl(e) => {
                PyValueError::new_err(format!("Invalid URL: {}", e))
            }
            RequestxError::SslError(msg) => {
                PyConnectionError::new_err(format!("SSL error: {}", msg))
            }
            RequestxError::RuntimeError(msg) => {
                PyRuntimeError::new_err(format!("Runtime error: {}", msg))
            }
            RequestxError::PythonError(msg) => {
                PyRuntimeError::new_err(format!("Python error: {}", msg))
            }
        }
    }
}