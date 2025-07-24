use hyper::{Method, Uri};
use pyo3::prelude::*;
use pyo3::types::PyDict;

mod core;
mod error;
mod response;
mod session;

use core::client::{RequestConfig, RequestxClient, ResponseData};
use error::RequestxError;
use response::Response;
use session::Session;

/// Parse kwargs into RequestConfig (basic implementation for now)
fn parse_kwargs(_kwargs: Option<&PyDict>) -> PyResult<Option<RequestConfig>> {
    // For now, return None - will be expanded in later tasks
    Ok(None)
}

/// Convert ResponseData to Python Response object
fn response_data_to_py_response(response_data: ResponseData) -> PyResult<Response> {
    let headers = response_data
        .headers
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();

    Ok(Response::new(
        response_data.status_code,
        response_data.url.to_string(),
        headers,
        response_data.body.to_vec(),
    ))
}

/// HTTP GET request
#[pyfunction]
fn get(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = url.parse().map_err(RequestxError::InvalidUrl)?;
    let _config = parse_kwargs(kwargs)?;
    let client = RequestxClient::new()?;

    // Execute synchronously for now - async detection will be added in task 5
    let response_data = client.request_sync(RequestConfig {
        method: Method::GET,
        url: uri,
        headers: None,
        params: None,
        data: None,
        json: None,
        timeout: None,
        allow_redirects: true,
        verify: true,
    })?;

    let response = response_data_to_py_response(response_data)?;
    Ok(response.into_py(py))
}

/// HTTP POST request
#[pyfunction]
fn post(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = url.parse().map_err(RequestxError::InvalidUrl)?;
    let _config = parse_kwargs(kwargs)?;
    let client = RequestxClient::new()?;

    let response_data = client.request_sync(RequestConfig {
        method: Method::POST,
        url: uri,
        headers: None,
        params: None,
        data: None,
        json: None,
        timeout: None,
        allow_redirects: true,
        verify: true,
    })?;

    let response = response_data_to_py_response(response_data)?;
    Ok(response.into_py(py))
}

/// HTTP PUT request
#[pyfunction]
fn put(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = url.parse().map_err(RequestxError::InvalidUrl)?;
    let _config = parse_kwargs(kwargs)?;
    let client = RequestxClient::new()?;

    let response_data = client.request_sync(RequestConfig {
        method: Method::PUT,
        url: uri,
        headers: None,
        params: None,
        data: None,
        json: None,
        timeout: None,
        allow_redirects: true,
        verify: true,
    })?;

    let response = response_data_to_py_response(response_data)?;
    Ok(response.into_py(py))
}

/// HTTP DELETE request
#[pyfunction]
fn delete(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = url.parse().map_err(RequestxError::InvalidUrl)?;
    let _config = parse_kwargs(kwargs)?;
    let client = RequestxClient::new()?;

    let response_data = client.request_sync(RequestConfig {
        method: Method::DELETE,
        url: uri,
        headers: None,
        params: None,
        data: None,
        json: None,
        timeout: None,
        allow_redirects: true,
        verify: true,
    })?;

    let response = response_data_to_py_response(response_data)?;
    Ok(response.into_py(py))
}

/// HTTP HEAD request
#[pyfunction]
fn head(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = url.parse().map_err(RequestxError::InvalidUrl)?;
    let _config = parse_kwargs(kwargs)?;
    let client = RequestxClient::new()?;

    let response_data = client.request_sync(RequestConfig {
        method: Method::HEAD,
        url: uri,
        headers: None,
        params: None,
        data: None,
        json: None,
        timeout: None,
        allow_redirects: true,
        verify: true,
    })?;

    let response = response_data_to_py_response(response_data)?;
    Ok(response.into_py(py))
}

/// HTTP OPTIONS request
#[pyfunction]
fn options(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = url.parse().map_err(RequestxError::InvalidUrl)?;
    let _config = parse_kwargs(kwargs)?;
    let client = RequestxClient::new()?;

    let response_data = client.request_sync(RequestConfig {
        method: Method::OPTIONS,
        url: uri,
        headers: None,
        params: None,
        data: None,
        json: None,
        timeout: None,
        allow_redirects: true,
        verify: true,
    })?;

    let response = response_data_to_py_response(response_data)?;
    Ok(response.into_py(py))
}

/// HTTP PATCH request
#[pyfunction]
fn patch(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = url.parse().map_err(RequestxError::InvalidUrl)?;
    let _config = parse_kwargs(kwargs)?;
    let client = RequestxClient::new()?;

    let response_data = client.request_sync(RequestConfig {
        method: Method::PATCH,
        url: uri,
        headers: None,
        params: None,
        data: None,
        json: None,
        timeout: None,
        allow_redirects: true,
        verify: true,
    })?;

    let response = response_data_to_py_response(response_data)?;
    Ok(response.into_py(py))
}

/// Generic HTTP request
#[pyfunction]
fn request(py: Python, method: String, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
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
        _ => return Err(RequestxError::RuntimeError(format!("Invalid HTTP method: {}", method)).into()),
    };
    
    let uri: Uri = url.parse().map_err(RequestxError::InvalidUrl)?;
    let _config = parse_kwargs(kwargs)?;
    let client = RequestxClient::new()?;

    let response_data = client.request_sync(RequestConfig {
        method,
        url: uri,
        headers: None,
        params: None,
        data: None,
        json: None,
        timeout: None,
        allow_redirects: true,
        verify: true,
    })?;

    let response = response_data_to_py_response(response_data)?;
    Ok(response.into_py(py))
}

/// RequestX Python module
#[pymodule]
fn _requestx(_py: Python, m: &PyModule) -> PyResult<()> {
    // Register HTTP method functions
    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(post, m)?)?;
    m.add_function(wrap_pyfunction!(put, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    m.add_function(wrap_pyfunction!(head, m)?)?;
    m.add_function(wrap_pyfunction!(options, m)?)?;
    m.add_function(wrap_pyfunction!(patch, m)?)?;
    m.add_function(wrap_pyfunction!(request, m)?)?;

    // Register classes
    m.add_class::<Response>()?;
    m.add_class::<Session>()?;

    Ok(())
}
