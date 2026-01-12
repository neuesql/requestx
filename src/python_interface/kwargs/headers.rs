//! Header parsing utilities
//!
//! Provides functions for parsing HTTP headers from Python objects.

use crate::error::RequestxError;
use hyper::header::HeaderMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Parse headers from Python object with comprehensive error handling
pub fn parse_headers(headers_obj: &Bound<'_, PyAny>) -> PyResult<HeaderMap> {
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

/// Detect content type from filename
#[inline]
pub fn detect_content_type(filename: &str) -> Option<String> {
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
