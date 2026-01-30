//! Top-level API functions (get, post, put, patch, delete, head, options, request, stream)

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::client::Client;
use crate::response::Response;
use crate::url::URL;

/// Extract URL string from either a string or URL object
fn extract_url_string(url: &Bound<'_, PyAny>) -> PyResult<String> {
    // Try to extract as string first
    if let Ok(s) = url.extract::<String>() {
        return Ok(s);
    }
    // Try to extract as URL object
    if let Ok(url_obj) = url.extract::<URL>() {
        return Ok(url_obj.to_string());
    }
    // Try to call __str__ method
    if let Ok(s) = url.str() {
        return Ok(s.to_string());
    }
    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "url must be a string or URL object",
    ))
}

/// Perform a GET request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn get(
    py: Python<'_>,
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, "GET", &url_str, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a POST request
#[pyfunction]
#[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn post(
    py: Python<'_>,
    url: &Bound<'_, PyAny>,
    content: Option<Vec<u8>>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, "POST", &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a PUT request
#[pyfunction]
#[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn put(
    py: Python<'_>,
    url: &Bound<'_, PyAny>,
    content: Option<Vec<u8>>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, "PUT", &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a PATCH request
#[pyfunction]
#[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn patch(
    py: Python<'_>,
    url: &Bound<'_, PyAny>,
    content: Option<Vec<u8>>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, "PATCH", &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a DELETE request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn delete(
    py: Python<'_>,
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, "DELETE", &url_str, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a HEAD request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn head(
    py: Python<'_>,
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, "HEAD", &url_str, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform an OPTIONS request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn options(
    py: Python<'_>,
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, "OPTIONS", &url_str, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform an HTTP request
#[pyfunction]
#[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn request(
    py: Python<'_>,
    method: &str,
    url: &Bound<'_, PyAny>,
    content: Option<Vec<u8>>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, method, &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a streaming HTTP request
#[pyfunction]
#[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn stream(
    py: Python<'_>,
    method: &str,
    url: &Bound<'_, PyAny>,
    content: Option<Vec<u8>>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    params: Option<&Bound<'_, PyAny>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    follow_redirects: Option<bool>,
    timeout: Option<&Bound<'_, PyAny>>,
    verify: Option<bool>,
    cert: Option<&str>,
    trust_env: Option<bool>,
) -> PyResult<Response> {
    let url_str = extract_url_string(url)?;
    let client = Client::default();
    client.execute_request(py, method, &url_str, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}
