//! RequestX - High-performance HTTP client for Python
//!
//! A drop-in replacement for the requests library, built with Rust for speed and memory safety.
//! Provides both synchronous and asynchronous APIs while maintaining full compatibility with
//! the familiar requests interface.

use hyper::{Method, Uri};
use pyo3::prelude::*;
use pyo3::types::PyDict;

mod auth;
mod config;
mod core;
mod error;
mod python_interface;
mod response;
mod session;
mod types;

use auth::{get_auth_from_url, urldefragauth};
use core::http_client::RequestxClient;
use error::RequestxError;
use python_interface::{parse_and_validate_url, parse_kwargs, response_data_to_py_response};
use response::{CaseInsensitivePyDict, Response};
use session::Session;

/// Create an HTTP request function for a specific method
macro_rules! make_request_function {
    ($name:ident, $method:expr) => {
        /// HTTP request with the specified method
        #[pyfunction(signature = (url, /, **kwargs))]
        fn $name(
            py: Python,
            url: String,
            kwargs: Option<&Bound<'_, PyDict>>,
        ) -> PyResult<PyObject> {
            let (uri, url_auth) = parse_and_validate_url(&url)?;
            let mut config_builder = parse_kwargs(py, kwargs)?;

            // Merge URL auth with kwargs auth (kwargs auth takes precedence)
            if let Some(url_auth) = url_auth {
                if config_builder.auth.is_none() {
                    config_builder.auth = Some(url_auth);
                }
            }

            let config = config_builder.build($method, uri);

            // Use enhanced runtime management for context detection and execution
            let runtime_manager = core::runtime::get_global_runtime_manager();

            let future = async move {
                let client = RequestxClient::new()?;
                let response_data = client.request_async(config).await?;
                response_data_to_py_response(response_data)
            };

            runtime_manager.execute_future(py, future)
        }
    };
}

// Create HTTP method functions
make_request_function!(get, Method::GET);
make_request_function!(post, Method::POST);
make_request_function!(put, Method::PUT);
make_request_function!(delete, Method::DELETE);
make_request_function!(head, Method::HEAD);
make_request_function!(options, Method::OPTIONS);
make_request_function!(patch, Method::PATCH);

/// Generic HTTP request with enhanced async/sync context detection
#[pyfunction(signature = (method, url, /, **kwargs))]
fn request(
    py: Python,
    method: String,
    url: String,
    kwargs: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    // Validate HTTP method - only allow standard methods
    let method_upper = method.to_uppercase();
    let method: Method = match method_upper.as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        "PATCH" => Method::PATCH,
        "TRACE" => Method::TRACE,
        "CONNECT" => Method::CONNECT,
        _ => {
            return Err(
                RequestxError::RuntimeError(format!("Invalid HTTP method: {method}")).into(),
            )
        }
    };

    let (uri, url_auth) = parse_and_validate_url(&url)?;
    let mut config_builder = parse_kwargs(py, kwargs)?;

    // Merge URL auth with kwargs auth (kwargs auth takes precedence)
    if let Some(url_auth) = url_auth {
        if config_builder.auth.is_none() {
            config_builder.auth = Some(url_auth);
        }
    }

    let config = config_builder.build(method, uri);

    // Use enhanced runtime management for context detection and execution
    let runtime_manager = core::runtime::get_global_runtime_manager();

    let future = async move {
        let client = RequestxClient::new()?;
        let response_data = client.request_async(config).await?;
        response_data_to_py_response(response_data)
    };

    runtime_manager.execute_future(py, future)
}

/// RequestX Python module
#[pymodule]
fn _requestx(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register HTTP method functions
    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(post, m)?)?;
    m.add_function(wrap_pyfunction!(put, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    m.add_function(wrap_pyfunction!(head, m)?)?;
    m.add_function(wrap_pyfunction!(options, m)?)?;
    m.add_function(wrap_pyfunction!(patch, m)?)?;
    m.add_function(wrap_pyfunction!(request, m)?)?;

    // Register utility functions
    m.add_function(wrap_pyfunction!(get_auth_from_url, m)?)?;
    m.add_function(wrap_pyfunction!(urldefragauth, m)?)?;

    // Register classes
    m.add_class::<Response>()?;
    m.add_class::<CaseInsensitivePyDict>()?;
    m.add_class::<Session>()?;

    // Register auth classes
    m.add_class::<auth::PyHTTPDigestAuth>()?;
    m.add_class::<auth::PyHTTPProxyAuth>()?;

    // Register custom exceptions
    error::register_exceptions(py, m)?;

    Ok(())
}
