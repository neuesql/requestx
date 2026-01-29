//! Multipart form data encoding

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

/// Generate a random boundary string for multipart forms
pub fn generate_boundary() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("----WebKitFormBoundary{:x}", timestamp)
}

/// Extract boundary from Content-Type header
pub fn extract_boundary_from_content_type(content_type: &str) -> Option<String> {
    for part in content_type.split(';') {
        let part = part.trim();
        if part.starts_with("boundary=") {
            let boundary = part.strip_prefix("boundary=").unwrap();
            // Remove quotes if present
            let boundary = boundary.trim_matches('"').trim_matches('\'');
            return Some(boundary.trim().to_string());
        }
    }
    None
}

/// Build multipart body with auto-generated boundary
pub fn build_multipart_body(
    py: Python<'_>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
) -> PyResult<(Vec<u8>, String)> {
    let boundary = generate_boundary();
    let body = build_multipart_body_with_boundary(py, data, files, &boundary)?;
    Ok((body.0, boundary))
}

/// Build multipart body with specified boundary
pub fn build_multipart_body_with_boundary(
    py: Python<'_>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
    boundary: &str,
) -> PyResult<(Vec<u8>, String)> {
    let mut body = Vec::new();
    let boundary_bytes = boundary.as_bytes();

    // Add data fields first
    if let Some(d) = data {
        for (key, value) in d.iter() {
            let k: String = key.extract()?;
            // Handle different value types
            add_data_field(py, &mut body, boundary_bytes, &k, &value)?;
        }
    }

    // Add file fields
    if let Some(f) = files {
        if let Ok(dict) = f.downcast::<PyDict>() {
            for (key, value) in dict.iter() {
                let field_name: String = key.extract()?;

                // Files can be:
                // - file-like object (has read() method)
                // - tuple: (filename, file-content)
                // - tuple: (filename, file-content, content-type)
                // - tuple: (filename, file-content, content-type, headers)
                let (filename, content, content_type, extra_headers) = parse_file_value(py, &value, &field_name)?;

                body.extend_from_slice(b"--");
                body.extend_from_slice(boundary_bytes);
                body.extend_from_slice(b"\r\n");

                // Build Content-Disposition header
                if let Some(ref fname) = filename {
                    body.extend_from_slice(format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                        field_name, fname
                    ).as_bytes());
                } else {
                    // No filename - just field name
                    body.extend_from_slice(format!(
                        "Content-Disposition: form-data; name=\"{}\"\r\n",
                        field_name
                    ).as_bytes());
                }

                // Add content-type if we have a filename
                if filename.is_some() {
                    body.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes());
                }

                // Add extra headers if any
                for (hk, hv) in extra_headers {
                    body.extend_from_slice(format!("{}: {}\r\n", hk, hv).as_bytes());
                }

                body.extend_from_slice(b"\r\n");
                body.extend_from_slice(&content);
                body.extend_from_slice(b"\r\n");
            }
        }
    }

    // Add closing boundary
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary_bytes);
    body.extend_from_slice(b"--\r\n");

    Ok((body, boundary.to_string()))
}

/// Add a data field to the multipart body
fn add_data_field(
    py: Python<'_>,
    body: &mut Vec<u8>,
    boundary_bytes: &[u8],
    key: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<()> {
    // Check if value is a list - if so, add multiple fields with same name
    if let Ok(list) = value.downcast::<PyList>() {
        for item in list.iter() {
            add_single_data_field(py, body, boundary_bytes, key, &item)?;
        }
        return Ok(());
    }

    // Single value
    add_single_data_field(py, body, boundary_bytes, key, value)
}

/// Add a single data field to the multipart body
fn add_single_data_field(
    _py: Python<'_>,
    body: &mut Vec<u8>,
    boundary_bytes: &[u8],
    key: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<()> {
    // Handle different value types
    let v_bytes: Vec<u8> = if let Ok(s) = value.extract::<String>() {
        s.into_bytes()
    } else if let Ok(b) = value.extract::<Vec<u8>>() {
        b
    } else if let Ok(b) = value.extract::<bool>() {
        // Convert boolean to lowercase string
        if b { b"true".to_vec() } else { b"false".to_vec() }
    } else if let Ok(i) = value.extract::<i64>() {
        i.to_string().into_bytes()
    } else if let Ok(f) = value.extract::<f64>() {
        f.to_string().into_bytes()
    } else if value.is_none() {
        b"".to_vec()
    } else {
        value.str()?.to_string().into_bytes()
    };

    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{}\"\r\n", key).as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(&v_bytes);
    body.extend_from_slice(b"\r\n");

    Ok(())
}

/// Parse a file value which can be a file-like object or tuple
fn parse_file_value(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    field_name: &str,
) -> PyResult<(Option<String>, Vec<u8>, String, Vec<(String, String)>)> {
    // Check if it's a tuple: (filename, content) or (filename, content, content_type) or (filename, content, content_type, headers)
    if let Ok(tuple) = value.downcast::<PyTuple>() {
        let len = tuple.len();
        if len >= 2 {
            // Get filename (can be None)
            let filename: Option<String> = if tuple.get_item(0)?.is_none() {
                None
            } else {
                Some(tuple.get_item(0)?.extract::<String>().unwrap_or_else(|_| "upload".to_string()))
            };

            // Get content
            let content_item = tuple.get_item(1)?;
            let content = read_file_content(py, &content_item)?;

            // Get content type if provided
            let content_type = if len >= 3 {
                let ct_item = tuple.get_item(2)?;
                if ct_item.is_none() {
                    guess_content_type(filename.as_deref().unwrap_or(""))
                } else {
                    ct_item.extract::<String>().unwrap_or_else(|_| guess_content_type(filename.as_deref().unwrap_or("")))
                }
            } else {
                guess_content_type(filename.as_deref().unwrap_or(""))
            };

            // Get extra headers if provided
            let extra_headers = if len >= 4 {
                let headers_item = tuple.get_item(3)?;
                if let Ok(dict) = headers_item.downcast::<PyDict>() {
                    let mut headers = Vec::new();
                    for (k, v) in dict.iter() {
                        headers.push((k.extract::<String>()?, v.extract::<String>()?));
                    }
                    headers
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            return Ok((filename, content, content_type, extra_headers));
        }
    }

    // It's a file-like object
    let content = read_file_content(py, value)?;
    let filename = Some("upload".to_string());
    let content_type = "application/octet-stream".to_string();

    Ok((filename, content, content_type, Vec::new()))
}

/// Read content from a file-like object or bytes/string
pub fn read_file_content(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    // Try to extract as bytes directly
    if let Ok(bytes) = value.extract::<Vec<u8>>() {
        return Ok(bytes);
    }

    // Try to extract as string
    if let Ok(s) = value.extract::<String>() {
        return Ok(s.into_bytes());
    }

    // Try to call read() method (file-like object)
    if let Ok(read_method) = value.getattr("read") {
        let content = read_method.call0()?;
        if let Ok(bytes) = content.extract::<Vec<u8>>() {
            return Ok(bytes);
        }
        if let Ok(s) = content.extract::<String>() {
            return Ok(s.into_bytes());
        }
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "File content must be bytes, str, or a file-like object with read() method"
    ))
}

/// Guess content type from filename
pub fn guess_content_type(filename: &str) -> String {
    if let Some(ext) = filename.rsplit('.').next() {
        match ext.to_lowercase().as_str() {
            "json" => "application/json".to_string(),
            "txt" => "text/plain".to_string(),
            "html" | "htm" => "text/html".to_string(),
            "xml" => "application/xml".to_string(),
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            "gif" => "image/gif".to_string(),
            "pdf" => "application/pdf".to_string(),
            "zip" => "application/zip".to_string(),
            "css" => "text/css".to_string(),
            "js" => "application/javascript".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    } else {
        "application/octet-stream".to_string()
    }
}
