//! Module-level request functions for requestx

use crate::client::Client;
use crate::response::Response;
use crate::streaming::StreamingResponse;
use crate::types::URL;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Extract URL string from either a string or URL object
fn extract_url_str(url: &Bound<'_, PyAny>) -> PyResult<String> {
    // First try to extract as URL object
    if let Ok(url_obj) = url.extract::<URL>() {
        return Ok(url_obj.as_str().to_string());
    }
    // Then try as string
    if let Ok(url_str) = url.extract::<String>() {
        return Ok(url_str);
    }
    // Finally try calling str() on the object
    if let Ok(s) = url.str() {
        return Ok(s.to_string());
    }
    Err(pyo3::exceptions::PyTypeError::new_err("url must be a string or URL object"))
}

/// Perform a generic HTTP request (sync)
#[pyfunction]
#[pyo3(signature = (
    method,
    url,
    params=None,
    headers=None,
    cookies=None,
    content=None,
    data=None,
    json=None,
    files=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn request(
    method: &str,
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyAny>>,
    data: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<Response> {
    // Create a one-shot client
    let client = Client::new(
        None, // base_url
        None, // headers
        None, // cookies
        None, // timeout
        follow_redirects,
        10,     // max_redirects
        verify, // verify (SSL verification)
        None,   // cert (client certificates)
        proxy,
        None,  // auth (passed per-request)
        false, // http2
        None,  // limits
        None,  // default_encoding
        true,  // trust_env
    )?;

    client.request(method, url, params, headers, cookies, content, data, json, files, auth, timeout, Some(follow_redirects))
}

/// Perform a GET request (sync)
#[pyfunction]
#[pyo3(signature = (
    url,
    params=None,
    headers=None,
    cookies=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn get(
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<Response> {
    request("GET", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects, verify, proxy)
}

/// Perform a POST request (sync)
#[pyfunction]
#[pyo3(signature = (
    url,
    params=None,
    headers=None,
    cookies=None,
    content=None,
    data=None,
    json=None,
    files=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn post(
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyAny>>,
    data: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<Response> {
    request("POST", url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects, verify, proxy)
}

/// Perform a PUT request (sync)
#[pyfunction]
#[pyo3(signature = (
    url,
    params=None,
    headers=None,
    cookies=None,
    content=None,
    data=None,
    json=None,
    files=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn put(
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyAny>>,
    data: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<Response> {
    request("PUT", url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects, verify, proxy)
}

/// Perform a PATCH request (sync)
#[pyfunction]
#[pyo3(signature = (
    url,
    params=None,
    headers=None,
    cookies=None,
    content=None,
    data=None,
    json=None,
    files=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn patch(
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyAny>>,
    data: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<Response> {
    request("PATCH", url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects, verify, proxy)
}

/// Perform a DELETE request (sync)
#[pyfunction]
#[pyo3(signature = (
    url,
    params=None,
    headers=None,
    cookies=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn delete(
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<Response> {
    request("DELETE", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects, verify, proxy)
}

/// Perform a HEAD request (sync)
#[pyfunction]
#[pyo3(signature = (
    url,
    params=None,
    headers=None,
    cookies=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn head(
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<Response> {
    request("HEAD", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects, verify, proxy)
}

/// Perform an OPTIONS request (sync)
#[pyfunction]
#[pyo3(signature = (
    url,
    params=None,
    headers=None,
    cookies=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn options(
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<Response> {
    request("OPTIONS", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects, verify, proxy)
}

/// Stream a request (sync) - returns a context manager for streaming responses
#[pyfunction]
#[pyo3(signature = (
    method,
    url,
    params=None,
    headers=None,
    cookies=None,
    content=None,
    data=None,
    json=None,
    files=None,
    auth=None,
    timeout=None,
    follow_redirects=true,
    verify=None,
    proxy=None
))]
pub fn stream(
    method: &str,
    url: &Bound<'_, PyAny>,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyAny>>,
    data: Option<&Bound<'_, PyAny>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyAny>>,
    auth: Option<&Bound<'_, PyAny>>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<&Bound<'_, PyAny>>,
) -> PyResult<StreamingResponse> {
    // Create a one-shot client
    let client = Client::new(
        None, // base_url
        None, // headers
        None, // cookies
        None, // timeout
        follow_redirects,
        10,     // max_redirects
        verify, // verify (SSL verification)
        None,   // cert (client certificates)
        proxy,
        None,  // auth (passed per-request)
        false, // http2
        None,  // limits
        None,  // default_encoding
        true,  // trust_env
    )?;

    client.stream(method, url, params, headers, cookies, content, data, json, files, auth, timeout, Some(follow_redirects))
}
