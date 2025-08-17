use hyper::{HeaderMap, Method, Uri};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use serde_json::Value;
use std::collections::HashMap;

use std::time::Duration;

mod core;
mod error;
mod response;
mod session;

use core::client::{RequestConfig, RequestData, RequestxClient, ResponseData};
use error::RequestxError;
use response::Response;
use session::Session;

/// Parse and validate URL with comprehensive error handling
fn parse_and_validate_url(url: &str) -> PyResult<Uri> {
    // Check for empty URL
    if url.is_empty() {
        return Err(RequestxError::UrlRequired.into());
    }

    // Check for missing schema
    if !url.contains("://") {
        return Err(RequestxError::MissingSchema.into());
    }

    // Parse the URL
    let uri: Uri = url.parse().map_err(|e: hyper::http::uri::InvalidUri| {
        let error_str = e.to_string();
        if error_str.contains("scheme") {
            RequestxError::InvalidSchema(url.to_string())
        } else {
            RequestxError::InvalidUrl(e)
        }
    })?;

    // Validate schema
    match uri.scheme_str() {
        Some("http") | Some("https") => Ok(uri),
        Some(scheme) => Err(RequestxError::InvalidSchema(scheme.to_string()).into()),
        None => Err(RequestxError::MissingSchema.into()),
    }
}

/// Parse kwargs into RequestConfig with comprehensive parameter support
fn parse_kwargs(py: Python, kwargs: Option<&PyDict>) -> PyResult<RequestConfigBuilder> {
    let mut builder = RequestConfigBuilder::new();

    if let Some(kwargs) = kwargs {
        // Parse headers
        if let Some(headers_obj) = kwargs.get_item("headers")? {
            let headers = parse_headers(headers_obj)?;
            builder.headers = Some(headers);
        }

        // Parse params (query parameters)
        if let Some(params_obj) = kwargs.get_item("params")? {
            let params = parse_params(params_obj)?;
            builder.params = Some(params);
        }

        // Parse data
        if let Some(data_obj) = kwargs.get_item("data")? {
            let data = parse_data(data_obj)?;
            builder.data = Some(data);
        }

        // Parse json
        if let Some(json_obj) = kwargs.get_item("json")? {
            let json = parse_json(py, json_obj)?;
            builder.json = Some(json);
        }

        // Parse timeout
        if let Some(timeout_obj) = kwargs.get_item("timeout")? {
            if !timeout_obj.is_none() {
                let timeout = parse_timeout(timeout_obj)?;
                builder.timeout = Some(timeout);
            }
        }

        // Parse allow_redirects
        if let Some(redirects_obj) = kwargs.get_item("allow_redirects")? {
            builder.allow_redirects = redirects_obj.is_true()?;
        }

        // Parse verify
        if let Some(verify_obj) = kwargs.get_item("verify")? {
            builder.verify = verify_obj.is_true()?;
        }

        // Parse cert
        if let Some(cert_obj) = kwargs.get_item("cert")? {
            if !cert_obj.is_none() {
                let cert = parse_cert(cert_obj)?;
                builder.cert = Some(cert);
            }
        }

        // Parse proxies
        if let Some(proxies_obj) = kwargs.get_item("proxies")? {
            if !proxies_obj.is_none() {
                let proxies = parse_proxies(proxies_obj)?;
                builder.proxies = Some(proxies);
            }
        }

        // Parse auth
        if let Some(auth_obj) = kwargs.get_item("auth")? {
            if !auth_obj.is_none() {
                let auth = parse_auth(auth_obj)?;
                builder.auth = Some(auth);
            }
        }

        // Parse stream
        if let Some(stream_obj) = kwargs.get_item("stream")? {
            builder.stream = stream_obj.is_true()?;
        }
    }

    Ok(builder)
}

/// Helper struct for building RequestConfig
#[derive(Debug, Clone)]
struct RequestConfigBuilder {
    pub headers: Option<HeaderMap>,
    pub params: Option<HashMap<String, String>>,
    pub data: Option<RequestData>,
    pub json: Option<Value>,
    pub timeout: Option<Duration>,
    pub allow_redirects: bool,
    pub verify: bool,
    pub cert: Option<String>,
    pub proxies: Option<HashMap<String, String>>,
    pub auth: Option<(String, String)>,
    pub stream: bool,
}

impl RequestConfigBuilder {
    fn new() -> Self {
        Self {
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
            cert: None,
            proxies: None,
            auth: None,
            stream: false,
        }
    }

    fn build(self, method: Method, url: Uri) -> RequestConfig {
        RequestConfig {
            method,
            url,
            headers: self.headers,
            params: self.params,
            data: self.data,
            json: self.json,
            timeout: self.timeout,
            allow_redirects: self.allow_redirects,
            verify: self.verify,
            cert: self.cert,
            proxies: self.proxies,
            auth: self.auth,
            stream: self.stream,
        }
    }
}

/// Parse headers from Python object with comprehensive error handling
fn parse_headers(headers_obj: &PyAny) -> PyResult<HeaderMap> {
    let mut headers = HeaderMap::new();

    if let Ok(dict) = headers_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_str = value.extract::<String>()?;

            // Validate header name
            let header_name = key_str.parse::<hyper::header::HeaderName>().map_err(|e| {
                RequestxError::InvalidHeader(format!("Invalid header name '{}': {}", key_str, e))
            })?;

            // Validate header value - ensure proper UTF-8 encoding
            let header_value = hyper::header::HeaderValue::from_str(&value_str).map_err(|e| {
                RequestxError::InvalidHeader(format!("Invalid header value '{}': {}", value_str, e))
            })?;

            headers.insert(header_name, header_value);
        }
    }

    Ok(headers)
}

/// Parse query parameters from Python object
fn parse_params(params_obj: &PyAny) -> PyResult<HashMap<String, String>> {
    let mut params = HashMap::new();

    if let Ok(dict) = params_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_str = value.extract::<String>()?;
            params.insert(key_str, value_str);
        }
    }

    Ok(params)
}

/// Parse request data from Python object
fn parse_data(data_obj: &PyAny) -> PyResult<RequestData> {
    // Try string first
    if let Ok(text) = data_obj.extract::<String>() {
        return Ok(RequestData::Text(text));
    }

    // Try bytes
    if let Ok(bytes) = data_obj.extract::<Vec<u8>>() {
        return Ok(RequestData::Bytes(bytes));
    }

    // Try dict (form data)
    if let Ok(dict) = data_obj.downcast::<PyDict>() {
        let mut form_data = HashMap::new();
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_str = value.extract::<String>()?;
            form_data.insert(key_str, value_str);
        }
        return Ok(RequestData::Form(form_data));
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Data must be string, bytes, or dict",
    ))
}

/// Parse JSON data from Python object
fn parse_json(py: Python, json_obj: &PyAny) -> PyResult<Value> {
    // Use Python's json module to serialize the object
    let json_module = py.import("json")?;
    let json_str = json_module
        .call_method1("dumps", (json_obj,))?
        .extract::<String>()?;

    // Parse the JSON string into serde_json::Value
    serde_json::from_str(&json_str).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to parse JSON: {}", e))
    })
}

/// Parse timeout from Python object with comprehensive validation
fn parse_timeout(timeout_obj: &PyAny) -> PyResult<Duration> {
    if let Ok(seconds) = timeout_obj.extract::<f64>() {
        if seconds < 0.0 {
            return Err(
                RequestxError::RuntimeError("Timeout must be non-negative".to_string()).into(),
            );
        }
        if seconds > 3600.0 {
            // 1 hour max
            return Err(RequestxError::RuntimeError(
                "Timeout too large (max 3600 seconds)".to_string(),
            )
            .into());
        }
        Ok(Duration::from_secs_f64(seconds))
    } else if let Ok(seconds) = timeout_obj.extract::<u64>() {
        if seconds > 3600 {
            // 1 hour max
            return Err(RequestxError::RuntimeError(
                "Timeout too large (max 3600 seconds)".to_string(),
            )
            .into());
        }
        Ok(Duration::from_secs(seconds))
    } else {
        Err(RequestxError::RuntimeError("Timeout must be a number".to_string()).into())
    }
}

/// Parse certificate from Python object
fn parse_cert(cert_obj: &PyAny) -> PyResult<String> {
    if let Ok(cert_path) = cert_obj.extract::<String>() {
        Ok(cert_path)
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Certificate must be a string path",
        ))
    }
}

/// Parse proxies from Python object
fn parse_proxies(proxies_obj: &PyAny) -> PyResult<HashMap<String, String>> {
    let mut proxies = HashMap::new();

    if let Ok(dict) = proxies_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let protocol = key.extract::<String>()?;
            let proxy_url = value.extract::<String>()?;

            // Validate proxy URL format
            if !proxy_url.contains("://") {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid proxy URL format: {}",
                    proxy_url
                )));
            }

            proxies.insert(protocol, proxy_url);
        }
    } else {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Proxies must be a dictionary",
        ));
    }

    Ok(proxies)
}

/// Parse authentication from Python object
fn parse_auth(auth_obj: &PyAny) -> PyResult<(String, String)> {
    // Try tuple/list first
    if let Ok(tuple) = auth_obj.extract::<(String, String)>() {
        return Ok(tuple);
    }

    // Try list
    if let Ok(list) = auth_obj.downcast::<pyo3::types::PyList>() {
        if list.len() == 2 {
            let username = list.get_item(0)?.extract::<String>()?;
            let password = list.get_item(1)?.extract::<String>()?;
            return Ok((username, password));
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Auth must be a tuple or list of (username, password)",
    ))
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

/// HTTP GET request with enhanced async/sync context detection
#[pyfunction(signature = (url, /, **kwargs))]
fn get(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = parse_and_validate_url(&url)?;
    let config_builder = parse_kwargs(py, kwargs)?;
    let config = config_builder.build(Method::GET, uri);

    // Use enhanced runtime management for context detection and execution
    let runtime_manager = core::runtime::get_global_runtime_manager();

    let future = async move {
        let client = RequestxClient::new()?;
        let response_data = client.request_async(config).await?;
        response_data_to_py_response(response_data)
    };

    runtime_manager.execute_future(py, future)
}

/// HTTP POST request with enhanced async/sync context detection
#[pyfunction(signature = (url, /, **kwargs))]
fn post(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = parse_and_validate_url(&url)?;
    let config_builder = parse_kwargs(py, kwargs)?;
    let config = config_builder.build(Method::POST, uri);

    // Use enhanced runtime management for context detection and execution
    let runtime_manager = core::runtime::get_global_runtime_manager();

    let future = async move {
        let client = RequestxClient::new()?;
        let response_data = client.request_async(config).await?;
        response_data_to_py_response(response_data)
    };

    runtime_manager.execute_future(py, future)
}

/// HTTP PUT request with enhanced async/sync context detection
#[pyfunction(signature = (url, /, **kwargs))]
fn put(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = parse_and_validate_url(&url)?;
    let config_builder = parse_kwargs(py, kwargs)?;
    let config = config_builder.build(Method::PUT, uri);

    // Use enhanced runtime management for context detection and execution
    let runtime_manager = core::runtime::get_global_runtime_manager();

    let future = async move {
        let client = RequestxClient::new()?;
        let response_data = client.request_async(config).await?;
        response_data_to_py_response(response_data)
    };

    runtime_manager.execute_future(py, future)
}

/// HTTP DELETE request with enhanced async/sync context detection
#[pyfunction(signature = (url, /, **kwargs))]
fn delete(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = parse_and_validate_url(&url)?;
    let config_builder = parse_kwargs(py, kwargs)?;
    let config = config_builder.build(Method::DELETE, uri);

    // Use enhanced runtime management for context detection and execution
    let runtime_manager = core::runtime::get_global_runtime_manager();

    let future = async move {
        let client = RequestxClient::new()?;
        let response_data = client.request_async(config).await?;
        response_data_to_py_response(response_data)
    };

    runtime_manager.execute_future(py, future)
}

/// HTTP HEAD request with enhanced async/sync context detection
#[pyfunction(signature = (url, /, **kwargs))]
fn head(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = parse_and_validate_url(&url)?;
    let config_builder = parse_kwargs(py, kwargs)?;
    let config = config_builder.build(Method::HEAD, uri);

    // Use enhanced runtime management for context detection and execution
    let runtime_manager = core::runtime::get_global_runtime_manager();

    let future = async move {
        let client = RequestxClient::new()?;
        let response_data = client.request_async(config).await?;
        response_data_to_py_response(response_data)
    };

    runtime_manager.execute_future(py, future)
}

/// HTTP OPTIONS request with enhanced async/sync context detection
#[pyfunction(signature = (url, /, **kwargs))]
fn options(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = parse_and_validate_url(&url)?;
    let config_builder = parse_kwargs(py, kwargs)?;
    let config = config_builder.build(Method::OPTIONS, uri);

    // Use enhanced runtime management for context detection and execution
    let runtime_manager = core::runtime::get_global_runtime_manager();

    let future = async move {
        let client = RequestxClient::new()?;
        let response_data = client.request_async(config).await?;
        response_data_to_py_response(response_data)
    };

    runtime_manager.execute_future(py, future)
}

/// HTTP PATCH request with enhanced async/sync context detection
#[pyfunction(signature = (url, /, **kwargs))]
fn patch(py: Python, url: String, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let uri: Uri = parse_and_validate_url(&url)?;
    let config_builder = parse_kwargs(py, kwargs)?;
    let config = config_builder.build(Method::PATCH, uri);

    // Use enhanced runtime management for context detection and execution
    let runtime_manager = core::runtime::get_global_runtime_manager();

    let future = async move {
        let client = RequestxClient::new()?;
        let response_data = client.request_async(config).await?;
        response_data_to_py_response(response_data)
    };

    runtime_manager.execute_future(py, future)
}

/// Generic HTTP request with enhanced async/sync context detection
#[pyfunction(signature = (method, url, /, **kwargs))]
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
        _ => {
            return Err(
                RequestxError::RuntimeError(format!("Invalid HTTP method: {}", method)).into(),
            )
        }
    };

    let uri: Uri = parse_and_validate_url(&url)?;
    let config_builder = parse_kwargs(py, kwargs)?;
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
fn _requestx(py: Python, m: &PyModule) -> PyResult<()> {
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

    // Register custom exceptions
    error::register_exceptions(py, m)?;

    Ok(())
}
