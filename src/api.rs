//! Top-level API functions (get, post, put, patch, delete, head, options, request, stream)

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::client::Client;
use crate::response::Response;

/// Perform a GET request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn get(
    py: Python<'_>,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, "GET", url, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a POST request
#[pyfunction]
#[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn post(
    py: Python<'_>,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, "POST", url, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a PUT request
#[pyfunction]
#[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn put(
    py: Python<'_>,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, "PUT", url, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a PATCH request
#[pyfunction]
#[pyo3(signature = (url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn patch(
    py: Python<'_>,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, "PATCH", url, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a DELETE request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn delete(
    py: Python<'_>,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, "DELETE", url, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a HEAD request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn head(
    py: Python<'_>,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, "HEAD", url, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform an OPTIONS request
#[pyfunction]
#[pyo3(signature = (url, *, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn options(
    py: Python<'_>,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, "OPTIONS", url, None, None, None, None, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform an HTTP request
#[pyfunction]
#[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn request(
    py: Python<'_>,
    method: &str,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, method, url, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}

/// Perform a streaming HTTP request
#[pyfunction]
#[pyo3(signature = (method, url, *, content=None, data=None, files=None, json=None, params=None, headers=None, cookies=None, auth=None, follow_redirects=None, timeout=None, verify=None, cert=None, trust_env=None))]
pub fn stream(
    py: Python<'_>,
    method: &str,
    url: &str,
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
    let client = Client::default();
    client.execute_request(py, method, url, content, data, files, json, params, headers, cookies, auth, timeout, follow_redirects)
}
