//! Module-level request functions for requestx

use crate::client::Client;
use crate::response::Response;
use crate::types::{Auth, Proxy};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

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
    url: &str,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyBytes>>,
    data: Option<&Bound<'_, PyDict>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyDict>>,
    auth: Option<Auth>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<Proxy>,
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
    url: &str,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<Auth>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<Proxy>,
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
    url: &str,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyBytes>>,
    data: Option<&Bound<'_, PyDict>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyDict>>,
    auth: Option<Auth>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<Proxy>,
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
    url: &str,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyBytes>>,
    data: Option<&Bound<'_, PyDict>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyDict>>,
    auth: Option<Auth>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<Proxy>,
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
    url: &str,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    content: Option<&Bound<'_, PyBytes>>,
    data: Option<&Bound<'_, PyDict>>,
    json: Option<&Bound<'_, PyAny>>,
    files: Option<&Bound<'_, PyDict>>,
    auth: Option<Auth>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<Proxy>,
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
    url: &str,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<Auth>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<Proxy>,
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
    url: &str,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<Auth>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<Proxy>,
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
    url: &str,
    params: Option<&Bound<'_, PyDict>>,
    headers: Option<&Bound<'_, PyAny>>,
    cookies: Option<&Bound<'_, PyAny>>,
    auth: Option<Auth>,
    timeout: Option<&Bound<'_, PyAny>>,
    follow_redirects: bool,
    verify: Option<&Bound<'_, PyAny>>,
    proxy: Option<Proxy>,
) -> PyResult<Response> {
    request("OPTIONS", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects, verify, proxy)
}
