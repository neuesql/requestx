//! HTTP Client implementations for requestx

use crate::error::{Error, Result};
use crate::response::Response;
use crate::streaming::{AsyncStreamingResponse, StreamingResponse};
use crate::types::{
    extract_auth, extract_cert, extract_cookies, extract_headers, extract_limits, extract_params, extract_proxy, extract_timeout, extract_verify, get_env_proxy, get_env_ssl_cert, Auth, AuthType, Cookies, Headers, Limits, Proxy,
    Request, Timeout, URL,
};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use reqwest::redirect::Policy;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read as IoRead;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

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

/// Shared client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: Option<String>,
    pub headers: Headers,
    pub cookies: Cookies,
    pub timeout: Timeout,
    pub follow_redirects: bool,
    pub max_redirects: usize,
    pub verify_ssl: bool,
    pub ca_bundle: Option<String>,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
    pub key_password: Option<String>,
    pub proxy: Option<Proxy>,
    pub auth: Option<Auth>,
    pub http2: bool,
    pub limits: Limits,
    pub default_encoding: Option<String>,
    pub trust_env: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            headers: Headers::default(),
            cookies: Cookies::default(),
            timeout: Timeout::default(),
            follow_redirects: true,
            max_redirects: 10,
            verify_ssl: true,
            ca_bundle: None,
            cert_file: None,
            key_file: None,
            key_password: None,
            proxy: None,
            auth: None,
            http2: false,
            limits: Limits::default(),
            default_encoding: None,
            trust_env: true,
        }
    }
}

/// Load certificate from PEM file
fn load_cert_pem(path: &str) -> Result<Vec<reqwest::Certificate>> {
    let mut file = File::open(path).map_err(|e| Error::request(format!("Failed to open cert file: {e}")))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| Error::request(format!("Failed to read cert file: {e}")))?;

    let cert = reqwest::Certificate::from_pem(&buf).map_err(|e| Error::request(format!("Failed to parse cert: {e}")))?;
    Ok(vec![cert])
}

/// Load identity (client cert + key) from PEM files
fn load_identity_pem(cert_path: &str, key_path: Option<&str>) -> Result<reqwest::Identity> {
    let mut cert_buf = Vec::new();
    File::open(cert_path)
        .map_err(|e| Error::request(format!("Failed to open cert file: {e}")))?
        .read_to_end(&mut cert_buf)
        .map_err(|e| Error::request(format!("Failed to read cert file: {e}")))?;

    if let Some(key_path) = key_path {
        // Separate key file - combine them
        let mut key_buf = Vec::new();
        File::open(key_path)
            .map_err(|e| Error::request(format!("Failed to open key file: {e}")))?
            .read_to_end(&mut key_buf)
            .map_err(|e| Error::request(format!("Failed to read key file: {e}")))?;

        // Combine cert and key
        cert_buf.extend_from_slice(b"\n");
        cert_buf.extend_from_slice(&key_buf);
    }

    reqwest::Identity::from_pem(&cert_buf).map_err(|e| Error::request(format!("Failed to create identity: {e}")))
}

/// Build reqwest client from config
fn build_reqwest_client(config: &ClientConfig) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();

    // Timeout configuration
    if let Some(timeout) = config.timeout.total {
        builder = builder.timeout(timeout);
    }
    if let Some(connect) = config.timeout.connect {
        builder = builder.connect_timeout(connect);
    }
    if let Some(read) = config.timeout.read {
        builder = builder.read_timeout(read);
    }
    if let Some(pool) = config.timeout.pool {
        builder = builder.pool_idle_timeout(pool);
    }

    // Resource limits
    if let Some(max_idle) = config.limits.max_keepalive_connections {
        builder = builder.pool_max_idle_per_host(max_idle);
    }

    // Redirect policy
    if config.follow_redirects {
        builder = builder.redirect(Policy::limited(config.max_redirects));
    } else {
        builder = builder.redirect(Policy::none());
    }

    // SSL verification
    if !config.verify_ssl {
        builder = builder.danger_accept_invalid_certs(true);
    }

    // Custom CA bundle
    let ca_bundle = config.ca_bundle.clone().or_else(|| {
        if config.trust_env {
            get_env_ssl_cert()
        } else {
            None
        }
    });
    if let Some(ref ca_path) = ca_bundle {
        for cert in load_cert_pem(ca_path)? {
            builder = builder.add_root_certificate(cert);
        }
    }

    // Client certificate
    if let Some(ref cert_path) = config.cert_file {
        let identity = load_identity_pem(cert_path, config.key_file.as_deref())?;
        builder = builder.identity(identity);
    }

    // HTTP/2
    if config.http2 {
        builder = builder.http2_prior_knowledge();
    }

    // Proxy configuration
    let proxy = config.proxy.clone().or_else(|| {
        if config.trust_env {
            get_env_proxy()
        } else {
            None
        }
    });
    if let Some(ref proxy_config) = proxy {
        if let Some(ref all_proxy) = proxy_config.all {
            if let Ok(p) = reqwest::Proxy::all(all_proxy) {
                builder = builder.proxy(p);
            }
        } else {
            if let Some(ref http_proxy) = proxy_config.http {
                if let Ok(p) = reqwest::Proxy::http(http_proxy) {
                    builder = builder.proxy(p);
                }
            }
            if let Some(ref https_proxy) = proxy_config.https {
                if let Ok(p) = reqwest::Proxy::https(https_proxy) {
                    builder = builder.proxy(p);
                }
            }
        }
    }

    // Default headers
    builder = builder.default_headers(config.headers.to_reqwest_headers());

    // Cookie store
    builder = builder.cookie_store(true);

    builder.build().map_err(|e| Error::request(e.to_string()))
}

/// Build reqwest blocking client from config
fn build_blocking_client(config: &ClientConfig) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder();

    // Timeout configuration
    // Note: blocking client only supports total timeout and connect_timeout
    // read_timeout is applied via the total timeout for blocking client
    if let Some(timeout) = config.timeout.total {
        builder = builder.timeout(timeout);
    } else if let Some(read) = config.timeout.read {
        // Use read timeout as the general timeout if no total timeout is set
        builder = builder.timeout(read);
    }
    if let Some(connect) = config.timeout.connect {
        builder = builder.connect_timeout(connect);
    }

    // Resource limits
    if let Some(max_idle) = config.limits.max_keepalive_connections {
        builder = builder.pool_max_idle_per_host(max_idle);
    }

    // Redirect policy
    if config.follow_redirects {
        builder = builder.redirect(Policy::limited(config.max_redirects));
    } else {
        builder = builder.redirect(Policy::none());
    }

    // SSL verification
    if !config.verify_ssl {
        builder = builder.danger_accept_invalid_certs(true);
    }

    // Custom CA bundle
    let ca_bundle = config.ca_bundle.clone().or_else(|| {
        if config.trust_env {
            get_env_ssl_cert()
        } else {
            None
        }
    });
    if let Some(ref ca_path) = ca_bundle {
        for cert in load_cert_pem(ca_path)? {
            builder = builder.add_root_certificate(cert);
        }
    }

    // Client certificate
    if let Some(ref cert_path) = config.cert_file {
        let identity = load_identity_pem(cert_path, config.key_file.as_deref())?;
        builder = builder.identity(identity);
    }

    // HTTP/2
    if config.http2 {
        builder = builder.http2_prior_knowledge();
    }

    // Proxy configuration
    let proxy = config.proxy.clone().or_else(|| {
        if config.trust_env {
            get_env_proxy()
        } else {
            None
        }
    });
    if let Some(ref proxy_config) = proxy {
        if let Some(ref all_proxy) = proxy_config.all {
            if let Ok(p) = reqwest::Proxy::all(all_proxy) {
                builder = builder.proxy(p);
            }
        } else {
            if let Some(ref http_proxy) = proxy_config.http {
                if let Ok(p) = reqwest::Proxy::http(http_proxy) {
                    builder = builder.proxy(p);
                }
            }
            if let Some(ref https_proxy) = proxy_config.https {
                if let Ok(p) = reqwest::Proxy::https(https_proxy) {
                    builder = builder.proxy(p);
                }
            }
        }
    }

    // Default headers
    builder = builder.default_headers(config.headers.to_reqwest_headers());

    // Cookie store
    builder = builder.cookie_store(true);

    builder.build().map_err(|e| Error::request(e.to_string()))
}

/// Resolve URL with base URL
fn resolve_url(base_url: &Option<String>, url: &str) -> Result<String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        return Ok(url.to_string());
    }

    if let Some(ref base) = base_url {
        let base_url = url::Url::parse(base)?;
        let resolved = base_url.join(url)?;
        Ok(resolved.to_string())
    } else {
        Err(Error::invalid_url(format!("Relative URL '{url}' requires a base_url")))
    }
}

/// Synchronous HTTP Client
#[pyclass(name = "Client", subclass)]
pub struct Client {
    client: reqwest::blocking::Client,
    config: ClientConfig,
    /// Whether the client is closed
    closed: bool,
    /// Transport (for HTTPX compatibility)
    transport: Option<PyObject>,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (
        base_url=None,
        headers=None,
        cookies=None,
        timeout=None,
        follow_redirects=true,
        max_redirects=10,
        verify=None,
        cert=None,
        proxy=None,
        auth=None,
        http2=false,
        limits=None,
        default_encoding=None,
        trust_env=true,
        transport=None,
        app=None,
        event_hooks=None,
        mounts=None,
        params=None
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_url: Option<String>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: bool,
        max_redirects: usize,
        verify: Option<&Bound<'_, PyAny>>,
        cert: Option<&Bound<'_, PyAny>>,
        proxy: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        http2: bool,
        limits: Option<&Bound<'_, PyAny>>,
        default_encoding: Option<String>,
        trust_env: bool,
        transport: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] app: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] event_hooks: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] mounts: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] params: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let mut config = ClientConfig {
            base_url,
            follow_redirects,
            max_redirects,
            http2,
            default_encoding,
            trust_env,
            ..Default::default()
        };

        if let Some(h) = headers {
            config.headers = extract_headers(h)?;
        }
        if let Some(c) = cookies {
            config.cookies = Cookies { inner: extract_cookies(c)? };
        }
        if let Some(t) = timeout {
            config.timeout = extract_timeout(t)?;
        }
        if let Some(v) = verify {
            let (verify_ssl, ca_bundle) = extract_verify(v)?;
            config.verify_ssl = verify_ssl;
            config.ca_bundle = ca_bundle;
        }
        if let Some(c) = cert {
            let (cert_file, key_file, key_password) = extract_cert(c)?;
            config.cert_file = cert_file;
            config.key_file = key_file;
            config.key_password = key_password;
        }
        if let Some(p) = proxy {
            config.proxy = extract_proxy(p)?;
        }
        if let Some(a) = auth {
            config.auth = extract_auth(a)?;
        }
        if let Some(l) = limits {
            config.limits = extract_limits(l)?;
        }

        let client = build_blocking_client(&config)?;

        let transport_obj = transport.map(|t| t.clone().unbind());

        Ok(Self { client, config, closed: false, transport: transport_obj })
    }

    /// Whether the client is closed
    #[getter]
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Get the transport (HTTPX compatibility)
    #[getter]
    pub fn _transport(&self, py: Python<'_>) -> Option<PyObject> {
        self.transport.as_ref().map(|t| t.clone_ref(py))
    }

    /// Get the client timeout configuration
    #[getter]
    pub fn timeout(&self) -> Timeout {
        self.config.timeout.clone()
    }

    /// Get the base URL (HTTPX compatibility)
    #[getter]
    pub fn base_url(&self) -> Option<URL> {
        self.config.base_url.as_ref().and_then(|s| URL::from_str(s).ok())
    }

    /// Get the default headers (HTTPX compatibility)
    #[getter]
    pub fn headers(&self) -> Headers {
        self.config.headers.clone()
    }

    /// Build a request without sending it
    #[pyo3(signature = (
        method,
        url,
        params=None,
        headers=None,
        cookies=None,
        content=None,
        data=None,
        json=None,
        timeout=None
    ))]
    pub fn build_request(
        &self,
        method: &str,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyBytes>>,
        data: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] timeout: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Request> {
        // Accept both string and URL object
        let url_str = if let Ok(s) = url.extract::<String>() {
            s
        } else if let Ok(url_obj) = url.extract::<URL>() {
            url_obj.as_url().to_string()
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err("url must be a string or URL object"));
        };
        let resolved_url = resolve_url(&self.config.base_url, &url_str)?;
        let parsed_url = URL::from_str(&resolved_url)?;

        // Merge headers
        let mut final_headers = self.config.headers.clone();
        if let Some(h) = headers {
            let req_headers = extract_headers(h)?;
            for (key, values) in &req_headers.inner {
                for value in values {
                    final_headers.add(key, value);
                }
            }
        }

        // Add cookies to headers
        if let Some(c) = cookies {
            let cookies_map = extract_cookies(c)?;
            for (name, value) in &cookies_map {
                final_headers.add("cookie", &format!("{name}={value}"));
            }
        }
        for (name, value) in &self.config.cookies.inner {
            final_headers.add("cookie", &format!("{name}={value}"));
        }

        // Add query params to URL
        let final_url = if let Some(p) = params {
            let params_vec = extract_params(Some(p))?;
            if !params_vec.is_empty() {
                let mut parsed = url::Url::parse(&resolved_url).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {e}")))?;
                for (k, v) in params_vec {
                    parsed.query_pairs_mut().append_pair(&k, &v);
                }
                URL::from_url(parsed)
            } else {
                parsed_url
            }
        } else {
            parsed_url
        };

        // Build content
        let body_content = if let Some(json_data) = json {
            let json_str = py_to_json_string(json_data)?;
            final_headers.set("content-type", "application/json");
            Some(json_str.into_bytes())
        } else if let Some(form_data) = data {
            let form: HashMap<String, String> = form_data
                .iter()
                .map(|(k, v)| Ok((k.extract::<String>()?, v.extract::<String>()?)))
                .collect::<PyResult<_>>()?;
            let encoded = form
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            final_headers.set("content-type", "application/x-www-form-urlencoded");
            Some(encoded.into_bytes())
        } else {
            content.map(|body| body.as_bytes().to_vec())
        };

        Ok(Request::new_internal(method.to_uppercase(), final_url, final_headers, body_content, false))
    }

    /// Send a pre-built request
    #[pyo3(signature = (request, stream=false))]
    pub fn send(&self, py: Python<'_>, request: &Request, stream: bool) -> PyResult<Py<PyAny>> {
        if stream {
            let streaming_response = self.send_streaming(request)?;
            Ok(streaming_response.into_pyobject(py)?.into_any().unbind())
        } else {
            let response = self.send_request(request)?;
            Ok(response.into_pyobject(py)?.into_any().unbind())
        }
    }

    /// Perform a request
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
        follow_redirects=None
    ))]
    pub fn request(
        &self,
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
        #[allow(unused_variables)] follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        let url_str = extract_url_str(url)?;
        let resolved_url = resolve_url(&self.config.base_url, &url_str)?;
        let start = Instant::now();

        // Build request
        let mut req = self.client.request(
            method
                .parse()
                .map_err(|_| Error::request(format!("Invalid method: {method}")))?,
            &resolved_url,
        );

        // Add query parameters
        if let Some(p) = params {
            let params_vec = extract_params(Some(p))?;
            req = req.query(&params_vec);
        }

        // Add headers
        if let Some(h) = headers {
            let headers_obj = extract_headers(h)?;
            for (key, values) in &headers_obj.inner {
                for value in values {
                    req = req.header(key.as_str(), value.as_str());
                }
            }
        }

        // Add cookies
        if let Some(c) = cookies {
            let cookies_map = extract_cookies(c)?;
            for (name, value) in &cookies_map {
                req = req.header("Cookie", format!("{name}={value}"));
            }
        }

        // Add client-level cookies
        for (name, value) in &self.config.cookies.inner {
            req = req.header("Cookie", format!("{name}={value}"));
        }

        // Set body
        if let Some(json_data) = json {
            let json_str = py_to_json_string(json_data)?;
            req = req.header("Content-Type", "application/json");
            req = req.body(json_str);
        } else if let Some(form_data) = data {
            // Handle data parameter - can be dict or other form-encodable
            if form_data.is_instance_of::<PyDict>() {
                let dict = form_data.cast::<PyDict>().unwrap();
                let form: HashMap<String, String> = dict
                    .iter()
                    .map(|(k, v)| {
                        let key = k.extract::<String>()?;
                        // Handle None values as empty strings
                        let val = if v.is_none() {
                            String::new()
                        } else if let Ok(b) = v.extract::<bool>() {
                            // Handle boolean values HTTPX-style
                            if b { "true".to_string() } else { "false".to_string() }
                        } else {
                            v.str()?.to_string()
                        };
                        Ok((key, val))
                    })
                    .collect::<PyResult<_>>()?;
                req = req.form(&form);
            } else {
                // Try to extract as bytes
                if let Ok(bytes) = form_data.extract::<Vec<u8>>() {
                    req = req.body(bytes);
                } else if let Ok(s) = form_data.extract::<String>() {
                    req = req.body(s);
                }
            }
        } else if let Some(body) = content {
            // Handle content parameter - bytes, string, or iterator
            if body.is_instance_of::<PyBytes>() {
                let bytes_obj: Bound<'_, PyBytes> = body.extract()?;
                req = req.body(bytes_obj.as_bytes().to_vec());
            } else if let Ok(bytes) = body.extract::<Vec<u8>>() {
                req = req.body(bytes);
            } else if let Ok(s) = body.extract::<String>() {
                req = req.body(s);
            } else if body.hasattr("__iter__")? {
                // Handle iterator - collect all bytes
                let mut all_bytes = Vec::new();
                let iter = body.try_iter()?;
                for item_result in iter {
                    let item: Bound<'_, PyAny> = item_result?;
                    if let Ok(bytes) = item.extract::<Vec<u8>>() {
                        all_bytes.extend(bytes);
                    } else if let Ok(s) = item.extract::<String>() {
                        all_bytes.extend(s.into_bytes());
                    }
                }
                req = req.body(all_bytes);
            }
        } else if let Some(files_data) = files {
            // Handle files parameter - dict of field_name -> file_info
            if files_data.is_instance_of::<PyDict>() {
                let files_dict = files_data.extract::<Bound<'_, PyDict>>()?;
                let mut form = reqwest::blocking::multipart::Form::new();
                for (field_name, file_info) in files_dict.iter() {
                    let field_name: String = field_name.extract()?;
                    if let Ok(tuple) = file_info.extract::<(String, Vec<u8>, String)>() {
                        let (filename, file_content, content_type) = tuple;
                        let part = reqwest::blocking::multipart::Part::bytes(file_content)
                            .file_name(filename)
                            .mime_str(&content_type)
                            .map_err(|e| Error::request(e.to_string()))?;
                        form = form.part(field_name, part);
                    } else if let Ok(tuple) = file_info.extract::<(String, Vec<u8>)>() {
                        let (filename, file_content) = tuple;
                        let part = reqwest::blocking::multipart::Part::bytes(file_content).file_name(filename);
                        form = form.part(field_name, part);
                    }
                }
                req = req.multipart(form);
            }
        }

        // Authentication - extract from PyAny
        let auth_extracted = if let Some(a) = auth {
            extract_auth(a)?
        } else {
            None
        };
        let auth_to_use = auth_extracted.as_ref().or(self.config.auth.as_ref());
        if let Some(auth_config) = auth_to_use {
            match &auth_config.auth_type {
                AuthType::Basic { username, password } => {
                    req = req.basic_auth(username, Some(password));
                }
                AuthType::Bearer { token } => {
                    req = req.bearer_auth(token);
                }
                AuthType::Digest { username, password } => {
                    // Reqwest doesn't support digest auth natively, fall back to basic
                    req = req.basic_auth(username, Some(password));
                }
            }
        }

        // Timeout (per-request)
        if let Some(t) = timeout {
            let timeout_config = extract_timeout(t)?;
            if let Some(total) = timeout_config.total {
                req = req.timeout(total);
            }
        }

        // Execute request
        let response = req.send().map_err(Error::from)?;

        // Convert to our Response type with default encoding
        let status_code = response.status().as_u16();
        let reason_phrase = response
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();
        let final_url = response.url().to_string();
        let http_version = format!("{:?}", response.version());

        let resp_headers = Headers::from_reqwest_headers(response.headers());

        let mut cookies_map = HashMap::new();
        for cookie in response.cookies() {
            cookies_map.insert(cookie.name().to_string(), cookie.value().to_string());
        }

        let body = response.bytes().map_err(Error::from)?.to_vec();
        let elapsed = start.elapsed().as_secs_f64();

        let mut resp = Response::new(
            status_code,
            resp_headers,
            body,
            final_url.clone(),
            http_version,
            Cookies { inner: cookies_map },
            elapsed,
            method.to_uppercase(),
            reason_phrase,
        );

        // Create and attach a Request object for HTTPX compatibility
        let request_url = URL::from_str(&final_url).ok();
        if let Some(url) = request_url {
            let request = Request::new_internal(
                method.to_uppercase(),
                url,
                Headers::default(), // The actual headers are already sent
                None,
                false,
            );
            resp.set_request(request);
        }

        // Set default encoding if configured
        if let Some(ref encoding) = self.config.default_encoding {
            resp.set_default_encoding(encoding.clone());
        }

        Ok(resp)
    }

    /// GET request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn get(
        &self,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        self.request("GET", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects)
    }

    /// POST request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, content=None, data=None, json=None, files=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn post(
        &self,
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
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        self.request("POST", url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects)
    }

    /// PUT request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, content=None, data=None, json=None, files=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn put(
        &self,
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
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        self.request("PUT", url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects)
    }

    /// PATCH request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, content=None, data=None, json=None, files=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn patch(
        &self,
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
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        self.request("PATCH", url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects)
    }

    /// DELETE request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn delete(
        &self,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        self.request("DELETE", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects)
    }

    /// HEAD request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn head(
        &self,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        self.request("HEAD", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects)
    }

    /// OPTIONS request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn options(
        &self,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        self.request("OPTIONS", url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects)
    }

    /// Close the client
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Stream a request - returns StreamingResponse without loading body
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
        follow_redirects=None
    ))]
    pub fn stream(
        &self,
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
        #[allow(unused_variables)] follow_redirects: Option<bool>,
    ) -> PyResult<StreamingResponse> {
        let url_str = extract_url_str(url)?;
        let resolved_url = resolve_url(&self.config.base_url, &url_str)?;
        let start = Instant::now();

        // Build request
        let mut req = self.client.request(
            method
                .parse()
                .map_err(|_| Error::request(format!("Invalid method: {method}")))?,
            &resolved_url,
        );

        // Add query parameters
        if let Some(p) = params {
            let params_vec = extract_params(Some(p))?;
            req = req.query(&params_vec);
        }

        // Add headers
        if let Some(h) = headers {
            let headers_obj = extract_headers(h)?;
            for (key, values) in &headers_obj.inner {
                for value in values {
                    req = req.header(key.as_str(), value.as_str());
                }
            }
        }

        // Add cookies
        if let Some(c) = cookies {
            let cookies_map = extract_cookies(c)?;
            for (name, value) in &cookies_map {
                req = req.header("Cookie", format!("{name}={value}"));
            }
        }

        // Add client-level cookies
        for (name, value) in &self.config.cookies.inner {
            req = req.header("Cookie", format!("{name}={value}"));
        }

        // Set body
        if let Some(json_data) = json {
            let json_str = py_to_json_string(json_data)?;
            req = req.header("Content-Type", "application/json");
            req = req.body(json_str);
        } else if let Some(form_data) = data {
            if form_data.is_instance_of::<PyDict>() {
                let dict = form_data.cast::<PyDict>().unwrap();
                let form: HashMap<String, String> = dict
                    .iter()
                    .map(|(k, v)| Ok((k.extract::<String>()?, v.str()?.to_string())))
                    .collect::<PyResult<_>>()?;
                req = req.form(&form);
            }
        } else if let Some(body) = content {
            if body.is_instance_of::<PyBytes>() {
                let bytes_obj: Bound<'_, PyBytes> = body.extract()?;
                req = req.body(bytes_obj.as_bytes().to_vec());
            } else if let Ok(bytes) = body.extract::<Vec<u8>>() {
                req = req.body(bytes);
            } else if let Ok(s) = body.extract::<String>() {
                req = req.body(s);
            }
        } else if let Some(files_data) = files {
            if files_data.is_instance_of::<PyDict>() {
                let files_dict = files_data.extract::<Bound<'_, PyDict>>()?;
                let mut form = reqwest::blocking::multipart::Form::new();
                for (field_name, file_info) in files_dict.iter() {
                    let field_name: String = field_name.extract()?;
                    if let Ok(tuple) = file_info.extract::<(String, Vec<u8>, String)>() {
                        let (filename, file_content, content_type) = tuple;
                        let part = reqwest::blocking::multipart::Part::bytes(file_content)
                            .file_name(filename)
                            .mime_str(&content_type)
                            .map_err(|e| Error::request(e.to_string()))?;
                        form = form.part(field_name, part);
                    } else if let Ok(tuple) = file_info.extract::<(String, Vec<u8>)>() {
                        let (filename, file_content) = tuple;
                        let part = reqwest::blocking::multipart::Part::bytes(file_content).file_name(filename);
                        form = form.part(field_name, part);
                    }
                }
                req = req.multipart(form);
            }
        }

        // Authentication
        let auth_extracted = if let Some(a) = auth {
            extract_auth(a)?
        } else {
            None
        };
        let auth_to_use = auth_extracted.as_ref().or(self.config.auth.as_ref());
        if let Some(auth_config) = auth_to_use {
            match &auth_config.auth_type {
                AuthType::Basic { username, password } => {
                    req = req.basic_auth(username, Some(password));
                }
                AuthType::Bearer { token } => {
                    req = req.bearer_auth(token);
                }
                AuthType::Digest { username, password } => {
                    req = req.basic_auth(username, Some(password));
                }
            }
        }

        // Timeout (per-request)
        if let Some(t) = timeout {
            let timeout_config = extract_timeout(t)?;
            if let Some(total) = timeout_config.total {
                req = req.timeout(total);
            }
        }

        // Execute request - don't consume body
        let response = req.send().map_err(Error::from)?;
        let elapsed = start.elapsed().as_secs_f64();

        Ok(StreamingResponse::from_blocking(response, elapsed, &method.to_uppercase()))
    }

    /// Context manager enter
    pub fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager exit
    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    pub fn __exit__(&mut self, _exc_type: Option<&Bound<'_, PyAny>>, _exc_val: Option<&Bound<'_, PyAny>>, _exc_tb: Option<&Bound<'_, PyAny>>) {
        self.close();
    }

    pub fn __repr__(&self) -> String {
        format!("<Client base_url={:?}>", self.config.base_url)
    }

    /// Get the transport for a given URL (HTTPX compatibility)
    /// Returns None since we use reqwest internally
    #[pyo3(signature = (url))]
    pub fn _transport_for_url(&self, #[allow(unused_variables)] url: &Bound<'_, PyAny>) -> Option<PyObject> {
        None
    }
}

impl Client {
    /// Internal method to send a Request and get a Response
    fn send_request(&self, request: &Request) -> PyResult<Response> {
        let start = Instant::now();

        // Build reqwest request
        let mut req = self.client.request(
            request
                .method
                .parse()
                .map_err(|_| Error::request(format!("Invalid method: {}", request.method)))?,
            request.url_str(),
        );

        // Add headers
        for (key, values) in &request.headers_ref().inner {
            for value in values {
                req = req.header(key.as_str(), value.as_str());
            }
        }

        // Add body
        if let Some(body) = request.content_ref() {
            req = req.body(body.clone());
        }

        // Authentication
        if let Some(auth_config) = self.config.auth.as_ref() {
            match &auth_config.auth_type {
                AuthType::Basic { username, password } => {
                    req = req.basic_auth(username, Some(password));
                }
                AuthType::Bearer { token } => {
                    req = req.bearer_auth(token);
                }
                AuthType::Digest { username, password } => {
                    req = req.basic_auth(username, Some(password));
                }
            }
        }

        // Execute request
        let response = req.send().map_err(Error::from)?;

        // Convert to our Response type
        let status_code = response.status().as_u16();
        let reason_phrase = response
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();
        let final_url = response.url().to_string();
        let http_version = format!("{:?}", response.version());

        let resp_headers = Headers::from_reqwest_headers(response.headers());

        let mut cookies_map = HashMap::new();
        for cookie in response.cookies() {
            cookies_map.insert(cookie.name().to_string(), cookie.value().to_string());
        }

        let body = response.bytes().map_err(Error::from)?.to_vec();
        let elapsed = start.elapsed().as_secs_f64();

        let mut resp = Response::new(
            status_code,
            resp_headers,
            body,
            final_url,
            http_version,
            Cookies { inner: cookies_map },
            elapsed,
            request.method.clone(),
            reason_phrase,
        );

        // Set the request on the response
        resp.set_request(request.clone());

        // Set default encoding if configured
        if let Some(ref encoding) = self.config.default_encoding {
            resp.set_default_encoding(encoding.clone());
        }

        Ok(resp)
    }

    /// Internal method to send a Request and get a StreamingResponse
    fn send_streaming(&self, request: &Request) -> PyResult<StreamingResponse> {
        let start = Instant::now();

        // Build reqwest request
        let mut req = self.client.request(
            request
                .method
                .parse()
                .map_err(|_| Error::request(format!("Invalid method: {}", request.method)))?,
            request.url_str(),
        );

        // Add headers
        for (key, values) in &request.headers_ref().inner {
            for value in values {
                req = req.header(key.as_str(), value.as_str());
            }
        }

        // Add body
        if let Some(body) = request.content_ref() {
            req = req.body(body.clone());
        }

        // Authentication
        if let Some(auth_config) = self.config.auth.as_ref() {
            match &auth_config.auth_type {
                AuthType::Basic { username, password } => {
                    req = req.basic_auth(username, Some(password));
                }
                AuthType::Bearer { token } => {
                    req = req.bearer_auth(token);
                }
                AuthType::Digest { username, password } => {
                    req = req.basic_auth(username, Some(password));
                }
            }
        }

        // Execute request
        let response = req.send().map_err(Error::from)?;
        let elapsed = start.elapsed().as_secs_f64();

        let mut streaming_resp = StreamingResponse::from_blocking(response, elapsed, &request.method);
        streaming_resp = streaming_resp.with_request(request.clone());

        Ok(streaming_resp)
    }
}

/// Asynchronous HTTP Client
#[pyclass(name = "AsyncClient", subclass)]
pub struct AsyncClient {
    client: Arc<reqwest::Client>,
    config: ClientConfig,
    #[allow(dead_code)]
    runtime: Arc<Runtime>,
    /// Whether the client is closed
    closed: Arc<std::sync::Mutex<bool>>,
    /// Transport (for HTTPX compatibility)
    transport: Option<PyObject>,
}

#[pymethods]
impl AsyncClient {
    #[new]
    #[pyo3(signature = (
        base_url=None,
        headers=None,
        cookies=None,
        timeout=None,
        follow_redirects=true,
        max_redirects=10,
        verify=None,
        cert=None,
        proxy=None,
        auth=None,
        http2=false,
        limits=None,
        default_encoding=None,
        trust_env=true,
        transport=None,
        app=None,
        event_hooks=None,
        mounts=None,
        params=None
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_url: Option<String>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        timeout: Option<&Bound<'_, PyAny>>,
        follow_redirects: bool,
        max_redirects: usize,
        verify: Option<&Bound<'_, PyAny>>,
        cert: Option<&Bound<'_, PyAny>>,
        proxy: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        http2: bool,
        limits: Option<&Bound<'_, PyAny>>,
        default_encoding: Option<String>,
        trust_env: bool,
        transport: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] app: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] event_hooks: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] mounts: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] params: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let mut config = ClientConfig {
            base_url,
            follow_redirects,
            max_redirects,
            http2,
            default_encoding,
            trust_env,
            ..Default::default()
        };

        if let Some(h) = headers {
            config.headers = extract_headers(h)?;
        }
        if let Some(c) = cookies {
            config.cookies = Cookies { inner: extract_cookies(c)? };
        }
        if let Some(t) = timeout {
            config.timeout = extract_timeout(t)?;
        }
        if let Some(v) = verify {
            let (verify_ssl, ca_bundle) = extract_verify(v)?;
            config.verify_ssl = verify_ssl;
            config.ca_bundle = ca_bundle;
        }
        if let Some(c) = cert {
            let (cert_file, key_file, key_password) = extract_cert(c)?;
            config.cert_file = cert_file;
            config.key_file = key_file;
            config.key_password = key_password;
        }
        if let Some(l) = limits {
            config.limits = extract_limits(l)?;
        }
        if let Some(p) = proxy {
            config.proxy = extract_proxy(p)?;
        }
        if let Some(a) = auth {
            config.auth = extract_auth(a)?;
        }

        let client = build_reqwest_client(&config)?;
        let runtime = Runtime::new().map_err(|e| Error::request(e.to_string()))?;
        let transport_obj = transport.map(|t| t.clone().unbind());

        Ok(Self {
            client: Arc::new(client),
            config,
            runtime: Arc::new(runtime),
            closed: Arc::new(std::sync::Mutex::new(false)),
            transport: transport_obj,
        })
    }

    /// Whether the client is closed
    #[getter]
    pub fn is_closed(&self) -> bool {
        *self.closed.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Get the transport (HTTPX compatibility)
    #[getter]
    pub fn _transport(&self, py: Python<'_>) -> Option<PyObject> {
        self.transport.as_ref().map(|t| t.clone_ref(py))
    }

    /// Get the client timeout configuration
    #[getter]
    pub fn timeout(&self) -> Timeout {
        self.config.timeout.clone()
    }

    /// Get the base URL (HTTPX compatibility)
    #[getter]
    pub fn base_url(&self) -> Option<URL> {
        self.config.base_url.as_ref().and_then(|s| URL::from_str(s).ok())
    }

    /// Get the default headers (HTTPX compatibility)
    #[getter]
    pub fn headers(&self) -> Headers {
        self.config.headers.clone()
    }

    /// Build a request without sending it
    #[pyo3(signature = (
        method,
        url,
        params=None,
        headers=None,
        cookies=None,
        content=None,
        data=None,
        json=None,
        timeout=None
    ))]
    pub fn build_request(
        &self,
        method: &str,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyBytes>>,
        data: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        #[allow(unused_variables)] timeout: Option<f64>,
    ) -> PyResult<Request> {
        // Accept both string and URL object
        let url_str = if let Ok(s) = url.extract::<String>() {
            s
        } else if let Ok(url_obj) = url.extract::<URL>() {
            url_obj.as_url().to_string()
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err("url must be a string or URL object"));
        };
        let resolved_url = resolve_url(&self.config.base_url, &url_str)?;
        let parsed_url = URL::from_str(&resolved_url)?;

        // Merge headers
        let mut final_headers = self.config.headers.clone();
        if let Some(h) = headers {
            let req_headers = extract_headers(h)?;
            for (key, values) in &req_headers.inner {
                for value in values {
                    final_headers.add(key, value);
                }
            }
        }

        // Add cookies to headers
        if let Some(c) = cookies {
            let cookies_map = extract_cookies(c)?;
            for (name, value) in &cookies_map {
                final_headers.add("cookie", &format!("{name}={value}"));
            }
        }
        for (name, value) in &self.config.cookies.inner {
            final_headers.add("cookie", &format!("{name}={value}"));
        }

        // Add query params to URL
        let final_url = if let Some(p) = params {
            let params_vec = extract_params(Some(p))?;
            if !params_vec.is_empty() {
                let mut parsed = url::Url::parse(&resolved_url).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {e}")))?;
                for (k, v) in params_vec {
                    parsed.query_pairs_mut().append_pair(&k, &v);
                }
                URL::from_url(parsed)
            } else {
                parsed_url
            }
        } else {
            parsed_url
        };

        // Build content
        let body_content = if let Some(json_data) = json {
            let json_str = py_to_json_string(json_data)?;
            final_headers.set("content-type", "application/json");
            Some(json_str.into_bytes())
        } else if let Some(form_data) = data {
            let form: HashMap<String, String> = form_data
                .iter()
                .map(|(k, v)| Ok((k.extract::<String>()?, v.extract::<String>()?)))
                .collect::<PyResult<_>>()?;
            let encoded = form
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            final_headers.set("content-type", "application/x-www-form-urlencoded");
            Some(encoded.into_bytes())
        } else {
            content.map(|body| body.as_bytes().to_vec())
        };

        Ok(Request::new_internal(method.to_uppercase(), final_url, final_headers, body_content, false))
    }

    /// Send a pre-built request (async)
    #[pyo3(signature = (request, stream=false))]
    pub fn send<'py>(&self, py: Python<'py>, request: Request, stream: bool) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let config = self.config.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let start = Instant::now();

            // Build reqwest request
            let mut req = client.request(
                request
                    .method
                    .parse()
                    .map_err(|_| Error::request(format!("Invalid method: {}", request.method)))?,
                request.url_str(),
            );

            // Add headers
            for (key, values) in &request.headers_ref().inner {
                for value in values {
                    req = req.header(key.as_str(), value.as_str());
                }
            }

            // Add body
            if let Some(body) = request.content_ref() {
                req = req.body(body.clone());
            }

            // Authentication
            if let Some(auth_config) = config.auth.as_ref() {
                match &auth_config.auth_type {
                    AuthType::Basic { username, password } => {
                        req = req.basic_auth(username, Some(password));
                    }
                    AuthType::Bearer { token } => {
                        req = req.bearer_auth(token);
                    }
                    AuthType::Digest { username, password } => {
                        req = req.basic_auth(username, Some(password));
                    }
                }
            }

            // Execute request
            let response = req.send().await.map_err(Error::from)?;
            let elapsed = start.elapsed().as_secs_f64();

            if stream {
                let mut streaming_resp = AsyncStreamingResponse::from_async(response, elapsed, &request.method);
                streaming_resp = streaming_resp.with_request(request);
                Ok(Python::attach(|py| {
                    streaming_resp
                        .into_pyobject(py)
                        .map(|o| o.into_any().unbind())
                })?)
            } else {
                let mut resp = crate::response::Response::from_reqwest(response, start, &request.method).await?;
                resp.set_request(request);
                if let Some(ref encoding) = config.default_encoding {
                    resp.set_default_encoding(encoding.clone());
                }
                Ok(Python::attach(|py| resp.into_pyobject(py).map(|o| o.into_any().unbind()))?)
            }
        })
    }

    /// Perform an async request - returns a coroutine
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
        follow_redirects=None
    ))]
    pub fn request<'py>(
        &self,
        py: Python<'py>,
        method: String,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyBytes>>,
        data: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        files: Option<&Bound<'_, PyDict>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        #[allow(unused_variables)] follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url = extract_url_str(url)?;
        let params_vec = params.map(|p| extract_params(Some(p))).transpose()?;
        let headers_obj = headers.map(|h| extract_headers(h)).transpose()?;
        let cookies_obj = cookies
            .map(|c| Ok::<_, PyErr>(Cookies { inner: extract_cookies(c)? }))
            .transpose()?;
        let content_vec = content.map(|c| c.as_bytes().to_vec());
        let auth_extracted = auth.map(|a| extract_auth(a)).transpose()?.flatten();
        let data_map = data
            .map(|d| {
                d.iter()
                    .map(|(k, v)| Ok((k.extract::<String>()?, v.extract::<String>()?)))
                    .collect::<PyResult<HashMap<String, String>>>()
            })
            .transpose()?;
        let json_str = json.map(|j| py_to_json_string(j)).transpose()?;
        let files_map = files
            .map(|f| {
                f.iter()
                    .map(|(k, v)| {
                        let field_name: String = k.extract()?;
                        let tuple: (String, Vec<u8>, String) = v.extract()?;
                        Ok((field_name, tuple))
                    })
                    .collect::<PyResult<HashMap<String, (String, Vec<u8>, String)>>>()
            })
            .transpose()?;

        let client = self.client.clone();
        let config = self.config.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let resolved_url = resolve_url(&config.base_url, &url)?;
            let start = Instant::now();

            // Build request
            let mut req = client.request(
                method
                    .parse()
                    .map_err(|_| Error::request(format!("Invalid method: {method}")))?,
                &resolved_url,
            );

            // Add query parameters
            if let Some(p) = params_vec {
                req = req.query(&p);
            }

            // Add headers
            if let Some(h) = headers_obj {
                for (key, values) in &h.inner {
                    for value in values {
                        req = req.header(key.as_str(), value.as_str());
                    }
                }
            }

            // Add cookies
            if let Some(c) = cookies_obj {
                for (name, value) in &c.inner {
                    req = req.header("Cookie", format!("{name}={value}"));
                }
            }

            // Add client-level cookies
            for (name, value) in &config.cookies.inner {
                req = req.header("Cookie", format!("{name}={value}"));
            }

            // Set body
            if let Some(json_str) = json_str {
                req = req.header("Content-Type", "application/json");
                req = req.body(json_str);
            } else if let Some(form_data) = data_map {
                req = req.form(&form_data);
            } else if let Some(body) = content_vec {
                req = req.body(body);
            } else if let Some(files_map) = files_map {
                let mut form = reqwest::multipart::Form::new();
                for (field_name, (filename, file_content, content_type)) in files_map {
                    let part = reqwest::multipart::Part::bytes(file_content)
                        .file_name(filename)
                        .mime_str(&content_type)
                        .map_err(|e| Error::request(e.to_string()))?;
                    form = form.part(field_name, part);
                }
                req = req.multipart(form);
            }

            // Authentication - use extracted auth or config auth
            let auth_to_use = auth_extracted.as_ref().or(config.auth.as_ref());
            if let Some(auth_config) = auth_to_use {
                match &auth_config.auth_type {
                    AuthType::Basic { username, password } => {
                        req = req.basic_auth(username, Some(password));
                    }
                    AuthType::Bearer { token } => {
                        req = req.bearer_auth(token);
                    }
                    AuthType::Digest { username, password } => {
                        req = req.basic_auth(username, Some(password));
                    }
                }
            }

            // Timeout (per-request)
            if let Some(t) = timeout {
                req = req.timeout(Duration::from_secs_f64(t));
            }

            // Execute request
            let response = req.send().await.map_err(Error::from)?;

            // Capture final URL before consuming response
            let final_url = response.url().to_string();

            // Convert to our Response type
            let mut resp = Response::from_reqwest(response, start, &method).await?;

            // Create and attach a Request object for HTTPX compatibility
            if let Ok(url) = URL::from_str(&final_url) {
                let request = Request::new_internal(method.to_uppercase(), url, Headers::default(), None, false);
                resp.set_request(request);
            }

            // Set default encoding if configured
            if let Some(ref encoding) = config.default_encoding {
                resp.set_default_encoding(encoding.clone());
            }

            Ok(resp)
        })
    }

    /// Async GET request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn get<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "GET".to_string(), url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects)
    }

    /// Async POST request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, content=None, data=None, json=None, files=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn post<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyBytes>>,
        data: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        files: Option<&Bound<'_, PyDict>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "POST".to_string(), url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects)
    }

    /// Async PUT request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, content=None, data=None, json=None, files=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn put<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyBytes>>,
        data: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        files: Option<&Bound<'_, PyDict>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "PUT".to_string(), url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects)
    }

    /// Async PATCH request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, content=None, data=None, json=None, files=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn patch<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyBytes>>,
        data: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        files: Option<&Bound<'_, PyDict>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "PATCH".to_string(), url, params, headers, cookies, content, data, json, files, auth, timeout, follow_redirects)
    }

    /// Async DELETE request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn delete<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "DELETE".to_string(), url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects)
    }

    /// Async HEAD request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn head<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "HEAD".to_string(), url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects)
    }

    /// Async OPTIONS request
    #[pyo3(signature = (url, params=None, headers=None, cookies=None, auth=None, timeout=None, follow_redirects=None))]
    pub fn options<'py>(
        &self,
        py: Python<'py>,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.request(py, "OPTIONS".to_string(), url, params, headers, cookies, None, None, None, None, auth, timeout, follow_redirects)
    }

    /// Close the client
    pub fn aclose<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let closed = self.closed.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            *closed.lock().unwrap_or_else(|e| e.into_inner()) = true;
            Ok(())
        })
    }

    /// Async stream a request - returns AsyncStreamingResponse without loading body
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
        follow_redirects=None
    ))]
    pub fn stream<'py>(
        &self,
        py: Python<'py>,
        method: String,
        url: &Bound<'_, PyAny>,
        params: Option<&Bound<'_, PyDict>>,
        headers: Option<&Bound<'_, PyAny>>,
        cookies: Option<&Bound<'_, PyAny>>,
        content: Option<&Bound<'_, PyBytes>>,
        data: Option<&Bound<'_, PyDict>>,
        json: Option<&Bound<'_, PyAny>>,
        files: Option<&Bound<'_, PyDict>>,
        auth: Option<&Bound<'_, PyAny>>,
        timeout: Option<f64>,
        #[allow(unused_variables)] follow_redirects: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let url = extract_url_str(url)?;
        let params_vec = params.map(|p| extract_params(Some(p))).transpose()?;
        let headers_obj = headers.map(|h| extract_headers(h)).transpose()?;
        let cookies_obj = cookies
            .map(|c| Ok::<_, PyErr>(Cookies { inner: extract_cookies(c)? }))
            .transpose()?;
        let content_vec = content.map(|c| c.as_bytes().to_vec());
        let auth_extracted = auth.map(|a| extract_auth(a)).transpose()?.flatten();
        let data_map = data
            .map(|d| {
                d.iter()
                    .map(|(k, v)| Ok((k.extract::<String>()?, v.extract::<String>()?)))
                    .collect::<PyResult<HashMap<String, String>>>()
            })
            .transpose()?;
        let json_str = json.map(|j| py_to_json_string(j)).transpose()?;
        let files_map = files
            .map(|f| {
                f.iter()
                    .map(|(k, v)| {
                        let field_name: String = k.extract()?;
                        let tuple: (String, Vec<u8>, String) = v.extract()?;
                        Ok((field_name, tuple))
                    })
                    .collect::<PyResult<HashMap<String, (String, Vec<u8>, String)>>>()
            })
            .transpose()?;

        let client = self.client.clone();
        let config = self.config.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let resolved_url = resolve_url(&config.base_url, &url)?;
            let start = Instant::now();

            // Build request
            let mut req = client.request(
                method
                    .parse()
                    .map_err(|_| Error::request(format!("Invalid method: {method}")))?,
                &resolved_url,
            );

            // Add query parameters
            if let Some(p) = params_vec {
                req = req.query(&p);
            }

            // Add headers
            if let Some(h) = headers_obj {
                for (key, values) in &h.inner {
                    for value in values {
                        req = req.header(key.as_str(), value.as_str());
                    }
                }
            }

            // Add cookies
            if let Some(c) = cookies_obj {
                for (name, value) in &c.inner {
                    req = req.header("Cookie", format!("{name}={value}"));
                }
            }

            // Add client-level cookies
            for (name, value) in &config.cookies.inner {
                req = req.header("Cookie", format!("{name}={value}"));
            }

            // Set body
            if let Some(json_str) = json_str {
                req = req.header("Content-Type", "application/json");
                req = req.body(json_str);
            } else if let Some(form_data) = data_map {
                req = req.form(&form_data);
            } else if let Some(body) = content_vec {
                req = req.body(body);
            } else if let Some(files_map) = files_map {
                let mut form = reqwest::multipart::Form::new();
                for (field_name, (filename, file_content, content_type)) in files_map {
                    let part = reqwest::multipart::Part::bytes(file_content)
                        .file_name(filename)
                        .mime_str(&content_type)
                        .map_err(|e| Error::request(e.to_string()))?;
                    form = form.part(field_name, part);
                }
                req = req.multipart(form);
            }

            // Authentication - use extracted auth or config auth
            let auth_to_use = auth_extracted.as_ref().or(config.auth.as_ref());
            if let Some(auth_config) = auth_to_use {
                match &auth_config.auth_type {
                    AuthType::Basic { username, password } => {
                        req = req.basic_auth(username, Some(password));
                    }
                    AuthType::Bearer { token } => {
                        req = req.bearer_auth(token);
                    }
                    AuthType::Digest { username, password } => {
                        req = req.basic_auth(username, Some(password));
                    }
                }
            }

            // Timeout (per-request)
            if let Some(t) = timeout {
                req = req.timeout(Duration::from_secs_f64(t));
            }

            // Execute request - don't consume body
            let response = req.send().await.map_err(Error::from)?;
            let elapsed = start.elapsed().as_secs_f64();

            Ok(AsyncStreamingResponse::from_async(response, elapsed, &method.to_uppercase()))
        })
    }

    /// Async context manager enter
    pub fn __aenter__<'py>(slf: Py<Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let slf_clone = slf.clone_ref(py);
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(slf_clone) })
    }

    /// Async context manager exit
    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    pub fn __aexit__<'py>(&self, py: Python<'py>, _exc_type: Option<&Bound<'_, PyAny>>, _exc_val: Option<&Bound<'_, PyAny>>, _exc_tb: Option<&Bound<'_, PyAny>>) -> PyResult<Bound<'py, PyAny>> {
        let closed = self.closed.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            *closed.lock().unwrap_or_else(|e| e.into_inner()) = true;
            Ok(())
        })
    }

    pub fn __repr__(&self) -> String {
        format!("<AsyncClient base_url={:?}>", self.config.base_url)
    }

    /// Get the transport for a given URL (HTTPX compatibility)
    /// Returns None since we use reqwest internally
    #[pyo3(signature = (url))]
    pub fn _transport_for_url(&self, #[allow(unused_variables)] url: &Bound<'_, PyAny>) -> Option<PyObject> {
        None
    }
}

/// Convert Python object to JSON string
fn py_to_json_string(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = py_to_json_value(obj)?;
    sonic_rs::to_string(&value).map_err(|e| Error::request(e.to_string()).into())
}

/// Convert Python object to sonic_rs::Value
fn py_to_json_value(obj: &Bound<'_, PyAny>) -> PyResult<sonic_rs::Value> {
    use pyo3::types::PyList;
    use sonic_rs::json;

    if obj.is_none() {
        Ok(sonic_rs::Value::default())
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(json!(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(json!(i))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(json!(f))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(json!(s))
    } else if obj.is_instance_of::<PyList>() {
        let list = obj.extract::<Bound<'_, PyList>>()?;
        let arr: Vec<sonic_rs::Value> = list
            .iter()
            .map(|item| py_to_json_value(&item))
            .collect::<PyResult<_>>()?;
        Ok(sonic_rs::Value::from(arr))
    } else if obj.is_instance_of::<PyDict>() {
        let dict = obj.extract::<Bound<'_, PyDict>>()?;
        let mut obj_map = sonic_rs::Object::new();
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            let value = py_to_json_value(&value)?;
            obj_map.insert(&key, value);
        }
        Ok(sonic_rs::Value::from(obj_map))
    } else {
        // Try to convert to string as fallback
        let s = obj.str()?.extract::<String>()?;
        Ok(json!(s))
    }
}
