//! Python kwargs parsing module
//!
//! Provides parsing functions for Python keyword arguments used in HTTP requests.
//! Each sub-module handles a specific category of parameters.

pub mod auth;
pub mod body;
pub mod config;
pub mod headers;
pub mod params;

pub use auth::parse_auth;
pub use body::{extract_file_data, parse_data, parse_files, parse_json, parse_single_file};
pub use config::{parse_cert, parse_proxies, parse_timeout};
pub use headers::{detect_content_type, parse_headers};
pub use params::parse_params;

use hyper::{HeaderMap, Method, Uri};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sonic_rs::Value;
use std::collections::HashMap;
use std::time::Duration;

use crate::core::http_client::FilePart;
use crate::error::RequestxError;

/// Helper struct for building RequestConfig
#[derive(Debug, Clone)]
pub struct RequestConfigBuilder {
    pub headers: Option<HeaderMap>,
    pub params: Option<HashMap<String, String>>,
    pub data: Option<super::super::core::http_client::RequestData>,
    pub json: Option<Value>,
    pub files: Option<Vec<FilePart>>,
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
    #[inline]
    pub fn new() -> Self {
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

    pub fn build(self, method: Method, url: Uri) -> super::super::core::http_client::RequestConfig {
        super::super::core::http_client::RequestConfig {
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

impl Default for RequestConfigBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Parse and validate URL with comprehensive error handling
#[inline]
pub fn parse_and_validate_url(url: &str) -> PyResult<(Uri, Option<(String, String)>)> {
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
pub fn parse_kwargs(
    py: Python,
    kwargs: Option<&Bound<'_, PyDict>>,
) -> PyResult<RequestConfigBuilder> {
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
