use hyper::{HeaderMap, Method, Uri};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use sonic_rs::Value;
use std::collections::HashMap;

use std::time::Duration;

mod auth;
mod config;
mod core;
mod error;
mod response;
mod session;

use auth::{get_auth_from_url, urldefragauth};
use core::client::{RequestConfig, RequestData, RequestxClient, ResponseData};
use error::RequestxError;
use response::{CaseInsensitivePyDict, Response};
use session::Session;

/// Parse and validate URL with comprehensive error handling
fn parse_and_validate_url(url: &str) -> PyResult<(Uri, Option<(String, String)>)> {
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
        Some("http") | Some("https") => {
            // Extract auth from URL if present
            let auth = uri
                .authority()
                .and_then(|a| a.as_str().split('@').next())
                .and_then(|userinfo| {
                    let parts: Vec<&str> = userinfo.split(':').collect();
                    if parts.len() >= 2 {
                        Some((parts[0].to_string(), parts[1..].join(":")))
                    } else {
                        None
                    }
                });
            Ok((uri, auth))
        }
        Some(scheme) => Err(RequestxError::InvalidSchema(scheme.to_string()).into()),
        None => Err(RequestxError::MissingSchema.into()),
    }
}

/// Parse kwargs into RequestConfig with comprehensive parameter support
fn parse_kwargs(py: Python, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<RequestConfigBuilder> {
    let mut builder = RequestConfigBuilder::new();

    if let Some(kwargs) = kwargs {
        // Parse headers
        if let Some(headers_obj) = kwargs.get_item("headers")? {
            let headers = parse_headers(&headers_obj)?;
            builder.headers = Some(headers);
        }

        // Parse params (query parameters)
        if let Some(params_obj) = kwargs.get_item("params")? {
            let params = parse_params(&params_obj)?;
            builder.params = Some(params);
        }

        // Parse data
        if let Some(data_obj) = kwargs.get_item("data")? {
            let data = parse_data(&data_obj)?;
            builder.data = Some(data);
        }

        // Parse json
        if let Some(json_obj) = kwargs.get_item("json")? {
            let json = parse_json(py, &json_obj)?;
            builder.json = Some(json);
        }

        // Parse timeout
        if let Some(timeout_obj) = kwargs.get_item("timeout")? {
            if !timeout_obj.is_none() {
                let timeout = parse_timeout(&timeout_obj)?;
                builder.timeout = Some(timeout);
            }
        }

        // Parse allow_redirects
        if let Some(redirects_obj) = kwargs.get_item("allow_redirects")? {
            builder.allow_redirects = redirects_obj.is_truthy()?;
        }

        // Parse verify
        if let Some(verify_obj) = kwargs.get_item("verify")? {
            builder.verify = verify_obj.is_truthy()?;
        }

        // Parse cert
        if let Some(cert_obj) = kwargs.get_item("cert")? {
            if !cert_obj.is_none() {
                let cert = parse_cert(&cert_obj)?;
                builder.cert = Some(cert);
            }
        }

        // Parse proxies
        if let Some(proxies_obj) = kwargs.get_item("proxies")? {
            if !proxies_obj.is_none() {
                let proxies = parse_proxies(&proxies_obj)?;
                builder.proxies = Some(proxies);
            }
        }

        // Parse auth
        if let Some(auth_obj) = kwargs.get_item("auth")? {
            if !auth_obj.is_none() {
                if let Some(auth) = parse_auth(&auth_obj)? {
                    builder.auth = Some(auth);
                }
            }
        }

        // Parse files for multipart upload
        if let Some(files_obj) = kwargs.get_item("files")? {
            if !files_obj.is_none() {
                let files = parse_files(&files_obj)?;
                builder.files = Some(files);
            }
        }

        // Parse stream
        if let Some(stream_obj) = kwargs.get_item("stream")? {
            builder.stream = stream_obj.is_truthy()?;
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
    pub files: Option<Vec<core::client::FilePart>>,
    pub timeout: Option<Duration>,
    pub allow_redirects: bool,
    pub max_redirects: Option<u32>,
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
            files: None,
            timeout: None,
            allow_redirects: true,
            max_redirects: None,
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
            files: self.files,
            timeout: self.timeout,
            allow_redirects: self.allow_redirects,
            max_redirects: self.max_redirects,
            verify: self.verify,
            cert: self.cert,
            proxies: self.proxies,
            auth: self.auth,
            stream: self.stream,
        }
    }
}

/// Parse headers from Python object with comprehensive error handling
fn parse_headers(headers_obj: &Bound<'_, PyAny>) -> PyResult<HeaderMap> {
    let mut headers = HeaderMap::new();

    if let Ok(dict) = headers_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_str = value.extract::<String>()?;

            // Validate header name
            let header_name = key_str.parse::<hyper::header::HeaderName>().map_err(|e| {
                RequestxError::InvalidHeader(format!("Invalid header name '{key_str}': {e}"))
            })?;

            // Validate header value - ensure proper UTF-8 encoding
            let header_value = hyper::header::HeaderValue::from_str(&value_str).map_err(|e| {
                RequestxError::InvalidHeader(format!("Invalid header value '{value_str}': {e}"))
            })?;

            headers.insert(header_name, header_value);
        }
    }

    Ok(headers)
}

/// Parse query parameters from Python object
fn parse_params(params_obj: &Bound<'_, PyAny>) -> PyResult<HashMap<String, String>> {
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
fn parse_data(data_obj: &Bound<'_, PyAny>) -> PyResult<RequestData> {
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
fn parse_json(py: Python, json_obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    // Use Python's json module to serialize the object
    let json_module = py.import("json")?;
    let json_str = json_module
        .call_method1("dumps", (json_obj,))?
        .extract::<String>()?;

    // Parse the JSON string into sonic_rs::Value
    sonic_rs::from_str(&json_str).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to parse JSON: {e}"))
    })
}

/// Parse timeout from Python object with comprehensive validation
fn parse_timeout(timeout_obj: &Bound<'_, PyAny>) -> PyResult<Duration> {
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
fn parse_cert(cert_obj: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(cert_path) = cert_obj.extract::<String>() {
        Ok(cert_path)
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Certificate must be a string path",
        ))
    }
}

/// Parse proxies from Python object
fn parse_proxies(proxies_obj: &Bound<'_, PyAny>) -> PyResult<HashMap<String, String>> {
    let mut proxies = HashMap::new();

    if let Ok(dict) = proxies_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let protocol = key.extract::<String>()?;
            let proxy_url = value.extract::<String>()?;

            // Validate proxy URL format
            if !proxy_url.contains("://") {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid proxy URL format: {proxy_url}"
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

/// Parse files from Python object for multipart upload
///
/// Supports formats:
/// - `{'fieldname': file_object}` - file object with fieldname as key
/// - `{'fieldname': ('filename', file_object)}` - with custom filename
/// - `{'fieldname': ('filename', file_object, 'content_type')}` - with content type
/// - List format for multiple files with same fieldname
fn parse_files(files_obj: &Bound<'_, PyAny>) -> PyResult<Vec<core::client::FilePart>> {
    let mut file_parts = Vec::new();

    // Handle list format: [('fieldname', ('filename', data)), ...]
    if let Ok(list) = files_obj.downcast::<pyo3::types::PyList>() {
        for i in 0..list.len() {
            let item = list.get_item(i)?;
            // Try to extract as tuple: (field_name, file_info)
            let tuple = item.downcast::<pyo3::types::PyTuple>()?;
            if tuple.len() == 2 {
                let field_name = tuple.get_item(0).unwrap().extract::<String>()?;
                let file_info = tuple.get_item(1).unwrap();

                let parts = parse_single_file(&field_name, file_info)?;
                file_parts.extend(parts);
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "File tuple must have 2 elements: (fieldname, file_data)",
                ));
            }
        }
    }
    // Handle dict format: {'fieldname': file_or_tuple}
    else if let Ok(dict) = files_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let field_name = key.extract::<String>()?;
            let parts = parse_single_file(&field_name, value)?;
            file_parts.extend(parts);
        }
    }
    // Handle single file passed directly (not in dict)
    else {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Files must be a dictionary or list of tuples",
        ));
    }

    Ok(file_parts)
}

/// Parse a single file entry into FilePart(s)
fn parse_single_file(
    field_name: &str,
    file_info: Bound<'_, PyAny>,
) -> PyResult<Vec<core::client::FilePart>> {
    let mut parts = Vec::new();

    // Try: ('filename', file_object_or_data)
    if let Ok(tuple) = file_info.downcast::<pyo3::types::PyTuple>() {
        if tuple.len() == 2 || tuple.len() == 3 {
            let filename = tuple.get_item(0).unwrap().extract::<String>()?;
            let file_data = tuple.get_item(1).unwrap();

            let content_type = if tuple.len() == 3 {
                Some(tuple.get_item(2).unwrap().extract::<String>()?)
            } else {
                detect_content_type(&filename)
            };

            // Extract file data - can be file object, bytes, or string
            let data = extract_file_data(file_data)?;

            parts.push(core::client::FilePart {
                field_name: field_name.to_string(),
                filename,
                content_type,
                data,
            });
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "File tuple must have 2 or 3 elements: ('filename', data) or ('filename', data, 'content_type')",
            ));
        }
    }
    // Try: file object directly
    else {
        // For file objects, try to get name for filename
        let filename = if let Ok(name_attr) = file_info.getattr("name") {
            if let Ok(name) = name_attr.extract::<String>() {
                name
            } else {
                "file".to_string()
            }
        } else {
            "file".to_string()
        };

        let content_type = detect_content_type(&filename);
        let data = extract_file_data(file_info)?;

        parts.push(core::client::FilePart {
            field_name: field_name.to_string(),
            filename,
            content_type,
            data,
        });
    }

    Ok(parts)
}

/// Extract file data from various Python objects
fn extract_file_data(obj: Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    // Try: file object with read() method
    if let Ok(read_method) = obj.getattr("read") {
        let content = read_method.call0()?.extract::<Vec<u8>>()?;
        // Reset file position if possible
        let _ = obj.call_method1("seek", (0,));
        return Ok(content);
    }

    // Try: bytes
    if let Ok(bytes) = obj.extract::<Vec<u8>>() {
        return Ok(bytes);
    }

    // Try: string
    if let Ok(string) = obj.extract::<String>() {
        return Ok(string.into_bytes());
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "File data must be a file object, bytes, or string",
    ))
}

/// Detect content type from filename
fn detect_content_type(filename: &str) -> Option<String> {
    let ext = filename.rsplit('.').next()?.to_lowercase();

    match ext.as_str() {
        "txt" => Some("text/plain".to_string()),
        "html" | "htm" => Some("text/html".to_string()),
        "css" => Some("text/css".to_string()),
        "js" | "mjs" => Some("application/javascript".to_string()),
        "json" => Some("application/json".to_string()),
        "xml" => Some("application/xml".to_string()),
        "pdf" => Some("application/pdf".to_string()),
        "zip" => Some("application/zip".to_string()),
        "tar" => Some("application/x-tar".to_string()),
        "gz" => Some("application/gzip".to_string()),
        "png" => Some("image/png".to_string()),
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "gif" => Some("image/gif".to_string()),
        "svg" => Some("image/svg+xml".to_string()),
        "ico" => Some("image/x-icon".to_string()),
        "mp3" => Some("audio/mpeg".to_string()),
        "mp4" => Some("video/mp4".to_string()),
        "wav" => Some("audio/wav".to_string()),
        "csv" => Some("text/csv".to_string()),
        _ => Some("application/octet-stream".to_string()),
    }
}

/// Parse authentication from Python object - supports tuple, list, and auth objects
fn parse_auth(auth_obj: &Bound<'_, PyAny>) -> PyResult<Option<(String, String)>> {
    // Try tuple/list first
    if let Ok(tuple) = auth_obj.extract::<(String, String)>() {
        return Ok(Some(tuple));
    }

    // Try list
    if let Ok(list) = auth_obj.downcast::<pyo3::types::PyList>() {
        if list.len() == 2 {
            let username = list.get_item(0).unwrap().extract::<String>()?;
            let password = list.get_item(1).unwrap().extract::<String>()?;
            return Ok(Some((username, password)));
        }
    }

    // Try extracting as dict with 'username' and 'password' keys (auth object style)
    if let Ok(dict) = auth_obj.downcast::<PyDict>() {
        if let (Some(username_obj), Some(password_obj)) =
            (dict.get_item("username")?, dict.get_item("password")?)
        {
            let username = username_obj.extract::<String>()?;
            let password = password_obj.extract::<String>()?;
            return Ok(Some((username, password)));
        }
    }

    // Check for auth object with username/password attributes (e.g., HTTPDigestAuth)
    if let Ok(username_attr) = auth_obj.getattr("username") {
        if let Ok(password_attr) = auth_obj.getattr("password") {
            let username = username_attr.extract::<String>()?;
            let password = password_attr.extract::<String>()?;
            return Ok(Some((username, password)));
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

    let mut response = Response::new(
        response_data.status_code,
        response_data.url.to_string(),
        headers,
        response_data.body.to_vec(),
        response_data.is_stream,
        response_data.elapsed_us,
    );

    // Convert history ResponseData items to Response objects
    let history: Vec<Response> = response_data
        .history
        .into_iter()
        .map(|history_data| {
            let history_headers = history_data
                .headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
                .collect();

            Response::new(
                history_data.status_code,
                history_data.url.to_string(),
                history_headers,
                history_data.body.to_vec(),
                history_data.is_stream,
                history_data.elapsed_us,
            )
        })
        .collect();

    response.history = history;

    Ok(response)
}

/// HTTP GET request with enhanced async/sync context detection
#[pyfunction(signature = (url, /, **kwargs))]
fn get(py: Python, url: String, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
    let (uri, url_auth) = parse_and_validate_url(&url)?;
    let mut config_builder = parse_kwargs(py, kwargs)?;

    // Merge URL auth with kwargs auth (kwargs auth takes precedence)
    if let Some(url_auth) = url_auth {
        if config_builder.auth.is_none() {
            config_builder.auth = Some(url_auth);
        }
    }

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
fn post(py: Python, url: String, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
    let (uri, url_auth) = parse_and_validate_url(&url)?;
    let mut config_builder = parse_kwargs(py, kwargs)?;

    // Merge URL auth with kwargs auth (kwargs auth takes precedence)
    if let Some(url_auth) = url_auth {
        if config_builder.auth.is_none() {
            config_builder.auth = Some(url_auth);
        }
    }

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
fn put(py: Python, url: String, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
    let (uri, url_auth) = parse_and_validate_url(&url)?;
    let mut config_builder = parse_kwargs(py, kwargs)?;

    // Merge URL auth with kwargs auth (kwargs auth takes precedence)
    if let Some(url_auth) = url_auth {
        if config_builder.auth.is_none() {
            config_builder.auth = Some(url_auth);
        }
    }

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
fn delete(py: Python, url: String, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
    let (uri, url_auth) = parse_and_validate_url(&url)?;
    let mut config_builder = parse_kwargs(py, kwargs)?;

    // Merge URL auth with kwargs auth (kwargs auth takes precedence)
    if let Some(url_auth) = url_auth {
        if config_builder.auth.is_none() {
            config_builder.auth = Some(url_auth);
        }
    }

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
fn head(py: Python, url: String, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
    let (uri, url_auth) = parse_and_validate_url(&url)?;
    let mut config_builder = parse_kwargs(py, kwargs)?;

    // Merge URL auth with kwargs auth (kwargs auth takes precedence)
    if let Some(url_auth) = url_auth {
        if config_builder.auth.is_none() {
            config_builder.auth = Some(url_auth);
        }
    }

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
fn options(py: Python, url: String, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
    let (uri, url_auth) = parse_and_validate_url(&url)?;
    let mut config_builder = parse_kwargs(py, kwargs)?;

    // Merge URL auth with kwargs auth (kwargs auth takes precedence)
    if let Some(url_auth) = url_auth {
        if config_builder.auth.is_none() {
            config_builder.auth = Some(url_auth);
        }
    }

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
fn patch(py: Python, url: String, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
    let (uri, url_auth) = parse_and_validate_url(&url)?;
    let mut config_builder = parse_kwargs(py, kwargs)?;

    // Merge URL auth with kwargs auth (kwargs auth takes precedence)
    if let Some(url_auth) = url_auth {
        if config_builder.auth.is_none() {
            config_builder.auth = Some(url_auth);
        }
    }

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
