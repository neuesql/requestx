//! Authentication header building utilities
//!
//! Provides functions for building HTTP authentication headers.

use base64::prelude::*;
use hyper::header::{HeaderMap, HeaderValue};
use hyper::Request;
use std::collections::HashMap;

/// Build Basic authentication header from username and password
pub fn build_basic_auth_header(username: &str, password: &str) -> String {
    let mut credentials = String::with_capacity(username.len() + password.len() + 1);
    credentials.push_str(username);
    credentials.push(':');
    credentials.push_str(password);
    let encoded = BASE64_STANDARD.encode(credentials.as_bytes());
    format!("Basic {encoded}")
}

/// Add authentication header to request builder
pub fn add_auth_header(
    request_builder: hyper::http::request::Builder,
    auth: Option<&(String, String)>,
) -> hyper::http::request::Builder {
    if let Some(auth) = auth {
        let auth_header = build_basic_auth_header(&auth.0, &auth.1);
        if let Ok(header_value) = HeaderValue::from_str(&auth_header) {
            return request_builder.header("authorization", header_value);
        }
    }
    request_builder
}

/// Add URL query parameters to URL
pub fn add_query_params(url: &hyper::Uri, params: Option<&HashMap<String, String>>) -> String {
    if let Some(params) = params {
        if params.is_empty() {
            return url.to_string();
        }

        let mut url_with_params = url.to_string();
        let separator = if url.query().is_some() { '&' } else { '?' };

        url_with_params.push(separator);

        let encoded_params: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect();

        url_with_params.push_str(&encoded_params.join("&"));
        url_with_params
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_basic_auth_header() {
        let header = build_basic_auth_header("user", "pass");
        assert!(header.starts_with("Basic "));
        // "user:pass" base64 encoded is "dXNlcjpwYXNz"
        assert!(header.contains("dXNlcjpwYXNz"));
    }

    #[test]
    fn test_build_basic_auth_header_empty_password() {
        let header = build_basic_auth_header("admin", "");
        assert!(header.starts_with("Basic "));
        // "admin:" base64 encoded is "YWRtaW46"
        assert!(header.contains("YWRtaW46"));
    }
}
